"""One request in, one JSON envelope out, then exit to release all Metal memory."""
import argparse
import contextlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import re
import resource
import subprocess
import sys
import time
import traceback

# Must precede any HF/Transformers imports. Network is setup-only.
for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE", "HF_HUB_DISABLE_TELEMETRY", "DO_NOT_TRACK"):
    os.environ[name] = "1"
os.environ["TOKENIZERS_PARALLELISM"] = "false"


def swap_bytes():
    if sys.platform != "darwin":
        return None
    try:
        value = subprocess.check_output(["/usr/sbin/sysctl", "-n", "vm.swapusage"], text=True, timeout=2)
        match = re.search(r"used = ([\d.]+)([MG])", value)
        return int(float(match[1]) * (1024**2 if match[2] == "M" else 1024**3)) if match else None
    except (OSError, subprocess.SubprocessError):
        return None


def validate_request(request):
    if request.get("protocol_version") != 1:
        raise ValueError("Unsupported worker protocol")
    settings = request["settings"]
    if not 256 <= settings["max_tokens"] <= 4096:
        raise ValueError("max_tokens out of range")
    if not 2048 <= settings["context_tokens"] <= 8192:
        raise ValueError("context_tokens out of range")
    if not 4 * 1024**3 <= settings["memory_limit_bytes"] <= 12 * 1024**3:
        raise ValueError("memory limit out of range")
    if not 0 <= settings["temperature"] <= 1.5:
        raise ValueError("temperature out of range")
    return settings


def require_downloaded(path):
    # macOS SF_DATALESS: reject a placeholder before a model read triggers hydration.
    if getattr(path.stat(), "st_flags", 0) & 0x40000000:
        raise ValueError(f"Model file is offloaded to iCloud: {path}. Keep the runtime downloaded before generating.")


def generate(model_path, request, meta):
    import mlx.core as mx
    from mlx_lm import stream_generate
    from mlx_lm.utils import load_config, load_model, load_tokenizer
    from mlx_lm.sample_utils import make_sampler
    from structured import StructuredDecoder

    settings = validate_request(request)
    if not mx.metal.is_available():
        raise RuntimeError("Metal is unavailable; run on Apple Silicon or use the GGUF backend")
    if not model_path.is_dir() or not (model_path / "config.json").is_file():
        raise ValueError("Pass an existing local MLX model directory; downloads are disabled")
    meta.update({"backend":"mlx-lm", "model_path":str(model_path), "mlx_version":importlib.metadata.version("mlx"),
                 "runtime_version":importlib.metadata.version("mlx-lm"), "settings":settings})
    manifest = model_path / "hornbill-model.json"
    if manifest.is_file():
        require_downloaded(manifest)
        meta["model"] = json.loads(manifest.read_text())
        selections = json.loads(Path(__file__).with_name('models.json').read_text())
        if meta['model'] not in selections.values():
            raise ValueError('Installed story model is superseded; download and select the current profile explicitly')
        for name, detail in meta['model']['files'].items():
            path = model_path / name
            if not path.is_file() or path.stat().st_size != detail['bytes']:
                raise ValueError(f'Missing or incomplete selected model file: {path}')
            require_downloaded(path)
    mx.set_memory_limit(settings["memory_limit_bytes"])
    mx.set_cache_limit(256 * 1024**2)
    mx.reset_peak_memory()
    config = load_config(model_path)
    tokenizer = load_tokenizer(model_path, {"trust_remote_code":False,"local_files_only":True},
                               eos_token_ids=config.get("eos_token_id"))
    formatted = tokenizer.apply_chat_template(request["messages"], tokenize=False, add_generation_prompt=True, enable_thinking=False)
    tokens = tokenizer.encode(formatted, add_special_tokens=False)
    meta["formatted_prompt"] = formatted
    meta["prompt_tokens"] = len(tokens)
    if len(tokens) + settings["max_tokens"] > settings["context_tokens"]:
        raise ValueError(f"Context budget exceeded: {len(tokens)} prompt + {settings['max_tokens']} output > {settings['context_tokens']}. Reduce the world/context or output length; no state was changed.")
    # Generate the story before its choices. Object-key ordering changes only
    # decoding order, never the contract or Rust's independent validation.
    schema = dict(request["schema"])
    properties = schema["properties"]
    order = ("narration", "dialogue", "scene", "state_changes", "memory_updates", "choices")
    schema["properties"] = {key: properties[key] for key in order}
    processor = StructuredDecoder(schema, tokenizer, meta)

    load_start = time.perf_counter()
    model, _ = load_model(model_path, lazy=False, strict=True, trust_remote_code=False)
    meta["load_ms"] = (time.perf_counter() - load_start)*1000
    meta["mlx_active_after_load_bytes"] = mx.get_active_memory()
    mx.random.seed(settings["seed"])
    generation_start = time.perf_counter()
    pieces = []
    last = None
    for part in stream_generate(model, tokenizer, tokens, max_tokens=settings["max_tokens"],
                                sampler=make_sampler(temp=settings["temperature"],top_p=0.9),
                                logits_processors=[processor], prefill_step_size=256):
        if last is None:
            meta["ttft_ms"] = (time.perf_counter() - generation_start)*1000
        pieces.append(part.text)
        last = part
    mx.synchronize()
    meta.update({"generation_ms":(time.perf_counter()-generation_start)*1000,
                 "generation_tokens":last.generation_tokens if last else 0,
                 "generation_tps":last.generation_tps if last else None,
                 "prompt_tps":last.prompt_tps if last else None,
                 "finish_reason":last.finish_reason if last else None,
                 "mlx_peak_bytes":mx.get_peak_memory(), "mlx_active_end_bytes":mx.get_active_memory(),
                 "mlx_cache_end_bytes":mx.get_cache_memory()})
    # The OS process exit, not a Python del/gc assertion, is the reclamation boundary.
    return "".join(pieces)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path)
    parser.add_argument("--doctor", action="store_true")
    args = parser.parse_args()
    if args.doctor:
        import mlx.core as mx
        print(json.dumps({"python":sys.version.split()[0],"machine":platform.machine(),
                          "mlx":importlib.metadata.version("mlx"),"mlx_lm":importlib.metadata.version("mlx-lm"),
                          "metal_available":mx.metal.is_available(),"device":mx.device_info(),
                          "swap_used_bytes":swap_bytes()},indent=2))
        return
    result = {"protocol_version":1,"raw_text":"","metadata":{},"error":None}
    meta = result["metadata"]
    meta["swap_before_bytes"] = swap_bytes()
    try:
        payload = sys.stdin.buffer.read(256*1024 + 1)
        if len(payload) > 256*1024:
            raise ValueError("Request exceeds 256 KiB")
        request = json.loads(payload)
        if args.model is None:
            raise ValueError("--model is required")
        # Python/runtime messages must not corrupt the stdout protocol.
        with contextlib.redirect_stdout(sys.stderr):
            result["raw_text"] = generate(args.model.resolve(), request, meta)
    except Exception as error:
        traceback.print_exc(file=sys.stderr)
        result["error"] = f"{type(error).__name__}: {error}"
    finally:
        usage = resource.getrusage(resource.RUSAGE_SELF)
        meta["peak_rss_bytes"] = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)
        meta["swap_after_bytes"] = swap_bytes()
    print(json.dumps(result,ensure_ascii=False,allow_nan=False),flush=True)


if __name__ == "__main__":
    main()

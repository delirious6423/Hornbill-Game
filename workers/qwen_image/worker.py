"""One offline Qwen Image 2.1 request, staged Metal inference, one result, exit."""
import argparse
import contextlib
import gc
import importlib.metadata
import json
import math
import os
from pathlib import Path
import re
import resource
import subprocess
import sys
import time
import traceback

for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE", "HF_HUB_DISABLE_TELEMETRY", "DO_NOT_TRACK"):
    os.environ[name] = "1"
os.environ["TOKENIZERS_PARALLELISM"] = "false"


def swap_bytes():
    try:
        value = subprocess.check_output(["/usr/sbin/sysctl", "-n", "vm.swapusage"], text=True, timeout=2)
        found = re.search(r"used = ([\d.]+)([MG])", value)
        return int(float(found[1]) * (1024**2 if found[2] == "M" else 1024**3)) if found else None
    except (OSError, subprocess.SubprocessError):
        return None


def validate_request(request):
    if request.get("protocol_version") != 1:
        raise ValueError("Unsupported image protocol")
    prompt = request.get("prompt")
    if not isinstance(prompt, str) or not prompt.strip() or len(prompt) > 9000:
        raise ValueError("Image prompt must contain 1–9000 characters")
    s = request["settings"]
    for field in ("width", "height"):
        if type(s[field]) is not int or not 256 <= s[field] <= 768 or s[field] % 32:
            raise ValueError("Image dimensions must be multiples of 32 between 256 and 768")
    if s["steps"] not in (6, 20, 40):
        raise ValueError("Use the verified six-step schedule, or 20/40 base-model steps")
    if type(s["seed"]) is not int or not 0 <= s["seed"] < 2**32:
        raise ValueError("Invalid seed")
    if not 4 * 1024**3 <= s["memory_limit_bytes"] <= 10 * 1024**3:
        raise ValueError("Image memory limit must be 4–10 GiB")
    strength = s.get("reference_strength", 0.6)
    if not isinstance(strength, (int, float)) or not math.isfinite(strength) or not 0.3 <= strength <= 1:
        raise ValueError("Reference strength must be 0.3–1")
    output = Path(request["output_path"])
    if not output.is_absolute() or output.suffix.lower() != ".png" or not output.parent.is_dir() or output.exists():
        raise ValueError("Output must be a new absolute PNG path in an existing directory")
    return s, output


def check_model(root):
    spec = json.loads(Path(__file__).with_name("models.json").read_text())
    if not (root / "hornbill-model.json").is_file():
        raise ValueError("Image weights are not installed; run ./scripts/image.sh download")
    if json.loads((root / "hornbill-model.json").read_text()) != spec:
        raise ValueError("Image weights use a different manifest; run the explicit installer")
    for part in ("base", "turbo"):
        for name, entry in spec[part]["files"].items():
            path = root / part / name
            if not path.is_file() or path.stat().st_size != entry["bytes"]:
                raise ValueError(f"Missing or incomplete image component: {path}")
    index = json.loads((root / "base/text_encoder/model.safetensors.index.json").read_text())
    if str(index["metadata"]["quantization_level"]) != "4" or not {
        "embed_tokens.weight", "embed_tokens.scales", "embed_tokens.biases"
    }.issubset(index["weight_map"]):
        raise ValueError("Expected the pinned native Q4 text encoder")
    return spec


def generate(root, request, meta):
    settings, output = validate_request(request)
    spec = check_model(root)
    import mlx.core as mx
    from mflux.models.common.config import ModelConfig
    from mflux.models.common.schedulers.linear_scheduler import LinearScheduler
    from mflux.models.common.vae.tiling_config import TilingConfig
    from mflux.models.qwen21.variants.txt2img.qwen_image_21 import QwenImage21
    from mflux.models.qwen21.weights.qwen21_weight_definition import Qwen21WeightDefinition

    if not mx.metal.is_available():
        raise RuntimeError("Qwen image backend requires Apple Silicon Metal")
    mx.set_memory_limit(settings["memory_limit_bytes"])
    mx.set_wired_limit(min(settings["memory_limit_bytes"], 8 * 1024**3))
    mx.set_cache_limit(0)
    mx.reset_peak_memory()
    meta.update(backend="mflux-qwen21", mflux_version=importlib.metadata.version("mflux"),
                mflux_revision=spec["mflux_revision"], mlx_version=importlib.metadata.version("mlx"),
                model=spec["base"]["repo"], model_revision=spec["base"]["revision"], settings=settings)
    turbo = settings["steps"] == 6
    original_components = Qwen21WeightDefinition.get_components

    def quantized_encoder():
        components = original_components()
        for component in components:
            if component.name == "text_encoder":
                component.skip_quantization = False
        return components

    # This adjustment is valid only for the checked full-Q4 pack, not arbitrary MFLUX models.
    Qwen21WeightDefinition.get_components = staticmethod(quantized_encoder)
    start = time.perf_counter()
    try:
        model = QwenImage21(quantize=4, model_path=str(root / "base"), model_config=ModelConfig.qwen_image_21(),
                            lora_paths=[str(root / "turbo" / spec["turbo"]["adapter"])] if turbo else None,
                            lora_scales=[1.0] if turbo else None, bake_lora=False)
    finally:
        Qwen21WeightDefinition.get_components = staticmethod(original_components)
    meta["load_ms"] = (time.perf_counter() - start) * 1000
    if turbo:
        meta.update(adapter=spec["turbo"]["adapter"], adapter_revision=spec["turbo"]["revision"],
                    sigmas=spec["turbo"]["sigmas"], adapter_merged=False)
    model.tiling_config = TilingConfig()
    tokens = model.tokenizers["qwen21"].tokenizer(request["prompt"], add_special_tokens=False)["input_ids"]
    if len(tokens) > 1950:
        raise ValueError(f"Image prompt is too long ({len(tokens)} tokens); shorten the active scene")
    meta["prompt_tokens"] = len(tokens)
    timings = {}

    def memory(stage):
        meta.setdefault("memory_stages", []).append({"stage": stage, "active_bytes": mx.get_active_memory(),
                                                      "peak_bytes": mx.get_peak_memory()})

    class Stages:
        def call_before_loop(self, **kwargs):
            # Materialize before deleting weights: lazy prompt graphs otherwise retain the encoder.
            arrays = [array for pair in model.prompt_cache.values() for array in pair]
            mx.eval(*arrays)
            memory("encoded")
            model.text_encoder = None
            gc.collect()
            mx.clear_cache()
            memory("encoder_released")
            timings["denoise_start"] = time.perf_counter()
            meta["encode_and_reference_ms"] = (timings["denoise_start"] - timings["generation_start"]) * 1000

        def call_in_loop(self, **kwargs):
            mx.eval(kwargs["latents"])

        def call_after_loop(self, **kwargs):
            mx.eval(kwargs["latents"])
            meta["denoise_ms"] = (time.perf_counter() - timings["denoise_start"]) * 1000
            memory("denoised")
            model.transformer = None
            gc.collect()
            mx.clear_cache()
            memory("transformer_released")
            timings["decode_start"] = time.perf_counter()

    model.callbacks.register(Stages())
    reference = request.get("reference_path")
    if reference:
        from PIL import Image
        p = Path(reference)
        if not p.is_absolute() or not p.is_file():
            raise ValueError("Reference must be an existing local image")
        with Image.open(p) as img:
            if img.width * img.height > 16_000_000:
                raise ValueError("Reference image is too large")
            img.verify()
        meta["reference_mode"] = "image_to_image_composition"
    else:
        meta["reference_mode"] = "text_to_image"
    original_sigmas = LinearScheduler._get_sigmas
    if turbo:
        # Install the author's fixed schedule before latent initialization, including img2img.
        LinearScheduler._get_sigmas = lambda self: mx.array(spec["turbo"]["sigmas"], dtype=mx.float32)
    timings["generation_start"] = time.perf_counter()
    try:
        result = model.generate_image(seed=settings["seed"], prompt=request["prompt"],
                                      width=settings["width"], height=settings["height"],
                                      num_inference_steps=settings["steps"], guidance=1.0, scheduler="linear",
                                      image_path=reference, image_strength=settings.get("reference_strength", 0.6) if reference else None)
    finally:
        LinearScheduler._get_sigmas = original_sigmas
    mx.synchronize()
    meta["decode_ms"] = (time.perf_counter() - timings["decode_start"]) * 1000
    memory("decoded")
    meta["mlx_peak_bytes"] = mx.get_peak_memory()
    with output.open("xb") as stream:
        result.image.save(stream, format="PNG")
        stream.flush()
        os.fsync(stream.fileno())
    return json.dumps({"path": str(output), "width": result.image.width, "height": result.image.height})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path)
    parser.add_argument("--doctor", action="store_true")
    args = parser.parse_args()
    if args.doctor:
        import mlx.core as mx
        print(json.dumps({"mflux": importlib.metadata.version("mflux"), "mlx": importlib.metadata.version("mlx"),
                          "metal_available": mx.metal.is_available(), "device": mx.device_info()}, indent=2))
        return
    result = {"protocol_version": 1, "raw_text": "", "metadata": {"swap_before_bytes": swap_bytes()}, "error": None}
    started = time.perf_counter()
    try:
        payload = sys.stdin.buffer.read(128 * 1024 + 1)
        if len(payload) > 128 * 1024:
            raise ValueError("Image request exceeds 128 KiB")
        request = json.loads(payload)
        validate_request(request)
        if args.model is None:
            raise ValueError("--model is required")
        with contextlib.redirect_stdout(sys.stderr):
            result["raw_text"] = generate(args.model.resolve(), request, result["metadata"])
    except Exception as error:
        traceback.print_exc(file=sys.stderr)
        result["error"] = f"{type(error).__name__}: {error}"
    finally:
        result["metadata"].update(total_ms=(time.perf_counter() - started) * 1000, swap_after_bytes=swap_bytes(),
                                   peak_rss_bytes=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if sys.platform == "darwin" else 1024))
    print(json.dumps(result, ensure_ascii=False, allow_nan=False), flush=True)


if __name__ == "__main__":
    main()

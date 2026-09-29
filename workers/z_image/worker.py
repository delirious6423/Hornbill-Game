"""One offline Z-Image-Turbo request with the selected BennyDaBall encoder, then exit."""
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
    settings = request["settings"]
    for field in ("width", "height"):
        if type(settings[field]) is not int or not 256 <= settings[field] <= 768 or settings[field] % 32:
            raise ValueError("Image dimensions must be multiples of 32 between 256 and 768")
    if type(settings["steps"]) is not int or not 4 <= settings["steps"] <= 12:
        raise ValueError("Z-Image-Turbo uses 4–12 steps; 9 is the default")
    if type(settings["seed"]) is not int or not 0 <= settings["seed"] < 2**32:
        raise ValueError("Invalid seed")
    if type(settings["memory_limit_bytes"]) is not int or not 4 * 1024**3 <= settings["memory_limit_bytes"] <= 10 * 1024**3:
        raise ValueError("Image memory limit must be 4–10 GiB")
    strength = settings.get("reference_strength", 0.6)
    if type(strength) not in (int, float) or not math.isfinite(strength) or not 0.3 <= strength <= 1:
        raise ValueError("Reference strength must be 0.3–1")
    output = Path(request["output_path"])
    if not output.is_absolute() or output.suffix.lower() != ".png" or not output.parent.is_dir() or output.exists():
        raise ValueError("Output must be a new absolute PNG path in an existing directory")
    reference = request.get("reference_path")
    if reference and (not Path(reference).is_absolute() or not Path(reference).is_file()):
        raise ValueError("Reference must be an existing local image")
    return settings, output


def require_downloaded(path):
    # macOS SF_DATALESS marks an iCloud placeholder; opening it can block for minutes.
    if getattr(path.stat(), "st_flags", 0) & 0x40000000:
        raise ValueError(f"Model file is offloaded to iCloud: {path}. Keep the runtime downloaded before generating.")


def check_model(root):
    spec = json.loads(Path(__file__).with_name("models.json").read_text())
    manifest = root / "hornbill-model.json"
    if not manifest.is_file():
        raise ValueError("Z-Image weights are not installed; run ./scripts/image.sh download")
    require_downloaded(manifest)
    receipt = json.loads(manifest.read_text())
    if receipt.get("spec") != spec:
        raise ValueError("Image weights use a different manifest; run the explicit installer")
    for name, detail in spec["renderer"]["files"].items():
        path = root / "pipeline" / name
        if not path.is_file() or path.stat().st_size != detail["bytes"]:
            raise ValueError(f"Missing or incomplete renderer component: {path}")
        require_downloaded(path)
    converted = receipt.get("converted_files", {})
    if "model.safetensors.index.json" not in converted:
        raise ValueError("Missing locally converted BennyDaBall encoder")
    for name, detail in converted.items():
        if Path(name).name != name:
            raise ValueError("Invalid converted weight path")
        path = root / "pipeline/text_encoder" / name
        if not path.is_file() or path.stat().st_size != detail["bytes"]:
            raise ValueError(f"Missing or incomplete chosen encoder: {path}")
        require_downloaded(path)
    for name in ("tokenizer.json", "tokenizer_config.json", "chat_template.jinja"):
        path = root / "pipeline/tokenizer" / name
        if not path.is_file() or path.stat().st_size != spec["encoder"]["files"][name]["bytes"]:
            raise ValueError(f"Missing chosen encoder tokenizer: {path}")
        require_downloaded(path)
    index = json.loads((root / "pipeline/text_encoder/model.safetensors.index.json").read_text())
    if str(index["metadata"]["quantization_level"]) != "8" or not {
        "embed_tokens.weight", "embed_tokens.scales", "embed_tokens.biases"
    }.issubset(index["weight_map"]):
        raise ValueError("Expected the locally repacked Q8_0 GGUF encoder")
    return spec


def generate(root, request, meta):
    settings, output = validate_request(request)
    spec = check_model(root)
    import mlx.core as mx
    from mflux.models.common.config import ModelConfig
    from mflux.models.common.config.config import Config
    from mflux.models.common.latent_creator.latent_creator import Img2Img, LatentCreator
    from mflux.models.common.tokenizer import TokenizerLoader
    from mflux.models.common.vae.tiling_config import TilingConfig
    from mflux.models.common.vae.vae_util import VAEUtil
    from mflux.models.common.weights.loading.weight_applier import WeightApplier
    from mflux.models.common.weights.loading.weight_loader import WeightLoader
    from mflux.models.z_image.latent_creator import ZImageLatentCreator
    from mflux.models.z_image.model.z_image_text_encoder.prompt_encoder import PromptEncoder
    from mflux.models.z_image.model.z_image_text_encoder.text_encoder import TextEncoder
    from mflux.models.z_image.model.z_image_transformer.transformer import ZImageTransformer
    from mflux.models.z_image.model.z_image_vae.vae import VAE
    from mflux.models.z_image.variants.z_image import ZImage
    from mflux.models.z_image.weights.z_image_weight_definition import ZImageWeightDefinition
    from mflux.utils.image_util import ImageUtil

    if not mx.metal.is_available():
        raise RuntimeError("Z-Image backend requires Apple Silicon Metal")
    mx.set_memory_limit(settings["memory_limit_bytes"])
    mx.set_wired_limit(min(settings["memory_limit_bytes"], 8 * 1024**3))
    mx.set_cache_limit(0)
    mx.reset_peak_memory()
    meta.update(backend="mflux-z-image-turbo", model=spec["renderer"]["repo"],
                model_revision=spec["renderer"]["revision"], text_encoder=spec["encoder"]["repo"],
                text_encoder_revision=spec["encoder"]["revision"], encoder_quantization=spec["conversion"],
                model_license=spec["license"], mflux_revision=spec["mflux_revision"],
                mflux_version=importlib.metadata.version("mflux"), mlx_version=importlib.metadata.version("mlx"), settings=settings)
    components = {c.name: c for c in ZImageWeightDefinition.get_components()}

    def memory(stage):
        mx.synchronize()
        meta.setdefault("memory_stages", []).append({"stage": stage, "active_bytes": mx.get_active_memory(), "peak_bytes": mx.get_peak_memory()})

    def finite(value, stage):
        if not bool(mx.all(mx.isfinite(value)).item()):
            raise ValueError(f"Non-finite values during {stage}; no image was saved")

    def load(name, factory):
        started = time.perf_counter()
        weights = WeightLoader.load_single_local(component=components[name], root_path=root / "pipeline")
        model = factory()
        WeightApplier.apply_and_quantize_single(weights=weights, model=model, component=components[name],
                                               quantize_arg=8 if name == "text_encoder" else 4, quantization_predicate=ZImageWeightDefinition.quantization_predicate)
        del weights
        mx.eval(model.parameters())
        meta.setdefault("component_load_ms", {}).setdefault(name, []).append((time.perf_counter() - started) * 1000)
        return model

    tokenizer = TokenizerLoader.load_all(ZImageWeightDefinition.get_tokenizers(), str(root / "pipeline"))["z_image"]
    # The selected tokenizer defaults to left padding. MFLUX takes the first
    # valid-token count and uses absolute positions from zero; left padding also
    # creates fully masked causal rows whose NaNs contaminate later layers.
    tokenizer.tokenizer.padding_side = "right"
    formatted = tokenizer.tokenizer.apply_chat_template([{"role": "user", "content": request["prompt"]}],
                                                        tokenize=False, add_generation_prompt=True, **tokenizer.chat_template_kwargs)
    tokens = tokenizer.tokenizer(formatted, add_special_tokens=tokenizer.add_special_tokens, truncation=False)["input_ids"]
    if len(tokens) > tokenizer.max_length:
        raise ValueError(f"Image prompt has {len(tokens)} tokens; Z-Image supports {tokenizer.max_length}. Shorten the active scene; no text was silently truncated")
    meta["prompt_tokens"] = len(tokens)
    encoder = load("text_encoder", TextEncoder)
    started = time.perf_counter()
    encodings = PromptEncoder.encode_prompt(request["prompt"], tokenizer, encoder)
    mx.eval(encodings)
    finite(encodings, "text encoding")
    meta["encode_ms"] = (time.perf_counter() - started) * 1000
    memory("encoded")
    del encoder, tokenizer
    gc.collect()
    mx.clear_cache()
    memory("encoder_released")

    reference = request.get("reference_path")
    tiling = TilingConfig()
    config = Config(model_config=ModelConfig.z_image_turbo(), width=settings["width"], height=settings["height"],
                    num_inference_steps=settings["steps"], guidance=0.0, scheduler="linear",
                    image_path=reference, image_strength=settings.get("reference_strength", 0.6) if reference else None)
    started = time.perf_counter()
    if reference:
        from PIL import Image
        with Image.open(reference) as picture:
            if picture.width * picture.height > 16_000_000:
                raise ValueError("Reference image is too large")
            picture.verify()
        vae = load("vae", VAE)
        latents = LatentCreator.create_for_txt2img_or_img2img(seed=settings["seed"], width=config.width, height=config.height,
                    img2img=Img2Img(vae=vae, latent_creator=ZImageLatentCreator, image_path=reference,
                                   sigmas=config.scheduler.sigmas, init_time_step=config.init_time_step, tiling_config=tiling))
        mx.eval(latents)
        finite(latents, "reference encoding")
        del vae
        gc.collect()
        mx.clear_cache()
        meta["reference_mode"] = "image_to_image_composition"
    else:
        latents = ZImageLatentCreator.create_noise(settings["seed"], config.height, config.width)
        mx.eval(latents)
        finite(latents, "noise initialization")
        meta["reference_mode"] = "text_to_image"
    meta["reference_and_latents_ms"] = (time.perf_counter() - started) * 1000
    memory("latents_ready")

    transformer = load("transformer", ZImageTransformer)
    predict = ZImage._predict(transformer)
    started = time.perf_counter()
    steps = list(range(config.init_time_step, config.num_inference_steps))
    meta["denoise_updates"] = len(steps)
    meta["sigmas"] = config.scheduler.sigmas.tolist()
    noise = None
    for i, t in enumerate(steps):
        sigma = config.scheduler.sigmas[t].reshape((1,))
        noise = predict(latents=latents, timestep=mx.ones_like(sigma) - sigma, sigmas=config.scheduler.sigmas,
                        text_encodings=encodings, negative_encodings=None, guidance=0.0)
        latents = config.scheduler.step(noise=noise, timestep=t, latents=latents)
        mx.eval(latents)
        finite(latents, f"denoising step {i+1}")
        print(f"Z-Image denoise {i+1}/{len(steps)}", file=sys.stderr, flush=True)
    meta["denoise_ms"] = (time.perf_counter() - started) * 1000
    memory("denoised")
    # A compiled prediction closure also owns weights; release it before loading the decoder.
    del predict, transformer, noise, encodings
    gc.collect()
    mx.clear_cache()
    memory("transformer_released")

    vae = load("vae", VAE)
    started = time.perf_counter()
    unpacked = ZImageLatentCreator.unpack_latents(latents, config.height, config.width)
    decoded = VAEUtil.decode(vae=vae, latent=unpacked, tiling_config=tiling)
    mx.eval(decoded)
    finite(decoded, "image decoding")
    picture = ImageUtil.to_pil(decoded)
    meta["decode_ms"] = (time.perf_counter() - started) * 1000
    memory("decoded")
    meta["mlx_peak_bytes"] = mx.get_peak_memory()
    meta["load_ms"] = sum(sum(values) for values in meta["component_load_ms"].values())
    with output.open("xb") as stream:
        picture.save(stream, format="PNG")
        stream.flush()
        os.fsync(stream.fileno())
    return json.dumps({"path": str(output), "width": picture.width, "height": picture.height})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path)
    parser.add_argument("--doctor", action="store_true")
    args = parser.parse_args()
    if args.doctor:
        spec = check_model(args.model) if args.model else None
        import mlx.core as mx
        print(json.dumps({"mflux": importlib.metadata.version("mflux"), "mlx": importlib.metadata.version("mlx"),
                          "metal_available": mx.metal.is_available(), "device": mx.device_info(),
                          "image_model": spec["renderer"]["repo"] if spec else None,
                          "text_encoder": spec["encoder"]["repo"] if spec else None}, indent=2))
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

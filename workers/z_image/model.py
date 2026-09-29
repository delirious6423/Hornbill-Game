"""Explicit, pinned installation; the generation worker never downloads weights."""
import argparse
import fcntl
import gc
import hashlib
import importlib.metadata
import json
import os
import re
from pathlib import Path
import shutil
import time


def file_hash(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify_file(path, detail):
    if not path.is_file() or path.stat().st_size != detail["bytes"]:
        raise ValueError(f"Missing or incomplete model file: {path}")
    if detail.get("sha256") and file_hash(path) != detail["sha256"]:
        raise ValueError(f"Checksum mismatch: {path}")
    if detail.get("git_blob_sha1"):
        digest = hashlib.sha1(b"blob " + str(path.stat().st_size).encode() + b"\0" + path.read_bytes()).hexdigest()
        if digest != detail["git_blob_sha1"]:
            raise ValueError(f"Git blob checksum mismatch: {path}")


def encoder_shapes():
    shapes = {"embed_tokens.weight": (151936, 2560), "norm.weight": (2560,)}
    layer = {
        "input_layernorm.weight": (2560,), "post_attention_layernorm.weight": (2560,),
        "self_attn.q_proj.weight": (4096, 2560), "self_attn.k_proj.weight": (1024, 2560),
        "self_attn.v_proj.weight": (1024, 2560), "self_attn.o_proj.weight": (2560, 4096),
        "self_attn.q_norm.weight": (128,), "self_attn.k_norm.weight": (128,),
        "mlp.gate_proj.weight": (9728, 2560), "mlp.up_proj.weight": (9728, 2560),
        "mlp.down_proj.weight": (2560, 9728),
    }
    for i in range(36):
        shapes.update({f"layers.{i}.{k}": v for k, v in layer.items()})
    return shapes


def converted_key(name):
    direct = {"token_embd.weight": "embed_tokens.weight", "output_norm.weight": "norm.weight"}
    if name in direct:
        return direct[name]
    match = re.fullmatch(r"blk\.(\d+)\.(.+)\.weight", name)
    layer_names = {
        "attn_norm": "input_layernorm", "ffn_norm": "post_attention_layernorm",
        "attn_q": "self_attn.q_proj", "attn_k": "self_attn.k_proj",
        "attn_v": "self_attn.v_proj", "attn_output": "self_attn.o_proj",
        "attn_q_norm": "self_attn.q_norm", "attn_k_norm": "self_attn.k_norm",
        "ffn_gate": "mlp.gate_proj", "ffn_up": "mlp.up_proj", "ffn_down": "mlp.down_proj",
    }
    if not match or int(match[1]) >= 36 or match[2] not in layer_names:
        raise ValueError(f"Unexpected GGUF encoder weight: {name}")
    return f"layers.{int(match[1])}.{layer_names[match[2]]}.weight"


def pack_q8_0(data, shape):
    """Lossless Q8_0 block mapping: signed byte * d == unsigned byte * d - 128*d."""
    import numpy as np
    rows, columns = shape
    if columns % 32:
        raise ValueError("Q8_0 input width must be divisible by 32")
    blocks = np.asarray(data, dtype=np.uint8).reshape(rows, columns // 32, 34)
    scales = np.ascontiguousarray(blocks[:, :, :2]).view("<f2").reshape(rows, -1).astype(np.float32)
    if not np.all(np.isfinite(scales)):
        raise ValueError("Non-finite GGUF block scale")
    signed = np.ascontiguousarray(blocks[:, :, 2:]).view(np.int8)
    unsigned = (signed.astype(np.int16) + 128).astype(np.uint8)
    if not np.array_equal(unsigned.astype(np.int16) - 128, signed):
        raise ValueError("GGUF integer repacking changed a weight")
    packed = np.ascontiguousarray(unsigned.reshape(rows, columns)).view("<u4")
    # F32 scales/biases preserve GGUF's exact dequantized values, including cancellation.
    return packed, scales, -128.0 * scales


def convert_encoder(source, destination):
    """Keep the selected GGUF's Q8 blocks; adapt storage without dequantize/requantize."""
    import mlx.core as mx
    import numpy as np
    from gguf import GGUFReader, GGMLQuantizationType
    from gguf.quants import dequantize

    mx.set_default_device(mx.cpu)
    mx.set_cache_limit(0)
    destination.mkdir(parents=True, exist_ok=True)
    shapes = encoder_shapes()
    reader = GGUFReader(source)
    if reader.byte_order == "S":
        raise ValueError("Expected the pinned little-endian GGUF")
    mapped = {converted_key(t.name): t for t in reader.tensors}
    if set(mapped) != set(shapes):
        raise ValueError(f"GGUF encoder keys differ: missing={sorted(set(shapes)-set(mapped))[:3]}")
    shard, shard_bytes, shard_number, weight_map = {}, 0, 0, {}
    version = importlib.metadata.version("mflux")
    max_error = 0.0

    def flush():
        nonlocal shard, shard_bytes, shard_number
        if not shard:
            return
        name = f"{shard_number}.safetensors"
        temporary = destination / (name + ".tmp.safetensors")
        mx.save_safetensors(str(temporary), shard, {"quantization_level": "8", "mflux_version": version})
        os.replace(temporary, destination / name)
        weight_map.update({k: name for k in shard})
        shard, shard_bytes = {}, 0
        shard_number += 1
        gc.collect()

    for i, (key, tensor) in enumerate(sorted(mapped.items())):
        if tuple(reversed(tensor.shape.tolist())) != shapes[key]:
            raise ValueError(f"Unexpected GGUF tensor shape: {tensor.name}")
        if len(shapes[key]) == 2:
            if tensor.tensor_type != GGMLQuantizationType.Q8_0:
                raise ValueError(f"Expected exact Q8_0 matrix: {tensor.name}")
            packed, scales, biases = pack_q8_0(tensor.data, shapes[key])
            values = {key: mx.array(packed), key[:-6] + "scales": mx.array(scales), key[:-6] + "biases": mx.array(biases)}
            # Compare MLX's actual dequantizer with GGUF's reference implementation at both ends of every matrix.
            for row in (0, shapes[key][0] - 1):
                restored = mx.dequantize(values[key][row:row+1], values[key[:-6]+"scales"][row:row+1],
                                        values[key[:-6]+"biases"][row:row+1], group_size=32, bits=8)
                expected = dequantize(tensor.data.reshape(shapes[key][0], -1)[row:row+1], tensor.tensor_type)
                error = float(np.max(np.abs(np.array(restored) - expected)))
                max_error = max(max_error, error)
                if error != 0:
                    raise ValueError(f"Q8 repacking changed decoded values: {tensor.name}, error={error}")
            del packed, scales, biases, restored, expected
        else:
            if tensor.tensor_type not in (GGMLQuantizationType.F32, GGMLQuantizationType.F16):
                raise ValueError(f"Unexpected norm precision: {tensor.name}")
            values = {key: mx.array(tensor.data)}
        mx.eval(*values.values())
        size = sum(v.nbytes for v in values.values())
        if shard_bytes + size > 256 * 1024**2:
            flush()
        shard.update(values)
        shard_bytes += size
        del values
        if i % 40 == 0:
            print(f"Repacked GGUF tensors: {i+1}/{len(mapped)}", flush=True)
    flush()
    index = {"metadata": {"quantization_level": "8", "mflux_version": version}, "weight_map": weight_map}
    (destination / "model.safetensors.index.json").write_text(json.dumps(index, indent=2) + "\n")
    names = sorted(set(weight_map.values())) + ["model.safetensors.index.json"]
    files = {name: {"bytes": (destination / name).stat().st_size, "sha256": file_hash(destination / name)} for name in names}
    return files, {"matrices_checked": sum(len(v) == 2 for v in shapes.values()), "max_dequantization_error": max_error,
                   "source_format": "Q8_0", "repacked_bits": 8, "repacked_group_size": 32}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--lock", type=Path, required=True)
    args = parser.parse_args()
    spec = json.loads(Path(__file__).with_name("models.json").read_text())
    root = args.runtime / "models" / "z-image-turbo-benny"
    root.mkdir(parents=True, exist_ok=True)
    receipt_path = root / "hornbill-model.json"
    if receipt_path.is_file():
        receipt = json.loads(receipt_path.read_text())
        if receipt.get("spec") != spec:
            raise ValueError("Installed image manifest differs; preserve it and select a new model directory")
        for part, folder in (("renderer", "pipeline"), ("encoder", "encoder-source")):
            for name, detail in spec[part]["files"].items():
                verify_file(root / folder / name, detail)
        for name, detail in receipt["converted_files"].items():
            verify_file(root / "pipeline/text_encoder" / name, detail)
        print(f"Verified existing Z-Image installation: {root}", flush=True)
        return
    missing = sum(max(0, detail["bytes"] - ((root / folder / name).stat().st_size if (root / folder / name).is_file() else 0))
                  for part, folder in (("renderer", "pipeline"), ("encoder", "encoder-source"))
                  for name, detail in spec[part]["files"].items())
    # Includes the repacked Q8 encoder, a temporary embedding shard and a 2 GiB reserve.
    cached = sum(f.stat().st_size for f in (root / "pipeline/text_encoder").glob("*.safetensors"))
    required = missing + max(0, int(5.2 * 1024**3) - cached) + int(2.5 * 1024**3)
    if shutil.disk_usage(root).free < required:
        raise ValueError(f"Need {required/1024**3:.2f} GiB free for pinned downloads, conversion and reserve; no existing files removed")
    os.environ.setdefault("HF_XET_CHUNK_CACHE_SIZE_BYTES", "0")
    from huggingface_hub import snapshot_download
    for part, folder in (("renderer", "pipeline"), ("encoder", "encoder-source")):
        model = spec[part]
        print(f"Installing {model['repo']} at {model['revision']}", flush=True)
        snapshot_download(model["repo"], revision=model["revision"], local_dir=root / folder,
                          allow_patterns=list(model["files"]), max_workers=1)
        for name, detail in model["files"].items():
            verify_file(root / folder / name, detail)
    config = json.loads((root / "encoder-source/config.json").read_text())
    for k, expected in {"hidden_size": 2560, "num_hidden_layers": 36, "vocab_size": 151936, "head_dim": 128}.items():
        if config.get(k) != expected:
            raise ValueError(f"Incompatible encoder configuration: {k}")
    args.lock.parent.mkdir(parents=True, exist_ok=True)
    with args.lock.open("a+") as lease:
        try:
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError("An inference worker is active; rerun the installer when the app is idle") from error
        started = time.perf_counter()
        converted, verification = convert_encoder(root / "encoder-source" / spec["encoder"]["weight_file"], root / "pipeline/text_encoder")
    tokenizer = root / "pipeline/tokenizer"
    tokenizer.mkdir(exist_ok=True)
    for name in ("tokenizer.json", "tokenizer_config.json", "chat_template.jinja"):
        shutil.copyfile(root / "encoder-source" / name, tokenizer / name)
    shutil.copyfile(Path(__file__).with_name("APACHE-2.0.txt"), root / "APACHE-2.0.txt")
    receipt = {"spec": spec, "converted_files": converted, "conversion_seconds": time.perf_counter() - started,
               "verification": verification, "conversion_note": "Selected BennyDaBall Q8_0 GGUF repacked to MLX affine Q8, group size 32. No requantization or replacement encoder."}
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"Verified Z-Image weights and chosen encoder: {root}", flush=True)


if __name__ == "__main__":
    main()

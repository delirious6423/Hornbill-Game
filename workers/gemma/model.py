"""Explicit, online model installation; the inference worker never downloads."""
import argparse
import json
import os
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["download", "path", "use"])
    parser.add_argument("profile", choices=["12b", "e4b"], default="12b", nargs="?")
    args = parser.parse_args()
    runtime = Path(os.environ["HORNBILL_RUNTIME"])
    destination = runtime / "models" / args.profile
    spec = json.loads(Path(__file__).with_name("models.json").read_text())[args.profile]
    if args.command == "path":
        print(destination)
        return
    if args.command == "download":
        from huggingface_hub import snapshot_download
        destination.mkdir(parents=True, exist_ok=True)
        installed_bytes = sum(p.stat().st_size for p in destination.glob("*.safetensors"))
        needed = max(0, spec["weights_bytes"] - installed_bytes) + 1024**3
        if shutil.disk_usage(destination).free < needed:
            raise SystemExit(f"Need at least {needed / 1024**3:.1f} GiB free to install this model.")
        print(f"Downloading {spec['repo']} at {spec['revision']}", flush=True)
        snapshot_download(
            spec["repo"], revision=spec["revision"], local_dir=destination,
            allow_patterns=["*.json", "*.jinja", "*.safetensors", "README.md"],
            max_workers=2,
        )
        actual = sum(p.stat().st_size for p in destination.glob("*.safetensors"))
        if actual != spec["weights_bytes"]:
            raise SystemExit(f"Incomplete weights: expected {spec['weights_bytes']}, got {actual} bytes.")
        (destination / "hornbill-model.json").write_text(json.dumps(spec, indent=2) + "\n")
        if not (runtime / "active-model").exists():
            (runtime / "active-model").write_text(str(destination) + "\n")
    else:
        if not (destination / "hornbill-model.json").is_file():
            raise SystemExit("Download the model first.")
        (runtime / "active-model").write_text(str(destination) + "\n")
    print(destination)


if __name__ == "__main__":
    main()

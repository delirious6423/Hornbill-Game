"""Explicit online installation. Generation only reads these verified local files."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", type=Path, required=True)
    args = parser.parse_args()
    spec = json.loads(Path(__file__).with_name("models.json").read_text())
    root = args.runtime / "models" / "qwen21"
    root.mkdir(parents=True, exist_ok=True)
    missing = sum(
        max(0, detail["bytes"] - ((root / part / name).stat().st_size if (root / part / name).is_file() else 0))
        for part in ("base", "turbo") for name, detail in spec[part]["files"].items()
    )
    # Keep room for the OS and temporary saves. Never remove other models to fit.
    if shutil.disk_usage(root).free < missing + 2 * 1024**3:
        raise SystemExit(f"Need {missing / 1024**3 + 2:.1f} GiB free, including a 2 GiB reserve; no files removed.")
    from huggingface_hub import snapshot_download
    for part in ("base", "turbo"):
        model = spec[part]
        print(f"Installing {model['repo']} at {model['revision']}", flush=True)
        snapshot_download(model["repo"], revision=model["revision"], local_dir=root / part,
                          allow_patterns=list(model["files"]), max_workers=1)
        for name, detail in model["files"].items():
            path = root / part / name
            if path.stat().st_size != detail["bytes"]:
                raise RuntimeError(f"Incorrect size: {path}")
            if detail["sha256"]:
                with path.open("rb") as stream:
                    digest = hashlib.file_digest(stream, "sha256").hexdigest()
                if digest != detail["sha256"]:
                    raise RuntimeError(f"Checksum mismatch: {path}")
    (root / "hornbill-model.json").write_text(json.dumps(spec, indent=2) + "\n")
    print(f"Verified image weights: {root}")
    print("Qwen Image 2.1 weights retain their upstream qwen-research license; see base/LICENSE.")


if __name__ == "__main__":
    main()

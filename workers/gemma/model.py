"""Explicit, online model installation; the inference worker never downloads."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil


def verify_file(path, detail):
    if not path.is_file() or path.stat().st_size != detail['bytes']:
        raise ValueError(f'Missing or incomplete model file: {path}')
    if detail.get('sha256'):
        with path.open('rb') as f:
            actual = hashlib.file_digest(f, 'sha256').hexdigest()
        if actual != detail['sha256']:
            raise ValueError(f'Weight checksum mismatch: {path}')
    elif detail.get('git_blob_sha1'):
        raw = path.read_bytes()
        actual = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        if actual != detail['git_blob_sha1']:
            raise ValueError(f'Metadata checksum mismatch: {path}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["download", "path", "use"])
    parser.add_argument("profile", choices=["12b", "e4b"], default="12b", nargs="?")
    args = parser.parse_args()
    runtime = Path(os.environ["HORNBILL_RUNTIME"])
    spec = json.loads(Path(__file__).with_name("models.json").read_text())[args.profile]
    destination = runtime / "models" / spec['directory']
    receipt = destination / 'hornbill-model.json'
    if receipt.is_file() and json.loads(receipt.read_text()) != spec:
        raise SystemExit('Installed model differs from selected revision; preserve it and choose a new directory.')
    if args.command == "path":
        print(destination)
        return
    if args.command == "download":
        from huggingface_hub import snapshot_download
        destination.mkdir(parents=True, exist_ok=True)
        needed = sum(d['bytes'] for name,d in spec['files'].items()
                     if not (destination/name).is_file() or (destination/name).stat().st_size != d['bytes']) + 2*1024**3
        if shutil.disk_usage(destination).free < needed:
            raise SystemExit(f"Need at least {needed / 1024**3:.1f} GiB free to install this model.")
        print(f"Downloading {spec['repo']} at {spec['revision']}", flush=True)
        snapshot_download(
            spec["repo"], revision=spec["revision"], local_dir=destination,
            allow_patterns=list(spec['files']), max_workers=1,
        )
        for name, detail in spec['files'].items():
            verify_file(destination / name, detail)
        temporary = destination / 'hornbill-model.json.tmp'
        temporary.write_text(json.dumps(spec, indent=2) + "\n")
        temporary.replace(receipt)
        if not (runtime / "active-model").exists():
            (runtime / "active-model").write_text(str(destination) + "\n")
    else:
        if not receipt.is_file():
            raise SystemExit("Download the model first.")
        for name, detail in spec['files'].items():
            verify_file(destination / name, detail)
        (runtime / "active-model").write_text(str(destination) + "\n")
    print(destination)


if __name__ == "__main__":
    main()

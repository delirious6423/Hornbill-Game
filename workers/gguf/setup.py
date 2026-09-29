"""Pinned, optional llama.cpp + Gemma 4 Q4_K_M installation for this Mac."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import tarfile
import urllib.request

RELEASE = "b11247"
URL = f"https://github.com/ggml-org/llama.cpp/releases/download/{RELEASE}/llama-{RELEASE}-bin-macos-arm64.tar.gz"
ARCHIVE_SHA = "ebf1ccab751a0a972dfb06225bc716174dffb0cf780206c03802e620b21f96e0"
MODEL = {"repo":"unsloth/gemma-4-12b-it-GGUF", "revision":"fc034cfff751157913579611efad8462ac1be606",
         "filename":"gemma-4-12b-it-Q4_K_M.gguf", "size":7121861440,
         "sha256":"0a270ec9fe6b34f4a0d33992b6135117b484ebc4766ab76b51d4ae8c457e4c42"}


def sha256(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file,"sha256").hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command",choices=["setup","binary","model","path"])
    args=parser.parse_args()
    runtime=Path(os.environ["HORNBILL_RUNTIME"])
    target=runtime/"llama.cpp"/RELEASE
    model_dir=runtime/"models"/"gguf-12b"
    if args.command=="path":
        print(model_dir/MODEL["filename"])
        return
    if args.command in ("setup","binary"):
        target.mkdir(parents=True,exist_ok=True)
        archive=target/"release.tar.gz"
        if not archive.exists() or sha256(archive)!=ARCHIVE_SHA:
            print(f"Downloading llama.cpp {RELEASE}",flush=True)
            urllib.request.urlretrieve(URL,archive)
        if sha256(archive)!=ARCHIVE_SHA:
            raise SystemExit("llama.cpp archive checksum mismatch")
        with tarfile.open(archive) as tar:
            tar.extractall(target,filter="data")
        binaries=list(target.rglob("llama-server"))
        if not binaries:
            raise SystemExit("Release has no llama-server binary")
        (runtime/"llama-binary").write_text(str(binaries[0])+"\n")
        print(binaries[0])
    if args.command in ("setup","model"):
        from huggingface_hub import hf_hub_download
        model_dir.mkdir(parents=True,exist_ok=True)
        path=model_dir/MODEL["filename"]
        if not path.exists() and shutil.disk_usage(model_dir).free<MODEL["size"]+1024**3:
            raise SystemExit("Need at least 7.7 GiB free for the optional GGUF download")
        print("Downloading pinned Gemma 4 12B Q4_K_M",flush=True)
        path=Path(hf_hub_download(MODEL["repo"],MODEL["filename"],revision=MODEL["revision"],local_dir=model_dir))
        if path.stat().st_size!=MODEL["size"] or sha256(path)!=MODEL["sha256"]:
            raise SystemExit("GGUF checksum/size mismatch")
        path.with_suffix(".hornbill.json").write_text(json.dumps(MODEL,indent=2)+"\n")
        print(path)


if __name__=="__main__":
    main()

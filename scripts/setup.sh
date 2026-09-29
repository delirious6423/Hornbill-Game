#!/bin/sh
set -eu
. "$(dirname -- "$0")/env.sh"
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
    printf '%s\n' 'The MLX setup requires an Apple Silicon Mac.' >&2
    exit 1
fi
xcrun --show-sdk-path >/dev/null
mkdir -p "$HORNBILL_RUNTIME"
printf '%s\n' "$HORNBILL_RUNTIME" > "$HORNBILL_ROOT/.runtime-path"
if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
    curl --proto '=https' --tlsv1.2 -fL --retry 3 \
        https://static.rust-lang.org/rustup/dist/aarch64-apple-darwin/rustup-init \
        -o "$HORNBILL_RUNTIME/rustup-init"
    chmod +x "$HORNBILL_RUNTIME/rustup-init"
    "$HORNBILL_RUNTIME/rustup-init" -y --no-modify-path --profile minimal \
        --default-toolchain 1.98.1 --component rustfmt --component clippy
fi
if [ ! -x "$HORNBILL_RUNTIME/bootstrap/bin/uv" ]; then
    python3 -m venv "$HORNBILL_RUNTIME/bootstrap"
    "$HORNBILL_RUNTIME/bootstrap/bin/python" -m pip install \
        --disable-pip-version-check --no-cache-dir 'uv==0.12.20'
fi
HORNBILL_UV="$HORNBILL_RUNTIME/bootstrap/bin/uv"
"$HORNBILL_UV" python install 3.12
if [ ! -d "$HORNBILL_RUNTIME/venv" ]; then
    "$HORNBILL_UV" venv --python 3.12 "$HORNBILL_RUNTIME/venv"
fi
if [ -f "$HORNBILL_ROOT/workers/gemma/requirements.lock" ]; then
    "$HORNBILL_UV" pip sync --python "$HORNBILL_RUNTIME/venv/bin/python" \
        "$HORNBILL_ROOT/workers/gemma/requirements.lock"
else
    "$HORNBILL_UV" pip compile --python "$HORNBILL_RUNTIME/venv/bin/python" \
        "$HORNBILL_ROOT/workers/gemma/requirements.in" \
        -o "$HORNBILL_ROOT/workers/gemma/requirements.lock"
    "$HORNBILL_UV" pip sync --python "$HORNBILL_RUNTIME/venv/bin/python" \
        "$HORNBILL_ROOT/workers/gemma/requirements.lock"
fi
printf '%s\n' 'Tools installed. Next: ./scripts/model.sh download 12b'


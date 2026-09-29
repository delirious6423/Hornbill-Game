#!/bin/sh
# Sourced by the entry points. Never modifies the user's shell profile.
HORNBILL_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if [ -z "${HORNBILL_RUNTIME:-}" ]; then
    if [ -f "$HORNBILL_ROOT/.runtime-path" ]; then
        HORNBILL_RUNTIME=$(cat "$HORNBILL_ROOT/.runtime-path")
    else
        HORNBILL_RUNTIME="$HORNBILL_ROOT/.local"
    fi
fi
export HORNBILL_ROOT HORNBILL_RUNTIME
export CARGO_HOME="$HORNBILL_RUNTIME/cargo"
export RUSTUP_HOME="$HORNBILL_RUNTIME/rustup"
export CARGO_TARGET_DIR="$HORNBILL_RUNTIME/target"
export UV_CACHE_DIR="$HORNBILL_RUNTIME/uv-cache"
export UV_PYTHON_INSTALL_DIR="$HORNBILL_RUNTIME/python"
export HF_HOME="$HORNBILL_RUNTIME/huggingface"
export XDG_CACHE_HOME="$HORNBILL_RUNTIME/cache"
export PYTHONPYCACHEPREFIX="$HORNBILL_RUNTIME/pycache"
export PATH="$CARGO_HOME/bin:$HORNBILL_RUNTIME/venv/bin:$PATH"


#!/bin/sh
set -eu
. "$(dirname -- "$0")/env.sh"
export HORNBILL_PYTHON="${HORNBILL_PYTHON:-$HORNBILL_RUNTIME/venv/bin/python}"
export HORNBILL_DATA_DIR="${HORNBILL_DATA_DIR:-$HORNBILL_ROOT/data}"
if [ -z "${HORNBILL_LLAMA_BINARY:-}" ] && [ -f "$HORNBILL_RUNTIME/llama-binary" ]; then
    HORNBILL_LLAMA_BINARY=$(cat "$HORNBILL_RUNTIME/llama-binary")
    export HORNBILL_LLAMA_BINARY
fi
if [ -z "${HORNBILL_MODEL:-}" ] && [ -f "$HORNBILL_RUNTIME/active-model" ]; then
    HORNBILL_MODEL=$(cat "$HORNBILL_RUNTIME/active-model")
    export HORNBILL_MODEL
fi
cd "$HORNBILL_ROOT"
exec cargo run --locked --quiet --release -- "$@"

#!/bin/sh
set -eu
. "$(dirname -- "$0")/env.sh"
"$HORNBILL_RUNTIME/venv/bin/python" "$HORNBILL_ROOT/workers/gemma/worker.py" --doctor

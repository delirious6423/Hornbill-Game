#!/bin/sh
set -eu
. "$(dirname -- "$0")/env.sh"
exec "$HORNBILL_RUNTIME/venv/bin/python" "$HORNBILL_ROOT/workers/gemma/model.py" "$@"

#!/bin/sh
set -eu
. "$(dirname -- "$0")/env.sh"
command=${1:-doctor}
case "$command" in
  setup)
    "$HORNBILL_RUNTIME/bootstrap/bin/uv" venv --python "$HORNBILL_RUNTIME/venv/bin/python" "$HORNBILL_RUNTIME/image-venv"
    exec "$HORNBILL_RUNTIME/bootstrap/bin/uv" pip install --python "$HORNBILL_RUNTIME/image-venv/bin/python" -r "$HORNBILL_ROOT/workers/qwen_image/requirements.lock"
    ;;
  download)
    exec "$HORNBILL_RUNTIME/venv/bin/python" "$HORNBILL_ROOT/workers/qwen_image/model.py" --runtime "$HORNBILL_RUNTIME"
    ;;
  doctor)
    exec "$HORNBILL_RUNTIME/image-venv/bin/python" "$HORNBILL_ROOT/workers/qwen_image/worker.py" --doctor
    ;;
  *) echo 'Usage: ./scripts/image.sh setup|download|doctor' >&2; exit 2 ;;
esac

#!/bin/sh
set -eu
. "$(dirname -- "$0")/env.sh"
cd "$HORNBILL_ROOT"
exec cargo "$@"

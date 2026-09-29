#!/bin/sh
set -eu
. "$(dirname -- "$0")/env.sh"
cd "$HORNBILL_ROOT"
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
"$HORNBILL_RUNTIME/venv/bin/python" -m unittest discover -s workers/tests -v

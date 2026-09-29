#!/bin/sh
# Run this from a normal macOS Terminal when the Codex sandbox hides Metal.
set -eu
. "$(dirname -- "$0")/env.sh"
"$HORNBILL_ROOT/scripts/doctor.sh"
"$HORNBILL_RUNTIME/venv/bin/python" - <<'PY'
import fcntl, os, pathlib, time
path=pathlib.Path(os.environ['HORNBILL_ROOT'])/'data/worker.lock'
path.parent.mkdir(parents=True,exist_ok=True)
with path.open('a') as lock:
    deadline=time.monotonic()+900
    notified=False
    while True:
        try:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            break
        except BlockingIOError:
            if not notified:
                print('Waiting for the current Hornbill worker to finish...',flush=True)
                notified=True
            if time.monotonic()>deadline:
                raise SystemExit('A Hornbill worker is still running. Try again when it finishes.')
            time.sleep(0.5)
PY
HORNBILL_VERIFY_SAVE="verify_$(date +%Y%m%d_%H%M%S)"
"$HORNBILL_ROOT/hornbill" new --save "$HORNBILL_VERIFY_SAVE" --title 'Local Gemma verification'
"$HORNBILL_ROOT/hornbill" turn --save "$HORNBILL_VERIFY_SAVE" \
    --action 'I inspect the radio and ask Mira what she recognizes about the transmission.'
"$HORNBILL_ROOT/hornbill" turn --save "$HORNBILL_VERIFY_SAVE" \
    --action 'I ask Mira to explain her reasoning while I look for a practical way into the observatory.'
"$HORNBILL_ROOT/hornbill" export --save "$HORNBILL_VERIFY_SAVE" \
    --output "$HORNBILL_ROOT/data/$HORNBILL_VERIFY_SAVE.json"
printf '\nVerification saved in %s\n' "$HORNBILL_ROOT/data/$HORNBILL_VERIFY_SAVE.json"

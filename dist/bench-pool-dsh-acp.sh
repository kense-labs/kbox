#!/bin/sh
# Warm-pool startup benchmark for dsh-acp (one-shot, --deferred + --exec).
#
# Measures end-to-end latency of `dsh-acp --version` on warm-pool hits:
# VM lease + agent process start + command output. Run 5 times, waiting
# for an idle pre-booted VM before each run.
#
#   sh dist/bench-pool-dsh-acp.sh [path-to-kbox]
set -eu

KBOX="${1:-kbox}"
IMAGE="ghcr.io/kense-labs/dsh-acp:latest"
RUNS=5

idle() {
  "$KBOX" pool status --json | python3 -c \
    "import json,sys
try: print(json.load(sys.stdin)[0]['idle'])
except Exception: print(0)"
}

wait_idle() {
  n=0
  while [ "$(idle)" -lt 1 ]; do
    n=$((n + 1))
    [ "$n" -le 60 ] || { echo "timed out waiting for an idle VM" >&2; exit 1; }
    sleep 4
  done
}

"$KBOX" pool start --deferred --image "$IMAGE" --size 1 --max 2 >/dev/null 2>&1 &
DAEMON=$!
trap '"$KBOX" pool stop >/dev/null 2>&1 || true' EXIT INT TERM

echo "warming pool for $IMAGE"
wait_idle

i=1
while [ "$i" -le "$RUNS" ]; do
  wait_idle
  start=$(python3 -c 'import time; print(time.monotonic())')
  "$KBOX" pool run --exec -- node /app/apps/cli/lib/bin.js --version >/dev/null
  end=$(python3 -c 'import time; print(time.monotonic())')
  python3 -c "print(f'hit run $i: {($end - $start) * 1000:.0f} ms')"
  i=$((i + 1))
done
echo "pool stopped"

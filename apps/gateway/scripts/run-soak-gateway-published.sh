#!/usr/bin/env bash
# Publish dedicated-hardware capacity envelopes (Enterprise GA).
# Fail closed unless required env pins are set. Default duration is 7200s (2h)
# per profile — do not run this on a laptop and claim GA.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

: "${KRUN_GATEWAY_DEDICATED_RUNNER:?set to 1 on a dedicated runner}"
: "${KRUN_GATEWAY_HW_HOST:?set dedicated host name}"
: "${KRUN_GATEWAY_HW_CPU_MODEL:?set CPU model string}"
: "${KRUN_GATEWAY_HW_MEMORY_GB:?set RAM in GiB}"

if [[ "${KRUN_GATEWAY_DEDICATED_RUNNER}" != "1" ]]; then
  echo "refuse: KRUN_GATEWAY_DEDICATED_RUNNER must be 1" >&2
  exit 2
fi

BIN="${1:-./target/release/krun-gateway}"
if [[ ! -x "$BIN" && ! -f "$BIN" ]]; then
  echo "building release krun-gateway..."
  cargo build --locked --release --bin krun-gateway
  BIN=./target/release/krun-gateway
fi

DUR="${KRUN_GATEWAY_SOAK_DURATION:-7200}"
C="${KRUN_GATEWAY_SOAK_CONCURRENCY:-16}"

for profile in http-json sse-finite openai-json openai-sse; do
  python3 scripts/soak-gateway.py \
    --bin "$BIN" \
    --profile "$profile" \
    --duration "$DUR" \
    --concurrency "$C" \
    --envelope-status published
done

echo "soak-gateway published: OK (update docs/ops/capacity-envelope-draft.md and ROADMAP with these JSON files)"

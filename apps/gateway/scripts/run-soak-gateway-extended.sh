#!/usr/bin/env bash
# Lab-extended soak (default 120s). Not a dedicated-hardware published envelope.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${1:-./target/debug/krun-gateway}"
if [[ ! -x "$BIN" && ! -f "$BIN" ]]; then
  echo "building krun-gateway..."
  cargo build --bin krun-gateway
  BIN=./target/debug/krun-gateway
fi

DUR="${KRUN_GATEWAY_SOAK_DURATION:-120}"
C="${KRUN_GATEWAY_SOAK_CONCURRENCY:-8}"

for profile in http-json sse-finite openai-json openai-sse; do
  python3 scripts/soak-gateway.py \
    --bin "$BIN" \
    --profile "$profile" \
    --duration "$DUR" \
    --concurrency "$C" \
    --envelope-status lab-extended
done
echo "soak-gateway lab-extended: OK"

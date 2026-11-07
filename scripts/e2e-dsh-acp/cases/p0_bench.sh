#!/usr/bin/env bash
# P0-1: upstream process-prewarm greeting bench with RUNS=1 under isolated K_HOME.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

CASE_ID="P0-1"
if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  e2e_record_result "$CASE_ID" "BLOCKED" "DEEPSEEK_API_KEY not set"
  exit 0
fi

BENCH_URL="${BENCH_URL:-https://raw.githubusercontent.com/kense-labs/dsh-acp/main/scripts/bench-pool-dsh-acp.sh}"
BENCH_LOCAL="$E2E_OUT/bench-pool-dsh-acp.sh"
e2e_log "fetching bench script"
curl -fsSL "$BENCH_URL" -o "$BENCH_LOCAL"
chmod +x "$BENCH_LOCAL"
# Upstream wait_idle defaults to ~180s; cold rootfs on isolated K_HOME often needs longer.
perl -pi -e 's/\[ "\$n" -le 90 \]/[ "\$n" -le 180 ]/' "$BENCH_LOCAL"

export RUNS="${RUNS:-1}"
export DSH_MODEL="${DSH_MODEL:-deepseek-flash}"
export DSH_REASONING_EFFORT="${DSH_REASONING_EFFORT:-low}"
export IMAGE="${IMAGE:-ghcr.io/kense-labs/dsh-acp:latest}"
export POOL_LOG="$E2E_OUT/p0-bench-pool.log"

# Ensure default pool socket from a prior aborted run is not wedged.
"$KBOX" pool stop >/dev/null 2>&1 || true

set +e
sh "$BENCH_LOCAL" "$KBOX" "${GREETING:-你好！请用一句话介绍你自己。}" >"$E2E_OUT/p0-bench.stdout" 2>"$E2E_OUT/p0-bench.stderr"
rc=$?
set -e

if [[ $rc -eq 0 ]]; then
  e2e_record_result "$CASE_ID" "PASS" "bench RUNS=$RUNS succeeded" \
    "$(jq -nc --arg log "$POOL_LOG" '{pool_log:$log}')"
else
  e2e_record_result "$CASE_ID" "FAIL" "bench exited $rc (see p0-bench.stderr)" \
    "$(jq -nc --argjson rc "$rc" '{exit_code:$rc}')"
  exit 1
fi

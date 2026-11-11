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

export RUNS="${RUNS:-1}"
export DSH_MODEL="${DSH_MODEL:-deepseek-flash}"
export DSH_REASONING_EFFORT="${DSH_REASONING_EFFORT:-low}"
export IMAGE="${IMAGE:-ghcr.io/kense-labs/dsh-acp:latest}"
export POOL_LOG="$E2E_OUT/p0-bench-pool.log"

# The upstream client fixes its image and socket; adapt only those CLI arguments.
export E2E_BENCH_KBOX="$KBOX"
export E2E_BENCH_IMAGE="$IMAGE"
export E2E_BENCH_SOCKET="$E2E_OUT/p0-pool.sock"
BENCH_KBOX="$E2E_OUT/bench-kbox"
cat >"$BENCH_KBOX" <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == pool ]]; then
  subcommand="$2"
  shift 2
  if [[ "$subcommand" == start ]]; then
    args=("$@")
    for i in "${!args[@]}"; do
      if [[ "${args[$i]}" == --image ]]; then
        args[$((i + 1))]="$E2E_BENCH_IMAGE"
      fi
    done
    exec "$E2E_BENCH_KBOX" pool "$subcommand" "${args[@]}" --socket "$E2E_BENCH_SOCKET"
  fi
  exec "$E2E_BENCH_KBOX" pool "$subcommand" "$@" --socket "$E2E_BENCH_SOCKET"
fi
exec "$E2E_BENCH_KBOX" "$@"
BASH
chmod +x "$BENCH_KBOX"

set +e
sh "$BENCH_LOCAL" "$BENCH_KBOX" "${GREETING:-你好！请用一句话介绍你自己。}" >"$E2E_OUT/p0-bench.stdout" 2>"$E2E_OUT/p0-bench.stderr"
rc=$?
set -e

if [[ $rc -eq 0 ]]; then
  e2e_record_result "$CASE_ID" "PASS" "bench RUNS=$RUNS succeeded" \
    "$(jq -nc --arg log "$POOL_LOG" --arg image "$IMAGE" --arg socket "$E2E_BENCH_SOCKET" '{pool_log:$log,image:$image,socket:$socket}')"
else
  e2e_record_result "$CASE_ID" "FAIL" "bench exited $rc (see p0-bench.stderr)" \
    "$(jq -nc --argjson rc "$rc" '{exit_code:$rc}')"
  exit 1
fi

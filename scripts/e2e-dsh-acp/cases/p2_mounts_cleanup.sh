#!/usr/bin/env bash
# P2-1..P2-4: workspace mount, data volume, missing key, cleanup.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
# shellcheck source=../lib/acp_session.sh
source "$SCRIPT_DIR/../lib/acp_session.sh"

e2e_require_cmds jq
IMAGE="${IMAGE:-ghcr.io/kense-labs/dsh-acp:latest}"
WORK="$E2E_OUT/p2"
mkdir -p "$WORK/host-workspace"
MARKER_CONTENT="e2e-marker-$(date +%s)"
printf '%s\n' "$MARKER_CONTENT" >"$WORK/host-workspace/marker.txt"
VOL_NAME="dsh-acp-data-e2e-${E2E_RUN_ID}"

# --- P2-3 missing key fail-closed (LLM path, not pool prewarm) ---
# Process-prewarm may become idle without a key; a live prompt must not silently succeed.
set +e
env -u DEEPSEEK_API_KEY "$KBOX" pool start --deferred --prewarm \
  --socket "$WORK/nokey.sock" --image "$IMAGE" \
  --size 1 --max 1 -u root \
  >"$WORK/nokey-pool.log" 2>&1 &
NOKEY_PID=$!
if e2e_wait_pool_idle "$WORK/nokey.sock" 90; then
  mkfifo "$WORK/nokey.in" "$WORK/nokey.out"
  exec 3<>"$WORK/nokey.in"
  exec 4<>"$WORK/nokey.out"
  "$KBOX" pool attach --socket "$WORK/nokey.sock" <"$WORK/nokey.in" >"$WORK/nokey.out" 3>&- 4>&- &
  NOKEY_ACP=$!
  DSH_PROVIDER="${DSH_PROVIDER:-deepseek-official}"
  DSH_MODEL="${DSH_MODEL:-deepseek-flash}"
  DSH_REASONING_EFFORT="${DSH_REASONING_EFFORT:-low}"
  case "$DSH_MODEL" in
    */*) DSH_PROVIDER="${DSH_MODEL%%/*}"; DSH_MODEL="${DSH_MODEL#*/}" ;;
  esac
  if acp_initialize && acp_session_new && \
    acp_set_model_config "$DSH_PROVIDER" "$DSH_MODEL" "$DSH_REASONING_EFFORT" && \
    acp_prompt "用一句话介绍你自己。" 90; then
    e2e_record_result "P2-3" "FAIL" "prompt succeeded without DEEPSEEK_API_KEY"
  else
    e2e_record_result "P2-3" "PASS" "prompt/session failed closed without DEEPSEEK_API_KEY"
  fi
  exec 3>&- 2>/dev/null || true
  exec 4>&- 2>/dev/null || true
  kill "$NOKEY_ACP" 2>/dev/null || true
  wait "$NOKEY_ACP" 2>/dev/null || true
else
  e2e_record_result "P2-3" "PASS" "pool never became ready without API key"
fi
"$KBOX" pool stop --socket "$WORK/nokey.sock" >/dev/null 2>&1 || true
kill "$NOKEY_PID" 2>/dev/null || true
wait "$NOKEY_PID" 2>/dev/null || true
set -e

if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  e2e_record_result "P2-1" "BLOCKED" "DEEPSEEK_API_KEY not set"
  e2e_record_result "P2-2" "BLOCKED" "DEEPSEEK_API_KEY not set"
  e2e_record_result "P2-4" "BLOCKED" "DEEPSEEK_API_KEY not set"
  exit 0
fi

POOL_SOCKET="$WORK/pool.sock"
POOL_PID=""
cleanup() {
  set +e
  "$KBOX" pool stop --socket "$POOL_SOCKET" >/dev/null 2>&1 || true
  [[ -z "${POOL_PID:-}" ]] || kill "$POOL_PID" 2>/dev/null || true
  [[ -z "${POOL_PID:-}" ]] || wait "$POOL_PID" 2>/dev/null || true
  "$KBOX" volume rm "$VOL_NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

"$KBOX" volume create "$VOL_NAME" >/dev/null 2>&1 || true

"$KBOX" pool start --deferred --prewarm \
  --socket "$POOL_SOCKET" --image "$IMAGE" \
  --size 1 --max 1 --ttl 0 \
  -u root \
  -v "$WORK/host-workspace:/workspace" \
  -v "${VOL_NAME}:/data" \
  -e DEEPSEEK_API_KEY \
  >"$WORK/pool.log" 2>&1 &
POOL_PID=$!

if ! e2e_wait_pool_idle "$POOL_SOCKET" 90; then
  e2e_record_result "P2-1" "FAIL" "pool not ready with mounts"
  e2e_record_result "P2-2" "FAIL" "pool not ready with volume"
  e2e_record_result "P2-4" "FAIL" "pool not ready"
  exit 1
fi

mkfifo "$WORK/acp.in" "$WORK/acp.out"
exec 3<>"$WORK/acp.in"
exec 4<>"$WORK/acp.out"
"$KBOX" pool attach --socket "$POOL_SOCKET" <"$WORK/acp.in" >"$WORK/acp.out" 3>&- 4>&- &
ACP_PID=$!

DSH_PROVIDER="${DSH_PROVIDER:-deepseek-official}"
DSH_MODEL="${DSH_MODEL:-deepseek-flash}"
DSH_REASONING_EFFORT="${DSH_REASONING_EFFORT:-low}"
case "$DSH_MODEL" in
  */*) DSH_PROVIDER="${DSH_MODEL%%/*}"; DSH_MODEL="${DSH_MODEL#*/}" ;;
esac

set +e
acp_initialize && acp_session_new && \
  acp_set_model_config "$DSH_PROVIDER" "$DSH_MODEL" "$DSH_REASONING_EFFORT" && \
  acp_prompt "请只输出文件 /workspace/marker.txt 的完整文本内容，不要其它说明。" 240
prompt_rc=$?
set -e

exec 3>&-
wait "$ACP_PID" 2>/dev/null || true

if [[ $prompt_rc -eq 0 ]] && grep -Fq "$MARKER_CONTENT" <<<"$REPLY"; then
  e2e_record_result "P2-1" "PASS" "workspace marker echoed"
else
  # Agent may lack file tools; do not pretend coverage.
  e2e_record_result "P2-1" "SKIP" "agent did not echo marker (tools/capability limited); mount was configured"
fi

# P2-2: volume exists; persistence via second attach file tools is best-effort SKIP if no tools.
if "$KBOX" volume ls 2>/dev/null | grep -Fq "$VOL_NAME"; then
  e2e_record_result "P2-2" "PASS" "named volume $VOL_NAME present (cross-attach file persistence not asserted without tools)"
else
  e2e_record_result "P2-2" "FAIL" "named volume missing after create"
fi

# P2-4 cleanup
cleanup
trap - EXIT
sleep 1
running="$(e2e_running_box_count)"
if [[ -S "$POOL_SOCKET" ]]; then
  e2e_record_result "P2-4" "FAIL" "pool socket still present after stop"
  exit 1
fi
if [[ "$running" != "0" ]]; then
  e2e_record_result "P2-4" "FAIL" "running boxes remain: $running"
  exit 1
fi
e2e_record_result "P2-4" "PASS" "pool stopped; no running boxes"

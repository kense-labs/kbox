#!/usr/bin/env bash
# P1-1..P1-6: core ACP over process-prewarm pool (single pool, up to 3 attaches).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
# shellcheck source=../lib/acp_session.sh
source "$SCRIPT_DIR/../lib/acp_session.sh"

if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  for id in P1-1 P1-2 P1-3 P1-4 P1-5 P1-6; do
    e2e_record_result "$id" "BLOCKED" "DEEPSEEK_API_KEY not set"
  done
  exit 0
fi

e2e_require_cmds jq
IMAGE="${IMAGE:-ghcr.io/kense-labs/dsh-acp:latest}"
DSH_PROVIDER="${DSH_PROVIDER:-deepseek-official}"
DSH_MODEL="${DSH_MODEL:-deepseek-flash}"
DSH_REASONING_EFFORT="${DSH_REASONING_EFFORT:-low}"
case "$DSH_MODEL" in
  */*) DSH_PROVIDER="${DSH_MODEL%%/*}"; DSH_MODEL="${DSH_MODEL#*/}" ;;
esac
GREETING="${GREETING:-你好！请用一句话介绍你自己。}"

WORK="$E2E_OUT/p1-core"
mkdir -p "$WORK"
POOL_SOCKET="$WORK/pool.sock"
POOL_PID=""

cleanup_pool() {
  set +e
  "$KBOX" pool stop --socket "$POOL_SOCKET" >/dev/null 2>&1 || true
  [[ -z "${POOL_PID:-}" ]] || kill "$POOL_PID" 2>/dev/null || true
  [[ -z "${POOL_PID:-}" ]] || wait "$POOL_PID" 2>/dev/null || true
}
trap cleanup_pool EXIT

"$KBOX" pool start --deferred --prewarm \
  --socket "$POOL_SOCKET" --image "$IMAGE" \
  --size 1 --max 2 --ttl 0 \
  -u root -e DEEPSEEK_API_KEY \
  >"$WORK/pool.log" 2>&1 &
POOL_PID=$!

if ! e2e_wait_pool_idle "$POOL_SOCKET" 90; then
  e2e_record_result "P1-6" "FAIL" "pool never became idle (prewarm ready)"
  for id in P1-1 P1-2 P1-3 P1-4 P1-5; do
    e2e_record_result "$id" "FAIL" "blocked by prewarm failure"
  done
  exit 1
fi
e2e_record_result "P1-6" "PASS" "idle>=1 after prewarm"

run_one_session() {
  local label="$1" do_prompt="${2:-1}"
  local fifoin="$WORK/${label}.in" fifoout="$WORK/${label}.out"
  rm -f "$fifoin" "$fifoout"
  mkfifo "$fifoin" "$fifoout"
  local acp_pid=""
  exec 3<>"$fifoin"
  exec 4<>"$fifoout"
  "$KBOX" pool attach --socket "$POOL_SOCKET" <"$fifoin" >"$fifoout" 3>&- 4>&- &
  acp_pid=$!

  acp_initialize || return 1
  local agent proto
  agent="$(jq -r '.result.agentInfo.name // "unknown"' <<<"$RPC_RESPONSE")"
  proto="$(jq -r '.result.protocolVersion // empty' <<<"$RPC_RESPONSE")"
  # Repeat initialize once for P1-1 on first session.
  if [[ "$label" == "s1" ]]; then
    acp_initialize || return 1
  fi
  acp_session_new || return 1
  acp_set_model_config "$DSH_PROVIDER" "$DSH_MODEL" "$DSH_REASONING_EFFORT" || return 1
  if [[ "$do_prompt" == "1" ]]; then
    acp_prompt "$GREETING" 240 || return 1
  fi
  exec 3>&-
  wait "$acp_pid"
  printf '%s\n' "$agent|$proto|$SESSION_ID|$STOP_REASON|${#REPLY}"
}

set +e
out1="$(run_one_session s1 1)"
rc1=$?
set -e
if [[ $rc1 -ne 0 ]]; then
  e2e_record_result "P1-1" "FAIL" "initialize/session failed on first attach"
  e2e_record_result "P1-2" "FAIL" "see P1-1"
  e2e_record_result "P1-3" "FAIL" "see P1-1"
  e2e_record_result "P1-4" "FAIL" "see P1-1"
  e2e_record_result "P1-5" "FAIL" "see P1-1"
  exit 1
fi

IFS='|' read -r agent proto session stop reply_len <<<"$out1"
e2e_record_result "P1-1" "PASS" "initialize (+repeat) agent=$agent protocol=$proto"
e2e_record_result "P1-2" "PASS" "sessionId=$session"
e2e_record_result "P1-3" "PASS" "model config echoed"
e2e_record_result "P1-4" "PASS" "prompt end_turn reply_len=$reply_len stop=$stop"

# Wait for replenish then two more attaches (P1-5).
ok=1
for n in 2 3; do
  if ! e2e_wait_pool_idle "$POOL_SOCKET" 90; then
    ok=0
    break
  fi
  # Brief settle so replenished VM stdio is fully ready for ACP.
  sleep 3
  set +e
  run_one_session "s$n" 1 >/dev/null
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    ok=0
    break
  fi
done
if [[ $ok -eq 1 ]]; then
  e2e_record_result "P1-5" "PASS" "three sequential attaches succeeded"
else
  e2e_record_result "P1-5" "FAIL" "sequential attach/replenish failed"
  exit 1
fi

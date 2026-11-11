#!/usr/bin/env bash
# P3-1..P3-4: disconnect, pool exhaustion, bad image, tools probe.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
# shellcheck source=../lib/acp_session.sh
source "$SCRIPT_DIR/../lib/acp_session.sh"

e2e_require_cmds jq
IMAGE="${IMAGE:-ghcr.io/kense-labs/dsh-acp:latest}"
WORK="$E2E_OUT/p3"
mkdir -p "$WORK"

# --- P3-3 bad image / never ready ---
BAD_SOCK="$WORK/bad.sock"
set +e
"$KBOX" pool start --deferred --prewarm \
  --socket "$BAD_SOCK" --image "ghcr.io/kense-labs/does-not-exist-e2e:latest" \
  --size 1 --max 1 -u root \
  >"$WORK/bad-pool.log" 2>&1 &
BAD_PID=$!
if e2e_wait_pool_idle "$BAD_SOCK" 8; then
  e2e_record_result "P3-3" "FAIL" "bad image unexpectedly became idle"
else
  e2e_record_result "P3-3" "PASS" "bad image did not become idle within timeout"
fi
"$KBOX" pool stop --socket "$BAD_SOCK" >/dev/null 2>&1 || true
kill "$BAD_PID" 2>/dev/null || true
wait "$BAD_PID" 2>/dev/null || true
set -e

if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  for id in P3-1 P3-2 P3-4; do
    e2e_record_result "$id" "BLOCKED" "DEEPSEEK_API_KEY not set"
  done
  exit 0
fi

POOL_SOCKET="$WORK/pool.sock"
POOL_PID=""
cleanup() {
  set +e
  "$KBOX" pool stop --socket "$POOL_SOCKET" >/dev/null 2>&1 || true
  [[ -z "${POOL_PID:-}" ]] || kill "$POOL_PID" 2>/dev/null || true
  [[ -z "${POOL_PID:-}" ]] || wait "$POOL_PID" 2>/dev/null || true
}
trap cleanup EXIT

"$KBOX" pool start --deferred --prewarm \
  --socket "$POOL_SOCKET" --image "$IMAGE" \
  --size 1 --max 1 --ttl 0 \
  -u root -e DEEPSEEK_API_KEY \
  >"$WORK/pool.log" 2>&1 &
POOL_PID=$!

if ! e2e_wait_pool_idle "$POOL_SOCKET" 90; then
  e2e_record_result "P3-1" "FAIL" "pool not ready"
  e2e_record_result "P3-2" "FAIL" "pool not ready"
  e2e_record_result "P3-4" "FAIL" "pool not ready"
  exit 1
fi

# --- P3-1 client disconnect mid-session ---
mkfifo "$WORK/d.in" "$WORK/d.out"
exec 3<>"$WORK/d.in"
exec 4<>"$WORK/d.out"
"$KBOX" pool attach --socket "$POOL_SOCKET" <"$WORK/d.in" >"$WORK/d.out" 3>&- 4>&- &
ACP_PID=$!
if acp_initialize; then
  exec 3>&-
  kill "$ACP_PID" 2>/dev/null || true
  wait "$ACP_PID" 2>/dev/null || true
  if e2e_wait_pool_idle "$POOL_SOCKET" 90; then
    e2e_record_result "P3-1" "PASS" "disconnect destroyed session; pool replenished"
  else
    e2e_record_result "P3-1" "FAIL" "pool did not replenish after disconnect"
  fi
else
  e2e_record_result "P3-1" "FAIL" "initialize before disconnect failed"
fi

# --- P3-2 pool exhaustion size=1 ---
if ! e2e_wait_pool_idle "$POOL_SOCKET" 90; then
  e2e_record_result "P3-2" "FAIL" "no idle slot for exhaustion test"
else
  mkfifo "$WORK/a.in" "$WORK/a.out" "$WORK/b.in" "$WORK/b.out"
  # Open both ends of each FIFO (O_RDWR) so attach open(2) never blocks forever.
  exec 3<>"$WORK/a.in"
  exec 4<>"$WORK/a.out"
  exec 5<>"$WORK/b.in"
  exec 6<>"$WORK/b.out"
  "$KBOX" pool attach --socket "$POOL_SOCKET" <"$WORK/a.in" >"$WORK/a.out" 3>&- 4>&- 5>&- 6>&- &
  A_PID=$!
  if ! acp_initialize; then
    e2e_record_result "P3-2" "FAIL" "first attach did not initialize"
    exit 1
  fi
  set +e
  "$KBOX" pool attach --socket "$POOL_SOCKET" <"$WORK/b.in" >"$WORK/b.out" 3>&- 4>&- 5>&- 6>&- &
  B_PID=$!
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}' >&5
  if IFS= read -r -t 8 -u 6 second_reply; then
    b_ok=0
    b_msg="second attach received protocol output while size=1 was busy"
    kill "$B_PID" 2>/dev/null || true
    wait "$B_PID" 2>/dev/null || true
  elif kill -0 "$B_PID" 2>/dev/null; then
    kill "$B_PID" 2>/dev/null || true
    wait "$B_PID" 2>/dev/null || true
    b_ok=1
    b_msg="second attach blocked while size=1 busy"
  else
    wait "$B_PID" 2>/dev/null
    b_rc=$?
    # Immediate error is also acceptable fail-closed behavior under exhaustion.
    [[ "$b_rc" -ne 0 ]] && b_ok=1 || b_ok=0
    b_msg="second attach exited rc=$b_rc while size=1 busy"
  fi
  set -e
  exec 3>&- || true
  exec 4>&- || true
  exec 5>&- || true
  exec 6>&- || true
  kill "$A_PID" 2>/dev/null || true
  wait "$A_PID" 2>/dev/null || true
  if [[ "${b_ok:-0}" -eq 1 ]]; then
    e2e_record_result "P3-2" "PASS" "$b_msg"
  else
    e2e_record_result "P3-2" "FAIL" "exhaustion check failed"
  fi
fi

# --- P3-4 real tool lifecycle, independent of capability advertisements ---
if e2e_wait_pool_idle "$POOL_SOCKET" 90; then
  mkfifo "$WORK/t.in" "$WORK/t.out"
  exec 3<>"$WORK/t.in"
  exec 4<>"$WORK/t.out"
  "$KBOX" pool attach --socket "$POOL_SOCKET" <"$WORK/t.in" >"$WORK/t.out" 3>&- 4>&- &
  T_PID=$!
  DSH_PROVIDER="${DSH_PROVIDER:-deepseek-official}"
  DSH_MODEL="${DSH_MODEL:-deepseek-flash}"
  case "$DSH_MODEL" in
    */*) DSH_PROVIDER="${DSH_MODEL%%/*}"; DSH_MODEL="${DSH_MODEL#*/}" ;;
  esac
  ACP_ALLOW_ONCE=1
  if acp_initialize && acp_session_new &&
    acp_set_model_config "$DSH_PROVIDER" "$DSH_MODEL" "${DSH_REASONING_EFFORT:-low}" &&
    acp_prompt "请实际使用 bash 工具执行 node -p '7*13'，然后给出执行结果，不要心算代替工具。" 240 &&
    [[ "${TOOL_CALL_COUNT:-0}" -ge 1 && "${TOOL_COMPLETE_COUNT:-0}" -ge 1 ]] &&
    [[ "$REPLY" == *91* ]]; then
    e2e_record_result "P3-4" "PASS" "real tool call and completed update observed; result=91"
  else
    e2e_record_result "P3-4" "FAIL" "tool execution did not complete with the expected result"
  fi
  exec 3>&-
  kill "$T_PID" 2>/dev/null || true
  wait "$T_PID" 2>/dev/null || true
else
  e2e_record_result "P3-4" "FAIL" "no idle after P3-2"
fi

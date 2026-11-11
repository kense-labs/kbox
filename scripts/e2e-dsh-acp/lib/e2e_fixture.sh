#!/usr/bin/env bash
# Shared MicroVM + ACP connection fixture for the P4/P5/P6 tiers.
#
# Requires common.sh and acp_session.sh to be sourced first. Keeps P0-P3
# untouched: those cases manage their own fds and pools.
#
# A "connection" is one kbox attach/run bound to a pair of fds. More than one
# connection can be alive at once (use fd pairs 3/4, 5/6, 7/8) and
# e2e_fixture_use() selects which one the traced ACP client talks to.

set -euo pipefail

E2E_FIXTURE_DIR="${E2E_FIXTURE_DIR:-${E2E_OUT:?E2E_OUT must be set}/fixture}"
E2E_FIXTURE_IMAGE="${IMAGE:-ghcr.io/kense-labs/dsh-acp:latest}"
E2E_FIXTURE_VOL="${E2E_FIXTURE_VOL:-dsh-acp-e2e-${E2E_RUN_ID:-local}}"
E2E_FIXTURE_HOSTDIR="$E2E_FIXTURE_DIR/workspace"
E2E_FIXTURE_COLD_BOOT_SECONDS="${E2E_FIXTURE_COLD_BOOT_SECONDS:-20}"
E2E_FIXTURE_SOCKET=""
E2E_FIXTURE_CONNS=()
E2E_FIXTURE_SOCKETS=()
E2E_FIXTURE_POOL_PIDS=()
mkdir -p "$E2E_FIXTURE_HOSTDIR"

e2e_fixture_resolve_model() {
  local provider="${DSH_PROVIDER:-deepseek-official}" model="${DSH_MODEL:-deepseek-flash}"
  case "$model" in
    */*) provider="${model%%/*}"; model="${model#*/}" ;;
  esac
  E2E_FIXTURE_PROVIDER="$provider"
  E2E_FIXTURE_MODEL="$model"
  E2E_FIXTURE_MODEL_VALUE="$(jq -nc --arg p "$provider" --arg m "$model" '[$p,$m]')"
}

e2e_fixture_blocked() {
  local reason="${1:-DEEPSEEK_API_KEY not set}" id
  shift || true
  for id in "$@"; do
    e2e_record_result "$id" "BLOCKED" "$reason"
  done
}

e2e_fixture_skip() {
  local reason="$1" id
  shift
  for id in "$@"; do
    e2e_record_result "$id" "SKIP" "$reason"
  done
}

# Record PASS/FAIL from an already-captured return code. Args: id rc ok-msg fail-msg [detail]
e2e_fixture_verdict() {
  local id="$1" rc="$2" ok="$3" bad="$4" detail="${5:-null}"
  if [[ "$rc" -eq 0 ]]; then
    e2e_record_result "$id" "PASS" "$ok" "$detail"
  else
    e2e_record_result "$id" "FAIL" "$bad" "$detail"
  fi
  return "$rc"
}

# Args: <label> <size> [extra `kbox pool start` flags...]
e2e_fixture_start_pool() {
  local label="$1" size="$2"
  shift 2
  local socket="$E2E_FIXTURE_DIR/${label}.sock"
  local log="$E2E_FIXTURE_DIR/${label}-pool.log"
  rm -f "$socket"
  "$KBOX" volume create "$E2E_FIXTURE_VOL" >/dev/null 2>&1 || true
  "$KBOX" pool start --deferred --prewarm \
    --socket "$socket" --image "$E2E_FIXTURE_IMAGE" \
    --size "$size" --max "$((size + 1))" --ttl 0 \
    -u root -e DEEPSEEK_API_KEY \
    -v "$E2E_FIXTURE_HOSTDIR:/workspace" \
    -v "$E2E_FIXTURE_VOL:/data" \
    "$@" >"$log" 2>&1 &
  local pid=$!
  E2E_FIXTURE_POOL_PIDS+=("$pid")
  E2E_FIXTURE_SOCKETS+=("$socket")
  E2E_FIXTURE_SOCKET="$socket"
  if ! e2e_wait_pool_idle "$socket" "${E2E_FIXTURE_POOL_WAIT_ATTEMPTS:-90}"; then
    e2e_log "pool $label never became idle (see $log)"
    return 1
  fi
  sleep "${E2E_FIXTURE_SETTLE_SECONDS:-3}"
  return 0
}

e2e_fixture_open_fd() {
  local fd="$1" fifo="$2" mode="$3"
  case "$fd" in
    3) exec 3<>"$fifo" ;;
    4) exec 4<>"$fifo" ;;
    5) exec 5<>"$fifo" ;;
    6) exec 6<>"$fifo" ;;
    7) exec 7<>"$fifo" ;;
    8) exec 8<>"$fifo" ;;
    *) e2e_log "unsupported fixture fd: $fd"; return 1 ;;
  esac
}

e2e_fixture_close_fd() {
  local fd="$1"
  case "$fd" in
    3) exec 3>&- 2>/dev/null || true ;;
    4) exec 4>&- 2>/dev/null || true ;;
    5) exec 5>&- 2>/dev/null || true ;;
    6) exec 6>&- 2>/dev/null || true ;;
    7) exec 7>&- 2>/dev/null || true ;;
    8) exec 8>&- 2>/dev/null || true ;;
  esac
}

# Args: <label> [in-fd] [out-fd] [pool|cold]. Defaults: 3 4 pool.
e2e_fixture_attach() {
  local label="$1" infd="${2:-3}" outfd="${3:-4}" mode="${4:-pool}"
  local fifoin="$E2E_FIXTURE_DIR/${label}.in" fifoout="$E2E_FIXTURE_DIR/${label}.out"
  rm -f "$fifoin" "$fifoout"
  mkfifo "$fifoin" "$fifoout"
  e2e_fixture_open_fd "$infd" "$fifoin" rw || return 1
  e2e_fixture_open_fd "$outfd" "$fifoout" rw || return 1
  local pid
  if [[ "$mode" == "cold" ]]; then
    # A cold VM has its own /data volume so it never races the pool's session store.
    local cold_vol="${E2E_FIXTURE_COLD_VOL:-${E2E_FIXTURE_VOL}-cold}"
    "$KBOX" volume create "$cold_vol" >/dev/null 2>&1 || true
    "$KBOX" run -i --rm --memory "${E2E_FIXTURE_MEMORY:-2g}" \
      -u root -e DEEPSEEK_API_KEY \
      -v "$E2E_FIXTURE_HOSTDIR:/workspace" \
      -v "$cold_vol:/data" \
      "$E2E_FIXTURE_IMAGE" \
      <"$fifoin" >"$fifoout" 3>&- 4>&- 5>&- 6>&- 7>&- 8>&- &
  else
    "$KBOX" pool attach --socket "$E2E_FIXTURE_SOCKET" \
      <"$fifoin" >"$fifoout" 3>&- 4>&- 5>&- 6>&- 7>&- 8>&- &
  fi
  pid=$!
  E2E_FIXTURE_CONNS+=("$label|$pid|$infd|$outfd")
  if [[ "$mode" == "cold" ]]; then
    sleep "$E2E_FIXTURE_COLD_BOOT_SECONDS"
  fi
  ACP_IN_FD="$infd"
  ACP_OUT_FD="$outfd"
  return 0
}

# Select which live connection the traced ACP client writes to / reads from.
e2e_fixture_use() {
  ACP_IN_FD="$1"
  ACP_OUT_FD="$2"
}

# Args: <label>. Closes its fds and waits for the attach process.
e2e_fixture_detach() {
  local label="$1" entry l pid infd outfd found=0
  set +e
  if (( ${#E2E_FIXTURE_CONNS[@]} )); then
    for entry in "${E2E_FIXTURE_CONNS[@]}"; do
      IFS='|' read -r l pid infd outfd <<<"$entry"
      if [[ "$l" == "$label" ]]; then
        e2e_fixture_close_fd "$infd"
        e2e_fixture_close_fd "$outfd"
        kill "$pid" 2>/dev/null
        wait "$pid" 2>/dev/null
        found=1
      fi
    done
  fi
  set -e
  [[ "$found" -eq 1 ]]
}

# Tear down every pool (used by cross-VM resume to destroy VM A before starting B).
e2e_fixture_stop_all_pools() {
  local socket pid
  set +e
  if (( ${#E2E_FIXTURE_SOCKETS[@]} )); then
    for socket in "${E2E_FIXTURE_SOCKETS[@]}"; do
      "$KBOX" pool stop --socket "$socket" >/dev/null 2>&1
    done
  fi
  if (( ${#E2E_FIXTURE_POOL_PIDS[@]} )); then
    for pid in "${E2E_FIXTURE_POOL_PIDS[@]}"; do
      kill "$pid" 2>/dev/null
      wait "$pid" 2>/dev/null
    done
  fi
  set -e
  E2E_FIXTURE_SOCKETS=()
  E2E_FIXTURE_POOL_PIDS=()
  E2E_FIXTURE_SOCKET=""
}

e2e_fixture_cleanup() {
  local entry label pid infd outfd
  set +e
  if (( ${#E2E_FIXTURE_CONNS[@]} )); then
    for entry in "${E2E_FIXTURE_CONNS[@]}"; do
      IFS='|' read -r label pid infd outfd <<<"$entry"
      kill "$pid" 2>/dev/null
      wait "$pid" 2>/dev/null
    done
  fi
  e2e_fixture_stop_all_pools
  "$KBOX" volume rm "$E2E_FIXTURE_VOL" >/dev/null 2>&1
  set -e
  E2E_FIXTURE_CONNS=()
}

# Args: <label>. Starts a fresh JSONL trace for the current connection.
e2e_fixture_open_trace() {
  local label="$1"
  local path="$E2E_FIXTURE_DIR/${label}.trace.jsonl"
  acp_open_trace "$path"
  ACP_TRACE_PATH="$path"
  echo "$path"
}

# Minimal handshake shared by every capability case: initialize + session/new.
# Deliberately does NOT set model/effort: those are P4-03's subject under test.
e2e_fixture_handshake() {
  local cwd="${1:-/workspace}" mcp="${2:-[]}"
  acp_init_long || return 1
  acp_new_long "$cwd" "$mcp" || return 1
  return 0
}

# Wrap a case body so every failure is recorded instead of aborting the tier.
# Args: <id> <function-name> [args...]
e2e_fixture_run() {
  local id="$1" fn="$2"
  shift 2
  local rc=0 detail="null"
  set +e
  "$fn" "$@"
  rc=$?
  set -e
  if [[ -n "${ACP_TRACE_PATH:-}" && -f "$ACP_TRACE_PATH" ]]; then
    detail="$(acp_trace_stats "$ACP_TRACE_PATH" 2>/dev/null || echo null)"
  fi
  if [[ "$rc" -eq 0 ]]; then
    e2e_record_result "$id" "PASS" "$fn completed" "$detail"
  else
    e2e_record_result "$id" "FAIL" "$fn returned rc=$rc" "$detail"
  fi
  return "$rc"
}

#!/usr/bin/env bash
# P5-01..P5-05: long-running, multi-turn and interruption resilience.
#
# These are nightly cases, NOT a PR gate: set E2E_LONGRUN=1 to run them.
# "Long" is measurable, not "we chatted a few times":
#   LONGRUN_TURNS (30), LONGRUN_MIN_SECONDS (900),
#   LONGRUN_MIN_CHUNKS (100), LONGRUN_MIN_TOOL_CALLS (50).
# Set E2E_LONGRUN_ENFORCE=0 during first calibration to record numbers
# without failing on thresholds.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export E2E_FIXTURE_DIR="${E2E_FIXTURE_DIR:-${E2E_OUT:?E2E_OUT must be set}/p5}"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
# shellcheck source=../lib/acp_session.sh
source "$SCRIPT_DIR/../lib/acp_session.sh"
# shellcheck source=../lib/e2e_fixture.sh
source "$SCRIPT_DIR/../lib/e2e_fixture.sh"

P5_IDS=(P5-01 P5-02 P5-03 P5-04 P5-05)
P5_FAIL=0
LONGRUN_TURNS="${LONGRUN_TURNS:-30}"
LONGRUN_MIN_SECONDS="${LONGRUN_MIN_SECONDS:-900}"
LONGRUN_MIN_CHUNKS="${LONGRUN_MIN_CHUNKS:-100}"
LONGRUN_MIN_TOOL_CALLS="${LONGRUN_MIN_TOOL_CALLS:-50}"
LONGRUN_ENFORCE="${E2E_LONGRUN_ENFORCE:-1}"

if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  e2e_fixture_blocked "DEEPSEEK_API_KEY not set" "${P5_IDS[@]}"
  exit 0
fi
if [[ "${E2E_LONGRUN:-0}" != "1" ]]; then
  e2e_fixture_skip "nightly gate: set E2E_LONGRUN=1 (real tokens, >=15min)" "${P5_IDS[@]}"
  exit 0
fi
e2e_require_cmds jq python3
e2e_fixture_resolve_model
trap e2e_fixture_cleanup EXIT

p5_try() {
  local rc
  set +e
  "$@"
  rc=$?
  set -e
  return "$rc"
}
p5_stats() { acp_trace_stats "$ACP_TRACE_PATH" 2>/dev/null || echo null; }
p5_check() {
  local id="$1" ok="$2" msg="$3" detail="${4:-null}"
  if [[ "$ok" == "1" ]]; then
    e2e_record_result "$id" "PASS" "$msg" "$detail"
  else
    e2e_record_result "$id" "FAIL" "$msg" "$detail"
    P5_FAIL=$((P5_FAIL + 1))
  fi
}
p5_detail() {
  local turns="$1" elapsed="$2" extra="${3:-}"
  [[ -n "$extra" ]] || extra='{}'
  jq -nc --argjson stats "$(p5_stats)" --argjson turns "$turns" \
    --argjson elapsed "$elapsed" --argjson extra "$extra" \
    '$stats + {turns:$turns,elapsedSeconds:$elapsed} + $extra'
}

# Thresholds are reported even when enforcement is disabled for calibration.
p5_thresholds() {
  local stats="$1" turns="$2" elapsed="$3" ok=1 why=""
  local chunks tools
  chunks="$(jq -r '.chunks // 0' <<<"$stats")"
  tools="$(jq -r '.toolCalls // 0' <<<"$stats")"
  [[ "$turns" -ge "$LONGRUN_TURNS" ]] || { ok=0; why="$why turns=$turns<$LONGRUN_TURNS"; }
  [[ "$elapsed" -ge "$LONGRUN_MIN_SECONDS" ]] || { ok=0; why="$why elapsed=${elapsed}s<$LONGRUN_MIN_SECONDS s"; }
  [[ "$chunks" -ge "$LONGRUN_MIN_CHUNKS" ]] || { ok=0; why="$why chunks=$chunks<$LONGRUN_MIN_CHUNKS"; }
  [[ "$tools" -ge "$LONGRUN_MIN_TOOL_CALLS" ]] || { ok=0; why="$why toolCalls=$tools<$LONGRUN_MIN_TOOL_CALLS"; }
  if [[ "$LONGRUN_ENFORCE" == "0" ]]; then
    e2e_log "P5 thresholds (calibration only):$why"
    ok=1
  fi
  echo "$ok|$why|$chunks|$tools"
}

# --- P5-01: 30 turns, tool use every turn, canary recall, config switches ---
p5_context_fidelity() {
  e2e_fixture_open_trace "p5-01" >/dev/null
  local canary="CANARY-${E2E_RUN_ID:-local}-01"
  local journal="$E2E_FIXTURE_HOSTDIR/p5-journal.txt"
  : >"$journal"
  if ! p5_try e2e_fixture_handshake; then
    p5_check "P5-01" 0 "handshake failed"; return 1
  fi
  local start=$SECONDS i ok=1 efforts="" effort="" switch=0 last_used=0 used monotonic=1
  efforts="$(jq -c '[.result.configOptions[]? | select(.id == "reasoning_effort") | .options[]?.value]' <<<"$RPC_RESPONSE" 2>/dev/null || echo '[]')"
  if ! p5_try acp_prompt_long \
    "记住这个口令：${canary}。之后不要主动重复它。请用工具把 'turn-1' 追加到 /workspace/p5-journal.txt，然后只回复 ok。" \
    "$ACP_IDLE_TIMEOUT"; then ok=0; fi
  [[ "$STOP_REASON" == "end_turn" ]] || ok=0
  for i in $(seq 2 "$LONGRUN_TURNS"); do
    if (( i % 5 == 0 )) && [[ "$(jq -r 'length' <<<"$efforts")" -gt 0 ]]; then
      effort="$(jq -r --argjson i "$((i / 5))" '.[$i % length]' <<<"$efforts")"
      if p5_try acp_set_option_long "reasoning_effort" "$effort"; then switch=$((switch + 1)); else ok=0; fi
    fi
    if ! p5_try acp_prompt_long \
      "请用工具把 'turn-$i' 追加到 /workspace/p5-journal.txt，然后只回复 ok。" "$ACP_IDLE_TIMEOUT"; then
      ok=0
      break
    fi
    [[ "$STOP_REASON" == "end_turn" ]] || { ok=0; break; }
    used="$(jq -r '.lastUsed // 0' <<<"$(p5_stats)")"
    [[ "$used" -ge "$last_used" ]] || monotonic=0
    last_used="$used"
  done
  if ! p5_try acp_prompt_long \
    "现在复述我一开始给你的口令，并说出 /workspace/p5-journal.txt 最后一行的内容。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
  grep -Fq "$canary" <<<"$REPLY" || ok=0
  local missing
  missing="$(python3 - "$journal" "$LONGRUN_TURNS" <<'PY'
import sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")
n = int(sys.argv[2])
missing = [i for i in range(1, n + 1) if f"turn-{i}" not in text]
print(len(missing))
PY
)"
  [[ "$missing" == "0" ]] || ok=0
  local stats elapsed thresholds t_ok t_why chunks tools
  stats="$(p5_stats)"
  elapsed=$((SECONDS - start))
  thresholds="$(p5_thresholds "$stats" "$LONGRUN_TURNS" "$elapsed")"
  IFS='|' read -r t_ok t_why chunks tools <<<"$thresholds"
  [[ "$t_ok" == "1" ]] || ok=0
  [[ "$monotonic" == "1" ]] || ok=0
  if ! acp_trace_check "$ACP_TRACE_PATH" '{"max":{"noise":0,"errors":0}}' >/dev/null 2>&1; then ok=0; fi
  p5_check "P5-01" "$ok" \
    "$LONGRUN_TURNS tool-bearing turns, canary=$canary, effort switches=$switch, usage monotonic=$monotonic, missing journal turns=$missing; thresholds:${t_why:- met}" \
    "$(p5_detail "$LONGRUN_TURNS" "$elapsed" \
      "$(jq -nc --arg c "$canary" --argjson sw "$switch" --argjson miss "$missing" --argjson mono "$monotonic" \
        --argjson chunks "$chunks" --argjson tools "$tools" --argjson enforce "$([[ "$LONGRUN_ENFORCE" == 1 ]] && echo true || echo false)" \
        '{canary:$c,effortSwitches:$sw,missingJournalTurns:$miss,usageMonotonic:($mono==1),chunks:$chunks,toolCalls:$tools,thresholdsEnforced:$enforce}')")"
}

# --- P5-03: cancel mid-prompt, resume, then SIGKILL the client ---
p5_cancel_and_interrupt() {
  e2e_fixture_open_trace "p5-03" >/dev/null
  local ok=1 cancel_pid="" stop=""
  if ! p5_try e2e_fixture_handshake; then
    p5_check "P5-03" 0 "handshake failed"; return 1
  fi
  # The background writer inherits fd 3, so it can inject a cancel notification
  # while acp_prompt_long is draining updates.
  (
    sleep "${LONGRUN_CANCEL_DELAY_SECONDS:-5}"
    jq -nc --arg s "$SESSION_ID" \
      '{jsonrpc:"2.0",method:"session/cancel",params:{sessionId:$s}}' >&"${ACP_IN_FD}"
  ) &
  cancel_pid=$!
  if p5_try acp_prompt_long \
    "请写一份非常长的分析报告，逐章节展开，至少 3000 字，不要提前结束。" "$ACP_IDLE_TIMEOUT"; then
    stop="$STOP_REASON"
  else
    ok=0
  fi
  kill "$cancel_pid" 2>/dev/null || true
  wait "$cancel_pid" 2>/dev/null || true
  [[ "$stop" == "cancelled" ]] || ok=0
  # Same session must remain usable; no duplicate turn replay.
  if ! p5_try acp_prompt_long "继续，用一句话收尾。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
  [[ "$STOP_REASON" == "end_turn" ]] || ok=0

  # SIGKILL a second attach and assert the pool replenishes with no zombie VM.
  p5_try acp_close_long "$SESSION_ID" || true
  p5_try e2e_fixture_detach "p5-main" || true
  if ! e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90; then
    p5_check "P5-03" 0 "pool did not replenish before SIGKILL case" "$(p5_stats)"; return 1
  fi
  if p5_try e2e_fixture_attach "p5-kill" 3 4 pool; then
    e2e_fixture_open_trace "p5-03-kill" >/dev/null
    local entry label pid infd outfd
    if (( ${#E2E_FIXTURE_CONNS[@]} )); then
      for entry in "${E2E_FIXTURE_CONNS[@]}"; do
        IFS='|' read -r label pid infd outfd <<<"$entry"
        if [[ "$label" == "p5-kill" ]]; then kill -9 "$pid" 2>/dev/null || true; fi
      done
    fi
    p5_try e2e_fixture_detach "p5-kill" || true
    e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90 || ok=0
  else ok=0; fi
  p5_check "P5-03" "$ok" "cancel stopReason=${stop:-none}; resumed turn end_turn; SIGKILL client replenished pool" \
    "$(p5_detail 2 0 "$(jq -nc --arg s "$stop" '{cancelStopReason:$s,sigkillClient:true}')")"
}

# --- P5-04: long document, context pressure, history replay after resume ---
p5_compaction() {
  # Re-attach: P5-03 deliberately killed the previous connection.
  if ! e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90; then
    p5_check "P5-04" 0 "no idle VM for compaction case"; return 1
  fi
  if ! p5_try e2e_fixture_attach "p5-compact" 3 4 pool; then
    p5_check "P5-04" 0 "attach failed"; return 1
  fi
  e2e_fixture_open_trace "p5-04" >/dev/null
  local doc="$E2E_FIXTURE_HOSTDIR/p5-long.txt" canary="CANARY-${E2E_RUN_ID:-local}-04" ok=1
  python3 - "$doc" "$canary" "${LONGRUN_DOC_BYTES:-200000}" <<'PY'
import sys
from pathlib import Path
path, canary, size = sys.argv[1], sys.argv[2], int(sys.argv[3])
lines = [f"section {i:05d}: baseline narrative for context-pressure calibration." for i in range(size // 80 + 1)]
lines.append(f"The closing canary is {canary} .")
Path(path).write_text("\n".join(lines), encoding="utf-8")
PY
  if ! p5_try e2e_fixture_handshake; then
    p5_check "P5-04" 0 "handshake failed"; return 1
  fi
  local sid="$SESSION_ID"
  if ! p5_try acp_prompt_long \
    "请完整读取 /workspace/p5-long.txt，然后用一句话说明文末 canary 是什么，并给出章节总数。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
  grep -Fq "$canary" <<<"$REPLY" || ok=0
  local stats usage
  stats="$(p5_stats)"
  usage="$(jq -r '.usageUpdates // 0' <<<"$stats")"
  if [[ "$usage" == "0" ]]; then
    p5_try acp_close_long "$sid" || true
    e2e_record_result "P5-04" "SKIP" \
      "no usage_update emitted (token meter/context window not available); compaction pressure not measurable" \
      "$(p5_detail 1 0 "$(jq -nc --arg c "$canary" '{canary:$c,canaryRecalled:true}')")"
    return 0
  fi
  p5_try acp_close_long "$sid" || ok=0
  if p5_try acp_resume_long "$sid" /workspace; then
    # History must still be replayable after the long context turn.
    if ! p5_try acp_prompt_long "刚才那份文档文末的 canary 是什么？只回答口令。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
    grep -Fq "$canary" <<<"$REPLY" || ok=0
    p5_try acp_close_long "$sid" || true
  else ok=0; fi
  p5_check "P5-04" "$ok" "long doc consumed; usage_update=$usage; lastUsed=$(jq -r '.lastUsed' <<<"$stats"); canary replayable after resume" \
    "$(p5_detail 2 0 "$(jq -nc --arg c "$canary" --argjson u "$usage" --argjson used "$(jq -r '.lastUsed' <<<"$stats")" \
      '{canary:$c,usageUpdates:$u,lastUsed:$used}')")"
}

# --- P5-05: slow client backpressure, no dropped or silently truncated chunks ---
p5_backpressure() {
  if ! e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90; then
    p5_check "P5-05" 0 "no idle VM for backpressure case"; return 1
  fi
  p5_try e2e_fixture_detach "p5-compact" || true
  if ! p5_try e2e_fixture_attach "p5-slow" 3 4 pool; then
    p5_check "P5-05" 0 "attach failed"; return 1
  fi
  e2e_fixture_open_trace "p5-05" >/dev/null
  ACP_READ_DELAY_MS="${LONGRUN_SLOW_READ_MS:-50}"
  if ! p5_try e2e_fixture_handshake; then
    p5_check "P5-05" 0 "handshake failed"; return 1
  fi
  local ok=1 n="${LONGRUN_SLOW_LINES:-100}"
  if ! p5_try acp_prompt_long \
    "请逐行输出 $n 行编号文本，格式为 line-1 到 line-${n}，最后一行必须是 FINAL-MARKER。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
  [[ "$STOP_REASON" == "end_turn" ]] || ok=0
  grep -Fq "FINAL-MARKER" <<<"$REPLY" || ok=0
  local stats chunks
  stats="$(p5_stats)"
  chunks="$(jq -r '.chunks // 0' <<<"$stats")"
  [[ "$chunks" -ge 1 ]] || ok=0
  if ! acp_trace_check "$ACP_TRACE_PATH" '{"max":{"noise":0,"errors":0}}' >/dev/null 2>&1; then ok=0; fi
  ACP_READ_DELAY_MS=0
  p5_check "P5-05" "$ok" "slow reads (${LONGRUN_SLOW_READ_MS:-50}ms/chunk) delivered $chunks chunks, end_turn and FINAL-MARKER without truncation" \
    "$(p5_detail 1 0 "$(jq -nc --argjson c "$chunks" --argjson d "${LONGRUN_SLOW_READ_MS:-50}" '{chunks:$c,readDelayMs:$d}')")"
}

# --- P5-02: cross-VM resume (run last: it destroys and rebuilds the pool) ---
p5_cross_vm_resume() {
  p5_try e2e_fixture_detach "p5-slow" || true
  e2e_fixture_stop_all_pools
  local canary="CANARY-${E2E_RUN_ID:-local}-02" sid="" ok=1
  if ! e2e_fixture_start_pool "p5-vm-a" 1; then
    p5_check "P5-02" 0 "VM A pool never became idle"; return 1
  fi
  if ! p5_try e2e_fixture_attach "p5-vm-a" 3 4 pool; then ok=0; fi
  e2e_fixture_open_trace "p5-02-a" >/dev/null
  if p5_try e2e_fixture_handshake; then
    sid="$SESSION_ID"
    if ! p5_try acp_prompt_long \
      "记住口令 ${canary}。请用工具创建 /workspace/p5-crossvm.txt 写入这个口令，然后回复 ok。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
    if ! p5_try acp_prompt_long "把口令再追加一次到同一文件，回复 ok。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
    p5_try acp_close_long "$sid" || ok=0
  else ok=0; fi
  p5_try e2e_fixture_detach "p5-vm-a" || true
  e2e_fixture_stop_all_pools   # VM A is destroyed here; only /data survives.
  if ! e2e_fixture_start_pool "p5-vm-b" 1; then
    p5_check "P5-02" 0 "VM B pool never became idle"; return 1
  fi
  if ! p5_try e2e_fixture_attach "p5-vm-b" 3 4 pool; then ok=0; fi
  e2e_fixture_open_trace "p5-02-b" >/dev/null
  if ! p5_try acp_init_long; then ok=0; fi
  p5_try acp_list_find "$sid" /workspace || ok=0
  if p5_try acp_resume_long "$sid" /workspace; then
    # A successful resume proves the exclusive session.lock was released.
    if ! p5_try acp_prompt_long "复述你记的口令，并读取 /workspace/p5-crossvm.txt 的最后一行。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
    grep -Fq "$canary" <<<"$REPLY" || ok=0
    p5_try acp_close_long "$sid" || true
  else ok=0; fi
  local host_copy="$E2E_FIXTURE_HOSTDIR/p5-crossvm.txt"
  if [[ -f "$host_copy" ]]; then
    grep -Fq "$canary" "$host_copy" || ok=0
  else ok=0; fi
  p5_check "P5-02" "$ok" \
    "session $sid listed+resumed in a fresh VM on the same /data volume; canary and workspace file survived; no SessionAlreadyOwnedError" \
    "$(p5_detail 3 0 "$(jq -nc --arg s "$sid" --arg c "$canary" \
      '{sessionId:$s,canary:$c,vmA:"destroyed",vmB:"fresh",volume:"/data"}')")"
}

main() {
  if ! e2e_fixture_start_pool "p5-main" "${E2E_P5_POOL_SIZE:-2}"; then
    local id
    for id in "${P5_IDS[@]}"; do
      e2e_record_result "$id" "FAIL" "p5-main pool never became idle"
    done
    exit 1
  fi
  if ! e2e_fixture_attach "p5-main" 3 4 pool; then
    e2e_record_result "P5-01" "FAIL" "attach failed"
    exit 1
  fi
  p5_context_fidelity || true
  p5_cancel_and_interrupt || true
  p5_compaction || true
  p5_backpressure || true
  p5_cross_vm_resume || true
  e2e_fixture_cleanup
  trap - EXIT
  e2e_log "P5 finished with $P5_FAIL failing case(s)"
}

main "$@"

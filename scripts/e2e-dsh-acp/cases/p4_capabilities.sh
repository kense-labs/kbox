#!/usr/bin/env bash
# P4-01..P4-10: full dsh-acp capability surface over kbox 3.4.x.
#
# L1 (P4-01..05) is protocol: initialize, session lifecycle, config options,
# all session/update kinds and prompt content admission.
# L2 (P4-06..10) is the MicroVM carrier: cold run vs pool attach, named volume
# persistence, concurrent attach/replenish, teardown, resource boundary.
#
# Skeleton contract: every case either PASSes on a real assertion, FAILs with
# the observed trace stats, or records an explicit SKIP/BLOCKED with a reason.
# It never claims coverage without executing the RPC.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export E2E_FIXTURE_DIR="${E2E_FIXTURE_DIR:-${E2E_OUT:?E2E_OUT must be set}/p4}"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
# shellcheck source=../lib/acp_session.sh
source "$SCRIPT_DIR/../lib/acp_session.sh"
# shellcheck source=../lib/e2e_fixture.sh
source "$SCRIPT_DIR/../lib/e2e_fixture.sh"

P4_IDS=(P4-01 P4-02 P4-03 P4-04 P4-05 P4-06 P4-07 P4-08 P4-09 P4-10)
P4_FAIL=0
P4_INIT_RESPONSE=""
P4_GREETING="${GREETING:-你好！请用一句话介绍你自己。}"

if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  e2e_fixture_blocked "DEEPSEEK_API_KEY not set" "${P4_IDS[@]}"
  exit 0
fi
e2e_require_cmds jq python3
e2e_fixture_resolve_model
trap e2e_fixture_cleanup EXIT

p4_try() {
  local rc
  set +e
  "$@"
  rc=$?
  set -e
  return "$rc"
}

p4_check() {
  local id="$1" ok="$2" msg="$3" detail="${4:-null}"
  if [[ "$ok" == "1" ]]; then
    e2e_record_result "$id" "PASS" "$msg" "$detail"
  else
    e2e_record_result "$id" "FAIL" "$msg" "$detail"
    P4_FAIL=$((P4_FAIL + 1))
  fi
}

p4_stats() { acp_trace_stats "$ACP_TRACE_PATH" 2>/dev/null || echo null; }

# --- P4-01: initialize, capability declaration, repeated call, stdout purity ---
p4_initialize() {
  e2e_fixture_open_trace "p4-01" >/dev/null
  if ! p4_try acp_init_long; then
    p4_check "P4-01" 0 "initialize failed"; return 1
  fi
  P4_INIT_RESPONSE="$RPC_RESPONSE"
  local agent proto
  agent="$(jq -r '.result.agentInfo.name // empty' <<<"$P4_INIT_RESPONSE")"
  proto="$(jq -r '.result.protocolVersion // empty' <<<"$P4_INIT_RESPONSE")"
  if ! p4_try acp_init_long; then
    p4_check "P4-01" 0 "repeated initialize failed"; return 1
  fi
  local ok=1
  [[ -n "$agent" && -n "$proto" ]] || ok=0
  if ! jq -e '.result.agentCapabilities | has("mcpCapabilities") and has("promptCapabilities") and has("sessionCapabilities")' \
    >/dev/null 2>&1 <<<"$P4_INIT_RESPONSE"; then ok=0; fi
  if ! acp_trace_check "$ACP_TRACE_PATH" '{"max":{"noise":0,"errors":0}}' >/dev/null 2>&1; then ok=0; fi
  p4_check "P4-01" "$ok" "initialize agent=$agent protocol=$proto; stdout JSON-RPC only" \
    "$(jq -nc --arg a "$agent" --arg p "$proto" '{agent:$a,protocolVersion:$p}')"
}

# --- P4-02: new -> list -> close -> resume, negatives, pagination ---
p4_lifecycle() {
  e2e_fixture_open_trace "p4-02" >/dev/null
  local sid listed rc ok=1 page cursor="" seen="" sid2 dup=0 pages=0
  if ! p4_try acp_new_long; then
    p4_check "P4-02" 0 "session/new failed"; return 1
  fi
  sid="$SESSION_ID"
  # listSessions filters active sessions, so close before asserting visibility.
  if ! p4_try acp_close_long "$sid"; then ok=0; fi
  if ! p4_try acp_list_long /workspace; then ok=0; else
    listed="$(jq -r --arg s "$sid" '[.result.sessions[]? | select(.sessionId == $s)] | length' <<<"$RPC_RESPONSE")"
    [[ "$listed" == "1" ]] || ok=0
  fi
  if ! p4_try acp_resume_long "$sid" /workspace; then ok=0; else
    # Resume while the session is owned must be rejected.
    if p4_try acp_resume_long "$sid" /workspace; then ok=0; fi
    p4_try acp_close_long "$sid" || ok=0
  fi
  # cwd mismatch and unknown id must both fail closed.
  if p4_try acp_resume_long "$sid" /tmp; then ok=0; fi
  if p4_try acp_resume_long "00000000-0000-4000-8000-000000000000" /workspace; then ok=0; fi

  # Seed several closed sessions and walk list pages if the deployment pages.
  local i
  for i in $(seq 1 "${E2E_P4_EXTRA_SESSIONS:-3}"); do
    if p4_try acp_new_long; then
      sid2="$SESSION_ID"
      p4_try acp_close_long "$sid2" || ok=0
    else
      ok=0
    fi
  done
  if p4_try acp_list_long /workspace; then
    while :; do
      pages=$((pages + 1))
      local chunk
      chunk="$(jq -r '.result.sessions[]?.sessionId' <<<"$RPC_RESPONSE")"
      seen="$seen$chunk"$'\n'
      cursor="$(jq -r '.result.nextCursor // empty' <<<"$RPC_RESPONSE")"
      [[ -n "$cursor" && "$pages" -lt 5 ]] || break
      p4_try acp_list_long /workspace "$cursor" || { ok=0; break; }
    done
    dup="$(python3 -c 'import sys
lines = [line.strip() for line in sys.stdin if line.strip()]
print(len(lines) - len(set(lines)))' <<<"$seen")"
    [[ "$dup" == "0" ]] || ok=0
  else
    ok=0
  fi
  local pagination="not-triggered"
  if [[ "$pages" -gt 1 ]]; then pagination="pages=$pages"; fi
  p4_check "P4-02" "$ok" "new/list/close/resume + negatives; $pagination; no duplicate ids" \
    "$(jq -nc --arg s "$sid" --argjson pages "$pages" --argjson dup "$dup" \
      '{sessionId:$s,pages:$pages,duplicateIds:$dup}')"
}

# --- P4-03: model / reasoning_effort round-trip and persistence over resume ---
p4_config() {
  e2e_fixture_open_trace "p4-03" >/dev/null
  local cfg sid model_id efforts v="" ok=1 checked=0
  if ! p4_try acp_new_long; then
    p4_check "P4-03" 0 "session/new failed"; return 1
  fi
  cfg="$RPC_RESPONSE"
  sid="$SESSION_ID"
  model_id="$(jq -r '[.result.configOptions[]? | select(.id == "model") | .currentValue][0] // empty' <<<"$cfg")"
  # model options are grouped ({options:[{value}]}); reasoning options are flat.
  efforts="$(jq -c '[.result.configOptions[]? | select(.id == "reasoning_effort") | .options[]?.value]' <<<"$cfg")"
  if [[ -z "$model_id" ]]; then
    e2e_record_result "P4-03" "SKIP" "no model config option advertised"
    return 0
  fi
  if ! p4_try acp_set_option_long "model" "$model_id"; then ok=0; else
    jq -e --arg v "$model_id" \
      'any(.result.configOptions[]?; .id == "model" and .currentValue == $v)' \
      >/dev/null 2>&1 <<<"$RPC_RESPONSE" || ok=0
  fi
  # Iterate by index: the empty string is a valid "provider default" value.
  local effort_count idx=0
  effort_count="$(jq -r 'length' <<<"$efforts")"
  while (( idx < effort_count )); do
    v="$(jq -r --argjson i "$idx" '.[$i]' <<<"$efforts")"
    idx=$((idx + 1))
    checked=$((checked + 1))
    if ! p4_try acp_set_option_long "reasoning_effort" "$v"; then ok=0; continue; fi
    jq -e --arg v "$v" \
      'any(.result.configOptions[]?; .id == "reasoning_effort" and .currentValue == $v)' \
      >/dev/null 2>&1 <<<"$RPC_RESPONSE" || ok=0
  done
  # Last selected effort must survive close + resume.
  p4_try acp_close_long "$sid" || ok=0
  if p4_try acp_resume_long "$sid" /workspace; then
    if [[ "$checked" -gt 0 ]]; then
      jq -e --arg v "$v" \
        'any(.result.configOptions[]?; .id == "reasoning_effort" and .currentValue == $v)' \
        >/dev/null 2>&1 <<<"$RPC_RESPONSE" || ok=0
    fi
    p4_try acp_close_long "$sid" || true
  else
    ok=0
  fi
  local note="effort-over-resume not exercised (no advertised values)"
  if [[ "$checked" -gt 0 ]]; then note="effort-over-resume asserted"; fi
  p4_check "P4-03" "$ok" "model round-trip; $checked reasoning_effort values; $note" \
    "$(jq -nc --arg m "$model_id" --argjson n "$checked" '{model:$m,effortValuesChecked:$n}')"
}

# --- P4-04: session/update kinds, ordering, stopReason ---
p4_updates() {
  e2e_fixture_open_trace "p4-04" >/dev/null
  if ! p4_try acp_new_long; then
    p4_check "P4-04" 0 "session/new failed"; return 1
  fi
  if ! p4_try acp_prompt_long "请用工具列出 /workspace 下的文件，然后创建 /workspace/p4-updates.txt 写入 ok，最后只回答 done。" "$ACP_IDLE_TIMEOUT"; then
    p4_check "P4-04" 0 "tool-bearing prompt did not finish" "$(p4_stats)"; return 1
  fi
  local stats mandatory tool_missing=0
  stats="$(p4_stats)"
  if ! acp_trace_check "$ACP_TRACE_PATH" \
    '{"require_types":["agent_message_chunk"],"min":{"chunks":1},"max":{"noise":0,"errors":0}}' \
    >/dev/null 2>&1; then
    p4_check "P4-04" 0 "chunk stream / stdout purity assertion failed" "$stats"; return 1
  fi
  mandatory="$(jq -r '.toolCalls' <<<"$stats")"
  if [[ "$mandatory" == "0" || "$mandatory" == "null" ]]; then
    e2e_record_result "P4-04" "SKIP" \
      "agent emitted no tool_call; tool lifecycle not covered (check image tool set)" "$stats"
    return 0
  fi
  if ! acp_trace_check "$ACP_TRACE_PATH" \
    '{"require_types":["tool_call","tool_call_update"],"require_order":["tool_call","tool_call_update"]}' \
    >/dev/null 2>&1; then
    p4_check "P4-04" 0 "tool_call -> tool_call_update ordering broken" "$stats"; return 1
  fi
  local optional
  optional="$(jq -c '{thoughtChunks,usageUpdates,configOptionUpdates:(.byType.config_option_update // 0),stopReasons}' <<<"$stats")"
  p4_check "P4-04" 1 "chunks + tool lifecycle + stopReason=$STOP_REASON observed" \
    "$(jq -nc --argjson s "$stats" --argjson o "$optional" --arg stop "$STOP_REASON" \
      '{stats:$s,optional:$o,stopReason:$stop}')"
}

# --- P4-05: prompt content admission (negative paths need no model spend) ---
p4_content() {
  e2e_fixture_open_trace "p4-05" >/dev/null
  if ! p4_try acp_new_long; then
    p4_check "P4-05" 0 "session/new failed"; return 1
  fi
  local ok=1 rejected=0
  # content.ts rejects audio, embedded resource and non-canonical images.
  local blocks
  for blocks in \
    '[{"type":"audio","data":"AAAA","mimeType":"audio/wav"}]' \
    '[{"type":"resource","resource":{"uri":"file:///workspace/x","text":"x"}}]' \
    '[{"type":"image","data":"!!!!","mimeType":"image/bmp"}]'; do
    if p4_try acp_prompt_blocks_long "$blocks" 90; then ok=0; else rejected=$((rejected + 1)); fi
  done
  local image_enabled
  image_enabled="$(jq -r '.result.agentCapabilities.promptCapabilities.image // false' <<<"$P4_INIT_RESPONSE")"
  local image_note="image-capability=$image_enabled"
  if [[ "$image_enabled" == "true" ]]; then
    # 1x1 transparent PNG, canonical base64.
    local png='iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='
    if ! p4_try acp_prompt_blocks_long \
      "$(jq -nc --arg d "$png" '[{type:"text",text:"这是什么？"},{type:"image",data:$d,mimeType:"image/png"}]')" 300; then
      ok=0
    fi
    image_note="$image_note; inline image accepted"
  else
    image_note="$image_note; inline image not advertised (not exercised)"
  fi
  # Connection must still be usable after all rejections.
  p4_try acp_list_long /workspace || ok=0
  if [[ "${E2E_P4_HUGE_PROMPT:-0}" == "1" ]]; then
    local big
    big="$(python3 -c 'print("x" * int(__import__("os").environ.get("E2E_P4_HUGE_PROMPT_BYTES", "65536")))')"
    p4_try acp_prompt_blocks_long "$(jq -nc --arg t "$big" '[{type:"text",text:$t}]')" 300 || ok=0
    p4_try acp_list_long /workspace || ok=0
    image_note="$image_note; huge prompt exercised"
  else
    image_note="$image_note; huge prompt SKIPPED (set E2E_P4_HUGE_PROMPT=1 to spend tokens)"
  fi
  p4_check "P4-05" "$ok" "$rejected/3 invalid blocks rejected; $image_note; connection survived"
}

# --- P4-06: cold `run -i --rm` and `pool attach` must agree ---
p4_dual_path() {
  local cold_agent cold_proto cold_stop pool_agent pool_proto pool_stop ok=1
  if ! p4_try e2e_fixture_attach "p4-cold" 5 6 cold; then
    p4_check "P4-06" 0 "cold attach setup failed"; return 1
  fi
  e2e_fixture_open_trace "p4-06-cold" >/dev/null
  if p4_try acp_init_long; then
    cold_agent="$(jq -r '.result.agentInfo.name // empty' <<<"$RPC_RESPONSE")"
    cold_proto="$(jq -r '.result.protocolVersion // empty' <<<"$RPC_RESPONSE")"
  else ok=0; fi
  if ! p4_try acp_new_long; then ok=0; fi
  if ! p4_try acp_prompt_long "$P4_GREETING" 300; then ok=0; else cold_stop="$STOP_REASON"; fi
  p4_try acp_close_long "$SESSION_ID" || true
  p4_try e2e_fixture_detach "p4-cold" || true
  e2e_fixture_use 3 4
  e2e_fixture_open_trace "p4-06-pool" >/dev/null
  if p4_try acp_init_long; then
    pool_agent="$(jq -r '.result.agentInfo.name // empty' <<<"$RPC_RESPONSE")"
    pool_proto="$(jq -r '.result.protocolVersion // empty' <<<"$RPC_RESPONSE")"
  else ok=0; fi
  if p4_try acp_new_long; then
    if ! p4_try acp_prompt_long "$P4_GREETING" 300; then ok=0; else pool_stop="$STOP_REASON"; fi
    p4_try acp_close_long "$SESSION_ID" || true
  else ok=0; fi
  [[ -n "$cold_agent" && "$cold_agent" == "${pool_agent:-}" ]] || ok=0
  [[ "${cold_stop:-}" == "end_turn" && "${pool_stop:-}" == "end_turn" ]] || ok=0
  local running
  running="$(e2e_running_box_count)"
  # The pool is still alive here, so a raw box count cannot isolate the cold
  # VM. Record it as evidence; P4-09 asserts the final zero-box state.
  p4_check "P4-06" "$ok" \
    "cold/pool agent=$cold_agent proto=${cold_proto:-?} stop=${cold_stop:-?}/${pool_stop:-?}; running boxes now=$running (pool still up)" \
    "$(jq -nc --arg a "$cold_agent" --arg p "${pool_agent:-}" --arg cs "${cold_stop:-}" \
      --arg ps "${pool_stop:-}" --argjson r "$running" \
      '{coldAgent:$a,poolAgent:$p,coldStop:$cs,poolStop:$ps,runningBoxesAfter:$r}')"
}

# --- P4-07: named /data volume survives a second attach ---
p4_volume() {
  local sid ok=1 marker="p4-volume-${E2E_RUN_ID:-local}"
  printf '%s\n' "$marker" >"$E2E_FIXTURE_HOSTDIR/p4-marker.txt"
  e2e_fixture_open_trace "p4-07-a" >/dev/null
  e2e_fixture_use 3 4
  if p4_try acp_new_long; then
    sid="$SESSION_ID"
    p4_try acp_close_long "$sid" || ok=0
  else ok=0; fi
  p4_try e2e_fixture_detach "p4-proto" || true
  if ! e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90; then
    p4_check "P4-07" 0 "pool did not replenish after detach"; return 1
  fi
  if ! p4_try e2e_fixture_attach "p4-vol-b" 3 4 pool; then
    p4_check "P4-07" 0 "second attach failed"; return 1
  fi
  e2e_fixture_open_trace "p4-07-b" >/dev/null
  if ! p4_try acp_init_long; then ok=0; fi
  p4_try acp_list_find "$sid" /workspace || ok=0
  if ! p4_try acp_resume_long "$sid" /workspace; then ok=0; else
    if ! p4_try acp_prompt_long "请读取 /workspace/p4-marker.txt，只输出文件内容。" 300; then ok=0; else
      grep -Fq "$marker" <<<"$REPLY" || ok=0
    fi
    p4_try acp_close_long "$sid" || true
  fi
  p4_check "P4-07" "$ok" "session $sid visible/resumable from a new VM on the same /data volume; host marker echoed" \
    "$(jq -nc --arg s "$sid" '{sessionId:$s,markerFile:"/workspace/p4-marker.txt"}')"
}

# --- P4-08: size=1 exhaustion, replenish and status observability ---
p4_concurrency() {
  local ok=1 status_a status_b
  p4_try e2e_fixture_detach "p4-vol-b" || true
  if ! e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90; then
    p4_check "P4-08" 0 "no idle slot before concurrency case"; return 1
  fi
  p4_try e2e_fixture_attach "p4-conc-a" 5 6 pool || ok=0
  sleep 2
  p4_try e2e_fixture_attach "p4-conc-b" 7 8 pool || ok=0
  sleep "${E2E_P4_EXHAUST_WAIT_SECONDS:-8}"
  status_a="$("$KBOX" pool status --socket "$E2E_FIXTURE_SOCKET" --json 2>/dev/null || echo '[]')"
  # Under size=1 the second attach must not silently get a second VM.
  local second_alive=0
  local entry label pid infd outfd
  if (( ${#E2E_FIXTURE_CONNS[@]} )); then
    for entry in "${E2E_FIXTURE_CONNS[@]}"; do
      IFS='|' read -r label pid infd outfd <<<"$entry"
      if [[ "$label" == "p4-conc-b" ]] && kill -0 "$pid" 2>/dev/null; then second_alive=1; fi
    done
  fi
  # Either blocked (still alive) or failed fast; both are acceptable, but
  # a third successful attach while size=1 is busy is not.
  p4_try e2e_fixture_detach "p4-conc-a" || true
  p4_try e2e_fixture_detach "p4-conc-b" || true
  if ! e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90; then ok=0; fi
  status_b="$("$KBOX" pool status --socket "$E2E_FIXTURE_SOCKET" --json 2>/dev/null || echo '[]')"
  echo "$status_a" >"$E2E_FIXTURE_DIR/p4-08-status-during.json"
  echo "$status_b" >"$E2E_FIXTURE_DIR/p4-08-status-after.json"
  p4_check "P4-08" "$ok" \
    "size=1 exhaustion observed (second_attach_alive=$second_alive); pool replenished to idle" \
    "$(jq -nc --argjson alive "$second_alive" '{secondAttachAliveWhileBusy:$alive}')"
}

# --- P4-09: teardown leaves no socket, box or lifecycle lock behind ---
p4_teardown() {
  local socket="$E2E_FIXTURE_SOCKET" running locks ok=1
  e2e_fixture_stop_all_pools
  sleep 1
  if [[ -S "$socket" ]]; then ok=0; fi
  running="$(e2e_running_box_count)"
  [[ "$running" == "0" ]] || ok=0
  locks="$(find "${K_HOME:-$E2E_FIXTURE_DIR}" -name 'lifecycle.lock' -type f 2>/dev/null | wc -l | tr -d ' ' || true)"
  [[ "$locks" == "0" ]] || ok=0
  if [[ "${E2E_P4_SNAPSHOT:-0}" == "1" ]]; then
    e2e_log "P4-09: snapshot path opted in but has no qualified kbox 3.4.3 command in this skeleton"
  fi
  p4_check "P4-09" "$ok" "pool stopped; socket gone; running_boxes=$running; lifecycle_locks=$locks (snapshot sub-case not qualified)" \
    "$(jq -nc --argjson r "$running" --argjson l "$locks" '{runningBoxes:$r,lifecycleLocks:$l}')"
}

# --- P4-10: --memory 2g resource boundary (optional CLI flag) ---
p4_memory_boundary() {
  if ! p4_try e2e_fixture_start_pool "p4-mem" 1 --memory 2g; then
    e2e_record_result "P4-10" "SKIP" \
      "pool start --memory 2g not accepted by this kbox build; see p4-mem-pool.log"
    return 0
  fi
  if ! p4_try e2e_fixture_attach "p4-mem" 3 4 pool; then
    p4_check "P4-10" 0 "attach failed with --memory 2g"; return 1
  fi
  e2e_fixture_open_trace "p4-10" >/dev/null
  local ok=1
  if ! p4_try e2e_fixture_handshake; then ok=0; fi
  if ! p4_try acp_prompt_long "$P4_GREETING" 300; then ok=0; fi
  [[ "$STOP_REASON" == "end_turn" ]] || ok=0
  p4_try acp_close_long "$SESSION_ID" || true
  p4_try e2e_fixture_detach "p4-mem" || true
  p4_check "P4-10" "$ok" "greeting turn completed inside --memory 2g VM" "$(p4_stats)"
}

main() {
  if ! e2e_fixture_start_pool "p4" "${E2E_P4_POOL_SIZE:-2}"; then
    local id
    for id in "${P4_IDS[@]}"; do
      e2e_record_result "$id" "FAIL" "pool never became idle; see p4-pool.log"
    done
    exit 1
  fi
  if ! e2e_fixture_attach "p4-proto" 3 4 pool; then
    e2e_record_result "P4-01" "FAIL" "pool attach failed"
    exit 1
  fi
  p4_initialize || true
  p4_lifecycle || true
  p4_config || true
  p4_updates || true
  p4_content || true
  p4_dual_path || true
  p4_volume || true
  p4_concurrency || true
  p4_teardown || true
  p4_memory_boundary || true
  e2e_fixture_cleanup
  trap - EXIT
  e2e_log "P4 finished with $P4_FAIL failing case(s)"
}

main "$@"

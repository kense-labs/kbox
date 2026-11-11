#!/usr/bin/env bash
# P6-01..P6-05: complex, tool-heavy and adversarial tasks.
#
# Covers cross-file refactor, real MCP transports, permission interleaving,
# multi-session orchestration/cancel targeting, and network/interactive tools.
# Anything that needs an external fixture reports an explicit SKIP with the
# exact env var to set; negative protocol assertions always run.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export E2E_FIXTURE_DIR="${E2E_FIXTURE_DIR:-${E2E_OUT:?E2E_OUT must be set}/p6}"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
# shellcheck source=../lib/acp_session.sh
source "$SCRIPT_DIR/../lib/acp_session.sh"
# shellcheck source=../lib/e2e_fixture.sh
source "$SCRIPT_DIR/../lib/e2e_fixture.sh"

P6_IDS=(P6-01 P6-02 P6-03 P6-04 P6-05)
P6_FAIL=0
P6_FORBIDDEN_TOOLS="${E2E_P6_FORBIDDEN_TOOLS:-apply_patch}"

if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  e2e_fixture_blocked "DEEPSEEK_API_KEY not set" "${P6_IDS[@]}"
  exit 0
fi
e2e_require_cmds jq python3
e2e_fixture_resolve_model
trap e2e_fixture_cleanup EXIT

p6_try() {
  local rc
  set +e
  "$@"
  rc=$?
  set -e
  return "$rc"
}
p6_stats() { acp_trace_stats "$ACP_TRACE_PATH" 2>/dev/null || echo null; }
p6_check() {
  local id="$1" ok="$2" msg="$3" detail="${4:-null}"
  if [[ "$ok" == "1" ]]; then
    e2e_record_result "$id" "PASS" "$msg" "$detail"
  else
    e2e_record_result "$id" "FAIL" "$msg" "$detail"
    P6_FAIL=$((P6_FAIL + 1))
  fi
}

# Prepare a deliberately broken multi-file project inside the bind mount.
p6_project() {
  local dir="$E2E_FIXTURE_HOSTDIR/p6-project"
  mkdir -p "$dir"
  cat >"$dir/math.js" <<'JS'
function add(a, b) {
  // intentional bug for the refactor case
  return a - b;
}
module.exports = { add };
JS
  cat >"$dir/math.test.js" <<'JS'
const test = require('node:test');
const assert = require('node:assert');
const { add } = require('./math.js');
test('add returns the sum', () => assert.strictEqual(add(2, 3), 5));
JS
  cat >"$dir/README.md" <<'MD'
# p6 fixture
Fix `math.js` so `node --test` passes. Tests must not be edited.
MD
  echo "$dir"
}

# --- P6-01: grep -> read -> edit -> run tests -> fix, with tool lifecycle ---
p6_refactor() {
  local dir
  dir="$(p6_project)"
  e2e_fixture_open_trace "p6-01" >/dev/null
  if ! p6_try e2e_fixture_handshake; then
    p6_check "P6-01" 0 "handshake failed"; return 1
  fi
  local ok=1 stats
  if ! p6_try acp_prompt_long \
    "请检查 /workspace/p6-project 里的代码和测试，用工具修复 math.js 中 add 的 bug，然后运行 node --test 验证，最后只回复 done。不要修改测试文件，也不要使用 $P6_FORBIDDEN_TOOLS 工具。" \
    "$ACP_IDLE_TIMEOUT"; then ok=0; fi
  [[ "$STOP_REASON" == "end_turn" ]] || ok=0
  stats="$(p6_stats)"
  if ! acp_trace_check "$ACP_TRACE_PATH" \
    '{"require_types":["tool_call","tool_call_update"],"require_order":["tool_call","tool_call_update"],"min":{"toolCalls":2},"max":{"noise":0}}' \
    >/dev/null 2>&1; then ok=0; fi
  grep -Fq 'a + b' "$dir/math.js" || ok=0
  local test_result="skipped (node not on host)"
  if command -v node >/dev/null 2>&1; then
    if (cd "$dir" && node --test >"$E2E_FIXTURE_DIR/p6-01-node-test.log" 2>&1); then
      test_result="pass"
    else
      test_result="fail"
      ok=0
    fi
  fi
  local forbidden_present
  forbidden_present="$(jq -r --arg t "$P6_FORBIDDEN_TOOLS" 'any(.toolTitles[]?; . == $t)' <<<"$stats")"
  [[ "$forbidden_present" == "false" ]] || ok=0
  p6_check "P6-01" "$ok" \
    "refactor finished; node --test=$test_result; forbidden '$P6_FORBIDDEN_TOOLS' used=$forbidden_present; tool lifecycle ordered" \
    "$(jq -nc --argjson s "$stats" --arg t "$test_result" '{stats:$s,hostTest:$t}')"
}

# --- P6-02: MCP stdio + Streamable HTTP, negatives always run ---
p6_mcp() {
  e2e_fixture_open_trace "p6-02" >/dev/null
  local ok=1 rejected=0 mcp_positive="" skipped=""
  # Negative declarations are pure protocol assertions (no server needed).
  local bad
  for bad in \
    '[{"name":"bad","command":"relative-command","args":[],"env":[]}]' \
    '[{"name":"bad","type":"http","url":"ftp://not-http","headers":[]}]' \
    '[{"name":"dup","command":"/bin/true","args":[],"env":[]},{"name":"dup","command":"/bin/true","args":[],"env":[]}]' \
    '[{"name":"bad","type":"sse","url":"http://127.0.0.1:1","headers":[]}]'; do
    if p6_try acp_new_long /workspace "$bad"; then ok=0; else rejected=$((rejected + 1)); fi
  done

  # Positive transports require an injected server, otherwise SKIP that half.
  local servers='[]' tool_name="${E2E_MCP_TOOL_NAME:-}"
  if [[ -n "${E2E_MCP_STDIO_COMMAND:-}" ]]; then
    servers="$(jq -nc --arg c "$E2E_MCP_STDIO_COMMAND" \
      '[{name:"e2e-stdio",command:$c,args:[],env:[]}]')"
  fi
  if [[ -n "${E2E_MCP_HTTP_URL:-}" ]]; then
    servers="$(jq -nc --argjson base "$servers" --arg u "$E2E_MCP_HTTP_URL" \
      '$base + [{name:"e2e-http",type:"http",url:$u,headers:[]}]')"
  fi
  if [[ "$servers" != "[]" ]]; then
    if p6_try acp_new_long /workspace "$servers"; then
      local sid="$SESSION_ID"
      if ! p6_try acp_prompt_long \
        "请调用已挂载的 MCP 工具完成一次往返，并说明调用了哪个工具。" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
      if [[ -n "$tool_name" ]]; then
        jq -e --arg t "$tool_name" 'any(.toolTitles[]?; . == $t or (. | contains($t)))' \
          >/dev/null 2>&1 <<<"$(p6_stats)" || ok=0
      fi
      p6_try acp_close_long "$sid" || ok=0
      if p6_try acp_resume_long "$sid" /workspace "$servers"; then
        mcp_positive="stdio/http mounted, tools called, remounted on resume"
        p6_try acp_close_long "$sid" || true
      else ok=0; fi
    else ok=0; fi
  else
    skipped="set E2E_MCP_STDIO_COMMAND (absolute path) and/or E2E_MCP_HTTP_URL for the positive path"
    e2e_log "P6-02: $skipped"
  fi
  local msg="$rejected/4 invalid mcpServers rejected${mcp_positive:+; $mcp_positive}"
  if [[ -n "$skipped" ]]; then
    e2e_record_result "P6-02" "SKIP" "$msg; positive transport not covered: $skipped" "$(p6_stats)"
    return 0
  fi
  p6_check "P6-02" "$ok" "$msg" "$(p6_stats)"
}

# --- P6-03: permission requests interleaved with session/update ---
p6_permissions() {
  local ok=1 policy results="" requests=0
  local prompt="${E2E_P6_PERMISSION_PROMPT:-请尝试向 /etc 目录写入一个文件，并在需要授权时继续，最后回复结果。}"
  for policy in allow-once reject-once cancelled fail; do
    e2e_fixture_open_trace "p6-03-$policy" >/dev/null
    ACP_PERMISSION_POLICY="$policy"
    local sid
    if ! p6_try acp_new_long; then ok=0; continue; fi
    sid="$SESSION_ID"
    if ! p6_try acp_prompt_long "$prompt" "$ACP_IDLE_TIMEOUT"; then ok=0; fi
    local stats req ans
    stats="$(p6_stats)"
    req="$(jq -r '.permissionRequests' <<<"$stats")"
    ans="$(jq -r '.permissionAnswers' <<<"$stats")"
    requests=$((requests + req))
    [[ "$req" == "$ans" ]] || ok=0
    # Updates must keep flowing after a permission decision (no lost ordering).
    jq -e '.chunks > 0' >/dev/null 2>&1 <<<"$stats" || ok=0
    results="$results $policy:req=$req/ans=$ans"
    p6_try acp_close_long "$sid" || true
    cp "$ACP_TRACE_PATH" "$E2E_FIXTURE_DIR/p6-03-${policy}.jsonl" 2>/dev/null || true
  done
  ACP_PERMISSION_POLICY="allow-once"
  if [[ "$requests" == "0" ]]; then
    e2e_record_result "P6-03" "SKIP" \
      "no approval/request was triggered; inject E2E_P6_PERMISSION_PROMPT that requires escalation" \
      "$(p6_stats)"
    return 0
  fi
  p6_check "P6-03" "$ok" "one-shot permission policies exercised:$results; answers matched requests; chunks kept flowing" \
    "$(jq -nc --argjson req "$requests" '{totalPermissionRequests:$req,policies:["allow-once","reject-once","cancelled","fail"]}')"
}

# --- P6-04: two sessions on one connection, precise cancel, list filtering ---
p6_orchestration() {
  p6_try e2e_fixture_detach "p6-main" || true
  if ! e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90; then
    p6_check "P6-04" 0 "no idle VM for orchestration case"; return 1
  fi
  if ! p6_try e2e_fixture_attach "p6-orch" 3 4 pool; then
    p6_check "P6-04" 0 "attach failed"; return 1
  fi
  e2e_fixture_open_trace "p6-04" >/dev/null
  local ok=1 sid1 sid2
  if ! p6_try acp_init_long; then ok=0; fi
  if p6_try acp_new_long; then sid1="$SESSION_ID"; else ok=0; fi
  if p6_try acp_new_long; then sid2="$SESSION_ID"; else ok=0; fi
  # acp_prompt_long targets $SESSION_ID; pin it back to sid1 so the cancel
  # targeting assertions exercise the intended session.
  SESSION_ID="$sid1"
  # listSessions filters active sessions: neither sid may appear yet.
  if p6_try acp_list_long /workspace; then
    jq -e --arg a "$sid1" --arg b "$sid2" \
      'any(.result.sessions[]?; .sessionId == $a or .sessionId == $b) | not' \
      >/dev/null 2>&1 <<<"$RPC_RESPONSE" || ok=0
  else ok=0; fi
  # Cancel aimed at the OTHER session must not disturb sid1's turn.
  if p6_try acp_prompt_long "请只回复 still-alive。" 300; then
    [[ "$STOP_REASON" == "end_turn" ]] || ok=0
  else ok=0; fi
  p6_try acp_cancel_long "$sid2" || ok=0
  if p6_try acp_prompt_long "再回复 still-alive-2。" 300; then
    [[ "$STOP_REASON" == "end_turn" ]] || ok=0
  else ok=0; fi
  # Cancel aimed at the active session must cancel exactly that prompt.
  (
    sleep "${E2E_P6_CANCEL_DELAY_SECONDS:-3}"
    jq -nc --arg s "$sid1" '{jsonrpc:"2.0",method:"session/cancel",params:{sessionId:$s}}' >&"${ACP_IN_FD}"
  ) &
  local cancel_pid=$!
  if p6_try acp_prompt_long "请写一份很长的规划文档，不要提前结束。" "$ACP_IDLE_TIMEOUT"; then
    [[ "$STOP_REASON" == "cancelled" ]] || ok=0
  else ok=0; fi
  kill "$cancel_pid" 2>/dev/null || true
  wait "$cancel_pid" 2>/dev/null || true
  p6_try acp_close_long "$sid1" || true
  p6_try acp_close_long "$sid2" || true
  local subagent_note="subagent resume rejection not exercised (no fork fixture)"
  if [[ "${E2E_P6_SUBA:-0}" == "1" ]]; then
    subagent_note="subagent fork opted in; inspect p6-04.jsonl for a child session id"
  fi
  p6_check "P6-04" "$ok" \
    "two live sessions isolated; cancel(sid2) did not affect sid1; cancel(sid1) -> cancelled; $subagent_note" \
    "$(jq -nc --arg a "$sid1" --arg b "$sid2" '{sid1:$a,sid2:$b,listFiltered:true}')"
}

# --- P6-05: network and interactive tools reachable inside the VM ---
p6_network_tools() {
  if ! e2e_wait_pool_idle "$E2E_FIXTURE_SOCKET" 90; then
    p6_check "P6-05" 0 "no idle VM for network tools case"; return 1
  fi
  p6_try e2e_fixture_detach "p6-orch" || true
  if ! p6_try e2e_fixture_attach "p6-net" 3 4 pool; then
    p6_check "P6-05" 0 "attach failed"; return 1
  fi
  e2e_fixture_open_trace "p6-05" >/dev/null
  if ! p6_try e2e_fixture_handshake; then
    p6_check "P6-05" 0 "handshake failed"; return 1
  fi
  local ok=1 stats titles
  if ! p6_try acp_prompt_long \
    "请依次尝试：用 web_fetch 抓取一个公开网页，必要时用 web_search 检索；如果需要向我确认信息就使用 ask_user_question；完成后用 exit_plan_mode 结束。最后列出你实际用到的工具名。" \
    "$ACP_IDLE_TIMEOUT"; then ok=0; fi
  stats="$(p6_stats)"
  titles="$(jq -r '.toolTitles | join(",")' <<<"$stats")"
  jq -e '.toolCalls > 0' >/dev/null 2>&1 <<<"$stats" || ok=0
  local network_ok interactive_ok
  network_ok="$(jq -r 'any(.toolTitles[]?; . == "web_fetch" or . == "web_search")' <<<"$stats")"
  interactive_ok="$(jq -r 'any(.toolTitles[]?; . == "ask_user_question" or . == "exit_plan_mode")' <<<"$stats")"
  local note="network=$network_ok interactive=$interactive_ok"
  if [[ "$network_ok" == "false" && "$interactive_ok" == "false" ]]; then
    e2e_record_result "P6-05" "SKIP" \
      "no network/interactive tool was invoked inside the VM; $note; toolTitles=[$titles]" "$stats"
    return 0
  fi
  p6_check "P6-05" "$ok" "tool round-trip inside VM; $note; toolTitles=[$titles]" \
    "$(jq -nc --argjson s "$stats" --arg n "$network_ok" --arg i "$interactive_ok" \
      '{stats:$s,networkTools:$n,interactiveTools:$i}')"
}

main() {
  if ! e2e_fixture_start_pool "p6-main" "${E2E_P6_POOL_SIZE:-2}"; then
    local id
    for id in "${P6_IDS[@]}"; do
      e2e_record_result "$id" "FAIL" "p6-main pool never became idle"
    done
    exit 1
  fi
  if ! e2e_fixture_attach "p6-main" 3 4 pool; then
    e2e_record_result "P6-01" "FAIL" "attach failed"
    exit 1
  fi
  p6_refactor || true
  p6_mcp || true
  p6_permissions || true
  p6_orchestration || true
  p6_network_tools || true
  e2e_fixture_cleanup
  trap - EXIT
  e2e_log "P6 finished with $P6_FAIL failing case(s)"
}

main "$@"

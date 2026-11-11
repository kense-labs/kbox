#!/usr/bin/env bash
# Minimal ACP JSON-RPC session over fifos attached to `kbox pool attach` or `kbox run -i`.
# Requires: jq, bash, open fds 3 (write to agent stdin) and 4 (read agent stdout).

acp_rpc() {
  local id="$1" payload="$2" timeout="${3:-60}"
  local line chunk update
  RPC_RESPONSE=""
  printf '%s\n' "$payload" >&3
  while IFS= read -r -t "$timeout" -u 4 line; do
    [[ "$line" == \{* ]] || continue
    if jq -e '.method? == "session/request_permission" and has("id")' >/dev/null 2>&1 <<<"$line"; then
      local permission_kind="reject_once" permission_reply
      [[ "${ACP_ALLOW_ONCE:-0}" != 1 ]] || permission_kind="allow_once"
      permission_reply="$(jq -c --arg kind "$permission_kind" '
        {jsonrpc:"2.0",id:.id,result:{outcome:(
          ([.params.options[] | select(.kind == $kind)] | first) as $option |
          if $option == null then {outcome:"cancelled"}
          else {outcome:"selected",optionId:$option.optionId} end)}}' <<<"$line")"
      printf '%s\n' "$permission_reply" >&3
      continue
    fi
    if jq -e --argjson id "$id" '.id? == $id and (has("method") | not)' >/dev/null 2>&1 <<<"$line"; then
      if jq -e 'has("error")' >/dev/null 2>&1 <<<"$line"; then
        jq -r '.error.message // "ACP request failed"' >&2 <<<"$line"
        return 1
      fi
      RPC_RESPONSE="$line"
      return 0
    fi
    update="$(jq -r 'select(.method? == "session/update") | .params.update.sessionUpdate // empty' <<<"$line" 2>/dev/null || true)"
    if [[ "$update" == "tool_call" ]]; then
      TOOL_CALL_COUNT=$((${TOOL_CALL_COUNT:-0} + 1))
    elif [[ "$update" == "tool_call_update" ]] &&
      jq -e '.params.update.status == "completed"' >/dev/null 2>&1 <<<"$line"; then
      TOOL_COMPLETE_COUNT=$((${TOOL_COMPLETE_COUNT:-0} + 1))
    fi
    chunk="$(jq -rj 'select(.method? == "session/update") |
      .params.update | select(.sessionUpdate == "agent_message_chunk") |
      .content | select(.type == "text") | .text // empty' <<<"$line" 2>/dev/null || true)"
    [[ -z "${chunk:-}" ]] || {
      printf '%s' "$chunk" >&2
      REPLY="${REPLY:-}$chunk"
    }
  done
  echo "ACP request $id timed out or disconnected" >&2
  return 1
}

acp_initialize() {
  acp_rpc 1 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}' 45
}

acp_session_new() {
  acp_rpc 2 "$(jq -nc '{jsonrpc:"2.0",id:2,method:"session/new",params:{cwd:"/workspace",mcpServers:[]}}')" 45
  SESSION_ID="$(jq -r '.result.sessionId' <<<"$RPC_RESPONSE")"
  [[ -n "$SESSION_ID" && "$SESSION_ID" != null ]]
}

acp_set_model_config() {
  local provider="$1" model="$2" effort="$3"
  local model_value
  model_value="$(jq -nc --arg provider "$provider" --arg model "$model" '[$provider,$model]')"
  acp_rpc 3 "$(jq -nc --arg session "$SESSION_ID" --arg value "$model_value" \
    '{jsonrpc:"2.0",id:3,method:"session/set_config_option",params:{sessionId:$session,configId:"model",value:$value}}')" 45
  jq -e --arg value "$model_value" \
    '[.result.configOptions[]? | select(.id == "model" and .currentValue == $value)] | length == 1' \
    >/dev/null <<<"$RPC_RESPONSE"
  acp_rpc 4 "$(jq -nc --arg session "$SESSION_ID" --arg value "$effort" \
    '{jsonrpc:"2.0",id:4,method:"session/set_config_option",params:{sessionId:$session,configId:"reasoning_effort",value:$value}}')" 45
  jq -e --arg value "$effort" \
    '[.result.configOptions[]? | select(.id == "reasoning_effort" and .currentValue == $value)] | length == 1' \
    >/dev/null <<<"$RPC_RESPONSE"
}

acp_prompt() {
  local text="$1" timeout="${2:-240}"
  REPLY=""
  TOOL_CALL_COUNT=0
  TOOL_COMPLETE_COUNT=0
  acp_rpc 5 "$(jq -nc --arg session "$SESSION_ID" --arg text "$text" \
    '{jsonrpc:"2.0",id:5,method:"session/prompt",params:{sessionId:$session,prompt:[{type:"text",text:$text}]}}')" "$timeout"
  STOP_REASON="$(jq -r '.result.stopReason // empty' <<<"$RPC_RESPONSE")"
  [[ -n "$REPLY" && "$STOP_REASON" == "end_turn" ]]
}

# ---------------------------------------------------------------------------
# Long-lived, traced ACP client (P4/P5/P6).
#
# The legacy helpers above keep ids 1..5 and fds 3/4 for P0-P3. This API starts
# at id 100, uses configurable fds, survives many turns on one connection and
# records EVERY incoming JSON-RPC message (all session/update kinds, server
# requests, responses and non-JSON stdout noise) as one compact JSONL row.
# Only short metadata is stored: chunk text is never written to evidence.
# ---------------------------------------------------------------------------

ACP_IN_FD="${ACP_IN_FD:-3}"
ACP_OUT_FD="${ACP_OUT_FD:-4}"
ACP_TRACE_FILE="${ACP_TRACE_FILE:-}"
ACP_PROTOCOL_VERSION="${ACP_PROTOCOL_VERSION:-1}"
ACP_IDLE_TIMEOUT="${ACP_IDLE_TIMEOUT:-900}"        # per-read silence limit, not total turn time
ACP_MAX_TURN_SECONDS="${ACP_MAX_TURN_SECONDS:-3600}" # hard wall-clock guard for one prompt
ACP_READ_DELAY_MS="${ACP_READ_DELAY_MS:-0}"          # >0 simulates a slow client (backpressure)
ACP_PERMISSION_POLICY="${ACP_PERMISSION_POLICY:-allow-once}" # allow-once|reject-once|cancelled|fail
ACP_NEXT_ID=100
ACP_TRACE_SEQ=0
ACP_NOISE_LINES=0
ACP_PERMISSION_REQUESTS=0

# Start a fresh trace. Args: <jsonl-path>
acp_open_trace() {
  ACP_TRACE_FILE="$1"
  : >"$ACP_TRACE_FILE" || return 1
  ACP_NEXT_ID=100
  ACP_TRACE_SEQ=0
  ACP_NOISE_LINES=0
  ACP_PERMISSION_REQUESTS=0
}

# Write one raw JSON-RPC line as a compact summary row.
acp_trace_line() {
  local line="$1" rec
  ACP_TRACE_SEQ=$((ACP_TRACE_SEQ + 1))
  [[ -n "$ACP_TRACE_FILE" ]] || return 0
  rec="$(jq -nc \
    --argjson seq "$ACP_TRACE_SEQ" \
    --argjson t "$SECONDS" \
    --arg line "$line" '
      ($line | try fromjson catch null) as $raw
      | (if ($raw|type)=="object" then $raw else {} end) as $m
      | if ($raw|type) != "object" then
          {seq:$seq,t:$t,kind:"noise",preview:($line[0:200])}
        else
          {
            seq:$seq, t:$t,
            kind: (if (($m|has("method")) and ($m|has("id"))) then "request"
                   elif ($m|has("method")) then "notification"
                   elif ($m|has("id")) then "response" else "other" end),
            id: ($m.id // null),
            method: ($m.method // null),
            sessionUpdate: ($m.params.update.sessionUpdate? // null),
            toolCallId: ($m.params.update.toolCallId? // null),
            title: ($m.params.update.title? // null),
            status: ($m.params.update.status? // null),
            used: ($m.params.update.used? // null),
            size: ($m.params.update.size? // null),
            textLen: ((($m.params.update.content.text? // "") | length)),
            stopReason: ($m.result.stopReason? // null),
            error: (if ($m|has("error")) then ($m.error.message? // "error") else null end),
            resultKeys: (if ($m.result|type)=="object" then ($m.result|keys) else [] end)
          }
        end
    ' 2>/dev/null)" || rec="{\"seq\":$ACP_TRACE_SEQ,\"t\":$SECONDS,\"kind\":\"noise\",\"preview\":\"unparseable\"}"
  printf '%s\n' "$rec" >>"$ACP_TRACE_FILE"
  if [[ "$rec" == *'"kind":"noise"'* ]]; then
    ACP_NOISE_LINES=$((ACP_NOISE_LINES + 1))
  fi
}

# Append a locally-produced marker (e.g. the answer we sent for a permission request).
acp_trace_note() {
  local kind="$1" extra="${2:-}"
  [[ -n "$extra" ]] || extra='{}'
  [[ -n "$ACP_TRACE_FILE" ]] || return 0
  ACP_TRACE_SEQ=$((ACP_TRACE_SEQ + 1))
  jq -nc --argjson seq "$ACP_TRACE_SEQ" --argjson t "$SECONDS" \
    --arg kind "$kind" --argjson extra "$extra" \
    '{seq:$seq,t:$t,kind:$kind} + $extra' >>"$ACP_TRACE_FILE"
}

acp_send() {
  printf '%s\n' "$1" >&"$ACP_IN_FD"
}

# Reply to a server -> client request so an in-flight prompt cannot deadlock.
acp_handle_server_request() {
  local line="$1" id method resp
  id="$(jq -c '.id' <<<"$line")"
  method="$(jq -r '.method // empty' <<<"$line")"
  case "$method" in
    session/request_permission)
      ACP_PERMISSION_REQUESTS=$((ACP_PERMISSION_REQUESTS + 1))
      case "$ACP_PERMISSION_POLICY" in
        allow-once|reject-once)
          resp="$(jq -nc --arg o "$ACP_PERMISSION_POLICY" '{outcome:{outcome:"selected",optionId:$o}}')" ;;
        cancelled) resp='{"outcome":{"outcome":"cancelled"}}' ;;
        fail)
          acp_send "$(jq -nc --argjson id "$id" \
            '{jsonrpc:"2.0",id:$id,error:{code:-32000,message:"client refused"}}')"
          acp_trace_note "permission_answer" "$(jq -nc '{policy:"fail",outcome:"error"}')"
          return 0 ;;
        *) resp='{"outcome":{"outcome":"cancelled"}}' ;;
      esac
      acp_send "$(jq -nc --argjson id "$id" --argjson result "$resp" \
        '{jsonrpc:"2.0",id:$id,result:$result}')"
      acp_trace_note "permission_answer" \
        "$(jq -nc --arg p "$ACP_PERMISSION_POLICY" --arg c "$(jq -r '.params.toolCall.toolCallId // empty' <<<"$line")" \
          '{policy:$p,toolCallId:(if $c == "" then null else $c end)}')"
      ;;
    *)
      # Unknown client method: fail explicitly instead of hanging the turn.
      acp_send "$(jq -nc --argjson id "$id" \
        '{jsonrpc:"2.0",id:$id,error:{code:-32601,message:"unsupported client method"}}')"
      acp_trace_note "server_request_rejected" "$(jq -nc --arg m "$method" '{method:$m}')"
      ;;
  esac
}

# Send one request and drain all interleaved messages until its response.
# Args: <method> <params-json> [idle-timeout]. Sets RPC_RESPONSE / REPLY / STOP_REASON.
acp_rpc_long() {
  local method="$1" params="$2" timeout="${3:-$ACP_IDLE_TIMEOUT}"
  local id="$ACP_NEXT_ID" payload line chunk started delay_s
  ACP_NEXT_ID=$((ACP_NEXT_ID + 1))
  RPC_RESPONSE=""
  REPLY=""
  STOP_REASON=""
  payload="$(jq -nc --argjson id "$id" --arg method "$method" --argjson params "$params" \
    '{jsonrpc:"2.0",id:$id,method:$method,params:$params}')" || return 1
  acp_send "$payload" || return 1
  acp_trace_note "request_sent" "$(jq -nc --arg m "$method" --argjson id "$id" '{method:$m,id:$id}')"
  started=$SECONDS
  while IFS= read -r -t "$timeout" -u "$ACP_OUT_FD" line; do
    [[ -z "$line" ]] && continue
    acp_trace_line "$line"
    if (( ACP_READ_DELAY_MS > 0 )); then
      delay_s="$(printf '%d.%03d' "$(( ACP_READ_DELAY_MS / 1000 ))" "$(( ACP_READ_DELAY_MS % 1000 ))")"
      sleep "$delay_s"
    fi
    # A request from the agent (permission, or anything else) must be answered.
    if jq -e '(.method? // "") != "" and (.id? != null)' >/dev/null 2>&1 <<<"$line"; then
      acp_handle_server_request "$line" || return 1
      continue
    fi
    if jq -e --argjson id "$id" \
      '.id? == $id and ((has("result")) or (has("error")))' >/dev/null 2>&1 <<<"$line"; then
      if jq -e 'has("error")' >/dev/null 2>&1 <<<"$line"; then
        jq -r '.error.message // "ACP request failed"' >&2 <<<"$line"
        return 1
      fi
      RPC_RESPONSE="$line"
      STOP_REASON="$(jq -r '.result.stopReason // empty' <<<"$RPC_RESPONSE")"
      return 0
    fi
    chunk="$(jq -rj 'select(.method? == "session/update") |
      .params.update | select(.sessionUpdate == "agent_message_chunk") |
      .content | select(.type == "text") | .text // empty' <<<"$line" 2>/dev/null || true)"
    [[ -z "${chunk:-}" ]] || REPLY="${REPLY}${chunk}"
    if (( SECONDS - started > ACP_MAX_TURN_SECONDS )); then
      echo "ACP $method exceeded ACP_MAX_TURN_SECONDS=$ACP_MAX_TURN_SECONDS" >&2
      return 1
    fi
  done
  echo "ACP $method (id=$id) timed out after ${timeout}s of silence or disconnected" >&2
  return 1
}

# Send a notification (no id): session/cancel is the important one.
acp_notify() {
  local method="$1" params="$2"
  acp_send "$(jq -nc --arg method "$method" --argjson params "$params" \
    '{jsonrpc:"2.0",method:$method,params:$params}')" || return 1
  acp_trace_note "notification_sent" "$(jq -nc --arg m "$method" '{method:$m}')"
}

acp_init_long() {
  acp_rpc_long "initialize" \
    "$(jq -nc --argjson v "$ACP_PROTOCOL_VERSION" \
      '{protocolVersion:$v,clientCapabilities:{}}')" 60
}

# Args: [cwd] [mcpServers-json]. Sets SESSION_ID.
acp_new_long() {
  local cwd="${1:-/workspace}" mcp="${2:-[]}"
  if ! acp_rpc_long "session/new" \
    "$(jq -nc --arg cwd "$cwd" --argjson mcp "$mcp" '{cwd:$cwd,mcpServers:$mcp}')" 120; then
    return 1
  fi
  SESSION_ID="$(jq -r '.result.sessionId // empty' <<<"$RPC_RESPONSE")"
  [[ -n "$SESSION_ID" ]]
}

# Args: [cwd] [cursor]. Response in RPC_RESPONSE.
acp_list_long() {
  local cwd="${1:-}" cursor="${2:-}" params
  params="$(jq -nc --arg cwd "$cwd" --arg cursor "$cursor" \
    '(if $cwd != "" then {cwd:$cwd} else {} end) +
     (if $cursor != "" then {cursor:$cursor} else {} end)')"
  acp_rpc_long "session/list" "$params" 120
}

# Find a session across list pages. Args: <sessionId> [cwd] [max-pages].
# listSessions is paged, so asserting against page 1 alone is a false negative
# once the volume holds more than one page of sessions.
acp_list_find() {
  local sid="$1" cwd="${2:-}" max="${3:-30}" cursor="" page=0 params
  while (( page < max )); do
    params="$(jq -nc --arg c "$cwd" --arg cur "$cursor" \
      '(if $c != "" then {cwd:$c} else {} end) +
       (if $cur != "" then {cursor:$cur} else {} end)')"
    acp_rpc_long "session/list" "$params" 120 || return 1
    if jq -e --arg s "$sid" 'any(.result.sessions[]?; .sessionId == $s)' \
      >/dev/null 2>&1 <<<"$RPC_RESPONSE"; then
      return 0
    fi
    cursor="$(jq -r '.result.nextCursor // empty' <<<"$RPC_RESPONSE")"
    [[ -n "$cursor" ]] || return 1
    page=$((page + 1))
  done
  return 1
}

acp_close_long() {
  local sid="${1:-$SESSION_ID}"
  acp_rpc_long "session/close" "$(jq -nc --arg s "$sid" '{sessionId:$s}')" 120
}

# Args: <sessionId> <cwd> [mcpServers-json]
acp_resume_long() {
  local sid="$1" cwd="${2:-/workspace}" mcp="${3:-[]}"
  acp_rpc_long "session/resume" \
    "$(jq -nc --arg s "$sid" --arg c "$cwd" --argjson m "$mcp" \
      '{sessionId:$s,cwd:$c,mcpServers:$m}')" 180
}

# Args: <configId> <value-string> (ACP values are strings, even for model tuples).
acp_set_option_long() {
  local config_id="$1" value="$2"
  acp_rpc_long "session/set_config_option" \
    "$(jq -nc --arg s "$SESSION_ID" --arg c "$config_id" --arg v "$value" \
      '{sessionId:$s,configId:$c,value:$v}')" 120
}

# Args: <text> [idle-timeout]. Sets REPLY / STOP_REASON.
acp_prompt_long() {
  local text="$1" timeout="${2:-$ACP_IDLE_TIMEOUT}"
  acp_rpc_long "session/prompt" \
    "$(jq -nc --arg s "$SESSION_ID" --arg text "$text" \
      '{sessionId:$s,prompt:[{type:"text",text:$text}]}')" "$timeout"
}

# Args: <content-blocks-json> [idle-timeout]. For image/resource negative cases.
acp_prompt_blocks_long() {
  local blocks="$1" timeout="${2:-$ACP_IDLE_TIMEOUT}"
  acp_rpc_long "session/prompt" \
    "$(jq -nc --arg s "$SESSION_ID" --argjson blocks "$blocks" \
      '{sessionId:$s,prompt:$blocks}')" "$timeout"
}

acp_cancel_long() {
  local sid="${1:-$SESSION_ID}"
  acp_notify "session/cancel" "$(jq -nc --arg s "$sid" '{sessionId:$s}')"
}

# Aggregate a trace into compact JSON on stdout.
acp_trace_stats() {
  python3 - "${1:-$ACP_TRACE_FILE}" <<'PY'
import json, sys
from collections import Counter
from pathlib import Path
p = Path(sys.argv[1])
rows = []
if p.exists():
    for line in p.read_text(encoding="utf-8").splitlines():
        if line.strip():
            try:
                rows.append(json.loads(line))
            except Exception:
                pass
types = [r.get("sessionUpdate") or "unknown"
         for r in rows if r.get("method") == "session/update"]
first_order = []
for t in types:
    if t not in first_order:
        first_order.append(t)
counts = Counter(types)
stats = {
    "lines": len(rows),
    "noise": sum(1 for r in rows if r.get("kind") == "noise"),
    "updates": len(types),
    "byType": dict(counts),
    "firstOrder": first_order,
    "toolCalls": counts.get("tool_call", 0),
    "toolCallUpdates": counts.get("tool_call_update", 0),
    "chunks": counts.get("agent_message_chunk", 0),
    "thoughtChunks": counts.get("agent_thought_chunk", 0),
    "usageUpdates": counts.get("usage_update", 0),
    "permissionRequests": sum(1 for r in rows if r.get("method") == "session/request_permission"),
    "permissionAnswers": sum(1 for r in rows if r.get("kind") == "permission_answer"),
    "toolTitles": sorted({r.get("title") for r in rows
                          if r.get("sessionUpdate") == "tool_call" and r.get("title")}),
    "stopReasons": sorted({r.get("stopReason") for r in rows if r.get("stopReason")}),
    "errors": sum(1 for r in rows if r.get("error")),
    "lastUsed": max([r.get("used") or 0 for r in rows
                     if r.get("sessionUpdate") == "usage_update"] or [0]),
}
print(json.dumps(stats, ensure_ascii=False, separators=(",", ":")))
PY
}

# Assert a trace against a spec. Args: <jsonl-path> <spec-json>
# spec keys: require_types[], forbid_types[], require_order[], min{}, max{}.
acp_trace_check() {
  python3 - "${1:-$ACP_TRACE_FILE}" "$2" <<'PY'
import json, sys
from pathlib import Path
p, spec = Path(sys.argv[1]), json.loads(sys.argv[2])
rows = []
for line in p.read_text(encoding="utf-8").splitlines():
    if line.strip():
        try:
            rows.append(json.loads(line))
        except Exception:
            pass
types = [r.get("sessionUpdate") or "unknown"
         for r in rows if r.get("method") == "session/update"]
from collections import Counter
counts = Counter(types)
values = {
    "lines": len(rows),
    "noise": sum(1 for r in rows if r.get("kind") == "noise"),
    "updates": len(types),
    "toolCalls": counts.get("tool_call", 0),
    "toolCallUpdates": counts.get("tool_call_update", 0),
    "chunks": counts.get("agent_message_chunk", 0),
    "usageUpdates": counts.get("usage_update", 0),
    "permissionRequests": sum(1 for r in rows if r.get("method") == "session/request_permission"),
    "permissionAnswers": sum(1 for r in rows if r.get("kind") == "permission_answer"),
    "errors": sum(1 for r in rows if r.get("error")),
}
problems = []
for t in spec.get("require_types", []):
    if t not in types:
        problems.append(f"missing update type: {t}")
for t in spec.get("forbid_types", []):
    if t in types:
        problems.append(f"unexpected update type: {t}")
order = spec.get("require_order", [])
if order:
    cursor = 0
    for want in order:
        try:
            cursor = types.index(want, cursor) + 1
        except ValueError:
            problems.append(f"order broken at: {want}")
            break
for key, floor in spec.get("min", {}).items():
    if values.get(key, 0) < floor:
        problems.append(f"{key}={values.get(key, 0)} < {floor}")
for key, ceiling in spec.get("max", {}).items():
    if values.get(key, 0) > ceiling:
        problems.append(f"{key}={values.get(key, 0)} > {ceiling}")
if problems:
    print("trace check failed: " + "; ".join(problems), file=sys.stderr)
    print(json.dumps(values), file=sys.stderr)
    raise SystemExit(1)
print(json.dumps(values, ensure_ascii=False, separators=(",", ":")))
PY
}

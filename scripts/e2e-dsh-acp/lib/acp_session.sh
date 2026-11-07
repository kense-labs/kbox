#!/usr/bin/env bash
# Minimal ACP JSON-RPC session over fifos attached to `kbox pool attach` or `kbox run -i`.
# Requires: jq, bash, open fds 3 (write to agent stdin) and 4 (read agent stdout).

acp_rpc() {
  local id="$1" payload="$2" timeout="${3:-60}"
  local line chunk
  RPC_RESPONSE=""
  printf '%s\n' "$payload" >&3
  while IFS= read -r -t "$timeout" -u 4 line; do
    [[ "$line" == \{* ]] || continue
    if jq -e --argjson id "$id" '.id? == $id' >/dev/null 2>&1 <<<"$line"; then
      if jq -e 'has("error")' >/dev/null 2>&1 <<<"$line"; then
        jq -r '.error.message // "ACP request failed"' >&2 <<<"$line"
        return 1
      fi
      RPC_RESPONSE="$line"
      return 0
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
  acp_rpc 5 "$(jq -nc --arg session "$SESSION_ID" --arg text "$text" \
    '{jsonrpc:"2.0",id:5,method:"session/prompt",params:{sessionId:$session,prompt:[{type:"text",text:$text}]}}')" "$timeout"
  STOP_REASON="$(jq -r '.result.stopReason // empty' <<<"$RPC_RESPONSE")"
  [[ -n "$REPLY" && "$STOP_REASON" == "end_turn" ]]
}

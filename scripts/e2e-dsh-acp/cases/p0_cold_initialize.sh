#!/usr/bin/env bash
# P0-2: cold kbox run -i --rm + ACP initialize only.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
# shellcheck source=../lib/acp_session.sh
source "$SCRIPT_DIR/../lib/acp_session.sh"

CASE_ID="P0-2"
if [[ -z "${DEEPSEEK_API_KEY:-}" ]]; then
  e2e_record_result "$CASE_ID" "BLOCKED" "DEEPSEEK_API_KEY not set"
  exit 0
fi

e2e_require_cmds jq
IMAGE="${IMAGE:-ghcr.io/kense-labs/dsh-acp:latest}"
WORK="$E2E_OUT/p0-cold"
mkdir -p "$WORK"
mkfifo "$WORK/acp.in" "$WORK/acp.out"

ACP_PID=""
cleanup() {
  set +e
  exec 3>&- 2>/dev/null || true
  exec 4>&- 2>/dev/null || true
  [[ -z "${ACP_PID:-}" ]] || kill "$ACP_PID" 2>/dev/null || true
  [[ -z "${ACP_PID:-}" ]] || wait "$ACP_PID" 2>/dev/null || true
}
trap cleanup EXIT

exec 3<>"$WORK/acp.in"
exec 4<>"$WORK/acp.out"

"$KBOX" run -i --rm \
  --memory 2g \
  -u root \
  -e DEEPSEEK_API_KEY \
  "$IMAGE" <"$WORK/acp.in" >"$WORK/acp.out" 3>&- 4>&- &
ACP_PID=$!

# Cold MicroVM boot + agent stdio can take well over 8s on first rootfs.
sleep 20

# Longer RPC window than warm-pool path.
if ! acp_rpc 1 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}' 120; then
  e2e_record_result "$CASE_ID" "FAIL" "initialize failed"
  exit 1
fi

AGENT="$(jq -r '.result.agentInfo.name // .result.agentInfo // "unknown"' <<<"$RPC_RESPONSE")"
PROTO="$(jq -r '.result.protocolVersion // empty' <<<"$RPC_RESPONSE")"
exec 3>&-
# Closing stdin should end the session; wait briefly then ensure cleanup.
sleep 2
kill "$ACP_PID" 2>/dev/null || true
wait "$ACP_PID" 2>/dev/null || true
ACP_PID=""

running="$(e2e_running_box_count)"
if [[ "$running" != "0" ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "running boxes remain after cold run: $running"
  exit 1
fi

e2e_record_result "$CASE_ID" "PASS" "initialize ok agent=$AGENT protocol=$PROTO" \
  "$(jq -nc --arg agent "$AGENT" --arg proto "$PROTO" '{agent:$agent,protocolVersion:$proto}')"

#!/usr/bin/env bash
# S-REDIS: SET/GET/DEL over host-mapped port; then AOF restart persistence check.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

CASE_ID="S-REDIS"
IMAGE="${REDIS_IMAGE:-docker.io/library/redis:7-alpine}"
NAME="e2e-redis-$$"
PORT="$(e2e_free_port)"
VOL="e2e-redis-data-$$"
KEY="e2e:kbox:$$"
VAL="redis-value-$$-$(date +%s)"
RESP="$SCRIPT_DIR/../lib/redis_resp.py"
WORK="$E2E_OUT/redis"
mkdir -p "$WORK"

cleanup() {
  e2e_rm_box "$NAME"
  "$KBOX" volume rm "$VOL" >/dev/null 2>&1 || true
}
trap cleanup EXIT

"$KBOX" volume create "$VOL" >/dev/null

set +e
"$KBOX" run -d --name "$NAME" --memory "${REDIS_MEMORY:-512m}" \
  -p "${PORT}:6379" \
  -v "${VOL}:/data" \
  "$IMAGE" -- redis-server --appendonly yes >"$WORK/run.log" 2>&1
rc=$?
set -e
if [[ $rc -ne 0 ]] || ! e2e_wait_running "$NAME" 90; then
  e2e_record_result "$CASE_ID" "FAIL" "redis failed to become running (see redis/run.log)"
  exit 1
fi

if ! e2e_wait_tcp 127.0.0.1 "$PORT" 60; then
  e2e_record_result "$CASE_ID" "FAIL" "redis TCP :$PORT never accepted connections"
  exit 1
fi

set +e
pong="$(python3 "$RESP" 127.0.0.1 "$PORT" PING)"
set_rc="$(python3 "$RESP" 127.0.0.1 "$PORT" SET "$KEY" "$VAL")"
got="$(python3 "$RESP" 127.0.0.1 "$PORT" GET "$KEY")"
set -e
if [[ "$pong" != "PONG" || "$set_rc" != "OK" || "$got" != "$VAL" ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "SET/GET failed (PING=$pong SET=$set_rc GET=$got)"
  exit 1
fi

# Persistence: stop+rm box, recreate same volume, GET must still return VAL.
"$KBOX" rm -f "$NAME" >/dev/null
set +e
"$KBOX" run -d --name "$NAME" --memory "${REDIS_MEMORY:-512m}" \
  -p "${PORT}:6379" \
  -v "${VOL}:/data" \
  "$IMAGE" -- redis-server --appendonly yes >"$WORK/run-restart.log" 2>&1
rc=$?
set -e
if [[ $rc -ne 0 ]] || ! e2e_wait_running "$NAME" 90 || ! e2e_wait_tcp 127.0.0.1 "$PORT" 60; then
  e2e_record_result "$CASE_ID" "FAIL" "redis restart on volume failed"
  exit 1
fi

got2="$(python3 "$RESP" 127.0.0.1 "$PORT" GET "$KEY")"
del_rc="$(python3 "$RESP" 127.0.0.1 "$PORT" DEL "$KEY")"
if [[ "$got2" != "$VAL" ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "AOF persistence miss after restart (GET=$got2)"
  exit 1
fi

e2e_record_result "$CASE_ID" "PASS" \
  "host SET/GET/DEL + AOF restart persistence ok on :$PORT (volume $VOL)" \
  "$(jq -nc --arg port "$PORT" --arg image "$IMAGE" --arg vol "$VOL" --arg key "$KEY" \
    --argjson del "$del_rc" \
    '{port:$port,image:$image,volume:$vol,key:$key,del:$del}')"

cleanup
trap - EXIT

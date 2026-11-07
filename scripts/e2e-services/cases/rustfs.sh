#!/usr/bin/env bash
# S-RUSTFS: health + S3 CreateBucket/PutObject/GetObject over host-mapped API port.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

CASE_ID="S-RUSTFS"
IMAGE="${RUSTFS_IMAGE:-docker.io/rustfs/rustfs:latest}"
NAME="e2e-rustfs-$$"
PORT_API="$(e2e_free_port)"
PORT_CONSOLE="$(e2e_free_port)"
VOL="e2e-rustfs-data-$$"
AK="e2eaccesskey$$"
SK="e2esecretkey$$"
BUCKET="e2ebucket$$"
KEY="hello.txt"
BODY="rustfs-e2e-$$-$(date +%s)"
S3="$SCRIPT_DIR/../lib/rustfs_s3.py"
WORK="$E2E_OUT/rustfs"
mkdir -p "$WORK"

cleanup() {
  e2e_rm_box "$NAME"
  "$KBOX" volume rm "$VOL" >/dev/null 2>&1 || true
}
trap cleanup EXIT

"$KBOX" volume create "$VOL" >/dev/null

set +e
"$KBOX" run -d --name "$NAME" --memory "${RUSTFS_MEMORY:-1g}" \
  -p "${PORT_API}:9000" \
  -p "${PORT_CONSOLE}:9001" \
  -v "${VOL}:/data" \
  -e "RUSTFS_ACCESS_KEY=${AK}" \
  -e "RUSTFS_SECRET_KEY=${SK}" \
  -e 'RUSTFS_ADDRESS=:9000' \
  -e 'RUSTFS_CONSOLE_ADDRESS=:9001' \
  -e RUSTFS_CONSOLE_ENABLE=true \
  -e RUSTFS_OBS_LOGGER_LEVEL=error \
  -e 'RUSTFS_OBS_LOG_DIRECTORY=/tmp/rustfs-logs' \
  "$IMAGE" \
  -- /data >"$WORK/run.log" 2>&1
rc=$?
set -e
if [[ $rc -ne 0 ]] || ! e2e_wait_running "$NAME" 120; then
  e2e_record_result "$CASE_ID" "FAIL" "rustfs failed to become running (see rustfs/run.log)"
  exit 1
fi

body=""
set +e
body="$(e2e_wait_http "http://127.0.0.1:${PORT_API}/health" "" 90)"
http_rc=$?
set -e
if [[ $http_rc -ne 0 ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "rustfs /health failed on host :$PORT_API"
  exit 1
fi

ENDPOINT="http://127.0.0.1:${PORT_API}"
# /health can succeed before storage quorum; retry CreateBucket through 503s.
mb_rc=1
for _ in $(seq 1 45); do
  set +e
  python3 "$S3" mb "$ENDPOINT" "$AK" "$SK" "$BUCKET" >"$WORK/s3-mb.out" 2>"$WORK/s3-mb.err"
  mb_rc=$?
  set -e
  if [[ $mb_rc -eq 0 ]]; then
    break
  fi
  if grep -Fq 'waiting for storage_quorum' "$WORK/s3-mb.err" 2>/dev/null || \
     grep -Eq '^503$' "$WORK/s3-mb.out" 2>/dev/null; then
    sleep 2
    continue
  fi
  break
done

set +e
python3 "$S3" put "$ENDPOINT" "$AK" "$SK" "$BUCKET" "$KEY" "$BODY" >"$WORK/s3-put.out" 2>"$WORK/s3-put.err"
put_rc=$?
got="$(python3 "$S3" get "$ENDPOINT" "$AK" "$SK" "$BUCKET" "$KEY" 2>"$WORK/s3-get.err")"
get_rc=$?
set -e

if [[ $mb_rc -ne 0 || $put_rc -ne 0 || $get_rc -ne 0 || "$got" != "$BODY" ]]; then
  e2e_record_result "$CASE_ID" "FAIL" \
    "S3 mb/put/get failed (mb=$mb_rc put=$put_rc get=$get_rc got='${got:0:80}')"
  exit 1
fi

e2e_record_result "$CASE_ID" "PASS" \
  "host /health + S3 CreateBucket/Put/Get ok on :$PORT_API (volume $VOL)" \
  "$(jq -nc --arg api "$PORT_API" --arg console "$PORT_CONSOLE" --arg image "$IMAGE" \
    --arg vol "$VOL" --arg bucket "$BUCKET" --arg key "$KEY" \
    '{api_port:$api,console_port:$console,image:$image,volume:$vol,bucket:$bucket,key:$key}')"

cleanup
trap - EXIT

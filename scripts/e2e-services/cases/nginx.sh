#!/usr/bin/env bash
# S-NGINX: serve a custom index via bind-mount; host GET must return that body.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

CASE_ID="S-NGINX"
IMAGE="${NGINX_IMAGE:-docker.io/library/nginx:alpine}"
NAME="e2e-nginx-$$"
PORT="$(e2e_free_port)"
WORK="$E2E_OUT/nginx"
MARKER="kbox-e2e-nginx-$$-$(date +%s)"
mkdir -p "$WORK/www"
printf '%s\n' "$MARKER" >"$WORK/www/index.html"

cleanup() { e2e_rm_box "$NAME"; }
trap cleanup EXIT

set +e
"$KBOX" run -d --name "$NAME" --memory "${NGINX_MEMORY:-512m}" \
  -p "${PORT}:80" \
  -v "$WORK/www:/usr/share/nginx/html" \
  "$IMAGE" >"$WORK/run.log" 2>&1
rc=$?
set -e
if [[ $rc -ne 0 ]] || ! e2e_wait_running "$NAME" 90; then
  e2e_record_result "$CASE_ID" "FAIL" "nginx failed to become running (see nginx/run.log)"
  exit 1
fi

body=""
set +e
body="$(e2e_wait_http "http://127.0.0.1:${PORT}/" "$MARKER" 60)"
http_rc=$?
set -e
if [[ $http_rc -ne 0 ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "host GET did not return custom marker from bind-mounted index"
  exit 1
fi

# Second path: confirm default site is overridden (not stock welcome page).
if grep -Fq "Welcome to nginx" <<<"$body"; then
  e2e_record_result "$CASE_ID" "FAIL" "still serving stock nginx welcome page"
  exit 1
fi

ports="$("$KBOX" port "$NAME" 2>/dev/null || true)"
e2e_record_result "$CASE_ID" "PASS" \
  "host GET :$PORT returned bind-mounted marker; ports=$ports" \
  "$(jq -nc --arg port "$PORT" --arg image "$IMAGE" --arg marker "$MARKER" \
    '{port:$port,image:$image,marker:$marker}')"

cleanup
trap - EXIT

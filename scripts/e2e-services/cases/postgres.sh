#!/usr/bin/env bash
# S-PG: CREATE/INSERT/SELECT round-trip inside postgres; host TCP must be open.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

CASE_ID="S-PG"
IMAGE="${POSTGRES_IMAGE:-docker.io/library/postgres:16-alpine}"
NAME="e2e-pg-$$"
PORT="$(e2e_free_port)"
VOL="e2e-pg-data-$$"
PASS="e2e_pg_pass_$$"
NOTE="pg-note-$$-$(date +%s)"
WORK="$E2E_OUT/postgres"
mkdir -p "$WORK"

cleanup() {
  e2e_rm_box "$NAME"
  "$KBOX" volume rm "$VOL" >/dev/null 2>&1 || true
}
trap cleanup EXIT

"$KBOX" volume create "$VOL" >/dev/null

set +e
"$KBOX" run -d --name "$NAME" --memory "${POSTGRES_MEMORY:-1g}" \
  -p "${PORT}:5432" \
  -v "${VOL}:/var/lib/postgresql/data" \
  -e "POSTGRES_PASSWORD=${PASS}" \
  -e POSTGRES_USER=e2e \
  -e POSTGRES_DB=e2e \
  "$IMAGE" >"$WORK/run.log" 2>&1
rc=$?
set -e
if [[ $rc -ne 0 ]] || ! e2e_wait_running "$NAME" 120; then
  e2e_record_result "$CASE_ID" "FAIL" "postgres failed to become running (see postgres/run.log)"
  exit 1
fi

# pg_isready only means the postmaster accepts connections; wait until DB e2e exists.
ready=0
for _ in $(seq 1 90); do
  if "$KBOX" exec "$NAME" -- \
    psql -U e2e -d e2e -tAc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 2
done
if [[ "$ready" -ne 1 ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "database e2e never became queryable"
  exit 1
fi

if ! e2e_wait_tcp 127.0.0.1 "$PORT" 30; then
  e2e_record_result "$CASE_ID" "FAIL" "postgres host TCP :$PORT not accepting"
  exit 1
fi

DDL=$(cat <<SQL
CREATE TABLE IF NOT EXISTS e2e_items (
  id INT PRIMARY KEY,
  note TEXT NOT NULL
);
INSERT INTO e2e_items(id, note) VALUES (1, '${NOTE}')
  ON CONFLICT (id) DO UPDATE SET note = EXCLUDED.note;
SQL
)
set +e
"$KBOX" exec "$NAME" -- \
  psql -U e2e -d e2e -v ON_ERROR_STOP=1 -c "$DDL" >"$WORK/psql-ddl.out" 2>"$WORK/psql.err"
ddl_rc=$?
set -e
if [[ $ddl_rc -ne 0 ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "DDL failed (see postgres/psql.err)"
  exit 1
fi

out="$("$KBOX" exec "$NAME" -- \
  psql -U e2e -d e2e -tAc "SELECT note FROM e2e_items WHERE id = 1" 2>>"$WORK/psql.err" | tr -d '[:space:]')"
if [[ "$out" != "$NOTE" ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "CREATE/INSERT/SELECT mismatch (got='$out', see postgres/psql.err)"
  exit 1
fi

# Restart on same volume — row must survive.
"$KBOX" rm -f "$NAME" >/dev/null
set +e
"$KBOX" run -d --name "$NAME" --memory "${POSTGRES_MEMORY:-1g}" \
  -p "${PORT}:5432" \
  -v "${VOL}:/var/lib/postgresql/data" \
  -e "POSTGRES_PASSWORD=${PASS}" \
  -e POSTGRES_USER=e2e \
  -e POSTGRES_DB=e2e \
  "$IMAGE" >"$WORK/run-restart.log" 2>&1
rc=$?
set -e
ready=0
if [[ $rc -eq 0 ]] && e2e_wait_running "$NAME" 120; then
  for _ in $(seq 1 90); do
    if "$KBOX" exec "$NAME" -- \
      psql -U e2e -d e2e -tAc 'SELECT 1' >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 2
  done
fi
if [[ "$ready" -ne 1 ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "postgres restart on volume failed"
  exit 1
fi

out2="$("$KBOX" exec "$NAME" -- \
  psql -U e2e -d e2e -tAc "SELECT note FROM e2e_items WHERE id = 1" 2>>"$WORK/psql.err" | tr -d '[:space:]')"
if [[ "$out2" != "$NOTE" ]]; then
  e2e_record_result "$CASE_ID" "FAIL" "volume persistence miss after restart (got='$out2')"
  exit 1
fi

e2e_record_result "$CASE_ID" "PASS" \
  "host TCP :$PORT + CREATE/INSERT/SELECT + volume restart ok" \
  "$(jq -nc --arg port "$PORT" --arg image "$IMAGE" --arg vol "$VOL" --arg note "$NOTE" \
    '{port:$port,image:$image,volume:$vol,note:$note}')"

cleanup
trap - EXIT

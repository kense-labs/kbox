#!/usr/bin/env bash
# Shared helpers for kbox service-deployment E2E.

set -euo pipefail

e2e_log() { printf '[e2e-svc] %s\n' "$*" >&2; }

e2e_require_cmds() {
  local c
  for c in "$@"; do
    command -v "$c" >/dev/null 2>&1 || {
      e2e_log "missing required command: $c"
      return 1
    }
  done
}

e2e_require_kbox() {
  command -v "$KBOX" >/dev/null 2>&1 || {
    e2e_log "kbox not found: $KBOX"
    return 1
  }
}

e2e_record_result() {
  local id="$1" status="$2" message="$3" detail="${4:-null}"
  RESULTS_JSONL="${RESULTS_JSONL:-$E2E_OUT/results.jsonl}"
  python3 - "$RESULTS_JSONL" "$id" "$status" "$message" "$detail" <<'PY'
import json, sys, time
path, case_id, status, message, detail = sys.argv[1:6]
try:
    detail_obj = json.loads(detail)
except Exception:
    detail_obj = {"raw": detail}
row = {
    "id": case_id,
    "status": status,
    "message": message,
    "detail": detail_obj,
    "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(path, "a", encoding="utf-8") as fh:
    fh.write(json.dumps(row, ensure_ascii=False) + "\n")
print(f"{case_id}: {status} — {message}", file=sys.stderr)
PY
}

e2e_finalize_results() {
  local jsonl="${RESULTS_JSONL:-$E2E_OUT/results.jsonl}"
  local out="${E2E_OUT}/results.json"
  python3 - "$jsonl" "$out" <<'PY'
import json, sys
from pathlib import Path
jsonl, out = Path(sys.argv[1]), Path(sys.argv[2])
rows = []
if jsonl.exists():
    for line in jsonl.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            rows.append(json.loads(line))
summary = {
    "pass": sum(1 for r in rows if r["status"] == "PASS"),
    "fail": sum(1 for r in rows if r["status"] == "FAIL"),
    "skip": sum(1 for r in rows if r["status"] == "SKIP"),
    "blocked": sum(1 for r in rows if r["status"] == "BLOCKED"),
}
Path(out).write_text(
    json.dumps({"summary": summary, "cases": rows}, indent=2, ensure_ascii=False) + "\n",
    encoding="utf-8",
)
print(json.dumps(summary), file=sys.stderr)
PY
}

e2e_free_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

e2e_wait_running() {
  local name="$1" attempts="${2:-60}"
  local i=0
  while ((i < attempts)); do
    if "$KBOX" ps --format json 2>/dev/null | python3 -c "
import json,sys
name=sys.argv[1]
try:
  d=json.load(sys.stdin)
except Exception:
  raise SystemExit(1)
items=d if isinstance(d,list) else (d or {}).get('boxes', (d or {}).get('items', []))
for b in items:
  n=str(b.get('name') or b.get('Names') or '')
  st=str(b.get('status') or b.get('Status') or '').lower()
  if name in n and 'running' in st:
    raise SystemExit(0)
raise SystemExit(1)
" "$name"; then
      return 0
    fi
    # Fallback: table parse (older/newer CLI flag drift).
    if "$KBOX" ps 2>/dev/null | awk -v n="$name" 'NR>1 && index($0,n) && tolower($0) ~ /running/ {found=1} END{exit !found}'; then
      return 0
    fi
    sleep 2
    i=$((i + 1))
  done
  return 1
}

e2e_wait_http() {
  local url="$1" needle="${2:-}" attempts="${3:-45}"
  local i=0 body
  while ((i < attempts)); do
    if body="$(curl -fsS -m 3 "$url" 2>/dev/null)"; then
      if [[ -z "$needle" ]] || grep -Fq "$needle" <<<"$body"; then
        printf '%s' "$body"
        return 0
      fi
    fi
    sleep 2
    i=$((i + 1))
  done
  return 1
}

e2e_wait_tcp() {
  local host="$1" port="$2" attempts="${3:-45}"
  local i=0
  while ((i < attempts)); do
    if python3 - "$host" "$port" <<'PY'
import socket,sys
host,port=sys.argv[1],int(sys.argv[2])
s=socket.socket()
s.settimeout(2)
try:
  s.connect((host,port))
except Exception:
  raise SystemExit(1)
finally:
  s.close()
raise SystemExit(0)
PY
    then
      return 0
    fi
    sleep 2
    i=$((i + 1))
  done
  return 1
}

e2e_rm_box() {
  local name="$1"
  "$KBOX" rm -f "$name" >/dev/null 2>&1 || true
}

e2e_running_box_count() {
  local n
  n="$("$KBOX" ps --format json 2>/dev/null | python3 -c "
import json,sys
try:
  d=json.load(sys.stdin)
except Exception:
  print(0); raise SystemExit(0)
items=d if isinstance(d,list) else (d or {}).get('boxes', (d or {}).get('items', []))
if not isinstance(items, list):
  items=[]
print(sum(1 for b in items if 'running' in str((b or {}).get('status',(b or {}).get('Status',''))).lower()))
" 2>/dev/null || true)"
  if [[ -z "${n:-}" ]]; then
    n="$("$KBOX" ps 2>/dev/null | awk 'NR>1 && tolower($0) ~ /running/ {c++} END{print c+0}')"
  fi
  n="$(printf '%s' "${n:-0}" | tr -d '[:space:]')"
  [[ "$n" =~ ^[0-9]+$ ]] || n=0
  printf '%s\n' "$n"
}

#!/usr/bin/env bash
# Shared helpers for dsh-acp × kbox E2E.

set -euo pipefail

e2e_log() { printf '[e2e] %s\n' "$*" >&2; }

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
  case "$("$KBOX" pool start --help 2>&1)" in
    *--prewarm*) ;;
    *)
      e2e_log "kbox too old: need pool start --prewarm"
      return 1
      ;;
  esac
  "$KBOX" pool attach --help >/dev/null 2>&1 || {
    e2e_log "kbox too old: need pool attach"
    return 1
  }
}

e2e_record_result() {
  # Args: id status message [detail_json]
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
payload = {
    "kbox": None,
    "image": None,
    "summary": summary,
    "cases": rows,
}
out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(json.dumps(summary), file=sys.stderr)
PY
}

e2e_wait_pool_idle() {
  local socket="$1" attempts="${2:-90}"
  local i=0
  while ((i < attempts)); do
    if "$KBOX" pool status --socket "$socket" --json 2>/dev/null |
      python3 -c "import json,sys; d=json.load(sys.stdin); sys.exit(0 if d and d[0].get('idle',0)>=1 else 1)"; then
      return 0
    fi
    sleep 2
    i=$((i + 1))
  done
  return 1
}

e2e_image_digest_hint() {
  # Best-effort: inspect local image metadata if available; else record tag only.
  local image="$1"
  if "$KBOX" image inspect "$image" --json >/tmp/e2e-image.json 2>/dev/null; then
    python3 - <<'PY'
import json
from pathlib import Path
p = Path("/tmp/e2e-image.json")
try:
    data = json.loads(p.read_text())
except Exception:
    print("unknown")
    raise SystemExit(0)
if isinstance(data, list) and data:
    data = data[0]
digest = data.get("digest") or data.get("Id") or data.get("id") or "unknown"
print(digest)
PY
  else
    echo "tag-only:${image}"
  fi
}

e2e_running_box_count() {
  local n
  n="$("$KBOX" ps --json 2>/dev/null | python3 -c "import json,sys
try:
  d=json.load(sys.stdin)
except Exception:
  print(0); raise SystemExit(0)
items=d if isinstance(d,list) else (d or {}).get('boxes', (d or {}).get('items', []))
if not isinstance(items, list):
  items=[]
print(sum(1 for b in items if str((b or {}).get('status','')).lower() in ('running','paused')))" 2>/dev/null || true)"
  n="$(printf '%s' "${n:-0}" | tr -d '[:space:]')"
  [[ "$n" =~ ^[0-9]+$ ]] || n=0
  printf '%s\n' "$n"
}

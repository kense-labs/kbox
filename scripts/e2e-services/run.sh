#!/usr/bin/env bash
# kbox service-deployment E2E orchestrator (nginx / redis / postgres / rustfs).
#
#   TIER=all ./scripts/e2e-services/run.sh
#   TIER=nginx,redis ./scripts/e2e-services/run.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

export KBOX="${KBOX:-kbox}"
export TIER="${TIER:-all}"
export E2E_RUN_ID="${E2E_RUN_ID:-$(date +%Y%m%d-%H%M%S)-$$}"
export E2E_OUT="${E2E_OUT:-$ROOT/docs/evidence/box/${E2E_RUN_ID}-services-e2e}"
mkdir -p "$E2E_OUT"
export RESULTS_JSONL="$E2E_OUT/results.jsonl"
: >"$RESULTS_JSONL"

export K_HOME="${E2E_K_HOME:-$E2E_OUT/k-home}"
mkdir -p "$K_HOME"

e2e_log "ROOT=$ROOT"
e2e_log "KBOX=$KBOX ($("$KBOX" version 2>/dev/null || echo unknown))"
e2e_log "K_HOME=$K_HOME"
e2e_log "TIER=$TIER"
e2e_log "E2E_OUT=$E2E_OUT"

e2e_require_cmds bash curl python3 jq
e2e_require_kbox
"$KBOX" version >"$E2E_OUT/kbox-version.txt" 2>&1 || true

want() {
  local t="$1"
  [[ "$TIER" == "all" ]] && return 0
  [[ ",$TIER," == *",$t,"* ]] && return 0
  return 1
}

FAIL=0
run_one() {
  local name="$1" script="$2"
  e2e_log "running $name"
  if bash "$script"; then
    :
  else
    FAIL=1
  fi
}

want nginx && run_one nginx "$SCRIPT_DIR/cases/nginx.sh"
want redis && run_one redis "$SCRIPT_DIR/cases/redis.sh"
want postgres && run_one postgres "$SCRIPT_DIR/cases/postgres.sh"
want rustfs && run_one rustfs "$SCRIPT_DIR/cases/rustfs.sh"

e2e_finalize_results

python3 - "$E2E_OUT/results.json" "$KBOX" "$TIER" <<'PY'
import json, subprocess, sys
from pathlib import Path
path, kbox, tier = sys.argv[1:4]
data = json.loads(Path(path).read_text(encoding="utf-8"))
try:
    ver = subprocess.check_output([kbox, "version"], text=True).strip()
except Exception:
    ver = "unknown"
data["kbox"] = ver
data["tier"] = tier
Path(path).write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY

e2e_log "results: $E2E_OUT/results.json"
if [[ "$FAIL" -ne 0 ]]; then
  e2e_log "one or more case scripts failed"
  exit 1
fi

python3 - "$E2E_OUT/results.json" <<'PY'
import json, sys
from pathlib import Path
data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
fails = [c for c in data.get("cases", []) if c.get("status") == "FAIL"]
if fails:
    print("FAIL cases:", ", ".join(c["id"] for c in fails), file=sys.stderr)
    raise SystemExit(1)
print("summary:", json.dumps(data.get("summary", {})), file=sys.stderr)
PY

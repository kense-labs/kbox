#!/usr/bin/env bash
# dsh-acp × kbox end-to-end orchestrator.
#
# Usage:
#   TIER=p0 ./scripts/e2e-dsh-acp/run.sh
#   TIER=p0,p1 DEEPSEEK_API_KEY=… ./scripts/e2e-dsh-acp/run.sh
#   TIER=p4 DEEPSEEK_API_KEY=… ./scripts/e2e-dsh-acp/run.sh
#   E2E_LONGRUN=1 TIER=p5 DEEPSEEK_API_KEY=… ./scripts/e2e-dsh-acp/run.sh
#   TIER=p6 DEEPSEEK_API_KEY=… ./scripts/e2e-dsh-acp/run.sh
#   E2E_CAPABILITY_SURFACE=1 TIER=p4 DEEPSEEK_API_KEY=… ./scripts/e2e-dsh-acp/run.sh
#   TIER=all ./scripts/e2e-dsh-acp/run.sh
#
# Tiers:
#   p0/p1/p2/p3  smoke, core ACP, mounts/cleanup, negative (pre-existing)
#   p4           required live-model task acceptance (cases/p4_tasks.py)
#   p5           long-run / multi-turn (nightly; SKIP unless E2E_LONGRUN=1)
#   p6           complex tool-heavy tasks
#
# Env:
#   KBOX, IMAGE, DSH_MODEL, DSH_REASONING_EFFORT, GREETING, E2E_OUT
#   DEEPSEEK_API_KEY  — required for live cases (otherwise BLOCKED)
#   E2E_CAPABILITY_SURFACE=1 — also run the optional shell P4 capability
#                             surface; its P4- rows join the required gate,
#                             so only enable it with live credentials
#   E2E_LONGRUN=1     — enable p5 (real tokens, >=15 min)
#   LONGRUN_TURNS / LONGRUN_MIN_SECONDS / LONGRUN_MIN_CHUNKS /
#   LONGRUN_MIN_TOOL_CALLS / E2E_LONGRUN_ENFORCE — p5 thresholds
#   E2E_MCP_STDIO_COMMAND / E2E_MCP_HTTP_URL    — p6 positive MCP transports
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

export KBOX="${KBOX:-kbox}"
export IMAGE="${IMAGE:-ghcr.io/kense-labs/dsh-acp:latest}"
export DSH_MODEL="${DSH_MODEL:-deepseek-flash}"
export DSH_REASONING_EFFORT="${DSH_REASONING_EFFORT:-low}"
export TIER="${TIER:-p0}"
export E2E_RUN_ID="${E2E_RUN_ID:-$(date +%Y%m%d-%H%M%S)-$$}"
export E2E_OUT="${E2E_OUT:-$ROOT/docs/evidence/box/${E2E_RUN_ID}-dsh-acp-e2e}"
mkdir -p "$E2E_OUT"
export RESULTS_JSONL="$E2E_OUT/results.jsonl"
if ! (set -C; : >"$RESULTS_JSONL"); then
  e2e_log "cannot create results.jsonl; use a fresh writable E2E_OUT to preserve earlier evidence"
  exit 1
fi

# Isolated K_HOME so the suite never touches the developer's default boxes.
# Prefer E2E_OUT path; ignore ambient K_HOME unless E2E_K_HOME is set explicitly.
export K_HOME="${E2E_K_HOME:-$E2E_OUT/k-home}"
mkdir -p "$K_HOME"

e2e_log "ROOT=$ROOT"
e2e_log "KBOX=$KBOX ($("$KBOX" version 2>/dev/null || echo unknown))"
e2e_log "K_HOME=$K_HOME"
e2e_log "IMAGE=$IMAGE"
e2e_log "TIER=$TIER"
e2e_log "E2E_OUT=$E2E_OUT"

e2e_require_cmds bash curl python3
e2e_require_kbox

if ! DIGEST="$(e2e_image_digest_hint "$IMAGE" 2>/dev/null)"; then
  "$KBOX" pull "$IMAGE" >"$E2E_OUT/image-pull.log" 2>&1
  DIGEST="$(e2e_image_digest_hint "$IMAGE")"
fi
printf '%s\n' "$DIGEST" >"$E2E_OUT/image-digest.txt"
"$KBOX" version >"$E2E_OUT/kbox-version.txt" 2>&1 || true

want_tier() {
  local t="$1"
  [[ "$TIER" == "all" ]] && return 0
  [[ ",$TIER," == *",$t,"* ]] && return 0
  return 1
}

run_case_script() {
  local script="$1"
  e2e_log "running $(basename "$script")"
  # shellcheck disable=SC1090
  bash "$script"
}

FAIL=0
if want_tier p0; then
  run_case_script "$SCRIPT_DIR/cases/p0_bench.sh" || FAIL=1
  run_case_script "$SCRIPT_DIR/cases/p0_cold_initialize.sh" || FAIL=1
fi
if want_tier p1; then
  run_case_script "$SCRIPT_DIR/cases/p1_core.sh" || FAIL=1
fi
if want_tier p2; then
  run_case_script "$SCRIPT_DIR/cases/p2_mounts_cleanup.sh" || FAIL=1
fi
if want_tier p3; then
  run_case_script "$SCRIPT_DIR/cases/p3_negative.sh" || FAIL=1
fi
if want_tier p4; then
  python3 "$SCRIPT_DIR/cases/p4_tasks.py" || FAIL=1
fi
if want_tier p5; then
  run_case_script "$SCRIPT_DIR/cases/p5_longrun.sh" || FAIL=1
fi
if want_tier p6; then
  run_case_script "$SCRIPT_DIR/cases/p6_complex.sh" || FAIL=1
fi
# The shell capability surface also emits P4- rows. Keep it opt-in so its
# BLOCKED/SKIP states can never weaken the required p4 acceptance gate.
if [[ "${E2E_CAPABILITY_SURFACE:-0}" == "1" ]] && want_tier p4; then
  run_case_script "$SCRIPT_DIR/cases/p4_capabilities.sh" || FAIL=1
fi

e2e_finalize_results

# Enrich results.json with metadata.
python3 - "$E2E_OUT/results.json" "$KBOX" "$IMAGE" "$DIGEST" "$TIER" <<'PY'
import json, subprocess, sys
from pathlib import Path
path, kbox, image, digest, tier = sys.argv[1:6]
data = json.loads(Path(path).read_text(encoding="utf-8"))
try:
    ver = subprocess.check_output([kbox, "version"], text=True).strip()
except Exception:
    ver = "unknown"
data["kbox"] = ver
data["image"] = image
data["image_digest"] = digest
data["tier"] = tier
Path(path).write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY

e2e_log "results: $E2E_OUT/results.json"
if [[ "$FAIL" -ne 0 ]]; then
  e2e_log "one or more case scripts failed"
  exit 1
fi

# Required task coverage must never pass through missing credentials or skips.
python3 - "$E2E_OUT/results.json" <<'PY'
import json, sys
from pathlib import Path
data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
fails = [c for c in data.get("cases", []) if c.get("status") == "FAIL"]
fails += [c for c in data.get("cases", []) if c["id"].startswith("P4-") and c["status"] != "PASS"]
if fails:
    print("FAIL cases:", ", ".join(c["id"] for c in fails), file=sys.stderr)
    raise SystemExit(1)
print("summary:", json.dumps(data.get("summary", {})), file=sys.stderr)
PY

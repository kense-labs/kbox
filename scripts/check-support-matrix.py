#!/usr/bin/env python3
"""Validate docs/architecture/support-matrix.json against the codebase.

The matrix is the single source of truth for kbox product readiness. This
check keeps it honest without requiring a build:

  * the current native targets match SUPPORTED_TARGETS in runtime_assets.rs
    (required_platforms records roadmap intent and is not part of that set)
  * every declared evidence / rejection source path exists in the repo
  * readiness values come from a closed set, and no target is marked
    'supported' without a prebuilt package and native evidence
  * target ids are unique

Run: python3 scripts/check-support-matrix.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
MATRIX_PATH = REPO_ROOT / "docs" / "architecture" / "support-matrix.json"
SCHEMA = "kbox.support-matrix.v1"
READINESS = {"probe-only", "experimental", "supported"}


def main() -> int:
    errors: list[str] = []

    try:
        matrix = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"FAIL: cannot read {MATRIX_PATH}: {exc}")
        return 1

    if matrix.get("schema") != SCHEMA:
        errors.append(f"schema must be {SCHEMA!r}, got {matrix.get('schema')!r}")

    declared_levels = set(matrix.get("readiness_levels", []))
    if declared_levels != READINESS:
        errors.append(f"readiness_levels must be exactly {sorted(READINESS)}")

    targets = matrix.get("targets", [])
    ids = [t.get("id") for t in targets]
    if len(ids) != len(set(ids)):
        errors.append("duplicate target id")

    for target in targets:
        label = target.get("id", "<unnamed>")
        readiness = target.get("readiness")
        if readiness not in READINESS:
            errors.append(f"{label}: invalid readiness {readiness!r}")
        evidence = target.get("evidence", [])
        for rel in evidence:
            if not (REPO_ROOT / rel).is_file():
                errors.append(f"{label}: missing evidence file {rel}")
        if readiness == "supported" and (
            not target.get("prebuilt_package") or not evidence
        ):
            errors.append(
                f"{label}: 'supported' requires prebuilt_package and native evidence"
            )
    for ref in matrix.get("sources", []):
        if not (REPO_ROOT / ref["path"]).is_file():
            errors.append(f"sources: missing file {ref['path']}")

    for item in matrix.get("rejected", []):
        if not (REPO_ROOT / item["source"]).is_file():
            errors.append(f"rejected/{item['id']}: missing source {item['source']}")

    for item in matrix.get("required_platforms", []):
        label = item.get("id", "<unnamed>")
        for key in ("status", "target_readiness", "current_readiness", "plan", "reason"):
            if key not in item:
                errors.append(f"required/{label}: missing {key}")
        plan = item.get("plan")
        if plan and not (REPO_ROOT / plan).is_file():
            errors.append(f"required/{label}: missing plan {plan}")
        if item.get("target_readiness") not in READINESS:
            errors.append(f"required/{label}: invalid target_readiness")
        if item.get("current_readiness") == "supported" and not item.get("evidence"):
            errors.append(f"required/{label}: 'supported' requires native evidence")

    code_rel = matrix.get("code_targets_source")
    if not code_rel:
        errors.append("code_targets_source is required")
    else:
        code_path = REPO_ROOT / code_rel
        if not code_path.is_file():
            errors.append(f"code_targets_source missing: {code_rel}")
        else:
            source = code_path.read_text(encoding="utf-8")
            block = re.search(
                r"const SUPPORTED_TARGETS.*?=\s*&\[(.*?)\];",
                source,
                re.DOTALL,
            )
            if not block:
                errors.append("cannot locate SUPPORTED_TARGETS in runtime_assets.rs")
            else:
                code_targets = {
                    (os, arch, asset)
                    for os, arch, asset in re.findall(
                        r'\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)',
                        block.group(1),
                    )
                }
                matrix_targets = {
                    (t["host_os"], t["host_arch"], t["runtime_asset"])
                    for t in targets
                }
                if code_targets != matrix_targets:
                    errors.append(
                        "target drift vs runtime_assets.rs: "
                        f"only-in-code={sorted(code_targets - matrix_targets)}, "
                        f"only-in-matrix={sorted(matrix_targets - code_targets)}"
                    )

    supported = [t["id"] for t in targets if t.get("readiness") == "supported"]
    required = matrix.get("required_platforms", [])
    print(
        f"targets={len(targets)} supported={len(supported)} "
        f"rejected={len(matrix.get('rejected', []))} required={len(required)}"
    )
    if errors:
        for error in errors:
            print(f"FAIL: {error}")
        return 1
    print("check-support-matrix: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())

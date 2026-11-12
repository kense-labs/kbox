#!/usr/bin/env python3
"""Check the single-workspace layout contract for kbox.

Pure standard library: no third-party packages, no network access, read-only.

Contract (kbox/AGENTS.md "Do not create another Cargo workspace or root
lockfile" and Microsoft Pragmatic Rust Guidelines M-CRATES-FLAT-FOLDER):

  * exactly one ``[workspace]`` declaration, at the repository root;
  * exactly one first-party ``Cargo.lock`` and one ``rust-toolchain`` file;
  * every path in ``[workspace].members`` exists and is a real package;
  * every member inherits the root lints, except the explicitly listed local
    lint exceptions.

Vendored sources, third_party trees and git submodules carry their own upstream
manifests on purpose and are excluded from the layout checks.
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXCLUDED_PARTS = frozenset({"target", "vendor", "third_party", "libkrun", "libkrunfw"})

# Crates that intentionally keep a local lint contract instead of inheriting
# the root workspace lints. Every entry needs a reason; do not add entries just
# to make this check pass.
LOCAL_LINT_EXCEPTIONS = {
    "crates/box/third_party/mkext4": 'keeps its own stricter unsafe_code = "forbid"',
}


def is_excluded(path: Path) -> bool:
    """True for vendored / third-party / submodule paths."""
    return bool(EXCLUDED_PARTS.intersection(path.relative_to(ROOT).parts))


def load(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def has_workspace_table(path: Path) -> bool:
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip() == "[workspace]":
            return True
    return False


def main() -> int:
    failures: list[str] = []
    root_manifest = ROOT / "Cargo.toml"
    if not root_manifest.is_file():
        print("FAIL: root Cargo.toml is missing")
        return 1
    manifest = load(root_manifest)
    if "workspace" not in manifest:
        failures.append("root Cargo.toml has no [workspace] table")

    workspace_decls = sorted(
        p for p in ROOT.rglob("Cargo.toml")
        if not is_excluded(p) and has_workspace_table(p)
    )
    if workspace_decls != [root_manifest]:
        found = ", ".join(str(p.relative_to(ROOT)) for p in workspace_decls) or "none"
        failures.append(
            f"exactly one [workspace] is allowed (root Cargo.toml); found "
            f"{len(workspace_decls)}: {found}"
        )

    lockfiles = sorted(p for p in ROOT.rglob("Cargo.lock") if not is_excluded(p))
    if lockfiles != [ROOT / "Cargo.lock"]:
        found = ", ".join(str(p.relative_to(ROOT)) for p in lockfiles) or "none"
        failures.append(f"exactly one root Cargo.lock is allowed; found: {found}")

    toolchains = sorted(p for p in ROOT.rglob("rust-toolchain*") if not is_excluded(p))
    if toolchains != [ROOT / "rust-toolchain.toml"]:
        found = ", ".join(str(p.relative_to(ROOT)) for p in toolchains) or "none"
        failures.append(f"exactly one root rust-toolchain.toml is allowed; found: {found}")

    members: list[str] = manifest.get("workspace", {}).get("members", [])
    inherited = 0
    for member in members:
        crate_manifest = ROOT / member / "Cargo.toml"
        if not crate_manifest.is_file():
            failures.append(f"member {member} has no Cargo.toml")
            continue
        crate = load(crate_manifest)
        if "package" not in crate:
            failures.append(f"member {member} is not a package")
        lints = crate.get("lints")
        if lints and lints.get("workspace") is True:
            inherited += 1
            if member in LOCAL_LINT_EXCEPTIONS:
                # A fixed crate must be removed from the exception list; otherwise
                # the stale entry hides future regressions.
                failures.append(
                    f"stale local lint exception (already inherits workspace lints): {member}"
                )
        elif member not in LOCAL_LINT_EXCEPTIONS:
            failures.append(
                f"member {member} neither inherits workspace lints nor is an "
                "allowed local lint exception"
            )
    for member in LOCAL_LINT_EXCEPTIONS:
        if member not in members:
            failures.append(f"stale local lint exception: {member}")

    print(f"[workspace] declarations : {len(workspace_decls)} (root only)")
    print(f"first-party Cargo.lock    : {len(lockfiles)}")
    print(f"rust-toolchain files      : {len(toolchains)}")
    print(f"workspace members         : {len(members)}")
    print(f"inherit root lints        : {inherited}")
    print(f"local lint exceptions     : {len(LOCAL_LINT_EXCEPTIONS)}")
    for member, reason in LOCAL_LINT_EXCEPTIONS.items():
        print(f"  - {member}: {reason}")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print("PASS: single-workspace layout contract")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Check that kbox (and, when supplied, krun) consume one pinned KCL revision.

Pure standard library, read-only, no network access, Python 3.9+.

Background: KCL is a standalone project (``kense-labs/kcl``), not a directory
inside kbox or krun. kbox historically used a sibling ``../kcl`` working tree
while krun pinned that standalone project at a git rev. The sibling tree was a
local, non-Git stand-in exported from kbox's older ``crates/acl``; it was never
the KCL project, and the two products did not actually use the same parser.
Both products now pin the same revision of the standalone project.

Contract (kbox/AGENTS.md "General rules"):

  * the root ``kcl`` dependency is ``package = "kense-kcl"`` with a ``git`` URL
    and the exact pinned ``rev``; a ``path`` override is forbidden;
  * the root ``Cargo.lock`` records the matching git source for ``kense-kcl``;
  * no first-party manifest refers to a ``../kcl`` path dependency;
  * with ``--krun <path>``, the krun manifest and lockfile pin the same rev.

Historical release notes, test plans and evidence logs are intentionally not
scanned: they record what was true at the time and must not be rewritten.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Optional

KCL_GIT = "https://github.com/kense-labs/kcl.git"
# Keep in sync with krun/Cargo.toml. A rev change is a cross-repository change.
KCL_REV = "76bf580454f1d69d7b826223ac50161efa685f85"
EXPECTED_LOCK_SOURCE = f"git+{KCL_GIT}?rev={KCL_REV}#{KCL_REV}"

DEP_LINE = re.compile(r"^\s*kcl\s*=\s*\{(?P<body>[^}]*)\}\s*(?:#.*)?$", re.MULTILINE)
LOCK_BLOCK = re.compile(r"\[\[package\]\]\n(.*?)(?=\n\[\[package\]\]|\Z)", re.DOTALL)
NAME_LINE = re.compile(r'^name = "([^"]+)"', re.MULTILINE)
SOURCE_LINE = re.compile(r'^source = "([^"]+)"', re.MULTILINE)
PATH_TO_SIBLING_KCL = re.compile(r'path\s*=\s*"\.\./kcl[^"]*"')

EXCLUDED_PARTS = frozenset(
    {
        "target",
        "vendor",
        "third_party",
        "libkrun",
        "libkrunfw",
        "docs",
        ".git",
        # Build outputs and staged prefixes can contain copied manifests from
        # older revisions; they are not declarations of this checkout.
        ".build",
        "dist",
        "node_modules",
    }
)


def read(path: Path) -> Optional[str]:
    if not path.is_file():
        return None
    return path.read_text(encoding="utf-8")


def dependency_body(text: str, label: str, failures: list[str]) -> Optional[str]:
    matches = DEP_LINE.findall(text)
    if len(matches) != 1:
        failures.append(
            f"{label}: expected exactly one `kcl = {{ ... }}` declaration, found {len(matches)}"
        )
        return None
    return matches[0]


def check_manifest(text: str, label: str, failures: list[str]) -> None:
    body = dependency_body(text, label, failures)
    if body is None:
        return
    for key, expected in (
        ("package", "kense-kcl"),
        ("git", KCL_GIT),
        ("rev", KCL_REV),
    ):
        if not re.search(rf'\b{key}\s*=\s*"{re.escape(expected)}"', body):
            failures.append(f'{label}: kcl dependency must declare {key} = "{expected}"')
    if "path" in body:
        failures.append(f"{label}: kcl dependency must not use a local `path` override")
    # A `branch`, `tag` or bare git URL without a rev is not a pin.
    for forbidden in ("branch", "tag"):
        if re.search(rf"\b{forbidden}\s*=", body):
            failures.append(f"{label}: kcl dependency must pin `rev`, not `{forbidden}`")


def check_lock(text: Optional[str], label: str, failures: list[str]) -> None:
    if text is None:
        failures.append(f"{label}: Cargo.lock is missing")
        return
    blocks = [
        block
        for block in LOCK_BLOCK.findall(text)
        if (name := NAME_LINE.search(block)) and name.group(1) == "kense-kcl"
    ]
    if len(blocks) != 1:
        failures.append(
            f"{label}: expected exactly one `kense-kcl` package in Cargo.lock, found {len(blocks)}"
        )
        return
    source = SOURCE_LINE.search(blocks[0])
    if source is None or source.group(1) != EXPECTED_LOCK_SOURCE:
        actual = source.group(1) if source else "missing"
        failures.append(f"{label}: kense-kcl lock source is {actual}, expected pinned git source")


def check_no_sibling_path(root: Path, label: str, failures: list[str]) -> int:
    scanned = 0
    for manifest in sorted(root.rglob("Cargo.toml")):
        relative = manifest.relative_to(root)
        if EXCLUDED_PARTS.intersection(relative.parts):
            continue
        scanned += 1
        content = read(manifest)
        if content and PATH_TO_SIBLING_KCL.search(content):
            failures.append(
                f"{label}: first-party manifest {relative} still declares a `../kcl` path dependency"
            )
    return scanned


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="kbox checkout to inspect (default: repository containing this script)",
    )
    parser.add_argument(
        "--krun",
        type=Path,
        help="path to a local krun checkout; also verifies the krun side",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    failures: list[str] = []

    manifest = read(root / "Cargo.toml")
    if manifest is None:
        failures.append(f"{root}: Cargo.toml is missing")
    else:
        check_manifest(manifest, "kbox/Cargo.toml", failures)
    check_lock(read(root / "Cargo.lock"), "kbox/Cargo.lock", failures)
    scanned = check_no_sibling_path(root, "kbox", failures)

    krun_scanned = 0
    if args.krun is not None:
        krun = args.krun.resolve()
        krun_manifest = read(krun / "Cargo.toml")
        if krun_manifest is None:
            failures.append(f"{krun}: Cargo.toml is missing")
        else:
            check_manifest(krun_manifest, "krun/Cargo.toml", failures)
        check_lock(read(krun / "Cargo.lock"), "krun/Cargo.lock", failures)
        krun_scanned = check_no_sibling_path(krun, "krun", failures)

    print(f"pinned KCL repo          : {KCL_GIT}")
    print(f"pinned KCL rev           : {KCL_REV}")
    print(f"kbox manifests scanned   : {scanned}")
    if args.krun is not None:
        print(f"krun manifests scanned   : {krun_scanned}")
        print("cross-repository mode    : on")
    else:
        print("cross-repository mode    : off (pass --krun <path> to verify both sides)")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print("PASS: pinned KCL dependency contract")
    return 0


if __name__ == "__main__":
    sys.exit(main())

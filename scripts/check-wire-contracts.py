#!/usr/bin/env python3
"""Guard the hand-synchronised kbox <-> krun wire schema identifiers.

The two repositories live in separate git repos and cannot share a crate, so
versioned `.v1` wire identifiers are duplicated as string literals in both
codebases. `wire-contracts.json` is the explicit contract listing which
identifiers are shared and which are deliberately one-sided.

Default (CI) mode checks only this repository, so no cross-repo credentials are
needed. Pass `--krun <path>` to a local krun checkout to verify both sides;
use that mode when changing any wire identifier.

Pure standard library, read-only, Python 3.9+.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = Path(__file__).resolve().parent / "wire-contracts.json"
SKIP_PARTS = frozenset({"target", "vendor", "third_party", "libkrun", "libkrunfw", ".git"})

# Allow an optional escape tail, e.g. b"kbox.engine.directory.v1\0".
LITERAL = re.compile(r'"([a-zA-Z0-9._-]+\.v1)(?:\\[^"]*)?"')


def load_contract() -> dict:
    data = json.loads(CONTRACT.read_text(encoding="utf-8"))
    if data.get("version") != 1:
        raise ValueError(f"unsupported wire-contracts.json version: {data.get('version')}")
    shared = set(data["shared"])
    kbox_only = set(data["kbox_only"])
    krun_only = set(data["krun_only"])
    overlap = (shared & kbox_only) | (shared & krun_only) | (kbox_only & krun_only)
    if overlap:
        raise ValueError(f"identifier listed in more than one category: {sorted(overlap)}")
    return data


def is_wire(value: str, data: dict) -> bool:
    return value.startswith(tuple(data["wire_prefixes"])) or value in data["wire_exact"]


def scan(root: Path, data: dict) -> dict[str, set[str]]:
    """Map every wire identifier to the source files that mention it."""
    found: dict[str, set[str]] = {}
    for path in sorted(root.rglob("*.rs")):
        if SKIP_PARTS.intersection(path.relative_to(root).parts):
            continue
        rel = str(path.relative_to(root))
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.lstrip().startswith("//"):
                continue  # doc comments and commented-out code are not contracts
            for match in LITERAL.finditer(line):
                value = match.group(1)
                if is_wire(value, data):
                    found.setdefault(value, set()).add(rel)
    return found


def check_side(label: str, found: dict[str, set[str]], expected: set[str]) -> list[str]:
    failures = []
    actual = set(found)
    missing = sorted(expected - actual)
    undeclared = sorted(actual - expected)
    print(f"{label}: {len(actual)} wire identifiers found, {len(expected)} declared")
    for value in missing:
        failures.append(f"{label}: declared contract missing from source: {value}")
    for value in undeclared:
        where = ", ".join(sorted(found[value])[:3])
        failures.append(
            f"{label}: undeclared wire identifier {value} (in {where}); "
            "update scripts/wire-contracts.json"
        )
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--krun",
        type=Path,
        help="path to a local krun checkout; also verifies the krun side",
    )
    args = parser.parse_args()

    try:
        data = load_contract()
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"FAIL: cannot load wire contract: {error}")
        return 1

    shared = set(data["shared"])
    failures: list[str] = []
    # `expected` contains every shared identifier, so each side is required to
    # carry all of them. One scan per side is enough.
    failures += check_side("kbox", scan(ROOT, data), shared | set(data["kbox_only"]))

    if args.krun is not None:
        krun_root = args.krun.resolve()
        if not (krun_root / "src").is_dir():
            print(f"FAIL: --krun does not look like a krun checkout: {krun_root}")
            return 1
        failures += check_side(
            "krun", scan(krun_root, data), shared | set(data["krun_only"])
        )
    else:
        print("krun side: not checked (pass --krun <path> for cross-repo verification)")

    for failure in failures:
        print(f"FAIL: {failure}")
    if failures:
        print("FAIL: wire contract drift")
        return 1
    print("PASS: wire schema identifiers match the declared contract")
    return 0


if __name__ == "__main__":
    sys.exit(main())

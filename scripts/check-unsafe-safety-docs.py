#!/usr/bin/env python3
"""Require a `# Safety` doc section on every `unsafe fn` in first-party crates.

Why a static check instead of `-D clippy::missing_safety_doc`: that clippy lint
only inspects *exported* items. `kbox-shim` is a binary crate whose modules are
private, so no amount of clippy denial catches a missing safety section there.
Windows-only `unsafe fn`s are also invisible to a macOS/Linux host build.

This script is visibility- and cfg-independent: it reads source text, so it
covers private functions, binary crates and every target-specific branch. It
also forbids reintroducing a blanket `allow(clippy::missing_safety_doc)`, which
would hide the gap again.

Read-only. Uses the Python standard library only.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CRATES = REPO_ROOT / "crates"
SKIP_PARTS = frozenset({"target", "vendor", "third_party", "libkrun", "libkrunfw"})

UNSAFE_FN = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?unsafe\s+fn\b"
)
SAFETY_HEADING = re.compile(r"^///\s*#\s*Safety\b", re.IGNORECASE)
BLANKET_ALLOW = re.compile(r"#!?\[allow\(\s*clippy::missing_safety_doc\s*\)\]")


def first_party_sources() -> list[Path]:
    return sorted(
        p
        for p in CRATES.rglob("*.rs")
        if not SKIP_PARTS.intersection(p.relative_to(REPO_ROOT).parts)
    )


def doc_lines_above(lines: list[str], index: int) -> list[str]:
    """Collect the contiguous doc/attribute block immediately above `index`."""
    docs: list[str] = []
    i = index - 1
    while i >= 0:
        stripped = lines[i].strip()
        if stripped.startswith("///"):
            docs.append(stripped)
        elif stripped.startswith("#[") and stripped.endswith("]"):
            pass  # cfg / allow / derive attributes may sit between docs
        else:
            break
        i -= 1
    return docs


def main() -> int:
    missing: list[str] = []
    suppressions: list[str] = []
    checked = 0
    sources = first_party_sources()

    for path in sources:
        text = path.read_text(encoding="utf-8")
        rel = path.relative_to(REPO_ROOT)
        for match in BLANKET_ALLOW.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            suppressions.append(f"{rel}:{line}: {match.group(0)}")
        lines = text.splitlines()
        for index, line in enumerate(lines):
            if not UNSAFE_FN.match(line.strip()):
                continue
            checked += 1
            docs = doc_lines_above(lines, index)
            if not any(SAFETY_HEADING.match(d) for d in docs):
                name = line.strip().split("fn ", 1)[-1].split("(", 1)[0].strip()
                missing.append(f"{rel}:{index + 1}: unsafe fn {name}")

    print(f"first-party .rs files        : {len(sources)}")
    print(f"unsafe fn declarations       : {checked}")
    print(f"missing # Safety sections    : {len(missing)}")
    print(f"blanket missing_safety_doc allows: {len(suppressions)}")
    for entry in missing + suppressions:
        print(f"  FAIL {entry}")

    if missing or suppressions:
        print("FAIL: unsafe fn safety documentation contract")
        return 1
    print("PASS: every unsafe fn has a # Safety section")
    return 0


if __name__ == "__main__":
    sys.exit(main())

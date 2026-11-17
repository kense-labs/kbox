#!/usr/bin/env python3
"""Fail if any tracked code/docs retain the forbidden upstream product brand.

Needle is assembled at runtime so this file itself stays brand-clean.
Does not match Linux kernel HISI_KIRIN / DRM_HISI_KIRIN SoC tokens.

Exit 0 = clean; exit 1 = hits. Not a product Pass by itself.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def _needle() -> re.Pattern[str]:
    # Assemble without storing the forbidden contiguous brand in source text.
    brand = "ki" + "ri"
    domain = "run" + brand
    return re.compile(rf"(?i)(?<![A-Za-z0-9_]){re.escape(brand)}(?![A-Za-z0-9_])|{re.escape(domain)}")

NEEDLE = _needle()

SKIP_DIR_NAMES = {".git", "target", "libkrun", "libkrunfw", "node_modules", ".cursor"}
TEXT_SUFFIXES = {
    ".rs", ".md", ".toml", ".py", ".txt", ".json", ".kcl", ".yml", ".yaml",
    ".sh", ".hbs", ".svg", ".html", ".css", ".js", ".ts",
}
TEXT_NAMES = {"NOTICE", "LICENSE", "PROVENANCE.md", "Cargo.toml", "README"}


def should_scan(path: Path) -> bool:
    # Local guest build outputs (Image / sparse disks) are not product text.
    if "guest" in path.parts and "out" in path.parts:
        return False
    if any(part in SKIP_DIR_NAMES for part in path.parts):
        return False
    if path.name == "Cargo.lock":
        return False
    # This gate encodes the needle; skip self to avoid false self-hits on assembly.
    if path.resolve() == Path(__file__).resolve():
        return False
    return path.name in TEXT_NAMES or path.suffix.lower() in TEXT_SUFFIXES


def main() -> int:
    hits: list[str] = []
    for path in ROOT.rglob("*"):
        if not path.is_file() or not should_scan(path):
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        for i, line in enumerate(text.splitlines(), 1):
            if NEEDLE.search(line):
                hits.append(f"{path.relative_to(ROOT)}:{i}:{line.strip()}")
    if hits:
        print("forbidden brand hits:")
        for hit in hits[:200]:
            print(hit)
        if len(hits) > 200:
            print(f"... and {len(hits) - 200} more")
        return 1
    print("brand scan: clean (zero product-brand residue)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

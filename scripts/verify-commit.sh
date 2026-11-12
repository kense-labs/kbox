#!/usr/bin/env bash
# Pre-commit verification for kbox.
#
# The machine-level core.hooksPath hook delegates to lefthook. When lefthook
# is not installed, that hook prints "Can't find lefthook in PATH" and exits
# zero, so every check it owns is silently skipped. Run this script before a
# commit (or wire it into CI) so the baseline is enforced independently of
# the host hook installation.
#
# Usage: scripts/verify-commit.sh
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "== whitespace and conflict markers =="
git diff --check
git diff --cached --check

echo "== support matrix =="
python3 scripts/check-support-matrix.py

echo "== workspace compile (all targets) =="
cargo check --workspace --all-targets --locked

echo "== engine unit tests =="
cargo test --locked -p kbox-runtime --lib engine

echo "verify-commit: OK"

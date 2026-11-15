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

echo "== Rust formatting =="
cargo fmt --all -- --check

echo "== default Runtime, CLI and contract tests =="
cargo test --locked -p kbox-runtime --lib
cargo test --locked -p kbox-cli --lib
cargo test --locked -p kbox-contract

echo "== scoped Clippy =="
cargo clippy --locked -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps

echo "== archive installer regression =="
sh -n scripts/install-package.sh
bash -n scripts/package-macos-arm64.sh
python3 scripts/test-install-package.py

echo "verify-commit: OK"

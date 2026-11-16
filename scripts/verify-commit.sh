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

# Keep default tests away from an inherited user installation; retain evidence.
verification_root="$(mktemp -d "${TMPDIR:-/tmp}/kbox-verify.XXXXXX")"
chmod 700 "$verification_root"
mkdir -m 700 "$verification_root/home"
printf '%s\n' kbox-verify-private-home > "$verification_root/.kbox-verify-owner"
export K_HOME="$verification_root/home"
printf 'verify-commit: K_HOME=%s\n' "$K_HOME"

# Native Unix socket tests need a short private temporary base.
verification_temporary_base=/tmp
if [[ -d /private/tmp ]]; then
  verification_temporary_base=/private/tmp
fi
verification_temporary_root="$(mktemp -d "$verification_temporary_base/kv.XXXXXX")"
chmod 700 "$verification_temporary_root"
printf '%s\n' "$verification_root" > "$verification_temporary_root/.kbox-verify-owner"
printf '%s\n' "$verification_temporary_root" > "$verification_root/.kbox-verify-tmp"
export TMPDIR="$verification_temporary_root"
printf 'verify-commit: TMPDIR=%s\n' "$TMPDIR"

echo "== whitespace and conflict markers =="
git diff --check
git diff --cached --check

echo "== verification home isolation =="
python3 scripts/test-verify-isolation.py

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

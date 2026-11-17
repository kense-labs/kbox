#!/usr/bin/env bash
# Build a Linux x86_64 Sandbox archive for Windows WSL2 (and native Linux).
# Never reuse an installed runtime. Product default is Sandbox namespaces.
# MicroVM/libkrun companions are not part of this archive; use a native
# MicroVM-capable host package when that path is qualified.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(uname -s)/$(uname -m)" == Linux/x86_64 ]] || {
  echo 'requires Linux x86_64 (native or WSL2)' >&2
  exit 1
}
PACKAGE_VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' crates/box/cli/Cargo.toml | head -1)
VERSION=${VERSION:-$PACKAGE_VERSION}
[[ "$VERSION" == "$PACKAGE_VERSION" ]] || {
  echo 'VERSION must match the compiled CLI package version' >&2
  exit 1
}
STEM="kbox-${VERSION}-windows-wsl-x86_64"
DIST="$ROOT/dist"
STAGE="$DIST/$STEM"
mkdir -p "$DIST"
BUILD=$(mktemp -d "$DIST/.package.XXXXXX")
trap 'rm -rf "$BUILD"' EXIT

cargo build --locked --profile release-dist \
  -p kbox-cli -p kbox-sandbox-engine -p kbox-sandbox-init

mkdir -p "$BUILD/$STEM/bin"
PKG="$BUILD/$STEM"
cp target/release-dist/kbox \
  target/release-dist/kbox-sandbox-engine \
  target/release-dist/kbox-sandbox-init \
  "$PKG/bin/"
[[ -x "$PKG/bin/kbox" ]] || {
  echo 'package-linux-x86_64: missing kbox' >&2
  exit 1
}
[[ -x "$PKG/bin/kbox-sandbox-engine" ]] || {
  echo 'package-linux-x86_64: missing kbox-sandbox-engine (Sandbox is the product default)' >&2
  exit 1
}
[[ -x "$PKG/bin/kbox-sandbox-init" ]] || {
  echo 'package-linux-x86_64: missing kbox-sandbox-init (Linux Sandbox agent)' >&2
  exit 1
}

cp scripts/install-package-linux-x86_64.sh "$PKG/install.sh"
chmod +x "$PKG/install.sh" "$PKG/bin/kbox" "$PKG/bin/kbox-sandbox-engine" \
  "$PKG/bin/kbox-sandbox-init"
printf '%s\n' "$VERSION" > "$PKG/VERSION"
python3 - "$PKG" <<'PY'
import hashlib, json, subprocess, sys
from pathlib import Path
pkg = Path(sys.argv[1])

def run(*args):
    return subprocess.check_output(args, text=True).strip()

inputs = hashlib.sha256()
for name in sorted(set(run("git", "ls-files", "--cached", "--others", "--exclude-standard").splitlines())):
    p = Path(name)
    if p.is_file() and (
        name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]
        or name.startswith(".cargo/")
        or (name.startswith("crates/") and p.suffix in [".rs", ".toml", ".proto", ".plist"])
        or name.startswith("scripts/package-")
        or name
        in [
            "scripts/install-package.sh",
            "scripts/install-package-linux-x86_64.sh",
        ]
    ):
        inputs.update(name.encode() + b"\0" + p.read_bytes() + b"\0")

(pkg / "build-info.json").write_text(
    json.dumps(
        dict(
            schema="kbox.release-build.v1",
            sourceCommit=run("git", "rev-parse", "HEAD"),
            sourceInputsSha256=inputs.hexdigest(),
            cargoLockSha256=hashlib.sha256(Path("Cargo.lock").read_bytes()).hexdigest(),
            rustc=run("rustc", "--version"),
            target="windows-wsl-x86_64",
            host="linux-x86_64",
            productDefault="sandbox",
            microvmCompanions=False,
        ),
        indent=2,
    )
    + "\n"
)
files = sorted(p for p in pkg.rglob("*") if p.is_file())
(pkg / "SHA256SUMS").write_text(
    "".join(
        hashlib.sha256(p.read_bytes()).hexdigest()
        + "  "
        + p.relative_to(pkg).as_posix()
        + "\n"
        for p in files
    )
)
PY

KBOX_PREFIX="$BUILD/install-check" sh "$PKG/install.sh"
tar -C "$BUILD" -czf "$BUILD/$STEM.tar.gz" "$STEM"
(cd "$BUILD" && sha256sum "$STEM.tar.gz" > "$STEM.tar.gz.sha256")
rm -rf "$STAGE"
mv "$PKG" "$STAGE"
mv "$BUILD/$STEM.tar.gz" "$BUILD/$STEM.tar.gz.sha256" "$DIST/"
echo "archive=$DIST/$STEM.tar.gz"
du -sh "$STAGE"
ls -lh "$DIST/$STEM.tar.gz"

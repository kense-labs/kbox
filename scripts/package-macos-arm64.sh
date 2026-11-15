#!/usr/bin/env bash
# Build every companion from this checkout; never borrow an installed runtime.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(uname -s)/$(uname -m)" == Darwin/arm64 ]] || { echo 'requires macOS arm64' >&2; exit 1; }
PACKAGE_VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' crates/box/cli/Cargo.toml | head -1)
VERSION=${VERSION:-$PACKAGE_VERSION}
[[ "$VERSION" == "$PACKAGE_VERSION" ]] || { echo 'VERSION must match the compiled CLI package version' >&2; exit 1; }
STEM="kbox-${VERSION}-macos-arm64"
DIST="$ROOT/dist"
STAGE="$DIST/$STEM"
mkdir -p "$DIST"
BUILD=$(mktemp -d "$DIST/.package.XXXXXX")
trap 'rm -rf "$BUILD"' EXIT

cargo zigbuild --locked --profile release-dist --target aarch64-unknown-linux-musl -p kbox-guest-init
cargo build --locked --profile release-dist -p kbox-cli -p kbox-shim
mkdir -p "$BUILD/$STEM/bin" "$BUILD/$STEM/lib"
PKG="$BUILD/$STEM"
cp target/release-dist/kbox target/release-dist/kbox-shim "$PKG/bin/"
cp target/aarch64-unknown-linux-musl/release-dist/kbox-guest-init "$PKG/bin/"
for name in libkrun.1.17.0.dylib libkrunfw.5.dylib; do
  cp "target/release-dist/$name" "$PKG/lib/$name"
done
ln -s libkrun.1.17.0.dylib "$PKG/lib/libkrun.1.dylib"
ln -s libkrun.1.17.0.dylib "$PKG/lib/libkrun.dylib"
ln -s libkrunfw.5.dylib "$PKG/lib/libkrunfw.dylib"
cp scripts/install-package.sh "$PKG/install.sh"
printf '%s\n' "$VERSION" > "$PKG/VERSION"
codesign --force --sign - --entitlements crates/box/shim/entitlements.plist "$PKG/bin/kbox-shim"
python3 - "$PKG" <<'PY'
import hashlib,json,subprocess,sys
from pathlib import Path
pkg=Path(sys.argv[1])
def run(*args): return subprocess.check_output(args,text=True).strip()
inputs=hashlib.sha256()
for name in sorted(set(run('git','ls-files','--cached','--others','--exclude-standard').splitlines())):
 p=Path(name)
 if p.is_file() and (name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml'] or
     name.startswith('.cargo/') or name.startswith('crates/') and p.suffix in ['.rs','.toml','.proto','.plist'] or
     name.startswith('scripts/package-') or name=='scripts/install-package.sh'):
  inputs.update(name.encode()+b'\0'+p.read_bytes()+b'\0')
modules={}
for name in ['libkrun','libkrunfw']:
 if run('git','-C',name,'status','--porcelain'): raise SystemExit('refusing dirty native submodule: '+name)
 modules[name]=run('git','-C',name,'rev-parse','HEAD')
(pkg/'build-info.json').write_text(json.dumps(dict(schema='kbox.release-build.v1',sourceCommit=run('git','rev-parse','HEAD'),sourceInputsSha256=inputs.hexdigest(),cargoLockSha256=hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest(),rustc=run('rustc','--version'),target='macos-aarch64',guestTarget='aarch64-unknown-linux-musl',nativeSources=modules),indent=2)+'\n')
files=sorted(p for p in pkg.rglob('*') if p.is_file())
(pkg/'SHA256SUMS').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+p.relative_to(pkg).as_posix()+'\n' for p in files))
PY
# Exercise the archive installer against the exact staged companions before publishing local outputs.
KBOX_PREFIX="$BUILD/install-check" sh "$PKG/install.sh"
tar -C "$BUILD" -czf "$BUILD/$STEM.tar.gz" "$STEM"
(cd "$BUILD" && shasum -a 256 "$STEM.tar.gz" > "$STEM.tar.gz.sha256")
rm -rf "$STAGE"
mv "$PKG" "$STAGE"
mv "$BUILD/$STEM.tar.gz" "$BUILD/$STEM.tar.gz.sha256" "$DIST/"
echo "archive=$DIST/$STEM.tar.gz"
du -sh "$STAGE"
ls -lh "$DIST/$STEM.tar.gz"

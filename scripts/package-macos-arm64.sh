#!/usr/bin/env bash
# Build and stage the macOS arm64 kbox release archive with soname symlinks
# (not duplicated dylib bodies). Usage:
#   ./scripts/package-macos-arm64.sh
#   VERSION=3.4.1 ./scripts/package-macos-arm64.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION="${VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' crates/box/cli/Cargo.toml | head -1)}"
[[ -n "$VERSION" ]] || { echo "could not determine VERSION" >&2; exit 1; }
STEM="kbox-${VERSION}-macos-arm64"
STAGE="$ROOT/dist/${STEM}"
ARCHIVE="$ROOT/dist/${STEM}.tar.gz"

echo "[package] version=$VERSION"

cargo build --locked --profile release-dist -p kbox-cli -p kbox-shim

rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/lib"

cp -f "$ROOT/target/release-dist/kbox" "$STAGE/bin/kbox"
cp -f "$ROOT/target/release-dist/kbox-shim" "$STAGE/bin/kbox-shim"

# Guest init is a static aarch64 Linux ELF; reuse last install or a prior build.
if [[ -x "$ROOT/target/x86_64-unknown-linux-musl/release/kbox-guest-init" ]]; then
  echo "[package] unexpected: x86_64 guest-init on arm64 host; refusing" >&2
  exit 1
fi
if [[ -x "$ROOT/target/aarch64-unknown-linux-musl/release/kbox-guest-init" ]]; then
  cp -f "$ROOT/target/aarch64-unknown-linux-musl/release/kbox-guest-init" "$STAGE/bin/kbox-guest-init"
elif [[ -x "${KBOX_GUEST_INIT:-$HOME/.kbox/bin/kbox-guest-init}" ]]; then
  cp -f "${KBOX_GUEST_INIT:-$HOME/.kbox/bin/kbox-guest-init}" "$STAGE/bin/kbox-guest-init"
else
  echo "missing kbox-guest-init (set KBOX_GUEST_INIT=...)" >&2
  exit 1
fi

# Prefer versioned real dylibs from the release-dist staging dir, then fall back.
pick_dylib() {
  local name="$1"
  if [[ -f "$ROOT/target/release-dist/$name" ]]; then
    echo "$ROOT/target/release-dist/$name"
  elif [[ -f "$ROOT/target/lib/$name" ]]; then
    echo "$ROOT/target/lib/$name"
  elif [[ -f "$HOME/.kbox/lib/$name" ]]; then
    echo "$HOME/.kbox/lib/$name"
  else
    return 1
  fi
}

KRUN_REAL="$(pick_dylib libkrun.1.17.0.dylib)" || { echo "missing libkrun.1.17.0.dylib" >&2; exit 1; }
FW_REAL="$(pick_dylib libkrunfw.5.dylib)" || { echo "missing libkrunfw.5.dylib" >&2; exit 1; }

cp -f "$KRUN_REAL" "$STAGE/lib/libkrun.1.17.0.dylib"
cp -f "$FW_REAL" "$STAGE/lib/libkrunfw.5.dylib"
ln -sf libkrun.1.17.0.dylib "$STAGE/lib/libkrun.1.dylib"
ln -sf libkrun.1.17.0.dylib "$STAGE/lib/libkrun.dylib"
ln -sf libkrunfw.5.dylib "$STAGE/lib/libkrunfw.dylib"

cp -f "$ROOT/dist/install.sh" "$STAGE/install.sh"
chmod +x "$STAGE/bin/"* "$STAGE/install.sh"

# Sanity: aliases must be symlinks, not duplicate bodies.
for link in libkrun.1.dylib libkrun.dylib libkrunfw.dylib; do
  [[ -L "$STAGE/lib/$link" ]] || { echo "expected symlink: $link" >&2; exit 1; }
done

tar -C "$ROOT/dist" -czf "$ARCHIVE" "$STEM"
(
  cd "$ROOT/dist"
  shasum -a 256 "${STEM}.tar.gz" >"${STEM}.tar.gz.sha256"
)

SHA="$(awk '{print $1}' "$ROOT/dist/${STEM}.tar.gz.sha256")"
ARC_SZ="$(ls -lh "$ARCHIVE" | awk '{print $5}')"
LIB_SZ="$(du -sh "$STAGE/lib" | awk '{print $1}')"
echo "[package] archive=$ARCHIVE size=$ARC_SZ sha256=$SHA lib=$LIB_SZ"
ls -lah "$STAGE/bin" "$STAGE/lib"

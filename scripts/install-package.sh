#!/bin/sh
# Install a verified macOS arm64 archive without touching workload data.
set -eu
SRC_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX=${KBOX_PREFIX:-$HOME/.kbox}
fail() { echo "kbox-install: $*" >&2; exit 1; }
case "$PREFIX" in /*) ;; *) fail "installation prefix must be absolute" ;; esac
[ "$PREFIX" != / ] && [ ! -L "$PREFIX" ] || fail "unsafe installation prefix"
NAME=$(basename -- "$PREFIX")
case "$NAME" in ''|.|..|/) fail "unsafe installation prefix" ;; esac
[ "$(uname -s)/$(uname -m)" = Darwin/arm64 ] || fail "this archive requires macOS arm64"

# Spec050 sandbox companions are required: product default is Sandbox.
FILES='bin/kbox bin/kbox-shim bin/kbox-guest-init bin/kbox-sandbox-engine lib/libkrun.1.17.0.dylib lib/libkrun.1.dylib lib/libkrun.dylib lib/libkrunfw.5.dylib lib/libkrunfw.dylib share/kbox/sandbox-guest/Image share/kbox/sandbox-guest/initramfs.cpio share/kbox/sandbox-guest/data.img.tgz install.sh VERSION build-info.json'
[ -f "$SRC_DIR/SHA256SUMS" ] && [ ! -L "$SRC_DIR/SHA256SUMS" ] || fail "missing checksum manifest"
for name in $FILES; do
  [ -f "$SRC_DIR/$name" ] || fail "missing companion: $name"
  expected=$(awk -v name="$name" '$2 == name { print $1 }' "$SRC_DIR/SHA256SUMS")
  actual=$(shasum -a 256 "$SRC_DIR/$name" | awk '{print $1}')
  [ "$expected" = "$actual" ] || fail "checksum mismatch: $name"
done
for pair in 'libkrun.1.dylib:libkrun.1.17.0.dylib' 'libkrun.dylib:libkrun.1.17.0.dylib' 'libkrunfw.dylib:libkrunfw.5.dylib'; do
  name=${pair%%:*}; target=${pair#*:}
  [ -L "$SRC_DIR/lib/$name" ] && [ "$(readlink "$SRC_DIR/lib/$name")" = "$target" ] || fail "invalid library alias: $name"
done
for name in bin/kbox bin/kbox-shim bin/kbox-guest-init bin/kbox-sandbox-engine lib/libkrun.1.17.0.dylib lib/libkrunfw.5.dylib share/kbox/sandbox-guest/Image share/kbox/sandbox-guest/initramfs.cpio share/kbox/sandbox-guest/data.img.tgz install.sh VERSION build-info.json; do
  [ ! -L "$SRC_DIR/$name" ] || fail "unexpected symlink: $name"
done
for name in bin/kbox bin/kbox-shim bin/kbox-sandbox-engine lib/libkrun.1.17.0.dylib lib/libkrunfw.5.dylib; do
  lipo "$SRC_DIR/$name" -verify_arch arm64 || fail "wrong architecture: $name"
done
codesign --verify "$SRC_DIR/bin/kbox-sandbox-engine" || fail "invalid sandbox engine signature"
case "$(file -b "$SRC_DIR/bin/kbox-guest-init")" in
  *'ELF 64-bit LSB'*'ARM aarch64'*'static'*) ;;
  *) fail "guest-init must be a static Linux aarch64 ELF" ;;
esac
codesign --verify "$SRC_DIR/bin/kbox-shim" || fail "invalid shim signature"

PARENT=$(dirname -- "$PREFIX")
mkdir -p "$PARENT"
PARENT=$(CDPATH= cd -P -- "$PARENT" && pwd)
PREFIX="$PARENT/$NAME"
[ ! -L "$PREFIX" ] || fail "unsafe installation prefix"
# shortcut: rollback covers errors/signals, add durable recovery before crash-safe hot upgrades.
STAGE=$(mktemp -d "$PARENT/.kbox-install.XXXXXX")
ITEMS='bin lib share install.sh VERSION build-info.json SHA256SUMS'
committed=0
locked=0
cleanup() {
  result=$?
  trap - EXIT INT TERM
  rollback_failed=0
  if [ "$committed" = 0 ]; then
    for name in $ITEMS; do
      if [ -f "$STAGE/changed-$name" ]; then
        if [ -e "$STAGE/backup/$name" ] || [ -L "$STAGE/backup/$name" ]; then
          rm -rf "$PREFIX/$name" || rollback_failed=1
          mv "$STAGE/backup/$name" "$PREFIX/$name" || rollback_failed=1
        elif [ ! -e "$STAGE/payload/$name" ] && [ ! -L "$STAGE/payload/$name" ]; then
          rm -rf "$PREFIX/$name" || rollback_failed=1
        fi
      fi
    done
  fi
  if [ "$locked" = 1 ]; then rmdir "$PREFIX/.install-lock" || true; fi
  if [ "$rollback_failed" = 0 ]; then
    rm -rf "$STAGE"
  else
    echo "kbox-install: rollback needs recovery; backups retained at $STAGE/backup" >&2
    result=1
  fi
  exit "$result"
}
trap cleanup EXIT
trap 'exit 1' INT TERM
mkdir -p "$STAGE/payload/bin" "$STAGE/payload/lib" "$STAGE/payload/share/kbox/sandbox-guest" "$STAGE/backup"
mkdir -p "$PREFIX"
mkdir "$PREFIX/.install-lock" || fail "another installer owns this prefix"
locked=1
for name in bin lib; do
  [ ! -L "$PREFIX/$name" ] || fail "runtime directory must not be a symlink: $name"
  if [ -d "$PREFIX/$name" ]; then cp -pPR "$PREFIX/$name/." "$STAGE/payload/$name/"; fi
done
if [ -d "$PREFIX/share" ]; then cp -pPR "$PREFIX/share/." "$STAGE/payload/share/"; fi
for name in $FILES SHA256SUMS; do
  rm -f "$STAGE/payload/$name"
  mkdir -p "$(dirname -- "$STAGE/payload/$name")"
  cp -pP "$SRC_DIR/$name" "$STAGE/payload/$name"
done
chmod +x "$STAGE/payload/bin/kbox" "$STAGE/payload/bin/kbox-shim" "$STAGE/payload/bin/kbox-guest-init" "$STAGE/payload/bin/kbox-sandbox-engine" "$STAGE/payload/install.sh"
for name in $ITEMS; do
  # Record the rollback obligation before the first rename.
  touch "$STAGE/changed-$name"
  if [ -e "$PREFIX/$name" ] || [ -L "$PREFIX/$name" ]; then mv "$PREFIX/$name" "$STAGE/backup/$name"; fi
  mv "$STAGE/payload/$name" "$PREFIX/$name"
done
actual_version=$(K_HOME="$PREFIX" "$PREFIX/bin/kbox" version) || fail "installed executable failed verification"
[ "$actual_version" = "kbox version $(cat "$PREFIX/VERSION")" ] || fail "installed version does not match the archive"
committed=1
echo "Installed $actual_version at $PREFIX"
echo "Sandbox engine: $PREFIX/bin/kbox-sandbox-engine"
echo "Sandbox guest: $PREFIX/share/kbox/sandbox-guest"

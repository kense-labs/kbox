#!/bin/sh
# kbox one-line installer (macOS / Linux, POSIX sh).
#
#   curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.4.0/install.sh | sh
#
# Environment overrides:
#   KBOX_VERSION   release tag version, default 3.4.0
#   KBOX_PREFIX    install prefix, default ~/.kbox
set -eu

REPO="kense-labs/kense"
VERSION="${KBOX_VERSION:-3.4.0}"
TAG="kbox-v${VERSION}"
PREFIX="${KBOX_PREFIX:-$HOME/.kbox}"

log() { printf 'kbox-install: %s\n' "$*"; }
fail() { printf 'kbox-install: error: %s\n' "$*" >&2; exit 1; }

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"

OS=$(uname -s)
ARCH=$(uname -m)
case "$OS" in
  Darwin) PLATFORM="macos" ;;
  Linux) PLATFORM="linux" ;;
  *) fail "unsupported operating system: $OS (prebuilt packages cover macOS and Linux)" ;;
esac
case "$ARCH" in
  arm64|aarch64) ARCH="arm64" ;;
  x86_64|amd64) ARCH="x86_64" ;;
  *) fail "unsupported architecture: $ARCH" ;;
esac

ASSET="kbox-${VERSION}-${PLATFORM}-${ARCH}.tar.gz"
BASE="https://github.com/${REPO}/releases/download/${TAG}"
log "platform: ${PLATFORM}/${ARCH}, version: ${VERSION}"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT INT TERM

download() {
  # $1 = url, $2 = output file; returns 1 on HTTP failure.
  # Interactive terminals get a compact progress bar; pipelines stay quiet.
  if [ -t 2 ]; then
    curl -fL --retry 3 --progress-bar -o "$2" "$1"
  else
    curl -fsSL --retry 3 -o "$2" "$1"
  fi
}

if ! download "${BASE}/${ASSET}" "$WORK/$ASSET"; then
  fail "no prebuilt package for ${PLATFORM}/${ARCH} at ${TAG}.
  See available assets: https://github.com/${REPO}/releases/tag/${TAG}
  Or build from source: https://github.com/kense-labs/kbox#33-%E6%9E%84%E5%BB%BA-cli"
fi

download "${BASE}/${ASSET}.sha256" "$WORK/$ASSET.sha256" || \
  fail "no SHA-256 asset published for $ASSET; refusing unverified installation"
EXPECTED=$(awk '{print $1}' "$WORK/$ASSET.sha256")
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL=$(sha256sum "$WORK/$ASSET" | awk '{print $1}')
else
  ACTUAL=$(shasum -a 256 "$WORK/$ASSET" | awk '{print $1}')
fi
[ "$EXPECTED" = "$ACTUAL" ] || fail "SHA-256 mismatch: expected $EXPECTED, got $ACTUAL"
log "checksum verified"

tar -xzf "$WORK/$ASSET" -C "$WORK"
PKG_DIR="$WORK/kbox-${VERSION}-${PLATFORM}-${ARCH}"
[ -f "$PKG_DIR/install.sh" ] || fail "archive does not contain install.sh"

KBOX_PREFIX="$PREFIX" sh "$PKG_DIR/install.sh"
log "installed to $PREFIX"

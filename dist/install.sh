#!/bin/sh
# kbox macOS installer — installs into ~/.kbox (no sudo required).
set -eu

SRC_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX="${KBOX_PREFIX:-$HOME/.kbox}"

echo "Installing kbox to $PREFIX"
mkdir -p "$PREFIX/bin" "$PREFIX/lib"

# Replace files atomically so a running kbox can update itself on macOS/Linux.
TMP_SUFFIX=".kbox-install-$$"
cleanup() {
  rm -f "$PREFIX"/bin/*"$TMP_SUFFIX" "$PREFIX"/lib/*"$TMP_SUFFIX"
}
trap cleanup EXIT
trap 'cleanup; exit 1' INT TERM
install_file() {
  SRC=$1
  DEST_DIR=$2
  DEST="$DEST_DIR/${SRC##*/}"
  TMP="$DEST$TMP_SUFFIX"
  # Preserve soname symlinks (cp without -R/-P follows and duplicates dylibs).
  if [ -L "$SRC" ]; then
    ln -sfn "$(readlink "$SRC")" "$TMP"
  else
    cp -f "$SRC" "$TMP"
  fi
  mv -f "$TMP" "$DEST"
}
for SRC in "$SRC_DIR"/bin/*; do
  install_file "$SRC" "$PREFIX/bin"
done
for SRC in "$SRC_DIR"/lib/*; do
  install_file "$SRC" "$PREFIX/lib"
done
chmod +x "$PREFIX/bin/kbox" "$PREFIX/bin/kbox-shim" "$PREFIX/bin/kbox-guest-init"

# The shim must carry the Hypervisor entitlement to create MicroVMs.
if command -v codesign >/dev/null 2>&1; then
  ENTITLEMENTS="$PREFIX/bin/.kbox-entitlements.plist"
  cat > "$ENTITLEMENTS" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.hypervisor</key>
    <true/>
</dict>
</plist>
PLIST
  codesign --sign - --entitlements "$ENTITLEMENTS" --force "$PREFIX/bin/kbox-shim" 2>/dev/null || true
  rm -f "$ENTITLEMENTS"
fi

"$PREFIX/bin/kbox" version

case ":$PATH:" in
  *":$PREFIX/bin:"*) ;;
  *)
    echo
    echo "Add kbox to your PATH, e.g.:"
    echo "  echo 'export PATH=\"$PREFIX/bin:\$PATH\"' >> ~/.zshrc"
    ;;
esac
echo "Done. Try: kbox run -i --rm ghcr.io/kense-labs/dsh-acp:latest"

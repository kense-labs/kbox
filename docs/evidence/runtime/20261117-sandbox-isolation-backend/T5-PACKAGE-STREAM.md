# T5 — Packaged CLI includes Sandbox streaming (`exec -i`)

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Gap closed

Release archive rebuilt after PTY + non-TTY streaming landed. Isolated
prefix install of the packaged binary streams stdin via `kbox exec -i`
(no `-t`).

## Archive

```text
archive=dist/kbox-3.4.3-macos-arm64.tar.gz
sha256=440de9da7b3c21b8f59f971782d6da589f1028bbd664bed6c4794fbc5adae079
size≈33M compressed / ≈65M staged
build: bash scripts/package-macos-arm64.sh
log: /tmp/kbox-package-macos-arm64-20261117171730.log
```

## Install + smoke

```bash
PREFIX=$(mktemp -d /tmp/kbox-pkg-stream-install-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-pkg-stream-home-XXXXXX)
export K_HOME
STAGE=$(mktemp -d /tmp/kbox-pkg-stream-stage-XXXXXX)
tar -C "$STAGE" -xzf dist/kbox-3.4.3-macos-arm64.tar.gz
STEM=$(ls "$STAGE")
KBOX_PREFIX="$PREFIX" sh "$STAGE/$STEM/install.sh"
xattr -dr com.apple.quarantine "$PREFIX" || true
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  "$PREFIX/bin/kbox-sandbox-engine"
export PATH="$PREFIX/bin:$PATH"
kbox run -d --name pkgstreambox alpine:3.20 -- sleep 3600
printf 'echo PKG_STREAM_OK\nexit\n' | kbox exec -i pkgstreambox -- sh
kbox rm -f pkgstreambox
```

Observed (this run):

- Install prefix: `/private/tmp/kbox-pkg-stream-install-1AJYZq`
- `K_HOME=/tmp/kbox-pkg-stream-home-OvHBbr`
- stdout contains `PKG_STREAM_OK`
- smoke exit `0`

## Explicitly not claimed

- Notarized / Gatekeeper-stapled Gatekeeper path
- Product Pass

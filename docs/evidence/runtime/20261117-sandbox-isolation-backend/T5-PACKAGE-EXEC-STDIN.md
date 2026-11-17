# T5 — Packaged CLI includes captured one-shot exec stdin

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Gap closed

Release archive rebuilt after captured one-shot `execute` stdin landed
(`T5-SANDBOX-EXEC-STDIN.md`). Isolated prefix install of the packaged binary
accepts piped stdin on `kbox exec` **without** `-i`/`-t`.

## Archive

```text
archive=dist/kbox-3.4.3-macos-arm64.tar.gz
sha256=5c32b81347a8e3259dcd7197cf3c41acc2dc4813caf3f2a7a49d0dfe0feec7be
size≈33M compressed / ≈65M staged
build: bash scripts/package-macos-arm64.sh
log: /tmp/kbox-package-macos-arm64-20261117173216.log
```

## Install + smoke

```bash
PREFIX=$(mktemp -d /tmp/kbox-pkg-cap-stdin-install-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-pkg-cap-stdin-home-XXXXXX)
export K_HOME
STAGE=$(mktemp -d /tmp/kbox-pkg-cap-stdin-stage-XXXXXX)
tar -C "$STAGE" -xzf dist/kbox-3.4.3-macos-arm64.tar.gz
STEM=$(ls "$STAGE")
KBOX_PREFIX="$PREFIX" sh "$STAGE/$STEM/install.sh"
xattr -dr com.apple.quarantine "$PREFIX" || true
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  "$PREFIX/bin/kbox-sandbox-engine"
export PATH="$PREFIX/bin:$PATH"
kbox run -d --name pkgcapstdinbox alpine:3.20 -- sleep 3600
printf 'echo PKG_CAP_STDIN_OK\n' | "$PREFIX/bin/kbox" exec pkgcapstdinbox -- sh
kbox rm -f pkgcapstdinbox
```

Observed (this run):

- Install prefix: `/private/tmp/kbox-pkg-cap-stdin-install-fHu5nq`
- `K_HOME=/tmp/kbox-pkg-cap-stdin-home-RvqAHP`
- stdout contains `PKG_CAP_STDIN_OK`
- smoke exit `0`

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0

## Explicitly not claimed

- Notarized / Gatekeeper-stapled Gatekeeper path
- Product Pass

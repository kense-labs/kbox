# T5 — Packaged CLI/engine includes F1 Secret Env scrub

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main` (P17)  
≠Pass · ≠notarize

## Gap closed

Release archive rebuilt after fused-engine F1 ephemeral-env scrub
(`T5-ENGINE-SECRET-ENV-SCRUB.md`, debug `SCRUB_OK`). Isolated prefix
install of packaged CLI/engine: compose `secret_environment` guest Env
matches privately (sha256), and packaged-home durable `containers.json`
omits Secret plaintext.

## Archive

```text
archive=dist/kbox-3.4.3-macos-arm64.tar.gz
sha256=a6c7a125ca4ed3a308467beade2e8c316b54e7c7aa292ba8c8eecc1862d25739
size≈32M compressed / ≈65M staged
sourceCommit=f52bbb50c3a723b4… (build-info.json)
build: bash scripts/package-macos-arm64.sh
```

## Install + smoke

```bash
PREFIX=$(mktemp -d /tmp/kbox-pkg-scrub-install-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-pkg-scrub-home-XXXXXX)
export K_HOME
STAGE=$(mktemp -d /tmp/kbox-pkg-scrub-stage-XXXXXX)
WORKDIR=$(mktemp -d /tmp/kbox-pkg-scrub-work-XXXXXX)
tar -C "$STAGE" -xzf dist/kbox-3.4.3-macos-arm64.tar.gz
STEM=$(ls "$STAGE")
KBOX_PREFIX="$PREFIX" sh "$STAGE/$STEM/install.sh"
xattr -dr com.apple.quarantine "$PREFIX" || true
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  "$PREFIX/bin/kbox-sandbox-engine"
export PATH="$PREFIX/bin:$PATH"
export KBOX_SMOKE_SECRET='…'   # not recorded
# compose.yaml: alpine sleep + secret_environment.API_TOKEN → KBOX_SMOKE_SECRET
cd "$WORKDIR"
kbox compose -p pkgscrub up -d
# private sha256 compare of API_TOKEN vs source
# assert plain absent from $K_HOME/**/containers.json (and boxes.json)
kbox compose -p pkgscrub down
```

Observed (this run):

- Install prefix: `/tmp/kbox-pkg-scrub-install-w73qy8`
- `K_HOME=/tmp/kbox-pkg-scrub-home-j8APY9`
- Sandbox compose path (fused-engine Docker networks; no MicroVM NetworkStore)
- `api (image=alpine:3.20, sandbox)` converged
- Private compare + durable Env clean → `PKG_SCRUB_OK`
- compose down exit `0`

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0 (clean)

## Explicitly not claimed

- Secret values in logs/evidence
- Notarized / Gatekeeper-stapled install
- Product Pass
- Ephemeral overlay surviving fused-engine daemon restart without Box recreate

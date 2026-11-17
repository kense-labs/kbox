# T5 — Packaged CLI includes Compose Secrets-on-Sandbox

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Gap closed

Release archive rebuilt after Compose Secrets-on-Sandbox landed
(`T5-COMPOSE-SECRETS.md`). Isolated prefix install of the packaged binary
projects `secret_environment` onto Sandbox private-disk binds and expands
guest Env via `map_env`. Packaged `kbox compose` smoke proves the value
without recording secret bytes.

## Archive

```text
archive=dist/kbox-3.4.3-macos-arm64.tar.gz
sha256=e5782d3f82e98f4a78a7effcbc6dbe9a0e9df620bec59cabfa5234ae9800102b
size≈33M compressed / ≈65M staged
build: bash scripts/package-macos-arm64.sh
log: /tmp/kbox-package-macos-arm64-20261117173801.log
```

## Install + smoke

```bash
PREFIX=$(mktemp -d /tmp/kbox-pkg-compose-sec-install-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-pkg-compose-sec-home-XXXXXX)
export K_HOME
STAGE=$(mktemp -d /tmp/kbox-pkg-compose-sec-stage-XXXXXX)
WORKDIR=$(mktemp -d /tmp/kbox-pkg-compose-sec-work-XXXXXX)
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
kbox compose -p pkgsec up -d
kbox compose -p pkgsec exec api -- printenv API_TOKEN
# compare privately to KBOX_SMOKE_SECRET (do not log value)
kbox compose -p pkgsec down
```

Observed (this run):

- Install prefix: `/tmp/kbox-pkg-compose-sec-install-OZJiBd`
- `K_HOME=/tmp/kbox-pkg-compose-sec-home-hIkQzw`
- Sandbox compose path (fused-engine Docker networks; no MicroVM NetworkStore)
- `api (image=alpine:3.20, sandbox)` converged
- Private compare → `PKG_COMPOSE_SECRETS_OK`
- compose down removed sandbox docker network
- smoke exit `0`

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0

## Explicitly not claimed

- Secret values in logs/evidence
- Notarized / Gatekeeper-stapled Gatekeeper path
- Product Pass

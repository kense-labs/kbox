# T5 — Packaged CLI includes Sandbox `run --rm` exit-code fix

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main` (P17)  
≠Pass · ≠notarize

## Gap closed

Release archive rebuilt after foreground Sandbox `run --rm` TrustProvider /
`skips_host_pid_reconcile` fix (`T5-RUN-RM-EXIT.md`). Isolated prefix
install of packaged CLI: success path exits `0`; fail-closed guest
`exit 7` exits `7`.

## Archive

```text
archive=dist/kbox-3.4.3-macos-arm64.tar.gz
sha256=5c1e8eba572ca539748cf48e77933093ed9d901a2e5d28507887c491afccc9d1
size≈32M compressed
sourceCommit=f52bbb50c3a723b4… (build-info.json)
build: bash scripts/package-macos-arm64.sh
```

Supersedes prior `a6c7a125…` archive (F1 scrub present, pre-fix CLI exit).

**Superseded for publish:** GitHub `v3.4.4` ships
`dist/kbox-3.4.4-macos-arm64.tar.gz`
(`sha256=11ef7ae81016bffd7d3eba19303ab3af7adad1048881efe5b7ecd9df2b51f466`)
with the same exit-code fix + Spec050 stack — see `T5-RELEASE-3.4.4.md`.
This 3.4.3 pulse remains historical packaging evidence only.

## Install + smoke

```bash
PREFIX=$(mktemp -d /tmp/kbox-pkg-rmexit-install-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-pkg-rmexit-home-XXXXXX)
export K_HOME
STAGE=$(mktemp -d /tmp/kbox-pkg-rmexit-stage-XXXXXX)
tar -C "$STAGE" -xzf dist/kbox-3.4.3-macos-arm64.tar.gz
STEM=$(ls "$STAGE")
KBOX_PREFIX="$PREFIX" sh "$STAGE/$STEM/install.sh"
xattr -dr com.apple.quarantine "$PREFIX" || true
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  "$PREFIX/bin/kbox-sandbox-engine"
export PATH="$PREFIX/bin:$PATH"
kbox run --rm alpine:3.20 -- echo PKG_RM_EXIT_OK; echo EXIT:$?
kbox run --rm alpine:3.20 -- sh -c 'exit 7'; echo EXIT:$?
```

Observed (this run):

- Install prefix: `/tmp/kbox-pkg-rmexit-install-EykX2M`
- `K_HOME=/tmp/kbox-pkg-rmexit-home-mnStrP`
- Success: `PKG_RM_EXIT_OK` / `EXIT:0`
- Fail-closed: `EXIT:7`

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0 (clean)

## Explicitly not claimed

- Product Pass
- Notarized / Gatekeeper-stapled install
- Multi-host matrix

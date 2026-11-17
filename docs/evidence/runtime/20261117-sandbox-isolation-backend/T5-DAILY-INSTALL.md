# T5 — Developer / daily install (darwin arm64)

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass · ≠notarize

Short note for developers while Apple notarize/Gatekeeper staple is parked
(`PARK-NOTARIZE.md`). Quarantine clear is **not** a Gatekeeper staple.

## Archive (current)

```text
dist/kbox-3.4.4-macos-arm64.tar.gz
sha256=11ef7ae81016bffd7d3eba19303ab3af7adad1048881efe5b7ecd9df2b51f466
```

Source listing: `T5-RELEASE-3.4.4.md` (Sandbox default + `run --rm`
TrustProvider exit; supersedes prior 3.4.3 `5c1e8eba…` /
`T5-PACKAGE-RUN-RM-EXIT.md` archive for this note).

## Install + quarantine clear

Isolated prefix (matches package smokes):

```bash
PREFIX=$(mktemp -d /tmp/kbox-daily-install-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-daily-home-XXXXXX)
STAGE=$(mktemp -d /tmp/kbox-daily-stage-XXXXXX)
export K_HOME
tar -C "$STAGE" -xzf dist/kbox-3.4.4-macos-arm64.tar.gz
KBOX_PREFIX="$PREFIX" sh "$STAGE/kbox-3.4.4-macos-arm64/install.sh"
xattr -dr com.apple.quarantine "$PREFIX" || true
export PATH="$PREFIX/bin:$PATH"
kbox version   # 3.4.4
```

In-place extract (same clear, no prefix): see `T5-INSTALL-QUARANTINE.md`
(`xattr -cr` → `QUARANTINE_OK`).

## Verify Sandbox default + one smoke

```bash
kbox run -d --name sandcheck alpine:latest -- sleep 60
# boxes.json … execution_boundary == "sandbox"  (T5-MICROVM-REGRESSION.md /
# T5-RELEASE-3.4.4.md)
kbox run --rm alpine:3.20 -- echo QUARANTINE_OK; echo EXIT:$?
kbox rm -f sandcheck
```

Broader day-to-day path (pull/run/exec/stream/PTY/pause/volume/rm →
`PROD_DAILY_OK`): `T5-PROD-DAILY-PATH.md` (earlier archive sha; path still valid).

## Cold Gatekeeper

Not covered here. Resume when Apple credentials exist: `PARK-NOTARIZE.md`.

## Related evidence

- Versioned local release: `T5-RELEASE-3.4.4.md`
- Prior packaged rm-exit (3.4.3): `T5-PACKAGE-RUN-RM-EXIT.md`
- Package / install layout: `T5-PACKAGE-INSTALL.md` · `T5-PACKAGE-COMPOSE-SECRETS.md`
- Quarantine clear only: `T5-INSTALL-QUARANTINE.md`
- Packaged daily path smoke: `T5-PROD-DAILY-PATH.md`
- Sandbox default hermetic: `T5-DEFAULT-SANDBOX.md`
- Foreground `run --rm` exit: `T5-RUN-RM-EXIT.md`

## Explicitly not claimed

- Product Pass
- Notarized / Gatekeeper-stapled cold install
- Multi-host matrix

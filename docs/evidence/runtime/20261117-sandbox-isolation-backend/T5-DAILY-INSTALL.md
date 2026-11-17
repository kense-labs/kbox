# T5 — Developer / daily install (darwin arm64)

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass · ≠notarize

Short note for developers while Apple notarize/Gatekeeper staple is parked
(`PARK-NOTARIZE.md`). Quarantine clear is **not** a Gatekeeper staple.

## Archive (current)

```text
dist/kbox-3.4.5-macos-arm64.tar.gz
sha256=355b871da99112218ad552e67fe74cdf71db157be9acd173ba5fc5a0ac9d6232
(covers CLI 61/61 on darwin packaged + Windows WSL2 — see `T5-RELEASE-3.4.5.md`)
```

Source listing: `T5-RELEASE-3.4.5.md` (Sandbox CLI coverage fixes + dual-end
61/61; supersedes 3.4.4 `fc3df1c0…` / `T5-SANDBOX-CMD-COVERAGE.md` archive
for this note).

## Install + quarantine clear

Isolated prefix (matches package smokes):

```bash
PREFIX=$(mktemp -d /tmp/kbox-daily-install-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-daily-home-XXXXXX)
STAGE=$(mktemp -d /tmp/kbox-daily-stage-XXXXXX)
export K_HOME
tar -C "$STAGE" -xzf dist/kbox-3.4.5-macos-arm64.tar.gz
KBOX_PREFIX="$PREFIX" sh "$STAGE/kbox-3.4.5-macos-arm64/install.sh"
xattr -dr com.apple.quarantine "$PREFIX" || true
export PATH="$PREFIX/bin:$PATH"
kbox version   # 3.4.5
```

In-place extract (same clear, no prefix): see `T5-INSTALL-QUARANTINE.md`
(`xattr -cr` → `QUARANTINE_OK`).

## Verify Sandbox default + one smoke

```bash
kbox run -d --name sandcheck alpine:latest -- sleep 60
# boxes.json … execution_boundary == "sandbox"
kbox run --rm alpine:3.20 -- echo QUARANTINE_OK; echo EXIT:$?
kbox rm -f sandcheck
```

Full command matrix (optional):

```bash
ARCHIVE=dist/kbox-3.4.5-macos-arm64.tar.gz bash scripts/e2e-sandbox-full.sh
# expect SANDBOX_CMD_COVERAGE_OK 61/61
```

## Related

- Release: `T5-RELEASE-3.4.5.md`
- Prior release: `T5-RELEASE-3.4.4.md`
- Notarize park: `PARK-NOTARIZE.md`

# T5 — Developer install without notarize (quarantine clear)

Date: 2026-11-17  
≠Pass · does **not** replace notarize

Daily stitch (current archive sha + sandbox smoke + `PARK-NOTARIZE`):
`T5-DAILY-INSTALL.md`.

## Path

```bash
tar -xzf dist/kbox-3.4.3-macos-arm64.tar.gz
xattr -cr kbox-3.4.3-macos-arm64
export PATH="$PWD/kbox-3.4.3-macos-arm64/bin:$PATH"
kbox run --rm alpine:3.20 -- echo QUARANTINE_OK
```

## Result

See session log: quarantine-cleared staged bin runs default sandbox echo.

## Explicitly not claimed

- Gatekeeper/notarized install for end users
- Product Pass

## Also: archive `install.sh` → isolated prefix

```bash
KBOX_PREFIX=/tmp/kbox-prefix bash dist/kbox-3.4.3-macos-arm64/install.sh
# → Installed … Sandbox engine/guest present
kbox run --rm alpine:3.20 -- echo PREFIX_INSTALL_OK
```

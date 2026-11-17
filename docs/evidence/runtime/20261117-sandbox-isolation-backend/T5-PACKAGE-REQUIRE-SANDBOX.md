# T5 — package/install require Sandbox companions (k-review F2)

Date: 2026-11-17  
Branch: `main` (P17)  
Status: gate landed · ≠ product Pass

## Why

Product default is `ExecutionBoundary::Sandbox`. Packaging and install used to
copy / checksum Sandbox engine + guest only when present (`if` / `OPTIONAL_FILES`),
so a companion-less archive could still install while default `kbox run` fails
closed at daemon/guest lookup. k-review F2:
`.kstudio/evidence/k-review/20261117T094631Z-dec823cd/review.md`.

## Gate

### `scripts/package-macos-arm64.sh`

- Requires executable `target/release-dist/kbox-sandbox-engine` (codesign after copy).
- Requires complete `guest/out`: `Image`, `initramfs.cpio`, and `data.img.tgz`
  (or `data.img` → sparse tarball). Soft `if` optional copy removed.
- Missing any of the above → exit nonzero with a clear `package-macos-arm64:` message.

### `scripts/install-package.sh`

- For Sandbox-default product archives, treats as **required** `FILES` (not optional):
  - `bin/kbox-sandbox-engine`
  - `share/kbox/sandbox-guest/{Image,initramfs.cpio,data.img.tgz}`
- Checksum / arch (`lipo`) / `codesign --verify` still apply; engine always verified.
- `share` is always in install `ITEMS`.

## Current dist (re-packaged — gate embedded)

```text
archive=dist/kbox-3.4.3-macos-arm64.tar.gz
sha256=e975167b8ed01332db3aafb389b83ef69e47204516de99a9b8518a19e4cfbd5c
dist/.../bin/kbox-sandbox-engine          → ENGINE_OK
dist/.../share/kbox/sandbox-guest/{Image,initramfs.cpio,data.img.tgz} → GUEST_OK
dist/.../install.sh                       → matches scripts/install-package.sh (required FILES; no OPTIONAL_FILES)
```

Follow-up package pulse: `T5-PACKAGE-REQUIRE-EMBEDDED.md` (neg-test +
isolated `PKG_REQUIRE_OK` smoke + brand scan).

## Fail-path (embedded install.sh)

Mutated copy of staged tree. Prefix never written past the check.

| Case | Exit | Message |
| --- | --- | --- |
| remove `bin/kbox-sandbox-engine` | 1 | `kbox-install: missing companion: bin/kbox-sandbox-engine` |
| remove `share/kbox/sandbox-guest/Image` | 1 | `kbox-install: missing companion: share/kbox/sandbox-guest/Image` |
| package gate: missing engine (message replica) | 1 | `package-macos-arm64: missing target/release-dist/kbox-sandbox-engine (Sandbox is the product default)` |
| package gate: incomplete guest/out (message replica) | 1 | `package-macos-arm64: guest/out incomplete (need Image + initramfs.cpio under …)` |

Brand scan: `python3 scripts/check-sandbox-brand.py` → exit 0 (embed pulse).

## Explicitly not claimed

- Notarize / product Pass
- Env-scrub engine refresh after concurrent engine-tree edits
  (may need a later package pulse)

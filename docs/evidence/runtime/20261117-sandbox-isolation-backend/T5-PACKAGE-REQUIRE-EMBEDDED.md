# T5 — Package embeds required Sandbox install gate (k-review F2)

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main` (P17)  
≠Pass · ≠notarize · ≠049

## Gap closed

Prior `dist/kbox-3.4.3-macos-arm64` still carried the old optional
`OPTIONAL_FILES` `install.sh` while source `scripts/install-package.sh`
already required Sandbox engine + guest. Re-ran
`bash scripts/package-macos-arm64.sh` so the archive embeds the required
gate. Companion packaging remains hard-required in the package script.

## Archive

```text
archive=dist/kbox-3.4.3-macos-arm64.tar.gz
sha256=e975167b8ed01332db3aafb389b83ef69e47204516de99a9b8518a19e4cfbd5c
size≈32M compressed / ≈65M staged
sourceCommit=f52bbb50c3a7… (build-info.json)
build: bash scripts/package-macos-arm64.sh
```

Staged `dist/kbox-3.4.3-macos-arm64/install.sh` matches
`scripts/install-package.sh` (`diff -q` clean). No `OPTIONAL_FILES`.
Required `FILES` includes `bin/kbox-sandbox-engine` and
`share/kbox/sandbox-guest/{Image,initramfs.cpio,data.img.tgz}`.

## Neg-test (embedded install.sh)

Mutated copy of staged tree; prefix never written past the check.

| Case | Exit | Message |
| --- | --- | --- |
| remove `bin/kbox-sandbox-engine` | 1 | `kbox-install: missing companion: bin/kbox-sandbox-engine` |
| remove `share/kbox/sandbox-guest/Image` | 1 | `kbox-install: missing companion: share/kbox/sandbox-guest/Image` |

## Isolated prefix install smoke

```bash
PREFIX=$(mktemp -d /tmp/kbox-pkg-require-smoke.XXXXXX)
KBOX_PREFIX="$PREFIX" sh dist/kbox-3.4.3-macos-arm64/install.sh
export K_HOME="$PREFIX/home" PATH="$PREFIX/bin:$PATH"
kbox version
kbox run --rm alpine:3.20 -- echo PKG_REQUIRE_OK
```

Observed:

- Install → `Installed kbox version 3.4.3` + Sandbox engine/guest lines
- `kbox version` → `kbox version 3.4.3`
- Guest stdout → `PKG_REQUIRE_OK`
- CLI process exit observed `1` after successful guest echo / `--rm`
  (stdout marker is the smoke claim; ≠Pass)

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0 (clean)

## Note (Env-scrub concurrency)

Workspace showed untracked/dirty `crates/box/sandbox/engine/` while this
package completed. Prefer completed install-gate embed; if Env scrub
lands further engine changes, refresh the archive in a later package pulse.

## Explicitly not claimed

- Product Pass / notarize / Gatekeeper staple
- That CLI exit `0` is required for the smoke marker
- Closure of Env-scrub / F1 engine cleanup
- `049-pool-oneshot-discard`

# T5 — local release archive 3.4.4 (Sandbox default + `run --rm` exit)

Date: 2026-11-17  
Host: darwin arm64 (Darwin 25.6.0)  
Branch: `main`  
≠Pass · ≠notarize

## Intent

Version bump **3.4.3 → 3.4.4** and produce a versioned local release archive
that embeds the Spec050 Sandbox-default path plus the foreground
`run --rm` TrustProvider exit fix (`T5-RUN-RM-EXIT.md`). Package seat did
bump/package/smoke/evidence; parent later published matching digest as
GitHub `v3.4.4` (commit `b1abac14`).
`PARK-NOTARIZE.md` remains.

## Version bump

- `crates/box/cli/Cargo.toml` → `3.4.4`
- `crates/box/core/Cargo.toml` → `3.4.4`
- `Cargo.lock` package entries for `kbox-cli` / `kbox-core` → `3.4.4`

## Archive

```text
archive=dist/kbox-3.4.4-macos-arm64.tar.gz
sha256=11ef7ae81016bffd7d3eba19303ab3af7adad1048881efe5b7ecd9df2b51f466
size≈32M compressed
```

Built with:

```bash
bash scripts/package-macos-arm64.sh
```

Package install-check observed: `Installed kbox version 3.4.4` + Sandbox
engine/guest companions.

## Isolated install smoke

```bash
PREFIX=$(mktemp -d /tmp/kbox-rel344-prefix-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-rel344-home-XXXXXX)
STAGE=$(mktemp -d /tmp/kbox-rel344-stage-XXXXXX)
export K_HOME
tar -C "$STAGE" -xzf dist/kbox-3.4.4-macos-arm64.tar.gz
KBOX_PREFIX="$PREFIX" sh "$STAGE/kbox-3.4.4-macos-arm64/install.sh"
xattr -dr com.apple.quarantine "$PREFIX" || true
export PATH="$PREFIX/bin:$PATH"
kbox version
kbox run --rm alpine:3.20 -- echo REL_OK; echo EXIT:$?
kbox run -d --name sand344b alpine:latest -- sleep 60
# boxes.json … managed_execution.request.config.execution_boundary == "sandbox"
# managed_execution.runtime_route == "sandbox"
kbox rm -f sand344b
```

Observed (this run):

```text
PREFIX=/tmp/kbox-rel344-prefix-kgg9gp
K_HOME (version/rm)=/tmp/kbox-rel344-home-ljiv0j
K_HOME (boundary)=/tmp/kbox-rel344-home2-m1FTGP
kbox version 3.4.4
REL_OK
EXIT:0
execution_boundary": "sandbox"
runtime_route: sandbox
```

## Brand

```bash
python3 scripts/check-sandbox-brand.py
```

Observed: `brand scan: clean (zero product-brand residue)` · `BRAND_RC:0`

## Related

- Prior daily pointer: `T5-DAILY-INSTALL.md` (now 3.4.4)
- TrustProvider fix: `T5-RUN-RM-EXIT.md`
- Notarize park: `PARK-NOTARIZE.md`

## Explicitly not claimed

- Product Pass
- Notarized / Gatekeeper-stapled cold install
- Multi-host matrix

Published (same sha256): https://github.com/kense-labs/kbox/releases/tag/v3.4.4

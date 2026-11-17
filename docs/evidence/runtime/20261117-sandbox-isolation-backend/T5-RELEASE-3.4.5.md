# T5 — release 3.4.5 (CLI 61/61 dual-end + Sandbox coverage fixes)

Date: 2026-11-17T16:20Z  
Hosts: darwin arm64 · Windows 4090 WSL2 Ubuntu 24.04  
Branch: `main` (P17) · commit `de97310d` · tag `v3.4.5`  
Release: https://github.com/kense-labs/kbox/releases/tag/v3.4.5  
≠Pass · ≠notarize · ≠native win32 Sandbox

## Intent

YOLO scope: **mac + win 双端测试通过后发布新版本**. Bump **3.4.4 → 3.4.5**,
land Sandbox CLI coverage product fixes, re-verify 61/61 on both ends, publish
GitHub `v3.4.5` with darwin arm64 archive.

## Version bump

- `crates/box/cli/Cargo.toml` → `3.4.5`
- `crates/box/core/Cargo.toml` → `3.4.5`
- `Cargo.lock` `kbox-cli` / `kbox-core` → `3.4.5`

## Product delta vs 3.4.4

| Gap | Fix |
| --- | --- |
| `start` / `restart` rootfs baseline on Sandbox | Skip when `ManagedRuntimeRoute::Sandbox` |
| `shell` MicroVM PTY socket | Delegate to managed `exec -it` |
| `cp` hung without guest socket | Managed route + Docker archive transfer |
| live `export` / `commit` host-PID gate | Sandbox engine archive path |
| E2E harness | `scripts/e2e-sandbox-full.sh` 61-cmd gate; compose DNS/ping timeouts; brand skip/timeout for WSL DrvFS |

## Archive

```text
archive=dist/kbox-3.4.5-macos-arm64.tar.gz
sha256=355b871da99112218ad552e67fe74cdf71db157be9acd173ba5fc5a0ac9d6232
size≈32M compressed
```

```bash
bash scripts/package-macos-arm64.sh
```

## Dual-end verification (required for this release)

### Darwin packaged

```bash
ARCHIVE=dist/kbox-3.4.5-macos-arm64.tar.gz bash scripts/e2e-sandbox-full.sh
```

```text
host=Darwin …
install_mode=archive
covered=61 / expected=61
SANDBOX_CMD_COVERAGE_OK 61/61 100.0%
archive_sha256=355b871da99112218ad552e67fe74cdf71db157be9acd173ba5fc5a0ac9d6232
BRAND_OK
```

Log: `T5-MACOS-CMD-COVERAGE-345.log`

### Windows 4090 WSL2 worktree (Spec050 namespaces)

```bash
# root on Ubuntu 24.04; repo D:\code\kense\kbox
E2E_USE_WORKTREE=1 E2E_SKIP_BRAND=1 bash scripts/e2e-sandbox-full.sh
```

```text
host=Linux x86_64
version_line=kbox version 3.4.5
SANDBOX_CMD_COVERAGE_OK 61/61 100.0%
COMPOSE_DNS_OK / COMPOSE_PING_OK
RUNNER_EXIT=0
```

Log: `T5-WINDOWS-WSL-CMD-COVERAGE-345.log`

## Checks (release seat)

```bash
cargo fmt --all -- --check
cargo test --locked -p kbox-cli commit:: --lib
cargo test --locked -p kbox-cli cp:: --lib
cargo clippy --locked -p kbox-cli -p kbox-runtime --all-targets --no-deps
```

Observed: fmt OK; commit 26 / cp 22 passed; clippy exit 0 (warnings only).
`kbox-sandbox-engine` still has pre-existing undocumented-unsafe clippy debt
(vendor fused engine) — not newly claimed clean.

## Explicitly not claimed

- Product Pass / Apple notarization / Gatekeeper staple
- Native win32 / WHPX Sandbox package
- Multi-host matrix beyond darwin + this WSL workstation
- Completion of unrelated `049-pool-oneshot-discard` WIP

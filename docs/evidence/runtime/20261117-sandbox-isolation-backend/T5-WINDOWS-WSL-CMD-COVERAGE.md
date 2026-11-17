# T5 — Windows (4090) WSL2 Sandbox CLI command coverage 61/61

Date: 2026-11-17T16:00Z  
Host: SSH `4090` → Windows + WSL2 Ubuntu 24.04 (`DESKTOP-BM5D1UH`, kernel
`6.18.33.2-microsoft-standard-WSL2`, x86_64)  
Repo path: `D:\code\kense\kbox` → `/mnt/d/code/kense/kbox`  
Branch: `main` (P17)  
≠Pass · ≠notarize · ≠native win32 Sandbox

## Intent

YOLO named scope: **到 ssh 4090 … windows 侧全部 61 个命令全覆盖端到端通过**.

Spec050 Sandbox fused-engine is Linux-namespace-bound. Native Windows WHPX
does not host this path; acceptance for this scope is **WSL2 root** on the
4090 workstation (real Windows machine, Linux guest for Sandbox).

## Harness

```bash
# On WSL2 as root, worktree bins under /mnt/d/code/kense/kbox/target/debug
E2E_USE_WORKTREE=1 E2E_SKIP_BRAND=1 bash scripts/e2e-sandbox-full.sh
```

- `E2E_SKIP_BRAND=1`: brand gate is **not** one of the 61 CLI commands; full
  `rglob` on DrvFS `/mnt/d` timed out previously. Brand already clean on
  darwin evidence (`BRAND-ZERO.md`). Script also bounds brand to 180s when
  not skipped.
- Compose DNS/ping probes wrapped in `with_timeout 45` (prior hang on
  untimeouted `nslookup`/`ping`).
- Disk: WSL `/` filled to 100% mid-campaign from prior `/tmp/kbox-sandbox-e2e-*`
  leftovers; cleaned → ~187–252 GiB free before final run.

## Binary

```text
kbox version 3.4.4
worktree sha256(kbox)=a60781703d011649d32f61065fbe51f9121095e840a771bcf137175e7540826d
companions: kbox-sandbox-engine + kbox-sandbox-init beside kbox
```

## Observed

```text
LOG=/tmp/kbox-sandbox-e2e-20261117235152.log
host=Linux x86_64
install_mode=worktree
expected_top_level_commands=61
covered=61 / expected=61
COVERAGE_PCT=100.0%
SANDBOX_FULL_E2E_OK
SANDBOX_CMD_COVERAGE_OK 61/61 100.0%
RUNNER_EXIT=0
```

Raw log retained: `T5-WINDOWS-WSL-CMD-COVERAGE.log` (same directory).

Notable happy paths on this host: compose bridge `COMPOSE_DNS_OK` +
`COMPOSE_PING_OK`; export/commit/cp; lifecycle create/start/restart/stop/
kill/rm; TEE fail-closed.

## Platform notes (still covered)

- `port-forward`: exercised fail-closed —
  `Box e2epub does not expose a Sandbox network namespace` (CLI covered;
  HTTP not confirmed).
- Brand scan skipped via `E2E_SKIP_BRAND=1` (not a coverage miss).
- Not a packaged Windows archive install (worktree Linux bins on WSL).

## Explicitly not claimed

- Product Pass / notarize / multi-host
- Native win32 / WHPX Sandbox default (Unavailable by design for Spec050)
- Packaged `dist/kbox-*-windows-*` install matrix
- GitHub Release refresh for this worktree digest
- Brand scan re-run on DrvFS

# T5 — Sandbox CLI top-level command coverage 61/61

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main` (P17)  
≠Pass · ≠notarize

## Intent

YOLO scope **实现命令全覆盖**: exercise every top-level `kbox` command from
`--help` against the Spec050 Sandbox default path (happy path or asserted
fail-closed), including the **packaged** install path.

## Harness

```bash
bash scripts/e2e-sandbox-full.sh
# Optional worktree iteration: E2E_USE_WORKTREE=1 bash scripts/e2e-sandbox-full.sh
# Coverage gate fails unless covered == expected from `kbox --help`.
```

## Archive (this pulse)

```text
archive=dist/kbox-3.4.4-macos-arm64.tar.gz
sha256=fc3df1c0653445bc6e0ef8993843c11d6c26b0fa4da7c8b165820c80b697c848
build: bash scripts/package-macos-arm64.sh
```

## Observed — packaged install

```text
LOG=/tmp/kbox-cov-archive.log
install_mode=archive
covered=61 / expected=61
COVERAGE_PCT=100.0%
SANDBOX_FULL_E2E_OK
SANDBOX_CMD_COVERAGE_OK 61/61 100.0%
```

## Observed — worktree re-verify (same session)

```text
LOG=/tmp/kbox-cov-reverify.log
install_mode=worktree
SANDBOX_CMD_COVERAGE_OK 61/61 100.0%
```

## Product fixes embedded in this archive

| Gap | Fix |
| --- | --- |
| `start` / `restart` rootfs baseline on Sandbox | Skip baseline when `ManagedRuntimeRoute::Sandbox` |
| `shell` used MicroVM PTY socket | Delegate to `exec -it` managed PTY |
| `cp` hung on missing guest socket | Managed route + `transfer_file` via Docker archive API |
| `export` / `commit` required host PID | Sandbox live archive via engine `GET …/archive` |

## Platform notes (still covered)

- `port-forward` on darwin: fail-closed  
  `Sandbox port forwarding requires Linux network namespaces` (CLI exercised).
- TEE (`attest`/`seal`/`unseal`/`inject-secret`): fail-closed non-zero.
- `push`: fail-closed without registry.
- `build` (no RUN): `K_BOX_UNSAFE_HOST_RUN=1` (macOS default warm-pool path
  avoided for metadata-only Dockerfile).

## Explicitly not claimed

- Product Pass / notarize / multi-host
- GitHub Release / tag refresh for this archive digest (local candidate only)
- Happy-path `port-forward` on macOS HVF

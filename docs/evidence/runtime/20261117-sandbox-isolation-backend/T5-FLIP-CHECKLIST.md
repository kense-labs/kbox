# T5 checklist — flip default to Sandbox

## Done (this host)

- [x] `ExecutionBoundary` `#[default]` → Sandbox
- [x] CLI / driver inherit Sandbox; `--isolation microvm` explicit
- [x] `ensure_daemon` + live API/CLI smoke
- [x] Package/install with guest + codesigned engine
- [x] Brand scan clean
- [x] Volumes / publish / ps / pause / rm
- [x] Compose via LocalExecutionManager (no silent VmManager)
- [x] `kbox exec` docker path (`uses_backend_session`)
- [x] Compose Bridge DNS + ICMP (`T5-COMPOSE-BRIDGE.md`)
- [x] Package refresh + quarantine-clear developer install path
- [x] Sandbox PTY (`kbox exec -it` / `start_pty`; `T5-SANDBOX-PTY.md`)
- [x] Sandbox non-TTY streaming (`kbox exec -i` / `start_process`; `T5-SANDBOX-STREAM.md`)
- [x] Package refresh with PTY+stream CLI (`T5-PACKAGE-STREAM.md`)
- [x] Packaged daily path smoke (`T5-PROD-DAILY-PATH.md`) — ≠Pass
- [x] Captured one-shot `execute` stdin (`T5-SANDBOX-EXEC-STDIN.md`)
- [x] Package refresh with captured exec stdin (`T5-PACKAGE-EXEC-STDIN.md`)
- [x] Compose Secrets on sandbox (`T5-COMPOSE-SECRETS.md`)
- [x] Package refresh with Compose Secrets (`T5-PACKAGE-COMPOSE-SECRETS.md`)
- [x] Developer / daily install note (`T5-DAILY-INSTALL.md`) — ≠Pass / ≠notarize

- [x] Package/install require Sandbox engine + guest (`T5-PACKAGE-REQUIRE-SANDBOX.md`; k-review F2)
- [x] Wave L3 lineage/roles (`verify-ai-eng.py` PASS L3)
- [x] Fused-engine Secret Env scrub (`T5-ENGINE-SECRET-ENV-SCRUB.md`)
- [x] Packaged refresh with F1 scrub (`T5-PACKAGE-ENGINE-SECRET-ENV-SCRUB.md`)
- [x] Foreground `run --rm` exit-code vs guest success (`T5-RUN-RM-EXIT.md`)

## Still open (≠Pass)

- [ ] Notarize / Gatekeeper staple (Apple creds parked)
- [ ] Multi-host matrix

## Explicitly not claimed

Product Pass. Late subagent notes that still say “T5 blocked / Microvm default /
pause Unavailable” are **stale** relative to this checklist.

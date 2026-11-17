# Tasks: 050 sandbox isolation backend

## T1 — Planning artifacts
- [x] `spec.md` / `research.md` / `plan.md` / `tasks.md`

## T2 — Vendor + debrand engine crates (compile)
- [x] Crates under `crates/box/sandbox/*` as `kbox-sandbox-*`
- [x] Workspace members + `ureq=3.5.0` pin
- [x] Apache-2.0 LICENSE + NOTICE + `PROVENANCE.md` without brand wording
- [x] `cargo check --locked` for six packages
- [x] Zero brand residue: `python3 scripts/check-sandbox-brand.py`

## T3 — Boundary / route types (hermetic)
- [x] `ExecutionBoundary::{Sandbox, Microvm}` (Sandbox is product default after T5)
- [x] `ManagedRuntimeRoute::Sandbox` + hermetic tests
- [x] Mapping copies driver boundary onto create; Sandbox pins sandbox route

## T4 — ExecutionManager adapter
- [x] Router `with_sandbox` + fail-closed / dispatch tests
- [x] `SandboxLocalExecutionBackend` + engine lib surface
- [x] Production managers attach sandbox via `with_sandbox`
- [x] Daemon-backed start/inspect/kill/exec + network create
- [x] Native smoke (`T5-NATIVE-SMOKE.md` / package evidence)

## T5 — Default Sandbox + evidence
- [x] Flip CLI/product default to Sandbox + `ensure_daemon`
- [x] Brand scan green
- [x] Package/install + compose/exec/bridge-DNS smokes on darwin-arm64
- [ ] OUTCOME.md product acceptance (notarize / multi-host still open)
- Parked: compose Secrets (`PARK-COMPOSE-SECRETS.md`); notarize (`PARK-NOTARIZE`)

# T5 — Default Sandbox + daemon ensure

Date: 2026-11-17  
Branch: `main` (P17)  
Status: **hermetic partial** · ≠Pass · live daemon smoke **not** achieved

## What landed

1. **`ExecutionBoundary::default()` → `Sandbox`** in `crates/box/core/src/config.rs`.
   Explicit `Microvm` / confidential paths remain available; no silent downgrade.
2. **`ensure_daemon(home)`** in `crates/box/sandbox/engine/src/ensure_daemon.rs`:
   - Short-circuits when `{home}/.kbox/run/docker.sock` already exists
   - Else spawns `kbox-sandbox-engine daemon` with `HOME` + `KBOX_SANDBOX_SOCKET`
     rooted at that Box home (binary via `KBOX_SANDBOX_ENGINE_BIN`,
     `CARGO_BIN_EXE_kbox-sandbox-engine`, sibling of current exe / deps parent, PATH)
   - Waits briefly; fail-closed string error if missing
   - Gated by `KBOX_SANDBOX_AUTOSTART` (default on; `0|false|no|off` disables)
3. **`SandboxLocalExecutionBackend`**: start path calls `client(true)` →
   `ensure_daemon`; inspect/kill use `client(false)` (no spawn).
   `preflight_launch` stays daemon-optional (discovery only).
4. **Driver**: `BoxRuntimeDriver::new` uses `ExecutionBoundary::default()`
   (Sandbox). `new_confidential` pins `Microvm` explicitly.
5. CLI request test renamed/asserted Sandbox default
   (`test_build_box_config_defaults_to_sandbox_isolation`).

## Commands (isolated `K_HOME`)

```bash
export K_HOME="$(mktemp -d /tmp/kbox-t5-XXXXXX)"
cargo fmt --all -- --check
cargo test --locked -p kbox-sandbox-engine --lib ensure_daemon
cargo test --locked -p kbox-sandbox-engine --lib docker_http
cargo test --locked -p kbox-core --lib execution_boundary_defaults_to_sandbox
cargo test --locked -p kbox-runtime --lib sandbox_backend
cargo test --locked -p kbox-runtime --lib driver_defaults_to_sandbox
cargo test --locked -p kbox-runtime --lib confidential_capabilities_require_explicit
cargo clippy --locked -p kbox-sandbox-engine --lib --no-deps --no-default-features
cargo clippy --locked -p kbox-core --lib --no-deps
cargo clippy --locked -p kbox-runtime --lib --no-deps
python3 scripts/check-sandbox-brand.py
```

## Results

| Check | Result |
| --- | --- |
| `ensure_daemon` (2) | ok |
| `docker_http` (4) | ok |
| `execution_boundary_defaults_to_sandbox` | ok |
| `sandbox_backend` (6, incl. fake Unix HTTP + fail-closed) | ok |
| `driver_defaults_to_sandbox` | ok |
| `confidential_capabilities_require_explicit` | ok |
| clippy lib (engine/core/runtime) | ok (pre-existing runtime warns) |
| `check-sandbox-brand.py` | clean |
| `kbox-cli` lib test `defaults_to_sandbox_isolation` | **blocked** — package does not compile tests due to unrelated 049 WIP (`destroy_vm_or_reap` missing in `pool/attach.rs`); source assertion present |

## Explicitly not claimed

- Native / live fused-daemon OCI smoke (spawn of real `kbox-sandbox-engine daemon` + guest)
- Product Pass / daily-usable default sandbox on a primary platform
- `kbox-cli` package test execution (049-pool-oneshot-discard dirty compile break)
- Engine **binary** clippy/check (`--all-targets` / `cli` feature) — pre-existing UI/bin errors outside T5
- Pause/resume on sandbox backend
- Windows/WSL sandbox path

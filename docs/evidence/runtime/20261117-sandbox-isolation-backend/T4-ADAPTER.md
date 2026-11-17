# T4 — SandboxLocalExecutionBackend + production wiring (T4→T5 prep)

Date: 2026-11-17  
Wave: sandbox-isolation · Spec050  
Branch: `main` (P17)

## Landed

- Minimal `kbox-sandbox-engine` library (`src/lib.rs`): state/socket layout +
  fail-closed daemon socket probe + Docker Engine HTTP client.
- `SandboxLocalExecutionBackend` in
  `crates/box/runtime/src/local_execution/sandbox_backend.rs`:
  - `route_for_create` → `ManagedRuntimeRoute::Sandbox`
  - start/inspect/kill against daemon socket (hermetic fake-daemon covered)
  - Missing daemon / unimplemented ops (`pause`/`resume`) return typed
    `Unavailable` (fail closed); never falls back to BoxVm / OciSdk
- Production attachment via `LocalExecutionBackendRouter::with_sandbox`:
  - `LocalExecutionManager::with_oci_migration_backend*`
  - `with_vm_backend`
  - `with_configured_backend` legacy path (`oci_migration::legacy_backend`)
  - `BoxRuntimeDriver` `production_manager`
- `BoxConfig.execution_boundary` (default Microvm until T5)
- Router `route_for_create`: `ExecutionBoundary::Sandbox` pins
  `ManagedRuntimeRoute::Sandbox` even when OCI migration policy would select
  OciSdk (no silent MicroVM/OCI downgrade)
- Mapping copies driver `ExecutionBoundary` onto create requests for
  `IsolationLevel::Sandbox`
- Historical `boxes.json` `isolation` field / hardware-vm `"sandbox"` string
  remain refused; Spec050 Sandbox is ExecutionBoundary / ManagedRuntimeRoute

## Commands

```bash
export K_HOME=$(mktemp -d /tmp/kbox-t4-wire-XXXXXX)
cargo test --locked -p kbox-core --lib execution_boundary_parses_sandbox
cargo test --locked -p kbox-sandbox-engine --lib
cargo test --locked -p kbox-runtime --lib sandbox_backend::
cargo test --locked -p kbox-runtime --lib sandbox_route_
cargo test --locked -p kbox-runtime --lib microvm_execution_boundary
cargo test --locked -p kbox-runtime --lib mapping_pins_execution_boundary
cargo test --locked -p kbox-runtime --lib historical_boxes_json_sandbox
cargo test --locked -p kbox-runtime --lib hardware_vm_sandbox_string
cargo fmt --all -- --check
python3 scripts/check-sandbox-brand.py
```

## Results

| Check | Result |
| --- | --- |
| `execution_boundary_parses_sandbox_and_keeps_microvm_default` | ok |
| `kbox-sandbox-engine --lib` | 6 passed |
| `sandbox_backend::` | 5 passed (pin, refuse, fail-closed, hermetic preflight, fake-daemon start/inspect) |
| `sandbox_route_` | 4 passed (refuse unwired; dispatch; stamp on create; pin without MicroVM downgrade) |
| `microvm_execution_boundary_still_follows_migration_policy` | ok |
| `mapping_pins_execution_boundary_for_sandbox_isolation` | ok |
| `historical_boxes_json_sandbox_isolation_field_is_rejected_explicitly` | ok |
| `hardware_vm_sandbox_string_stays_rejected_while_vm_only_features_stay_permitted` | ok |
| `cargo fmt --all -- --check` | clean |
| `python3 scripts/check-sandbox-brand.py` | clean |

`K_HOME` for the verification run: isolated under `/tmp/kbox-t4-wire-*`.

## Blockers / honest gaps

- No native smoke against a live fused sandbox daemon (product install path).
- CLI/product default remains MicroVM (T5 blocked until real start path works).
- `pause` / `resume` still typed `Unavailable`.

## Explicitly not claimed

- Sandbox can run OCI workloads end-to-end on a real daemon
- Default isolation is Sandbox
- Product Pass / GA
- Windows/WSL sandbox daemon support

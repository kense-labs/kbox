# T3 — ExecutionBoundary / ManagedRuntimeRoute Sandbox

Date: 2026-11-17  
Wave: sandbox-isolation · Spec050

## Commands

```bash
export K_HOME=$(mktemp -d /tmp/kbox-yolo-t3-XXXXXX)
cargo test --locked -p kbox-core execution_boundary_parses_sandbox --lib
cargo test --locked -p kbox-runtime managed_runtime_route_sandbox_round_trips --lib
cargo test --locked -p kbox-runtime sandbox_route_refuses_until_backend_is_wired --lib
cargo fmt --all -- --check
```

## Results

- `execution_boundary_parses_sandbox_and_keeps_microvm_default` — ok
- `managed_runtime_route_sandbox_round_trips_without_oci_claim` — ok
- `sandbox_route_refuses_until_backend_is_wired` — ok
- `cargo fmt --all -- --check` — apply then pass
- Default remains Microvm (fail-closed until sandbox engine wired)

## Changes

- `ExecutionBoundary::{Sandbox, Microvm}` — Microvm remains `#[default]` until T5
- `ManagedRuntimeRoute::Sandbox` — router fail-closed until T4 adapter
- Hermetic tests for parse/serde and refuse-unwired

## Explicitly not claimed

- Default product isolation flip
- Sandbox engine can run OCI
- Brand scrub complete
- Product Pass

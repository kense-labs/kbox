# T4 (partial) — Router accepts sandbox backend

Date: 2026-11-17  
Wave: sandbox-isolation · Spec050

## Landed

- `LocalExecutionBackendRouter::with_sandbox`
- Fail-closed when Sandbox route has no backend (no MicroVM downgrade)
- Dispatch to attached sandbox backend when present
- Hermetic tests in `router_tests.rs`

## Not yet (T4 remainder)

- Concrete `SandboxLocalExecutionBackend` calling fused engine
- Engine exposed as `lib` (currently bin `kbox-sandbox-engine`)
- Product wiring / default flip (T5)

## Commands

```bash
export K_HOME=$(mktemp -d /tmp/kbox-yolo-t4-XXXXXX)
cargo test --locked -p kbox-runtime sandbox_route_ --lib
cargo check --locked -p kbox-sandbox-init -p kbox-sandbox-engine
```

## Results

- `sandbox_route_refuses_until_backend_is_wired` — ok
- `sandbox_route_dispatches_only_to_attached_sandbox_backend` — ok
- sandbox init/engine check — Finished (warnings only)
- Fixed init `#[path]` to `engine/src/usb/sysfs.rs` after rebrand

## Explicitly not claimed

- Sandbox can run OCI workloads
- Default isolation is Sandbox
- Product Pass

# T4 — Production composition attaches sandbox backend

Date: 2026-11-17  
Follow-up to adapter landing.

## Landed

- `LocalExecutionManager::with_oci_migration_backend*` attaches
  `SandboxLocalExecutionBackend` via `with_sandbox`
- `legacy_backend` (configured MicroVM create-default path) uses a
  `LegacyOnly` router with sandbox attached so durable Sandbox routes
  dispatch without inventing a second ExecutionManager
- `K_BOX_OCI_MIGRATION=sandbox` remains rejected (retired SharedHostKernel
  migration); distinct from Spec050 `ManagedRuntimeRoute::Sandbox`

## Commands

```bash
export K_HOME=$(mktemp -d /tmp/kbox-yolo-wire-XXXXXX)
cargo test --locked -p kbox-runtime sandbox_ --lib
python3 scripts/check-sandbox-brand.py
```

## Explicitly not claimed

- Daemon OCI start works
- CLI default flipped to Sandbox
- Product Pass

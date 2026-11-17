# T5 — Compose on default Sandbox (Spec050)

Date: 2026-11-17  
Host: macOS arm64 · branch `main`  
≠Pass (overall goal still unmet)

## Claim

`kbox compose up` with default `ExecutionBoundary::Sandbox` boots via
`LocalExecutionManager` / `ManagedRuntimeRoute::Sandbox` — never silent
`VmManager` MicroVM. Managed `compose down` uses kill/remove (no host PID).

Project Bridge uses fused-engine Docker networks (see `T5-COMPOSE-BRIDGE.md`).
Earlier “Bridge Unavailable / TSI-only” notes from intermediate agents are
**superseded**.

## Commands

```bash
export K_HOME=$(mktemp -d /tmp/kbox-compose-sandbox-XXXXXX)/home
# compose.yaml: alpine:3.20 sleep 3600
cargo build --locked -p kbox-cli
kbox compose up -d
kbox ps -a
kbox compose down
```

## Results

| Step | Result |
| --- | --- |
| compose up | sandbox route · exit 0 |
| boxes.json | `runtime_route: sandbox` · `pid: null` |
| compose down | exit 0 |
| hermetic | `sandbox_compose_network_admits_bridge` · TSI/Host/None keep |

## Deadlock fix

Compose teardown drops the CLI lifecycle lock before
`LocalExecutionManager::{kill,remove}` (parity with `kbox rm`).

## Explicitly not claimed

- Product Pass
- ICMP between services (DNS claimed in `T5-COMPOSE-BRIDGE.md`)

Compose Secret projection: see `T5-COMPOSE-SECRETS.md` (unparked).

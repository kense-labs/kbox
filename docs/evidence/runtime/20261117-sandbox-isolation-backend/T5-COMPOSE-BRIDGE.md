# T5 — Compose Bridge / project Docker networks on Sandbox

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Goal

Default multi-service Compose on Spec050 Sandbox must use fused-engine Docker
project networks (`NetworkMode::Bridge`), not MicroVM `NetworkStore`, and must
no longer fail-close Bridge as Unavailable. Prefer service DNS; otherwise both
services on the same docker network with honest DNS status.

## Commands

```bash
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
export KBOX_SANDBOX_ENGINE_BIN=$PWD/target/debug/kbox-sandbox-engine
export PATH=$PWD/target/debug:$PATH
export K_HOME=$(mktemp -d /tmp/kbox-compose-bridge-XXXXXX)
# compose.yaml: web + client alpine sleep 3600 (default project network)
cd /tmp/kbox-compose-bridge-proj
kbox compose -f compose.yaml -p t5bridge up -d
kbox exec t5bridge-client -- nslookup web
kbox exec t5bridge-client -- ping -c 1 -W 2 web
kbox compose -p t5bridge down
```

## Observed (this host)

| Step | Result |
| --- | --- |
| `compose up -d` | exit 0; `[+] Network t5bridge_default (sandbox docker)`; both services sandbox |
| `boxes.json` | `execution_boundary=sandbox`, `network_mode=bridge:t5bridge_default`, `runtime_route=sandbox`, status running |
| `nslookup web` | `Name: web` → `Address: 172.18.0.2` (resolver `127.0.0.11`) |
| `ping web` | 1/1 replies, 0% loss |
| `wget http://web:80` | connection refused (no HTTP listener — expected) |
| `compose down` | exit 0; network removed via sandbox docker path |

K_HOME for this run: `/tmp/kbox-compose-bridge-ptSl2Q`

## Hermetic

```bash
export K_HOME=$(mktemp -d)/khome
cargo test --locked -p kbox-sandbox-engine docker_http --lib
cargo test --locked -p kbox-cli --lib commands::compose::sandbox_boot
cargo test --locked -p kbox-runtime --lib local_execution::sandbox_backend
```

- `fake_unix_http_network_ensure_connect_remove` — ok  
- `container_create_with_network_sets_host_config_and_aliases` — ok  
- `sandbox_compose_network_admits_bridge` — ok  

## Brand

```bash
python3 scripts/check-sandbox-brand.py   # exit 0
```

## Explicitly not claimed

- Product Pass / notarized install  
- HTTP service discovery beyond DNS/ICMP (no app listener in this smoke)  
- Windows/Linux host native Pass  

Compose Secret projection: see `T5-COMPOSE-SECRETS.md` (unparked).

# T5 — Compose Secret projection on default Sandbox

Date: 2026-11-17  
Host: darwin arm64 · branch `main`  
≠Pass (overall goal still unmet)

## Claim

Default Sandbox `compose up` with `secret_environment` projects Secret files
onto a private provider-owned disk directory under `K_HOME`
(`runtime-secrets/compose-sandbox`), bind-mounts them read-only into the fused
engine, and expands `K_BOX_SECRET_ENV_V1` into process Env at container create.
Never silent-boots MicroVM via `VmManager`. Durable `boxes.json` keeps only the
non-sensitive manifest + `com.kbox.compose.secret-identity` label.

MicroVM memory-backed staging (`configure_vm` / tmpfs·`ram://`) remains
available for MicroVM-boundary compose; Sandbox leases refuse `configure_vm`.

## Native smoke (this host)

```bash
export K_HOME=$(mktemp -d /tmp/kbox-compose-secrets-smoke-XXXXXX)/home
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
export KBOX_SANDBOX_ENGINE_BIN=$PWD/target/debug/kbox-sandbox-engine
export PATH=$PWD/target/debug:$PATH
export KBOX_SMOKE_SECRET='compose-sandbox-secret-value'
# compose.yaml: alpine:3.20 sleep 3600 + secret_environment.API_TOKEN=KBOX_SMOKE_SECRET
kbox compose -f compose.yaml -p t5secrets up -d
kbox exec t5secrets-web -- /bin/sh -c 'printf %s "$API_TOKEN" | sha256sum'
kbox exec t5secrets-web -- /bin/sh -c 'test -z "${K_BOX_SECRET_ENV_V1+x}" && echo MANIFEST_ABSENT'
kbox compose -p t5secrets down
```

### Observed

| Step | Result |
| --- | --- |
| `compose up -d` | exit 0 · `[+] web (…, sandbox)` · no MicroVM |
| `API_TOKEN` sha256 | `8693ea7b120c3806fa2d5a817bad6421787e7bfb180493a9312176c93d6f955b` (matches source) |
| workload manifest | `MANIFEST_ABSENT` (create expanded; exec strips durable control key) |
| pid1 env keys | `API_TOKEN HOME HOSTNAME PATH` |
| `boxes.json` | manifest present · no plaintext · secret-identity label set |
| `compose down` | exit 0 · Secret identity cleaned under `runtime-secrets/compose-sandbox` |

K_HOME for this run: `/tmp/kbox-compose-secrets-smoke-vvkS39/home`

## Hermetic

```bash
export K_HOME=$(mktemp -d)/home
cargo test --locked -p kbox-runtime --lib -- secret_manifest private_disk_store
cargo test --locked -p kbox-runtime --lib map_env_refuses_missing_secret_bind
cargo test --locked -p kbox-cli --lib sandbox_project_service
```

- `private_disk_store_projects_sandbox_binds_without_memory_mount` — ok  
- `map_env_expands_secret_manifest_from_readonly_binds` — ok  
- `map_env_refuses_missing_secret_bind` — ok  
- `sandbox_exec_strips_secret_manifest_control_key` — ok  
- `sandbox_project_service_binds_private_disk_without_microvm` — ok  

Counts: **5 passed** (focused filters).

## Brand

```bash
python3 scripts/check-sandbox-brand.py   # clean
```

## Residual (narrow)

**Closed** on this host — see `T5-ENGINE-SECRET-ENV-SCRUB.md`. Secret values
attach via engine ephemeral-env (not durable create Env). `containers.json`
and inspect Config.Env omit Secret plaintext; Box `boxes.json` unchanged.

## Explicitly not claimed

- Product Pass / notarized install  
- MicroVM compose Secret native re-smoke on this darwin host (path retained;
  still requires memory-backed root)  
- Windows host  
- Ephemeral overlay surviving fused-engine daemon process restart without
  Box recreate  

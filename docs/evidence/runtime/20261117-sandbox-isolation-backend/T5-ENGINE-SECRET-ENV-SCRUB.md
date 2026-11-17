# T5 — Engine durable Env omits Secret values

Date: 2026-11-17  
Host: darwin arm64 · branch `main`  
≠Pass (overall goal still unmet)

## Claim

Sandbox compose Secret expansion attaches Secret values through the fused
engine **ephemeral-env** overlay (`POST /kbox/containers/{id}/ephemeral-env`).
Process start/exec merge the overlay; durable `containers.json` / inspect
`Config.Env` keep only non-secret create Env. Box `boxes.json` still holds
only the non-sensitive manifest. No MicroVM fallback.

## Approach

Prefer avoid (not post-start scrub of Spec.env): create with durable Env
only; attach Secret pairs as in-memory ephemeral Env before start so
`containers.json` never stores plaintext Secret values. Exec inherits the
same overlay via `Container::process_env`.

## Native smoke (this host)

```bash
export K_HOME=$(mktemp -d /tmp/kbox-engine-secret-env-XXXXXX)/home
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
export KBOX_SANDBOX_ENGINE_BIN=$PWD/target/debug/kbox-sandbox-engine
export PATH=$PWD/target/debug:$PATH
export KBOX_SMOKE_SECRET='compose-sandbox-secret-value'
# compose.yaml: alpine:3.20 sleep 3600 + secret_environment.API_TOKEN=KBOX_SMOKE_SECRET
kbox compose -f compose.yaml -p t5scrub up -d
kbox exec t5scrub-web -- /bin/sh -c 'printf %s "$API_TOKEN" | sha256sum'
# prove durable store lacks plaintext / API_TOKEN= (hash or key-absent)
kbox compose -p t5scrub down
```

### Observed

| Step | Result |
| --- | --- |
| `compose up -d` | exit 0 · `[+] web (…, sandbox)` · no MicroVM |
| `API_TOKEN` sha256 | `8693ea7b120c3806fa2d5a817bad6421787e7bfb180493a9312176c93d6f955b` (matches source) |
| workload manifest | `MANIFEST_ABSENT` |
| `containers.json` Env keys | `PATH` only · no `API_TOKEN=` · no plaintext Secret |
| plaintext grep | `PLAINTEXT_HITS=0` |
| `compose down` | exit 0 |

K_HOME for this run: `/tmp/kbox-engine-secret-env-lrzcuD/home`  
Verdict: **ENGINE_DURABLE_CLEAN**

## Hermetic

```bash
export K_HOME=$(mktemp -d)/home
cargo test --locked -p kbox-runtime --lib -- \
  map_env_expands_secret_manifest_from_readonly_binds \
  map_env_refuses_missing_secret_bind \
  start_attaches_secret_env_via_ephemeral_not_create
cargo test --locked -p kbox-sandbox-engine --lib -- \
  container_set_ephemeral_env_posts_kbox_path
cargo test --locked -p kbox-runtime --lib -- \
  secret_manifest private_disk_store sandbox_exec_strips_secret
cargo test --locked -p kbox-cli --lib sandbox_project_service
```

- `map_env_expands_secret_manifest_from_readonly_binds` — ok (Secret in ephemeral only)  
- `map_env_refuses_missing_secret_bind` — ok  
- `start_attaches_secret_env_via_ephemeral_not_create` — ok  
- `container_set_ephemeral_env_posts_kbox_path` — ok  
- `private_disk_store_projects_sandbox_binds_without_memory_mount` — ok  
- `sandbox_exec_strips_secret_manifest_control_key` — ok  
- `sandbox_project_service_binds_private_disk_without_microvm` — ok  

Counts: **7 passed** (focused filters).

## Brand

```bash
python3 scripts/check-sandbox-brand.py   # clean
```

## Explicitly not claimed

- Product Pass / notarized install  
- Engine ephemeral Env surviving daemon process restart (Saved reload clears
  in-memory overlay; Box recreate path re-attaches from binds)  
- Windows host  
- Secret bytes absent from live process memory / guest RAM  

Packaged refresh: see `T5-PACKAGE-ENGINE-SECRET-ENV-SCRUB.md` (`PKG_SCRUB_OK`).

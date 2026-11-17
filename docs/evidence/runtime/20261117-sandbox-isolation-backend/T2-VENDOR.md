# T2 — Vendor + debrand sandbox engine crates

Date: 2026-11-17  
Wave: sandbox-isolation · Spec050  
Upstream: designated Apache-2.0 engine @ commit `2d6c2d6`

## Landed

Workspace members under `crates/box/sandbox/`:

| Package | Path |
| --- | --- |
| kbox-sandbox-proto | `proto/` |
| kbox-sandbox-share-proto | `share-proto/` |
| kbox-sandbox-image | `image/` |
| kbox-sandbox-vmm | `vmm/` |
| kbox-sandbox-init | `init/` |
| kbox-sandbox-engine | `engine/` (bin) |

`ureq` pinned to `=3.5.0`. Brand gate: `python3 scripts/check-sandbox-brand.py`.

## Commands

```bash
export K_HOME=$(mktemp -d /tmp/kbox-yolo-t2-XXXXXX)
cargo check --locked -p kbox-sandbox-proto -p kbox-sandbox-share-proto \
  -p kbox-sandbox-image -p kbox-sandbox-vmm -p kbox-sandbox-init -p kbox-sandbox-engine
python3 scripts/check-sandbox-brand.py
```

## Results

- Package checks Finished (engine unused-code warnings).
- Brand scan: clean (zero product-brand residue).

## Explicitly not claimed

- Default isolation flipped to Sandbox
- ExecutionManager adapter can run OCI
- Native smoke / product Pass

# dsh-acp × kbox E2E — DeepSeek live

Live run using `deepseek-flash` + key from repo-root `config.acl` (`providers "deepseek"`).
**Secrets are not recorded** in this file or in `results.json`.

## Outcome

- Overall: **PASS** (15 pass / 0 fail / 1 skip / 0 blocked)
- Tier: `all`
- Wall clock: ~8 min
- Command:

```sh
# Key loaded from ../config.acl (not echoed). Model: deepseek-flash.
TIER=all DSH_MODEL=deepseek-flash DSH_PROVIDER=deepseek-official \
  DSH_REASONING_EFFORT=low E2E_OUT=docs/evidence/box/20261107-dsh-acp-e2e-deepseek \
  ./scripts/e2e-dsh-acp/run.sh
```

## Environment

| Item | Value |
| --- | --- |
| Host OS / arch | macOS 26.6.2 / arm64 |
| kbox version | kbox version 3.3.0 |
| kbox binary path | `/Users/roylin/.kbox/bin/kbox` |
| Image | `ghcr.io/kense-labs/dsh-acp:latest` |
| Image digest | tag-only (local inspect unavailable) |
| Model / effort | `deepseek-official` / `deepseek-flash` / `low` |
| Isolated `K_HOME` | yes (`E2E_OUT/k-home`) |
| Credentials source | `config.acl` → `DEEPSEEK_API_KEY` (length only in operator log) |

## Case results

| ID | Status | Notes |
| --- | --- | --- |
| P0-1 | PASS | upstream process-prewarm bench `RUNS=1` |
| P0-2 | PASS | cold `kbox run -i --rm` initialize; agent=`deepseek-harness-acp` |
| P1-1 | PASS | initialize (+repeat) |
| P1-2 | PASS | `session/new` |
| P1-3 | PASS | model config echoed |
| P1-4 | PASS | greeting prompt `end_turn` |
| P1-5 | PASS | three sequential attaches + replenish |
| P1-6 | PASS | prewarm `idle>=1` |
| P2-1 | PASS | workspace marker echoed via prompt |
| P2-2 | PASS | named volume present |
| P2-3 | PASS | without key: prompt/session fail-closed |
| P2-4 | PASS | pool stopped; no running boxes |
| P3-1 | PASS | disconnect replenished pool |
| P3-2 | PASS | second attach blocked at `size=1` |
| P3-3 | PASS | bad image never idle |
| P3-4 | SKIP | capabilities present; tool round-trip not automated |

## Artifacts

- `results.json`
- pool logs under this directory
- `image-digest.txt`, `kbox-version.txt`

## Not covered / follow-ups

- P3-4 minimal tool round-trip still SKIP
- Image digest recorded as tag-only (need `kbox image inspect` digest path)

## Failure diagnostics (if any)

```text
none — suite exit 0
```

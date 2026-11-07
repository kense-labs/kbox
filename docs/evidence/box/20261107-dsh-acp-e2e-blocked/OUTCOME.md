# dsh-acp × kbox E2E — 2026-11-07 (P0 blocked)

## Outcome

- Overall: **BLOCKED** (no live DeepSeek session)
- Tier: `p0`
- Command:

```sh
TIER=p0 E2E_OUT=docs/evidence/box/20261107-dsh-acp-e2e-blocked \
  ./scripts/e2e-dsh-acp/run.sh
```

Live greeting / cold initialize were not executed because `DEEPSEEK_API_KEY` was not available in the agent environment. The orchestrator completed, wrote `results.json`, and recorded both P0 cases as `BLOCKED`.

To obtain a green P0 baseline on this host:

```sh
export DEEPSEEK_API_KEY=…   # do not commit
TIER=p0 ./scripts/e2e-dsh-acp/run.sh
```

Then copy `results.json` into a new dated evidence directory and fill PASS rows.

## Environment

| Item | Value |
| --- | --- |
| Host OS / arch | macOS / arm64 |
| kbox version | 3.3.0 (`/Users/roylin/.kbox/bin/kbox`) |
| Image | `ghcr.io/kense-labs/dsh-acp:latest` |
| Image digest | tag-only (local inspect unavailable in this run) |
| Model / effort | defaults (`deepseek-flash` / `low`) — not exercised |
| Isolated `K_HOME` | yes (`…/20261107-dsh-acp-e2e-blocked/k-home`) |

## Case results

| ID | Status | Notes |
| --- | --- | --- |
| P0-1 | BLOCKED | `DEEPSEEK_API_KEY not set` |
| P0-2 | BLOCKED | `DEEPSEEK_API_KEY not set` |

## Artifacts

- `results.json`
- `kbox-version.txt`
- `image-digest.txt`

## Not covered / follow-ups

- Re-run `TIER=p0` (then `p1` / `all`) once a key is present.
- Pin image digest in evidence after a successful pull.

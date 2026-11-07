# dsh-acp × kbox E2E — TEMPLATE

Copy to `docs/evidence/box/YYYYMMDD-dsh-acp-e2e/OUTCOME.md` after a real run.
Attach `results.json` from the same directory (produced by `scripts/e2e-dsh-acp/run.sh`).
**Never** paste `DEEPSEEK_API_KEY` or other secrets into this file.

## Outcome

- Overall: PASS / FAIL / BLOCKED
- Tier: `p0` / `p0,p1` / `all`
- Command:

```sh
TIER=p0 ./scripts/e2e-dsh-acp/run.sh
```

## Environment

| Item | Value |
| --- | --- |
| Host OS / arch | |
| kbox version | |
| kbox binary path | |
| Image | `ghcr.io/kense-labs/dsh-acp:latest` |
| Image digest | |
| Model / effort | |
| Isolated `K_HOME` | yes (under `E2E_OUT/k-home`) |

## Case results

| ID | Status | Notes |
| --- | --- | --- |
| P0-1 | | |
| P0-2 | | |
| P1-1 | | |
| P1-2 | | |
| P1-3 | | |
| P1-4 | | |
| P1-5 | | |
| P1-6 | | |
| P2-1 | | |
| P2-2 | | |
| P2-3 | | |
| P2-4 | | |
| P3-1 | | |
| P3-2 | | |
| P3-3 | | |
| P3-4 | | |

## Artifacts

- `results.json`
- pool logs under `E2E_OUT/`
- `image-digest.txt`, `kbox-version.txt`

## Not covered / follow-ups

-

## Failure diagnostics (if any)

```text
# paste pool status / box list / log paths only — no secrets
```

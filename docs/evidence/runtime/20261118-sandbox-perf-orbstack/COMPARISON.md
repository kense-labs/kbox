# kbox vs OrbStack — lifecycle comparison

Date: 2026-11-18 · Host: darwin arm64 · ≠Pass  
Full write-up: [`OUTCOME.md`](OUTCOME.md) · raw: [`darwin-final.json`](darwin-final.json)

## Scope

**In:** create / start / run / exec / stop / restart / compose / cp / commit …  
**Out of verdict:** cold registry `pull` (network/auth; not a lifecycle claim)

## Headline (warm lifecycle)

| | Result |
| --- | --- |
| Fair warm pairs | 41 |
| kbox faster (ratio &lt; 0.8) | **39** |
| kbox slower (ratio &gt; 1.2) | **1** (`commit` ×1.24) |
| Median kbox/orb ratio | **0.34** |
| Verdict | **kbox ahead on lifecycle** |

## Lifecycle table

| cmd | kbox | OrbStack | ratio |
| --- | ---: | ---: | ---: |
| `run --rm` | 194 ms | 274 ms | 0.71 |
| `create` | 54 ms | 87 ms | 0.61 |
| `start` | 28 ms | 82 ms | 0.34 |
| `exec` | 6 ms | 26 ms | 0.24 |
| `stop` | 25 ms | ~10.1 s | ≪1 |
| `restart` | 36 ms | ~10.2 s | ≪1 |
| `compose up -d` | 135 ms | 289 ms | 0.47 |
| `cp` | 9 ms | 18 ms | 0.51 |
| `commit` | 240 ms | 194 ms | **1.24** |

Steady-state after a warm daemon: sub-second. First `run` after a **fresh**
daemon (~3.5–4 s) is one-time engine bring-up, not the daily loop.

## Pull (recorded, not scored)

Cold `pull alpine:3.20` ≈ 6.5 s (kbox) vs ≈ 2.1 s (OrbStack). Kept in raw JSON
only; do not use it as the product latency headline.

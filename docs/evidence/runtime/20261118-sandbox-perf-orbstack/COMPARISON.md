# kbox vs OrbStack — short comparison sheet

Date: 2026-11-18 · Host: darwin arm64 · ≠Pass  
Full write-up: [`OUTCOME.md`](OUTCOME.md) · raw: [`darwin-final.json`](darwin-final.json)

## Headline

| Mode | Winner |
| --- | --- |
| Warm daily path (41 fair pairs) | **kbox** — 39 faster / 1 slower; median ratio **0.34** |
| Cold `pull alpine:3.20` | **OrbStack** — ~2.1 s vs kbox ~6.5 s (~3×) |

## Warm highlights

| cmd | kbox | OrbStack | ratio |
| --- | ---: | ---: | ---: |
| `run --rm` | 194 ms | 274 ms | 0.71 |
| `exec` | 6 ms | 26 ms | 0.24 |
| `compose up -d` | 135 ms | 289 ms | 0.47 |
| `create` | 54 ms | 87 ms | 0.61 |
| `start` | 28 ms | 82 ms | 0.34 |
| `commit` | 240 ms | 194 ms | **1.24** |
| `cp` | 9 ms | 18 ms | 0.51 |

## Windows note

4090 WSL2 compared kbox (debug) to **Docker Engine 29.1.3**, not OrbStack.
Median fair ratio ≈ 1.0 — [`windows-wsl.json`](windows-wsl.json).

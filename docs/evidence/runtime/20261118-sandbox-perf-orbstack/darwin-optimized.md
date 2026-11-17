# Bench — kbox vs orbstack|29.8.2|OrbStack

Label: `darwin-arm64-optimized-manifest+daemon`  
Host: `Darwin arm64`  
Timed commands: **61** / expected **61**  
Comparable pairs: **46**  
Cold pull: kbox `6617` ms · orb `2315` ms  

| cmd | kbox_ms | orb_ms | ratio (k/o) | skip |
| --- | ---: | ---: | ---: | --- |
| `version` | 6.01 | 20.41 | 0.294 |  |
| `help` | 6.54 | 37.89 | 0.173 |  |
| `info` | 31.63 | 36.72 | 0.861 |  |
| `df` | 6.11 | 771.3 | 0.008 |  |
| `audit` | 7.23 | — | — | no docker equivalent (kbox audit ledger) |
| `pull` | 24.5 | 2088.12 | 0.012 | warm cache |
| `images` | 9.19 | 237.23 | 0.039 |  |
| `image-inspect` | 9.96 | 28.98 | 0.344 |  |
| `history` | 6.84 | 24.02 | 0.285 |  |
| `tag` | 15.21 | 38.79 | 0.392 |  |
| `save` | 12.84 | 188.93 | 0.068 |  |
| `load` | 27.02 | 144.98 | 0.186 |  |
| `rmi` | 15.2 | 23.42 | 0.649 | best-effort remove |
| `import` | 16.15 | 116.44 | 0.139 |  |
| `build` | 108.45 | 337.73 | 0.321 | metadata-only Dockerfile; kbox_rc=1 |
| `login` | 6.51 | — | — | orb skipped: would mutate docker creds store |
| `logout` | 6.35 | — | — | orb skipped: would mutate docker creds store |
| `push` | 1913.86 | — | — | fail-closed without registry; docker push would hang/auth |
| `create` | 51.35 | 84.63 | 0.607 |  |
| `start` | 30.38 | 78.64 | 0.386 |  |
| `rename` | 20.98 | 25.63 | 0.819 |  |
| `restart` | 35.3 | 10228.79 | 0.003 |  |
| `stop` | 27.34 | 10163.92 | 0.003 |  |
| `wait` | 9.45 | 20.85 | 0.453 | already stopped |
| `kill` | 25.5 | 118.32 | 0.216 |  |
| `rm` | 30.71 | 42.93 | 0.715 |  |
| `run` | 193.23 | 287.9 | 0.671 | warm daemon + cached image |
| `inspect` | 7.9 | 19.7 | 0.401 |  |
| `ps` | 6.4 | 18.7 | 0.342 |  |
| `exec` | 6.86 | 31.49 | 0.218 | kbox_rc=1 |
| `logs` | 6.26 | 16.16 | 0.388 |  |
| `pause` | 21.93 | 40.4 | 0.543 | kbox_rc=1 |
| `unpause` | 5.5 | 26.16 | 0.21 | kbox_rc=1 |
| `top` | 5.39 | 29.01 | 0.186 | kbox_rc=1 |
| `stats` | 5.94 | 1027.33 | 0.006 | kbox_rc=1 |
| `attach` | 6.98 | — | — | orb skipped: attach hangs without TTY protocol in bench |
| `shell` | 6.13 | 53.2 | 0.115 | orb: docker exec proxy (no TTY); kbox_rc=2 |
| `port` | 6.44 | 17.96 | 0.359 | may be empty without -p |
| `port-forward` | 5.9 | — | — | docker uses -p at create; no 1:1 CLI |
| `volume` | 13.95 | 24.06 | 0.58 |  |
| `network` | 5.57 | 39.81 | 0.14 |  |
| `cp` | 10.45 | 20.48 | 0.51 |  |
| `compose` | 130.35 | 315.53 | 0.413 |  |
| `diff` | 5.06 | 46.35 | 0.109 | kbox_rc=1 |
| `export` | 29.8 | 79.4 | 0.375 |  |
| `commit` | 237.26 | 200.62 | 1.183 |  |
| `container-update` | 16.13 | 32.32 | 0.499 |  |
| `events` | 2004.17 | 19.87 | 100.87 | bounded ~2s |
| `snapshot` | 6.03 | — | — | no docker CLI snapshot peer (checkpoint experimental) |
| `engine` | 4.97 | 41.0 | 0.121 | orb: docker info as engine status proxy; kbox_rc=2 |
| `pool` | 5.72 | — | — | kbox warm-pool; no orb peer |
| `monitor` | 4.66 | — | — | kbox monitor; no orb peer |
| `scale-api` | 4.64 | — | — | kbox scale-api; help-path timing only |
| `update` | 1103.16 | — | — | kbox update check; no orb peer |
| `attest` | 6.63 | — | — | TEE fail-closed; no docker peer |
| `seal` | 6.07 | — | — | TEE fail-closed; no docker peer |
| `unseal` | 5.25 | — | — | TEE fail-closed; no docker peer |
| `inject-secret` | 6.29 | — | — | TEE fail-closed; no docker peer |
| `prune` | 20.41 | 16.9 | 1.208 |  |
| `image-prune` | 5.46 | 124.02 | 0.044 |  |
| `system-prune` | 8.03 | 155.33 | 0.052 | destructive; timed last |

## Top kbox slower (ratio>1.2)

- `events`: 2004.17 / 19.87 ms (×100.87)
- `prune`: 20.41 / 16.9 ms (×1.208)

## Top kbox faster (ratio<0.8)

- `restart`: 35.3 / 10228.79 ms (×0.003)
- `stop`: 27.34 / 10163.92 ms (×0.003)
- `stats`: 5.94 / 1027.33 ms (×0.006)
- `df`: 6.11 / 771.3 ms (×0.008)
- `pull`: 24.5 / 2088.12 ms (×0.012)
- `images`: 9.19 / 237.23 ms (×0.039)
- `image-prune`: 5.46 / 124.02 ms (×0.044)
- `system-prune`: 8.03 / 155.33 ms (×0.052)
- `save`: 12.84 / 188.93 ms (×0.068)
- `diff`: 5.06 / 46.35 ms (×0.109)
- `shell`: 6.13 / 53.2 ms (×0.115)
- `engine`: 4.97 / 41.0 ms (×0.121)
- `import`: 16.15 / 116.44 ms (×0.139)
- `network`: 5.57 / 39.81 ms (×0.14)
- `help`: 6.54 / 37.89 ms (×0.173)

## Explicitly not claimed

- Product Pass / notarize


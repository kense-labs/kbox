# Bench — kbox vs orbstack|29.8.2|OrbStack

Label: `darwin-arm64-baseline-3.4.5`  
Host: `Darwin arm64`  
Timed commands: **61** / expected **61**  
Comparable pairs: **46**  
Cold pull: kbox `7120` ms · orb `2115` ms  

| cmd | kbox_ms | orb_ms | ratio (k/o) | skip |
| --- | ---: | ---: | ---: | --- |
| `version` | 5.75 | 18.79 | 0.306 |  |
| `help` | 6.62 | 32.32 | 0.205 |  |
| `info` | 31.85 | 37.04 | 0.86 |  |
| `df` | 5.89 | 848.3 | 0.007 |  |
| `audit` | 7.31 | — | — | no docker equivalent (kbox audit ledger) |
| `pull` | 28.76 | 2071.35 | 0.014 | warm cache |
| `images` | 6.12 | 234.63 | 0.026 |  |
| `image-inspect` | 7.68 | 21.12 | 0.364 |  |
| `history` | 5.82 | 19.52 | 0.298 |  |
| `tag` | 14.31 | 24.99 | 0.573 |  |
| `save` | 12.49 | 221.63 | 0.056 |  |
| `load` | 23.71 | 169.52 | 0.14 |  |
| `rmi` | 17.91 | 29.37 | 0.61 | best-effort remove |
| `import` | 15.57 | 100.17 | 0.155 |  |
| `build` | 108.22 | 458.42 | 0.236 | metadata-only Dockerfile; kbox_rc=1 |
| `login` | 6.1 | — | — | orb skipped: would mutate docker creds store |
| `logout` | 5.99 | — | — | orb skipped: would mutate docker creds store |
| `push` | 1946.35 | — | — | fail-closed without registry; docker push would hang/auth |
| `create` | 52.9 | 94.65 | 0.559 |  |
| `start` | 30.67 | 80.96 | 0.379 |  |
| `rename` | 19.27 | 23.47 | 0.821 |  |
| `restart` | 35.65 | 10222.14 | 0.003 |  |
| `stop` | 30.76 | 10141.42 | 0.003 |  |
| `wait` | 6.97 | 19.93 | 0.35 | already stopped |
| `kill` | 8.96 | 120.45 | 0.074 | kbox_rc=1 |
| `rm` | 6.31 | 43.31 | 0.146 | kbox_rc=1 |
| `run` | 92.06 | 254.39 | 0.362 | warm daemon + cached image; kbox_rc=1 |
| `inspect` | 5.88 | 16.79 | 0.35 |  |
| `ps` | 6.33 | 19.59 | 0.323 | kbox_rc=1 |
| `exec` | 5.83 | 35.92 | 0.162 | kbox_rc=1 |
| `logs` | 5.65 | 17.77 | 0.318 |  |
| `pause` | 15.42 | 41.19 | 0.374 | kbox_rc=1 |
| `unpause` | 6.11 | 32.71 | 0.187 | kbox_rc=1 |
| `top` | 6.5 | 39.23 | 0.166 | kbox_rc=1 |
| `stats` | 5.7 | 1024.86 | 0.006 | kbox_rc=1 |
| `attach` | 6.75 | — | — | orb skipped: attach hangs without TTY protocol in bench |
| `shell` | 6.33 | 60.7 | 0.104 | orb: docker exec proxy (no TTY); kbox_rc=2 |
| `port` | 7.35 | 22.32 | 0.33 | may be empty without -p; kbox_rc=1 |
| `port-forward` | 7.23 | — | — | docker uses -p at create; no 1:1 CLI |
| `volume` | 21.38 | 31.09 | 0.688 |  |
| `network` | 6.24 | 45.01 | 0.139 |  |
| `cp` | 6.06 | 20.98 | 0.289 | kbox_rc=1 |
| `compose` | 25.84 | 335.03 | 0.077 | kbox_rc=1 |
| `diff` | 5.94 | 42.96 | 0.138 | kbox_rc=1 |
| `export` | 5.8 | 58.22 | 0.1 | kbox_rc=1 |
| `commit` | 5.72 | 132.49 | 0.043 | kbox_rc=1 |
| `container-update` | 6.64 | 26.13 | 0.254 | kbox_rc=1 |
| `events` | 6.54 | 20.64 | 0.317 | bounded ~2s |
| `snapshot` | 5.78 | — | — | no docker CLI snapshot peer (checkpoint experimental) |
| `prune` | 6.36 | 18.7 | 0.34 | kbox_rc=1 |
| `image-prune` | 6.38 | 22.11 | 0.289 |  |
| `system-prune` | 6.82 | 410.85 | 0.017 | destructive within bench homes/engines; kbox_rc=1 |
| `engine` | 5.87 | 37.31 | 0.157 | orb: docker info as engine status proxy; kbox_rc=2 |
| `pool` | 5.36 | — | — | kbox warm-pool; no orb peer |
| `monitor` | 5.23 | — | — | kbox monitor; no orb peer |
| `scale-api` | 4.97 | — | — | kbox scale-api; help-path timing only |
| `update` | 1121.04 | — | — | kbox update check; no orb peer |
| `attest` | 5.97 | — | — | TEE fail-closed; no docker peer |
| `seal` | 4.99 | — | — | TEE fail-closed; no docker peer |
| `unseal` | 4.73 | — | — | TEE fail-closed; no docker peer |
| `inject-secret` | 5.24 | — | — | TEE fail-closed; no docker peer |

## Top kbox slower (ratio>1.2)


## Top kbox faster (ratio<0.8)

- `restart`: 35.65 / 10222.14 ms (×0.003)
- `stop`: 30.76 / 10141.42 ms (×0.003)
- `stats`: 5.7 / 1024.86 ms (×0.006)
- `df`: 5.89 / 848.3 ms (×0.007)
- `pull`: 28.76 / 2071.35 ms (×0.014)
- `system-prune`: 6.82 / 410.85 ms (×0.017)
- `images`: 6.12 / 234.63 ms (×0.026)
- `commit`: 5.72 / 132.49 ms (×0.043)
- `save`: 12.49 / 221.63 ms (×0.056)
- `kill`: 8.96 / 120.45 ms (×0.074)
- `compose`: 25.84 / 335.03 ms (×0.077)
- `export`: 5.8 / 58.22 ms (×0.1)
- `shell`: 6.33 / 60.7 ms (×0.104)
- `diff`: 5.94 / 42.96 ms (×0.138)
- `network`: 6.24 / 45.01 ms (×0.139)

## Explicitly not claimed

- Product Pass / notarize


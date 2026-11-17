# Bench — kbox vs orbstack|29.8.2|OrbStack

Label: `darwin-arm64-final`  
Host: `Darwin arm64`  
Timed commands: **61** / expected **61**  
Comparable pairs: **46**  
Cold pull: kbox `6540` ms · orb `2107` ms  

| cmd | kbox_ms | orb_ms | ratio (k/o) | skip |
| --- | ---: | ---: | ---: | --- |
| `version` | 5.51 | 16.72 | 0.33 |  |
| `help` | 5.68 | 32.66 | 0.174 |  |
| `info` | 35.52 | 34.03 | 1.044 |  |
| `df` | 6.0 | 764.89 | 0.008 |  |
| `audit` | 6.92 | — | — | no docker equivalent (kbox audit ledger) |
| `pull` | 25.78 | 2060.45 | 0.013 | warm cache |
| `images` | 6.56 | 254.47 | 0.026 |  |
| `image-inspect` | 7.84 | 25.45 | 0.308 |  |
| `history` | 6.06 | 17.94 | 0.338 |  |
| `tag` | 13.97 | 26.18 | 0.534 |  |
| `save` | 12.01 | 205.76 | 0.058 |  |
| `load` | 24.68 | 145.94 | 0.169 |  |
| `rmi` | 15.46 | 25.84 | 0.598 | best-effort remove |
| `import` | 16.01 | 131.79 | 0.121 |  |
| `build` | 110.21 | 401.67 | 0.274 | metadata-only Dockerfile; kbox_rc=1 |
| `login` | 6.57 | — | — | orb skipped: would mutate docker creds store |
| `logout` | 6.51 | — | — | orb skipped: would mutate docker creds store |
| `push` | 1985.37 | — | — | fail-closed without registry; docker push would hang/auth |
| `create` | 53.54 | 87.23 | 0.614 |  |
| `start` | 27.59 | 81.51 | 0.339 |  |
| `rename` | 15.26 | 25.47 | 0.599 |  |
| `restart` | 35.57 | 10235.43 | 0.003 |  |
| `stop` | 25.46 | 10148.71 | 0.003 |  |
| `wait` | 6.34 | 17.52 | 0.362 | already stopped |
| `kill` | 24.96 | 124.91 | 0.2 |  |
| `rm` | 29.1 | 48.8 | 0.596 |  |
| `run` | 194.17 | 274.46 | 0.707 | warm daemon + cached image |
| `inspect` | 6.47 | 16.38 | 0.395 |  |
| `ps` | 6.09 | 17.8 | 0.342 |  |
| `exec` | 6.24 | 26.35 | 0.237 |  |
| `logs` | 5.7 | 15.7 | 0.363 |  |
| `pause` | 28.06 | 40.75 | 0.689 |  |
| `unpause` | 22.1 | 31.11 | 0.71 |  |
| `top` | 7.51 | 32.8 | 0.229 |  |
| `stats` | 5.87 | 1029.06 | 0.006 |  |
| `attach` | 211.39 | — | — | orb skipped: attach hangs without TTY protocol in bench |
| `shell` | 5.31 | 45.15 | 0.118 | orb: docker exec proxy (no TTY); kbox_rc=2 |
| `port` | 5.3 | 17.07 | 0.311 | may be empty without -p |
| `port-forward` | 5.3 | — | — | docker uses -p at create; no 1:1 CLI |
| `volume` | 13.84 | 23.46 | 0.59 |  |
| `network` | 5.7 | 40.5 | 0.141 |  |
| `cp` | 9.07 | 17.75 | 0.511 |  |
| `compose` | 134.74 | 289.36 | 0.466 |  |
| `diff` | 5.98 | 49.15 | 0.122 | kbox_rc=1 |
| `export` | 29.54 | 83.16 | 0.355 |  |
| `commit` | 239.97 | 193.59 | 1.24 |  |
| `container-update` | 17.13 | 27.43 | 0.625 |  |
| `events` | 2005.42 | 22.41 | 89.507 | bounded ~2s |
| `snapshot` | 6.21 | — | — | no docker CLI snapshot peer (checkpoint experimental) |
| `engine` | 5.25 | 43.28 | 0.121 | orb: docker info as engine status proxy; kbox_rc=2 |
| `pool` | 6.02 | — | — | kbox warm-pool; no orb peer |
| `monitor` | 5.36 | — | — | kbox monitor; no orb peer |
| `scale-api` | 5.25 | — | — | kbox scale-api; help-path timing only |
| `update` | 1064.56 | — | — | kbox update check; no orb peer |
| `attest` | 6.2 | — | — | TEE fail-closed; no docker peer |
| `seal` | 5.32 | — | — | TEE fail-closed; no docker peer |
| `unseal` | 5.2 | — | — | TEE fail-closed; no docker peer |
| `inject-secret` | 5.66 | — | — | TEE fail-closed; no docker peer |
| `prune` | 4.99 | 20.71 | 0.241 |  |
| `image-prune` | 5.34 | 127.98 | 0.042 |  |
| `system-prune` | 8.23 | 170.07 | 0.048 | destructive; timed last |

## Top kbox slower (ratio>1.2)

- `events`: 2005.42 / 22.41 ms (×89.507)
- `commit`: 239.97 / 193.59 ms (×1.24)

## Top kbox faster (ratio<0.8)

- `restart`: 35.57 / 10235.43 ms (×0.003)
- `stop`: 25.46 / 10148.71 ms (×0.003)
- `stats`: 5.87 / 1029.06 ms (×0.006)
- `df`: 6.0 / 764.89 ms (×0.008)
- `pull`: 25.78 / 2060.45 ms (×0.013)
- `images`: 6.56 / 254.47 ms (×0.026)
- `image-prune`: 5.34 / 127.98 ms (×0.042)
- `system-prune`: 8.23 / 170.07 ms (×0.048)
- `save`: 12.01 / 205.76 ms (×0.058)
- `shell`: 5.31 / 45.15 ms (×0.118)
- `import`: 16.01 / 131.79 ms (×0.121)
- `engine`: 5.25 / 43.28 ms (×0.121)
- `diff`: 5.98 / 49.15 ms (×0.122)
- `network`: 5.7 / 40.5 ms (×0.141)
- `load`: 24.68 / 145.94 ms (×0.169)

## Explicitly not claimed

- Product Pass / notarize


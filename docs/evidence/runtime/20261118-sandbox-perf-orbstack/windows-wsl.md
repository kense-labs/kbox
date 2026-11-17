# Bench — kbox vs DESKTOP-BM5D1UH|29.1.3|Ubuntu 24.04.3 LTS

Label: `windows-4090-wsl-vs-docker`  
Host: `Linux x86_64`  
Timed commands: **61** / expected **61**  
Comparable pairs: **46**  
Cold pull: kbox `5770` ms · orb `2531` ms  

| cmd | kbox_ms | orb_ms | ratio (k/o) | skip |
| --- | ---: | ---: | ---: | --- |
| `version` | 136.67 | 195.89 | 0.698 |  |
| `help` | 151.0 | 206.63 | 0.731 |  |
| `info` | 260.82 | 216.54 | 1.204 |  |
| `df` | 254.12 | 9435.48 | 0.027 |  |
| `audit` | 183.95 | — | — | no docker equivalent (kbox audit ledger) |
| `pull` | 441.66 | 2479.42 | 0.178 | warm cache |
| `images` | 211.28 | 425.23 | 0.497 |  |
| `image-inspect` | 333.23 | 215.13 | 1.549 |  |
| `history` | 231.06 | 192.96 | 1.197 |  |
| `tag` | 244.26 | 208.79 | 1.17 |  |
| `save` | 230.33 | 362.74 | 0.635 |  |
| `load` | 400.51 | 524.98 | 0.763 |  |
| `rmi` | 272.52 | 219.44 | 1.242 | best-effort remove |
| `import` | 249.77 | 351.01 | 0.712 |  |
| `build` | 1009.86 | 1145.31 | 0.882 | metadata-only Dockerfile |
| `login` | 143.12 | — | — | orb skipped: would mutate docker creds store |
| `logout` | 39.26 | — | — | orb skipped: would mutate docker creds store |
| `push` | 1742.49 | — | — | fail-closed without registry; docker push would hang/auth |
| `create` | 386.57 | 307.92 | 1.255 |  |
| `start` | 346.99 | 354.33 | 0.979 |  |
| `rename` | 234.09 | 209.01 | 1.12 |  |
| `restart` | 356.76 | 10544.0 | 0.034 |  |
| `stop` | 323.72 | 10355.87 | 0.031 |  |
| `wait` | 267.33 | 184.48 | 1.449 | already stopped |
| `kill` | 321.88 | 382.45 | 0.842 |  |
| `rm` | 342.94 | 239.82 | 1.43 |  |
| `run` | 675.91 | 675.09 | 1.001 | warm daemon + cached image |
| `inspect` | 229.87 | 184.68 | 1.245 |  |
| `ps` | 226.04 | 212.96 | 1.061 |  |
| `exec` | 266.27 | 244.46 | 1.089 |  |
| `logs` | 209.68 | 188.32 | 1.113 |  |
| `pause` | 321.86 | 219.98 | 1.463 |  |
| `unpause` | 316.65 | 185.54 | 1.707 |  |
| `top` | 259.37 | 192.85 | 1.345 |  |
| `stats` | 225.85 | 1203.06 | 0.188 |  |
| `attach` | 440.49 | — | — | orb skipped: attach hangs without TTY protocol in bench |
| `shell` | 137.36 | 242.09 | 0.567 | orb: docker exec proxy (no TTY); kbox_rc=2 |
| `port` | 214.16 | 184.52 | 1.161 | may be empty without -p |
| `port-forward` | 228.02 | — | — | docker uses -p at create; no 1:1 CLI |
| `volume` | 204.94 | 195.56 | 1.048 |  |
| `network` | 189.7 | 214.52 | 0.884 |  |
| `cp` | 276.86 | 189.67 | 1.46 |  |
| `compose` | 623.24 | 1063.39 | 0.586 |  |
| `diff` | 231.58 | 253.27 | 0.914 | kbox_rc=1 |
| `export` | 345.32 | 421.83 | 0.819 |  |
| `commit` | 1701.77 | 505.51 | 3.366 |  |
| `container-update` | 264.29 | 224.1 | 1.179 |  |
| `events` | 2010.2 | 217.33 | 9.25 | bounded ~2s |
| `snapshot` | 143.34 | — | — | no docker CLI snapshot peer (checkpoint experimental) |
| `engine` | 9.97 | 228.27 | 0.044 | orb: docker info as engine status proxy; kbox_rc=2 |
| `pool` | 159.89 | — | — | kbox warm-pool; no orb peer |
| `monitor` | 149.9 | — | — | kbox monitor; no orb peer |
| `scale-api` | 11.43 | — | — | kbox scale-api; help-path timing only |
| `update` | 235.17 | — | — | kbox update check; no orb peer |
| `attest` | 224.18 | — | — | TEE fail-closed; no docker peer |
| `seal` | 142.76 | — | — | TEE fail-closed; no docker peer |
| `unseal` | 141.54 | — | — | TEE fail-closed; no docker peer |
| `inject-secret` | 226.16 | — | — | TEE fail-closed; no docker peer |
| `prune` | 242.15 | 4164.66 | 0.058 |  |
| `image-prune` | 297.74 | 302.82 | 0.983 |  |
| `system-prune` | 313.86 | 4167.45 | 0.075 | destructive; timed last |

## Top kbox slower (ratio>1.2)

- `events`: 2010.2 / 217.33 ms (×9.25)
- `commit`: 1701.77 / 505.51 ms (×3.366)
- `unpause`: 316.65 / 185.54 ms (×1.707)
- `image-inspect`: 333.23 / 215.13 ms (×1.549)
- `pause`: 321.86 / 219.98 ms (×1.463)
- `cp`: 276.86 / 189.67 ms (×1.46)
- `wait`: 267.33 / 184.48 ms (×1.449)
- `rm`: 342.94 / 239.82 ms (×1.43)
- `top`: 259.37 / 192.85 ms (×1.345)
- `create`: 386.57 / 307.92 ms (×1.255)
- `inspect`: 229.87 / 184.68 ms (×1.245)
- `rmi`: 272.52 / 219.44 ms (×1.242)
- `info`: 260.82 / 216.54 ms (×1.204)

## Top kbox faster (ratio<0.8)

- `df`: 254.12 / 9435.48 ms (×0.027)
- `stop`: 323.72 / 10355.87 ms (×0.031)
- `restart`: 356.76 / 10544.0 ms (×0.034)
- `engine`: 9.97 / 228.27 ms (×0.044)
- `prune`: 242.15 / 4164.66 ms (×0.058)
- `system-prune`: 313.86 / 4167.45 ms (×0.075)
- `pull`: 441.66 / 2479.42 ms (×0.178)
- `stats`: 225.85 / 1203.06 ms (×0.188)
- `images`: 211.28 / 425.23 ms (×0.497)
- `shell`: 137.36 / 242.09 ms (×0.567)
- `compose`: 623.24 / 1063.39 ms (×0.586)
- `save`: 230.33 / 362.74 ms (×0.635)
- `version`: 136.67 / 195.89 ms (×0.698)
- `import`: 249.77 / 351.01 ms (×0.712)
- `help`: 151.0 / 206.63 ms (×0.731)

## Explicitly not claimed

- Product Pass / notarize


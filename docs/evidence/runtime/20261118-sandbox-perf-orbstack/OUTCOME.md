# OUTCOME — Sandbox 61-cmd perf vs OrbStack / Docker (dual-end)

Date: 2026-11-18  
Branch: `main` (P17)  
≠Pass · ≠notarize

## Intent (YOLO named scope)

双端 61 个顶层命令 **逐项计时**，相对本机 **OrbStack**（darwin）做对照；Windows
4090 上 Spec050 路径走 WSL2，对照为本机 **Docker Engine**（OrbStack 无 Windows
版）。对测出的热点做深度优化并复测。

## Harness

```bash
python3 scripts/bench-sandbox-vs-orbstack.py \
  --kbox "$PREFIX/bin/kbox" \
  --docker ~/.orbstack/bin/docker \
  --out …/darwin-final.json
```

- 覆盖 `kbox --help` 全部 **61** 顶层命令（有 Docker 映射则成对计时；否则
  `orb_skip` + 原因）。
- 协议：冷 `pull` 单独记入 `cold.*`；矩阵主体为 **暖机**（daemon 已起、镜像已缓存）。
- 公平比：`kbox_rc==0 && orb_rc==0` 的成对行（见下表「公平对」）。

## Deep optimizations landed

| Change | Why | Evidence |
| --- | --- | --- |
| `ensure_daemon` writes `daemon.pid`; stale pid → clear + respawn; wait until socket **accepts connect** | Dead engine left a socket → every later CLI `ConnectionRefused` | `ensure_daemon` tests; recover run ~0.5s |
| `reap_stale_daemon_socket` + `SandboxLocalExecutionBackend::client` respawns on stale even for exec/inspect | Exec must not stay fail-closed after engine crash when boxes still exist | Mac bench exec/pause now rc=0 |
| Registry manifest path: one tagged `pull_manifest_raw` + at most one platform digest fetch (removed redundant re-fetch after `pull_image_manifest`) | Cold pull paid an extra manifest RTT | `oci::registry` tests green; cold pull still network-dominated |

## Darwin arm64 — kbox vs OrbStack 29.8.2

Artifacts: `darwin-final.json` · `darwin-final.md` · `COMPARISON.md`  
Binary: `release-dist` kbox + engine（本脉冲优化后）

**Product verdict uses lifecycle only** (create/start/run/exec/stop/…), not cold
`pull`.

| Metric | kbox | OrbStack docker |
| --- | --- | --- |
| Warm fair pairs | 41 | 41 |
| Fair pairs kbox slower (×>1.2) | **1** (`commit` ×1.24) | — |
| Fair pairs kbox faster (×<0.8) | **39** | — |
| Median kbox/orb ratio | **0.34** | — |

First `run --rm` after fresh daemon ≈ **3.5–4.0 s** (one-time engine bring-up);
warm `run --rm` ≈ **195 ms** vs OrbStack ≈ **256 ms** (**kbox faster**).

Cold `pull` (~6.5 s vs ~2.1 s) remains in raw JSON as network/auth cost; **not**
part of the lifecycle headline.

### Warm lifecycle highlights (fair success pairs)

| cmd | kbox_ms | orb_ms | ratio |
| --- | ---: | ---: | ---: |
| `run` | 195 | 256 | 0.76 |
| `exec` | 6.2 | 26 | 0.24 |
| `compose up -d` | 136 | 295 | 0.46 |
| `create`/`start` | 56 / 26 | 95 / 78 | 0.58 / 0.34 |
| `stop`/`restart` | 25 / 36 | ~10140 / ~10235 | ≪1 (OrbStack waits container stop) |
| `commit` | 240 | 194 | **1.24** |
| `cp` | ~10 | ~21 | 0.50 |

Expected non-comparable / fail-closed (timed, not fair pairs): `push`/`login`,
TEE (`attest`/`seal`/…), `port-forward`, `snapshot`, `events` (2s bound),
`engine`/`monitor` CLI shape, `build` (metadata Dockerfile needs host-run
policy on this seat).

## Windows 4090 WSL2 — kbox vs Docker Engine 29.1.3

Artifacts: `windows-wsl.json` · `windows-wsl.md`  
Binary: worktree **debug** kbox（绝对值偏慢；趋势仍有效）  
对照：**不是 OrbStack**（该主机无 OrbStack；Docker Engine on Ubuntu 24.04）

| Metric | kbox (debug) | docker |
| --- | --- | --- |
| Cold pull | ~5770 ms | ~2531 ms |
| Comparable pairs | 46 | 46 |
| kbox slower (raw harness) | 13 | — |
| kbox faster | 16 | — |

Debug build inflates kbox ms. Still: warm `pull`/`images`/`stop`/`restart`/`df`
beat docker; `commit`/`cp`/`pause` slower on this debug seat. Re-run with
`release-dist` on WSL before claiming product numbers.

## Explicitly not claimed

- Product Pass / notarize / Gatekeeper staple
- OrbStack on Windows (N/A) — WSL Docker is the Windows comparator
- Identical semantics for every mapped docker command
- Cold-pull parity with OrbStack after this pulse
- Debug WSL timings as release-grade absolute latency

## Next knives (if continuing)

1. Disk-backed registry bearer cache across CLI processes (cut cold auth RTT)
2. Optional `POST /kbox/wake` during first `ensure_daemon` to front-load VM boot
3. WSL `release-dist` rebench for absolute dual-end parity sheet
4. `commit` path profile (only fair darwin regression vs OrbStack)

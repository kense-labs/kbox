# KBox

A Docker-shaped OCI runtime for local agents and workloads. Same CLI shape as Docker (`run` / `exec` / `compose` / …), with two isolation grades:

- **Sandbox** (default) — in-tree container engine
- **MicroVM** — `kbox run --isolation microvm` via libkrun / libkrunfw

English | [简体中文](README.zh-CN.md)

## Architecture

![KBox architecture](docs/architecture/kbox-architecture.png)

[Editable SVG](docs/architecture/kbox-architecture.svg)

```text
Users / CLI / CRI / shim
        │
        ▼
ExecutionManager  (resource authority)
        │
        ├─ default ► Sandbox engine
        └─ optional ► libkrun MicroVM (--isolation microvm)
```

## Install

### macOS arm64

From [GitHub Releases](https://github.com/kense-labs/kbox/releases/tag/v3.4.5):

```sh
tar -xzf kbox-3.4.5-macos-arm64.tar.gz
KBOX_PREFIX="$HOME/.kbox" sh kbox-3.4.5-macos-arm64/install.sh
xattr -dr com.apple.quarantine "$HOME/.kbox" || true
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

```sh
kbox update --check
kbox update
```

### Windows (WSL2)

```sh
tar -xzf kbox-3.4.5-windows-wsl-x86_64.tar.gz
KBOX_PREFIX="$HOME/.kbox" sh kbox-3.4.5-windows-wsl-x86_64/install.sh
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

## Quick start

```sh
kbox run --rm alpine:3.20 -- echo hello-kbox
```

```sh
kbox run -d --name web -p 8080:80 nginx:alpine
curl -fsS http://127.0.0.1:8080/ | head
kbox rm -f web
```

```sh
kbox run --rm --isolation microvm alpine:3.20 -- echo hello-microvm
```

## Performance

Warm lifecycle vs OrbStack Docker 29.8.2 on darwin arm64 (cached image):

| Lifecycle | kbox | OrbStack | ratio |
| --- | ---: | ---: | ---: |
| `run --rm` | 194 ms | 274 ms | 0.71 |
| `create` | 54 ms | 87 ms | 0.61 |
| `start` | 28 ms | 82 ms | 0.34 |
| `exec` | 6 ms | 26 ms | 0.24 |
| `stop` | 25 ms | ~10.1 s | ≪1 |
| `restart` | 36 ms | ~10.2 s | ≪1 |
| `compose up -d` | 135 ms | 289 ms | 0.47 |
| `cp` | 9 ms | 18 ms | 0.51 |
| `commit` | 240 ms | 194 ms | 1.24 |

Reproduce: `python3 scripts/bench-sandbox-vs-orbstack.py`. Details: [`docs/evidence/runtime/20261118-sandbox-perf-orbstack/`](docs/evidence/runtime/20261118-sandbox-perf-orbstack/).

## What you get

| Area | Support |
| --- | --- |
| Images | `pull` / `build` / `commit` / `export` / `import` / login |
| Run | `-d`, `-i`, `-v`, `-e`, `-p`, `-u`, `--rm`, limits, `--isolation` |
| Compose / pool | Multi-service; Sandbox networks; `pool start --prewarm` |
| Snapshot / volume / network | First-class CLI groups |
| Isolation | Sandbox default · MicroVM via `--isolation microvm` |

Command details: `kbox --help`.

## Platforms

| Host | Sandbox | MicroVM | Prebuilt package |
| --- | --- | --- | --- |
| macOS arm64 | Yes | HVF + libkrun | `v3.4.5` |
| Linux x86_64 | Yes | KVM | WSL asset / build from source |
| Windows | WSL2 | WHPX (in-tree) | `v3.4.5` WSL archive |

## Development

```sh
git clone --recursive https://github.com/kense-labs/kbox
cd kbox
cargo build --locked -p kbox-cli -p kbox-sandbox-engine
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
```

Set a task-owned `K_HOME` before local tests.

```text
crates/box/           CLI · runtime · sandbox engine
crates/oci-runtime/   OCI / DedicatedVm stack
crates/runtime/       Task / Service contracts
libkrun/ libkrunfw/   virtualization submodules
```

## License

[MIT](LICENSE). Git submodules and third-party dependencies keep their own licenses; see crate `NOTICE` files where present.

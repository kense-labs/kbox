# KBox

A **Docker-shaped MicroVM runtime** for local agents and OCI workloads. Pull public images, run each box in its own guest Linux kernel (libkrun), and manage lifecycle, volumes, networks, compose, snapshots, and warm pools from one CLI.

English | [简体中文](README.zh-CN.md)

## How it works

Every `kbox run` starts a **DedicatedVm**: OCI rootfs + guest init inside an HVF MicroVM—not a shared-host-kernel container.

```text
Host (macOS arm64)
    │
    ▼
KBox CLI  ──►  image store / volumes / networks
    │
    ▼
kbox-shim + libkrun / libkrunfw
    │
    ▼
Guest Linux MicroVM  ──►  your process (nginx, redis, agent, …)
```

- **OCI images in, MicroVMs out** — alpine, nginx, postgres, custom agents all use the same path.
- **No Docker Engine API** — CLI (and optional CRI) only; default networking is TSI, not Docker bridge.

## Install

macOS arm64 (Apple Silicon):

```sh
curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.4.2/install.sh | sh
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

```sh
kbox update --check
kbox update
```

Package ≈ **22 MB** compressed / **≈ 42 MB** installed (`kbox`, `kbox-shim`, `kbox-guest-init`, libkrun, libkrunfw). Other platforms: build from source below.

## Quick start

```sh
kbox run --rm docker.io/library/alpine:latest -- echo hello-kbox
```

Detached service with host port publish:

```sh
kbox run -d --name web -p 8080:80 docker.io/library/nginx:alpine
curl -fsS http://127.0.0.1:8080/ | head
kbox rm -f web
```

ACP agent (stdio launcher—not a terminal chat):

```sh
export DEEPSEEK_API_KEY=your_key
kbox run -i --rm --memory 2g -u root \
  -v dsh-acp-data:/data -v "$PWD:/workspace" \
  -e DEEPSEEK_API_KEY \
  ghcr.io/kense-labs/dsh-acp:latest
```

Live greeting bench (no clone):

```sh
DEEPSEEK_API_KEY=your_key DSH_MODEL=deepseek-flash DSH_REASONING_EFFORT=low RUNS=1 \
  bash -o pipefail -c \
  'curl -fsSL https://raw.githubusercontent.com/kense-labs/dsh-acp/main/scripts/bench-pool-dsh-acp.sh | sh'
```

## What you get

| Area | Support |
| --- | --- |
| Images | `pull` / `build` / `commit` / `export` / `import` / registry login |
| Run | `-d`, `-i`, `-v`, `-e`, `-p`, `-u`, `--entrypoint`, `--rm`, resource limits |
| Compose / pool | Multi-service MicroVMs; `pool start --prewarm` + `pool attach` |
| Snapshot / network / volume | First-class CLI groups |

Command details: `kbox --help` and `kbox <command> --help` (source of truth).

## Platforms

| Host | Runtime | Prebuilt package |
| --- | --- | --- |
| macOS arm64 | Apple HVF + libkrun | Yes (`kbox-v*` on [kense Releases](https://github.com/kense-labs/kense/releases)) |
| Linux | KVM path in-tree | Build + qualify locally |
| Windows | WHPX path in-tree | Build + qualify locally |

Shared-host-kernel is **not** a Box product path.

## Development

```sh
git clone --recursive https://github.com/kense-labs/kbox
cd kbox
cargo build --locked -p kbox-cli
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
```

E2E (isolated `K_HOME`):

```sh
# dsh-acp × pool
DEEPSEEK_API_KEY=… TIER=p0 ./scripts/e2e-dsh-acp/run.sh

# nginx / redis / postgres / rustfs functional deploy
TIER=all ./scripts/e2e-services/run.sh
```

Plans and evidence: [docs/e2e-dsh-acp/](docs/e2e-dsh-acp/), [docs/e2e-services/](docs/e2e-services/), [docs/evidence/box/](docs/evidence/box/).

Layout:

```text
crates/box/         MicroVM Box CLI + runtime
crates/oci-runtime/ OCI / DedicatedVm platform stack
crates/runtime/     Task / Service contracts
libkrun/ libkrunfw/ virtualization submodules
```

## License

MIT. See [LICENSE](LICENSE). Submodules retain their upstream licenses.

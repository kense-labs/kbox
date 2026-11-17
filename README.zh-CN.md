# KBox

面向本地 Agent 与 OCI 工作负载的 Docker 风格运行时。CLI 形态与 Docker 同形（`run` / `exec` / `compose` / …），隔离分两级：

- **Sandbox**（默认）— 仓内容器引擎
- **MicroVM** — `kbox run --isolation microvm`，经 libkrun / libkrunfw

[English](README.md) | 简体中文

## 架构

![KBox 架构](docs/architecture/kbox-architecture.png)

[可编辑 SVG](docs/architecture/kbox-architecture.svg)

```text
用户 / CLI / CRI / shim
        │
        ▼
ExecutionManager  （资源权威）
        │
        ├─ 默认 ► Sandbox 引擎
        └─ 可选 ► libkrun MicroVM（--isolation microvm）
```

## 安装

### macOS arm64

资产见 [GitHub Releases](https://github.com/kense-labs/kbox/releases/tag/v3.4.5)：

```sh
tar -xzf kbox-3.4.5-macos-arm64.tar.gz
KBOX_PREFIX="$HOME/.kbox" sh kbox-3.4.5-macos-arm64/install.sh
xattr -dr com.apple.quarantine "$HOME/.kbox" || true
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

### Windows（WSL2）

```sh
tar -xzf kbox-3.4.5-windows-wsl-x86_64.tar.gz
KBOX_PREFIX="$HOME/.kbox" sh kbox-3.4.5-windows-wsl-x86_64/install.sh
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

## 快速开始

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

## 性能

darwin arm64 暖机生命周期对照 OrbStack Docker 29.8.2（镜像已缓存）：

| 生命周期 | kbox | OrbStack | 比值 |
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

复现：`python3 scripts/bench-sandbox-vs-orbstack.py`。细节见 [`docs/evidence/runtime/20261118-sandbox-perf-orbstack/`](docs/evidence/runtime/20261118-sandbox-perf-orbstack/)。

## 能力

| 领域 | 支持 |
| --- | --- |
| 镜像 | `pull` / `build` / `commit` / `export` / `import` / login |
| 运行 | `-d`, `-i`, `-v`, `-e`, `-p`, `-u`, `--rm`, 限额, `--isolation` |
| Compose / pool | 多服务；Sandbox 网络；`pool start --prewarm` |
| 快照 / 卷 / 网络 | 一级 CLI 分组 |
| 隔离 | 默认 Sandbox · MicroVM 用 `--isolation microvm` |

命令细节：`kbox --help`。

## 平台

| 宿主 | Sandbox | MicroVM | 预编译包 |
| --- | --- | --- | --- |
| macOS arm64 | 有 | HVF + libkrun | `v3.4.5` |
| Linux x86_64 | 有 | KVM | WSL 资产 / 源码构建 |
| Windows | WSL2 | WHPX（树内） | `v3.4.5` WSL 归档 |

## 开发

```sh
git clone --recursive https://github.com/kense-labs/kbox
cd kbox
cargo build --locked -p kbox-cli -p kbox-sandbox-engine
```

本地测试前设置任务私有 `K_HOME`。

## 许可

[MIT](LICENSE)。Git 子模块与第三方依赖保留各自许可；见各 crate 的 `NOTICE`（如有）。

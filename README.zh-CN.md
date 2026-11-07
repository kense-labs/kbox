# kbox

[English](README.md) | 简体中文

面向本地 Agent 与 OCI 工作负载的 **Docker 风格 MicroVM 运行时**。拉取公网镜像，每个 Box 跑在独立客体内核（libkrun）里；用同一套 CLI 管理生命周期、卷、网络、Compose、快照与预热池。

## 工作模型

每次 `kbox run` 启动的是 **DedicatedVm**：OCI rootfs + guest init，落在 HVF MicroVM 中——不是共享宿主内核的容器。

```text
宿主 (macOS arm64)
    │
    ▼
kbox CLI  ──►  镜像库 / 卷 / 网络
    │
    ▼
kbox-shim + libkrun / libkrunfw
    │
    ▼
Guest Linux MicroVM  ──►  你的进程（nginx、redis、agent…）
```

- **镜像进、MicroVM 出** — alpine、nginx、postgres、自定义 agent 同一路径。
- **无 Docker Engine API** — 只有 CLI（及可选 CRI）；默认网络为 TSI。

## 安装

macOS arm64：

```sh
curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.4.2/install.sh | sh
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

```sh
kbox update --check
kbox update
```

压缩包约 **22 MB**，安装后约 **42 MB**。其他平台请从源码构建。

## 快速开始

```sh
kbox run --rm docker.io/library/alpine:latest -- echo hello-kbox
```

后台服务 + 宿主端口：

```sh
kbox run -d --name web -p 8080:80 docker.io/library/nginx:alpine
curl -fsS http://127.0.0.1:8080/ | head
kbox rm -f web
```

更多：`kbox --help`。dsh-acp 与常用服务 E2E 见英文 README 的 Development 一节与 `docs/e2e-*`。

## 平台

| 宿主 | 运行时 | 预编译包 |
| --- | --- | --- |
| macOS arm64 | Apple HVF + libkrun | 有（[kense Releases](https://github.com/kense-labs/kense/releases) 的 `kbox-v*`） |
| Linux / Windows | 树内有路径 | 需本地构建与资格化 |

共享宿主内核**不是** Box 产品路径。

## 开发

```sh
git clone --recursive https://github.com/kense-labs/kbox
cd kbox
cargo build --locked -p kbox-cli
```

## 许可

见仓库 `LICENSE` / crate 清单；子模块保留各自上游许可。

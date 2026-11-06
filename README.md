# kbox

kbox 是本地 Agent Runtime 与 OCI 工作负载执行工作区。它提供隔离执行环境、资源生命周期和 Task／Service 合同。

本文先介绍安装与快速开始，再列出 `kbox` 的完整命令层级。命令内容以当前源码的 `--help` 输出为准。

## 1. 安装

### 1.1 一键安装（macOS / Linux）

```sh
curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.2.7/install.sh | sh
```

脚本自动检测操作系统与架构，从 GitHub Releases 下载对应安装包、校验 SHA-256，并安装到 `~/.kbox`（无需 sudo）。按输出提示把 `~/.kbox/bin` 加入 `PATH` 即可。

可选环境变量：`KBOX_VERSION`（默认 `3.2.7`）、`KBOX_PREFIX`（默认 `~/.kbox`）。

预编译包支持矩阵：macOS arm64（Apple Silicon，Hypervisor Framework）。其他平台会给出明确提示，请改用源码构建。

### 1.2 手动安装

从 [kense Releases](https://github.com/kense-labs/kense/releases/tag/kbox-v3.2.7) 下载对应平台的 tar.gz，解压后运行包内 `install.sh`：

```sh
tar -xzf kbox-3.2.7-macos-arm64.tar.gz
sh kbox-3.2.7-macos-arm64/install.sh
```

安装包包含 `kbox` CLI、`kbox-shim`、静态 Linux guest init、libkrun 与 libkrunfw（含客体内核）。压缩包约 23 MB，安装后约 42 MB。

## 2. 快速开始

```sh
# 运行一个 alpine MicroVM
kbox run --rm docker.io/library/alpine:latest -- echo hello-kbox

# 运行 dsh-acp（公开镜像，匿名拉取），接入 ACP 客户端
export DEEPSEEK_API_KEY=your_key
kbox run -i --rm \
  --memory 2g \
  -v dsh-acp-data:/data \
  -v "$PWD:/workspace" \
  -e DEEPSEEK_API_KEY \
  ghcr.io/kense-labs/dsh-acp:latest
```

`kbox` 管理 Box 生命周期、执行、镜像、网络、卷、Compose 项目、快照和预热池。完整命令见第 6 章起。

## 3. 工程结构

```text
kbox/
├── crates/box/            执行资源、MicroVM 与生命周期
├── crates/oci-runtime/    OCI 工作负载运行时
├── crates/runtime/        Task／Service 合同与控制接口
├── libkrun/               libkrun 子模块
└── libkrunfw/             libkrunfw 子模块
```

`libkrun` 和 `libkrunfw` 是独立 Git submodule。修改原生依赖时，先在对应子仓库提交并验证，再更新父仓库的 gitlink。

## 4. 当前能力与目标边界

当前代码包含 Box、OCI Runtime、Task／Service 合同和多个命令行工具。

`docs/architecture/agent-paas-runtime.md` 中的 Agent Runtime v1.1 是目标架构。文档中的新增能力不代表已经实现。实际支持范围以源码、测试和原生平台资格化结果为准。

## 5. 从源码构建

### 5.1 克隆

```bash
git clone --recursive https://github.com/kense-labs/kbox
cd kbox
```

已有检出使用：

```bash
git submodule update --init --recursive
```

### 5.2 工具链

Rust 版本见 `rust-toolchain.toml`。`publish = false` 的包只通过本地路径解析。

### 5.3 构建 CLI

```bash
cargo build --locked -p kbox-cli
```

生成的入口位于：

```text
target/debug/kbox
```

### 5.4 验证

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --locked -p kbox_contract -p kbox-core -p klibksys --lib
cargo clippy --locked -p klibksys --all-targets --no-deps
```

Linux KVM、macOS HVF 和 Windows WHPX 的运行时支持声明需要对应原生测试。交叉编译不能替代运行时资格化。

## 6. `kbox` 命令入口

```bash
./target/debug/kbox <COMMAND>
```

`kbox` 管理 Box 生命周期、执行、镜像、网络、卷、Compose 项目、快照和预热池。

## 7. 生命周期与执行

| 二级命令 | 说明 |
| --- | --- |
| `kbox run [OPTIONS] <IMAGE> [-- <CMD>...]` | 拉取镜像、创建并启动 Box |
| `kbox create [OPTIONS] <IMAGE> [-- <CMD>...]` | 创建 Box，但不启动 |
| `kbox start <BOXES>...` | 启动一个或多个 Box |
| `kbox stop [OPTIONS] <BOXES>...` | 停止运行中的 Box |
| `kbox restart [OPTIONS] <BOXES>...` | 重启 Box |
| `kbox rm [OPTIONS] <BOXES>...` | 删除 Box |
| `kbox kill [OPTIONS] <BOXES>...` | 强制终止运行中的 Box |
| `kbox pause <BOXES>...` | 暂停运行中的 Box |
| `kbox unpause <BOXES>...` | 恢复已暂停的 Box |
| `kbox ps [OPTIONS]` | 列出 Box |
| `kbox stats [OPTIONS] [BOX]` | 查看 CPU、内存和资源统计 |
| `kbox logs [OPTIONS] <BOX>` | 查看 Box 日志 |
| `kbox exec [OPTIONS] <BOX> -- <CMD>...` | 在运行中的 Box 内执行命令 |
| `kbox top [OPTIONS] <BOX> [-- <PS_ARGS>...]` | 查看 Box 内运行进程 |
| `kbox inspect <BOX>` | 查看 Box 详细信息 |
| `kbox attach [OPTIONS] <BOX>` | 连接 Box 控制台输出 |
| `kbox wait [OPTIONS] <BOXES>...` | 等待一个或多个 Box 停止 |
| `kbox rename <BOX> <NEW_NAME>` | 重命名 Box |
| `kbox port <BOX>` | 查看端口映射 |
| `kbox port-forward [OPTIONS] --host-port <HOST_PORT> --guest-port <GUEST_PORT> <BOX>` | 将宿主回环 TCP 端口转发到 Box |
| `kbox shell [OPTIONS] <BOX>` | 在运行中的 Box 内打开交互 Shell |
| `kbox scale-api [OPTIONS]` | 提供面向 Gateway 的扩缩容接口 |

## 8. TEE、审计与安全

| 二级命令 | 说明 |
| --- | --- |
| `kbox attest [OPTIONS] <BOX>` | 获取并验证 TEE 证明报告 |
| `kbox audit [OPTIONS]` | 查询审计日志 |
| `kbox seal [OPTIONS] --data <DATA> <BOX>` | 使用 TEE 身份加密数据 |
| `kbox unseal [OPTIONS] --blob <BLOB> <BOX>` | 在 TEE 内解密数据 |
| `kbox inject-secret [OPTIONS] <BOX>` | 通过 RA-TLS 向运行中的 TEE Box 注入 Secret |

## 9. 文件系统与镜像

| 二级命令 | 说明 |
| --- | --- |
| `kbox export --output <OUTPUT> <NAME>` | 将 Box 文件系统导出为 tar 包 |
| `kbox commit [OPTIONS] <NAME> [REPOSITORY]` | 将 Box 修改提交为镜像 |
| `kbox diff <NAME>` | 查看 Box 文件系统变化 |
| `kbox events [OPTIONS]` | 查看实时系统事件 |
| `kbox container-update [OPTIONS] <NAME>` | 更新 Box 资源限制 |
| `kbox build [OPTIONS] [PATH]` | 从 Dockerfile 或 Containerfile 构建镜像 |
| `kbox images [OPTIONS]` | 列出缓存镜像 |
| `kbox pull [OPTIONS] <IMAGE>` | 从镜像仓库拉取镜像 |
| `kbox push [OPTIONS] <IMAGE>` | 将镜像推送到仓库 |
| `kbox login [OPTIONS] [SERVER]` | 登录镜像仓库 |
| `kbox logout [SERVER]` | 退出镜像仓库 |
| `kbox rmi [OPTIONS] <IMAGES>...` | 删除一个或多个缓存镜像 |
| `kbox image-inspect <IMAGE>` | 以 JSON 查看镜像详情 |
| `kbox history [OPTIONS] <IMAGE>` | 查看镜像层历史 |
| `kbox image-prune [OPTIONS]` | 清理未使用镜像 |
| `kbox tag <SOURCE> <TARGET>` | 为现有镜像创建新标签 |
| `kbox save --output <OUTPUT> <IMAGE>` | 将镜像保存为 tar 包 |
| `kbox load [OPTIONS] --input <INPUT>` | 从 tar 包导入镜像 |
| `kbox import [OPTIONS] <FILE> [REFERENCE]` | 将 rootfs tar 包导入为单层镜像 |
| `kbox cp <SRC> <DST>` | 在宿主与运行中的 Box 间复制文件 |

## 10. `kbox compose`

```bash
kbox compose [OPTIONS] <COMMAND>
```

公共参数：

```text
-f, --file <FILE>                  Compose 文件路径
-p, --project-name <PROJECT_NAME>  Compose 项目名称
```

### 8.1 三级命令

| 三级命令 | 说明 |
| --- | --- |
| `kbox compose up [OPTIONS] [SERVICE]...` | 创建并启动全部或指定服务。支持 `-d`、`--timeout`、`--isolation sandbox` |
| `kbox compose down [OPTIONS]` | 停止并删除全部服务。`--volumes` 同时删除命名卷 |
| `kbox compose ps [SERVICE]...` | 列出服务及状态 |
| `kbox compose config` | 校验并显示 Compose 配置 |
| `kbox compose logs [OPTIONS] [SERVICE]...` | 查看服务日志。支持 `-f`、`--tail` |
| `kbox compose start [SERVICE]...` | 启动已有服务 Box |
| `kbox compose stop [OPTIONS] [SERVICE]...` | 停止服务 Box，但不删除。支持 `-t` |
| `kbox compose restart [OPTIONS] [SERVICE]...` | 重启服务 Box。支持 `-t` |
| `kbox compose rm [OPTIONS] [SERVICE]...` | 删除已停止服务 Box。支持 `-s`、`-f` |
| `kbox compose kill [OPTIONS] [SERVICE]...` | 向服务 Box 发送终止信号。支持 `-s` |
| `kbox compose pause [SERVICE]...` | 暂停运行中的服务 Box |
| `kbox compose unpause [SERVICE]...` | 恢复已暂停的服务 Box |
| `kbox compose wait [OPTIONS] [SERVICE]...` | 等待服务停止。支持心跳间隔和总超时 |
| `kbox compose exec [OPTIONS] <SERVICE> -- <COMMAND>...` | 在指定服务内执行命令。支持环境变量、工作目录、TTY、用户和请求 ID |
| `kbox compose top [SERVICE]...` | 查看服务内运行进程 |
| `kbox compose port <SERVICE> [PRIVATE_PORT]` | 查看服务发布端口 |
| `kbox compose cp <SRC> <DST>` | 在服务 Box 与宿主间复制文件 |
| `kbox compose images [SERVICE]...` | 列出服务声明的镜像 |
| `kbox compose pull [OPTIONS] [SERVICE]...` | 拉取服务镜像。支持 `-q`、`--platform` |
| `kbox compose ls [OPTIONS]` | 列出 Compose 项目。`-q` 仅输出项目名 |
| `kbox compose volumes` | 列出项目声明的命名卷 |

## 11. `kbox snapshot`

```bash
kbox snapshot <COMMAND>
```

### 9.1 三级命令

| 三级命令 | 说明 |
| --- | --- |
| `kbox snapshot create [OPTIONS] <BOX_ID>` | 从已停止 Box 或受管 Sandbox 创建快照。支持 `--name`、`--description` |
| `kbox snapshot restore [OPTIONS] <SNAPSHOT>` | 从快照恢复 Box。支持 `--name` |
| `kbox snapshot ls [OPTIONS]` | 列出全部快照。支持 `--json` |
| `kbox snapshot rm [OPTIONS] <IDS>...` | 删除一个或多个快照。`-f` 强制解除 copy-on-write 引用 |
| `kbox snapshot inspect <ID>` | 查看快照详情 |
| `kbox snapshot prune [OPTIONS]` | 清理旧快照。支持 `--keep`、`--max-bytes` |

## 12. `kbox network`

```bash
kbox network <COMMAND>
```

### 10.1 三级命令

| 三级命令 | 说明 |
| --- | --- |
| `kbox network create [OPTIONS] <NAME>` | 创建网络。支持 `--subnet`、`--driver`、`--isolation`、`--egress`、`--label` |
| `kbox network ls [OPTIONS]` | 列出网络。`-q` 仅输出名称 |
| `kbox network rm [OPTIONS] [NAMES]...` | 删除网络。`-f` 先断开所有端点 |
| `kbox network inspect <NAME>` | 查看网络详情 |
| `kbox network connect <NETWORK> <CONTAINER>` | 将 Box 接入网络 |
| `kbox network disconnect [OPTIONS] <NETWORK> <CONTAINER>` | 将 Box 从网络断开。支持 `-f` |
| `kbox network prune [OPTIONS]` | 删除未使用网络。`-f` 跳过确认 |

## 13. `kbox volume`

```bash
kbox volume <COMMAND>
```

### 11.1 三级命令

| 三级命令 | 说明 |
| --- | --- |
| `kbox volume create [OPTIONS] <NAME>` | 创建命名卷。支持 `--driver`、`--label` |
| `kbox volume ls [OPTIONS]` | 列出卷。`-q` 仅输出名称 |
| `kbox volume rm [OPTIONS] [NAMES]...` | 删除一个或多个卷。`-f` 强制删除 |
| `kbox volume inspect <NAME>` | 查看卷详情 |
| `kbox volume prune [OPTIONS]` | 删除未使用卷。`-f` 跳过确认 |

## 14. `kbox pool`

```bash
kbox pool <COMMAND>
```

### 12.1 三级命令

| 三级命令 | 说明 |
| --- | --- |
| `kbox pool start [OPTIONS]` | 启动预热池守护进程。支持 `--image`、`--size`、`--max`、`--boot-concurrency`、`--ttl`、`--lease-ttl`、`--socket`、`--warm`、`--deferred`、`--ksm`、`--snapshot-fork`、`--metrics-addr`、`--json` |
| `kbox pool run [OPTIONS] -- <CMD>...` | 在预热 Sandbox 中执行命令。支持 `--socket`、`--image`、用户、工作目录、环境变量、挂载、CPU、内存和 `--exec` |
| `kbox pool stop [OPTIONS]` | 排空并停止预热池。支持 `--socket`、`--json` |
| `kbox pool status [OPTIONS]` | 查看预热池统计。支持 `--socket`、`--json` |

## 15. 系统与维护

| 二级命令 | 说明 |
| --- | --- |
| `kbox df [OPTIONS]` | 查看磁盘使用。`-v` 显示逐项占用 |
| `kbox prune [OPTIONS]` | 删除全部已停止 Box |
| `kbox system-prune [OPTIONS]` | 删除已停止 Box 和未使用镜像 |
| `kbox version` | 查看版本信息 |
| `kbox info` | 查看系统信息 |
| `kbox monitor [OPTIONS]` | 启动后台监控并重启异常 Box |

## 16. 工程文档

- [Agent Runtime 架构](docs/architecture/agent-paas-runtime.md)
- [kbox Box 优化与 Agent Runtime 开发计划](docs/kbox-smolvm-inspired-development-plan.md)
- [kbox CLI 端到端测试计划](docs/kbox-cli-e2e-test-plan.md)
- [多 Harness 协作架构](docs/architecture/harness-gateway.md)
- [沙箱、观测与干预架构](docs/architecture/sandbox-control.md)
- [开发标准](docs/development-standards.md)
- [Spec-Driven Development](docs/spec-driven-development.md)
- [Speckit 宪法](.specify/memory/constitution.md)

## 17. 许可

见 [LICENSE](LICENSE)。第三方组件保留各自许可和源码说明。

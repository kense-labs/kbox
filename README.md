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
  -u root \
  -v dsh-acp-data:/data \
  -v "$PWD:/workspace" \
  -e DEEPSEEK_API_KEY \
  ghcr.io/kense-labs/dsh-acp:latest
```

也可以直接运行 dsh-acp 仓库的问候示例（启动 VM → 发送问候 → 打印模型回复）：

```sh
python3 examples/acp_hello.py   # 见 https://github.com/kense-labs/dsh-acp
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

### 10.1 三级命令

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

### 11.1 三级命令

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

### 12.1 三级命令

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

### 13.1 三级命令

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

### 14.1 三级命令

| 三级命令 | 说明 |
| --- | --- |
| `kbox pool start [OPTIONS]` | 启动预热池守护进程。支持 `--image`、`--size`、`--max`、`--ttl`、`--socket`、`--deferred`、`--prewarm` 等选项；`--prewarm` 支持 `-e/--env`、`-u/--user`、`-w/--workdir`、`-v/--volume` |
| `kbox pool attach [OPTIONS]` | 接管一个已初始化的进程，双向转发 stdin/stdout，并返回应用真实退出码；要求守护进程使用 `--prewarm` |
| `kbox pool run [OPTIONS] -- <CMD>...` | 在预热 Sandbox 中执行一次性命令。支持 `--socket`、`--image`、用户、工作目录、环境变量、挂载、CPU、内存和 `--exec` |
| `kbox pool stop [OPTIONS]` | 排空并停止预热池。支持 `--socket`、`--json` |
| `kbox pool status [OPTIONS]` | 查看预热池统计。支持 `--socket`、`--json`；进程预热模式下 `idle >= 1` 表示已有 ready 的 VM 和应用进程 |

### 14.2 示例：用进程预热加速 dsh-acp ACP 会话

进程预热由用户显式启用。以下命令启动一个独立 VM，预先运行镜像的 `ENTRYPOINT + CMD`，待 ACP 连接和模型能力解析完成后才标记 ready。`--ttl 0` 禁止空闲 VM 到期回收；在单独终端运行守护进程：

```sh
export DEEPSEEK_API_KEY=你的密钥

kbox pool start --deferred --prewarm \
  --image ghcr.io/kense-labs/dsh-acp:latest \
  --size 1 --max 2 --ttl 0 \
  -u root \
  -v dsh-acp-data:/data \
  -v "$PWD:/workspace" \
  -e DEEPSEEK_API_KEY
```

等待 `pool status --json` 显示 `idle >= 1`，然后将 TUI/GUI 的 ACP agent 命令配置为 `kbox pool attach`。也可以直接在终端运行：

```sh
kbox pool attach
```

每次 `attach` 消耗一个独立的 VM＋进程；客户端断开后销毁该 VM，守护进程自动补充新的预热实例。一个进程不能跨客户端会话复用。未命中空闲实例时会等待新实例完成启动和预热。`pool run` 仍用于一次性命令，不转发交互式 stdin；不需要后台池时，使用 `kbox run -i --rm`。

以下完整命令只需要 Bash、`jq` 和已安装的 kbox。它使用独立的临时 pool socket，完成真实 ACP `initialize → session/new → session/set_config_option → session/prompt`，打印模型回复并验证 `stopReason` 和退出码，最后清理临时池，不影响已有默认池：

```bash
DEEPSEEK_API_KEY=你的密钥 \
DSH_MODEL=deepseek-flash \
DSH_REASONING_EFFORT=low \
bash <<'BASH'
set -euo pipefail

KBOX="${KBOX:-kbox}"
IMAGE="ghcr.io/kense-labs/dsh-acp:latest"
DSH_PROVIDER="${DSH_PROVIDER:-deepseek-official}"
DSH_MODEL="${DSH_MODEL:-deepseek-flash}"
DSH_REASONING_EFFORT="${DSH_REASONING_EFFORT:-low}"
GREETING="${GREETING:-你好！请用一句话介绍你自己。}"
: "${DEEPSEEK_API_KEY:?Set DEEPSEEK_API_KEY first}"
command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }
case "$DSH_MODEL" in
  */*) DSH_PROVIDER="${DSH_MODEL%%/*}"; DSH_MODEL="${DSH_MODEL#*/}" ;;
esac

WORK="$(mktemp -d "${TMPDIR:-/tmp}/dsh-acp-hello.XXXXXX")"
POOL_SOCKET="$WORK/pool.sock"
mkfifo "$WORK/acp.in" "$WORK/acp.out"
POOL_PID=""
ACP_PID=""
cleanup() {
  set +e
  exec 3>&- 2>/dev/null
  exec 4>&- 2>/dev/null
  [[ -z "$ACP_PID" ]] || kill "$ACP_PID" 2>/dev/null
  [[ -z "$ACP_PID" ]] || wait "$ACP_PID" 2>/dev/null
  "$KBOX" pool stop --socket "$POOL_SOCKET" >/dev/null 2>&1
  [[ -z "$POOL_PID" ]] || kill "$POOL_PID" 2>/dev/null
  [[ -z "$POOL_PID" ]] || wait "$POOL_PID" 2>/dev/null
  rm -rf -- "$WORK"
}
trap cleanup EXIT

"$KBOX" pool start --deferred --prewarm \
  --socket "$POOL_SOCKET" --image "$IMAGE" \
  --size 1 --max 1 -u root -e DEEPSEEK_API_KEY \
  >"$WORK/pool.log" 2>&1 &
POOL_PID=$!

READY=0
for ((attempt = 0; attempt < 90; attempt++)); do
  if "$KBOX" pool status --socket "$POOL_SOCKET" --json 2>/dev/null |
    jq -e '.[0].idle >= 1' >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 2
done
[[ "$READY" == 1 ]] || { echo "dsh-acp did not become ready" >&2; exit 1; }

exec 3<>"$WORK/acp.in"
exec 4<>"$WORK/acp.out"
"$KBOX" pool attach --socket "$POOL_SOCKET" <"$WORK/acp.in" >"$WORK/acp.out" 3>&- 4>&- &
ACP_PID=$!

RPC_RESPONSE=""
REPLY=""
rpc() {
  local id="$1" payload="$2" timeout="$3" line chunk
  printf '%s\n' "$payload" >&3
  while IFS= read -r -t "$timeout" -u 4 line; do
    [[ "$line" == \{* ]] || continue
    if jq -e --argjson id "$id" '.id? == $id' >/dev/null 2>&1 <<<"$line"; then
      if jq -e 'has("error")' >/dev/null 2>&1 <<<"$line"; then
        jq -r '.error.message // "ACP request failed"' >&2 <<<"$line"
        return 1
      fi
      RPC_RESPONSE="$line"
      return 0
    fi
    chunk="$(jq -rj 'select(.method? == "session/update") |
      .params.update | select(.sessionUpdate == "agent_message_chunk") |
      .content | select(.type == "text") | .text // empty' <<<"$line")"
    [[ -z "$chunk" ]] || { printf '%s' "$chunk"; REPLY="$REPLY$chunk"; }
  done
  echo "ACP request $id timed out or disconnected" >&2
  return 1
}

rpc 1 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}' 30
AGENT="$(jq -r '.result.agentInfo.name' <<<"$RPC_RESPONSE")"
rpc 2 "$(jq -nc '{jsonrpc:"2.0",id:2,method:"session/new",params:{cwd:"/workspace",mcpServers:[]}}')" 30
SESSION_ID="$(jq -r '.result.sessionId' <<<"$RPC_RESPONSE")"
MODEL_VALUE="$(jq -nc --arg provider "$DSH_PROVIDER" --arg model "$DSH_MODEL" '[$provider,$model]')"
rpc 3 "$(jq -nc --arg session "$SESSION_ID" --arg value "$MODEL_VALUE" \
  '{jsonrpc:"2.0",id:3,method:"session/set_config_option",params:{sessionId:$session,configId:"model",value:$value}}')" 30
jq -e --arg value "$MODEL_VALUE" \
  '[.result.configOptions[] | select(.id == "model" and .currentValue == $value)] | length == 1' \
  >/dev/null <<<"$RPC_RESPONSE"
rpc 4 "$(jq -nc --arg session "$SESSION_ID" --arg value "$DSH_REASONING_EFFORT" \
  '{jsonrpc:"2.0",id:4,method:"session/set_config_option",params:{sessionId:$session,configId:"reasoning_effort",value:$value}}')" 30
jq -e --arg value "$DSH_REASONING_EFFORT" \
  '[.result.configOptions[] | select(.id == "reasoning_effort" and .currentValue == $value)] | length == 1' \
  >/dev/null <<<"$RPC_RESPONSE"

printf 'Connected: %s\nModel: %s/%s (reasoning: %s)\nYou: %s\n%s: ' \
  "$AGENT" "$DSH_PROVIDER" "$DSH_MODEL" "$DSH_REASONING_EFFORT" "$GREETING" "$AGENT"
rpc 5 "$(jq -nc --arg session "$SESSION_ID" --arg text "$GREETING" \
  '{jsonrpc:"2.0",id:5,method:"session/prompt",params:{sessionId:$session,prompt:[{type:"text",text:$text}]}}')" 240
STOP_REASON="$(jq -r '.result.stopReason' <<<"$RPC_RESPONSE")"
[[ -n "$REPLY" && "$STOP_REASON" == "end_turn" ]] || { echo "invalid model response" >&2; exit 1; }
printf '\n(stop: %s)\n' "$STOP_REASON"
exec 3>&-
wait "$ACP_PID"
printf 'ACP exit code: 0\n'
BASH
```

`DSH_MODEL` 支持模型 ID 或 `provider/model`，`DSH_REASONING_EFFORT` 使用所选模型公布的值；`GREETING` 可覆盖默认中文问候。下方基准的原始样本分别记录 CLI 启动到 initialize 响应、initialize RTT 和模型 prompt RTT，模型网络耗时不计入启动加速。

2026-11-06 本机实测：macOS arm64、kbox 3.2.7、`deepseek-flash`＋`low`，5 次真实 DeepSeek API 会话均返回文本、`end_turn` 和退出码 0。镜像 digest 为 `sha256:4237b8ac59d8ad1fb89ce065877fdd8d02f9114c17412cca8163d4276242fea5`。

| 指标 | 最小值 | 中位数 | 最大值 |
| --- | ---: | ---: | ---: |
| 完整 CLI 启动到 initialize 响应 | 15.066 ms | 19.368 ms | 28.955 ms |
| initialize 请求 RTT | 13.822 ms | 17.642 ms | 27.502 ms |
| 守护进程内部接管（不含 CLI 与协议） | 0.222 ms | 0.251 ms | 0.313 ms |
| 模型 prompt RTT（网络请求，非启动指标） | 676.883 ms | 1436.499 ms | 1676.068 ms |

守护进程内部接管低于 1 ms，不代表完整 CLI 启动或 initialize 为 1 ms。原始样本与验证边界见[实测证据](docs/evidence/box/20261106-process-prewarm-attach/OUTCOME.md)。

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

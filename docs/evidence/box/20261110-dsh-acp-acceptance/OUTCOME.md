# dsh-acp × kbox：复杂、长程、多轮和外部文件系统 E2E

## Outcome

**真实 E2E：25 PASS / 0 FAIL / 0 SKIP / 0 BLOCKED。**

- [基础回归](../20261110-dsh-acp-baseline/results.json)：P0–P3，共 16 项，进程退出码 0。
- [复杂任务验收](results.json)：P4，共 9 项，进程退出码 0。
- [合并结果](all-results.json)：25 个不同用例 ID，无重复计数。
- [测试器自检](harness-selfcheck.log)：10 项通过；这些 mock/无密钥检查不计入真实 E2E。

验收使用真实 DeepSeek 模型、真实 macOS HVF MicroVM、实际 ACP stdio、宿主挂载及命名卷。文件结果由宿主独立计算和读取校验；能力声明、模型自述和卷“存在”不作为任务完成证据。

## Environment and provenance

| 项目 | 实际值 |
| --- | --- |
| 宿主 | macOS 26.6.2 / arm64 / HVF |
| kbox 基线 | `0304123f562ce7264954a8a104dced9e19b3d483`，加本次运行状态输出改为 stderr 的工作区修改 |
| 被测二进制 | 本地 `cargo build --release --locked -p kbox-cli -p kbox-shim` 产物，版本 3.4.3 |
| guest init | 已安装 3.4.3 包中的静态 Linux arm64 ELF，复制到本地 release 目录 |
| dsh-acp 源码 | `git@github.com:kense-labs/dsh-acp.git`，`ba01b05988c3cd0c982ca71069efd242f813d6c4`，工作区干净 |
| 镜像 | 从上述源码的 `Containerfile` 实际构建，`dsh-acp:e2e-ba01b05`，linux/arm64 |
| 镜像 manifest digest | `sha256:5d8c283ef8cc74d4f6b76f1bbb585cb0758f26eb7a898201bc65502b9e0c6f7b` |
| 模型配置 | `deepseek-official` / `deepseek-flash` / reasoning `low`，ACP 设置并回读 |
| 凭据 | 从工作区外层现有 `config.kcl` 的确切 DeepSeek provider 读取，仅注入子进程环境，不在命令或证据中回显 |
| 测试隔离 | 每组独立 K_HOME、socket、生成的工作区和卷；只共享镜像存储；不使用开发者默认 pool |

二进制和被测源码文件的 SHA-256 见 [environment.json](environment.json)。镜像 digest 由真实 `kbox image-inspect` 取得。

[凭据扫描](secret-scan.json)检查了 414 个证据和临时运行状态文件；一个早期失败快照已按实际密钥值脱敏，最终无凭据字面值命中。没有提交或发布含凭据的文件。

## Task results

| 用例 | 结果及证据 |
| --- | --- |
| P4-1 复杂编码 | 真实实现 `ledger.mjs`、`reconcile.mjs` 和 `test-ledger.mjs`；10 次工具调用，101.785 秒。宿主随机生成订单，独立计算去重、借贷、账户排序和整数金额；另经 `kbox exec` 运行未给模型的留出数据并核对。宿主重新运行的 [Node 测试](p4-tasks/guest-node-tests.log) 为 13 PASS / 0 FAIL。 |
| P4-2 多轮变更 | 同一会话连续 6 轮，每轮均真实使用工具，合计 30 次。覆盖宿主更新同一输入、过滤要求、失败恢复、畸形输入不覆盖已有输出、会话口令记忆。[逐轮指标](p4-tasks/turns.json)。 |
| P4-3 长程执行 | worker 写出启动标记后，宿主才开始每 10 秒投递一批，共 12 批，第 125 秒才给 DONE。ACP prompt 保持在途至完成，实际耗时 197.592 秒，7 次工具调用；宿主逐批比对随机输入的汇总。 |
| P4-4 外部文件系统 | 独立 `/external` 宿主挂载读写、宿主更新可见、中文/空格路径、重命名、256 KiB 二进制逐字节相同；指向未挂载宿主路径的符号链接读取失败。`:ro` 请求在 macOS 启动前明确拒绝，详见下方限制。 |
| P4-5 跨进程恢复 | 冷启动进程正常 EOF 退出；新池化进程 list/resume 同一 session，配置保留，不用文件工具回忆旧口令，再运行原项目并通过宿主计算结果。冷/热客户端 PID 分别为 84328 / 85603。 |
| P4-6 并发隔离 | 同一连接中 A 的真实命令在运行时，B 可完成独立 prompt；两组随机口令不串线；取消 A 后 B 仍能继续。 |
| P4-7 取消与恢复 | 宿主观察到实际父子进程启动后发 cancel，28 毫秒返回 cancelled，已发布的工具调用均有终态更新。观察至子进程原定 45 秒写入时间之后，共 50 秒，无延迟文件；原会话随后成功写入恢复结果。 |
| P4-8 负面与权限 | 7 个错误/冲突请求返回预期 JSON-RPC 错误；真实拒绝一次性外部写权限，目标文件未出现。 |
| P4-9 清理 | 客户端正常 EOF 退出、pool drain 并退出、socket 消失，[box 列表](p4-tasks/boxes-after.json)和[卷列表](p4-tasks/volumes-after.json)均为空；额外宿主进程扫描确认[无本次构建的残留 shim](processes-after.json)。 |

完整协议记录：[cold.jsonl](p4-tasks/cold.jsonl)、[resumed.jsonl](p4-tasks/resumed.jsonl)。agent thought 仅保留出现记录，正文不写入证据。

## Commands and changes

源码镜像第一次使用默认 512 MB 构建 VM 时，Node 构建触发 OOM，退出 134；随后用 4 GB 的专用构建池成功完成。原始日志均保留：

- [初次 OOM](../20261110-dsh-acp-long-tasks/source-image-build.log)
- [4 GB 构建成功](../20261110-dsh-acp-long-tasks/source-image-build-pool.log)
- [kbox release 构建](../20261110-dsh-acp-long-tasks/kbox-release-build.log)

实际构建入口：

```sh
cargo build --locked -p kbox-cli -p kbox-shim
cargo build --release --locked -p kbox-cli -p kbox-shim
# shim 按 crates/box/shim/entitlements.plist 签名；使用安装包的 guest-init。
K_HOME="$E2E_BUILD_HOME" ./target/debug/kbox build \
  -f Containerfile -t dsh-acp:e2e-ba01b05 \
  --run-pool --run-pool-autostart \
  --run-pool-image node:24-bookworm-slim \
  --run-pool-socket /tmp/kbox-dsh-build-20261110.sock \
  --run-pool-memory 4g --run-pool-cpus 4 ../dsh-acp
```

真实测试按两组在独立 K_HOME 中运行，分别使用上述同一镜像存储；环境已设置 `DEEPSEEK_API_KEY`。复跑时，路径变量需指向新的隔离目录，并让各测试 K_HOME 的 `images` 指向构建时的同一镜像存储：

```sh
KBOX="$PWD/target/release/kbox" IMAGE=dsh-acp:e2e-ba01b05 \
DSH_MODEL=deepseek-flash DSH_REASONING_EFFORT=low \
TIER=p0,p1,p2,p3 E2E_OUT="$E2E_BASELINE_OUT" E2E_K_HOME="$E2E_BASELINE_HOME" \
BENCH_URL=https://raw.githubusercontent.com/kense-labs/dsh-acp/ba01b05988c3cd0c982ca71069efd242f813d6c4/scripts/bench-pool-dsh-acp.sh \
./scripts/e2e-dsh-acp/run.sh

KBOX="$PWD/target/release/kbox" IMAGE=dsh-acp:e2e-ba01b05 \
DSH_MODEL=deepseek-flash DSH_REASONING_EFFORT=low \
TIER=p4 E2E_OUT="$E2E_TASK_OUT" E2E_K_HOME="$E2E_TASK_HOME" \
./scripts/e2e-dsh-acp/run.sh
```

也可用 `TIER=all` 在一个新的独立环境顺序执行。镜像必须先存在于对应 K_HOME 的镜像存储中。旧结果目录会被拒绝复用，避免覆盖证据。

本次修复和测试加强：

- `run` 的创建、拉取、启动、停止和完成提示写入 stderr，避免污染 ACP stdout；严格客户端不再忽略杂行。
- 镜像检查使用真实 `image-inspect` 和 `Digest`；清理使用 `ps --all --format json` 的 `raw_status`。查询错误不再变成“零残留”。
- shell 客户端处理真实一次性权限请求，并区分服务端请求 ID 与响应 ID；P3-4 必须真实调用工具并完成，不能 SKIP。
- 池耗尽会实际发送 initialize，不能把第二个活着但已连接的进程直接认定为“阻塞”。
- 上游 bench 保持原始内容，通过 CLI 参数适配器指定本组镜像和私有 socket，不停止默认 pool。
- 文件读取失败和 P4 缺凭据均使相应门禁失败；关闭 FIFO 不再永久吞掉 stderr。

## Other verification

| 检查 | 结果 |
| --- | --- |
| `python3 scripts/e2e-dsh-acp/test_harness.py -v` | 10 PASS，包括权限 ID 碰撞、总超时、协议杂行、错误不伪装清理成功、缺密钥失败及旧证据防覆盖 |
| shell 语法、Python 编译、`git diff --check` | PASS |
| 两个修改 Rust 文件的 rustfmt | PASS，[记录](../20261110-dsh-acp-long-tasks/changed-rust-fmt.log) |
| `cargo clippy --locked -p kbox-cli --all-targets --no-deps` | PASS，存在仓库原有 warning，[记录](../20261110-dsh-acp-long-tasks/cargo-clippy.log) |
| `cargo test --locked -p kbox-cli --lib commands::run::tests::` | 58 PASS / 1 FAIL：[记录](../20261110-dsh-acp-long-tasks/cargo-run-tests.log) |
| `cargo fmt --all -- --check` | FAIL：基线中多处格式差异，[记录](../20261110-dsh-acp-long-tasks/cargo-fmt.log) |

失败的 Rust 测试是 `test_foreground_sandbox_trusts_provider_exit_zero`；测试本身及其被测退出码逻辑均未由本任务修改。本任务没有改动该断言，也没有格式化无关文件。因此 **E2E 门禁通过不表示全仓库检查全绿**。

早期失败和中断的运行保留在本目录的 `attempt-01-mount-contract/`、基础回归的 `attempt-01-bare-volume/`、以及 [早期诊断目录](../20261110-dsh-acp-long-tasks/)。修正前误生成的相对卷目录也已移入相应证据目录，不留在仓库根目录。

## Explicitly not claimed

- 仅验证本机 macOS arm64 / HVF；未验证 Linux KVM、Windows WHPX、跨架构和多租户压测。
- macOS 当前不支持宿主强制只读的 virtio-fs bind，`:ro` 被启动前拒绝。本报告不声称成功使用了只读挂载。
- `pool start -v` 当前接受宿主路径。测试使用 `volume inspect` 的 Mountpoint 复用真实命名卷；不声称 pool 可直接解析裸卷名。普通 `run -v NAME:/data` 的命名卷路径已实测。
- 客体内的 `workspace-write` shell 沙箱不可用时，默认执行会拒绝；本测试的可信 ACP 客户端只针对生成的测试数据批准一次性客体内工具执行。未证明客体内 shell 沙箱可用。
- 未验证小时/天级稳定性、达到模型上下文上限后的压缩、真实用户项目或未授权宿主目录。取消后的迟到写入观察窗口为 50 秒，随后正常销毁 VM。
- 未发布镜像、未修改安装中的 kbox、未提交或推送代码。

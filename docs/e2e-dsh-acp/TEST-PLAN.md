# dsh-acp × kbox 端到端测试计划

本计划验证 **dsh-acp** 经 **kbox MicroVM**（cold `run -i` 与 process-prewarm `pool attach`）的 ACP 功能是否可用。

编排入口：[`scripts/e2e-dsh-acp/run.sh`](../../scripts/e2e-dsh-acp/run.sh)。

## 1. 默认前提

| 项 | 值 |
| --- | --- |
| 宿主 | macOS arm64（HVF）；官方预编译 kbox 主门禁 |
| 镜像 | `ghcr.io/kense-labs/dsh-acp:latest`（结果必须记录 digest） |
| 密钥 | `DEEPSEEK_API_KEY`（证据中禁止写入明文） |
| 模型 | `DSH_MODEL` 默认 `deepseek-flash`；`DSH_REASONING_EFFORT` 默认 `low` |
| 隔离 | 每轮独立 `K_HOME`、独立 pool socket、唯一卷名；失败也清理 |

Linux KVM / Windows 不在本轮主门禁。无 secret 的 CI 只跑 `cargo test -p kbox-runtime pool::` 与本脚本的语法检查。

## 2. 被测路径

```text
ACP client ──stdio──► kbox pool attach ──► prewarmed MicroVM (dsh-acp)
                              │
                              └─ destroy + replenish

Cold: ACP client ──stdio──► kbox run -i --rm … dsh-acp
```

协议序列（核心）：`initialize` → `session/new`（`cwd=/workspace`）→ `session/set_config_option` → `session/prompt` → `stopReason=end_turn`。

问候基准脚本复用上游 [bench-pool-dsh-acp.sh](https://github.com/kense-labs/dsh-acp/blob/main/scripts/bench-pool-dsh-acp.sh)，不在 kbox 维护第二套协议客户端。

## 3. 分层用例

### P0 — 冒烟

| ID | 场景 | 断言 |
| --- | --- | --- |
| P0-1 | 上游 bench `RUNS=1`（隔离 `K_HOME`） | 池 ready、`idle>=1`、回复非空、`end_turn`、exit 0、池清理 |
| P0-2 | Cold `kbox run -i --rm` + ACP `initialize` | JSON-RPC initialize 成功；EOF 后无残留 running box |

### P1 — ACP 核心 × kbox 承载

| ID | 场景 | 断言 |
| --- | --- | --- |
| P1-1 | `initialize` | 返回协议版本与 agent 信息；可重复不崩溃 |
| P1-2 | `session/new` + `cwd=/workspace` | 有效 `sessionId` |
| P1-3 | `set_config_option` 模型 / reasoning | 回读与设置一致 |
| P1-4 | `session/prompt` 中文问候 | 非空文本、`end_turn` |
| P1-5 | 连续 3 次 `pool attach` | 三次均成功；每次后池可 replenish |
| P1-6 | Prewarm 就绪契约 | 仅 `idle>=1` 后 attach；无半初始化失败 |

### P2 — 挂载、数据与清理

| ID | 场景 | 断言 |
| --- | --- | --- |
| P2-1 | `-v $CASE_DIR:/workspace` | prompt 必须读出宿主写入的 `marker.txt` 内容 |
| P2-2 | 命名卷 `/data` | 卷可创建；跨进程内容持久性由 P4-5 单独断言 |
| P2-3 | 无 `DEEPSEEK_API_KEY` | LLM 路径 fail-closed（prompt/session 不得静默成功；pool 仍可 prewarm） |
| P2-4 | `pool stop` / 异常退出 | 无 running box；临时 socket 删除 |

### P3 — 取消、并发与负面

| ID | 场景 | 断言 |
| --- | --- | --- |
| P3-1 | 客户端断开 / cancel | 进程退出；池补位；无僵尸 VM |
| P3-2 | `size=1` 双 attach | 第二请求阻塞或明确错误；释放后可再 attach |
| P3-3 | 错误镜像 / 无 ready | 超时内不标 idle；错误可观测 |
| P3-4 | 真实工具调用 | 必须有 `tool_call` 和 completed `tool_call_update`，实际 Node 运算返回正确结果 |

### P4 — 复杂、长程、多轮任务（真实模型，强制门禁）

实现：[`p4_tasks.py`](../../scripts/e2e-dsh-acp/cases/p4_tasks.py)。客户端只使用 Python 标准库；接收服务端权限请求、关联并发响应，并使用不会被流式通知重置的请求总期限。

| ID | 场景 | 必须成立的事实 |
| --- | --- | --- |
| P4-1 | Cold MicroVM 中实现多文件 Node 账本 CLI | 实现、模块、测试均存在；宿主独立计算随机订单结果；宿主经 `kbox exec` 重跑测试和未给模型的留出数据 |
| P4-2 | 同一会话连续六轮 | 每轮真实调用工具；至少 12 次工具调用；宿主更新输入、增加过滤要求、验证畸形输入不破坏已有输出；记忆口令不重新放入提示 |
| P4-3 | 125 秒持续到达的外部批次 | 每 10 秒写入一批，12 批；第 125 秒才写 DONE；ACP prompt 必须保持在途直至处理完毕，结果逐批由宿主校验 |
| P4-4 | 主工作区以外的宿主文件系统 | 独立 `/external` 挂载双向读写、宿主更新可见、中文与空格路径、重命名、256 KiB 二进制逐字节一致；指向未挂载宿主路径的符号链接不能读出数据 |
| P4-5 | 命名 `/data` 卷，跨进程与 MicroVM 恢复 | 冷启动进程正常 EOF 退出，新池化进程 list/resume 原 session，配置保留；不用文件工具回忆原口令，再执行原项目 |
| P4-6 | 同一 ACP 连接的并发会话 | A 的真实命令在运行时 B 仍可完成 prompt；各自口令不串线；取消 A 不影响 B |
| P4-7 | 取消真实父子进程并继续任务 | 在宿主看到启动标记后 cancel；15 秒内返回 cancelled；观察至子进程原定写入时间之后，无迟到文件；同一会话可继续写入 |
| P4-8 | 协议和权限负面场景 | 相对 cwd、错误 cwd 恢复、活跃会话重复恢复、同会话重入 prompt、未知 session/model/load 返回相应 JSON-RPC 错误；拒绝一次性外部写权限后无文件副作用 |
| P4-9 | 清理 | EOF 正常退出；池守护进程退出；socket 消失；正确 CLI 返回无 running/paused box；任务命名卷删除 |

只读挂载有单独的宿主契约检查：支持的平台必须在实际写入时拒绝；当前 macOS 后端因缺少宿主侧 virtio-fs 只读强制能力，会在启动前明确拒绝 `:ro`。后者作为“拒绝不安全挂载”记录，不能报告为“成功使用只读挂载”，也不会改成客体自行遵守只读。

dsh-acp 不提供 ACP 客户端文件系统接口。本组文件系统证据来自真正的 kbox 挂载和 agent 工具，不能用能力字段或模型自述代替。所有任务只挂载生成的测试数据。

`run -v NAME:/data` 支持命名卷解析；当前 `pool start -v` 的参数契约是宿主路径。P4-5 通过 `volume inspect NAME` 返回的 `Mountpoint` 将同一数据目录挂入池化 VM，不能声称池接口已支持直接解析裸卷名。

## 4. 门禁

| 时机 | 命令 |
| --- | --- |
| 本地开发（有 key） | `TIER=p0 ./scripts/e2e-dsh-acp/run.sh` |
| pool/prewarm/attach 改动 | `TIER=p0,p1 ./scripts/e2e-dsh-acp/run.sh` |
| 发版 | `TIER=all ./scripts/e2e-dsh-acp/run.sh` + 填写证据 |
| 复杂任务验收 | `TIER=p4 IMAGE=<固定版本镜像> ./scripts/e2e-dsh-acp/run.sh` |
| 无 secret CI | `python3 scripts/e2e-dsh-acp/test_harness.py -v` + shell 语法检查；这些自检不算真实 E2E |

## 5. 通过标准

- P0/P1 全部 PASS，证据含 kbox 版本与镜像 digest。
- P2 挂载与清理 PASS；密钥不出现在提交的证据中。
- P3 客户端断开、池耗尽和工具调用 PASS。
- P4 九项全部 PASS；缺密钥、跳过、前置失败都使门禁非零退出。重跑必须使用新的证据目录，避免旧文件误通过。
- 镜像 digest 来自实际 `kbox image-inspect`，清理来自 `ps --all --format json` 的 `raw_status`；查询失败不能被当成零残留。
- 失败保留 `pool status`、daemon 日志路径与 box 列表。

## 6. 证据

模板：[TEMPLATE-dsh-acp-e2e/OUTCOME.md](../evidence/box/TEMPLATE-dsh-acp-e2e/OUTCOME.md)。

实测目录建议：`docs/evidence/box/YYYYMMDD-dsh-acp-e2e/OUTCOME.md`，并附 `results.json`（由 runner 生成，可复制）。

# dsh-acp × kbox 端到端测试计划

本计划验证 **dsh-acp** 经 **kbox MicroVM**（cold `run -i` 与 process-prewarm `pool attach`）的 ACP 功能是否可用。

编排入口：[`scripts/e2e-dsh-acp/run.sh`](../../scripts/e2e-dsh-acp/run.sh)。

## 1. 默认前提

| 项 | 值 |
| --- | --- |
| 宿主 | macOS arm64（HVF）；官方预编译 kbox 主门禁 |
| 运行时 | kbox **3.4.3** 官方 release；不源码构建（3.4.3 依赖本机缺失的 `../kcl`） |
| 镜像 | `ghcr.io/kense-labs/dsh-acp:latest`（结果必须记录 digest） |
| 密钥 | `DEEPSEEK_API_KEY`（证据中禁止写入明文） |
| 模型 | `DSH_MODEL` 默认 `deepseek-flash`；`DSH_REASONING_EFFORT` 默认 `low` |
| 隔离 | 每轮独立 `K_HOME`、独立 pool socket、唯一卷名；失败也清理 |

Linux KVM / Windows 不在本轮主门禁。`port-forward`（非 Linux / 无 netns）、TEE/attest、`--isolation`（3.4.0 起被拒）均不 qualify。无 secret 的 CI 只跑 `cargo test -p kbox-runtime pool::` 与全部脚本的 `bash -n`。

## 2. 被测路径

```text
ACP client ──stdio──► kbox pool attach ──► prewarmed MicroVM (dsh-acp)
                              │
                              └─ destroy + replenish

Cold: ACP client ──stdio──► kbox run -i --rm … dsh-acp
```

协议序列（核心）：`initialize` → `session/new`（`cwd=/workspace`）→ `session/set_config_option` → `session/prompt` → `stopReason=end_turn`。P4+ 另覆盖 `session/list` / `resume` / `close` / `set_config_option`，`session/cancel`（**通知，无 id**）以及服务端发来的 `session/request_permission`（客户端必须即时应答，否则长连接会死锁；选项固定 `allow-once` / `reject-once`）。六种 `session/update`：`agent_message_chunk`、`agent_thought_chunk`、`tool_call`、`tool_call_update`、`usage_update`、`config_option_update`。

问候基准脚本复用上游 [bench-pool-dsh-acp.sh](https://github.com/kense-labs/dsh-acp/blob/main/scripts/bench-pool-dsh-acp.sh)，不在 kbox 维护第二套协议客户端。P0–P3 沿用 `lib/acp_session.sh` 里 id 1–5 的单轮函数；P4–P6 使用同一文件新增的长连接 trace API，未另起客户端。

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

### P4（可选）— dsh-acp 全能力面（L1 协议 + L2 承载）

本节是上一节强制任务门禁之外的补充能力面，**默认不运行**：用 `E2E_CAPABILITY_SURFACE=1 TIER=p4` 显式开启，开启后其 P4-01..10 行同样受强制门禁约束。脚本：[`cases/p4_capabilities.sh`](../../scripts/e2e-dsh-acp/cases/p4_capabilities.sh)。与 P0–P3 不同，P4+ 使用 `lib/acp_session.sh` 的**长连接 trace API**（自增 id、可配置 fd、记录全部 `session/update` 类型与顺序）和 `lib/e2e_fixture.sh`（pool / FIFO / 卷 / 清理）。旧函数与 P0–P3 保持不变。

| ID | 场景 | 断言 |
| --- | --- | --- |
| P4-01 | `initialize`（含重复调用）、能力声明 | `agentInfo` / `protocolVersion`、`mcpCapabilities` / `promptCapabilities` / `sessionCapabilities`、stdout 只有 JSON-RPC（`noise=0`） |
| P4-02 | `new` → `list` → `close` → `resume` 全链 | active 会话被 `list` 过滤；重复 resume / cwd 不符 / 不存在 id 均 fail-closed；`nextCursor` 翻页无重复（未分页时记录 `not-triggered`） |
| P4-03 | `set_config_option`：model、`reasoning_effort` | 值取自响应里实际 advertised 的 options（不硬编码枚举）；逐个回读一致；最后一次 effort 在 close+resume 后保持 |
| P4-04 | 六种 `session/update` + `stopReason` | 至少 `agent_message_chunk`；`tool_call` → `tool_call_update` 顺序；`usage_update` / `agent_thought_chunk` / `config_option_update` 出现即计入 trace，不出现不谎报；无工具时正式 SKIP |
| P4-05 | 内容准入 | audio / embedded resource / 非法 image 三种非法块必须被拒；`image` 能力开启才发内联 1×1 PNG；拒绝后连接仍可用。超大 prompt 默认不跑（`E2E_P4_HUGE_PROMPT=1` 才消耗 token） |
| P4-06 | cold `run -i --rm` 与 `pool attach` 双路径 | 同一组 initialize / new / prompt 结果一致、`end_turn`；cold VM 用独立 `/data` 卷避免与池竞争 |
| P4-07 | 命名卷 `/data` + `-v :/workspace` 跨 attach | 关闭会话、销毁连接后，新 attach 能 `list` 到该 sessionId 并 `resume`，宿主 marker 可被读回 |
| P4-08 | `size=1` 并发 attach / 池耗尽与补位 | 第二个 attach 阻塞或明确失败；保存 busy / after 两份 `pool status --json`；释放后回到 `idle>=1` |
| P4-09 | `pool stop` / 锁与残留 | socket 消失、`ps` 无 running box、`K_HOME` 下无 `lifecycle.lock`；snapshot 子项在 3.4.3 下无合格命令，不纳入 |
| P4-10 | `--memory 2g` 资源边界 | 该 flag 被接受时完成一次真实 prompt；CLI 不接受则正式 SKIP 并保留 pool 日志 |

### P5 — 长程 / 多轮（nightly，重点）

脚本：[`cases/p5_longrun.sh`](../../scripts/e2e-dsh-acp/cases/p5_longrun.sh)。**默认 SKIP**，必须显式 `E2E_LONGRUN=1`；单场景会消耗大量真实 token，不进 PR 门禁。

「长程」是可度量门槛，不是「多聊几句」：

| 门槛 | 默认值 | 环境变量 |
| --- | --- | --- |
| 轮数 | ≥ 30 | `LONGRUN_TURNS` |
| 墙钟 | ≥ 900 s | `LONGRUN_MIN_SECONDS` |
| 流式 chunk | ≥ 100 | `LONGRUN_MIN_CHUNKS` |
| 工具调用 | ≥ 50 | `LONGRUN_MIN_TOOL_CALLS` |

首次校准可设 `E2E_LONGRUN_ENFORCE=0`：只记录实测值、不因门槛失败。

| ID | 场景 | 断言 |
| --- | --- | --- |
| P5-01 | 30 轮工具型任务 + canary 保真 | 每轮用工具追加 `turn-N` 到日志，末轮复述 canary；宿主逐行核对无丢轮；每 5 轮切 effort；`usage_update.used` 单调不减；四项门槛达标 |
| P5-02 | 跨 VM 续跑 | VM A 多轮 + 写文件 → 销毁 VM A → 新 VM B 挂同一卷 → `list` → `resume` → 续跑；sessionId 不变、canary 与工作区文件仍在、无 `SessionAlreadyOwnedError`（证明排他锁已释放） |
| P5-03 | 取消与中断 | prompt 中途注入 `session/cancel` → `stopReason=cancelled` → 同会话继续 `end_turn`；另做客户端 SIGKILL 后池补位、无僵尸 VM |
| P5-04 | 长上下文 / 压缩 | 喂入 ~200 KB 文档触发上下文压力；`usage_update` 可读、历史在 close+resume 后可复述；无 token meter 时正式 SKIP |
| P5-05 | 慢客户端背压 | 每 chunk 人为延迟（`LONGRUN_SLOW_READ_MS`，默认 50 ms）；不丢 chunk、不死锁、`end_turn` 与结尾标记均到达、`noise=0` |

### P6 — 复杂 / 工具密集任务

脚本：[`cases/p6_complex.sh`](../../scripts/e2e-dsh-acp/cases/p6_complex.sh)。需要外部 fixture 的部分一律正式 SKIP 并写明要设置的环境变量；纯协议负面断言始终执行。

| ID | 场景 | 断言 |
| --- | --- | --- |
| P6-01 | 跨文件重构 | 宿主准备含 bug 的 `math.js` + `node:test`；`tool_call` → `tool_call_update` 有序、≥2 次工具调用、宿主 `node --test` 通过、禁用工具（默认 `apply_patch`）未出现 |
| P6-02 | 真实 MCP transport | 相对 command / 非 HTTP URL / 重复 name / 不支持的 transport 四种非法 `mcpServers` 必须被 `session/new` 拒绝；提供 `E2E_MCP_STDIO_COMMAND`（绝对路径）与 `E2E_MCP_HTTP_URL` 时验证工具往返与 resume 重挂 |
| P6-03 | 权限交错 | `allow-once` / `reject-once` / `cancelled` / `fail` 四种策略轮换；`permissionRequests == permissionAnswers`、决策后 chunk 继续流动；未触发 `approval/request` 时正式 SKIP |
| P6-04 | 子代理与编排 | 一条连接两个 live session；`list` 过滤 active；`cancel(sid2)` 不影响 `sid1`，`cancel(sid1)` 返回 `cancelled`；subagent resume 拒绝需 `E2E_P6_SUBA=1` 与 fork fixture |
| P6-05 | 网络 / 交互工具 | `web_fetch` / `web_search` / `ask_user_question` / `exit_plan_mode` 在 VM 内可达；一个都没触发时正式 SKIP 并列出实际 `toolTitles` |

### 追踪格式与状态语义

每个用例独立生成 `<label>.trace.jsonl`，一行一条收发消息，只存元数据（类型、顺序、toolCallId、title、status、used/size、stopReason、error），**不落盘 chunk 正文**。`acp_trace_stats` 聚合出 `byType` / `firstOrder` / `chunks` / `toolCalls` / `permissionRequests` / `toolTitles` 等；`acp_trace_check` 用 `{require_types, forbid_types, require_order, min, max}` 做断言，随 `results.json` 的 `detail` 一并留证。

- `BLOCKED`：缺 `DEEPSEEK_API_KEY`（runner 门禁视为可接受）。
- `SKIP`：nightly 未开启、外部 fixture 未注入、或能力未被观测到——必须写明原因，**不得用 SKIP 冒充覆盖**。
- `FAIL`：真实断言未通过；`results.json` 中的 FAIL 行使整体退出码非 0。

## 4. 门禁

| 时机 | 命令 |
| --- | --- |
| 无 secret CI / PR | `bash -n scripts/e2e-dsh-acp/run.sh scripts/e2e-dsh-acp/lib/*.sh scripts/e2e-dsh-acp/cases/*.sh` + `cargo test -p kbox-runtime pool::` + `python3 scripts/e2e-dsh-acp/test_harness.py -v`；这些自检不算真实 E2E。缺 `DEEPSEEK_API_KEY` 时 p4 强制门禁按设计失败（9 条 BLOCKED → 非零退出），无密钥环境不要跑 `TIER=p4` |
| 本地开发（有 key） | `TIER=p0 ./scripts/e2e-dsh-acp/run.sh` |
| pool/prewarm/attach 改动 | `TIER=p0,p1 ./scripts/e2e-dsh-acp/run.sh` |
| 复杂任务验收 | `TIER=p4 IMAGE=<固定版本镜像> ./scripts/e2e-dsh-acp/run.sh` |
| 能力面改动（可选） | `E2E_CAPABILITY_SURFACE=1 TIER=p4,p6 DEEPSEEK_API_KEY=… ./scripts/e2e-dsh-acp/run.sh` |
| 长程 nightly | `E2E_LONGRUN=1 TIER=p5 DEEPSEEK_API_KEY=… ./scripts/e2e-dsh-acp/run.sh` |
| 发版 | `E2E_LONGRUN=1 TIER=all DEEPSEEK_API_KEY=… ./scripts/e2e-dsh-acp/run.sh` + `python3 scripts/e2e-dsh-acp/test_harness.py -v` + 填写证据 |

## 5. 通过标准

- P0/P1 全部 PASS，证据含 kbox 版本与镜像 digest。
- P2 挂载与清理 PASS；密钥不出现在提交的证据中。
- P3 客户端断开、池耗尽和工具调用 PASS。
- P4 九项全部 PASS；缺密钥、跳过、前置失败都使门禁非零退出。重跑必须使用新的证据目录，避免旧文件误通过。
- 可选能力面（`E2E_CAPABILITY_SURFACE=1`）的 P4-01..10 协议负面断言与双路径 PASS；SKIP 项必须有原因（未分页、无 token meter、CLI 不支持等）。开启后这些 P4- 行同样进入强制门禁，不得用 SKIP/BLOCKED 冒充覆盖。
- 镜像 digest 来自实际 `kbox image-inspect`，清理来自 `ps --all --format json` 的 `raw_status`；查询失败不能被当成零残留。
- P5 四项长程门槛达标（`E2E_LONGRUN_ENFORCE=0` 仅用于首次校准，不得作为发版依据）；跨 VM resume 与 cancel 续跑必须 PASS。
- P6 至少一条真实工具链 PASS；MCP 四条非法声明必须被拒；外部 fixture 缺失的 SKIP 需写明所需环境变量。
- 任何 FAIL 行使 `results.json` 门禁失败；BLOCKED / SKIP 仅对 P5/P6 可接受且需在 OUTCOME 中逐条解释，不得用于 P4 强制覆盖。
- 失败保留 `pool status`、daemon 日志路径、box 列表与对应 `.trace.jsonl`。

## 6. 证据

模板：[TEMPLATE-dsh-acp-e2e/OUTCOME.md](../evidence/box/TEMPLATE-dsh-acp-e2e/OUTCOME.md)。

实测目录建议：`docs/evidence/box/YYYYMMDD-dsh-acp-e2e/OUTCOME.md`，并附 `results.json`（由 runner 生成，可复制）。P4–P6 另应保留 `fixture/*.trace.jsonl`、`p4-08-status-*.json`、`p6-01-node-test.log` 以及 `p6-03-<policy>.jsonl`。`results.json` 的每条 case 都带 `detail`（trace 统计、轮数、耗时、chunk / tool / token 数、sessionId 等），runner 顶层记录 `kbox` 版本、`image` 与 `image_digest`。

## 7. 已知待校准项

骨架按 dsh-acp 当前源码与 kbox 3.4.3 文档写成，首次 nightly 需在真实环境校准三处：① `session/list` 的 `sessionListPageSize` 默认值决定 P4-02 是否触发分页；② `pool start --memory` 是否被该构建接受（否则 P4-10 正式 SKIP）；③ 触发 `approval/request` 的 prompt 是否足够（P6-03 未触发时 SKIP）。三处都设计为「显式记录、不假 PASS」，不会因为环境差异静默通过。

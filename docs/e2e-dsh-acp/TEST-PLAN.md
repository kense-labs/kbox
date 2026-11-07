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
| P2-1 | `-v $CASE_DIR:/workspace` | prompt 读出宿主写入的 `marker.txt` 内容（或明确 SKIP） |
| P2-2 | 命名卷 `/data` | 卷可创建；跨 attach 持久性按 agent 能力 PASS/SKIP |
| P2-3 | 无 `DEEPSEEK_API_KEY` | LLM 路径 fail-closed（prompt/session 不得静默成功；pool 仍可 prewarm） |
| P2-4 | `pool stop` / 异常退出 | 无 running box；临时 socket 删除 |

### P3 — 取消、并发与负面

| ID | 场景 | 断言 |
| --- | --- | --- |
| P3-1 | 客户端断开 / cancel | 进程退出；池补位；无僵尸 VM |
| P3-2 | `size=1` 双 attach | 第二请求阻塞或明确错误；释放后可再 attach |
| P3-3 | 错误镜像 / 无 ready | 超时内不标 idle；错误可观测 |
| P3-4 | 工具/权限 | initialize 声明 tools 则最小往返；否则正式 SKIP |

## 4. 门禁

| 时机 | 命令 |
| --- | --- |
| 本地开发（有 key） | `TIER=p0 ./scripts/e2e-dsh-acp/run.sh` |
| pool/prewarm/attach 改动 | `TIER=p0,p1 ./scripts/e2e-dsh-acp/run.sh` |
| 发版 | `TIER=all ./scripts/e2e-dsh-acp/run.sh` + 填写证据 |
| 无 secret CI | `bash -n scripts/e2e-dsh-acp/run.sh` + `cargo test -p kbox-runtime pool::` |

## 5. 通过标准

- P0/P1 全部 PASS，证据含 kbox 版本与镜像 digest。
- P2 挂载与清理 PASS；密钥不出现在提交的证据中。
- P3 取消与池耗尽 PASS；工具项 PASS 或正式 SKIP（写明原因）。
- 失败保留 `pool status`、daemon 日志路径与 box 列表。

## 6. 证据

模板：[TEMPLATE-dsh-acp-e2e/OUTCOME.md](../evidence/box/TEMPLATE-dsh-acp-e2e/OUTCOME.md)。

实测目录建议：`docs/evidence/box/YYYYMMDD-dsh-acp-e2e/OUTCOME.md`，并附 `results.json`（由 runner 生成，可复制）。

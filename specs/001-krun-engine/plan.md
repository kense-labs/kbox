# 实施计划：KRun 引擎入口

负责路径：`crates/box/{core,runtime,cli}`。规格见 [spec](spec.md)，有序任务见 [tasks](tasks.md)。本计划没有创建分支或提交。

## Constitution Check

以下 PASS 表示设计满足约束，不表示实现或测试已经通过。

| 原则 | 规划检查 | 落点 |
| --- | --- | --- |
| I / V：拒绝与类型 | PASS | 版本／能力发现、稳定错误、边界大小限制；未知安全值拒绝 |
| II：单一资源权威 | PASS | ExecutionManager；ManagedRuntimeClient 回执；连接映射不控制第二份进程 |
| III / IV / XV：证据与门槛 | PASS | 各任务带故障与原生测试；证据保留，不修改失败判定 |
| VI / XII：仓库边界 | PASS | kbox 原 workspace 与 lockfile；独立 krun 经用户明确授权，通过协议集成 |
| VII：KCL | PASS | KCL 是产品配置；wire／清单只作数据合同 |
| VIII / XIV：能力与平台 | PASS | 流单独协商；不回退后端；原生矩阵逐项声明 |
| IX / X：凭据与效果 | PASS | 控制路径临时凭据；真实退出／清理观测优于记录和决策 |
| XI / XIII：判断与身份 | PASS | adversarial 用例；身份来自清单和精确存储值，不从包文件名猜测 |

## 复用与计划变更

复用依据见 [research](research.md)。不修改 Runtime unary Exec 的原义。

| 位置 | 计划变更 |
| --- | --- |
| `crates/box/cli/src/commands/engine.rs`（新增）及 `commands/mod.rs` | `kbox engine serve` 本地入口，发现与单实例管理，不输出业务协议到诊断通道 |
| `crates/box/runtime/src/engine/`（新增）及 `src/lib.rs` | 类型化入口、接收 revision、原回执接线、连接映射、恢复与资源清理 |
| `crates/box/core/src/traits/{execution,session}.rs` | 复用现有执行和会话边界；只有缺失的 fencing／能力接线才扩展，不复制接口 |
| `crates/box/runtime/src/local_execution/session.rs` | 复用 `start_process`；实现所需的连接／停止检查并覆盖实际副作用 |
| `crates/runtime/src/{managed.rs,state/}` | 复用现有持久回执；只有明确缺口时扩展，不增加平行状态机 |
| 现有 pool 客户端及 attach 测试 | 回归兼容；不直接对外承诺内部 wire 稳定，不要求 Harness 输出私有 ready marker |

本 epic 不实现 pool 到 MateHarness 的全部装配；接口必须能返回精确执行身份，后续租约桥接不得创建重复资源。

## 数据与失败行为

见 [数据模型](data-model.md)和[协议合同](contracts/engine-v1.md)。请求先持久接收，再调用现有执行权威；响应丢失使用相同 request ID 对账。stop 的 revision fence 在实际资源入口生效，防止旧启动晚到。

发现能力、解析配置和接受操作属于控制面。ACP、文件与终端回调是进程流内容，网关不解释推理或工具调用；MCP 会话绑定由单独协议层处理。

## 验证计划

- Hermetic：身份、请求冲突、并发提交、revision、重启、帧边界、半关闭、背压与唯一终态。
- 原生：macOS/arm64 + Linux/aarch64 MicroVM，精确 generation 上的真实进程和父子进程清理。
- 原有 unary／pool 测试保持原判定；不把流能力归入 `RuntimeFeature::Exec`。
- Rust 变更按根 AGENTS 执行 fmt、所属 package 的 focused tests 与 Clippy；跨 crate 时增加 workspace check。
- 证据目标：`docs/evidence/box/<date>-krun-engine/OUTCOME.md`，包含未运行项与未声明能力。

## 顺序

合同与拒绝测试 → 持久控制入口 → 流接线与断线清理 → 故障恢复 → 原生资格检查 → 独立客户端兼容检查。每一步实现与其测试同批落地。

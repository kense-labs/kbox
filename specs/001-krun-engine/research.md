# 复用依据与决定

代码基线：`0304123f562ce7264954a8a104dced9e19b3d483`；当前工作区另有先前任务修改，实施时必须保留。本文来自本地代码检查，不代表接口已经公开。

| 现有依据 | 结论 |
| --- | --- |
| `crates/runtime/src/managed.rs`、`driver.rs`、`state/` | 已有持久请求、provider 操作与观察归属；复用回执，不自建 VM 状态 |
| `crates/box/core/src/traits/session.rs`、`crates/box/runtime/src/local_execution/session.rs` | `start_process` 已有 generation 校验和双向输入／事件输出，不需从零实现 PTY 替代品 |
| `crates/box/runtime/src/pool/client.rs` | lease／exec 为内部协议，lease ID 本身不能冒充公开执行身份；大包超出单控制帧预算 |
| `crates/box/runtime/src/pool/attach.rs` | 已有输入 close、独立 stdout/stderr/exit、可取消读取的分帧与边界测试，可复用机制 |
| `crates/box/runtime/src/pool/prewarm.rs` | 当前进程预热要求私有 marker；KRun 采用环境预热，不向新 Harness 强加该合同 |
| `crates/runtime/docs/adr/0003-keep-interactive-streaming-exec-outside-v0.2-core.md` | unary Exec 不含交互流；新能力独立版本化并明确不可恢复 |
| `crates/runtime/docs/adr/0004-type-service-endpoints-and-protocol-capabilities.md` | 端点必须由后端真实发布并按 generation 管理，客户端不能虚构转发地址 |

采用本地版本化引擎门面，是因为独立 krun 不能依赖本机 workspace 或不稳定 pool wire。首版不增加远端引擎、第三方后端或流恢复；避免扩大实现范围。

前置实现需验证：现有 Runtime 请求持久存储与引擎 admission journal 的原请求恢复接线，以及 stop fence 到实际迟到创建的约束。这两项有强制任务与故障测试，不能假定已由网关锁解决。

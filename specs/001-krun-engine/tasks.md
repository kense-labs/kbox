# 任务：KRun 引擎入口

状态：E01–E09 均有部分实现；同用户控制、持久 admission、原始 primary 流、独立 Rust 客户端联通、原生故障及通用 OCI 回归见 [当前证据](../../docs/evidence/engine/20261111-primary-current/OUTCOME.md)。控制 API、冻结派发与 primary 请求身份已有实现；正式 CLI／恢复接线、Secret 与完整故障门槛仍未完成，没有阶段整体完成声明。每项任务包含调用方、必要配置、导出、测试和本批证据；不把部分证据勾选为整体通过。

- [ ] E01 / US1：在 `crates/box/runtime/src/engine/` 新增版本、能力、请求和错误类型；补输入大小、未知版本／必要能力及请求冲突测试，冻结 [wire 合同](contracts/engine-v1.md)。
- [ ] E02 / US1：在同模块实现持久 admission 与 `crates/runtime/src/managed.rs`／`state/` 回执接线；测试接收前后崩溃、响应丢失、原 ID 恢复与引擎 identity 变化，禁止重复 apply。
- [ ] E03 / US1：把 intent revision fence 接至 `crates/box/core/src/traits/execution.rs` 和实际 `local_execution` 副作用；测试新 stop 先到、旧 create 晚到和停止／替换竞态，证明无迟到活动资源。
- [ ] E04 / US1：新增 `crates/box/cli/src/commands/engine.rs` 并在 `commands/mod.rs` 注册；处理同一 home 的单实例、本地权限、Discover、查询、停止、日志；进程级测试 daemon 已存在、启动失败、不可达和其他调用方资源不可被接管。
- [ ] E05 / US2：复用 `crates/box/runtime/src/local_execution/session.rs` 的 `start_process`，在 `engine/` 接通独立流 capability；复用 `pool/attach.rs` 的成熟分帧行为，测试分片、截断帧、半关闭、独立 stderr 和完整尾部。
- [ ] E06 / US2：在流 session 模块实现队列预算、背压、独占连接与唯一退出结果；测试慢消费者、大输出、双连接、Open 响应丢失、重复请求与不重放输入。
- [ ] E07 / US1+US2：将断线／退出／停止接回原 ExecutionManager 清理；测试父子进程、崩溃恢复、Secret 引用失效、旧 generation、cleanup_pending 与 unknown，不提前报告成功。
- [ ] E08 / US1+US2：新增 `crates/box/runtime/tests/krun_engine_native.rs`，在 macOS/arm64 + Linux/aarch64 客体运行真实控制／流测试；独立客户端不依赖本机源码路径，记录执行身份和零遗留资源。
- [ ] E09：运行所属 package 的 fmt、focused tests、Clippy，跨 crate 变更运行 workspace check；回归既有 unary 与 pool 测试。在 `docs/evidence/box/<date>-krun-engine/OUTCOME.md` 保存实际命令、计数与未声明能力。

依赖：E01 → E02 → E03 → E04；E05 依赖 E01–E03；E06 依赖 E05；E07 依赖 E04、E06；E08、E09 是最终门槛。实现不以 mock 通过代替原生验证。

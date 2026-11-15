# 任务与产物

每个任务以 H2 标题记录稳定的 `T日期-slug` ID，正文写明完成日期、具体结果、可打开的实际产物及验证依据/限制。新完成项置前，历史保留；未知日期、产物或验证不得补造。

单人任务可自行登记；并行工作由指定的 D/O 或台账写入者汇总真实短报后，再发布最终完成报告。证据不足时保持部分完成/未验证，不列为完成。

每个任务的验收证据分路径保存，不覆盖旧报告。当前源码链接可更新，历史结论以回执或快照为准；`/k-wiki` 刷新本页入口并打包实际引用的产物。

## T2026-11-16-pool-postboot-cleanup

- **完成日期**:2026-11-16（有界修复，非总体完成）。
- **结果**: 默认 libkrun pool 在启动后 exec 复验失败时清理实际 VM；原始错误与超时保留。
- **产物/证据**: [源码、最终归档、原生结果和10-home清理](../../docs/evidence/runtime/20261116-pool-postboot-cleanup/OUTCOME.md)。
- **验证**: focused4、pool87；3 Rust native +3 Python场景及4-daemon/16-VM并发控制通过；默认门禁仍6项磁盘服务失败。
- **限制**: 原始15s启动超时、stop失败所有权边界、完整GA及外部平台门槛未完成；RSS未测量；tokens/cost unknown。

## T2026-11-16-k-init

- **完成日期**: 2026-11-16
- **结果**: create-missing 初始化 `.kstudio` + Cursor 发现路径；`AGENTS.md` 保留；冲突 0。`sync-k-skills --check` 与 `verify-ai-eng --level L3` 均 PASS。产品目标未澄清。
- **产物**: `.kstudio/`（含 skills、workflow/bootstrap、wiki、evidence）、`.cursor/{commands,rules,skills}/`
- **证据**: `.kstudio/evidence/T2026-11-16-k-init/OUTCOME.md`
- **限制**: 未跑 `cargo test`；未声明产品 Pass；skill picker 刷新未观测；usage/prices unknown

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

## T2026-11-16-boot-diagnostics-boundaries

- **结果**: 固定目录描述符并限制启动失败日志；原生链接攻击/尾部保留与清理通过。
- **证据**: [完整记录](../../docs/evidence/runtime/20261116-boot-diagnostics-boundaries/OUTCOME.md)。
- **验证**: focused20、CLI935、contracts118、installer8；3 Rust native+3 Python；默认2299/6/50，磁盘服务失败保留。
- **限制**: 原始15s故障、stop失败所有权、完整GA和外部Git/平台门槛未完成；RSS/tokens/cost未知。

## T2026-11-16-boot-stop-preservation

- **结果**: 启动失败的stop拒绝保留原handler和工作资源；重试完成才清理。
- **证据**: [原生故障注入、归档和清理](../../docs/evidence/runtime/20261116-boot-stop-preservation/OUTCOME.md)。
- **验证**: focused3、VM236、CLI935、contracts24、installer8；3Rust+3Python native；默认2302/6/51。
- **限制**: stop错误为合成注入，VM和写入/实际重试停止为原生；外层pool/caller丢弃owner仍待做，GA未完成；RSS/tokens/cost未知。

## T2026-11-16-verify-home-isolation

- **结果**: 门禁强制私有K_HOME与短TMPDIR，原始退出码和留证保留。
- **证据**: [真实执行与历史目录声明更正](../../docs/evidence/runtime/20261116-verify-home-isolation/OUTCOME.md)。
- **验证**: 4个边界/实际Host socket检查；真实Runtime2302/6/51，归档源码未变，本轮未重跑VM原生。
- **限制**: 旧默认门禁未建立真实用户目录不变的证明；显式隔离原生证据仍有效；pool/template/GA待办继续；RSS/tokens/cost未知。

## T2026-11-16-k-init

- **完成日期**: 2026-11-16
- **结果**: create-missing 初始化 `.kstudio` + Cursor 发现路径；`AGENTS.md` 保留；冲突 0。`sync-k-skills --check` 与 `verify-ai-eng --level L3` 均 PASS。产品目标未澄清。
- **产物**: `.kstudio/`（含 skills、workflow/bootstrap、wiki、evidence）、`.cursor/{commands,rules,skills}/`
- **证据**: `.kstudio/evidence/T2026-11-16-k-init/OUTCOME.md`
- **限制**: 未跑 `cargo test`；未声明产品 Pass；skill picker 刷新未观测；usage/prices unknown

## T2026-11-16-pool-template-ownership

- **结果**: 独立私有模板和实际 consumer/finalizer 约束清理；保留 caller 目录，修复原生复现的 APFS 重复清理失败。
- **证据**: [失败、最终归档和清理](../../docs/evidence/runtime/20261116-pool-template-ownership/OUTCOME.md)。
- **验证**: ownership19、close4、CLI935、contract118、installer8；3Rust+3Python native、4daemon/16初始Ready VM/4命令、41home清理；默认2315/6/51。
- **限制**: Mac 冷池证明不等于 Linux snapshot 原生；外层 pool/legacy owner 丢失和 GA 未完成，Git UID501/DNS 外阻塞；RSS/tokens/cost未知。

## T2026-11-16-pool-shutdown-ownership

- **结果**: 已登记池/租约的关闭失败与取消保留实际owner；Stop等待清理，rootfs/匿名卷失败可精确重试，替代socket对象保留。
- **证据**: [回归、最终安装包与原生清理](../../docs/evidence/runtime/20261116-pool-shutdown-ownership/OUTCOME.md)。
- **验证**: Runtime2322/6/52、CLI938、contract library24、installer8；3Rust+3Python native、4daemon/16初始Ready/4命令、52home审计通过。
- **限制**: 初始endpoint准入、pre-admission/legacy owner路径和GA继续；Git UID501/DNS阻塞远端；Windows ARM64/快照原生/RSS未验证，tokens/cost未知。

## T2026-11-16-pool-socket-ownership

- **结果**: 初始端点保留调用方对象/活动满队列；精确receipt和生命周期锁恢复自有失效socket，无法证明的旧端点拒绝。
- **证据**: [失败、最终归档与53home审计](../../docs/evidence/runtime/20261116-pool-socket-ownership/OUTCOME.md)。
- **验证**: CLI949、contracts106顶层+12子回执、installer8/Clippy；默认2322/6/52；2Rust+3Python AF、13拒绝+重复启动、2autostart命令、4daemon/16Ready/4命令。
- **限制**: pre-admission/legacy owner和GA继续；远端Git UID501/DNS阻塞，WindowsARM64依赖不支持/无原生，RSS/tokens/cost未知。历史036的contract118计数含子回执，正确独立顶层数为106。

## T2026-11-16-pool-handoff-ownership

- **结果**: published acquire/release 保留实际owner，独占交接；取消关闭准入，维护只清理所选对象。
- **证据**: [失败、最终包和32home审计](../../docs/evidence/runtime/20261116-pool-handoff-ownership/OUTCOME.md)。
- **验证**: Pool64/0/3、CLI949/Clippy；默认2329/6/53；4Rust+3Python、4daemon/16Ready/4命令；wrapper冲突失败保留，实际原生1/0已独立核验。
- **限制**: 内部boot/allocation/template/batch/unpublished/legacy owner与GA继续；Git UID501/DNS外阻塞；WindowsARM64依赖不支持/无原生，RSS/tokens/cost未知。

## T2026-11-16-pool-batch-ownership

- **结果**: 完成的boot批次先登记实际owner，结果携带取消guard；等待后精确发布，关闭失败保留重试。
- **证据**: [原生0/1至1/0、最终包及31home审计](../../docs/evidence/runtime/20261116-pool-batch-ownership/OUTCOME.md)。
- **验证**: Pool68/0/4、CLI949/Clippy；默认2333/6/54；5Rust+3Python、4daemon/16Ready/4命令；失败源码/client保留。
- **限制**: 内部boot/template/unpublished/legacy/rollback与GA继续，Git UID501/DNS阻塞；WindowsARM64依赖不支持/无原生，RSS/tokens/cost未知。

## T2026-11-16-pool-rollback-identity

- **结果**: 回滚只选择当前精确ID，修复实际工作数据误删；计数一致，等待前登记owner。
- **证据**: [0/1至1/0、最终包与26home审计](../../docs/evidence/runtime/20261116-pool-rollback-identity/OUTCOME.md)。
- **验证**: Pool71/0/5、CLI949/Clippy；默认2336/6/55；3Rust+3Python、4daemon/16Ready/4命令；审计命令拼写失败保留。
- **限制**: drain取消/internal/template/unpublished/legacy及GA继续；GitUID501/DNS外阻塞，WindowsARM64依赖不支持/无原生，RSS/tokens/cost未知。

## T2026-11-16-pool-drain-detachment

- **结果**: 两种drain及失效处理在移出VM前等待统计锁，取消保留实际owner；0/3至3/0。
- **证据**: [最终包、原生和清理](../../docs/evidence/runtime/20261116-pool-drain-detachment/OUTCOME.md)。
- **验证**: Pool74/0/6、CLI949/Clippy；默认2339/6/56；3Rust+3Python、4daemon/16Ready/4命令/26home审计；AF准备失败保留。
- **限制**: maintenancejoin/internal/template/unpublished/legacy与GA继续；GitUID501/DNS外阻塞；WindowsARM64依赖不支持/无原生，RSS/tokens/cost未知。

## T2026-11-16-pool-maintenance-join

- **结果**: 两种drain取消等待后保留实际JoinHandle，重试等待后台物理清理；0/2至2/0。
- **证据**: [最终包、真实VM和清理](../../docs/evidence/runtime/20261116-pool-maintenance-join/OUTCOME.md)。
- **验证**: Pool76/0/7、CLI949/Clippy；默认2341/6/57；3Rust+3Python、4daemon/16Ready/4命令/25home审计。
- **限制**: legacy publication/internal/template/unpublished等owner及GA继续；GitUID501/DNS外阻塞；WindowsARM64不支持/无原生，RSS/tokens/cost未知。

## T2026-11-16-legacy-handler-publication

- **结果**: 复用pending槽保留返回handler，取消/重复获取/冲突不丢owner；默认清理接通，0/1至1/0。
- **证据**: [失败、归档、真实VM和清理](../../docs/evidence/runtime/20261116-legacy-handler-publication/OUTCOME.md)。
- **验证**: VM246/0/3、CLI949/Clippy；默认2344/6/58；3Rust+3Python、4daemon/16Ready/4命令/26home审计；受控实际handler交接，救援未用。
- **限制**: internalboot/unpublished/legacy/from_pid与GA继续；GitUID501/DNS仍外阻塞；WindowsARM64不支持/无原生，RSS/tokens/cost未知。

## T2026-11-16-pool-boot-owner

- **结果**: 冷/恢复启动在await前登记实际owner，前台和批次消费同一guard；0/1至1/0，干净失败仍可准入。
- **证据**: [失败、归档、真实VM和清理](../../docs/evidence/runtime/20261116-pool-boot-owner/OUTCOME.md)。
- **验证**: Pool78/0/8、CLI949/Clippy；默认2346/6/59；3Rust+3Python、4daemon/16Ready/4命令/28home审计；编译失败保留/修复。
- **限制**: unpublished/queuedshutdown/providerinner/legacy/from_pid与GA继续；GitUID501/DNS外阻塞；WindowsARM64不支持/无原生，RSS/tokens/cost未知。

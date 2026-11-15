# kbox 本地引擎边界

kbox 提供通用 OCI 执行资源、Task／Service 生命周期、目录授权、预热和进程流。
引擎不识别上层产品、安装包格式、应用安装器或应用回执。

- ExecutionManager 管理实际 VM、进程、挂载、网络及清理。
- ManagedRuntimeClient 管理 Task／Service 操作回执和最后一次供应商观测。
- 调用方管理包下载与校验、客体安装、依赖与认证、应用协议和就绪判定。
- 调用方可通过通用进程流传入字节、执行客体命令。引擎不解释输入和输出。
- 引擎不预留任何应用安装路径，也不根据应用类型自动选择特权。

意图使用 `kbox.engine.intent.v2`，发现协议为 `kbox.engine.v2`。
Service 的 `semantics_profile_digest` 由调用方显式指定，默认保留供应商安全策略。
可选 `prewarmBindingsDigest` 只为干净的空闲 Service 开启预热，不依赖包安装。
目录授权和执行代次约束继续在实际执行点校验。

`diagnose` 只报告引擎身份、版本、宿主平台和虚拟化探测结果，不启动环境，
也不证明应用依赖、认证或任务可用性。

v1 意图和 journal 不再兼容。旧 journal 被拒绝并原样保留；升级前使用旧程序停止
旧环境和预热资源，再使用新的 `K_HOME`。保留旧目录和旧程序可回滚。

[Runtime 合同](../../crates/box/runtime/README.md#local-execution-engine) ·
[本次验证](../evidence/engine/20261114-generic-engine/OUTCOME.md)

# 引擎数据模型草案

所有 wire 字段为版本化数据合同，不是第二种产品配置语言。安全值采用闭合类型；实体真实状态不在客户端复制。

| 实体 | 必要信息 | 所有者与不变量 |
| --- | --- | --- |
| EngineIdentity | `engineId`、协议版本、平台、声明能力 | 引擎；重建 home 不继承旧引擎身份 |
| AcceptedIntent | caller、`instanceId`、`intentRevision`、desired state、`planDigest`、原请求引用 | 控制面持久接收记录；更新 revision 可先到，旧 revision 拒绝；不作为 VM 状态 |
| OperationRef | `requestId`、请求摘要、`operationId`、Runtime receipt 引用 | 复用 ManagedRuntimeClient；同 ID 不同请求冲突；副作用前记录恢复身份 |
| ExecutionBinding | `engineId`、`executionId`、`generation`、原计划摘要 | ExecutionManager 真实观察；客户端或 pool lease 字符串不能自行生成 |
| ProcessStream | `streamId`、request／execution binding、连接占用、输入关闭标记、唯一退出结果 | 引擎会话映射；绑定精确代次，首版不可恢复，不自动重放 stdin |
| CleanupObservation | 原绑定、待回收项、真实退出与回收结果 | 原执行权威；未证实为 `cleanup_pending`／`unknown`，保留记录 |
| SecretBindingRef | caller／Run／generation、用途与材料引用 | 临时凭据所有者；持久模型不含凭据值 |

KRun SQLite 保存的是待派发用户意图和原请求；引擎只对已持久接收的请求承担后台执行责任。引擎恢复 AcceptedIntent 后，以原请求身份查询或重新接线现有 Runtime 回执，不新建 operation 或执行代次掩盖不确定结果。

`intentRevision` 表示用户意图次序，`generation` 表示引擎执行代次，二者不得混用。旧进程清理未确认时，不能建立替代代次。操作截止时间、CLI 等待截止时间与任务期限分别建模。

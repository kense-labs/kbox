# 本地引擎 v1 与 process-stream.v1

`kbox engine endpoint` 输出 `kbox.engine.endpoint.v1` 数据，字段为
`transport: "unix"` 和 `path`，不启动引擎或任务。默认端点由 KBox 根据
home 与用户身份生成，适配 Unix socket 路径长度；独立客户端不复制该算法。

状态：协议规划，编码前由合同测试冻结。此文不是现有公共 API 的声明。

## 控制通道

首版为同一用户的本地 IPC。Unix socket 限制为所属用户访问并验证对端身份；由平台适配层处理。不支持的本地传输返回明确错误，不暴露网络监听作为静默替代。

| 操作 | 合同 |
| --- | --- |
| Discover | 返回 engine ID、协议版本、原生平台和能力；未知但未要求的能力不影响已有客户端 |
| ApplyIntent | caller、instance、request ID、intent revision、plan digest、固定计划引用及执行策略；先持久接收，返回原 operation 引用 |
| InspectOperation／InspectExecution | 按精确身份查询；不可达／观测不足返回未知，不合成成功 |
| Stop | 提交更新停止意图并 fence 旧启动；真实清理证明后才报告完成 |
| ReadLogs | 返回诊断及生命周期事件；不默认提供完整 ACP 正文 |
| OpenProcessStream | 独立能力；精确绑定执行代次与唯一请求，独占启动／连接，不重用 unary Exec schema |
| BindSecrets | 单独可发现的能力；材料经独立受保护控制路径进入临时所有者，回执只返回引用 |

控制请求／回执用版本化结构和稳定错误。编码帧上限先沿用现有 16 MiB 上界；安装包用独立分块制品传输或既有制品端口交付，不放入 JSON 请求。

已有 Runtime unary `Exec` 合同保持不变。`process-stream.v1` 单独发现，不根据 `Exec` 是否存在推断流能力。

## 流通道

使用独立输入 data／close 与输出 stdout／stderr／exit 事件。协议内部元数据不成为 ACP 前导行；KRun 面向用户的 stdout 只有 ACP 内容，诊断走 stderr。

- 数据帧初始上限 64 KiB，每方向待发送队列预算不超过 1 MiB；大消息分块，不能因此截断完整 ACP 正文。通过背压等待消费者。
- 每个 stdout／stderr 流内部顺序保持；不承诺两个独立 OS 流的全局时间顺序。
- stdin half-close 后继续读取剩余输出，最终交付一个退出结果。输入关闭不代表输出已读完。
- 第二个 attach 返回 `ALREADY_CONNECTED`。首版无流恢复；数据通道完全断开后返回 `STREAM_LOST` 并进入原代次清理，不使用新请求重放旧输入。
- Open 的结果丢失时按原请求查询；查询或重试不得生成第二个进程，不能假装可以恢复已经丢失的数据流。
- 退出、停止、断线和截止时间竞态由引擎裁定唯一终态；超时未确认清理时仍保留待清理状态。
- 任务可长程运行；帧读取期限防止半帧占用，不构成长程任务总期限。显式任务期限另行绑定。

引擎不解析 ACP method，不修改私有扩展，不注入 MCP 配置。标准 ACP 会话绑定属于上层 Session Manager；进程流为该层和普通 ACP Client 提供完整字节传输。

## 拒绝与幂等

至少覆盖：`UNSUPPORTED_VERSION`、`UNSUPPORTED_CAPABILITY`、`FORBIDDEN`、`REQUEST_CONFLICT`、`STALE_INTENT`、`GENERATION_MISMATCH`、`NOT_FOUND`、`ALREADY_CONNECTED`、`STREAM_LOST`、`FRAME_TOO_LARGE`、`CLEANUP_PENDING`。内部错误保留来源，客户端不匹配人类错误文本。

同一 request ID 的原请求返回原回执；正文变化拒绝。较旧 revision 即使晚到也不能创建资源；相同 revision 不同内容拒绝。更新停止意图的 fence 必须传至真实执行权威，并经迟到创建故障测试证明。

凭据值不进入回执、Debug、日志或流事件；SecretBindingRef、Run 与 generation 的不匹配被拒绝。

## 当前接线约束

控制入口和 `process-stream.primary.v1` 已编码。provider 的
`start_runtime_process` 复用 unary exec 的所有权／代次检查，并返回真实
execution ID、execution generation 和已有进程流。

本地 wire：`OpenPrimaryProcess` 在持久接收后返回 primary 回执与一次性 attach
token，原控制 socket 保持连接；`AttachPrimaryProcess` 在另一条同用户 socket
交付 token，成功响应绑定实际 provider identity 后开始数据帧。
`InspectPrimaryProcess` 只查询原请求的回执，不恢复字节流或重新启动进程。
控制连接独立于 stdin／stdout 背压；控制连接断开可以中止仍阻塞的数据连接。
token 只保存在连接所有者内存中，Debug 隐去其内容，持久日志只有请求摘要。

数据帧复用现有 pool attach 编码：小端 u32 长度，输入 tag 0 + 原始 bytes、
tag 1 关闭 stdin；输出 tag 0 stdout、tag 1 stderr、tag 2 + 小端 i32 退出码。
数据 payload 每帧最多 64 KiB，空闲读取没有期限，已开始但未完成的帧有 30 秒
读取期限。原控制请求和 attach 握手预算结束后释放控制并发许可，不限制流寿命。
退出的完整结果还保存在 primary 回执中。

真实 MicroVM 的大流、半关闭、控制断线、daemon SIGKILL、原请求拒绝重放、
父子进程 50 秒无迟到写入与清理证据见
[native result](../../../docs/evidence/engine/20261111-primary-native/2d048346173e4a11ad5015fdf1b1f6db/result.json)。

后续同批制品已完成默认端点、独立 Rust 客户端、早退尾部、原生断线／崩溃
清理和普通 OCI 回归，见[当前证据](../../../docs/evidence/engine/20261111-primary-current/OUTCOME.md)。
这不是完整 KRun、普通工具流扩展或三种 Harness 的验收声明。

- 上层在启动期间持有现有 Runtime 单元操作租约，持久保存流请求身份；普通
  unary replay ID 不能传入非恢复流。实际进程仍交由 ExecutionManager。
- primary Harness 连接独占；同一 Harness 的多个 ACP session 共用该连接。
  客体工具／终端可以有多个独立进程流，不能因 primary 独占而被整体禁止。
- stdin 半关闭使用显式 close 帧，数据 socket 保持连接直至读取终态；数据
  socket 断开进入清理。只有 primary Harness 的退出／断线触发整代客体回收；
  普通工具进程的结束不能误杀仍在运行的 Harness。
- 控制连接预算只约束握手和控制请求，不得变成长连接数量或 Harness 并发
  上限。流的输入与输出独立推进，以 I/O 背压满足每方向的队列预算。
- guest streaming exec 已增加显式无期限值、有限输入／输出队列和独立 stdin
  写入线程；libkrun Unix 连接保留非阻塞 FD、短写队列和按实际转发计数的 credit。
  应用层按背压推进；底层 vsock 沿用其原有 credit 窗口，不将其声称为应用队列
  预算。普通 unary Exec 的默认值与合同继续由其原 API 定义。
- primary 结束后的新代次属于环境准备，不重放用户输入或重新执行旧任务。
  流回执和新的环境 attempt 必须一起持久更新；cleanup 未确认时不发布可用
  新代次。实际凭据刷新交付仍须通过其独立所有者完成。

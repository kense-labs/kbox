# Krun Box 优化与 Agent Runtime 完整开发计划

**版本**：v2.0

**状态**：待实施

**基线**：当前工作区源码，包含未提交修改

**架构输入**：[Agent Runtime v1.1](architecture/agent-paas-runtime.md)

**范围**：Box、Runtime、数据安全、证据与发布

**估算**：68–101 人周，不作为日历承诺

## 1. 交付目标

本计划交付以下能力：

1. 清理 Krun Box 中无实际执行能力的公开合同。
2. 复用 `krun-runtime`，建立本地 Agent Runtime 控制面。
3. 将 ACP Harness 和异构 Plugin 作为独立 OCI 工作负载运行。
4. 通过 Lifecycle API 管理 Plugin、Volume、Network、Memory 和 Checkpoint。
5. 通过 Service Gateway 和 Capability Broker 访问本地及云端 Provider。
6. 提供工作负载身份、数据分类、任务级凭证和数据删除能力。
7. 记录环境快照、执行轨迹、干预效果和删除证据。
8. 提供全状态 checkpoint、冻结源分支、动态网络和热 resize。
9. 保留现有 Box CLI、SDK、Compose、CRI、暖池和后端兼容路径。
10. 按真实后端和原生平台发布支持矩阵。

### 1.1 非目标

- 不重写 ExecutionManager 或 ManagedRuntimeClient。
- 不新建第二套执行生命周期存储。
- 不要求修改 Harness 的推理循环。
- 不把 rootfs snapshot 当作全状态 checkpoint。
- 不把暖池 snapshot-fork 当作用户分支。
- 不在核心计划内实现 GPU、S3 卷、VNC、nested virtualization 或 TDX。
- 不新增 Cargo 工作区或第二套产品配置语言。
- 不承诺未经测量的启动、恢复、分支和体积指标。

## 2. 所有权与模块边界

### 2.1 组件职责

| 组件 | 所有权 | 复用与扩展 |
| --- | --- | --- |
| `krun-runtime` | Task／Service 合同、Run／Service Binding、操作回执和最后观测 | 扩展现有 contract、ManagedRuntimeClient 和 conformance |
| Krun Box | MicroVM、Sandbox、ExecutionId、ExecutionGeneration 和实际资源生命周期 | 扩展现有 ExecutionManager、后端路由和 SDK |
| ACP Session Manager | ACP 会话、权限请求、取消和代次映射 | 放在 `crates/runtime`，不依赖具体 Harness 业务代码 |
| Lifecycle Controller | 将 ResourceSpec 转为 Box 或 Cloud Adapter 操作 | 复用 RuntimeClient，不直接操作 VMM |
| Service Registry | Plugin endpoint、readiness、liveness、版本和流量 | 与 Box 健康证据对账 |
| Service Gateway／Broker | 服务发现、调用准入、协议适配和 Provider 路由 | 作为 Runtime 内接口，不要求独立远程进程 |
| Data Security PDP | 数据访问决定和执行义务 | 与授权 PDP 协同，不替代 PEP |
| Data Protection Service | 分类、敏感识别、脱敏和令牌化 | 检测器只提供辅助信号 |
| Data Lineage Controller | 数据血缘、保留、删除和删除证据 | 删除由 Lifecycle 和 Provider 实际执行 |
| Sentry | 行为事件判定和可选升级调查 | 复用现有 L1–L3，不能代替同步授权 |
| Evidence Store | 事件、决定、干预、效果、血缘和查询投影 | 保存有界引用，不复制无限正文 |
| Model Gateway | 模型 Provider 适配 | 不承担所有 Plugin 流量 |

### 2.2 状态权威

| 状态 | 权威来源 | Runtime 保存内容 |
| --- | --- | --- |
| VM、进程、挂载和网络 | Box ExecutionManager | ExecutionId、generation 和最后 Observation |
| Task／Service 操作 | ManagedRuntimeClient | request receipt、lease 和 unit record |
| ACP 会话 | ACP Session Manager | session 与 RunBinding 映射 |
| Plugin endpoint | Service Registry | ServiceBinding 和健康证据 |
| Provider 位置与约束 | ProviderBinding | 本地／云端适配和策略版本 |
| 数据血缘 | Data Lineage Controller | DataRef、衍生关系和删除状态 |
| 干预效果 | 实际 PEP | 执行结果和证据引用 |

Runtime 重启后通过 provider `inspect()` 对账。Runtime 不根据本地副本猜测 VM 存活，不自动切换后端。

### 2.3 代码布局

```text
crates/runtime/src/
  contract/                 镜像清单、Binding、Resource、Data 和 Event 合同
  acp/                      ACP Session Manager 与协议 fixture
  lifecycle/                Resource Lifecycle Controller
  service/                  Registry、Gateway、Broker
  security/                 Data PDP、Protection、Lineage
  evidence/                 Event Store、Projection、TraceQuery

crates/box/
  core/                     执行、制品、凭据和策略类型
  runtime/                  capture、restore、branch、credential、resize
  netproxy/                 egress、拦截和拒绝证据
  sdk/                      typed API 与能力发现
  cli/                      新命令和兼容入口

crates/sentry/              行为判定与干预请求
apps/gateway/               模型 Provider 适配和模型侧证据
```

独立节点二进制只有在部署测试需要时加入现有 crate。产品配置继续使用 ACL。镜像清单和事件 schema 属于数据合同，不另建产品配置系统。

## 3. Krun Box 优化清单

| 项目 | 处理 | 边界 |
| --- | --- | --- |
| `volume.size_limit` | 新请求拒绝非零值 | 旧记录只读，不用 rootfs 配额冒充命名卷配额 |
| OOM 两个覆盖项 | 新请求直接拒绝 | 旧记录继续读取 |
| TDX | 在 CRI／CLI 入口拒绝 | SEV-SNP 和 RA-TLS 保持不变 |
| `Ipam6` | 删除未接线实现 | IPv4 不变，IPv6 继续拒绝 |
| peer helper | 删除未接线逻辑 | 真实数据面完成后再恢复 API |
| KBS／periodic reattestation | 移入 `tee/experimental/`，移除正式 re-export | 不新增 feature flag |
| 历史 Pipeline | 迁到内部 CLI 测试工具 | 证明等价后再删除 SDK 旧实现 |
| Legacy 路由和状态 setter | 盘点调用与退休条件 | 不在首个切片强删 |
| 暖池、双后端、SEV-SNP、Compose、CRI、Sidecar | 保留 | 均有真实执行链或兼容职责 |

优化只阻止无效新输入。禁止借清理删除旧用户状态。

## 4. 统一合同

### 4.1 工作负载与绑定

- `HarnessImageManifest`：ACP 版本、digest、AI-BOM、入口、能力和资源。
- `PluginImageManifest`：协议、能力、AI-BOM、健康检查、endpoint 和 rollout。
- `RunBinding`：Task／Run／attempt、ACP session、Harness digest 和 generation。
- `ServiceBinding`：Service／Replica／endpoint、Plugin digest、健康和流量。
- `ProviderBinding`：位置、信任域、数据等级、驻留、保留、训练使用和凭据。

### 4.2 资源与状态

- `ResourceSpec`：期望状态。
- `Operation`：生命周期操作与重复请求键。
- `ResourceObservation`：实际状态和健康证据。
- `ExecutionCheckpointManifest`：RAM、vCPU、设备、磁盘、挂载、网络和 Secret 引用。
- `BranchLineage`：源 checkpoint、父执行和分支深度。

### 4.3 数据安全

- `DataRef`：数据标识、分类、来源、驻留和删除根引用。
- `DataContext`：身份、任务、目的、DataRef、信任级别和委托链。
- `DataSecurityDecision`：effect、obligations 和 policy_version。
- `ApprovalGrant`：请求摘要、数据等级、Provider、代次和有效期。
- `LineageRecord`：来源、衍生关系、保留和删除状态。
- `DeletionOperation`：删除范围、执行状态和 Provider 回执。

### 4.4 证据

- `ObservedEvent`：已观测事实。
- `Decision`：授权、数据安全或行为判定。
- `Intervention`：实际干预、执行点、结果和效果。
- `EnvironmentSnapshot`：环境和策略快照。
- `ExecutionTrace`：执行 DAG。
- `TraceQuery`：有界脱敏查询。

所有公开枚举使用闭集。安全关键 schema 拒绝未知字段。错误匹配使用类型或稳定错误码。

## 5. 里程碑、依赖与估算

```text
M0 Box 合同清理
  └─ M1 Runtime 合同与验证基线
      ├─ M2 本地 ACP 执行 MVP
      │   └─ M3 Plugin 与 Provider 数据面
      │       └─ M4 数据安全
      │           └─ M5 证据与干预
      │               └─ M7 发布与规模化
      └─ M6 Box 高级状态能力
          └─ M7 发布与规模化
```

M6 的制品库和凭据代理可在 M1 后并行启动。全状态 checkpoint 必须等待 M2 建立真实执行基线。M2 的恢复仅指新 attempt 或既有 rootfs 恢复，不代表内存现场恢复。

| 里程碑 | 内容 | 估算（人周） | 主要出口 |
| --- | --- | ---: | --- |
| M0 | Box 无效合同清理 | 3–4 | 无效输入提前拒绝，旧状态可读 |
| M1 | 统一合同、conformance 和性能基线 | 4–6 | 合同冻结，测试环境可重复 |
| M2 | 本地 ACP Harness OCI 执行 MVP | 6–9 | 两种 Harness 完成启动、执行、取消、恢复和回收 |
| M3 | Plugin、Registry、Gateway 和 Broker | 8–12 | 两类 Plugin 可受管、可路由 |
| M4 | 数据安全与 Provider 数据约束 | 8–12 | 身份、分类、记忆和路由约束生效 |
| M5 | 证据、行为干预和 TraceQuery | 6–9 | 决定与实际效果可分别查询 |
| M6 | 制品、凭据、checkpoint、分支和动态资源 | 27–39 | Linux KVM 高级状态能力合格 |
| M7 | 迁移、性能、归档和发布 | 6–10 | 安装后 smoke 和支持矩阵完成 |
| 合计 | 不含可选增强 | **68–101** | M1、M2、M6C 后重新估算 |

估算假设：2 名 Rust／虚拟化工程师、1 名 Runtime／SDK 工程师、1 名网络／安全工程师，测试与发布兼职参与。

## 6. 详细任务

### M0：Krun Box 合同清理

| ID | 工作 | 验证与出口 |
| --- | --- | --- |
| B001 | 拒绝非零 `volume.size_limit` | 创建前返回稳定错误，旧卷可查询和删除 |
| B002 | 拒绝无效 OOM 覆盖 | 新 execution 不带无效字段，旧记录不误拒 |
| B003 | 提前拒绝 TDX | SEV-SNP、模拟 SNP 和 RA-TLS 回归通过 |
| B004 | 删除 `Ipam6` 和 peer helper | IPv4 与 DNS 不变，Strict／Custom 仍拒绝 |
| B005 | 隔离 KBS 和 periodic reattestation | 正式 API 无占位能力，秘密扫描通过 |
| B006 | 盘点 Legacy、Pipeline 和 setter | 形成生产／测试／未知调用清单 |
| B007 | 迁移历史 Pipeline 到 CLI 测试工具 | typed client parity 和 KVM 测试等价 |
| B008 | 更新支持矩阵和能力 fixture | 文档不再把类型存在写成执行支持 |

M0 的 B001–B005 阻断后续新 execution 入口。B006–B007 可并行，不阻塞 M1。

### M1：统一合同与验证基线

| ID | 工作 | 验证与出口 |
| --- | --- | --- |
| R001 | 冻结工作负载、Binding、Resource 和错误合同 | serde fixtures、未知字段和版本测试通过 |
| R002 | 冻结 DataContext、Decision、Intervention 和 Lineage 合同 | 秘密字段不进入 Debug、Display 和日志 |
| R003 | 扩展 Runtime capability discovery | 调用方可在创建前识别不支持组合 |
| R004 | 建立 Runtime provider conformance | 覆盖 apply、inspect、stop、remove 和恢复对账 |
| R005 | 固定两个 ACP OCI fixture | Rust 与 TypeScript 实现具有不同能力集 |
| R006 | 固定两个 Plugin OCI fixture | 一个无状态 MCP，一个带持久卷的 HTTP 服务 |
| R007 | 固定 Linux KVM 环境和镜像 digest | 真实非 ignored smoke 可重复 |
| R008 | 建立恶意输入、崩溃和秘密扫描框架 | 支持截断、篡改、路径逃逸和 SIGKILL |
| R009 | 建立性能与体积基线 | 同镜像重复测量并保存原始结果 |

### M2：本地 ACP 执行 MVP

| ID | 工作 | 验证与出口 |
| --- | --- | --- |
| R101 | 实现 ACP Session Manager | 版本、会话、prompt、权限和取消映射完整 |
| R102 | 实现 RunBinding 与代次 fencing | 旧 session 和旧 generation 请求全部失效 |
| R103 | 实现 Lifecycle Controller | ResourceSpec→Operation→Observation 可追踪 |
| R104 | 接入 Box RuntimeDriver | Runtime 不直接操作 VMM，不自动切换后端 |
| R105 | 实现 OCI Harness 启动和受控 relay | 不开放任意宿主端口，镜像按 digest 固定 |
| R106 | 实现 prompt、事件和权限请求传输 | 不支持事件返回错误，不转成普通文本冒充 |
| R107 | 实现取消、超时和进程组回收 | 无晚写、无失控 shim 和 socket |
| R108 | 实现新 attempt 和既有 rootfs 恢复 | 不宣称 RAM 恢复，恢复后重新认证 |
| R109 | 实现重启对账 | Runtime 记录与 Box Observation 一致 |
| R110 | 完成两种 Harness 端到端验收 | 启动→执行→取消→恢复→回收全部通过 |

### M3：Plugin 与 Provider 数据面

| ID | 工作 | 验证与出口 |
| --- | --- | --- |
| R201 | 实现 Plugin ServiceBinding | Service／Replica／endpoint 与 digest 固定 |
| R202 | 实现 readiness 注册和 liveness 摘除 | 未 ready 不接流量，失败只影响对应 Replica |
| R203 | 实现 Service Registry | 查询返回协议、代次、能力和健康证据 |
| R204 | 实现 Runtime Service Gateway | 身份、配额、限流、超时、背压和取消生效 |
| R205 | 实现 Capability Broker | 按 schema、健康和 ProviderBinding 路由 |
| R206 | 实现 Harness 管理 Plugin 的 API | 创建、扩缩容、升级、停止和删除经过 Controller |
| R207 | 实现 Local ProviderBinding | 支持 host-service／vsock 和本地 Plugin 引用 |
| R208 | 实现 Cloud Adapter | 云端 ID、状态和错误转为统一 Observation |
| R209 | 接入模型 Provider 适配器 | 复用 Model Gateway，不转发无关 Plugin 流量 |
| R210 | 完成 MCP 与 HTTP Plugin 验收 | 探活→注册→路由→扩缩容→升级→退役通过 |

### M4：数据安全

| ID | 工作 | 验证与出口 |
| --- | --- | --- |
| R301 | 实现工作负载身份和任务级凭证 | 凭证绑定 Run、Capability、DataRef 和 generation |
| R302 | 实现委托权限衰减 | 子级权限不超过父级，过期授权失效 |
| R303 | 实现 DataContext 和信任标记 | 外部内容默认不可信，类型在入口校验 |
| R304 | 实现 Data Security PDP | allow、redact、deny、require_approval 正确 |
| R305 | 实现 Data Protection Service | 分类、脱敏和令牌化在真实 PEP 生效 |
| R306 | 实现 Memory 写入隔离和加密 | 不可信内容不能直接进入长期记忆 |
| R307 | 扩展 ProviderBinding 数据约束 | 等级、驻留、保留和训练使用参与路由 |
| R308 | 实现 ApprovalGrant | 审批绑定请求摘要、Provider 和 generation |
| R309 | 实现 DataRef 与基础 LineageRecord | 源数据可关联摘要、嵌入、记忆和产物 |
| R310 | 实现 DeletionOperation | 删除有实际 Provider 回执，未知结果先查询 |
| R311 | 将 AI-BOM 接入镜像准入 | 未签名或不符合策略的镜像被拒绝 |
| R312 | 完成数据安全对抗测试 | local_only、委托、记忆、删除和秘密扫描通过 |

### M5：证据与干预

| ID | 工作 | 验证与出口 |
| --- | --- | --- |
| R401 | 实现统一 ObservedEvent | 关联 tenant、run、replica、generation 和 DataRef |
| R402 | 接入 ACP、Lifecycle、Gateway 和 Registry 事件 | 来源归属和顺序经过校验 |
| R403 | 接入 Sandbox 和 guest-init／VMM 事件 | 内部事件只声明实际钩子覆盖范围 |
| R404 | 接入 Cloud Adapter 证据 | 云端回执与本地事件分别标识 |
| R405 | 实现有界 Evidence Store | 默认不保存 CoT、Secret 和敏感正文 |
| R406 | 实现 EnvironmentSnapshot 和 ExecutionTrace | 环境、调用、产物和清理可关联 |
| R407 | 实现 TraceQuery | scope、游标、大小限制和脱敏生效 |
| R408 | 将 Sentry 接入行为决策 | L1 优先，L2／L3 可选，不替代同步授权 |
| R409 | 实现分布式 Intervention | ACP、Lifecycle、Gateway、Sandbox 和 Cloud PEP 执行 |
| R410 | 回写 PEP 实际效果 | Decision.block 不被记录为已阻断 |
| R411 | 完成干预和证据对抗测试 | 伪造事件、晚到事件和未知结果正确处理 |

### M6：Krun Box 高级状态能力

| ID | 工作 | 验证与出口 |
| --- | --- | --- |
| B101 | 实现版本化 checkpoint manifest | CPU、ABI、设备、后端和 Secret 引用校验通过 |
| B102 | 实现内容寻址对象和原子发布 | 去重、no-replace、损坏拒绝和崩溃恢复通过 |
| B103 | 实现有界读取、安全解包和 GC | 路径逃逸、链接、超限和引用保留通过 |
| B201 | 实现 header-only 占位符凭据代理 | 客体不可读真实值，目标外请求拒绝 |
| B202 | 实现 name-constrained CA 和按请求解析 | 未授权域名不可用，resolver 失败不直连 |
| B203 | 扫描所有持久和输出面 | 合成 Secret 不出现在记录、日志和 checkpoint |
| B301 | 抽象后端 capture 能力 | 暂停源之前完成能力探测 |
| B302 | 捕获 RAM、vCPU、设备和磁盘一致现场 | Linux KVM 同机恢复进程和内存标记 |
| B303 | 恢复新 generation 并重新认证 | 旧代次写入全部失效 |
| B304 | 默认拒绝宿主绑定状态 | 未授权挂载、socket、GPU 和网络状态被拒绝 |
| B401 | 实现冻结源分支和身份重建 | 子机拥有独立身份和资源授权 |
| B402 | 实现 worker-ready 和批量回滚 | 未 ready 不交付，失败清理整批子机 |
| B403 | 实现分支池、lease 和深度限制 | 池耗尽、超时和链上限行为正确 |
| B501 | 实现动态 egress | 更新后真实 allow／deny 与回执一致 |
| B502 | 实现内存和磁盘只增 | 部分生效可恢复对账 |
| B503 | 实现 resize receipt 和崩溃恢复 | 旧 VMM 操作不重放到新 execution |

M6B 只声明 HTTPS 443、HTTP/1.1 和 header。M6C 只声明 Linux KVM、BoxVm、同机和无宿主绑定状态。其他平台逐项资格化。

### M7：迁移、性能与发布

| ID | 工作 | 验证与出口 |
| --- | --- | --- |
| R501 | 实现 checkpoint 历史和签名准入 | 历史选择无歧义，不可信制品拒绝 |
| R502 | 验证同构第二主机恢复 | 只声明同 OS／arch 和已验证配置 |
| R503 | 实现便携制品和完整同伴检查 | 缺文件、错误 ABI 和不可信来源被拒绝 |
| R504 | 完成冷启动、调用、恢复和分支基准 | 保存 P50／P95／P99 和环境信息 |
| R505 | 完成 N=1／4／8／16 密度与 soak | 无泄漏、晚写和资源失控 |
| R506 | 测量最小 Runtime 引导包 | 记录压缩包、安装磁盘和内存，不预设达标 |
| R507 | 完成安装前缀 smoke | 最终归档而非开发机 binary 通过 |
| R508 | 更新支持、恢复、回滚和迁移文档 | 支持矩阵只含原生实测项目 |

## 7. 端到端验收场景

| 场景 | 必须验证 |
| --- | --- |
| ACP 执行 | 两种 Harness 启动、prompt、权限、取消和回收 |
| ACP 恢复 | 新 attempt 可用，旧 session 和 generation 失效 |
| Plugin 生命周期 | Harness 创建 Plugin，readiness 后调用，失败后替换 |
| Provider 透明 | 同一 schema 使用本地或云端 Provider |
| 云端故障 | 本地能力继续，敏感请求不自动改道 |
| 身份委托 | 子任务权限衰减，任务结束凭证失效 |
| 不可信输入 | 网页或工具输出不进入指令通道和长期记忆 |
| 数据脱敏 | Decision.redact 与实际输出变化一致 |
| 数据删除 | 源数据及衍生数据返回逐项删除证据 |
| 行为干预 | 决定、PEP 动作和实际效果分别记录 |
| 全状态恢复 | 进程、RAM、磁盘和 listener 在同一暂停点恢复 |
| 冻结源分支 | 子机身份独立，源保持冻结，失败整批回收 |
| 动态资源 | egress 和 resize 的实际状态与回执一致 |
| 发布安装 | 隔离前缀安装后执行真实 ACP 和 Plugin smoke |

## 8. 测试与资格化

| 层级 | 内容 | 可声明结果 |
| --- | --- | --- |
| Hermetic unit | 合同、策略、状态机、存储、GC 和错误 | 逻辑合同成立 |
| Hermetic integration | Runtime、Box fake、ACP、Plugin、CLI 和 SDK | 调用链与兼容性成立 |
| 恶意输入／fuzz | 镜像、manifest、事件、路径和巨大声明 | 不 panic、不越界、不无限工作 |
| 真实 ACP／Plugin fixture | 两种 Harness 和两类 Plugin | 协议和生命周期成立 |
| Linux KVM 后端 | exec、网络、checkpoint、branch 和 resize | 仅声明被测配置 |
| 崩溃恢复 | SIGKILL、重启、部分操作和旧代次 | 无假成功、可重试、无泄漏 |
| 数据安全对抗 | 注入、委托、秘密、云端改道和删除 | 控制实际生效 |
| Soak | 反复启动、取消、恢复、分支和删除 | 无晚写和资源失控 |
| 发布归档 | 隔离前缀安装和 smoke | 最终制品合格 |

每个 Rust 行为切片执行：

```bash
cargo fmt --all -- --check
cargo test --locked -p <owning-package> <focused-filter>
cargo clippy --locked -p <owning-package> --all-targets --no-deps
```

随后运行根工作区适用的 CI 检查。macOS 和 Windows 声明需要对应原生测试。交叉编译不能替代运行时资格化。

每个行为切片在 `docs/evidence/box/<date-slug>/OUTCOME.md` 或对应 Runtime 域记录实际命令、计数、平台、日志和 **Explicitly not claimed**。

## 9. 性能与体积门禁

M1 建立基线，M7 比较候选版本。至少记录：

- 冷启动到 authenticated ACP prompt。
- Plugin readiness 和首次调用延迟。
- exec／MCP／HTTP RTT 和失败率。
- 取消到无晚写的时间。
- rootfs 恢复和全状态恢复时间。
- capture 暂停时间和完整发布时间。
- 分支 P50／P95／P99 和内存密度。
- Evidence Store 写入量、查询延迟和丢弃计数。
- 清理后的 shim、socket、挂载和目录残留。
- 最小引导包压缩体积、安装磁盘和运行时内存。

40–50 MB 只指最小 Runtime 引导包，不包含 Harness／Plugin 镜像、客体内核、rootfs、模型、索引和缓存。最终是否达标以 M7 测量为准。

## 10. 灰度、兼容与回滚

1. M0 只拒绝无效新输入，旧状态保持可读和可清理。
2. M1 新合同先进入 conformance，不改变默认 VM 行为。
3. M2 ACP OCI 通过显式 RunSpec 启用，现有 Box CLI／SDK 不变。
4. M3 Plugin 通过显式 ServiceSpec 启用，不影响现有 Compose／CRI。
5. M4 数据策略默认最小权限，未声明能力不自动授予。
6. M5 证据查询只读，干预按 PEP 逐项开放。
7. M6 checkpoint／branch 只在已资格化后端开放。
8. M7 跨机和便携包按平台单独开放。

回滚只停止新 API、新授权和新写入。禁止删除旧制品、旧记录或证据。凭据代理回滚禁止静默改为直连。新 schema 不做 best-effort 猜测读取。

## 11. 风险与停止条件

| 风险 | 停止条件 |
| --- | --- |
| Runtime 与 Box 双状态漂移 | 对账不一致时停止 Ready，以后端 Observation 为准 |
| ACP 语义丢失 | 无法表达权限、取消或事件时拒绝接入 |
| Plugin 绕过 Gateway | 发现直连宿主端口或未授权网络时停止服务 |
| Provider 静默降级 | 数据约束不满足时拒绝路由 |
| 秘密泄漏 | 任一合成 Secret 出现在持久或输出面即阻断发布 |
| 记忆污染 | 不可信来源绕过隔离时关闭对应写入路径 |
| 干预假成功 | 无实际 PEP 效果时禁止发布干预能力 |
| checkpoint 语义降级 | RAM 或进程不一致时禁止声明恢复和分支 |
| 旧代次写入 | 任一旧 generation 写成功即阻断恢复发布 |
| 证据无限增长 | 超限时停止无界导出并启用有界保留 |
| 平台宣称过度 | 无原生证据的平台从支持矩阵移除 |
| 范围膨胀 | GPU、S3、VNC、nested 和 TDX 不进入核心切片 |
| 工作区污染 | 逐路径 diff，不回滚或混入其他会话修改 |

## 12. 团队分工与评审

| 角色 | 责任 |
| --- | --- |
| Box／虚拟化 | ExecutionManager、capture、restore、branch、resize 和后端资格化 |
| Runtime／SDK | Runtime 合同、ACP、Lifecycle、Registry、Gateway 和 Binding |
| 网络／安全 | egress、凭据、数据安全、记忆、删除和威胁测试 |
| 测试／发布 | fixture、conformance、崩溃、soak、性能、证据和归档 |

每个里程碑执行 3 次评审：

1. 入口评审：确认合同、依赖、非目标和验证方式。
2. 对抗评审：验证越权、伪造、泄漏、旧代次和部分成功。
3. 出口评审：按实际证据更新支持矩阵和后续估算。

## 13. 完成定义

只有同时满足以下条件，核心计划才算完成：

- [ ] M0–M7 的出口分别通过，没有相互借用证据。
- [ ] Runtime 没有绕过 Box 建立第二套实际执行状态机。
- [ ] 两种 ACP Harness 和两类 Plugin 完成端到端验收。
- [ ] Harness 可管理 Plugin，但不能直接操作宿主资源。
- [ ] 本地和云端 Provider 使用同一调用合同。
- [ ] 受限数据不能路由到不符合约束的 Provider。
- [ ] 不可信输入不能直接进入长期记忆。
- [ ] 删除操作能返回衍生数据的实际结果。
- [ ] 每个干预都有执行点和效果证据。
- [ ] Linux KVM 全状态恢复和冻结源分支通过真实测试。
- [ ] 旧 CLI、SDK、rootfs snapshot、暖池、Compose 和 CRI 回归通过。
- [ ] 所有 Secret 扫描、恶意输入、崩溃恢复和 soak 通过。
- [ ] 支持矩阵只包含原生实测项目。
- [ ] 最终归档安装后 smoke 通过。
- [ ] 所有 OUTCOME.md 都包含真实命令、计数和 Explicitly not claimed。

## 14. 首批开发切片

### 切片 1：合同清理与验证底座

1. 完成 B001–B005。
2. 完成 R001–R004。
3. 建立恶意输入、秘密扫描和 Linux KVM 环境。
4. 固定两个 ACP 和两个 Plugin fixture。
5. 保存当前性能和支持矩阵基线。

### 切片 2：本地 ACP 执行 MVP

1. 实现 ACP Session Manager 和 RunBinding。
2. 实现 Lifecycle Controller 和 Box RuntimeDriver 接入。
3. 完成启动、prompt、权限和取消。
4. 完成新 attempt、重启对账和资源回收。
5. 用两种 Harness 验证完整执行链路。

切片 1 未完成前，不开放新 execution 入口。切片 2 未完成前，不启动 Plugin 规模和全状态恢复宣传。

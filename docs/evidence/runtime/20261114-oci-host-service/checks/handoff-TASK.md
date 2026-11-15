在 /Users/roylin/code/kense/kbox 继续开发。请直接检查代码、实现、测试和记录证据，完成一个子任务后立即推进下一项，不等待我反复说“继续”。

已有状态：
A–F 及最终 macOS arm64 归档验收已完成，历史产物和证据必须保留。下一阶段是独立 OCI Host Service 的 Box 集成与原生验收。

本轮目标：
打通并验证真实链路：
Box ExecutionManager → 持久化 OciSdk 路由 → 公共 SDK → 独立 Host Service → HVF。

先读取：
- AGENTS.md 和 .specify/memory/constitution.md
- docs/evidence/runtime/20261114-final-archive/OUTCOME.md
- crates/oci-runtime/README.md、ROADMAP.md、docs/macos-hvf.md
- Box 路由、OCI backend、迁移入口及相关测试

按以下顺序连续执行：

1. 建立有界任务
   梳理实际缺口，建立或复用 spec、plan、tasks。
   优先复用现有组件，只实现完成目标所必需的代码，不重做已通过的 A–F。

2. 完成 macOS 集成
   检查 Host Service 启动与发现、BundleProvider、系统镜像、依赖资产和资源归属。
   通过现有显式迁移入口创建真实 OciSdk 执行。
   不绕过公共 SDK，不新增重复状态机，不使用 BoxVm 成功结果冒充该链路通过。

3. 验收存量与混合路由
   同时保留真实 BoxVm、OciSdk 记录。
   重建客户端、重启服务、改变新建策略后，存量记录仍按持久化路由运行。
   验证执行身份、provider binding、generation 和资源所有权。
   错误绑定、旧 generation、缺失身份必须明确拒绝，禁止跨后端回退或改写存量路由。

4. 验收故障恢复
   复用现有恢复及 owner-replacement 测试入口。
   覆盖服务 SIGKILL、操作完成后响应丢失、重复请求、并发操作和客户端断连。
   检查真实副作用、进程身份、重复执行及恢复后的可操作性。
   无法恢复的会话按既定协议明确失效，不伪造续接或成功。

5. 验收行为
   覆盖生命周期、exec/I/O、文件和目录授权、宿主只读攻击、挂载、资源限制及清理。
   未实现能力必须在产生副作用前拒绝。
   使用真实独立 Host Service 和固定资产，记录二进制、manifest、系统镜像及依赖校验值。

6. 完成验证与交付
   行为修改先跑针对性测试，阶段完成后运行 scripts/verify-commit.sh。
   原生测试必须确认实际执行数量，并检查执行、volume、进程、挂载和 socket 遗留。
   产品源码变化后，重新构建并验收受影响的安装包，保留旧归档。
   在 docs/evidence/ 中记录实际命令、失败、结果、环境、产物身份和未验证内容。

7. 核对默认切换条件
   对照 ROADMAP 和实际恢复能力，逐项列出切换门槛。
   已满足的范围继续实施和验证；外部条件未满足时保留显式入口并写清交接步骤。
   禁止仅修改默认枚举、放宽验收规则或凭本机结果宣称全局 W4 完成。

持续执行规则：
- 测试失败先修根因，不削弱断言、不跳过失败。
- 外部阻塞先记录，再继续其他可推进项；不要反复等待 Windows 主机。
- 定期简短汇报进展，汇报后继续工作。
- 上下文切换前记录完成项、失败项、下一步和阻塞，恢复后接着执行。
- 只有本轮目标完成，或剩余工作全部被明确的外部条件阻塞且可推进项已做完，才收尾。

保持 kbox 无 KRun 业务特判，保留 ExecutionManager 权威和 generation fencing。
不修改真实 ~/.kbox，不在父仓库编辑原生子模块，不新增 workspace 或无必要依赖，不提交、推送或发布，不提高平台支持等级。

最终说明：实际修改、验证结果、产物和证据位置、剩余阻塞，以及下一条可执行任务。

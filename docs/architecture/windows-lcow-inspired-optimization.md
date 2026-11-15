# Windows：借鉴 LCOW 设计优化 krun/kbox（架构映射）

**状态**：Phase 0 范围锁定（2026-11-15）  
**权威执行提示词**：`docs/agent-prompts/lcow-inspired-krun-windows-optimization.AGENT.md`  
**近期排期**：`docs/agent-prompts/lcow-inspired-krun-next-workplan.AGENT.md`  
**产品就绪权威**：`docs/architecture/support-matrix.json`（`windows-x86_64` = `probe-only`）

本文只做**设计映射与兼容边界**。不把 LCOW/HCS 接进产品路径；不因本文升级 readiness。

---

## 1. 兼容基线（默认路径必须可跑）

| 维度 | 默认必须支持 | 不得作为默认硬依赖 |
| --- | --- | --- |
| OS / SKU | Windows 10/11 x86_64，WHPX 可用（证据类：Win10 Pro 23H2 build 22631） | 仅 Server 2025+、仅某 Insider |
| Hypervisor | WinHvPlatform（WHPX） | 必须以 HCS/runhcs/LCOW v2 创建 UVM；必须以 WSL2 |
| 功能包 | 不要求 Windows「Containers」功能、不要求 Docker/containerd | 默认路径调用 HNS 管理平面 |
| 机密计算 | 普通 x86_64 + WHPX | SEV-SNP / TDX 硬件 |
| 网络 | 无 HNS 时仍有诚实合同（至少 `none` + 可演进的用户态/TSI/port） | 默认 = HNS/vSwitch/Hyper-V 容器网络 |

说明：宿主上可能因启用 Hyper-V/WHPX 而存在 `vmms`/`hns` 等服务——**存在 ≠ 产品依赖**。默认优化不得把 HNS API 写成硬前提。

更高 OS/功能包能力若实现，必须是 **opt-in**，且基线路径行为不变。

---

## 2. 非目标

- 用 HCS / hcsshim / runhcs / `containerd-shim-lcow-v2` 替换 `kbox-shim` + libkrun。  
- 用 WCOW process isolation 或 Host Process Containers「加速」Linux OCI。  
- 静默降级隔离、静默换 hypervisor、第二套执行状态机。  
- 在 Windows 上把 `bridge` 谎称为 supported。  
- 把 libkrun `WINDOWS_ROADMAP` 完成度写成 kbox `supported`。  
- 将机密容器写入默认支持矩阵。

---

## 3. 与现状关系

| 能力 | 现状（Windows） | 本文立场 |
| --- | --- | --- |
| warm pool CLI / daemon | fail-closed / unsupported | Phase 1 先做**运行时预热合同**（WHPX+自研），再决定是否暴露 CLI；不引入 LCOW multipod API |
| bridge networking | CLI 硬拒绝 | 保持拒绝；Phase 3 另立**具名**基线网络后端，HNS 仅 opt-in |
| MicroVM / WHPX | 本机可跑 Linux guest（见既有 E2E） | 继续作为唯一默认虚拟化路径 |
| TEE / attest | unavailable | Phase 5 仅对照文档 |

---

## 4. 特性映射表

| LCOW/生态特性 | 可借鉴设计点 | 基线落地方式（WHPX+自研） | 禁止依赖（高版本/功能包） | 默认 or opt-in | 对应后期 Phase |
| --- | --- | --- | --- | --- | --- |
| UVM 一等公民；生命周期与 container task 分离 | MicroVM 生命周期 ≠ 单次 workload | 扩展现有 shim / named pipe / Job Object / live-session；自有状态机 create→ready→attach/exec→release | HCS 创建 UVM；LCOW v2 Sandbox API；WS2025-only shim | **默认**（设计目标） | Phase 1 |
| 预热 / 可附着 sandbox | 降冷启动：二次任务复用已 ready 的 VM | 预热池或可附着 session；generation fencing；泄漏清理 | containerd Windows；必须 Server SKU | **默认** | Phase 1 |
| 多 pod 共一 UVM（multipod） | 密度：同 guest 多有界任务 | **仅当宪法/架构允许**：guest 内多 execution unit + cgroup/配额合同；默认仍每 Box 一 VM | hcsshim multipod、CRI Sandbox 硬依赖 | **opt-in**（默认不变） | Phase 2 |
| Sandbox API vs Task API 分层 | 控制面与任务面分离，避免每任务全量 boot | kbox/engine 自有合同分层；不引入 containerd 为硬前提 | containerd Sandbox 插件、runhcs | **默认**（合同层） | Phase 1–2 |
| HNS / 原生 vSwitch | 「Windows 上要有诚实网络后端」的产品压力 | **优先**增强 virtio-net / TSI / port-forward / `none`；新后端须具名 | 默认路径绑定 HNS；把 bridge 改名充数 | 基线增强=**默认**；HNS=**opt-in** | Phase 3 |
| 优化 UVM 冷启动（微软栈侧） | boot/virtio/guest-init 成本意识 | libkrun + guest-init + 预热模板；测量后改 | 新版 Windows 专有加速作唯一路径 | **默认**（WHPX 可用 API） | Phase 4 |
| 机密 UVM（SNP 等） | TEE 证明与门禁对照 | 文档：硬件/OS 超出兼容基线；默认不实现 | SNP/TDX、机密 LCOW 运行时 | **非默认**（对照 only） | Phase 5 |
| WCOW process isolation | — | **不落地** | 共享宿主内核 | **禁止** | — |

---

## 5. 兼容性自检（§2.4）

针对本文作为 Phase 0 范围文件：

- [x] 未引入仅 Server 2022/2025、仅 LCOW v2、仅 Containers 功能包才有的 **默认** API 依赖  
- [x] 在「WHPX-only、不以 HNS 为网络平面」设定下，默认优化路径仍可定义完成合同  
- [x] 未把可选能力写成全 Windows 必达  
- [x] 未提高产品最低 OS 版本要求  

---

## 6. 相关证据与提示词

- 基线测量（Phase 0）：`docs/evidence/box/20261115-windows-lcow-opt-phase0-baseline/`  
- Phase 1 设计：`docs/architecture/windows-environment-prewarm-phase1.md`  
- Phase 1 推进证据：`docs/evidence/box/20261115-windows-lcow-opt-phase1-prewarm/OUTCOME.md`  
- 既有 WHPX E2E：`docs/evidence/box/20261109-windows-visualized-e2e/OUTCOME.md`  
- 执行提示词：`docs/agent-prompts/lcow-inspired-krun-windows-optimization.AGENT.md`  
- 持续排期：`docs/agent-prompts/lcow-inspired-krun-next-workplan.AGENT.md`  

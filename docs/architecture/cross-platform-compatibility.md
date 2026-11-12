# kbox 跨平台与兼容性优化：调研与分层方案

状态：**架构调研与提案，2026-11-11**。本文不是实现记录，不声明任何新的平台支持，不代表已通过原生验收，也不改动默认构建链。

证据边界：本次为只读调研（源码、文档、CI 配置、证据链与公开生态资料），未运行任何 HVF／KVM／WHPX 原生验收，未修改构建链或执行后端。文中所有「建议」必须按 `AGENTS.md` 的证据纪律单独实施与验证：`Cross-compilation proves compilation only. Native runtime claims require native tests on the affected target.`（`AGENTS.md:84`）。

参考图：[KRun 集成边界](krun.md) · [沙箱控制](sandbox-control.md) · [KRun 架构](../../../krun/docs/architecture.md) · [运行器决定](../../../krun/docs/decisions/0001-mcp-service-runners.md)

---

## 1. 结论摘要

**kbox 的跨平台问题不是「支持的宿主太少」，而是三件事叠加：宿主与客体被硬绑、平台能力判定散落在数千处 `cfg`、以及每增加一个目标就要重做一整套原生证据。** 因此优化目标应是「降低新增平台／架构的边际成本 + 消除已有口径冲突」，而不是急着扩大支持矩阵。

四条主结论：

1. **虚拟化后端的三角是物理约束，不该被「抽象掉」。** macOS 用 Hypervisor.framework（HVF）、Linux 用 KVM、Windows 用 WHPX，上游 libkrun 自己也是按 `vmm/{linux,macos,windows}/` 分目录维护三份 vstate／MMIO 实现。Firecracker 两次明确拒绝 macOS 支持（issue #2845 `not_planned`）；Virtualization.framework 虽然能少写代码，但它**拥有设备模型**，无法保证跨宿主设备语义一致——不适合作为 kbox 的产品后端。**保留 HVF／KVM／WHPX 是正确选择。**
2. **真正可优化的是产品层，不是 hypervisor 层。** `crates/` 下约 2,900 处含 `target_os` 的 cfg、`libkrun/` 另有 727 处；`crates/box/guest/init/src/exec_server.rs` 单文件 66 处。这些绝大多数不是 hypervisor API，而是 PTY、Unix socket、路径、信号、凭据、文件身份等 OS 表面——可以用能力描述层收敛。
3. **瓶颈是证据链，不是编译。** `crates/oci-runtime/ROADMAP.md:233-235` 明确 W4+ 只剩硬件证据；`README.md:869-872` 把 WHPX R2、KVM R2L x86_64／AArch64 全部标成 `Blocked — needs a newly provisioned ... host`。这是基础设施与流程问题。目前 kbox 有 20 份含 `Explicitly not claimed` 的 OUTCOME，krun 有 35 个证据目录，全靠人工核对。
4. **有一处几乎零成本、收益最大的代码缺陷**：`krun/src/package.rs:144-150` 的 `native_guest_architecture()` 直接用**宿主** `std::env::consts::ARCH` 推导**客体**架构，被 `install`／`up`／`doctor`／guest installer 共 8 处调用。这让 `manifest.target.arch` 事实上失效，也把 macOS/arm64 钉死在 Linux/aarch64。应改为由引擎 Discovery／`ResolvedPlan.runtime.target` 提供。

与前两轮 WASM 讨论的衔接：WASM（Spin／Wasmtime／container2wasm）**不是** kbox 跨平台主路径的解。container2wasm 是把 CPU 模拟器 + Linux 内核 + rootfs 塞进一个 `.wasm`，默认 1 核 128 MB、实验性、CNCF Sandbox；WASM 组件适合放在能力侧（MCP／工具／策略），不适合替换独立内核执行边界。见 `krun/docs/decisions/0001-mcp-service-runners.md`。

---

## 2. 现状：跨平台到底卡在哪

### 2.1 后端分层（保持，不动）

| 层 | 职责 | 关键出处 |
| --- | --- | --- |
| `libkrun` / `libkrunfw`（submodule） | VMM + virtio 设备 / 客体内核 | `AGENTS.md:24-25` |
| `crates/oci-runtime` | OCI 执行、backend routing、平台专属行为 | `AGENTS.md:21`，`crates/runtime/src/platform/mod.rs:1-14` |
| `crates/box` | ExecutionManager：VM／进程／挂载／网络／generation | `AGENTS.md:20` |
| `crates/runtime` | 供应商中立 Task／Service 契约 | `AGENTS.md:22` |

四目标硬编码在 `crates/oci-runtime/crates/kbox/src/runtime_assets.rs:9-14`：`macos/aarch64`、`linux/x86_64`、`linux/aarch64`、`windows/x86_64`。明确拒绝：Intel Mac（`crates/box/runtime/src/host_check.rs:58-63`）、Windows ARM64（`:73-77`）、其它 OS（`:81-87`）、共享宿主内核容器（`README.md:95`）。**2026-11-12 更新**：Windows ARM64 的产品要求已提升为「必须支持」，但当前实现仍拒绝、且无任何原生证据；影响分析与分阶段规划见 [Windows ARM64 规划](windows-arm64-plan.md)，本文 P0 的优先级需结合该约束重新评估。

### 2.2 就绪等级与口径冲突（必须先解决）

| 来源 | 声明 | 问题 |
| --- | --- | --- |
| `dist/RELEASE_NOTES.md:36,46` | 仅 macOS arm64 有发布包；`Not covered: Linux/Windows packages, non-arm64 platforms` | 产品事实 |
| `crates/oci-runtime/README.md:65-71`、`docs/feature-coverage.md:9-10` | **无任何 built-in driver 达到 `supported`**；HVF = `experimental`，KVM／WHPX = `probe-only` | 产品事实 |
| `libkrun/README.md:311-313` | Windows WHPX「Production-ready」「~95%」 | **上游库能力，不是 kbox 产品状态** |
| `crates/box/deps/libksys/README.md:27-30` | 四平台全标 ✅ Supported | **FFI 层口径** |
| `README.md:92-93` | Linux／Windows「path in-tree, build + qualify locally」 | 容易被读成已支持 |

同一份代码库里「Supported」有三种互相矛盾的含义。**这不是文档瑕疵，是兼容性风险**：上层（KRun、kstudio、调用方）会据此判断能否派发，而 `doctor` 的 `diagnose` 只证明宿主虚拟化探测，明确不证明客体依赖、认证或任务可用性（`docs/architecture/krun.md:41-42`）。

### 2.3 平台代码扩散（可优化）

- `crates/` 约 2,924 处 `target_os` 字面量，`libkrun/` 727 处（`libkrunfw` 0，纯 Makefile 按 ARCH 分支）；最集中：`crates/oci-runtime/crates/runtime/src`、`crates/box/guest/init/src`、`libkrun/src/devices/src`、`libkrun/src/vmm/src`。
- 全工作区 Rust 谓词：`cfg(target_os)` 1,497、`cfg(unix)` 1,661、`cfg(windows)` 769，**`target_family` 与 `target_vendor` 均为 0**——没有粗粒度抽象，全靠细粒度组合。
- Go 侧（inkstone/core）315 个文件含 `//go:build`，其中 `windows` 154、`!windows` 107，三平台被编码成 `windows` / `!windows` / `_unix` / `_other` 四种写法。
- `build.rs` 的平台分支很重：`crates/box/deps/libksys/build.rs` 1,261 行里，按 `macos+aarch64` / `linux+x86_64` / `linux+aarch64` 分别钉了不同的 libkrunfw URL 与 SHA256（`:19-37`），Windows 单独走预编译 DLL（`:94-121`）。

### 2.4 证据链（真正的边际成本）

- 纪律本身是对的：`AGENTS.md:84,96,112`、`constitution.md:47,85-87`、`docs/development-standards.md:77,137` 都要求每个被点名的平台有独立原生证据；`AGENTS.md:27` 禁止删除证据。
- 代价是：kbox 20 份 OUTCOME 含 `Explicitly not claimed`，krun 35 个证据目录 / 30 份 OUTCOME，跨仓双向互链靠相对路径人工维护；每加一条平台声明要同步 README、feature-coverage、ROADMAP、release notes、install.sh、support matrix 与证据目录。
- `ROADMAP.md:158-170` 的 `Existing-host greening does not promote readiness` 目前是文字约定，不是机器门禁。
- 测试侧残留：`krun/docs/evidence/2026-11-11-doctor-native/OUTCOME.md:63` 承认原生测试脚本仍依赖配套 KBox 开发树辅助脚本，与「生产构建不绑定 `../kbox`」的合同有张力。

---

## 3. 兼容性合同：现状与三处硬绑

### 3.1 三种 `v1` 不能混为一谈

| 层 | 所有者 | 协商方式 | 不兼容 |
| --- | --- | --- | --- |
| 引擎能力协议（`kbox.engine.v1` + 10 个 `*.v1` 扩展） | kbox | `Discover.protocols[]`，派发前逐项检查 | `UNSUPPORTED_CAPABILITY`，无副作用 |
| 客体协议（`runtime-requirements.v1`、`secret-launch.v1`、`capability-packages.v1`、`native-bindings.v1`） | krun 底座 | `harness-assembly.guest-protocols.v1` 的 `requiredGuestProtocols[]` | `RUNTIME_REQUIREMENTS_UNMET` / 拒绝，不静默降级 |
| wire／制品 schema（`kbox.engine.*.v1`、`krun.*.v1`、`krun.launch-bootstrap.v2`） | 分属两侧 | `schema` 字段 + `deny_unknown_fields` | `UNSUPPORTED_VERSION` |

权威能力清单在 `crates/box/runtime/src/engine/controller.rs:109-136`；稳定错误集合见 `specs/001-krun-engine/contracts/engine-v1.md:43`；JSON Schema 由 Rust 类型生成（`krun/schemas/README.md:3-18`），但 `schemas/README.md:18` 明确「开发期，正式兼容版本在 M0／M1 通过后冻结」——**两仓目前各自维护一份理解**，`krun/docs/implementation-plan.md:47` 已提出要「避免手工维护两套相互矛盾的合同」，尚未落地。

### 3.2 把宿主与客体绑死的三处

1. **`native_guest_architecture()`**（`krun/src/package.rs:144-150`）：宿主 ARCH → 客体 arch，8 处调用（`cli.rs:452,478`、`main.rs:100,155`、`doctor.rs:202,364`）。隐含假设「宿主与客体 CPU 同构」。
2. **`Discovery.platform` / `Diagnostics.platform` 是宿主 `OS/ARCH`**（`controller.rs:134,154`），且 `Diagnostics.validate()` 要求两者逐字相等（`krun/src/engine/protocol.rs:52-59`）；`doctor` 用它做宿主检查，但不能推导客体能力。
3. **制品与 ABI 全字段必填**：`kbox.harness-package.v1` 的 `target.os/arch/libc/minimumLibcVersion`（`krun/src/contract.rs:95-122`），`harness/index.json` 里 14 个包全是 `linux/aarch64/glibc`（Aster ≥2.31、Codex／Dsh ≥2.36）；底座镜像按 OCI digest 钉死（`krun/runtime/base/Containerfile:1`）。`ResolvedPlan.runtime.target` 再钉一次。

### 3.3 已知能力缺口与供应链风险

- **macOS 宿主强制只读 virtio-fs bind 不支持**：`:ro` 在启动前被拒（`docs/evidence/box/20261110-dsh-acp-acceptance/OUTCOME.md:115`、`crates/box/runtime/README.md:99-105`）。这直接影响 Secret 只读授权与外部只读目录，是 KRun 能力的实质缺口，不只是平台声明问题。
- **客体运行清单覆盖不全**：`krun/runtime/base/runtime-catalog.cjs:21-36` 只采集 bubblewrap／glibc／node／openssl 四项；架构基线里写了 Python 3.11（`krun/docs/architecture.md:148`），但生产清单未采集其身份。
- **上游 ABI 断裂风险**：libkrun 官方说明 `main` 已是 2.0，**不向后兼容 1.x API/ABI**，且 2.0 仍在变动，生产应使用 `stable-*` 分支。kbox 当前钉的是 `libkrunfw v5.3.0` 的 `boxlite-ai` fork 预编译包 + 固定 SHA256（`libksys/build.rs:20-37`）、`libkrun.1.17.0.dylib`（`scripts/package-macos-arm64.sh:41-62`）。**任何上游升级都是一次全平台 breaking change，需要独立 ADR 与四目标重验。**
- Windows WHPX 上游限制：单 vCPU、无 libkrunfw-windows、无 virtio-gpu、x86_64 only（`libkrun/README.md:408-413`）；Windows interactive stdin 未实现，通道仅 Unix（`docs/evidence/box/2026-11-06-interactive-stdin/OUTCOME.md:41`）。
- 已有原生证据里反复出现宿主专属坑：ASCII-only kernel cmdline 路径编码、macOS `SUN_LEN` Unix socket 路径限制、macOS 关停 hangup。

---

## 4. 生态方案评估（为什么不换后端）

| 方案 | 能带来什么 | 为什么不作为 kbox 产品路径 |
| --- | --- | --- |
| Apple `container` 1.0 / Virtualization.framework（2026-06 稳定，macOS 26+ / Apple Silicon） | 每容器一 VM、亚秒启动、官方维护、Rosetta 跑 amd64 镜像 | 框架**拥有 vCPU／内存／virtio 设备／bootloader**，kbox 无法保证跨宿主一致的设备模型与凭据、PTY、generation 语义；且限 macOS 26 + Apple Silicon，反而收窄平台。可作**开发态第二驱动**用于加速证据收集，但不得用于产品 claim |
| Firecracker / Cloud Hypervisor | Linux 上成熟 | Firecracker 明确不做 macOS（#2845 `not_planned`）；`event-manager` 依赖 `vmm_sys_util::epoll` 无法在 macOS 编译。libkrun 本身已是它的可移植分支 |
| container2wasm | 把 OCI 镜像变成单文件 `.wasm`，浏览器可跑 | CPU 模拟（Bochs／TinyEMU）、实验性、CNCF Sandbox、默认 1 核 128 MB、网络需额外桥接。对长程 agent 与独立内核隔离都是倒退 |
| WASM 组件 / Spin / Wasmtime | 能力侧细粒度沙箱、跨架构制品 | 无 fork／exec／PTY／信号／native addon；`krun/docs/architecture.md:30` 的原生等价红线禁止用转译到 Wasm 完成接入。**只放能力侧**（MCP／工具／策略钩子） |
| Rosetta 2 跑 linux/amd64 | 解「arm64 宿主跑 x86_64 包」 | 技术可行但成本高：smolvm PR #580 显示 Rosetta 的 Linux binary 用未公开 ioctl 校验是否运行在 Virtualization.framework，libkrun 用 HVF 会失败，需要 ptrace wrapper + binfmt_misc 注册 + guest rootfs 重建。**纳入可选 profile，不进主路径** |
| 共享宿主内核容器（`native-linux`） | Linux 上省掉 VM | Box 产品明确排除（`README.md:95`）；它只能作为 OCI Runtime 的低级 driver，不能成为 Box 的隔离边界 |

**结论：后端不换。** 优化空间在宿主适配层、客体 ABI 解耦与证据链工业化。

---

## 5. 优化方案（按代价／收益分层）

### P0 — 不改执行后端，纯收益，低风险

**P0-1 单一机器可读支持矩阵 + 口径分层**
- 建立 `support-matrix.json`（或 KCL 侧单一数据源）：`{host_os, host_arch, backend, artifact, readiness: probe-only|experimental|supported, evidence[]}`。
- 明确三档口径：`upstream_capability`（libkrun／FFI）、`ffi_binding`（libksys）、`product_readiness`（kbox）。README、feature-coverage、release notes、install.sh 全部从该矩阵生成。
- 验收：`doctor` 输出、README 表、`dist/RELEASE_NOTES.md` 与该矩阵一致；CI 有一致性检查；不产生任何新的支持声明。
- 状态（2026-11-12）：已落地 `docs/architecture/support-matrix.json` 与 `scripts/check-support-matrix.py`。当前校验能力：四目标与 `runtime_assets.rs::SUPPORTED_TARGETS` 一致、证据与来源路径存在、readiness 闭集、禁止无证据或无预编译包的 `supported`。矩阵已区分 `product_readiness` / `upstream_capability` / `ffi_binding` 三档口径，并显式记录被拒绝的宿主组合与已知能力缺口。**README、feature-coverage、RELEASE_NOTES、install.sh 仍为手工维护**，生成化与 CI 接线是这一步的剩余工作；在此之前矩阵状态为 `machine-readable draft`，不作为新的产品支持声明。

**P0-2 解除宿主／客体架构硬绑（最高性价比代码改动）**
- `krun/src/package.rs:144-150` 改为从引擎 `Discovery`／`Diagnostics` 或 `ResolvedPlan.runtime.target` 取客体 arch，宿主 arch 只用于选择宿主侧制品。
- `Diagnostics` 增加独立的 `guest_target` 字段（可选扩展，保持 legacy wire 形状），让 `doctor` 能区分「宿主平台」与「客体 ABI」。
- 验收：新增 `linux/x86_64` 包在 macOS/arm64 宿主上的**明确拒绝或明确接受**用例（二选一，但必须可解释），且现有 14 个 `linux/aarch64` 包的凭据、初始化、EOF、清理回归全部保持（`krun/docs/evidence/2026-11-11-native-bindings-compatibility/OUTCOME.md` 的既有结果不得回归）。

**P0-3 冻结最小合同集 + 补齐客体运行清单**
- 按 `krun/docs/implementation-plan.md:47` 落地：M0／M1 涉及的 wire schema 由一份生成源产出，两仓共用 conformance fixture（含拒绝用例），CI 双向校验。
- `runtime-catalog.cjs` 补采 Python（以及任何被 `runtimeRequirements` 引用但当前未采集的组件），并补一组「组件缺失 → `RUNTIME_REQUIREMENTS_UNMET`」的拒绝测试。

**P0-4 上游升级 ADR 与供应链清单**
- 记录当前 `libkrun 1.17.0` + `libkrunfw v5.3.0 (boxlite-ai fork)` + 各 SHA256 的来源、fork 差异与升级触发条件；明确 2.0 不兼容，任何升级需四目标重验与全量证据。
- 这不需要改代码，但能防止一次意外的全平台 breaking change。

### P1 — 抽象层收敛，中成本、高杠杆

**P1-1 抽 `platform-capabilities` 共享 crate**
- 借鉴两个已被验证的现成模式：`ksandbox/src/policy/capabilities.rs` 的 `BackendCapabilities`（每字段 `cfg!` 声明、`validate_for_backend` 构造期 fail-closed、`capability_report()` 上报 `unavailable`），以及 inkstone `internal/platformruntime/runtime.go` 的 `Descriptor{Platform, Architecture, Capabilities}` 闭集校验 + `unavailable` 桩（无 nil、无隐式降级）。
- 目标是把**能力判定**从散落的 `cfg` 收敛到一处，**不是**抽象掉 hypervisor API。先覆盖 PTY、只读挂载、目录授权、进程树终止、凭据材料化、socket 类型这六类高频分叉。
- 引入 `target_family` 粗粒度分层，减少 `cfg(unix)` × `cfg(target_os)` 的组合爆炸。
- 验收门禁：新增平台时，能力缺口以 `unavailable` 显式上报并在派发前拒绝，不允许静默降级；现有 3 套平台行为与证据不得回归。

**P1-2 让 kbox 真正接上 ksandbox**
- `docs/architecture/sandbox-control.md:49,75,176-181` 已把 ksandbox 定为默认命令执行边界，但工作区 7 个项目的 `Cargo.toml`／`Cargo.lock`／源码里 `ksandbox` 依赖数均为 **0**；ksandbox 自身 `publish = false`。
- 这是「文档声称已接线、代码零依赖」的架构缺口。接上它，可把 Seatbelt／bwrap+seccomp／AppContainer+Job+ACL 这套原语从工作区 4–5 份独立实现（ksandbox、codex-acp `sandboxing` + `windows-sandbox-rs`、dsh-acp Landlock、inkstone `sandbox/darwin` + `windowsadapter`、kbox MicroVM）收敛到一份共同底座。
- 前置：决定发布方式（workspace path 依赖 / git tag / 内部 registry），并补一篇 ADR 说明「单一平台能力层」的所有权。

**P1-3 消除整棵子树副本**
- `kpower/src` 与 `kstack/crates/contexts/model-supply/power/src` 有 **371 个完全相同的相对路径**，两份 220 行支持矩阵逐字相同。合并为单一 crate 是工作区里最大、最确定的重复成本，且与 kbox 兼容性合同（`kbox-contract` 被两处分别以 `path` + `version "=0.8.0"` 引用）直接相关。

### P2 — 架构策略：一个客体 ABI，宿主只提供 VM

- **目标**：固定 Linux 底座 + OCI digest + `runtime-v1.json` 作为唯一客体 ABI；宿主侧只负责「提供一台独立内核 VM」，不参与客体 ABI 推断。跨架构由 `ResolvedPlan.runtime.target` 与包 `target` 显式协商，冲突时在派发前明确拒绝。
- **好处**：新增宿主平台不再需要重建三套 Harness 包、不重复处理 Node／glibc／OpenSSL 版本矩阵；平台差异收敛到 VM 启动层与 §2.1 的四个 runtime assets。
- **暂不做**：Rosetta 跨架构 profile（见 §4，需 ptrace shim + rootfs 重建 + 独立证据），以及任何把宿主能力（GPU、系统 API）透传进 VM 的方案——`krun/docs/architecture.md:134` 已规定宿主能力走宿主服务。
- **风险**：这条会触及 `kbox.harness-package.v1` 的 `target` 语义与 `native-guest.v1` 的 `semantics_profile_digest`，属于合同变更，必须走版本化扩展而非原地改字段。

### P3 — 证据链工业化（真正的瓶颈）

- 把 `crates/oci-runtime/.github/scripts/` 下约 50 个分散的平台脚本，收敛成**声明式 `qualification-matrix.yaml`**：目标平台 × 驱动 × 用例 × 证据路径 × claim 等级。
- 每个驱动一个可复现 runner，输出结构化结果；`docs/evidence/<domain>/<date-slug>/OUTCOME.md` 与该结果绑定。
- **claim 只能从证据自动汇总生成**：`probe-only → experimental → supported` 的提升由机器判定，把 `ROADMAP.md:158-170` 的 `Existing-host greening does not promote readiness` 变成硬门禁；`Explicitly not claimed` 段由矩阵中未覆盖项生成。
- 把 ksandbox `docs/E2E_TEST_MATRIX.md` 的 claim↔evidence 汇总表提升为工作区级模板。
- 解除 `krun/docs/evidence/2026-11-11-doctor-native/OUTCOME.md:63` 记录的测试脚本对配套 KBox 开发树的依赖，让两仓证据真正可独立复现。
- **注意：这是提高证据密度与可复现性，不是放宽证据标准。** `AGENTS.md:27` 的「不删除证据」与 `:84` 的「交叉编译不算数」保持不变；`B2 self-certify Forbidden` 也保持。

### 顺序建议

`P0-1 → P0-2 → P0-3 → P0-4`（一个迭代内可完成，不触碰执行后端）→ `P1-1`（先在一个 crate 内试点能力描述，验证不回归）→ `P3`（与 fresh-host 硬件到位并行，因为它是纯流程改造）→ `P1-2 / P1-3`（跨仓协调，需所有权决策）→ `P2`（合同变更，最后做）。

---

## 6. 不改动的红线

以下为现有合同原文或等价约束，任何优化都不得绕过：

- **不收窄通用 OCI 合同**：`kbox/docs/architecture/krun.md:15`、`krun/docs/architecture.md:17` —— 普通 OCI 路径不要求 ACP／Harness／KRun；新增 profile 不能成为全局准入条件。每个引擎变更批次必须跑独立 OCI 回归。
- **不建第二套执行状态机**：`kbox/docs/architecture/krun.md:35`、`krun/docs/architecture.md:192`、`AGENTS.md:22` —— Box ExecutionManager 是唯一权威，KRun 只记录 desired state 与回执。
- **原生等价**：`krun/docs/architecture.md:30`、`implementation-plan.md:30,141` —— 不得用转译到 Wasm、裁剪工具、静默禁用能力完成接入；未验证的能力差异不能改称「不支持」后算通过。
- **证据纪律**：`AGENTS.md:27,84,96,112`、`constitution.md:47,85-87` —— 交叉编译／mock／ignored test／启动成功都不等于运行时支持；证据不可删除；支持矩阵只含原生实测配置。
- **默认构建链**：`cargo build --locked`、`release-dist`、`scripts/package-macos-arm64.sh`、四目标 CI 矩阵、`build.rs` 的默认路径不因本提案改动；任何实验性后端走独立 target／独立 runner。
- **上游 submodule**：`libkrun` / `libkrunfw` 的改动必须在各自上游仓库完成并测试，再更新 gitlink（`AGENTS.md:24-25`）。

---

## 7. 需要拍板的问题

1. **是否真的要扩平台？** 如果答案是「先稳住 macOS arm64 + Linux/aarch64 客体」，那么 P3（证据链）和 P0（口径／解绑）就是全部工作，P2 可以推迟；如果要争取 WHPX／KVM `supported`，§2.4 的 fresh-host 硬件从哪来是前置条件，代码改完也过不了门。
2. **ksandbox 是否作为工作区共同底座？** 涉及所有权、发布方式与 codex-acp／dsh-acp／inkstone 三套既有沙箱的迁移节奏，需要一篇 ADR，不能由 kbox 单方面决定。
3. **跨架构是否进入产品范围？** 若要支持 macOS/arm64 宿主跑 `linux/x86_64` 包，需接受 Rosetta profile 的 ptrace shim 维护成本与独立证据链；若不进入，P0-2 的验收就应固定为「明确拒绝」。
4. **合同冻结时机**：`krun/schemas/README.md:18` 说 M0／M1 后冻结，但 KRun 实施状态显示 M0–M4 仍在进行。是否先冻结一个最小子集供两仓共用，需要两仓同步决定。

---

## 8. 主要参考

| 主题 | 位置 |
| --- | --- |
| 平台表与就绪等级 | `README.md:87-95`、`crates/oci-runtime/README.md:65-71,867-885`、`crates/oci-runtime/docs/feature-coverage.md:9-10,67-77` |
| 发布覆盖 | `dist/RELEASE_NOTES.md:36,46`、`scripts/package-macos-arm64.sh` |
| 平台准入 | `crates/box/runtime/src/host_check.rs:39-116`、`crates/oci-runtime/crates/runtime/src/platform/mod.rs:1-64` |
| 平台构建 | `crates/box/deps/libksys/build.rs:19-37,94-121`、`.cargo/config.toml:1-23`、`Cargo.toml:190-202` |
| 引擎能力清单 | `crates/box/runtime/src/engine/controller.rs:109-156`、`specs/001-krun-engine/contracts/engine-v1.md:25-49` |
| 客体协议与制品合同 | `krun/docs/architecture.md:76-106`、`krun/docs/capability-packages.md`、`krun/docs/native-bindings.md`、`krun/runtime/base/README.md` |
| 宿主／客体架构绑定 | `krun/src/package.rs:144-150`、`krun/schemas/resolved-plan.v1.json:469-487` |
| 证据纪律 | `AGENTS.md:26-27,84,96,105,112`、`docs/development-standards.md:77,137`、`krun/docs/implementation-plan.md:158` |
| 平台能力模式 | `ksandbox/src/policy/capabilities.rs`、`ksandbox/docs/E2E_TEST_MATRIX.md`、`inkstone/core/internal/platformruntime/runtime.go` |
| 运行器决定 | `krun/docs/decisions/0001-mcp-service-runners.md` |

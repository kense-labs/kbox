# Windows ARM64 成为必须支持平台：影响分析与落地规划

状态：**原始规划 2026-11-12；API、架构准入与构建检查已于 2026-11-14 更新**。
当前完整栈结论为 **当前依赖不支持**；Windows ARM64 运行仍 **缺少原生验证**。
最新交付和命令见 [F 证据](../evidence/runtime/20261114-windows-arm64/OUTCOME.md) 与
[有界 spec](../../specs/005-windows-arm64/spec.md)。本文其他硬件/云平台规划保留为
2026-11-12 的历史调研，不作为本轮重新验证的事实或运行结论。

证据边界：本次为只读调研（源码、CI、制品清单、上游文档）与公开资料核对，**没有在真实 Windows ARM64 主机上运行任何 WHPX / libkrun / kbox 验收**。按 `AGENTS.md:84,96,105,112`，交叉编译、静态检查、mock 与启动成功都不构成运行时证据；任何 `supported` 声明都必须有该目标上的原生 `docs/evidence/<domain>/<date-slug>/OUTCOME.md`。

关联：[跨平台与兼容性优化](cross-platform-compatibility.md) · [支持矩阵](support-matrix.json) · [矩阵校验](../../scripts/check-support-matrix.py)

---

## 0. 结论摘要

1. **平台层面可行，但不是「补一个 target」。** Windows ARM64 宿主 + Linux/aarch64 客体是**同构**组合，不需要 Rosetta / binfmt / 跨架构模拟，理论上比 x86_64 Windows 更干净。真正的代价在 libkrun 上游：它的 Windows 后端是纯 x86_64 实现（X64 寄存器、APIC、CPUID、MSR），`libkrunfw-win` 只有 x86_64 内核，Windows 三件套 DLL 也只有 x64。这是一次**新 VMM 后端**工程，不是 kbox 侧加 cfg。
2. **已核实：公开 ARM64 WHP API 存在。** Microsoft 已发布 ARM64 寄存器、GIC 与 `Arm64Support` 定义，QEMU ARM 后端直接调用 `WHvCreatePartition`、`WHvSetupPartition` 和 `WHvRunVirtualProcessor`。Vid 是 WHP 在 Windows 内部的实现层，不代表 QEMU 绕过公开 WHv API。当前缺口是所选 libkrun 的 ARM64 Windows VMM/固件/ABI 资产及真实宿主验证。见 [官方能力合同](https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/funcs/whvgetcapability) 与 [QEMU ARM 源码](https://github.com/qemu/qemu/blob/master/target/arm/whpx/whpx-all.c)。
3. **最重要的排序建议：先把 Windows x86_64 推出 `probe-only`。** 目前 WHPX x86_64 自身仍 `probe-only`，fresh-host 门在 `crates/oci-runtime/README.md:869-872` 全部 `Blocked`。在一个未资格化的兄弟平台上叠加新架构，会让新平台的证据链失去参照基线。S1 必须先把 WHPX 家族的共同基线（fresh-host 矩阵、session owner、handle reclamation、live reattach）做实。
4. **CI 硬件不再是硬阻塞，但 WHPX 有明确的能力门槛：那个 Windows ARM64 必须自己能启动 Hyper-V，即占有 EL2。** 「必须真实物理设备」是过强说法（修正见 §2.4）。WHPX 不是「有 Windows 就行」，它要求宿主 Windows 里 Hyper-V hypervisor 处于运行态；若 Windows 本身是 guest，L0 必须把虚拟化扩展下放给它。满足这一条的现实环境是**物理 Windows on ARM 设备**，或 **ARM64 裸金属服务器上自行安装 Windows 11 ARM**（Linaro 已有在 aarch64 KVM 裸金属上跑 WoA guest 的公开方案，但许可/分发、virtio 驱动、TPM 需自行解决，且不存在 Windows Server for ARM）。GitHub 托管 `windows-11-arm` runner 已 GA（公开仓库免费，2026-09 起默认 VS2026 镜像），底层是 Azure Ampere Altra / Cobalt ARM VM；2026-09 公开资料显示 **Azure / GCP / AWS 的 ARM 实例都不提供嵌套虚拟化**（Microsoft 文档中的 Azure 嵌套虚拟化支持指 x86_64 系列，勿混用）。本机 macOS（M5 Max、macOS 26.6.2、`kern.hv_support=1`）的 OrbStack 只支持 Linux 容器与 Linux 发行版 VM；改用 Parallels / VMware Fusion / UTM 能装起 Windows 11 ARM，但 Apple 只对 **Linux guest** 暴露嵌套虚拟化（M3+ 芯片 + macOS 15+：Tart `--nested` / Lima `vmType: vz, nestedVirtualization: true`），Windows guest 拿不到；且 ARM64 嵌套 KVM 目前**不保证递归嵌套**（kvmtool 2026-03 主线补丁注明 non-VHE guest 会失去 recursive nested virtualisation），所以「macOS → Linux KVM guest → Windows ARM guest → WHPX」三层链也不成立。`kern.hv_support=1` 只说明 macOS 自己能当宿主。结论：**编译与打包可以上云或用本机 Linux VM；WHPX 相关验证延后到一个让 Windows ARM 占有 EL2 的环境。S0 中不依赖 WHPX 的部分现在就能做。**
5. **Prism（x64 模拟）不是合规路径。** 它只翻译用户态指令，内核驱动必须原生 ARM64；虚拟化/底层工具属于官方列为高风险的类别。kbox 的 x64 三件套 DLL 依赖 WHPX 内核接口，模拟环境下不可用；即便「能跑」，按证据纪律也不能算 `supported`。

---

## 1. 先澄清「必须支持」的产品含义

这句话有三种差别极大的解读，决定了要不要做 S2–S4：

| 含义 | 用户实际得到什么 | 是否需要本地 MicroVM | 主要工作 |
| --- | --- | --- | --- |
| **H1 本地原生执行**：WoA 设备上 kbox 必须能直接启动独立内核 MicroVM | 完整离线能力、原生隔离边界 | **是** | 全部 S0–S5，含 libkrun aarch64 后端 |
| **H2 设备上可用**：WoA 用户必须能使用该产品，执行可放在别处 | 客户端 / 控制面在 WoA，VM 跑在远端 x86_64 或 Linux 主机 | 否 | 只需 ARM64 客户端构建 + 远端绑定 + 显式声明「本地执行不支持」 |
| **H3 兼容兜底**：现有 x64 包通过 Prism 能在 WoA 上跑起来 | 有限功能、模拟性能 | 否（且通常不可用） | 低，但不能声明支持 |

**这是第一个必须拍板的问题。** 如果是 H2，本规划 80% 的工作量可以取消，且能立刻交付；H3 按证据纪律不能算「支持」。下文按 **H1** 展开。

---

## 2. 事实校正：外部阻塞到底还在不在

### 2.1 仓库里的记载（截至 2026-11）

| 出处 | 原文要点 |
| --- | --- |
| `libkrun/docs/WINDOWS_ROADMAP.md:110-112,125` | 「跟踪 Microsoft WHPX ARM64 partition type 支持进度」，优先级低（外部依赖）；`x86_64 only`；文档最后更新 2026-03-10 |
| `libkrun/README.md:410,490` | Known limitations `x86_64 only (no ARM64/WHPX support on Windows)`；Roadmap 把 Windows ARM64 列为 medium-term，理由同上 |
| `crates/oci-runtime/docs/hardware-vm-platform-remainder.md:93` | 「客户机 ISA 是 x86_64。WHPX 不把 aarch64 算作合格主机」 |
| `crates/box/runtime/src/host_check.rs:73-77,207-211` | 两处运行时硬拒绝：`Kbox Box on Windows currently requires x86_64 for the WHPX backend.` |

### 2.2 2026 年的外部事实

- QEMU 11.0 `docs/system/whpx.rst`（2026-03 合入）：WHPX「enables QEMU with hardware acceleration on **both x86_64 and arm64** Windows machines」；实现在 `Vid` 之上，Vid 使用与 Linux `mshv` 驱动相同的 hypercalls。
- ARM64 最低版本：**Windows 11 24H2 + 2025 年 4 月可选更新或 5 月安全更新**；更早的 24H2 带的是 QEMU 不支持的预发布 WHPX API。
- ARM64 已知限制：**SVE / SME 不暴露**；非 Windows 客体建议 `virtio-gpu-pci`；嵌套虚拟化相关的 Hyper-V enlightenments 未暴露。
- libkrun 1.19.8（2026-11-06）仍标注 Windows x86_64 only，medium-term 才考虑 ARM64 —— 即**上游至今没有实现**。
- GitHub 托管 runner：`windows-11-arm` / `windows-11-vs2026-arm`（4 vCPU / 16 GB，公开仓库免费；私有仓库可用但计费）。
- Windows 11 ARM 的 Prism：只翻译用户态 x86/x64，内核驱动必须 ARM64 原生；虚拟化工具属于官方明确要求实测的类别。

### 2.3 正确的定性

**按平台接口、当前依赖和原生验证分别判定：**

- Windows 11 24H2+ 的 ARM64 主机上，通过 WHPX/Vid 路径跑硬件加速的 **aarch64 guest 已被 QEMU 证明可行**；
- 公开 WHv API 已提供 ARM64/GIC 类型；当前所选 libkrun 的 Windows 实现仍使用 X64 vCPU/APIC/CPUID/MSR，Windows DLL 与固件仍是 x64；
- S0 的剩余原生步骤是验证真实宿主的 HypervisorPresent、Arm64Support、partition/vCPU 与清理；平台 API 可行性已有官方依据，不再保留「必须绕过公开 API」的假设。当前产品入口保持 Unsupported，待原生依赖和证据齐备后再推进。

---

### 2.4 WHPX 的充要条件（对「必须真机」的措辞修正）

「WHPX 必须真实 Windows ARM 设备」是**过强**的说法。准确判据是：**那个 Windows ARM64 实例能否自己启动 Hyper-V hypervisor**。

1. Windows 11 ARM64，24H2 + 2025/04 可选更新或 05 安全更新（WHPX API 门槛）；
2. `hypervisorlaunchtype Auto`、`HypervisorPlatform` 功能启用，`msinfo32` 显示检测到 hypervisor；
3. 若 Windows 本身是 VM，L0 必须向它暴露虚拟化扩展——ARM 上是 **EL2 下放**，等价于 x86 的 `Set-VMProcessor -ExposeVirtualizationExtensions`；
4. 之后才能调 `WHvGetCapability` / `WHvCreatePartition`，创建 aarch64 vCPU 与 guest。

由此得到环境清单：

| 环境 | Windows ARM 是否占有 EL2 | 结论 |
| --- | --- | --- |
| 物理 Windows on ARM（Snapdragon X Elite 等） | 是，直接跑在硬件上 | ✅ 最现实 |
| ARM64 裸金属服务器 + 自装 Windows 11 ARM（aarch64 KVM + swtpm + AAVMF） | 是 | ⚠️ 技术可行（Linaro 有公开方案），但许可/分发、virtio 驱动、TPM 待解；无 Windows Server for ARM |
| Azure / GitHub 托管的 ARM Windows VM | 否，ARM 实例不暴露嵌套 | ❌ |
| Mac 上 Parallels / VMware / UTM 的 Windows ARM guest | 否，Apple 只对 Linux guest 暴露嵌套 | ❌ |
| macOS → Linux guest with nested KVM → Windows ARM guest（三层递归） | 否，ARM nested KVM 不保证递归 | ❌ 不可作为依据 |

**S0 可以拆开，不必整体等真机。** 不依赖 WHPX 的部分现在就能推进：上游 libkrun / libkrunfw 的代码与 API 契约静态核查、`aarch64-pc-windows-msvc` 交叉编译与制品摘要、第五个 runtime bundle 的数据合同设计、CI 编译矩阵与打包脚本、支持矩阵与证据模板。只有「WHPX API 探测 + 启动 aarch64 guest + fresh-host 原生矩阵」需要上述 EL2 环境。

> 保留 S0-2 实测的价值：如果 Microsoft 未来为 ARM Windows VM 开放嵌套虚拟化，或 ARM 递归嵌套成熟，上表结论会变；届时以实测结果更新，不要靠推测。

## 3. kbox 现状阻塞面

### 3.1 硬阻塞（必须改实现或等上游）

| 组 | 位置 | 内容 |
| --- | --- | --- |
| 运行时拒绝 | `crates/box/runtime/src/host_check.rs:73-77,207-211` | 两处 `cfg(not(target_arch="x86_64"))` 拒绝；`:266-294` 测试断言 `x86_64` 也需同步 |
| 架构白名单 | `crates/oci-runtime/crates/runtime/src/platform/mod.rs:13-14` | `whpx_driver_capability` 只在 `windows+x86_64` 导出；`features()` 在 aarch64-windows 会落到 `windows::features()` 误报可用 |
| 依赖注入 | `crates/oci-runtime/crates/kbox/Cargo.toml:23` | `[target.'cfg(all(windows, target_arch="x86_64"))'.dependencies]` 不命中 → `klibksys` / `windows-sys` / `sha2` 缺失 |
| cfg 扩散 | `crates/` 下 **60+ 文件、数百处** `cfg(all(windows, x86_64))` | 含整文件级门 `windows_session_owner.rs:17`、`crates/oci-runtime/crates/kbox/src/lib.rs:98-107` 六个 Windows 模块、`main.rs:45-46,61`；另有 6+ 处兜底文案写死「Windows x86_64/WHPX」 |
| 上游 VMM | `libkrun/src/vmm/src/windows/whpx_vcpu.rs:59-75,108-115,410-417` | X64 寄存器 / APIC / CPUID / MSR；`libkrun/src/vmm/src/lib.rs:81,90,102,116` 的 x86_64 布局；aarch64 运行循环仅在 `docs/plans/2026-02-28-x86_64-e2e-flow-design.md:69` 是 TODO |
| 客体内核 | `libkrun/src/libkrunfw-win/README.md:4,13,34,43-47` | 只接受 x86_64 ELF vmlinux；config fragment 全 x86_64；`build.rs:61,203` 同 |
| 制品 | `crates/box/deps/libksys/build.rs:52-67,1113,1120` | Windows 三件套的 4 个 SHA256 常量、`vendor/kwindows-x64.tar.xz` 写死 x64；`libkrun/krun-sys-windows/prebuilt/` 只有 `x86_64-pc-windows-msvc/`；`crates/oci-runtime/crates/kbox/runtime/` 只有 4 个平台归档 |
| 四目标合同 | `crates/oci-runtime/crates/kbox/src/runtime_assets.rs:9-14,115-167,199,256-261,335-339` | `SUPPORTED_TARGETS` 四条 + manifest 数量双向校验 + Windows 固定 3 文件 + 测试断言 4；`runtime-assets.json` 无 windows/aarch64 bundle，且 Windows bundle 的 `guest_load_address` 是 x86 的 16 MiB（aarch64 是 2 GiB） |
| 矩阵合同 | `docs/architecture/support-matrix.json:130-133` | `windows-aarch64` 当前列在 `rejected`；`scripts/check-support-matrix.py:101-106` 会把「加第 5 个目标」判为 drift |
| 证据 | `docs/evidence/` | **零** Windows ARM64 原生结果；连 Windows x86_64 都只是 existing-host、`probe-only` |

### 3.2 已经预留 / 只需补配置

- `.cargo/config.toml:16-17` **已有** `[target.aarch64-pc-windows-msvc]` rustflags（`force-unwind-tables` + `crt-static`）。
- `libksys/build.rs:1164-1165`、`krun-sys-windows/build.rs:19-22`、`crates/oci-runtime/crates/kbox/build.rs:24-30,46-48` 均按 `CARGO_CFG_TARGET_ARCH` 动态拼 triple，**架构无关**；`crates/box/shim/build.rs:16-17,63-96` 纯 `cfg(windows)`。
- 根 `Cargo.toml` 无 target 段，通用 `cfg(windows)` 依赖段无需改。
- 需补：`release.yml:50-64` 第 5 条矩阵 + `:244` 的 `-eq 5` 计数；`ci.yml:17,77`、`windows_ci.yml` 全文件的 `x86_64-pc-windows-msvc`；7 个 `windows-whpx-*.ps1` 里 `architecture -ne 'x86_64'` 断言；`build-windows.ps1:4`；`install.sh:24-35` 的 OS / asset 命名；新增 Windows 打包脚本（现在只有 `package-macos-arm64.sh`）；README / feature-coverage / RELEASE_NOTES 文案。

---

## 4. 路线评估

| 路线 | 做法 | 优点 | 代价 / 风险 | 判断 |
| --- | --- | --- | --- | --- |
| **A. 原生 aarch64 WHPX 后端**（推荐 H1） | fork libkrun，新增 aarch64 vCPU/GIC/系统寄存器路径 + libkrunfw-win aarch64 内核 + 全套 DLL；kbox 侧解绑 | 保持单一设备模型与跨宿主语义一致，符合 `AGENTS.md:78` | 上游工作量最大；需长期维护 fork；libkrun 2.0 又是不兼容 1.x ABI 的重构 | **正解**，但必须 S0 先证伪 API 前提 |
| **B. 换后端：QEMU WHPX** | Windows ARM64 上用 QEMU（已支持 arm64 WHPX）作为执行后端，libkrun 只用于其它宿主 | 外部能力现成，落地快 | 引入第二套设备模型 / 生命周期，违反「不建第二套状态权威」；QEMU 体积与启动成本远超 microVM；跨宿主一致性丢失 | 仅作 S0 的**验证工具**与短期过渡，不作为产品路径 |
| **C. Prism 跑 x64 包** | 依赖 Windows x64 模拟 | 零上游改动 | 内核驱动不模拟；虚拟化类高风险；性能不可控；**按证据纪律不能算 supported** | 排除为合规路径 |
| **D. 远端执行**（推荐 H2） | WoA 只跑客户端 / 控制面，MicroVM 在远端 x86_64 或 Linux 主机 | 成本最低，立刻可交付 | 不是「本地运行时支持」；需显式声明能力边界与网络依赖 | 若产品含义是 H2，选它 |

**排序观点**：先做 H1/H2 判定，再走 A；B 只用于 S0 验证；C 不进产品；D 是 H2 的答案。

---

## 5. 分阶段规划

### S0 — 可行性 spike（决定性，先于任何产品承诺）

| 实验 | 判据 | 失败时 |
| --- | --- | --- |
| S0-1 在 Win11 24H2 更新后或更新版本的原生 ARM64 宿主上，用当前 SDK 查询 HypervisorPresent、Arm64Support，并保留 partition/vCPU 与精确清理结果 | API、缓冲区长度、真实架构与清理均需通过；QEMU 可作独立平台对照，不能替代 kbox 验收 | 缺能力位时记录 OS/功能/虚拟化条件；不切换后端，不提升 readiness |
| S0-2 在 `windows-11-arm` 托管 runner 上实测 WHPX 是否可用（HypervisorPlatform 功能 + 能否 `WHvCreatePartition`） | 已知 runner 底层为 Azure ARM VM，且主流云 ARM 实例不开嵌套虚拟化 → **预期只能编译、不能跑 WHPX**；此步用于把「预期」变成有记录的实测结果 | 原生验收改由物理 Windows on ARM 设备或裸金属实例承担，不等待 runner |
| S0-3 确认 libkrun 1.x 与 2.0 的 fork 基线（当前钉 `libkrun.1.17.0` + `boxlite-ai/libkrunfw v5.3.0`） | 明确在 `stable-*` 分支还是 2.0 上做 aarch64 后端，避免与全平台 ABI 升级叠加 | 拆成两个独立变更 |

产出：`docs/evidence/engine/<date>-windows-arm64-feasibility/OUTCOME.md`，含实际命令、Windows 版本、API 返回值、QEMU 对照结果与 `Explicitly not claimed`。

#### S0 候选环境能力对照

| 环境 | 能否跑 Windows ARM | 能否跑 WHPX | 可用于 |
| --- | --- | --- | --- |
| 本机 OrbStack 2.2.5（macOS 26.6.2 / M5 Max） | 否，只支持 16 种 Linux 发行版、arm64 / amd64 | 否 | Linux/aarch64 客体侧交叉编译、guest rootfs / init、制品校验；现有 `kstack-drain-arm`（Debian bookworm arm64）可直接复用 |
| 本机 Parallels / VMware Fusion / UTM / VirtualBox 7.2+ | 是（Windows 11 ARM64） | **否**，无嵌套虚拟化，WSL2 / Hyper-V / VBS 不可用 | Windows 侧应用兼容性与非虚拟化行为观察，不能作为 WHPX 证据 |
| 项目现有远程 Windows（UU远程 + VirtualBox + MSVC 打包链路） | 现有文档未标注架构，按 VirtualBox / MSVC 用途看是常规 x86 Windows | 否（且非 ARM 宿主） | `aarch64-pc-windows-msvc` 交叉构建、安装包打包与 SHA256 证据 |
| GitHub `windows-11-arm` 托管 runner / Azure Windows on Arm VM | 是 | 预期否（云 ARM VM 不开嵌套虚拟化），由 S0-2 实测确认 | CI 编译、单元测试、打包；不作为原生 WHPX 证据 |
| 物理 Windows on ARM 设备（Snapdragon X Elite 等） | 是 | **是**，Windows 直接占有 EL2 | S0-1 API spike、S2 上游原生测试、S5 fresh-host 证据 |
| ARM64 裸金属 + 自装 Windows 11 ARM（aarch64 KVM + swtpm + AAVMF） | 是（技术上） | 是 | 备选；许可/分发、virtio 驱动、TPM 待解；无 Windows Server for ARM |
| macOS M3+ → Linux guest with nested KVM → Windows ARM guest（三层递归） | 理论 | **否**，ARM 递归嵌套不保证 | 不可作为依据；见 §2.4 |

> 注意：Windows 11 ARM 的 24H2 + 2025 年 4/5 月更新是 **WHPX API** 的门槛；Mac 上的 VM 即使满足该版本，仍受嵌套虚拟化限制。
>
> **关于「UTM 基于 QEMU，而 QEMU 11.0 支持 arm64 WHPX」的澄清**：两者宿主不同。`-accel whpx` 是 **Windows 宿主**上的加速后端（`WinHvPlatform` / `Vid`）；macOS 上 UTM 的 QEMU 走的是 Apple HVF，guest Windows 里也没有 Hyper-V hypervisor，所以 UTM 里能跑 QEMU，但**跑不了 WHPX**。ARM 的嵌套虚拟化需要把 EL2 下放给 guest；M3 及以后的芯片硬件上具备该能力，Apple 已开始对 **Linux guest** 暴露，但当前未通过 QEMU 路径到达 **Windows guest**（Hyper-V / WSL2 / Docker Desktop for Windows / VBS 均不可用）。
>
> UTM 装 Windows 11 ARM 的技术前提（仅作内部开发验证环境，非 Microsoft 授权路径）：Windows 11 **ARM64** ISO（当前 build，勿用 x64 或过旧镜像）、`Virtualize → Windows` 而非 Emulate、UEFI + Secure Boot + TPM 2.0（新版 UTM 模板自动配置）、≥2 vCPU / ≥8 GB RAM / ≥64 GB 磁盘、安装时挂 virtio 驱动 CD 并在 Setup 里「加载驱动」、装完补 SPICE / virtio guest tools。已知限制：无 3D 加速、Windows 11 24H2 与 UTM guest tools 有显卡驱动兼容问题需查当前文档、Windows 许可需单独解决。

### S1 — WHPX 家族基线（**必须早于 S2**）

把现有 Windows x86_64 从 `probe-only` 往上推：补 fresh-host attestation-bound 矩阵（`crates/oci-runtime/README.md:869` 现为 Blocked）、libkrunfw-windows 稳定分发、multi-vCPU/INIT-SIPI、session owner 与 handle reclamation 的原生证据。
门槛：WHPX x86_64 至少 `experimental`，且 fresh-host 证据闭环。**理由**：新架构要复用这套证据模板；没有基线就没有「等价」的参照。

### S2 — 上游 aarch64 后端（按 `AGENTS.md:24-25`，改动在 submodule 上游仓库完成、测试，再更新 gitlink）

- `libkrun/src/vmm/src/windows/`：aarch64 vCPU 运行循环、系统寄存器、GIC/中断路径，替换 X64 专用结构；`devices/src/legacy/`、`utils/` 相应分支。
- `libkrunfw-win`：接受 aarch64 ELF vmlinux 与对应 config fragment，保留可复现的来源与摘要。
- 上游测试：至少 `WHvCreatePartition` → vCPU run → guest 到 userspace 的最小闭环，以及重复生命周期的资源回收。
- 门槛：在上游 CI 的 ARM64 Windows 目标上原生通过，不是本地交叉编译。

### S3 — 制品与第五个 runtime bundle

- 产出 aarch64 `krun.dll` / `krun.lib` / `libkrunfw.dll`，固定 SHA256 与来源；`runtime/windows-aarch64/*.tar.xz`。
- `libksys/build.rs` 增加 aarch64 归档 URL / SHA256 分支（当前 `:19-37` 只有三套）；`runtime-assets.json` 加第 5 bundle，`kernel.guest_load_address` 用 aarch64 布局（对照 macos/linux-aarch64 的 2 GiB，不能复用 x86 的 16 MiB）。
- `runtime_assets.rs` 的 `SUPPORTED_TARGETS` 加第 5 项，同步 `:335-339` 的测试断言；`build.rs:28-30` 不再是「无 bundle 静默 return」。
- 门槛：`validate_runtime_manifest` 通过、摘要可复现、`scripts/check-support-matrix.py` 同步更新后通过。

### S4 — kbox 侧解绑 + CI / 分发

- 把 `host_check.rs` 两处拒绝改为能力探测；`platform/mod.rs:13-14` 白名单扩到 `any(x86_64, aarch64)`；`Cargo.toml:23` 依赖段同步。
- **系统性地收敛 60+ 文件的 `cfg(all(windows, x86_64))`**：借这次机会按 P1-1 的思路引入 `platform-capabilities` 风格的能力描述，而不是逐个补 `aarch64` 分支；否则第五个目标之后每加一个平台都要再改一遍。兜底文案里的三平台清单改为从支持矩阵生成。
- CI：`windows-11-arm` 编译 job（可上云）+ 自托管真机 WHPX job；`release.yml` 第 5 个目标与 `:244` 计数；7 个 ps1 的架构断言参数化；`install.sh` 覆盖 Windows；新增 Windows 打包脚本。
- `support-matrix.json`：把 `windows-aarch64` 从 `rejected` 迁到 `targets`，但 **readiness 只能是 `probe-only`**，且须带真实 evidence 路径；校验脚本对「supported 必须有预编译包 + 原生证据」的门禁不放宽。

### S5 — 证据链与 readiness 提升

- 一台 fresh-host ARM64 Windows + WHPX 上的完整矩阵：驱动操作、live session、取消与清理、重启恢复、soak、资源回收。
- `docs/evidence/<domain>/<date-slug>/OUTCOME.md` 含实际命令、计数、环境、日志与 `Explicitly not claimed`；claim↔evidence 汇总表。
- 按 `probe-only → experimental → supported` 逐级提升，任何一级都由矩阵自动生成，禁止手写；`AGENTS.md:84,112` 与 `specs/001-krun-engine/spec.md:44` 的「未原生测试的平台不新增支持声明」保持有效。

---

## 6. 验收门槛与不改动的红线

红线（沿用 [跨平台报告](cross-platform-compatibility.md) §6）：

- 不收窄通用 OCI 合同；Harness profile 不成为全局准入条件。
- 不建第二套执行状态机；Box ExecutionManager 仍是唯一权威。
- 原生等价；不得用模拟、裁剪能力或标 `unsupported` 完成验收。
- 交叉编译 / mock / ignored test / 启动成功不构成运行时证据；证据不可删除；支持矩阵只含原生实测配置。
- 默认构建链、四目标 CI 矩阵的现有路径不因本规划改动；第五个目标通过独立 runner 增量加入。
- `libkrun` / `libkrunfw` 改动必须在各自上游仓库完成并测试后再更新 gitlink。

新增的 Windows ARM64 专属门槛：

- 最低宿主版本固定为 **Windows 11 24H2 + 2025 年 4 月可选更新或 5 月安全更新**，并在支持矩阵与 `doctor` 中显式声明；更早版本明确拒绝而非静默降级。
- SVE / SME 不暴露这一限制写入能力描述；guest 不依赖这两类扩展。
- Prism / x64 模拟路径不得计入任何 readiness 声明。

---

## 7. 风险登记

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| 原生宿主未满足 ARM64 WHP 能力，或 SDK/bindings 与公开合同不匹配 | 本地编译不能代替宿主 API/vCPU 结果 | 按原生交接记录能力位、ABI、HRESULT 与清理；保留当前明确拒绝 |
| 托管 `windows-11-arm` runner / Azure ARM VM 不支持 WHPX（2026-09 社区确认 Azure / GCP / AWS ARM 实例均无嵌套虚拟化） | CI 只能编译与打包，无法产出原生证据 | S0-2 实测留证；原生验收直接规划物理 Windows on ARM 设备或裸金属实例，不抱侥幸 |
| libkrun 2.0 不兼容 1.x API/ABI | 与 aarch64 后端叠加成一次全平台 breaking change | S0-3 固定 fork 基线；升级单独排期、四目标重验 |
| Windows x86_64 自身 `probe-only`、fresh-host 门 Blocked | 新平台缺参照基线 | S1 先补齐，不跳过 |
| 60+ 文件的 cfg 逐个补 aarch64 | 后续每加平台重复一次 | S4 用能力描述层收敛，不复制分支 |
| 上游 fork 长期维护 | 与上游分叉 | 优先向上游提 PR；记录 fork 差异与同步节奏 |
| 「必须支持」被理解为 H2/H3 | 做了大量不必要工作 | §1 先拍板 |

---

## 8. 对现有文档与矩阵的影响（本次已同步）

- `docs/architecture/support-matrix.json`：新增 `required_platforms` 段声明产品约束与目标门槛；`rejected` 中的 `windows-aarch64` 条目改标为「当前实现的拒绝，产品要求已提升」，并指向本文。**`targets` 不变**——后端尚未实现，不能提前声明 `probe-only`；`supported` 仍为 0。
- `scripts/check-support-matrix.py`：注释里「four native targets」改为说明「四目标是当前代码事实，required 段不影响漂移校验」。
- `docs/architecture/cross-platform-compatibility.md`：原先「明确拒绝 Windows ARM64」改为指向本文的规划状态。

以上均为**规划与状态标注，不构成新的产品支持声明**。

---

## 9. 待拍板

1. **H1 / H2 / H3**：「必须支持」是本地原生 MicroVM，还是设备上可用即可？（决定是否需要 S2–S4）
2. 是否接受**长期维护 libkrun fork**，或优先推动上游实现并等待？
3. 是否投入一个**让 Windows ARM64 占有 EL2 的环境**——物理 Windows on ARM 设备，或 ARM64 裸金属上自装 Windows 11 ARM？按 §2.4，这是 WHPX 相关验证的能力门槛（不是「必须买笔记本」）；本机 OrbStack、Mac 上的 Windows ARM VM、托管 runner 与三层递归嵌套都不能替代。注意 **S0 里不依赖 WHPX 的部分可以现在就做**，不必整体卡在这笔投入上。
4. 是否同意 **S1 先行**（先让 Windows x86_64 脱 `probe-only`），这会推迟 ARM64 的上线时间但降低整体返工。
5. 「必须」是否有时间点？若有，S2 的上游依赖需要单独排期与并行推进。

---

## 10. 主要参考

| 主题 | 位置 |
| --- | --- |
| 硬拒绝 | `crates/box/runtime/src/host_check.rs:73-77,207-211` |
| 架构白名单 | `crates/oci-runtime/crates/runtime/src/platform/mod.rs:13-14`、`crates/oci-runtime/crates/kbox/Cargo.toml:23` |
| 四目标与制品 | `crates/oci-runtime/crates/kbox/src/runtime_assets.rs:9-14`、`runtime/runtime-assets.json`、`crates/box/deps/libksys/build.rs:52-67,1113-1120` |
| 上游限制 | `libkrun/README.md:408-413,490`、`libkrun/docs/WINDOWS_ROADMAP.md:110-125`、`libkrun/src/libkrunfw-win/README.md` |
| kbox 内部定性 | `crates/oci-runtime/docs/hardware-vm-platform-remainder.md:93`、`crates/oci-runtime/docs/feature-coverage.md:71` |
| 证据纪律 | `AGENTS.md:78,84,96,105,112`、`specs/001-krun-engine/spec.md:44` |
| 外部事实 | QEMU 11.0 `docs/system/whpx.rst`；GitHub-hosted runners reference（`windows-11-arm`）；Windows 11 on Arm Prism 限制说明 |

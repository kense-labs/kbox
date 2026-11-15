# Windows ARM64 feasibility findings

Verified 2026-11-14 against the selected native gitlinks and official sources.

## Conclusions

| Layer | Conclusion | Evidence / required work |
| --- | --- | --- |
| Windows native virtualization API | 可实现, inferred from official interfaces and QEMU's implementation; 缺少原生验证 in this session | Microsoft publishes ARM64 registers, GIC parameters and Arm64Support; QEMU's ARM backend calls the public WHv API |
| Selected libkrun Windows VMM | 当前依赖不支持 | `libkrun` 3c2889c76460d81e3fb16d6144580fd31eb1bae3 uses X64 vCPU registers, APIC, CPUID and MSRs; no Windows ARM vCPU/GIC implementation |
| Windows firmware and guest | 当前依赖不支持 | `libkrunfw-win` builds/checks x86_64 kernel inputs; selected Windows firmware contains x86_64 ELF headers. A Linux/aarch64 kernel/init/rootfs is required for the future ARM VMM |
| Host ABI and native runtime bundle | 当前依赖不支持 | Both current Windows krun.dll/libkrunfw.dll bundles have PE Machine 0x8664, not ARM64 0xaa64; x64 import archives cannot establish a native ARM64 ABI |
| Rust / MSVC toolchain | 可实现; local Rust compilation checked separately | aarch64-pc-windows-msvc is Tier 1 with host tools. Rust extern C uses the target's Microsoft ARM64 ABI. Native MSVC/SDK/CRT are needed for C dependencies and linking |
| Current complete kbox stack | 当前依赖不支持; 缺少原生验证 | No ARM Windows bundle or native eligible host evidence; diagnostics compilation is not VM support |

`libkrunfw` gitlink remains fc3ea06895301f1df6a274dad75fbd9317b3bfee.
No native repository or gitlink was changed. Unix ARM64 assets are not substituted
for a Windows ARM64 runtime, and x64 emulation is not treated as native execution.

## Official API and ABI facts

Microsoft specifies a two-step ARM check: HypervisorPresent, then
WHvCapabilityCodeFeatures / WHV_CAPABILITY_FEATURES.Arm64Support. Its published
header contains ARM64 register, exit-context and GIC definitions. See
[WHP API overview](https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/hypervisor-platform),
[capability contract](https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/funcs/whvgetcapability)
and [official header](https://github.com/MicrosoftDocs/Virtualization-Documentation/blob/main/virtualization/api/hypervisor-platform/headers/WinHvPlatformDefs.h).

QEMU lists ARM64 WHPX with Windows 11 24H2 plus the April 2025 optional or May
2025 security updates. Its ARM source checks Arm64Support and invokes
WHvCreatePartition, WHvSetupPartition and WHvRunVirtualProcessor. Vid describes
the Windows implementation underneath WHP, not a separate API that QEMU must
use instead of public WHv calls. See [QEMU WHPX](https://www.qemu.org/docs/master/system/whpx.html)
and [ARM source](https://github.com/qemu/qemu/blob/master/target/arm/whpx/whpx-all.c).

The [Rust target policy](https://doc.rust-lang.org/rustc/platform-support.html)
and [MSVC target contract](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
confirm the target and ABI. The latter explicitly does not promise MSVC cross
toolchains on a non-Windows build host. Microsoft's [ARM64 ABI](https://learn.microsoft.com/en-us/cpp/build/arm64-windows-abi-conventions)
also distinguishes ARM64EC interoperability from a native ARM64 ABI.

Native CPU detection uses [IsWow64Process2](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2),
not an environment variable or the program's compilation architecture. Its
Windows 10 1709 minimum precedes WHP's existing Windows 10 1803 baseline.

## Local source and artifact checks

- `libkrun/src/vmm/src/windows/whpx_vcpu.rs`: X64 register/emulator constants,
  APIC, CPUID and MSRs; `vstate.rs` owns the corresponding Windows state.
- `libkrun/src/libkrunfw-win/build.rs` / README: x86_64 Linux kernel and WHPX
  MMIO discovery constraints. Neither DLL nor Unix kernel reuse supplies an ARM VMM.
- `windows-sys` 0.59's Hypervisor bindings lack WHV_ARM64 register/GIC definitions;
  native port work must audit bindings against the current official SDK.
- `klibksys/build.rs` previously chose its branch using build-host cfg, admitted
  ARM Windows stubs, and could fall back to the x64 archive for another target.
  It now checks the Cargo target/ABI before stub or asset selection; only
  x86_64-pc-windows-msvc may consume the Windows bundle. Unix source builds
  require a matching build host; explicit stubs remain compilation-only.
- The SDK's one shared native-architecture admission is used by Box reservation,
  launch and direct shim entry, OCI discovery/smoke and native context creation.
  Unsupported CPUs/emulation use Unsupported; a failed native-CPU query uses
  Unavailable and does not fall through to another backend.

Source URL/content hashes and exact observed lines are in
`docs/evidence/runtime/20261114-windows-arm64/checks/official-source-observations.json`.
Archive hashes, host PE machines and embedded ELF observations are retained
there separately. Full implementation/test results belong in OUTCOME, not here.

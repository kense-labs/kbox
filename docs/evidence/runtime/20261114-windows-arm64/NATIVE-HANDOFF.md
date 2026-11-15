# Native Windows ARM64 handoff

This work has not run on Windows. Use a new native ARM64 developer terminal on
an eligible Windows host; retain every failure. Windows 11 24H2 needs the April
2025 optional or May 2025 security updates (or a later release), enabled Windows
Hypervisor Platform, and usable virtualization. A VM host must expose the
required virtualization level; an ARM CPU name alone does not establish it.

## 1. Capture the actual host and compiler

Run in an already provisioned ARM64 MSVC developer environment with a current
Windows SDK containing the published ARM64 WHP definitions. These commands do
not enable Windows features, install packages or reboot the host.

```powershell
$probeRoot = Join-Path $env:TEMP ('kbox-arm64-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $probeRoot | Out-Null
[Runtime.InteropServices.RuntimeInformation]::OSArchitecture
[Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
Get-ComputerInfo -Property WindowsVersion, OsBuildNumber, OsArchitecture |
  ConvertTo-Json | Set-Content (Join-Path $probeRoot 'host.json')
Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform |
  Format-List | Out-File (Join-Path $probeRoot 'hypervisor-feature.txt')
cl /Bv 2>&1 | Out-File (Join-Path $probeRoot 'compiler.txt')
rustc -Vv | Out-File (Join-Path $probeRoot 'rustc.txt')
```

Require both process and OS architecture to be ARM64. Save the exact SDK header
path/version and SHA-256 alongside the compiler report. Stop if the SDK lacks
Arm64Support or ARM64/GIC types; retain that diagnostic rather than substituting
x64 headers or an emulated compiler target.

## 2. Query native ARM64 API support and partition-object cleanup

Save this as `arm64-api-probe.cpp` inside the private directory. It loads the
API from System32, confirms the native process/machine with IsWow64Process2,
queries both required capabilities, and creates/deletes one partition object.
It does not set up a VM, create a vCPU or boot a guest.

```cpp
#include <windows.h>
#include <WinHvPlatform.h>
#include <cstdio>
#if !defined(_M_ARM64) || defined(_M_ARM64EC)
#error Build a native ARM64 MSVC executable
#endif
int main() {
    USHORT process = 0, native = 0;
    if (!IsWow64Process2(GetCurrentProcess(), &process, &native) ||
        process != IMAGE_FILE_MACHINE_UNKNOWN || native != IMAGE_FILE_MACHINE_ARM64) {
        std::printf("{\"stage\":\"architecture\",\"process\":%u,\"native\":%u}\n", process, native);
        return 2;
    }
    HMODULE module = LoadLibraryExW(L"WinHvPlatform.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!module) {
        std::printf("{\"stage\":\"load\",\"win32Error\":%lu}\n", GetLastError());
        return 2;
    }
    auto capability = reinterpret_cast<decltype(&WHvGetCapability)>(GetProcAddress(module, "WHvGetCapability"));
    auto create = reinterpret_cast<decltype(&WHvCreatePartition)>(GetProcAddress(module, "WHvCreatePartition"));
    auto destroy = reinterpret_cast<decltype(&WHvDeletePartition)>(GetProcAddress(module, "WHvDeletePartition"));
    if (!capability || !create || !destroy) {
        std::puts("{\"stage\":\"missing-export\"}");
        FreeLibrary(module);
        return 2;
    }
    BOOL present = FALSE;
    UINT32 presentBytes = 0, featureBytes = 0;
    WHV_CAPABILITY_FEATURES features{};
    HRESULT hp = capability(WHvCapabilityCodeHypervisorPresent, &present, sizeof(present), &presentBytes);
    HRESULT hf = E_FAIL, hc = E_FAIL, hd = E_FAIL;
    bool arm = false;
    WHV_PARTITION_HANDLE partition = nullptr;
    if (SUCCEEDED(hp) && presentBytes == sizeof(present) && present) {
        hf = capability(WHvCapabilityCodeFeatures, &features, sizeof(features), &featureBytes);
        arm = SUCCEEDED(hf) && featureBytes == sizeof(features) && features.Arm64Support;
        if (arm) hc = create(&partition);
    }
    if (partition) hd = destroy(partition);
    std::printf("{\"pid\":%lu,\"hypervisor\":%d,\"arm64Support\":%d,\"presentBytes\":%u,\"featureBytes\":%u,\"presentHr\":\"%08lx\",\"featuresHr\":\"%08lx\",\"createHr\":\"%08lx\",\"deleteHr\":\"%08lx\"}\n",
        GetCurrentProcessId(), present != FALSE, arm, presentBytes, featureBytes,
        static_cast<unsigned long>(hp), static_cast<unsigned long>(hf),
        static_cast<unsigned long>(hc), static_cast<unsigned long>(hd));
    FreeLibrary(module);
    return arm && SUCCEEDED(hc) && partition && SUCCEEDED(hd) ? 0 : 2;
}
```

Compile and retain exact output and bytes:

```powershell
Push-Location $probeRoot
cl /nologo /W4 /EHsc /std:c++17 arm64-api-probe.cpp /Fe:arm64-api-probe.exe 2>&1 |
  Tee-Object compile.txt
if ($LASTEXITCODE -ne 0) { throw 'Probe compilation failed; retain compile.txt' }
Get-FileHash arm64-api-probe.cpp, arm64-api-probe.exe -Algorithm SHA256 |
  Format-List | Out-File hashes.txt
.\arm64-api-probe.exe > api.json
$probeExit = $LASTEXITCODE
"exit=$probeExit" | Out-File api-exit.txt
Pop-Location
```

Require successful native architecture detection, exact capability buffer
lengths, Arm64Support and successful create/delete HRESULTs. Even an exit 0
qualifies only this API/partition-object probe on this host. A partition delete
failure is a failed cleanup result. Retain the process identity and do not
report VM or vCPU success.

## 3. Exercise the actual kbox refusal on that host

Use a fresh K_HOME under the probe root. With the pinned Rust toolchain, run:

```powershell
$env:K_HOME = Join-Path $probeRoot 'home'
New-Item -ItemType Directory -Path $env:K_HOME | Out-Null
cargo test --locked --target aarch64-pc-windows-msvc -p koci-sdk windows_architecture -- --nocapture
cargo check --locked --target aarch64-pc-windows-msvc -p koci-runtime -p koci-kbox --all-targets
cargo check --locked --target aarch64-pc-windows-msvc -p klibksys
```

The SDK native-ARM admission test must run, not merely compile. The final
klibksys command must currently refuse with its explicit unsupported-dependency
reason, even if the platform probe succeeded. Capture all exit codes separately.
An x64 process running under emulation must likewise receive Unsupported before
Box reservation/launch or OCI driver activation; do not change the route to test it.

## 4. Resume the native dependency work

Work in the owning libkrun/libkrunfw repositories. Implement ARM64 vCPU registers,
exits, GIC/interrupts, memory/device layout and native Windows bindings; build
an ARM64 MSVC krun.dll/import library and Windows firmware around a verified
Linux/aarch64 kernel. Keep guest-init/agent/rootfs architecture and ABI distinct
from the host DLL. Freeze PE Machine 0xaa64, guest ELF Machine 0x00b7, source
revisions, provenance and hashes before introducing another runtime bundle.

Then run actual guest boot, OCI lifecycle/I/O, read-only write attacks,
generation/recovery, owner-death and full resource cleanup on the same native
host. Only native evidence may change the support matrix. Current x64 archives,
cross compilation and this API probe do not replace those gates.

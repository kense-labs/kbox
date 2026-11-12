# klibksys

## 概述

FFI bindings to libkrun with Windows WHPX backend support

[![Crates.io](https://img.shields.io/crates/v/klibksys.svg)](https://crates.io/crates/klibksys)
[![Documentation](https://docs.rs/klibksys/badge.svg)](https://docs.rs/klibksys)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

FFI bindings to [libkrun](https://github.com/containers/libkrun) with **Windows WHPX backend support**.

## Features

- ✅ **Cross-platform**: Linux (KVM), macOS (Hypervisor.framework), Windows (WHPX)
- ✅ **Windows WHPX Backend**: Full Windows Hypervisor Platform support
- ✅ **virtiofs**: Passthrough filesystem on all platforms
- ✅ **virtio-net**: Network device with TCP backend on Windows
- ✅ **virtio-blk**: Block device support
- ✅ **virtio-console**: Serial console
- ✅ **TSI**: Transparent Socket Impersonation for vsock

## Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| Linux x86_64 | KVM | ✅ Supported |
| Linux aarch64 | KVM | ✅ Supported |
| macOS arm64 | Hypervisor.framework | ✅ Supported |
| **Windows x86_64** | **WHPX** | ✅ **Supported** |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
klibksys = "3"
```

### Windows Requirements

On Windows, you need to enable the Windows Hypervisor Platform:

```powershell
# Run as Administrator
Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform

# Reboot required
Restart-Computer
```

Verify it's enabled:

```powershell
Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform
```

## Usage

```rust
use klibksys::*;
use std::ffi::CString;

unsafe {
    // Create VM context
    let ctx = kcreate_ctx();
    let ctx_id = ctx as u32;

    // Configure VM (2 vCPUs, 512 MiB RAM)
    kset_vm_config(ctx_id, 2, 512);

    // Set kernel
    let kernel = CString::new("/path/to/vmlinux").unwrap();
    let cmdline = CString::new("console=ttyS0 root=/dev/vda rw").unwrap();
    kset_kernel(
        ctx_id,
        kernel.as_ptr(),
        K_KERNEL_FORMAT_ELF,
        std::ptr::null(),
        cmdline.as_ptr(),
    );

    // Set root filesystem (virtiofs)
    let root = CString::new("/path/to/rootfs").unwrap();
    kset_root(ctx_id, root.as_ptr());

    // Configure network (Windows: TCP backend)
    #[cfg(target_os = "windows")]
    {
        let iface = CString::new("eth0").unwrap();
        let mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        kadd_net_tcp(ctx_id, iface.as_ptr(), mac.as_ptr(), std::ptr::null());
    }

    // Set workload
    let exec = CString::new("/bin/sh").unwrap();
    let arg0 = CString::new("sh").unwrap();
    let argv = [arg0.as_ptr(), std::ptr::null()];
    kset_exec(ctx_id, exec.as_ptr(), argv.as_ptr(), std::ptr::null());

    // Start VM (does not return on success)
    kstart_enter(ctx_id);
}
```

## Windows-Specific APIs

### Network Device (TCP Backend)

```rust
#[cfg(target_os = "windows")]
unsafe {
    let iface = CString::new("eth0").unwrap();
    let mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    let tcp_addr = CString::new("127.0.0.1:9000").unwrap();

    // Add virtio-net with TCP backend
    kadd_net_tcp(ctx_id, iface.as_ptr(), mac.as_ptr(), tcp_addr.as_ptr());

    // Or disconnected device (null backend)
    kadd_net_tcp(ctx_id, iface.as_ptr(), mac.as_ptr(), std::ptr::null());
}
```

### Block Device

```rust
#[cfg(target_os = "windows")]
unsafe {
    let disk_id = CString::new("vda").unwrap();
    let disk_path = CString::new("C:\\\\path\\\\to\\\\disk.img").unwrap();

    kadd_disk(ctx_id, disk_id.as_ptr(), disk_path.as_ptr(), false);
}
```

### VSock with Named Pipes

```rust
#[cfg(target_os = "windows")]
unsafe {
    // Add vsock device
    kadd_vsock(ctx_id, 3);

    // Map vsock port to Named Pipe
    let pipe_name = CString::new("myservice").unwrap();
    kadd_vsock_port_windows(ctx_id, 8080, pipe_name.as_ptr());
    // Creates: \\.\pipe\myservice
}
```

## Examples

See the [examples](examples/) directory:

- [`windows_vm_test.rs`](examples/windows_vm_test.rs) - Basic Windows VM test
- [`nginx_test.rs`](examples/nginx_test.rs) - Full nginx container test

Run examples:

```powershell
# Windows
cargo run --example windows_vm_test --target x86_64-pc-windows-msvc
cargo run --example nginx_test --target x86_64-pc-windows-msvc
```

## Building from Source

### Linux and macOS

The build script compiles libkrun automatically when a compatible system or
cached library is unavailable. A recursive kbox checkout uses the root
`libkrun` submodule directly. Because Cargo does not package submodule
contents, the crates.io archive instead carries a deterministic,
checksum-verified `vendor/libksource.tar` and extracts it under `OUT_DIR`.

Because libkrun invokes Cargo from inside the outer Cargo build, the nested
process uses a target-local Cargo home next to the build outputs. That location
is deliberately not configurable: reusing the outer package-cache lock can
deadlock the nested build.

### Windows

```powershell
# Clone with submodules
git clone --recursive https://github.com/kense-labs/kbox
cd kbox/libkrun

# Build libkrun
winget install --id zig.zig --exact --version 0.16.0
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1 -Packages libkrun

# Copy the runtime DLL and MSVC import library
Copy-Item target\x86_64-pc-windows-msvc\release\krun.dll ..\crates\box\deps\libksys\prebuilt\x86_64-pc-windows-msvc\
Copy-Item target\x86_64-pc-windows-msvc\release\krun.dll.lib ..\crates\box\deps\libksys\prebuilt\x86_64-pc-windows-msvc\krun.lib

# Run tests
cd ..\crates\box\deps\libksys
cargo test --target x86_64-pc-windows-msvc --lib -- --test-threads=1
```

The Windows runtime bundle is complete only when the prebuilt directory also
contains `libkrunfw.dll`. That companion DLL supplies the Linux guest kernel;
building `krun.dll` does not build a kernel. Repository and GitHub release
bundles contain all three files. The crates.io package carries the exact tested
trio in a checksum-pinned, deterministic `vendor/kwindows-x64.tar.xz`.
That archive plus the reduced Unix source archive keeps the universal crate
below crates.io's 10 MiB limit without mixing runtime versions or requiring a
Windows build-time download.
The native files have additional license and source-delivery obligations; see
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) and
[`SOURCE-PROVENANCE.md`](SOURCE-PROVENANCE.md). The crate's MIT metadata covers
the Kbox Rust wrapper and does not relicense the bundled native components.
The Windows build script always creates a real stripped Linux `init/init`
payload first and remaps host source/profile paths out of the Rust release
artifacts; the generated payload remains ignored by Git.

Windows virtiofs retains guest-created and guest-updated POSIX mode, UID, and
GID values for the VM lifetime. A runtime that needs cross-generation
persistence must capture these values before shutdown and replay them after
boot; Kbox Box performs that handoff with its rootfs metadata manifests.

## Architecture

The Windows WHPX backend includes:

- **WHPX VM/vCPU Management** (`src/vmm/src/windows/vstate.rs`, `whpx_vcpu.rs`)
- **virtio Devices**:
  - `virtio-fs` - virtiofs passthrough (`src/devices/src/virtio/fs/windows/`)
  - `virtio-net` - TCP backend (`src/devices/src/virtio/net_windows.rs`)
  - `virtio-blk` - File-backed block device (`src/devices/src/virtio/block_windows.rs`)
  - `virtio-console` - Serial console (`src/devices/src/virtio/console_windows.rs`)
  - `virtio-vsock` - TSI implementation (`src/devices/src/virtio/vsock/tsi/windows/`)
- **EventFd** - Windows event wrapper (`src/utils/src/windows/eventfd.rs`)

## Testing

```powershell
# All tests (Windows)
cargo test --target x86_64-pc-windows-msvc --lib -- --test-threads=1

# Specific test
cargo test --target x86_64-pc-windows-msvc --lib test_kcreate_ctx -- --test-threads=1
```

**Note**: Use `--test-threads=1` on Windows due to WHPX partition limits.

## Documentation

- [Windows Integration Guide](../../WINDOWS_INTEGRATION_TEST.md)
- [API Documentation](https://docs.rs/klibksys)
- [libkrun Documentation](https://github.com/containers/libkrun)

## License

The Kbox Rust wrapper is MIT licensed; see [LICENSE](LICENSE). Bundled native
components retain their own licenses and notices as documented in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Contributing

Contributions welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md).

## Credits

- Based on [libkrun](https://github.com/containers/libkrun) by Red Hat
- Windows WHPX backend by Kbox Team

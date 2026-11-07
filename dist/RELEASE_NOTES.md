# kbox 3.4.3 — macOS (Apple Silicon)

Product configuration now uses external **KCL** (`.kcl`) via `kense-kcl` instead of the in-tree ACL crate.

## Highlights

- Removed `crates/acl`; kbox depends on `../kcl` (`package = "kense-kcl"`).
- Compose / Scale / OCI build-plan APIs and discovery prefer `.kcl` (`compose.kcl`, `services.kcl`).
- Windows DACL / filesystem ACL paths are unchanged.
- Packaging still stages one real dylib per library family plus soname symlinks (~22 MB archive / ~42 MB installed).

## Install

```sh
curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.4.3/install.sh | sh
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

Or upgrade in place:

```sh
kbox update
```

## Package contents

| Component | Purpose |
| --- | --- |
| `bin/kbox` | CLI (stripped, fat LTO, opt-level=z) |
| `bin/kbox-shim` | Per-VM host shim |
| `bin/kbox-guest-init` | Static aarch64 Linux guest PID 1 |
| `lib/libkrun.1.17.0.dylib` | HVF library; aliases are symlinks |
| `lib/libkrunfw.5.dylib` | Guest kernel bundle; alias is a symlink |

- Platform: macOS arm64, Apple Hypervisor Framework.
- Compressed archive: **22 MB**.
- SHA-256: `44771882345f469478919da8ea016fa5ff60eab5f47ecd542861ea7a85e5d656`

## Verified

- Isolated-prefix install prints `kbox version 3.4.3`.
- Compose KCL MicroVM smoke (`test_real_compose_kcl_smoke`) passed on this host.
- Installed `lib/` retains soname symlinks.

Not covered: Linux/Windows packages, non-arm64 platforms.

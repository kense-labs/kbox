# kbox 3.4.1 — macOS (Apple Silicon)

Packaging fix over 3.4.0: restore soname symlinks for `libkrun*` / `libkrunfw*` so the release archive is not inflated by duplicate dylib bodies.

## Highlights

- Release archive returns to ~22 MB compressed / ~42 MB installed (was ~37 MB / ~72 MB in 3.4.0).
- `scripts/package-macos-arm64.sh` stages versioned dylibs once and links aliases (`libkrun.dylib` → `libkrun.1.17.0.dylib`, `libkrunfw.dylib` → `libkrunfw.5.dylib`).
- Product surface unchanged from 3.4.0 (MicroVM-focused runtime; process sandbox lives in `kense-labs/sandbox`).

## Install

```sh
curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.4.1/install.sh | sh
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
| `bin/kbox` | CLI (7.4 MB, stripped, fat LTO, opt-level=z) |
| `bin/kbox-shim` | Per-VM host shim (3.2 MB) |
| `bin/kbox-guest-init` | Static aarch64 Linux guest PID 1 (5.1 MB) |
| `lib/libkrun.1.17.0.dylib` | HVF virtualization library (4.3 MB); `libkrun.1.dylib` / `libkrun.dylib` are symlinks |
| `lib/libkrunfw.5.dylib` | Guest kernel bundle (22 MB); `libkrunfw.dylib` is a symlink |

- Platform: macOS arm64, Apple Hypervisor Framework.
- Compressed archive: **22 MB**.
- SHA-256: `5f04825adfeefd0673145d6aa97172965bf0291a973340d744786b2fb0f4909b`

## Verified

- Isolated-prefix install prints `kbox version 3.4.1`.
- Staged `lib/` contains three symlinks and two real dylibs (`du` ≈ 26 MB).

Not covered: Linux/Windows packages, non-arm64 platforms.

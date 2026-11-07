# kbox 3.4.2 — macOS (Apple Silicon)

Packaging and installer fix so soname symlinks stay symlinks end-to-end.

## Highlights

- Archive stays ~**22 MB** compressed (same as 3.3.0 / 3.4.1).
- Installed prefix is ~**42 MB** again: `dist/install.sh` preserves `libkrun*.dylib` / `libkrunfw*.dylib` soname links instead of copying each target body three times (~72 MB).
- `scripts/package-macos-arm64.sh` continues to stage one real dylib per library family plus relative symlinks.

## Install

```sh
curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.4.2/install.sh | sh
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
| `lib/libkrun.1.17.0.dylib` | HVF library (4.3 MB); aliases are symlinks |
| `lib/libkrunfw.5.dylib` | Guest kernel bundle (22 MB); alias is a symlink |

- Platform: macOS arm64, Apple Hypervisor Framework.
- Compressed archive: **22 MB**.
- SHA-256: `03493b7db00b0189350e5a2d159616954669cc5f5084db1a79fd469cd5accbc7`

## Verified

- Isolated-prefix install prints `kbox version 3.4.2`.
- Installed `lib/` retains three symlinks; `du -sh` prefix ≈ 42 MB / lib ≈ 26 MB.

Not covered: Linux/Windows packages, non-arm64 platforms.

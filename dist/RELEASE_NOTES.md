# kbox 3.2.7 — macOS (Apple Silicon)

Local Agent Runtime and OCI workload execution: MicroVM lifecycle, images, volumes, networks, snapshots, and a Docker-compatible CLI.

## Install

```sh
curl -LO https://github.com/kense-labs/kense/releases/download/kbox-v3.2.7/kbox-3.2.7-macos-arm64.tar.gz
tar -xzf kbox-3.2.7-macos-arm64.tar.gz
sh kbox-3.2.7-macos-arm64/install.sh        # installs into ~/.kbox, no sudo
export PATH="$HOME/.kbox/bin:$PATH"         # add to your shell profile
kbox version
```

Quick start with the public dsh-acp image (no registry login required):

```sh
export DEEPSEEK_API_KEY=your_key
kbox run -i --rm -e DEEPSEEK_API_KEY ghcr.io/kense-labs/dsh-acp:latest
```

## Package contents

| Component | Purpose |
| --- | --- |
| `bin/kbox` | CLI (7.8 MB, stripped, fat LTO, opt-level=z) |
| `bin/kbox-shim` | Per-VM host shim (3.4 MB) |
| `bin/kbox-guest-init` | Static aarch64 Linux guest PID 1 (5.3 MB) |
| `lib/libkrun*` | HVF virtualization library (4.5 MB) |
| `lib/libkrunfw*` | Guest kernel bundle (22.8 MB) |

- Compressed archive: **22.9 MB** (21.8 MiB); installed on disk: **42 MB**.
- Platform: macOS arm64, Apple Hypervisor Framework. Linux KVM / Windows WHPX are not included in this archive.
- SHA-256: `4c3895096c2731bc86c8703550f759635d099a77f5a4652315d3f17d85152a0f`

## Verified

- Isolated-prefix install, `kbox version`, and a MicroVM launch (`alpine` echo) on macOS arm64.
- The same source tree built and ran `dsh-acp` end to end with a live DeepSeek ACP session.

Not covered: the full CLI test matrix on this archive, TEE features, and non-arm64 platforms.

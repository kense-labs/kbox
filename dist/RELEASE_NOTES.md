# kbox 3.3.0 — macOS (Apple Silicon)

Local Agent Runtime and OCI workload execution with MicroVM lifecycle, images, volumes, networks, snapshots, warm pools, and a Docker-compatible CLI.

## Highlights

- `kbox update --check` checks the latest stable kbox Release without installing it.
- `kbox update` downloads and installs the latest stable release; `kbox update 3.3.0` selects a version and `--force` repairs an existing installation.
- Interactive downloads show a progress bar. SHA-256 verification is mandatory, installed files are replaced atomically, and a post-install version check must pass.
- `kbox pool start --prewarm` initializes the application before advertising an idle VM; `kbox pool attach` connects ACP stdio to that prewarmed process.
- The public `ghcr.io/kense-labs/dsh-acp:latest` image can be pulled anonymously.

## Install

```sh
curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.3.0/install.sh | sh
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

Or download and install manually:

```sh
curl -LO https://github.com/kense-labs/kense/releases/download/kbox-v3.3.0/kbox-3.3.0-macos-arm64.tar.gz
tar -xzf kbox-3.3.0-macos-arm64.tar.gz
sh kbox-3.3.0-macos-arm64/install.sh
```

## Update

```sh
kbox update --check
kbox update
```

By default, update installs beside the running binary under its prefix. Use `--prefix <PATH>` for a custom installation directory.

## Live dsh-acp greeting

```sh
DEEPSEEK_API_KEY=your_key \
DSH_MODEL=deepseek-flash \
DSH_REASONING_EFFORT=low \
RUNS=1 bash -o pipefail -c \
  'curl -fsSL https://raw.githubusercontent.com/kense-labs/dsh-acp/main/scripts/bench-pool-dsh-acp.sh | sh'
```

## Package contents

| Component | Purpose |
| --- | --- |
| `bin/kbox` | CLI with update and process-prewarmed pool support (7.9 MB, stripped, fat LTO, opt-level=z) |
| `bin/kbox-shim` | Per-VM host shim (3.4 MB) |
| `bin/kbox-guest-init` | Static aarch64 Linux guest PID 1 (5.3 MB) |
| `lib/libkrun*` | HVF virtualization library (4.5 MB) |
| `lib/libkrunfw*` | Guest kernel bundle (22.8 MB) |

- Compressed archive: **22.9 MB** (21.9 MiB); installed on disk: approximately **42 MB**.
- Platform: macOS arm64, Apple Hypervisor Framework.
- SHA-256: `c75ee78e9be922ed19910ae3d4e1a68bd9012b58b350ba14f4070babf6b967ac`

## Verified

- Update unit tests: 4 passed; CLI command-coverage tests: 16 passed.
- `cargo fmt`, focused Clippy, locked release-dist build, and installer shell syntax checks passed.
- Isolated-prefix installation and `kbox version` reported 3.3.0.
- Live update discovery correctly selected the latest published stable kbox release.
- The public Release installer and a real `kbox update --force` completed download, SHA-256 verification, atomic replacement, and post-install checks. A pseudo-terminal run displayed download progress from 0 to 100%.
- The officially installed and self-updated CLI ran the complete HTTPS benchmark with an isolated `K_HOME`, returned real DeepSeek text with `end_turn`, and exited 0.

Not covered: the full CLI test matrix on this archive, TEE features, Linux/Windows packages, and non-arm64 platforms.

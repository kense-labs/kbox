# kbox 3.4.0 — macOS (Apple Silicon)

Local Agent Runtime and OCI workload execution focused on MicroVM lifecycle, images, volumes, networks, snapshots, warm pools, and a Docker-compatible CLI.

## Highlights

- Process sandbox has been extracted to the standalone [`kense-labs/sandbox`](https://github.com/kense-labs/sandbox) (`ksandbox`) project; kbox ships as a MicroVM / DedicatedVm runtime.
- Unused non-MicroVM product surface (shared-kernel Box Sandbox stubs, terminal helper, updater leftovers, related third-party paths) removed from this tree.
- End-to-end dsh-acp × kbox suite under `scripts/e2e-dsh-acp/` with isolated `K_HOME`, tiered cases, and evidence templates.
- Live DeepSeek E2E (`TIER=all`, `deepseek-flash`) verified on macOS arm64: 15 PASS / 0 FAIL / 1 SKIP (P3-4 tool round-trip still deferred).

## Install

```sh
curl -fsSL https://github.com/kense-labs/kense/releases/download/kbox-v3.4.0/install.sh | sh
export PATH="$HOME/.kbox/bin:$PATH"
kbox version
```

Or download and install manually:

```sh
curl -LO https://github.com/kense-labs/kense/releases/download/kbox-v3.4.0/kbox-3.4.0-macos-arm64.tar.gz
tar -xzf kbox-3.4.0-macos-arm64.tar.gz
sh kbox-3.4.0-macos-arm64/install.sh
```

## Update

```sh
kbox update --check
kbox update
```

## Live dsh-acp E2E

```sh
DEEPSEEK_API_KEY=your_key \
DSH_MODEL=deepseek-flash \
DSH_REASONING_EFFORT=low \
TIER=all ./scripts/e2e-dsh-acp/run.sh
```

## Package contents

| Component | Purpose |
| --- | --- |
| `bin/kbox` | CLI with MicroVM pool + update (7.4M, stripped, fat LTO, opt-level=z) |
| `bin/kbox-shim` | Per-VM host shim (3.2M) |
| `bin/kbox-guest-init` | Static aarch64 Linux guest PID 1 (5.1M) |
| `lib/libkrun*` | HVF virtualization library (4.3M) |
| `lib/libkrunfw*` | Guest kernel bundle (22M) |

- Platform: macOS arm64, Apple Hypervisor Framework.
- Compressed archive: **37M**.
- SHA-256: `7a80bfa566151804c9a6ae855a6fad3d740de7efc75264f02f1fda3ab3393a81`

## Verified

- `cargo check --locked -p kbox-cli -p kbox-core -p kbox-runtime`
- Live `TIER=all` dsh-acp E2E against public `ghcr.io/kense-labs/dsh-acp:latest` with DeepSeek (evidence under `docs/evidence/box/20261107-dsh-acp-e2e-deepseek/`).

Not covered: Linux/Windows packages, non-arm64 platforms, P3-4 automated tool round-trip.

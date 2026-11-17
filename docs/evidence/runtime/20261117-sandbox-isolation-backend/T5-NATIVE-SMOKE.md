# T5 native smoke — fused daemon + guest OCI echo

Date: 2026-11-17  
Host: darwin arm64 (OrbStack available for guest *build* only)  
Status: **observed green** for this command set · ≠ product Pass

## Setup

- Binary: `target/debug/kbox-sandbox-engine` codesigned with
  `crates/box/sandbox/engine/sandbox.entitlements`
  (`com.apple.security.virtualization`)
- Guest dir (assembled under `/tmp/kbox-guest-*`, not yet product-packaged):
  - `Image` — built via scout `cargo xtask kernel` (linux 6.18.58)
  - `data.img` — scout `cargo xtask disk` (64 GiB sparse ext4)
  - `initramfs.cpio` — packed with **kbox** `kbox-sandbox-init`
    (`aarch64-unknown-linux-musl` + `rust-lld`)
- Env: `HOME=<tmp>`, `KBOX_SANDBOX_SOCKET`, `KBOX_SANDBOX_GUEST_DIR`,
  `KBOX_SANDBOX_NO_FINDER=1`

## Observed

```text
GET /_ping → OK
POST /images/create?fromImage=alpine&tag=latest → 200 (pull ~3s)
POST /containers/create → Id=3f9f6f57… 
POST /containers/{id}/start → 204
inspect State.Status=exited ExitCode=0
logs stdout → "hi"
daemon: VM booted ms=172; container exited exit_code=0
```

## Explicitly not claimed

- Product install / archive ships guest assets automatically
- Brand-clean in-tree `guest/kernel` (scout kernel still used for Image)
- `kbox run` CLI end-to-end (HTTP API path only here)
- Windows/Linux host native Pass
- Overall GOALS Pass / daily-usable install

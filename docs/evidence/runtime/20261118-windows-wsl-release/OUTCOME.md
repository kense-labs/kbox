# Windows WSL x86_64 release asset for v3.4.5

Date: 2026-11-18T05:04Z  
Host: Windows 4090 · WSL2 Ubuntu · `Linux x86_64`  
Branch: `main` (P17) · packaging commit `810574cd`  
Release: https://github.com/kense-labs/kbox/releases/tag/v3.4.5  
≠Pass · ≠native win32 / WHPX · ≠MicroVM companions in this archive

## Intent

YOLO scope: **使用 ssh 4090 完成 windows 版本构建和符合规范的发布推送**.

Native `x86_64-pc-windows-msvc` `kbox-cli` still fails to compile
(`BoxSecretMaterial` / `BoxTransientSecretStore` are gated to
`linux|macos` in `kbox-runtime`). The verified Windows daily path remains
WSL2 Sandbox namespaces (prior dual-end 61/61 on this host). Deliver a
compliant archive for that channel.

## Archive contract

| Field | Value |
| --- | --- |
| stem | `kbox-3.4.5-windows-wsl-x86_64` |
| builders | `scripts/package-linux-x86_64.sh` |
| installer | `scripts/install-package-linux-x86_64.sh` → `install.sh` |
| required companions | `bin/kbox`, `bin/kbox-sandbox-engine`, `bin/kbox-sandbox-init` |
| manifests | `VERSION`, `build-info.json`, `SHA256SUMS` |

## Build (4090 WSL2)

```bash
git fetch origin main && git reset --hard origin/main   # 810574cd
bash scripts/package-linux-x86_64.sh
```

Observed:

```text
Installed kbox version 3.4.5 at …/install-check
archive=dist/kbox-3.4.5-windows-wsl-x86_64.tar.gz
compressed=9,415,145 bytes
installed_stage≈20,572 KiB
sha256=aa20ebe2c4fa0f333dc525fbb4524b5e6f0c25b84021163cb3b5bb59483349c8
```

`build-info.json`:

```json
{
  "schema": "kbox.release-build.v1",
  "sourceCommit": "810574cd242346187b884658c839c053dd926092",
  "sourceInputsSha256": "8f9874aeb3e9ae7897f106a9971c948609c4ab9c619bf75c06e8af024b352ea5",
  "cargoLockSha256": "8adee72f414b38b82f24b163f175d3f1b1aeb3fadf9daefa5c633f50d1258972",
  "rustc": "rustc 1.97.1 (8bab26f4f 2026-07-14)",
  "target": "windows-wsl-x86_64",
  "host": "linux-x86_64",
  "productDefault": "sandbox",
  "microvmCompanions": false
}
```

Package install smoke (embedded in package script): version match OK;
checksum + ELF x86-64 admission OK.

## Publish

```bash
gh release upload v3.4.5 \
  kbox-3.4.5-windows-wsl-x86_64.tar.gz \
  kbox-3.4.5-windows-wsl-x86_64.tar.gz.sha256 \
  --repo kense-labs/kbox
```

Prior macOS asset retained:

- `kbox-3.4.5-macos-arm64.tar.gz` (`355b871d…`)

## Explicitly not claimed

- Product Pass / Apple notarization
- Native win32 `.exe` / WHPX Sandbox package (MSVC compile still broken on secrets cfg)
- MicroVM / libkrun companions inside this WSL archive
- Re-run of full 61/61 in this pulse (retained prior dual-end evidence for 3.4.5)

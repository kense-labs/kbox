# T5 package + clean-prefix install smoke

Date: 2026-11-17  
Host: darwin arm64  
≠Pass (not notarized; one host)

## Commands

```bash
bash scripts/package-macos-arm64.sh
# archive=dist/kbox-3.4.3-macos-arm64.tar.gz (~33 MB compressed)
tar -xzf dist/kbox-3.4.3-macos-arm64.tar.gz -C /tmp/extract
KBOX_PREFIX=/tmp/kbox-clean sh /tmp/extract/kbox-3.4.3-macos-arm64/install.sh
export K_HOME=$(mktemp -d)
export PATH=/tmp/kbox-clean/bin:$PATH
unset KBOX_SANDBOX_ENGINE_BIN
kbox run --rm alpine:latest -- /bin/sh -c 'echo CLEAN_PREFIX_OK'
```

## Observed

- Archive includes `bin/kbox-sandbox-engine` (virtualization entitlement) and
  `share/kbox/sandbox-guest/{Image,initramfs.cpio,data.img.tgz}`
- Installer reports Sandbox engine + guest paths
- `CLEAN_PREFIX_OK` printed; exit 0

## Explicitly not claimed

- Notarization / Gatekeeper stapling
- Multi-host matrix / Windows
- Overall GOALS product Pass

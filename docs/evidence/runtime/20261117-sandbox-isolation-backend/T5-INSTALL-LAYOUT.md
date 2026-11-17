# T5 install layout — sandbox engine + guest share

Date: 2026-11-17  
≠Pass

## Layout

```text
$PREFIX/bin/kbox-sandbox-engine   # codesigned (virtualization)
$PREFIX/share/kbox/sandbox-guest/Image
$PREFIX/share/kbox/sandbox-guest/initramfs.cpio
$PREFIX/share/kbox/sandbox-guest/data.img.tgz   # sparse template (~111 KiB)
```

`guest_dir()` resolves `../share/kbox/sandbox-guest` from the engine binary.

## Observed

Staged prefix under `/tmp/kbox-prefix-*` (no full `package-macos-arm64.sh` this
pulse). Daemon create/start alpine → exit 0; VM booted ~163 ms.

## Packaging hooks

- `scripts/package-macos-arm64.sh` **requires** engine + complete `guest/out`
  (refuses product archive if missing; see `T5-PACKAGE-REQUIRE-SANDBOX.md`)
- `scripts/install-package.sh` treats sandbox companions as required checksummed
  FILES and always installs `share/`
- `scripts/build-sandbox-guest.sh` builds guest/out

## Explicitly not claimed

- Full `package-macos-arm64.sh` archive produced this session
- Notarized / daily install Pass

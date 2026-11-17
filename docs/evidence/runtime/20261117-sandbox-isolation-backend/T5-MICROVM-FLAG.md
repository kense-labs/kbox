# T5 `--isolation microvm` native smoke

Date: 2026-11-17  
≠Pass

## Command

Installed prefix from `kbox-3.4.3-macos-arm64.tar.gz` + overlay CLI with
`--isolation`; then:

```bash
kbox run --rm --isolation microvm alpine:latest -- /bin/echo MICROVM_OK
```

## Observed

`MICROVM_OK` printed; exit 0 (libkrun path).

## Explicitly not claimed

Product Pass; notarize.

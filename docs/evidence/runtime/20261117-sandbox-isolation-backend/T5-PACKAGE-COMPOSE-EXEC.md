# T5 — Packaged CLI compose + exec (refresh)

Date: 2026-11-17  
Archive: `dist/kbox-3.4.3-macos-arm64.tar.gz` (~32 MB)  
≠Pass

## Commands

```bash
./scripts/package-macos-arm64.sh
tar -xzf dist/kbox-3.4.3-macos-arm64.tar.gz -C "$PREFIX"
export PATH="$PREFIX/kbox-3.4.3-macos-arm64/bin:$PATH"
export K_HOME=$(mktemp -d)
kbox compose -p pkgdemo up -d
kbox exec pkgdemo-web -- echo PKG_EXEC_OK
kbox compose -p pkgdemo down
```

## Results

- Package rebuild green (sandbox engine + guest shipped)
- Compose sandbox path from packaged binary: green
- `PKG_EXEC_OK` from packaged `kbox exec` (after project-name fix)

## Explicitly not claimed

- Notarized Gatekeeper install
- Product Pass

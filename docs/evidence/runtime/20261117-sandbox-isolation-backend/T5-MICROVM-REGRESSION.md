# T5 — Optional MicroVM path regression (post Sandbox default + Secrets)

Date: 2026-11-17  
Host: darwin arm64 (macOS 26.6.2)  
Branch: `main`  
≠Pass

## Intent

Near-term HG3: after Sandbox default + packaged Compose Secrets green,
verify optional `--isolation microvm` still boots via libkrun/HVF on this
host without weakening Sandbox as the default boundary.

## Archive

```text
archive=dist/kbox-3.4.3-macos-arm64.tar.gz
sha256=e5782d3f82e98f4a78a7effcbc6dbe9a0e9df620bec59cabfa5234ae9800102b
size≈32M compressed
```

## Install

```bash
PREFIX=$(mktemp -d /tmp/kbox-microvm-reg-install-XXXXXX)
K_HOME=$(mktemp -d /tmp/kbox-microvm-reg-home-XXXXXX)
STAGE=$(mktemp -d /tmp/kbox-microvm-reg-stage-XXXXXX)
tar -C "$STAGE" -xzf dist/kbox-3.4.3-macos-arm64.tar.gz
KBOX_PREFIX="$PREFIX" sh "$STAGE/kbox-3.4.3-macos-arm64/install.sh"
xattr -dr com.apple.quarantine "$PREFIX" || true
# adhoc re-sign engine/shim/libkrun for isolated prefix
export PATH="$PREFIX/bin:$PATH"
export DYLD_LIBRARY_PATH="$PREFIX/lib:${DYLD_LIBRARY_PATH:-}"
```

Observed install: `PREFIX=/tmp/kbox-microvm-reg-install-9LNPjb`,
`K_HOME=/tmp/kbox-microvm-reg-home-4LKvxV`, `kbox version 3.4.3`.

## Smoke

```bash
kbox run -d --isolation microvm --name microvmreg alpine:latest -- sleep 3600
kbox exec microvmreg -- /bin/echo MICROVM_OK
kbox rm -f microvmreg
```

Observed (this run):

```text
run_rc=0
ps: microvmreg STATUS running
exec → MICROVM_OK (exit 0)
rm_rc=0
MICROVM_OK
```

Log: `/tmp/kbox-microvm-reg-smoke.log`

## Sandbox default unchanged

Same packaged prefix, no `--isolation`:

```bash
kbox run -d --name sandcheck alpine:latest -- sleep 60
# boxes.json managed_execution.request.config.execution_boundary == "sandbox"
kbox rm -f sandcheck
```

Observed: `execution_boundary": "sandbox"` — default not weakened.

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0 (clean)

## Explicitly not claimed

- Product Pass / notarized Gatekeeper staple
- Multi-host MicroVM matrix
- Completion of unrelated `049-pool-oneshot-discard` WIP

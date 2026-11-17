# T5 CLI — default `kbox run` sandbox path

Date: 2026-11-17  
Host: darwin arm64  
K_HOME: isolated temp  
≠Pass

## Command

```bash
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
export KBOX_SANDBOX_ENGINE_BIN=$PWD/target/debug/kbox-sandbox-engine
export PATH=$PWD/target/debug:$PATH
export K_HOME=$(mktemp -d)
./target/debug/kbox run --rm alpine:latest -- /bin/sh -c 'echo HI_FROM_SANDBOX'
```

## Observed

```text
Pulling alpine:latest...
Creating box … started.
HI_FROM_SANDBOX
Box … exited and was removed.
exit 0
```

Requires in-tree `guest/out/{Image,data.img,initramfs.cpio}` (gitignored) and a
codesigned engine binary with Virtualization entitlement.

## Explicitly not claimed

- Product install ships guest + signed engine automatically
- Overall GOALS Pass / notarized daily install

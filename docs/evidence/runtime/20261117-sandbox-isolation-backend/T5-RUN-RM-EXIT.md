# T5 — foreground `kbox run --rm` CLI exit code

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Bug

Packaged/debug Sandbox: `kbox run --rm alpine:3.20 -- echo OK` printed
workload stdout and removed the box, but the CLI process exited **1**.
Smoke scripts checking `$?` failed.

## Root cause

`ManagedRuntimeRoute::Sandbox` is not `is_oci_routed()`. Foreground used the
host-PID path (`pid: null` → immediately “dead”) and
`ProviderExitPolicy::RequireGuestProof`, which drops authenticated provider
exit `0` when no `.kexit_code` exists. `foreground_exit_code` then
fail-closed to `1`.

## Fix

CLI foreground/PTY lifecycle selects managed-provider inspect +
`TrustProvider` via `ManagedExecutionMetadata::skips_host_pid_reconcile`
(OCI DedicatedVm **and** Spec050 Sandbox). Legacy libkrun MicroVM still
requires guest proof. Missing trusted exit still fails closed to `1`.

## Hermetic

```bash
export K_HOME=$(mktemp -d)
cargo test --locked -p kbox-cli --lib sandbox_route_uses_managed_provider
cargo test --locked -p kbox-cli --lib foreground_
```

Observed: focused tests pass (including
`test_sandbox_route_uses_managed_provider_exit_path` and RequireGuestProof
fail-closed coverage).

## Native smoke (in-tree debug CLI + codesigned engine)

```bash
export K_HOME=$(mktemp -d)
codesign --force --sign - --entitlements \
  crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
PATH=$PWD/target/debug:$PATH \
  kbox run --rm alpine:3.20 -- echo RM_EXIT_OK; echo EXIT:$?
PATH=$PWD/target/debug:$PATH \
  kbox run --rm alpine:3.20 -- sh -c 'exit 7'; echo EXIT:$?
```

Observed:

```text
RM_EXIT_OK
EXIT:0
EXIT:7
```

## Explicitly not claimed

- Product Pass / notarized daily install
- Packaged archive refresh embedding this CLI fix (debug/native only here)

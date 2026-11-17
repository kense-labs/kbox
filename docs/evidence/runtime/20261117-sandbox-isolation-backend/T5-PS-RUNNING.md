# T5 sandbox `kbox ps` stays running

Date: 2026-11-17  
≠Pass

## Bug

PID reconcile treated missing MicroVM shim PID as death → STATUS `dead`
while the fused daemon container was still `running`.

## Fix

`ManagedExecutionMetadata::skips_host_pid_reconcile` includes Sandbox (with OCI).

## Observed

```text
kbox run -d alpine:latest -- sleep 120
kbox ps -a
→ STATUS running
```

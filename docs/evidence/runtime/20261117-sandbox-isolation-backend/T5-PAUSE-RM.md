# T5 sandbox pause / unpause / rm

Date: 2026-11-17  
≠Pass

## Observed

```text
kbox run -d alpine -- sleep 60
kbox pause <name>   → STATUS paused
kbox unpause <name> → STATUS running
kbox rm -f <name>   → exit 0; ps empty
```

Fixes: Docker pause/unpause API; remove path allows sandbox daemon socket as
exec endpoint for Sandbox-routed records.

## Explicitly not claimed

Memory-preserving pause semantics beyond container freeze; multi-host.

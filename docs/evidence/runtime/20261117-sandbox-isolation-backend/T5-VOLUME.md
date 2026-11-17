# T5 sandbox volume bind

Date: 2026-11-17  
≠Pass

## Command

```bash
kbox run --rm -v "$HOSTDIR:/work:ro" alpine:latest -- cat /work/payload.txt
```

## Observed

`vol_payload` printed; exit 0. Adapter passes `record.volumes` as Docker `HostConfig.Binds`.

## Explicitly not claimed

Named volumes / anonymous volume lifecycle completeness.

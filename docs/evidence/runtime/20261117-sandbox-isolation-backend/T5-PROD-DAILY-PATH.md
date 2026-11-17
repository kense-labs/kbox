# T5 — Packaged daily path (生产可用 smoke)

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass · ≠notarize

## Intent

YOLO scope **推进到生产可用**: exercise the packaged Sandbox default path
users need day-to-day after install (pull/run/exec/stream/PTY/pause/volume/rm).

## Archive

`dist/kbox-3.4.3-macos-arm64.tar.gz`  
sha256 `440de9da7b3c21b8f59f971782d6da589f1028bbd664bed6c4794fbc5adae079`

## Observed

```text
K_HOME=/tmp/kbox-prod-daily-home-Bqdvug
PREFIX=/tmp/kbox-prod-daily-prefix-acvtJ3
kbox version 3.4.3
RUN_OK
STREAM_OK
PTY_OK
pause → ps STATUS paused → unpause/resume
volume create + bind + cat → vol
rm boxes + volume
PROD_DAILY_OK
```

Log: `/tmp/kbox-prod-daily-172401.log`

## Explicitly not claimed

- Notarized Gatekeeper staple (Apple credentials parked)
- Compose Secrets (seat in flight)
- Captured one-shot exec stdin (seat in flight)
- Multi-host matrix / Product Pass

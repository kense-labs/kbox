# T5 — Local release candidate kbox 3.4.4 (darwin arm64)

Date: 2026-11-17  
Branch: `main`  
≠Pass · ≠notarize

## Claim

Versioned Spec050 sandbox-default archive for developer/daily install on this
host. Quarantine clear still required for Gatekeeper (`PARK-NOTARIZE.md`).

## Archive

```text
dist/kbox-3.4.4-macos-arm64.tar.gz
sha256=11ef7ae81016bffd7d3eba19303ab3af7adad1048881efe5b7ecd9df2b51f466
```

CLI/core version: `3.4.4`

## Smoke

Isolated prefix install → `kbox version 3.4.4` →
`kbox run --rm alpine:3.20 -- echo REL_OK` → stdout `REL_OK`, **EXIT:0**.

Brand: `check-sandbox-brand.py` clean.

## Explicitly not claimed

- Product Pass / Apple notarize staple
- Multi-host matrix
- That unpublished worktree state equals a git-tagged release until tag/gh lands

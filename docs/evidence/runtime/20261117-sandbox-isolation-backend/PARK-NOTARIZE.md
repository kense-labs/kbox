# PARK — Notarize / Gatekeeper staple (production install)

Date: 2026-11-17  
Status: parked (slice-local) · overall campaign continues on other paths  
≠Pass

## Blocked capability

Apple notarization + stapling so a cold Gatekeeper install works without
developer quarantine clear (`T5-DAILY-INSTALL.md` / `T5-INSTALL-QUARANTINE.md`
cover that developer path only — ≠Pass / ≠notarize).

## Exact missing intent

- Apple Developer ID + notarize credentials / profile on this machine
- Authorization to spend notarize/cloud signing for release archives

## Resume

Re-open when credentials + explicit notarize intent are provided. Evidence:
this file + `dist/kbox-*-macos-arm64.tar.gz`.

## Explicitly not claimed

- Product Pass / Gatekeeper-clean first-run for end users

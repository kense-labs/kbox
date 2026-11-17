# PARK — Multi-host product matrix

Date: 2026-11-17  
Status: parked (slice-local) · overall campaign continues on this darwin arm64 host  
≠Pass

## Blocked capability

Same product gates re-run on additional native hosts (other macOS arches,
Linux KVM, Windows/WSL) required by GOALS stretch / multi-host claims.

## Exact missing intent / resources

- Access to additional native machines (or CI runners) for each claimed OS/arch
- Authorization to drive those hosts for Spec050 Sandbox default + package smokes

## Resume

Re-open per host with fresh `K_HOME`, packaged archive
`dist/kbox-3.4.3-macos-arm64.tar.gz` (or host-matched package), and evidence
under `docs/evidence/runtime/<date>-sandbox-isolation-backend/`.

## Explicitly not claimed

- Product Pass beyond this darwin arm64 host

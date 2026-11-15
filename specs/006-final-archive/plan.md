# Implementation Plan: final archive acceptance

Spec: `spec.md`. Start from all retained uncommitted A–F changes and evidence.

## Constitution check

I/V/X: actual host EPERM and closed refusals, no fallback or log-only claims.
II/VI/VII/VIII: reuse existing Runtime clients, ExecutionManager and one workspace;
no route, identity, state-machine or configuration changes.
III/IV/IX/XI/XIII/XV: synthetic private fixtures, exact counts and result checks,
retain failures, no weakened assertion. Existing native host only is qualified.
XII/XIV: native repositories/gitlinks and platform support levels unchanged.
All principles pass. Planning or compilation alone cannot satisfy acceptance.

## Sequence and ownership

1. Read root/owning docs and A–F, inventory current host/resources and preserve old dist.
2. Reuse `scripts/package-macos-arm64.sh` and `scripts/install-package.sh`; audit
   actual native sources, loader paths and installed companion signatures/architectures.
3. Reuse `scripts/e2e-engine/native_{smoke,readonly,primary,prewarm}.py` and the
   Runtime directory native test. Freeze a same-source library client, explicitly
   distinguish it from archive CLI, and point every VM companion to the install.
4. Verify count/result/cleanup artifacts after each run; fix root causes and
   regenerate affected binaries if needed. Never clean earlier or foreign resources.
5. Run full verification; record final source/size/identity and resource inventories;
   update release/evidence references without changing readiness.

No new product API/configuration is intended. Any necessary fix reaches its
callers, fixtures, focused tests, affected artifacts and native replay.

## Final acceptance

Current-source archive built and installed; signatures/architectures, root source
digest, external KCL source, actual native kernel bundle and loader maps verified.
All five native scenarios pass; prewarm/directory each execute one test with a
checked PASS artifact. Full verify passes in 231.88 s. Seven private home
inventories are empty; no owned process/mount/image remains and seven earlier
foreign RAM devices are unchanged. Archive and installed hashes remain intact.
Only an evidence-extraction Python API mismatch needed correction; no product
source changed, so no affected-artifact rebuild was necessary after acceptance.
Release docs now link the final evidence and retain all readiness/Host Service/
Windows ARM64 limits. No locally actionable acceptance work remains.

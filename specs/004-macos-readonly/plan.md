# Implementation Plan: macOS host-enforced read-only mounts

Spec: `spec.md`. Start from the uncommitted A–D release baseline.

## Constitution check

I/V/VIII: typed refusal for grants that cannot be enforced; no fallback.
II: keep ExecutionManager and existing durable routes/recovery.
III/IV/X/XI/XV: retain failures; prove host PEP effects with native adversarial
tests, privileged guest remount, positive writable control and cleanup.
VI/VII: same workspace, local APIs and KCL; no configuration framework.
IX/XIII: synthetic exact paths/identities only; no credential logging.
XII/XIV: no native submodule edits; claims name the actual macOS arm64 host.
All principles pass. Native evidence is acceptance, not planning completion.

## Research and reuse

`VmManager::prepare_volume_mount` -> `FsMount` -> Box shim `vm_launch` ->
`KContext::add_virtiofs` -> `start_enter` currently omits host access mode.
`virtiofs_ro` stages Linux/Windows read-only aliases; macOS refuses in preflight.
OCI's `macos_context::confine` calls `macos_worker_sandbox` before entry and
retains descriptor-pinned directory grants. Reuse this platform boundary.
`LocalExecutionBackendRouter` persists creation routes and dispatches retained
records independently of creation policy; do not change that contract.

## Scope and delivery

1. Reproduce current refusal and native grant bypass limits; retain baseline.
2. Share macOS enforcement, bind it into the Box shim, preserve descriptor and
   validation semantics; add native adversarial tests before admitting mounts.
3. Update staging/admission callers and affected refusal fixtures, with tests.
4. Run real HVF tests using isolated homes, privileged remount and host
   integrity checks; record exact artifacts and cleanup.
5. Run focused suites, affected Clippy, format and full verification entry point.

Callers: Runtime/CLI/Compose mount conversion, shim launch, OCI worker.
Tests: volume, managed preflight, CLI refusal expectations, native sandbox and
HVF qualification; router/recovery/generation regressions. Exports: only the
shared platform API required by the two shims. Configuration: no new options.

## Failure behavior

Refuse unsafe grant overlap, unsupported resource combinations, missing or
replaced pinned sources and failed Seatbelt installation before VM entry.
No error reroutes records or changes isolation. Cleanup owns only runtime data.

## Verification

Focused owning-package tests, native disposable-process tests, actual HVF
guest write attempts after remount, default router/recovery tests,
`cargo fmt --all -- --check`, affected Clippy, `scripts/verify-commit.sh`.
Evidence is separate under `docs/evidence/runtime/20261114-macos-readonly/`.

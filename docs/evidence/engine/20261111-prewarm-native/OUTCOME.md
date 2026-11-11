# Canonical prewarm handoff and independent OCI regression

Date: 2026-11-11. Host: macOS/arm64 HVF; guest: Linux/aarch64.
Result: PASS for the ExecutionManager / Box Runtime driver handoff and the
listed ordinary OCI regression. A04 scheduling and the wider KRun goal are open.

The opt-in driver prepares a digest-pinned idle bootstrap with no Harness
package, user process or credentials. Preparation reserves a canonical Box
record; the existing manager starts it. Compatibility includes the complete
boot request, actual mount set and resource/security configuration, excluding
unit/display identities. An explicit caller scope partitions trust and build
context. No new VM registry or direct WarmPool ownership is introduced.

Claim checks a live backend handle under the execution lifecycle lock and
atomically binds the new immutable creation intent and next Box generation in
the canonical store. The same physical runtime is retained, including its exact
OCI provider binding when present. The old reservation operation and generation
cannot replay effects. Unclaimed slots deny user data-plane sessions. Once
claimed, their provenance cannot make them eligible for the clean pool again.
Ordinary OCI and drivers without explicit prewarm composition use normal create.

## Native results

- [Final HVF result](2e4aba1bfda8439589218884c9495fb7/result.json): a reconstructed
  driver reuses the same reservation; a different Runtime unit claims it through
  ManagedRuntimeClient. Its execution ID, PID, PID start time and start timestamp
  match, Box generation advances from 1 to 2, and stale stop is refused. Idle
  exec is refused. Actual Node and a child process use Unicode/space directories,
  with exact 2 MiB contents and an unmounted canary. After user code dirties
  rootfs, a different clean VM is prepared and claimed; the dirty marker is
  absent. Final inventory is zero executions and volumes. The test executable
  contains the candidate driver/manager; the prefix CLI is used for inventory
  and cleanup. [Exact native invocation](2e4aba1bfda8439589218884c9495fb7/command.json).
- [Current ordinary OCI result](../20261111-prewarm-oci/593e613917964cea8b91243264a6876a/result.json):
  new release binary `a3d3a05022c66e436f27814061387af3ccf5e0934c32d88ac5d2d4d85f327f3a`.
  Pull, COPY/RUN with private owned build pool, run/exec/child, external 1 MiB
  file hash, named volume, HTTP and snapshot/restore pass without ACP or KRun.
  Engine SIGKILL/restart keeps the same ordinary execution. Native guest policy
  matches the direct baseline and zero private executions remain.

The final HVF sample measured 117,813 ms preparing the base and 75 ms claiming
it. These are two different operations in one sample, including host fixture
costs; they are not cold/hot P50/P95 or a performance qualification.

## Checks and retained failures

- [ExecutionManager suite](checks/prewarm-final-manager-tests.log): 74 PASS.
- [Prewarm focused suite](checks/prewarm-final-focused-tests.log): 8 PASS,
  including competing claims, exact scope/configuration, stale generations,
  existing pool probes and the OCI adapter binding regression.
- [Final commands and exits](checks/prewarm-final-checks.json): workspace
  all-target check, owning-package clippy, scoped rustfmt and manager tests
  PASS. [Release build](checks/prewarm-final-release-build.log): PASS.
- Whole-workspace fmt still fails on 50 existing paths, versus 55 in the
  baseline, with [no new paths](checks/prewarm-fmt-comparison.json).
- [Broader Runtime driver suite](checks/prewarm-driver-tests.log): 108 PASS,
  3 FAIL, 11 ignored. All three are [reproduced on HEAD](checks/prewarm-driver-baseline-proof.json)
  in a detached worktree. Exact HEAD cannot compile one older networking
  fixture (`BoxConfig.isolation` no longer exists); the [isolated compile repair](checks/prewarm-baseline-compile-repair.patch)
  allowed the unchanged three failing assertions to run. The worktree and its
  owned KCL symlink were removed. No failing assertion was loosened.

The first native attempt [refused a raw scope string](78996cd5c636445e8f8d0caa086adb62/failure.json)
before allocation. The fixture now hashes provider-build context into a bounded
scope. The [earlier native pass](6b01e2ef37fb4970a42d665c6beb801b/result.json)
preceded the OCI binding fix and is retained. A stale driver spelling in the
OCI test fixture was replaced with the existing typed enum. With that fixture
valid, the [new regression reproduced a missing OCI binding](checks/prewarm-oci-binding-current-repro-tests.log);
handoff now uses the existing handle projection and retains the binding. Other
initial compile/fixture failures remain in checks.

## Explicitly not claimed

This is the canonical handoff and explicit driver API. KRun `up` does not yet
schedule or replenish the pool. Complete scope derivation, in-flight
prewarm/crash recovery, engine and CLI orchestration, memory credentials,
generic dependency admission, five capability classes and the long-model,
resume, cancel and release/performance gates remain open. Native KVM/WHPX and
final archive installation were not exercised. No commit, push or global
installation was performed.

# Tasks: Box / independent OCI Host Service

Status: **complete for the bounded explicit macOS arm64 profile**, 2026-11-14.
Default/global W4 cutover remains open. No platform support level changed.

- [x] H1 Read handoff/contracts, trace gap, preserve A–F and previous archives.
- [x] H2 Build/sign/freeze Host, shim, Agent, pinned libraries and system image.
- [x] H3 Explicit macOS composition, provider/caller, admission and focused tests.
- [x] H4 Native mixed routes, reconstructed clients, policy/identity/generation.
- [x] H5 Committed response loss, repeat/concurrent/disconnected calls, SIGKILL/restart.
- [x] H6 I/O, files/directories/RO attacks, exact resource and capability enforcement.
- [x] H7 Final full verify, rebuilt archives, isolated installed native regressions.
- [x] H8 Source/dependency audit, owned cleanup, release/evidence links and W4 ledger.

## Accepted results

[OUTCOME](../../docs/evidence/runtime/20261114-oci-host-service/OUTCOME.md) records
exact products, commands, environment and limitations. Final Box candidate is
final-dist-3 / installed-3; independent OCI assets remain separately frozen.
[run-context](../../docs/evidence/runtime/20261114-oci-host-service/run-context.json)
contains full paths. [FAILURES](../../docs/evidence/runtime/20261114-oci-host-service/FAILURES.md)
and [history](../../docs/evidence/runtime/20261114-oci-host-service/TASKS-HISTORY.md)
retain every failure/correction, including final cap_drop normalization review.

- Final verify-commit PASS: Runtime2173/0/14ignored, CLI933/0/0, all contract
  suites, installer7/7, workspace all-target check, fmt and scopedClippy.
- OCI owning suites Agent23/0/0, Runtime426/0/0, SDK208/0/0; OCI scopedClippy PASS.
- Final archive-installed Box/HVF native15 exactly1/0/0 with CLI checked:
  14 actual host RO denials plus child repeated after restart; file/FS/streaming;
  mixed routes, malformed bindings and old-generation refusals; response loss,
  first concurrent duplicate with one side effect; authenticated stopped recovery;
  old input fails, Box/SDK generation1→2; five kernel capability masks zero for
  normalized cap_drop ALL. Unit red reproduced the bug; green checks four spellings.
- Independent Host matrix: 23 operations, 25/25 waves and owner-death recovery PASS.
- Five final installed regressions PASS with loader proof: OCI/engine, RO/restart,
  primary streams/cancel, prewarm1/0/0 and directory1/0/0.
- 30 CLI execution inventories empty, no volumes/process/mounted image/live socket;
  seven prior RAM devices unchanged. Interrupted attempt10 reconciled/removed.
- Final source digest, installed SHA256SUMS and preserved archives verified.
  Native repositories remain clean; no commits, staging, push or publication.

## Continuation and limits

No work remains in this bounded epic. Current references resolve; two historical
ROADMAP evidence targets are absent from checkout/history and need their original
archive to audit (listed in outcome/final-review-3.json). No evidence was invented.

Next local bounded task: guest-native ownership-preserving rootfs transport for
foreign-UID images. Until then, directory transport refuses them. Full feature
parity, fresh-host matrices, rollback/long soak and default cutover remain in
[W4-GATES](../../docs/evidence/runtime/20261114-oci-host-service/W4-GATES.md).
Windows ARM64 remains dependency-unsupported, with no native verification.

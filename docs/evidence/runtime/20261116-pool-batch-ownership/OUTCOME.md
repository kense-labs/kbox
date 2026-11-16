# Completed pool batch ownership

Bounded040 is accepted on this existing macOS26.6.2 arm64 host. Completed pool
boot results remain registered actual owners until exact idle publication or
physical finalization, including cancelled consumers. Internal incomplete boot,
unpublished pool lifetime and legacy callers remain required work. Full W4,
default OCI Host Service cutover/live continuity and enterprise GA remain open.

## Actual native failure and fix

Baseline `native-batch-red-check1` actually exits101 with0pass/1fail/0ignored.
The real boot_batch completes an authenticated native VM with live caller writes,
but parent registered-owner count is0. The fixture fails before deliberately
dropping an unregistered VM. Its still-owning worker receives a cleanup command
and actually stops the VM/removes its allocation; caller data remains, volumes/
box/socket directories are empty. No intentional native orphan is left behind.

This baseline uses accepted039 installed companions (Box1223c6b1/source87247f93).
Only a cfg(test) module declaration/new native fixture differ in its client;
`baseline-production-comparison.json` verifies that production warm_pool code
matches committedcf7240c9. Probe source
`e1ecd6533df14bf5135b726b9632ff5a912bef36c1108fd531b672a93d0a956c`,1727 inputs;
clienta1886b712cb33e1d4b9dcfcee88d87f9f79428742989602b5d7d2439b99ffc3b.
Exact baseline source copies and client are retained under the artifact root,
alongside original stdout/stderr/loader maps/before-cancel/cleanup records.

Boot workers now register the completed VmManager in the existing retained-owner
collection before returning to JoinSet/results. CompletedBoot carries the actual
owner and existing cancellation guard; the guard owns collection/watch references
so it survives static workers. Dropping a task output, result vector or waiting
consumer closes existing admission while the registered actual manager survives
in its published pool. There is no copied execution state, new config/dependency/
workspace or global VM Drop killer. Foreground callers share the same registration
and exclusive-transfer implementation. Closed/queued batches check admission
before and after boot permits, refusing before new allocation.

Initial-fill and maintenance consumers retain the guard across idle/stat waits,
then synchronously transfer the exact exclusive manager into idle membership.
Shutdown finalizes the already registered owner directly, preserving refusal/
retry. Stats wait precedes transfer/push. Selected maintenance still cannot sweep
unrelated pending owners; explicit drain retains its all-owner retry semantics.
Configuration, ExecutionManager authority, persisted routes/generation fencing,
templates, endpoint receipts and backend/isolation defaults stay unchanged.

## Actual checks

Fresh task-owned K_HOME/short temporary roots are recorded for every test/check.
No real user-home scan, repair or native submodule content edit occurs. Historical
default-gate real-home nonmodification stays unestablished as corrected by035.

- Four new hermetic cases pass:completed task-output drop, complete-vector drop
  with exact stop-refusal retry, closed admission before permit wait and queued
  admission recheck after permit. They exercise shared production registration/
  actual manager handlers and the real boot_batch gate, not copied business logic.
- Final pool-focused1:68pass/0fail/4ignored. CLI-full1:949pass/0fail/0ignored.
  Runtime/CLI/contracts all-target Clippy passes.
- `scripts/verify-commit.sh` passes isolation4/support/workspace-all-targets/fmt,
  then exits101 at Runtime2333pass/6fail/54ignored. The same six external RAM/APFS/
  disk-service failures remain. No green default gate, deleted/ignored assertion
  or zero-test pass is claimed.
- Unchanged full contracts106 top-level(+12 successful filtered child receipts)
  and installer8 are038 history, not new040 runs. Final install/manifest checks
  below execute with current040 assets.

Commands and actual counts/exits/environments are in `checks`:

```sh
cargo test --locked --offline -p kbox-runtime --lib completed_batch
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Frozen final archive and installed evidence

Artifact root `artifacts/pool-batch-ownership-2991d356111a4062a26f4954f197efdc`;
owned private root `/private/tmp/kbox-pool-batch-2991d356111a`.
Final compiled/package source
`2f56ae1ef4ccb1693587de2a71069b2a62571c5fca63a7a29a1f935347ac88dd`,1727 inputs,
based on maincf7240c9 plus040 changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.
Frozen current Runtime client:
0652c4ff5a4628ece9f1aa1734bdfd93f060bdb45b59fd5c00b0be20a51a483f.

Final archive:
`artifacts/pool-batch-ownership-2991d356111a4062a26f4954f197efdc/final-dist-poolbatch-1/kbox-3.4.3-macos-arm64.tar.gz`

SHA256 `28849f6c53c4fdc5ccc65c8a400f5724c83b86ab3f4947cf4e80f20ec3803faf`;
23427559 compressed bytes /44208128 installed bytes. Fresh prefix
`installed-poolbatch-final-1` under that root. All11 manifest files, source
provenance, CLI/shim ARM64 strict ad-hoc signatures, Linux aarch64 guest-init,
native identities and embedded OCI Agent pass. No notarization/publication.
Prior archives and failed baseline source/client are retained, not overwritten.

Separate unchanged OCI archive d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79
keeps original sourced2b442f175f89b34d6ba9b63e6d2df4a478dc2a6c3493f8ca647a761e9012871
and owning-repository native patch provenance; it is not relabeled040. Parent
libkrun3c2889c7/libkrunfwfc3ea068 gitlinks remain unchanged/clean.

Actual final tests use the frozen current client where required and companions
from the fresh installed archive, with actual loaded paths/hashes captured:

- Same completed-batch native entry becomes exactly1pass/0fail/0ignored. Actual
  parent registration is1 before consumer cancellation. The exact VM/PID stays
  live and owned after cancellation, admission closes, then real drain stops it,
  removes its created allocation/box/sockets and leaves no late writes/caller loss.
- Existing foreground and published-shutdown native entries each execute exactly
  1/0/0. Real healthy handoff, withheld health/stop ACK cancellation, synthetic
  one-shot refusal, exact allocation/template identity and actual stop retry
  assertions remain. Stalls/refusals are synthetic; VM/writes/identities/physical
  finalization are real. No actual OS stop-failure or Linux snapshot claim.
- AF-final1 passes all5 rows:3 Python scenarios (OCI lifecycle, Host RO attacks/
  restart, engine process flow) plus2 Rust native tests (prewarm and directory/
  generation fencing), each Rust exactly1/0/0. Host RO has14 real denials on both
  first/restart probes after privileged RW remount/new mount/child attempts, with
  RW control. Identity/replay/process streams/no-late-write/fencing remain intact.
- Separate4-daemon matrix passes16 initial authenticated Ready VMs and4 Guest
  commands. All4 pre-existing canonical caller template directories keep exact
  dev/inode/bytes. The Mac unsupported snapshot request retains its existing
  explicit cold optimization in the same backend, not native snapshot support.

Current total:5 Rust native tests +3 Python AF scenarios, plus the separate
4-daemon/16-Ready/4-command matrix. The single actual0/1 baseline failure remains
recorded separately and is not included in current passes. Counts come from
stdout/exit/result files, not only PASS fields or ignored/default tests.

## Owned cleanup and continuing work

`final-cleanup.json`/inventory/FD receipts audit31 private homes, both owned roots,
the bound verifier short root,37 observed VM socket directories,154 exact socket
paths,2 engines and755 pre-existing immutable cache files (bytes/UID/GID/mode).
Managed executions/volumes, owned VM/shim/engine/Host processes, mounts/disk images,
FD holders and sockets are empty. Only the mutable image catalog changes. Caller
sources, original failure, baseline source/client and old archives remain. Audit
deletes nothing. Runtime RSS is unmeasured.

Internal allocation/boot_or_restore/template cancellation before completion,
unpublished pool and legacy CLI ownership remain open. A later bounded task must
establish/fix remove_idle_vms selecting indices under one idle lock then removing
under another without rechecking IDs; publication/capacity racing release is a
separate recorded hypothesis. This phase does not claim those closed. Full W4,
default OCI Host Service cutover/live continuity/GA, original15s boot cause, six
disk/prior SDK gates, historical unexplained RAM loss, fresh-host/other-platform/
OCI conformance/security review/soak remain open. Windows ARM64 current dependencies
unsupported/no native run; platform levels and default libkrun/BoxVm stay unchanged.
Arbitrary hostile same-UID namespaces and actual OS stop/unmount failures are not
qualified.

Git baseline ancestry is already local in main. Fresh remote equality/push still
needs UID501 lookup/GitHub DNS recovery ([latest prerequisite proof](../20261116-pool-handoff-ownership/checks/git-prerequisites-closeout1.json)).
The accepted03914-commit bundle remains retained;040 task-only commit and a new
verified incremental bundle follow source/diff review, with actual refs/SHA in
the artifact receipt. No unchanged push retry, force push, remote merge or
publication is claimed. Continue the remaining local ownership/rollback gap;
the continuous goal remains active.

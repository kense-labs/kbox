# Queued pool boot admission

Bounded046 is accepted on this existing macOS26.6.2 arm64 host. Already closed
cold entry and shutdown during template-resource wait now refuse before a new
claim/boot. Restore checks closure before claim and actual boot, and returns
before cold fallback when closing. Full W4/default OCI Host Service cutover/live
continuity/enterprise GA remain open.

## Failure, implementation and actual checks

admission-red1 broadcold_boot_filter actually2pass/2fail/0ignored, exit101. Only
the new alreadyclosed/queued-template cases fail: production enters boot/config
validation despite closed admission. Malformed snapshot input prevents actual
backend work in the hermetic baseline; it is not a VM run. Original assertions
remain. admission-green1 executes4/0, including the two existing clean-error/
cancel controls. No test or validation is removed, lowered or skipped.

Cold boot reads the existing watch at entry and again AFTER template mutex wait,
BEFORE creating a template consumer or calling boot. A local refused-before-boot
observation lets the existing actual-owner finalizer remove the exact prepared
owner and report typed PoolError; actual stop failure still propagates/retains.
This is scoped control flow, not persisted execution state. Restore checks before
new claim/boot; a known never-started claim is released on that refusal, then
actual finalization and return precede fallback. Already admitted in-progress
work remains registered for drain. No new lifecycle/config/dependency/workspace,
global killer, identity/route/generation/default-isolation or native source edit.

Canonical VmManager boot_failed_before_provider_start, cleanup and ExecutionManager
callers stay byte-identical to base6d8ca729; hashes in canonical-authority-unchanged.
This phase does not broaden their acquisition/failed-start/recovery semantics.

Pool-focused1 actually80/0/9ignored, CLI949/0 and Runtime/CLI/contracts all-target
Clippy pass. Every check uses fresh owned K_HOME/short TMPDIR/stdout/stderr/exit.
scripts/verify-commit.sh passes isolation4/support/workspace-all-targets/fmt, then
exits101 at Runtime2348pass/6fail/60ignored with the same six external RAM/APFS/
disk-service failures. No green whole gate or skipped failure. Unchanged038
contracts106 top-level(+12 child receipts)/installer8 and044 VM246 are retained
history, not new046 runs. Actual commands/counts are in checks, including:

```sh
cargo test --locked --offline -p kbox-runtime --lib cold_boot_
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Frozen assets and native count separation

Artifact root `artifacts/pool-boot-admission-fbf62892dbbc4268809857d00c987840`;
private root `/private/tmp/kbox-pool-admit-fbf62892dbbc`.
Compiled/package source `190148b3347368ac965c1ff6f7450269a35f5d2fa3ce17073bfb9bbd7c8b8052`,1731 inputs,
based on main6d8ca729 plus046 changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.
Actual compiler-artifact frozen Runtime client `6789300ad93e9c7a7413d6ae22e2d561aa775be37ca5b359cb3a3fc6cd9427a6`.

Archive `final-dist-pooladmit-1/kbox-3.4.3-macos-arm64.tar.gz` under the artifact root;
SHA256 `0da9ec212b955898378a114289cf978a466d803d53bce61de113ad0a920c2b8a`;
23428828 compressed bytes /44224512 installed bytes.
Fresh prefix `installed-pooladmit-final-1`; all11 manifest/source/architecture/
strict ad-hoc signatures/native dependency/embedded OCI Agent checks pass. CLI/
shim Mach-O arm64, guest-init Linux aarch64. Loaded paths/hashes prove installed
companions for actual VM cases; no development/user environment substitution.
Parent libkrun3c2889c7/libkrunfwfc3ea068 stay clean/unchanged. Unchanged separate
OCI archived5f59995… retains sourced2b442f1… and original owning-repository native
patch provenance, not relabeled046. Prior archives/failures remain.

- Actual VM functional control reruns the045 native entry against current assets:
  exactly1/0/0. Fresh real VmController start, cancelled internal boot, actual
  manager/PID/allocation JSON/dev/inode and continuing Guest writes remain owned;
  no falseReady; product drain physically finalizes/no late writes/caller loss.
  Actual installed library maps are captured.
- A separate native prelaunch NEGATIVE executes exactly1/0/0 against valid image/
  installed companions. It queues at actual template mutex, receives shutdown,
  and returns typed PoolError without entering provider; prepared owner is
  finalized, caller dev/inode/bytes retained, no workload or late writes.
  Its actual VM launch count is0 and VM loader maps are empty, as required.
  LaunchGate's recovery path refuses before actual provider start on any failure.
  This is prelaunch refusal evidence, not a successful virtualization run.
- AF-final1 passes5 actual rows:3 Python scenarios (OCI lifecycle, Host RO attacks/
  restart, engine process flow), plus2 Rust native VM tests (prewarm, directory/
  identity/generation fencing), each Rust1/0/0. Host RO survives privileged RW
  remount/new mount/child attempts with RW control; streams/replay/fencing remain.
- Separate4-daemon matrix passes16 initial authenticated Ready VMs and4 actual
  Guest commands, retaining all4 pre-existing caller template dev/inode/data.

Current counts:3 Rust tests running real VMs +1 Rust prelaunch-negative test
running0 VMs +3 Python AF scenarios. That is4 actually executed Rust native
entries, not4 VM-run passes. The4daemon/16Ready/4commands matrix is separate.
stdout/exit/result/loader evidence is inspected, not just PASS/ignored/compile.

## Owned cleanup and remaining work

Audit covers27 owned homes,35 observed VM socket directories,
146 exact socket paths,2 engines,839 immutable cache files
(bytes/UID/GID/mode), both roots and bound verifier short root. No remaining
managed executions/volumes, owned VM/shim/engine/Host processes, mounts/disk images,
FD holders or sockets. The negative has no provider/VM/workload writes and keeps
caller canary identity. Only mutable image catalog changes; audit deletes nothing.
Runtime RSS unmeasured. Historical default-gate user-home nonmodification remains
unestablished as035 corrected; current explicit isolated native proof is valid.

Next047: unpublished WarmPool::start failure/cancel can still drop the owner
collection itself. Retain actual startup owner through existing callers/lifecycle,
without an untracked VM killer/duplicate execution state. Provider-inner cancel,
legacy/from_pid/template-allocation/capacity and other local lifetime work remain.
This gate does not cancel already-admitted in-progress provider work or qualify
native Linux snapshot. Full W4/default OCI Host Service cutover/live continuity/
GA, original15s boot cause, six disk/prior SDK gates, historical RAM loss, fresh-
host/other-platform/conformance/security/soak stay open.

User authorizes scoped ordinary commit/merge/push. Baseline is already integrated
locally; latest04408:45:09Z proof still has UID501 missing/GitHub DNS errno8.
No unchanged push retry, force operation, fresh remote equality/merge or publish
claim. Current source/ref/diff review precedes authorized commit/verified bundle;
actual post-commit refs/SHA in ignored artifact git-closeout.json. Continue047;
continuous goal remains active.

## Explicitly not claimed

No whole green gate/full W4/GA/default OCI cutover/live continuation, native Linux
snapshot/fresh-host/other platforms/actual OS stop or unmount fault/hostile same-
UID namespace qualification/runtime memory measurement/platform promotion.
Windows ARM64 current dependencies unsupported, no native run. No KRun business
coupling, real ~/.kbox operation, native submodule content edit or publication.

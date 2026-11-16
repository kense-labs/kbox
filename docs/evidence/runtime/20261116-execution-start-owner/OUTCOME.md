# Admitted execution starts retain their original operation

Bounded048 is verified on this existing macOS26.6.2 arm64 host. Once
LocalExecutionManager::start acquires its existing lifecycle lock, cancellation
or control timeout of its caller no longer drops the actual startup operation
before its result is durable. This is an admitted start, not an explicit workload
stop: retry/inspect the same execution; stop serializes after the original start.
Full W4/default OCI Host Service cutover/live continuity/enterprise GA remain open.

## Failure and shared implementation

047 left two first-start failures Starting/PIDNone; automatic cleanup correctly
refused an unproved original PID and required explicit operator quarantine.
Those failures, homes and archives remain. Their60/120s cold-start delay cause is
NOT established by048, and the prior package is not relabeled fully accepted.

The actual shared start API used to hold its guard and backend.start future only
in the requesting future. Cancelling that future released the lifecycle lock
before the original start returned. start-cancel-red1 actually0pass/1fail, exit101:
a replay bypasses the pending original start. The fixture finalizes its fake
execution before asserting, without launching any native VM in this baseline.

The implementation reuses the existing inspect pattern: acquire the same lock,
move that exact guard and a clone of the same manager/backend into one committed
Tokio operation, validate the existing record/generation there and finish the
existing ensure_started transition. Join failures remain typed Internal errors.
Cancellation while waiting for admission starts nothing. No timeout-triggered
kill of a different original request, new lifecycle registry/state/config,
permission widening, PID discovery fallback or dependency. ExecutionManager
remains resource authority; persisted BoxVm/OciSdk routes and generation fencing
are unchanged. Prewarm/recovery-specific paths and whole Runtime/process shutdown
mid-operation are separate boundaries, not claimed solved by a Tokio task.

Actual start-cancel-green2:3/0. Original completion, cancelled waiter, pre-admission
no effect, stop serialization and stale generation refusal are checked. The
Runtime Driver's actual bounded startup also expires, drops its first caller,
then retries the same execution and waits for the original result:1/0. Its paused
clock retains the existing2s budget, observes actual backend startup first, then
advances3s to expire that control timer and resumes real time for replay checks.
A completion flag prevents filesystem delay from impersonating serialization.

## Actual checks and retained failures

- local-execution2:350pass/1known external disk failure/37ignored.
- driver-focused2:118pass/1known external disk failure/11ignored.
- CLI-full1:950/0. Final all-target Runtime/CLI/contracts Clippy3 passes.
- verify3 exits101 at Runtime2353pass/6fail/62ignored, after isolation4/support/
  workspace-all-targets/fmt pass. These are the same2 RAM/Secret and4 APFS/disk
  service failures. No green whole gate or skipped failure is claimed.
- start-cancel-green1 failed because the test checked PID AFTER stop intentionally
  clears it. The same PID/token assertions now inspect the captured Running
  record BEFORE stop; the Stopped assertion remains after stop.
- local-execution1 compiled zero tests (missing Native fixture Route import),
  fixed explicitly. driver-focused1 exposed a fixture timer firing in reservation
  before startup; the synchronized paused-clock control above fixes that setup.
- verify1 stopped at formatting after the fixture edit, zero Runtime tests;
  verify2/3 actual2353/6/62 follow formatting. Original receipts remain.
- Native candidate1 actually0/1: its fixture expected the not-yet-committed
  Starting record's anonymous-volume list to be populated. That list is committed
  with the returned startup handle. The same count/identity assertions now use the
  REAL returned handle, assert Starting remains empty, then assert the durable
  Running list matches that handle and generation. No assertion is removed.
  Candidate1 product kill/remove succeeded, no live PID/execution/volume remained;
  no operator rescue. Candidate1/archive/client/source/failure logs are retained.

Every command uses fresh owned K_HOME and retained stdout/stderr/exit/counts.
Commands include cargo test --locked --offline -p kbox-runtime --lib local_execution,
kbox_contract_driver, cargo test -p kbox-cli --lib, all-target Clippy and
bash scripts/verify-commit.sh. Full commands/actual results are in checks/.

## Frozen assets and installed native effects

Artifact root `artifacts/execution-start-owner-7853d3943ed747b2a53a5d03911a968a`; private root `/private/tmp/kbox-start-owner-7853d3943ed7`.
Final source `50884bbc7a517c113a434242d8ca3fbd6b532eca29c50a1f01f49bf4410bf5c9`,1733 compiled inputs,
based on main3ebbd363 plus048. Only startup_native.rs differs from candidate1,
recorded in source-candidate1-to2-delta.json. All5 shipped runtime companions
are byte-identical across the two candidates; manifests bind the respective
source snapshots. Frozen actual compiler client `8ae9ed04ffc833ebb01438247b2a822bc0616ce4e9aac453286513171f2c8c5d`.
Rustc1.97.1/Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.

Final archive `final-dist-execstart-2/kbox-3.4.3-macos-arm64.tar.gz` under that root;
SHA256 `4a70939e1ea7b4a1e2e8e76fcf08ecdea865245476d9c0ff439397fd99ee79f2`;
23428519 compressed bytes/44208128 installed disk bytes.
Fresh prefix installed-execstart-final-2;11 manifest/source files checked,
Mach-O arm64 CLI/shim/native libraries, Linux aarch64 guest-init, actual ad-hoc
signatures and native dependencies checked. Parent libkrun3c2889c7/fwfc3ea068
remain clean/unchanged. Separate OCI archive d5f59995/source d2b442f1 keeps its
original owning-repository provenance and native patch; no relabeling as048.

New Native test actually1/0/0 (14.52s). A wrapper only delays RETURN from the REAL
VmLocalExecutionBackend start; it does not replace any VM, handler, stop or
physical identity. The caller's actual1s control timeout expires while the same
original operation continues. Actual Ready VM PID20408/token/generation1,
anonymous allocation JSON/dev/inode and continuing Guest writes are checked.
A rebuilt client waits100ms at the original lock, then the same original result
persists exactly that PID/token/generation/allocation. Starting and durable
Running records are saved. Product stop/remove physically finalizes the actual
VM and its created allocation, preserves caller canary/directory identity and
proves no late writes. INFO logs show actual phases; no cold-start SLA, old047
latency cause, whole-runtime teardown or synthetic OS stop fault is claimed.

Current installed AF-final1 passes5 actual rows:3 Python scenarios (OCI lifecycle,
Host RO attacks/restart, process streams), plus2 Rust native tests (prewarm,
directory/identity/generation), each Rust1/0/0. Host RO checks privileged RW
remount/new mount/child denial with RW control. Separate pool matrix passes
4daemons/16 initial authenticated Ready VMs/4 actual Guest commands and preserves
all4 caller template identities. Native and AF/pool run serially to avoid overlap
of our acceptance workloads. Current total3 successful Rust VM tests+3 Python
scenarios; the original Native0/1 is retained separately, not silently replaced.

## Cleanup, integration and next work

Audit3 adds the failed Native1 loader IDs/global socket paths to audit2 coverage.
Both receipts remain; final audit covers49 owned homes, BOTH Native attempts and all3 verifier
homes/short roots,36 observed VM socket directories,
150 exact socket paths,2 engines and
867 immutable cache files (bytes/UID/GID/mode).
No remaining owned managed execution/volume, VM/shim/engine process, mount,
disk image, FD holder or socket. Caller sources, failed candidate, prior archives
and old047 quarantines remain; mutable image catalog changes separately; audit
deletes nothing. No real user installation or native submodule content edit.
Historical default-gate user-home nonmodification remains unestablished as035
corrected; current explicit isolated evidence is valid. Runtime memory unmeasured.

User authorizes ordinary task-only commit/merge/push. Baseline69fffadb and cached
origin/main9d279 are already ancestors of local main. Fresh environment receipt
11:30:39Z still reports UID501 missing/GitHub DNS errno8; do not repeat unchanged
failed transports, claim fresh remote equality, force push or publish. Actual
post-commit refs/source/archive/verified incremental bundle live in ignored
artifact git-closeout.json. This is a bounded repair, not overall completion.

Next required local work: shutdown/drop of an entire runtime or original owner
mid-start and other confirmed ownership gaps; keep the original cold-start delay
unresolved until measured. Full OCI Host Service/default cutover/live continuity/
W4/GA, six disk and prior SDK gates, fresh host/other platforms/conformance/security/
soak remain open. Continue local work; no platform support level is raised.

## Explicitly not claimed

No green whole gate/full GA/W4/default OCI cutover/live continuity, fresh host,
other-platform native pass, whole Runtime/process-shutdown safety, original047
cold delay fix, actual OS stop/unmount fault qualification or runtime-memory
measurement. Windows ARM64 dependencies unsupported and no native run. No KRun
business coupling, new configuration or native-submodule source modification.

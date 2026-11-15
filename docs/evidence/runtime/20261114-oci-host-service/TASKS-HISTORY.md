# Tasks: Box/independent OCI Host Service

- [x] H1 Read handoff/constraints, trace actual gap, preserve source/old archive/resources.
- [x] H2 Build/freeze independent Host Service, shim, guest/system image and manifest.
- [x] H3 Implement explicit macOS composition/provider with focused refusal tests.
- [x] H4 Native mixed BoxVm/OciSdk routes, lifecycle, identity/binding/generation.
- [x] H5 Fault recovery/owner replacement, response loss/replay/concurrency/disconnect.
- [x] H6 I/O, files/directories/RO attacks, mounts/resource admission and exact cleanup.
- [ ] H7 Full verification; rebuild/install affected archive; repeat affected native gates.
- [ ] H8 Final owned inventory, docs/evidence/review and actual W4 gate ledger.

## Continuation

Evidence/run paths: docs/evidence/runtime/20261114-oci-host-service/run-context.json.
Baseline hashes/status, mounts/processes and prior RAM devices retained. Current
final archive was fully copied/checksummed before any change. Both native
repositories are clean. Root source has preexisting A–F uncommitted work.
Confirmed: Mac Host Service/public SDK exists; Box migration/provider has only
KVM/WHPX. Current common qualification profile refuses all mounts/resources;
Mac support must use actual portable bundle/storage capability admission.
Next: resolve/freeze existing OCI assets and build tools; add Mac explicit
composition and focused tests, then execute actual Box route on HVF.

## Current implementation and checks

New `local_execution/oci_hvf.rs` composes MacosHvfOciMigrationConfig, public SDK
HVF/DedicatedVm capability admission and the existing router/backend. Endpoint
layout is exact; off+endpoint keeps retained OciSdk usable while new creates use
BoxVm. Configured macOS CLI composition uses it only with explicit endpoint;
missing/foreign endpoint and unknown/shared-kernel modes refuse. Default stays
legacy. Mac BundleProvider reuses the existing DedicatedVm provider; its initial
bounded profile still refuses mounts/custom controls pending H6 implementation.
Exports wired in local_execution/mod.rs and lib.rs. Two focused config tests pass.

OCI Host/shim release-dist build PASS (63.96 s), Linux/aarch64 agent zigbuild
PASS (43.76 s). Signed frozen assets are in run-root/oci-installed. Pinned Alpine
download verified; reproducible system-image builder PASS (6.21 s) using existing
GNU/e2fsprogs and an owned llvm-readelf alias. Manifest SHA256
a2c535926563f34dcc886ea5039bd39b9c12f778894c7aec8fc26ee3a4aa7c8c.
Asset hashes/source manifest under checks/initial-oci-assets.json; no native edits.

New ignored native test:
local_execution::oci_hvf::tests::native::native_box_hvf_host_service_preserves_routes_and_recovery.
Requires sentinel-owned K_BOX_TEST_HVF_ROOT, fixed Host/shim/manifest and pinned
image; starts an independent real service and checks actual ExecutionManager
OciSdk create/replay/start/exec, reconstruction and mixed legacy/policy routes.
Initial compile exposed missing ExecutionSessionManager import/ExecRequest fields;
fixed without changing assertions, original build failure log retained.

Native attempt 1 ran one test and FAILED before VM entry: default macOS direct
ext4 assembly left a placeholder directory for portable bundle process lookup.
Root cause fixed in new_oci_preparation_manager: use existing CopyProvider only
for macOS OCI preparation/cleanup. Hermetic regression proves OCI directory
transport and unchanged BoxVm direct ext4 behavior (1/1 pass). No executable
check or test command weakened. Attempt 1 cleaned boxes and gracefully stopped
Host; all logs under checks/box-native-attempt-1.* and privateRoot/attempt-1.

Native attempt 2 running (session 17836), fresh privateRoot/attempt-2 with frozen
native-client-attempt-2. Next: collect and fix actual native result, finish H3/H4,
extend Mac storage/resource admission with tests, run service fault/owner matrix,
then full verify and rebuilt archive qualification. No external blocker so far.

Attempt 2 FAILED with the existing unprivileged directory/foreign-UID gate; gate
retained, not weakened. It also exposed why the directory provider must match
the existing macOS APFS cache. Preparation now uses existing case-sensitive APFS
(not the interim CopyProvider); default BoxVm remains direct ext4. Its regression
passes. The native test retains an explicit foreign-UID image refusal assertion
before testing the supported same-UID fixture. Pinned Alpine input has all 520
entries UID 0; a new generic OCI fixture built from that input is pinned as
docker.io/library/kbox-hvf-same-uid@sha256:a248721396855c34760e07392ddae71acc6125d1c1700fbaaaee39d79a81f136.
Foreign-UID refusal is a limitation, not successful feature implementation.

Attempt 3 PASS, exactly 1/0/0, 19.27 s. Actual Box ExecutionManager -> persisted
OciSdk -> public SDK -> independent Host Service -> HVF works; real BoxVm also
starts, policy/reconstructed manager retain both routes, exact binding and
stdout/stderr/exit23 pass. Both routes clean to zero and service stops gracefully.
Artifacts: checks/box-native-attempt-3.*, privateRoot/attempt-3/result.json.
Full independent macos-hvf-host-service-smoke (23 operations, owner-death and
25 waves) now running; use checks/host-service-matrix.* and active exec result.
Next: Mac storage/resource compiler+admission; native binding/fence/fault coverage;
archive rebuild/full verify and owned cleanup. No whole-task external blocker.

Host matrix first invocation correctly refused a too-long generated Unix socket
path before effects (host-service-matrix.stdout/json). Shortened only the owned
work parent to privateRoot/s; second invocation PASS (24.69 s), status available,
23 operations plus owner-death and soak 25/25. Raw host-service-matrix-2.* retained.

H6 implementation now added (targeted tests 2/2 PASS):
- vm/oci_hvf_spec.rs uses existing typed resource compiler, external directory
  mounts, explicit guest SYS_ADMIN, v2 handoff and readonly cgroup view. Unknown
  additions/file binds/invalid resource values refuse; drop ALL keeps precedence.
- Mac profile admits bounded existing caller directories and typed resource
  limits; retains named/anonymous/secret mounts, privileges, ulimits, unsupported
  modes refusal. Other platform profiles stay strict. Ephemeral quota explicitly
  refused instead of dropped.
- Dedicated provider classifies every emitted bind as caller/DetachOnly storage;
  attachment failures clean the prepared handoff/rootfs. OciPreparedExecution
  chooses the SDK's v1/v2 handoff based on the actual versioned annotation.
- Mac composition now requires v2 storage-handoff capability. New helper unit
  tests preserve exact quota/shares/memory/reservation/swap/PIDs and RO bind,
  Guest SysAdmin/drop ALL plus invalid input denial. First compile used absent
  capability MutGetters; corrected with existing setters, failure retained.

Current Native3 PASS covered pre-H6 basic composition; must rebuild/freeze the
client and rerun with actual storage/resource attacks before claiming H6. Next:
add static Guest RO probe + real Box file/FS/cgroup readback; wrong/missing binding,
old generation, concurrent/keyed exec and Host SIGKILL/restart session invalidation;
use current diagnostic run-box-native.py for fresh attempt numbers. Then full
verify, affected archive rebuild/install and final replay. No default cutover.

Native4 FAILED on mode0644 config.json with external grants. Existing worker
validator retained; portable publisher now creates control config0600. New
assertion reproduced red then passed green (1/1). Native5/6 FAILED because guest
probe's positive host write was absent. Native6 mount diagnostics retained.
Root cause in test fixture: Zig -O2 implicitly defines NDEBUG, removing asserts
and side effects inside them. Probe now undefines NDEBUG; preprocessed artifact
proves assertions exist. Earlier probe outputs are INVALID evidence. Native7
then honestly failed writable-remount because Box OCI exec discarded explicit
workload capabilities. Shared Mac workload capability compiler now feeds both
initial process and exec, bounded by the admitted container profile; existing
other driver exec behavior unchanged. Rebuild/retest H6 next, then H5 and H7/H8.
All failed test roots cleaned managed boxes and shut down Host; logs retained.

H6 Native8 now passed actual NDEBUG-free remount/fresh-mount + 14 host EPERM
attacks, child denial, host writable/file/FS effects, exact cgroup readback and
live update. It failed next at missing-binding variant0: inspect rediscovered a
current runtime for an active record without exact identity. Fixed shared
current_runtime: active running/warm-paused missing binding refuses; only
Starting/RestartStarting may discover an interrupted launch. Cold/terminal
unbound records do not adopt a current generation. Hermetic regression 1/1 PASS.
H5 real wire fault relay + SIGKILL/replacement/stopped recovery native checks
added; uses public SDK bytes to an independent real Host (no fake runtime).
Client compile initial failure (missing import, Generation tuple access) retained,
fixed; second build PASS. Native9 running with entire H3–H6 and faults. Then full
Runtime tests/Clippy, full verify, archive rebuild/install and replay all affected
native contracts. Need inspect actual result JSON/counts/cleanup before claim.

Native9 completed H6 (behavior.json PASS) then FAILED after deliberately dropped
committed exec response: retry's capability HashSet serialization varied, so same
request falsely mismatched the Host journal digest. Shared SDK process_serde now
sorts only the five capability sets (argv/env/order unchanged); existing bundle
normalization/digests unchanged. Host new operation schema v7 uses normalized
Exec fingerprint; v1–v6 keep their prior algorithms/accepted versions. Guest
already uses shared process_serde, no second serializer/state machine added.
Focused stable-wire test PASS; reconstructed-capability service test PASS with
16 retries, service reopen, changed-command refusal and exactly one driver exec;
v1–v6 encoding compatibility test PASS. Initial test helper compile/lock errors
retained and fixed. OCI owning full lib tests running (session62480), followed
by Box native client rebuild. Must freeze previous OCI installed assets BEFORE
rebuilding Host/shim/agent/system image; native10 must use rebuilt fixed assets.
Native fault test now also covers first-time concurrent duplicate exec across
two managers, positive stdin/stdout/stderr/exit7, and RO probe after restart.
No source changes to native repositories, no readiness/default promotion.

OCI owning suites PASS: Agent23, Host Runtime426 (including877 durable fault
points), SDK208; no failures/ignores. New immutable OCI prefix oci-v7-installed
built/signed/reproducible system image PASS; previous oci-installed untouched.
Native10 failed by test-thread stack overflow after actual response-loss replay,
first concurrent duplicate side effect, stream and SIGKILL/stopped recovery.
Retained recovered-sdk.json, owner-death-report.json and original logs; exit-6,
no test-count/result/cleanup claimed. Store shows restart_starting generation2.
Root cause: new native harness did not reuse the existing large-future runner
stack pattern. It now uses same32MiB runner/8MiB Tokio workers as directory,
prewarm and engine native tests. Added separately named ignored maintenance
cleanup entry, which asserts the same private sentinel and uses ExecutionManager.
Next: compile it, clean attempt10 (owned BoxVm remains), rerun native11. Then
CLI-installed profile/final package and all affected gates; no hardware blocker.
verify-first currently passed Runtime2171/13ignored andCLI933/contract suites,
waiting scopedClippy; it predates maintenance-test addition, so final gate must
run again on final source. Source-only native library/gitlinks remain unchanged.

verify-first PASS, Runtime2171+13ignored/CLI933/allcontract suites/installer7.
Native11 (normal harness stacks) FAILED at terminal inspection: exact worker
was alive at ensure, then died before drain, so wait_drained's Unavailable was
not handled by prior ensure-only text-matching fallback. Shared backend now
covers both calls and asks the provider to confirm exact worker terminal
identity; leaves exit_code unknown, never creates drain evidence. New hermetic
regression exercises the race. Native11 cleaned boxes/socket despite failure.
Maintenance attempt10 cleanup1 FAILED on pending restart_starting (no resources
removed as a workaround); maintenance now calls canonical reconcile on original
create operation first, reloads generation, then kill/remove. Existing failed
logs retained; Host waves now choose unused names so retries preserve old logs.
Need collect projection unit result, rebuild client, run maintenance cleanup2,
then Native12, package/installed native, final full gate and OCI scopedClippy.

Native12 PASS exactly1/0/0 (43.47s with loader audit). Fixed v7 Host/shim/Agent
and same-source Box client; real response loss, two independent first-time
concurrent requests create one effect, stdin/stdout/stderr/exit7, SIGKILL,
authenticated exact stopped/SIGKILL SDK recovery, unchanged live BoxVm PID/start,
old input refusal, explicit restart toBox2/SDK2, RO attacks repeated afterrestart,
wrong binding/generation refusal, zero boxes/socket cleanup. Loader maps prove
both declared native dependency sets. CLI flag false in diagnostic12, not claimed.
Maintenance cleanup10-2 PASS1/0/0: canonical reconcile resumed interrupted restart,
then killed/removed exact owned resources. No broad process/mount cleanup used.
Host v7 public smoke PASS23operations/25waves/authenticated owner death (26.81s).
OCI scopedClippy PASS (20.10s); owning suites23/426/208pass. Full final gatePASS
Runtime2172/14ignored,CLI933,allcontract/installer7.
Candidate1 archive installed and checksummed. Native13 FAILED only at new CLI
create syntax (missing '--' before command); retained CLI off-policy inspect and
existing OCI exec succeeded. Test fixture corrected, portable config tests now
use tempdir (2/2PASS), no product code change. Archive qualification harness
failed a loader path assertion because its new output root was below installed
prefix; actual installed shim hash matched. Guard now permits exact installed
shim path OR copied owned shim, retaining library exact-prefix/hash checks.
All five first archive scenarios finished independently with retained results;
not counted as a passing harness. Candidate2 build/install and final gate rerun
in progress; old archive/install/logs untouched. Next native14 + five fresh
archive2 scenarios, source/companion audits, W4 ledger and exact owned inventory.


## Final capability review continuation

# Tasks: Box / independent OCI Host Service

Status: **final review correction in progress**, 2026-11-14.
Default/global W4 cutover remains open. No platform support level changed.

- [x] H1 Read constraints and handoff, trace the gap, preserve A–F and old archive.
- [x] H2 Build/sign/freeze independent Host/shim/Agent/native assets/system image.
- [ ] H3 Explicit macOS SDK composition, provider, environment caller and refusals.
- [x] H4 Native mixed BoxVm/OciSdk, reconstructed clients, policy/identity/fences.
- [x] H5 Real committed-response loss, replay/concurrency/disconnect, SIGKILL/restart.
- [x] H6 Streaming/file/directory grants, actual host RO, resources and cleanup.
- [ ] H7 Final verification, regenerated archives, isolated installed native replay.
- [ ] H8 Source/dependency audit, exact owned inventory, release docs and W4 ledger.

## Results and evidence

[OUTCOME](OUTCOME.md) is the
acceptance record; [run-context](run-context.json)
contains exact artifact roots. [FAILURES](FAILURES.md)
and [task history](TASKS-HISTORY.md)
retain the failed attempts and corrections. All test counts were read from actual
outputs, not inferred from commands or ignored tests.

- Final verify-commit PASS: Runtime2172/0/14ignored, CLI933/0/0, all contract
  suites and installer7/7; workspace check/fmt/scopedClippy pass.
- OCI owning suites Agent23, Runtime426, SDK208 all pass; OCI scopedClippy pass.
- Archive-installed full Box/HVF matrix: native14, exactly1/0/0, CLI checked;
  14 real host RO denials + child repeated after restart; exact binding and
  Box/SDK generations1→2; no old input redirect or duplicate side effect.
- Host v7 service smoke: 23 operations, 25/25 waves and authenticated stopped
  recovery. Explicit session loss remains the HVF protocol, not live continuity.
- Five installed archive regressions PASS: OCI/engine, RO/restart, primary
  process streams/cancel, prewarm1/0/0 and directory1/0/0; actual loader maps.
- 24 final CLI inventories empty, volume catalogs empty, no owned process,
  mounted image or live socket; seven prior RAM devices unchanged.
- Source hash and both installed SHA256SUMS match after all native checks;
  root native repositories remain clean; old artifacts and all failures retained.

## Commands / continuation

Focused checks and scripts with complete arguments/exits are under outcome's
checks directory. Main gates were `cargo test --locked -p kbox-runtime <filter>`,
`cargo test --locked -p koci-sdk -p koci-runtime -p koci-agent --lib`, OCI scoped
Clippy, `bash scripts/verify-commit.sh`, and `K_BUILD_LIBKRUN=1 bash
scripts/package-macos-arm64.sh`. Immutable native runners in the artifact root
record exact commands/assets/private homes and assert actual results/counts.
No work remains in this bounded epic. No commit/push/publication was performed.

Next bounded local task: guest-native ownership-preserving rootfs transport for
foreign-UID images. Current directory transport continues to refuse them. Full
feature parity, rollback/fresh-host matrices, longer soak and default cutover
remain in [W4-GATES](W4-GATES.md).
Do not wait for Windows ARM64 or start a VMM/firmware port as a continuation of
this completed task.

Final review found an actual cap_drop precedence bug: shared normalization accepts
CAP_ALL/whitespace, but the HVF extension checked raw ALL and could re-add
SYS_ADMIN. Reopened H3/H7/H8; red regression running. Fix must reuse shared name
normalizer, then green test, full verify, regenerate Box archive and repeat
installed native/G regressions. OCI assets unchanged. Prior candidate2 and all
successful checks remain valid historical evidence, not the new final candidate.

# Spec009 tasks

- [x] Read current handoff/rules/owning docs and accepted spec007/008; preserve current dirty tree, source hashes and old assets. Run current file refusal baseline.
- [x] Prove exact live projection + Host file PEP (RO/RW/FDs/parent/siblings/alias mutation). Choose first complete safe mechanism and document limits.
- [x] SDK file identity/required extension; compatible manifest new version; malformed/drift tests and exports.
- [x] Host private projection, pinning, publication/retry/cancel and bound cleanup; shim exact data-only grant; Agent file mounts; real focused tests.
- [x] Existing Box resource-plan/durable source binding, OCI-owned caller, restart/fencing; preserve old directory/rootfs/BoxVm and default suites.
- [x] Full owning checks/fmt/Clippy/verify-commit; frozen affected assets and newly installed Box/OCI native file + regression matrices; actual counts/failures.
- [x] Verify only owned resources revoked, source/parent/canaries and other runs retained; final identities/evidence, ledger and continuation.

Artifact root: artifacts/oci-files-38d1ee741be94955907fc6c8b3fad645.
Private root: /private/tmp/kbox-files-38d1ee741be9.
Evidence: docs/evidence/runtime/20261114-oci-files.

## Historical checkpoints (2026-11-14)

Implemented but not yet accepted: SDK required file-grants v1 and strict inode
identity; Agent manifest v6; durable Box caller-file resource reservation; OCI-only
file mount preparation. BoxVm file staging remains unchanged. Native Seatbelt
parents passed 2 tests (each launches a confined child). Unsafe preopened O_RDWR
FD was reproduced in file-pep-1.log, then admission fixed by matching dev/inode in
the existing readonly FD guard; file-pep-2.log passes. SDK focused tests pass 2.
/dev/fd link follow/nofollow both EPERM; fd-link-probe.json retained, no retry.

file-box-tests-1.log failed compilation: plan_create_resources was inserted in a
wrapper rather than the shared owning provider. Moved to shared provider and
forwarded all wrappers; recheck pending. All failure logs remain.

Next: implement generation-private live hardlink projections outside writable
runtime share, strict replay/cleanup; wire shim pins/sandbox and Agent mounts;
then focused lifecycle/identity/cancel tests. No native lib source edits, asset
freeze or stage009 installed-archive acceptance yet. Preserve all prior assets.
No external blocker to current implementation. Continue W4 after full stage009
acceptance; do not final at the stage boundary.


## Checkpoint: transport connected, candidate native pending

- SDK full 213 / protocol full 63 pass: checks/file-sdk-protocol-full-1.log.
- Runtime projection 4 pass: checks/file-host-tests-3.log; Box file source 1 pass:
  file-box-tests-3.log; durable resource-plan 2 pass: file-resource-tests-2.log.
- Host private per-generation hardlinks, manifest v6, shim readonly pins and exact
  Seatbelt data rules, Guest projection mount/file rewrite now compile. Only HVF
  advertises the required extension. Rootfs v1/v5 and old directory code retained.
- Failures retained: misplaced wrapper method, noncanonical temporary fixture,
  test import error, shim type visibility, and unsafe preopened RW FD evidence.
  Fixed shared causes; no assertions removed or ignored to obtain passing results.
- Candidate OCI build is running from build-oci-candidate1.py; build source hash
  asserted unchanged at end. Fresh image-cache is a copy of spec008, never shared
  writable. Cross-built native probe hvf-file-probe, source tests/fixtures C file.
- Native harness adds two independent file executions, normal restart, stale
  generation, source replacement refusal, live bytes, exact parent withholding,
  writable remount/fresh tags + 16 Host denials + fork. Main mixed-route/owner-
  death path also repeats files before and after stopped recovery. Not run yet.
- Next: candidate native attempt, fix any real failure; add shim identity tests,
  full owning suites/Clippy/workspace/verify, rebuild final Box/OCI archives and
  isolated installed replay. Then cleanup/evidence and next W4 queue item.


## Checkpoint: actual Host denials and cancellation fix

Candidate1 full native test executed 1/0/0 in 390.92s; files lifecycle/restart,
source replacement refusal, mixed routes, owner-death and cleanup succeeded. Its
RO probe initially counted Guest EROFS: NOT accepted as Host RO proof. Retained
checks/native-proof-gap-1.md and probe-1 source/binary. Strengthened probe-2 clears
Guest readonly superblock, verifies statvfs writable and requires EPERM/EACCES.
Focused candidate2 native PEP executed 1/0/0 in 44.86s; all 16 actual Host denials
are EPERM, plus fork child refusal, real RW writeback/live host update.

Full suites this stage so far: CLI933, Box Runtime2177/14 ignored, OCI Agent25,
OCI Runtime434, SDK213, protocol63, shim73/3 ignored. Clippy and workspace check
passed (existing warnings). Runtime full434 preceded the cancellation fix.

Cancellation regression reproduced late aliases in file-cancel-1.log (1 failed).
Fixed with tempfile-owned exact 0700 root and nonblocking flock, retained through
the blocking result until caller adoption; unconsumed result removes aliases.
Test first exposed tempfile default 0777; explicit private creation fixed it.
file-host-tests-5.log: 5 pass, including cancelled job output, competing retry
and cleanup refusal while preparation owns its lock. No new dependency/store.

Native HFS RAM -> APFS cross-filesystem test executed 1/0/0, Unsupported without
copy/data/mode/owner changes; owned device unmounted/detached (PASS in
checks/crossfs-native-1.json). No external blocker. Candidate1/2 assets predate
cancellation fix, so final OCI MUST be rebuilt. Final Box archive also pending.

Next: freeze current source; owning checks affected by latest fix, verify-commit;
new Box/OCI archive build and isolated install, final native full matrix using
probe2 + current source client + actual installed assets, A-F regression and
identity/resource audit. Then stage009 acceptance, immediately next W4 stage.


Production-check correction: Clippy2 caught that tempfile was dev-only, although
unit tests could see it. Added the already-locked tempfile as macOS arm64-only
production dependency (no new crate/version/lockfile/dependency family). Reused
existing fs2 instead of handwritten flock unsafe. Production check3 and host
focused6 pass (5/0/1 ignored; crossfs ignored entry was separately run natively).
Source freeze2 f46a5fd7d2123215b1a7de6e37fee0051623a8d96cc73cdf630a0d78d4e9a9fa.
Box final1 is retained (old freeze54621...), final2 rebuilding after source change.
No final native asset acceptance yet. Full OCI test2 still running its existing
315s durable commit fault matrix; no failed assertion removed or bypassed.


## Checkpoint: final2 rejected, native cache boundary under repair

- Final2 installed Box archive: SHA0ce9f1bb23283e60d0c27bed8671267a0dcafa6aff71b3138c128b75c48bdb60,
  23331528 compressed / 44060672 installed; 11 file hashes. OCI: SHA66d5facf0cec0c88623c2f746cdd3ce0680a6dea43075047d25b6b3d999fbbe2,
  33199160 / 113905664; 10 hashes. Both source freeze f46a5f...; old versions retained.
- verify-commit1 passed; Clippy3 passed. Installed A-F five scenarios passed,
  including actual prewarm1 and directory1 /0/0, all loader identities checked.
  final-identities.json verifies signatures, arm64/ELF, dependencies and embedded Agent.
- Final native attempt3 FAILED strict read after prior Guest read then Host size
  update. Returned new data truncated at cached old length (12 bytes). Its test
  cleanup cleared owned executions; failure/assertions retained, not accepted.
- Root cause: current pinned libkrun has 5s attribute/entry TTL and Auto data
  caching, no cache control C API. nm confirms only add_virtiofs and add_virtiofs2
  (DAX size). Official kernel FUSE cache docs corroborate attribute cache behavior.
- Native own worktree created via git worktree (same repository, no new repo):
  /Users/roylin/code/kense/libkrun-oci-files-38d1ee741be9, base3c2889c76460d81e3fb16d6144580fd31eb1bae3.
  Parent libkrun/libkrunfw source worktrees untouched; no commit/gitlink/push.
  Added macOS generic krun_add_virtiofs_uncached, per-device flag, data Never +
  attr/entry ZERO. Existing APIs keep default behavior. No workload/tag special case.
- Native device focused test1 passed (native-cache-unit-2.log). First build failed
  missing generated init/init; actual init.c cross-built with zig, log retained.
  C API unit test compiling (native-cache-ffi-unit-1.log). Native changes uncommitted
  in own worktree; record base + patch + full source digest before building assets.
- Next: native FFI/default checks, build exact new library against retained OCI
  firmware, checksum-pin a NEW native archive with full provenance (preserve old),
  wire required API for caller files, rebuild affected OCI/source-bound Box assets,
  focused strong live-read/Host PEP native proof then full installed matrix/cleanup.
- Additional W4 follow-up after009: investigate old Box rollback ignoring new
  caller_files optional metadata (source type replacement could be reinterpreted
  by an old directory-only preparer). Not reproduced yet; prioritize if confirmed.
- Stage009 is NOT complete; full Host cutover/default/readiness not claimed.
  Continue W4 after genuine acceptance; no final response at this checkpoint.


## Native-source checkpoint

New uncached API implemented only in native own worktree (not parent submodule).
Device1 + FFI1 focused passed. First release14.0 candidate retained but rejected:
old prebuilt minimum is11.0, rebuilt with11.0 and explicit Rust1.97.1.
Full native devices exposed 2 old i8042 failures: pipe eventfd returned first
notification instead of accumulated count. Fixed shared macOS eventfd with shared
counter/readiness pipe, clone/semaphore/overflow/blocking wake and raw shutdown
write-FD preservation. Native utils tests also had stale epoll caller signatures;
updated borrowed arguments/buffer limit only, no assertions removed. Now devices60
and utils21 pass (native-eventfd-full-2.log). FFI unit2, Clippy2 and release3 run
via native-cache-final-checks.json. Native old source snapshot retained at
artifacts/.../native-source; snapshot again to a NEW directory after these changes.

Parent Shim now requires krun_add_virtiofs_uncached for file projections. Its
old pinned native archive cannot satisfy this yet. Next: generate a NEW archive
and source provenance/constant hashes from release3, retain exact old OCI firmware;
build candidate OCI3, run strict immediate-live-read PEP native test. Then source
freeze3, rebuilt isolated final archives, full native + affected gates/cleanup.
Old final2/native attempt3 remains rejected. No phase completion claim.


Native source2 frozen in artifacts/.../native-source-v2/source-identity.json:
source ad3288c000a6c71bc1e29d7c2d8146557fde7739e22d09b76820de99945be052,
patch43c57cd0b2109f1c9049f47f385c5352d07d9c77024d1021bd184680ff7c2dcd,
full source archive8b010b6a96e295e1016a33f6dc08a6530ca1a7add0c00ccd6e494f18fc5aea04.
Native new archive ec21a5ec84baf732aac7175ea3dac93d23b74d2d2607937965181662710617ed,
library1a3cdee8e0fe7140e9969742898b3b1f0ca53075b625ba301184b1bf6fbdabbb;
firmware remains841bc9..., kernel b118... exactly. New archive path separate
kmacos-arm64-uncached-files.tar.xz; old archive untouched. Parent generator changed
runtime-assets.json and macos_assets constants; sidecar uncached-source.json
records uncommitted own native source, not a landed gitlink. Native release3,
FFI unit2 and native Clippy2 pass; library minOS11.0/arm64/symbol confirmed.
Candidate OCI3 build + shim full2 running. Next strong file-only native attempt4
using existing strict-read current client, then final source freeze/build/accept.


Candidate OCI3 built; shim full2 caught a stale signing-neutral code pin from the
old native library (raw pins already updated). Generated shared LIBK_CODE_SHA256
0d004fe90205a9c60ccd72cfeef0458936793ebdace958c23ed221bf44ce1ed9,
updated verifier/test to that same generated constant; kept mutation rejection.
Shim full3 passes73/3 ignored. Candidate native attempt4 failed closed BEFORE VM
entry: system image builder still emitted hardcoded old native archive SHA.
Execution cleanup [] retained. Fixed actual macOS builder to derive native identity
from checked-in runtime-assets.json via existing jq, validates target/roles/kernel
addresses, assignment before export preserves parser failure status. Added real
parser tests: 2 passed incl5 negative subcases; no assertions removed. New OCI
candidate4 rebuilding now with correct manifest and code pin. Need run focused
strict live read +16 EPERM native attempt5, then final full asset acceptance.


Native attempt5 timed out before authenticated Agent; cleanup [] retained.
Instrumented attempt6/7 also retained; not blind retries: sample6 caught hashing,
late sample7 shows VCPU waiting in Hypervisor and FS workers waiting in kevent,
rootfs disk4GiB. Native library/firmware loaded exact new/old pair. This is NOT
native acceptance. GIC string comparison showed both old/new libs already have
native GIC symbols; do not add an unproven GIC fallback or firmware port.

Found new-source root disk API can create an implicit temporary virtio-fs bootstrap
outside Seatbelt grants when no root is configured. Parent now uses the existing
krun_set_root to pin the ALREADY OWNED runtime share as bootstrap BEFORE selecting
the read-only system disk. Updated all3 macOS callers (VM/Agent/guest-command),
keeps system disk first among block disks, immutable root and permission boundary.
No new host directory grant or broad sandbox rule. Shim full4 running/passed log;
next candidate OCI5 build and targeted native attempt8. Need validate this actual
boundary correction before claiming it explains the boot stall. Global tmp names
are not ownership proof; never delete unrelated native temporary directories.


## MMIO root-cause checkpoint

Candidate5/shim4 compilation failed on an unverified bootstrap guard type error.
Before fixing it, inspecting native C API showed krun_set_root_disk_remount
explicitly rejects a preexisting root. The speculative bootstrap edit was fully
withdrawn from Context and all3 callers; not a validated root cause.
Confirmed actual defect: MMIO queue_notify writes the queue index to EventFd.
After repairing counter semantics, queue0 writes0 (no event) and FS/block stall.
New native MMIO regression first failed0/1/0 at queue0; fixed caller to signal1
for any selected queue. Focused1, full devices61 +utils21, native Clippy and
release4 pass. No assertion removed, timeout increased or firmware/GIC changed.
Native source3/source tar, new native archive-v2 and old candidates retained.
Next candidate6 +strict native attempt8, then frozen final installed acceptance.
Stage009 still incomplete. Continue W4 after acceptance, no final checkpoint.


Strict native candidate6 attempt8 PASSED1/0/0 in40.40s. Actual files-pep-g1
report confirms immediate full-length live read,16EPERM/EACCES, cleared Guest
readonly superblock/fresh mounts, fork child and real RW writeback. Cleanup[]
and loader hashes recorded. It is diagnostic, not final archive acceptance.
Source freeze3:7f46875ee247f2814e86ee899aad15af90ada493029b986392b73a696fbb8f67.
Final Box3 building. OCI final3 archive retained; final4 corrects an inherited
nativeSelection description only, payload unchanged; final4 installed in newprefix.
Native bundle d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc,
library7dcaf94032e22d2044aed36b7c43f17dd311fed65369f0f292c2e65337856ddf.
Parent fmt/Clippy4/workspace2 pass; verify2 running. Need final full installed
native matrix, identities and owned resource audit before stage009 closure.


Final installed full native attempt9 ABORTED(-6) at353.92s, no completed Rust
test result, no final result/cleanup. Foreign rootfs/files lifecycle, main RO/file
PEP and cap-drop ALL reports passed before failure. Installed CLI create worked,
then first CLI exec saw Guest protocol already closed. Host stderr reports90s
smoke-worker timeout. Cleanup failed remove(state killing), then Host destructor
failed shutdown. Do not mark stage009 accepted or ignore this. Maintenance9 via
existing canonical manager starts next, owned leftover3records/processes only.
Investigate shared smoke-worker lifetime vs product owner; never merely increase
timeout or weaken full matrix. Final Box3/OCI4 retained as candidates.


Maintenance9-1 PASSED1/0/0 in3.31s, cleanup[]; owned workers reaped.
Confirmed lifecycle defect: product agent-vm-smoke used bounded90s diagnostic
wait despite an installed exact-parent owner watchdog. Small real-child test
first failed owner mode (1pass/1fail), then repaired closed Smoke/RuntimeOwner
mode passed2. Product mode is selected only after existing owner_process::start
validation; direct diagnostics retain90s, startup/request/cleanup budgets unchanged.
No new session/state/config, no mere timeout increase. Full Shim all-targets7
passes75lib/3ignored +17bin/3ignored +3integration. Dedicated native >95s
owner test added (default ignored, must execute explicitly). Candidate lifetime1
ready, client metadata building. Next native attempt10, freeze4 +new Box4/OCI5,
full installed replay/verify3 and resource audit. Still NOT accepted.


Native lifetime candidate attempt10 PASSED1/0/0; actual before/after commands
span97.535s (>90), full test135.11s; cleanup[] +loaderidentity. This diagnostic
used lifetime1 candidate; final package replay remains. Review caught standalone
CLI diagnostics also require owner PID, so the product lifetime MUST be explicit:
added macOS-only hidden --runtime-owner from Runtime AgentSession, selected in
Shim only after exact-owner watchdog registration. CLI default and library default
remain bounded90s; invalid flag values refuse. Parser test first failed due its
missing required --runtime-share fixture argument (full8 retained), fixed fixture
without changing assertions; full9 running. Native dependency bytes unchanged.
Box4/OCI5 freeze55fdf... retained; rebuild NEW source5 after this caller correction.
Verify3 predates flag correction; run verify4 once source stable. Next freeze5,
Box5/OCI6 isolated install, final full native11 and installed >90s12, cleanup.


## Final5/6 acceptance running

Source freeze5:64abc3d1a882bea0582ccdfb7a493480a720ac750e1ca904cf3ffbb1738d6ccc.
Box5 db389464ab8dfad31138d50e5dcc434001b00c34cffe004fb9a3d31d8aae7acc,
23331587compressed/44060672installed; OCI6
0195f714cca8d511c58136fd4bb56134c2daf20c277b1bd3a45dddb5f6dc6160,
33331039compressed/116031488installed. Both11verifiedfiles, fresh isolatedprefix.
Source/ELF/arm64/signatures/embeddedAgent identity5 passes. AFfinal2 all5
scenarios are exact-byte applicable: five Box payloads match final5, proof in
checks/af-final5-equivalence.json. No new AF execution claimed.
Runtime AgentSession focused18 passes; Shim full9:75lib/3ignored,18bin/3ignored,
3integration. Runtime full3 +verify4 running. Previous verify3 passed, but predates
explicit mode flag and is not the final gate. Final installed native11 fullmatrix
and12 >90s owner proof now running; both use actual installed Box5/OCI6 and source5
client, isolated roots with sentinels. Next inspect exact counts/results/loader maps,
run owned inventory/descriptor/socket/source audit, update OUTCOME/ledger/release
only if allpass. Then next W4 concrete rollback-safety investigation, not final.


Bootstrap hypothesis closed by source review: agent worker already uses env_clear
with TMPDIR set to its pinned private runtime share (macos_agent_smoke.rs). Native
implicit bootstrap roots are therefore inside that grant and scoped cleanup;
there was no need for krun_set_root before root_disk_remount. The attempted
bootstrap edit was withdrawn. No global temporary-root deletion is authorized
by filename or timestamps alone. Actual queue-zero regression fixed the boot.


## Cleanup audit failure — stage remains open

Final installed native11 PASSED1/0/0 (397.88s), final lifetime12 PASSED1/0/0
(136.75s, actual commands span97.755s), verify4 passes, Runtime435/1ignored.
Audit failed on4real leftover retryable Creating generations from attempts4–7:
4GiB disks and live hardlink projections retained in their exact Host roots.
No broad/raw deletion done. Source shows SDK deliberately retains a retryable
Create intent; force-delete Creating refuses until original operation reconciles.
Box kill changed Starting ->Killing with no published OCI binding, then
current_runtime incorrectly treated non-Starting as absence, allowing Box records
to be forgotten. Add shared fail-closed check: require Host absence before cleanup
if Killing has no binding; do not discover/adopt an arbitrary current generation.
TDD unbound_kill covers4Host states +never-started reservation. Next fix proven
failure, reconcile4old intents through exact public SDK replay thendelete (never
start workload), add native authority/cleanup proof; rebuild affected archives.
Creating cancellation is not implemented merely by refusal; retain as next W4
capability gap if safe cancellation requires separate bounded protocol work.
No stage acceptance or final response until actual owned cleanup succeeds.


Unbound-kill regression first failed1/1; shared guard now passes2 (fourHost
states plus never-started absence). Does not adopt current generation when
binding is missing; retains Killing reservation and its data on uncertain ownership.
Historical pending4 exact SDK replay failed terminally because old Box cleanup
had already removed its workspace directory. Failed0/1/0 log retained; no VM
launched. SDK settled original Create as Failed and quarantined originalrecord;
its4GiB disk/projections remain, proving an additional shared gap:
UtilityVmDriver.prepare_create_bundle terminal error clears pending session but
does not clean existing handoff. Add exact-config/target-bound rejected-handoff
cleanup; cleanup failure MUST remain retryable so durable owner is not discarded.
Two focused real-handoff tests added; run before fix. Old4 needs offline existing
cleanup API under durable-root lock, matched failed receipt+quarantine+manifest,
no data recreation or guessed deletion. Old5–7 pending will use repaired Host.
Native failed-create test13 (entitlement withdrawn on ownedinstalledShimcopy)
added, will prove bindingless kill refuses, original Create replay and exact SDK
delete permit cleanup without starting workload; default ignored mustrun.


Shared Runtime terminal preparation regression: test1 left an owned handoff
before fix; new exact target/config cleanup removes it. Test2’s marker-conflict
expectation was initially the wrong error code: source ensure_marker and its
existing tests require Conflict. Corrected only that exact expected type; retained
retryability/no-deletion/caller-content assertions. Both now pass. Cleanup failure
returns retryable and keeps pending ownership, including other terminal-create
cleanup sites, so journal claims cannot disappear before resources are reaped.
New offline maintenance test locks the actual durable root and requires no active
container/process, failed Create receipt +quarantined record +matching config/
attachments; invokes existing identity-bound cleanup_prepared API. Used only for
old attempt4 whose caller workspace had already been deleted by old unsafe Box
kill. Other pending failures will settle with repaired Host, no fake workspace
recreation. Audit process matching corrected to full command executable because
macOS comm column truncates long paths; no weakening of process cleanup checks.


Offline terminal-repair4-1 PASSED1/0/0 in0.91s using exactfailedreceipt/quarantine/
config+attachments under DurableStateStore exclusive lock and no active runtime
process. Existing cleanup_prepared API removed old4disk/projections only, source
files retained. No fake workspace or Box state was recreated.
Shared tests exposed an existing shared-session empty-root assertion after
keeping pending ownership until cleanup: current pending target was incorrectly
counted as another owner. Reused registry and excluded only this exacttarget
from other-owner check;40sharedtests now pass, old11 remains and empty12reaped.
Nativepending3 build failed on protocol crate not being a Box dependency; use
already-established artifact path contract share.with_extension in this test,
not a new dependency. Clientpending4 rebuilding. Next candidatecleanup1 +
old5/6/7terminalsettlement, native13 failedcreate/reconciliation, owning gates,
newarchives+affectednative/AF and finalresourceaudit. Then continue W4.


Pending old5/6/7 settlement each PASSED1/0/0 with repaired cleanup1 Host.
Their missing old Box workspace triggers original SDK Create terminalfailure,
which is replayed exactly; Host listempty and disks/projectionsgone. Zero new
VMs/starts; not a VM or readonly positive claim. Old4offline cleanup passed.
Shared cleanup40passes; Clippy1passes(existingwarnings). Root41diskorphan audit
must rerun aftercurrentnative13. Prototype13 nowexecutes real no-HVF-entitlement
negative on ownedcopy +restored installedShim, sourceManagerguard, exactSDK
Create replay/delete (never Start) and retained callerdata. Prototype intentionally
does not invoke oldBox5CLIguard; finalnewBoxmustrepeat using actualinstalledCLI.
Boxnativeclientpending5 compiles; a prior test-only protocol crate reference was
removed via existing documented artifact paths, no dependency added.
Runtimefull4 running aftersharedcleanupchanges. Source not frozen yet. After13,
freeze6, rebuildBox6/OCI7, verify5, nativefull/failedcreate+AF, identities/audit.
Then W4 nextknown correctnesstask rollback metadata or pendingCreate cancellation,
keep continuous task active.


Native prototype13 PASSED1/0/0 in53.84s: ownedinstalledShimcopy has noHVF
entitlement, failedCreate retains2callerIDs, unboundkill refuses andKeepsKilling;
restoredactualShim exactoriginalSDKCreate resumesCreated and forceDelete reaps
without anyStart; caller3files unchanged/nlink1; cleanup[]. ActualCLI guard is
reserved for rebuiltnewBox archive (oldBox5 remainsunsafe for this path).
Postrepair audit PASSED18homes:0executions/volumes/processes/mounts/disks,
127observedsocketsabsent,8immutableblobsunchanged,7initialRAMidentitiesuntouched.
Saved complete after-repair audit copies; no audit-wide deletion.
Cleanupcandidate2 building from currentstableSource. Runtimefull4running; once
candidate2sourcecheck succeeds freeze6, buildBox6/OCI7, verify5. Finalaffected
native full14/failedcreate15withinstalledCLI +AFfivecases usingBox6; identity
check andupdatedowned-resource audit/evidence. After009close continueW4.


## Final accepted checkpoint — continuous work continues

Spec009 qualified with source6 `3d07d7318f277094813eb973e746339c8fce5934bf95bcf1021a9011a7e54ca5`.
Box6 archive `a35388f45b1c326d6fd549a30fb3545e30176194e3b8f7bdb088a9016df313bb`;
OCI7 archive `ce2b65c4b93b8843b9d4a9b897e5b37e217a78895458ae5fd7f2e58586fd9239`.
verify5/fmt/Clippy/workspace pass; actual Box2179/18ignored, Runtime437/2ignored,
SDK213, protocol63, Agent25, shared40. Full installed native14 1/0/0,
installed CLI failedCreate15 1/0/0; final6 A–F all five scenarios pass.
No source change since freeze6. Final audit25homes/215absent sockets:
0managed executions/volumes/processes/mounts/workload disks/descriptors;
caller sources/parents/canaries and initial7RAM identities preserved.
Final review-source6 verifies15preserved archives,50references, supplemental
source hashes, clean parent submodules, empty index and diff check.
See docs/evidence/runtime/20261114-oci-files/OUTCOME.md for all commands,
failures, exact counts/identities, cleanup and limitations. No runtime memory
measurement, native cross-platform or support/default/readiness promotion.

Next immediate W4 local correctness task: reproduce old accepted Box008 CLI
reading new caller-file authority metadata, including pathname replaced by a
directory. This is a source-analysis concern, not yet a proven runtime failure.
Create bounded010 and test real old CLI before choosing shared metadata fence.
Preserve legacy no-file records and all009 source identities/routes/generations;
no weaker Exec fingerprints/new resource state machine. Then complete focused,
owning gates/new affected archive/native replay/cleanup and continue W4 again.

# Tasks036

- [x] Trace unowned canonical deletion and shared image-hash build directory;
  prior035 committed074321a6, private verification rule enforced.
- [x] Actual isolated filesystem red for stopping an unbuilt pool deleting a
  pre-existing caller template. No real user-home operations.
- [x] Implement actual directory/consumer ownership, independent private build
  paths, safe cleanup/retry; retain original owned-directory deletion assertions
  with explicit valid ownership, add adversarial/cross-pool controls.
- [x] Focused/full/default/Clippy/fmt, rebuild affected source-bound assets and
  installed native Mac cold-pool/filesystem checks; no Linux snapshot-native claim.
- [x] Full own resource/source/cache/archive audit/outcome/docs/diff review.
  Task-only local commit is the immediate next action; its receipt is recorded in
  the owned artifact root, then continue outer pool/legacy manager lifetime.

Checkpoint: no036 VM, source freeze or artifact build. R41/context provides exact
paths; its run-check.py now always supplies a new private K_HOME and short owned
TMPDIR for every command. Targeted tests must use it or explicit isolated env.
Prior source629e2bac…/Box4093b9d5… remains accepted, and035 did not rerun VM native.
Historical default-home nonmodification is unestablished; append-only corrections
are in033/034/035. Do not scan/clean real ~/.kbox. External Git UID501/DNS, six disk
failures, prior SDK failure, other hosts/Windows ARM64 remain parked. Full GA active.

Checkpoint after actual red: unowned-red1 exits101, parent0pass/1fail and child
0pass/1fail. The child-owned home/non-ASCII caller directory was deleted by an
Unbuilt empty pool; actual path and panic retained. No real home touched.
All future tests use R41/run-check.py (explicit fresh check-home + short P41/t).

Implemented pending wiring: warm_pool/template_directory.rs (cap-std pinned
private tempfile allocation, identity/no-follow removal, actual Arc consumer
references and explicit close guard; unclosed drop poisons cleanup conservatively).
Sibling template_directory_tests.rs covers independent private allocations,
caller preservation, close/drop refusal and path replacement. Module imported
in warm_pool.rs. directory-primitives1 handle15255 just awaited; inspect its
receipt before trusting results. This is not yet used by production callers;
original unowned regression remains red, no phase acceptance/commit.

Next implementation: integrate one actual template-resource owner into the
existing WarmPool/TemplateState flow (not a second lifecycle machine). Keep
owned directory and any failed-stop source VmManager, allocate a unique directory
per pool/attempt instead of shared image hash, attach TemplateUse to restored/
source managers and release only after actual stop or proven prelaunch failure.
Directory cleanup refuses live/unproven consumers and replacement; don't infer
ownership from state/hash. Update original positive cleanup tests by recording
explicit actual fixture ownership, retaining their !dir.exists assertions, and
add unowned Failing/Unavailable/other-Ready canaries. Missing Linux snapshot native
host remains a separate gate; Mac real filesystem and cold-pool native possible.

Unchanged broader blocker: pool destroy_one/destroy_vms/boot_or_restore/release/
eviction and CLI pool drain_all discard managers or ignore errors; CLI Stop ACKs
before final cleanup, and legacy CLI boot/compose rollback can drop a failed VM.
These remain required work, not solved by directory primitives. Source VM/template
cleanup must not delete its input after a failed stop. Dir cap remove is handle-
based but final concurrent rename is not atomic; enforce private owned namespace
and refuse observed replacement, do not overclaim hostile-parent races.

Current accepted commits: a85fb534 (033), c634d76e (034),074321a6 (035).
Runtime archive4093b9d5…/source629e2bac… prior accepted; current036 Rust changes need
new source freeze/rebuild/native/audit after implementation, not yet done. Source
status otherwise only036 additions; no native VM started. Full GA active.

Latest checkpoint: directory-primitives1 failed1/3 because tempfile default0755;
explicit0700 fixed it, directory-primitives2 passed3/3. Production wiring now
retains TemplateResources (exact directory/source manager), guards restored/source
VM consumers until actual backend finalization or proven prelaunch failure, and
refuses private-input workload grants. Retry checks exclusive ownership before
rewriting mapped inputs. wiring-check1 found4 test-fixture compile errors; all4
updated with explicit resources/ownership and original deletion assertions kept.
template-focused1 is running (session89824); inspect exact result before relying
on it. Added cross-pool/caller preservation, active-consumer retry and attempted
overwrite controls. No036 freeze/package/native run/commit yet. Outer manager-loss
and cancellation/retention lifetime are still required next; don't claim solved.

Latest actual results: template-focused1 52/0/1; template-focused2 16/0;
pool-final1 54/0/1; vm-template-stop1 4/0/1; clippy1 passes. verify1 actual
2313/6/51 exit101 (same six external disk failures); workspace/fmt/isolation4 pass.
CLI935, full contract tests and installer8 separately executed (counts pending
parsing). Candidate1 source039d2717…/1723 files, archivefd27b0b4… preserved with
11-file install identity. Native fault1 actual0/1 despite internal resultPASS:
first real close/input removal succeeds, second final cleanup failsNotFound.
Owned VM/volume/socket are actually absent; do not count this as native pass.

Root cause reproduced: APFS fstat(open unlinked dir).st_nlink remains2, not0.
unlinked-fd-metadata.json proves it; removed-retry-red1 actual0/1. Shared directory
now records successful capability-based unlink inside the blocking worker, so
retry never depends on nlink and can't delete a same-path replacement. pool-final2
55/0/1 and dropped-waiter test1/0 pass. All assertions retained; cold fallback now
keeps private artifact grant checks; ConfigError is propagated instead of cached
as unavailable. Candidate1 is obsolete and cannot qualify current source.

Current new check vm-only-red1 (session19038): inspect expected missing pool
module reference under --no-default-features --features vm. Correct owning layer
by moving TemplateDirectory/TemplateUse to vm (pool depends on vm, not vice versa),
then compile same feature combination. No new features/dependencies. After that,
freeze source2, rebuild/install final2, run native fault2 + AF + cold-pool matrix,
full/default/final audit. Sessions87841/50427/66863/75244/34973 terminal. No other
036 native resources running from fault1; complete final scoped audit later.

Current checkpoint: vm-only-red1 found the new dependency and an old unconditional
prewarm field. Moved directory/use types into vm and gated the pool-only prewarm
field/fixtures; vm-only-green1 exits0. source2 frozencc8175d2…/1723 inputs; final2
archive d5393321… built/installed,11 files and source/native/Agent identity pass.
Ownership19/0, VM close4/0/1, final Clippy/workspace/fmt pass; actual verify2
2315/6/51 exit101, same six disk errors and isolation4/0. CLI final2 935/0.
contracts1 actual118/0,installer1 actual8/0, unchanged contract/install source.

Native fault2 actual1/0/0 passes (real VM, synthetic one-shot finalizer error,
guard/input preservation, actual close retry, repeat cleanup/caller/no-late-write).
AF final2 all5 rows PASS with actual2 Rust native +3 Python scenarios and loader
proof; no internal result.json alone used as Pass. Harness initial nested-context
root assertion failed before any resources; discovery fixed with same confinement
assertion, receipt retained. Candidate1 native0/1 and archive remain as failures.

Pool native wave final2 running session46013:4 daemons/16 initial Ready VMs/4 Guest commands,
one Mac unsupported-snapshot cold control, exact old hash-named caller canaries
and installed-library capture. After its result, audit all native/check homes,
both verifier short roots, old/new fault scopes and exact sockets; then finish
outcome/release/W4/KStudio pointers, diff review and task-only local commit.
Remote UID501/DNS still fail on fresh prerequisite probe; old baseline branch
already ancestor of main. Do not claim remote push/GA or Linux snapshot native.

Final native wave exits0:4daemons/16 initial Ready VMs/4 commands, all4 pre-existing
template dev/inode/bytes retained,16 actual installed loader maps. Final audit1
passes41 private homes,35 VM socket dirs,146 sockets,2 engines,699 immutable cached
files, both verifier short roots; no own managed execution/volume/process/mount/
disk/FD/socket remains. Audit deletes nothing. Native final3Rust+3Python pass,
failed native1 retained0/1, no snapshot-native/actual OS stop error claim.
Outcome reflects final cc8175d2…/archive d5393321…; documentation/diff/stage/commit
closeout next. All tool sessions through32533 terminal. Outer manager lifetime
remains required continuing local work; GA active. Remote prerequisites unchanged.

Closeout preparation: accepted bounded036 only, all requested native/owned checks
recorded in OUTCOME. Task-only staging/commit follows these facts; the subsequent
R41/git-closeout.json proves actual commit/source/archive/clean status, rather than
promoting a planned Git step to completed remote synchronization. Current source
remains cc8175d2…; no runtime source change after final2. Next required local gap is
outer pool/legacy caller manager loss after failed finalization. Full goal active.

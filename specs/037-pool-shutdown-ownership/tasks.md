# Tasks037

- [x] Preserve committed036 and read actual pool/CLI shutdown callers.
- [x] Actual isolated false-success/owner-loss red.
- [x] Shared registered-owner retention and truthful shutdown/CLI response/retry.
- [x] Focused/owner/default/Clippy/fmt, frozen rebuild/install/native/owned audit.
- [x] Outcome/release/W4/KStudio evidence and final source/diff review.
  Task-only commit follows; its artifact receipt proves actual Git result.
  Continue initial endpoint/pre-admission/legacy local gaps, full goal active.

Baseline036 committed86a6bf08; artifact/source/native/audit closed at R41/git-closeout.json.
Current root /Users/roylin/code/kense/kbox/artifacts/pool-shutdown-ownership-fe886a631080491db66abe1b4f2deac8, private root /private/tmp/kbox-pool-stop-fe886a631080. Use R/run-check.py for every Cargo check/test;
it allocates fresh K_HOME and owned short TMPDIR. No real user-home operations.
Default036 actual2315/6/51, CLI935, contracts118, installer8;3Rust+3Python native,
4daemon/16initialReady/4Guest commands;41-home/35VM socketdirs/146socket audit clean.
Latest accepted sourcecc8175d2…/Boxd5393321…; no037 freeze/native/assets yet.
Remote UID501/GitHubDNS still fail; baseline branch already ancestor of main,
no fresh remote synchronization claimed. Linux snapshot/WindowsARM64 native
unqualified, full default OCI cutover/W4/GA active. Do not repeat parked blockers.

Actual drain-red1 exits101,0pass/1fail: published pool returnsOk after synthetic
backend stop refusal. New sibling shutdown_ownership_tests.rs keeps all actual
handler/drop-count/working-data/retry assertions. Initial implementation stores
actual Arc<Mutex<VmManager>> handles before any async bulk teardown, releases only
after actual destroy success, propagates first original error, and retries retained
handles on later drain. It never uses ambient-home orphan reap in this new owned
bulk path. Existing VmManager remains resource authority; collection has no new
lifecycle status. drain-green1 passes1/0 and proves same actual handler retained
until retry. Default pool stats are cleared before returning the failure.

Shared bulk call sites wired:drain/drain_idle, fill rollback, snapshot invalidation
and maintenance eviction. Non-shutdown bulk errors log/close admission where the
caller owns shutdown signaling. Latest patch makes eviction return Result and
stops maintenance via existing watch sender on failure. This latest patch is not
accepted yet; bulk-focused1 running (use actual tool session returned for it).
No037 frozen source/native/client/archive/commit yet. Only Runtime warm_pool.rs,
new shutdown_ownership_tests.rs, specs037 and evidence037 are dirty.

Next:inspect bulk-focused1; add cancelled-waiter ownership/retry control; wire CLI
leases retention and drain_all Result, existing StopResponse.error, request barrier
before cleanup, exclude Stop from its own request accounting and keep registry/
listener alive on failure. Actual CLI lease.vm already Arc<Mutex<VmManager>>;
retain entries/permits until success, avoid dropping them before awaiting stop.
serve currently adds request_guard outside handle_conn, Stop ACKs before cleanup,
and drain helper unlinks socket/drains before waiting requests. Normal signal and
startup prewarm error callers also ignore drain errors; fix caller ownership,
not just the Result signature. WarmPool::start only errors before allocation;
fill failures still return a pool, but publication-after-draining and bootstrap
cancellation/lost-manager paths remain explicit continuing work.

Existing foreground acquire/release/boot/postboot restore/shutdown-late-boot code
still calls destroy_manager_or_reap and can drop owners after Err; don't claim
globally fixed. Linux orphan reap uses ambient home and is unsafe for an actually
retained manager; remove only together with complete ownership-aware callers.
Legacy CLI boot/compose rollback also remains open. Possible pre-existing socket
path clobber/removal in serve must be audited against an existing owned-endpoint
adapter before any broad cleanup claim. No real user-home/system/DNS changes.

Latest actual continuation: bulk-focused1 Runtime pool51/0/1. CLI sibling tests
pool/tests/shutdown.rs actual cli-stop-red1 0/2 (early Stop ACK and lease map
removal), cli-stop-green1 2/0, cli-pool-focused1 40/0. CLI drain_all now returns
failure and snapshots actual lease Arc handles without removing entries/permits;
success removes only matching owner identity. Release/expiry share success-only
removal; failed TTL cleanup no longer increments success count. Existing Stop
wire error field now carries close failure; Stop excludes itself from request
guard, closes admission/waits before teardown and signals daemon exit only after
clean result/write. Signal failures keep service alive; startup guard covers
prewarm, failed startup cleanup keeps owning service for operator retry. Late
initializer publishes its actual owner even if admission already closed.

Further actual resource-lifetime red:cancel-handler-red1 0/1 proves retaining a
VmManager alone was insufficient. destroy_with_rootfs_policy took handler into a
local variable across guest-control awaits; cancelling actually dropped handler.
It now remains in its existing owner slot under async write guard, taken only
after actual stop; cancel-handler-green1 1/0 proves retained exact handler and
retry. This changes shared lifecycle; final full/default/native requalification
is still required. No037 build/freeze/native/commit yet.

New mandatory dependent root bug:host-cleanup-red1 0/1 proves rootfs cleanup Err
still wiped the working directory. Rootfs-clean success now gates directory
deletion. host-cleanup-red2 running (actual session from tool) must retain original
canary assertion then expose second failure: state Stopped early-return can skip
unfinished cleanup on retry. Fix that shared cause without inventing Ready while
provider stopped or losing errors/metrics/idempotent success. Consider a VM-local
cleanup-complete observation (inside same authority, no duplicate lifecycle),
reset on shared boot entry, publish only after successful host cleanup. Preserve
guest persistent handoff proof on retries; missing quiesce must still refuse.
All original/new assertions remain. No mock/cross compilation native claims.

Latest root results:host-cleanup-red2 actual0/1 reaches the original retry assertion
after directory-preservation fix, cleanup count remains1 instead of2. First
host-cleanup-green1 was a compile failure (moved _handler_stopped variable outside
scope); retained raw receipt, not a green test. Corrected shared implementation
adds VM-local cleanup_complete (resource observation, no second execution state),
resets at common boot entry, publishes only after actual host cleanup succeeds.
Stopped remains actual workload state; unfinished host cleanup retries. Repeated
successful close remains a no-op. Rootfs cleanup success now gates working-dir
removal. Persistent Guest quiesce/record-clean validation runs again on incomplete
close retries so missing handoff cannot become clean by retry alone. Stopped
metrics/event are emitted once. All constructors/layout fixture updated.

host-cleanup-green2 1/0; pool-focus2 53/0/1; vm-focus1 243/0/2. All sessions through
56761/13049/21310 terminal. CLI pool focused40/0 precedes the last shared VM changes,
will be requalified with final source. No037 native run, freeze, package, full gate,
Clippy or commit yet. Stage03686a6bf08 remains latest accepted installed source
cc8175d2…/archive d5393321…; dirty037 is not acceptance. Need finish idempotent/
persistent-proof controls, review scoped socket ownership, actual native pool
stop-refusal/cancel/retry (real VM with explicitly synthetic one-shot boundary),
final installed CLI/A–F/cold-pool and owned audit before task-only commit.

Added ignored Mac-only native fixture warm_pool/shutdown_native.rs, actual entry
pool::warm_pool::shutdown_native::native_published_pool_retains_vm_across_cancel_refusal_and_actual_close_retry.
It cold boots a real VOLUME VM, records IDs/data/allocation, withholds a synthetic
Guest-control socket ACK to cancel stop, then injects one finalizer refusal and
proves live writes/exact ownership before actual close/no-late-writes/template/
anonymous cleanup. Explicitly no actual OS stop-failure/Linux snapshot claim.
native-fixture-check1 all-target check passes; not run yet. Fake-server finally
now selects finish even before accept, preventing a fixture failure hang.

Native fixture trace uncovered another necessary close gap: normal ephemeral
destroy never removed its own newly-created anonymous allocations; the existing
boot-failure helper mem::take lost failed allocation identities. anonymous-close-red1
actual0/1 proves even a wrong anonymous owner binding was reported clean by close.
Added caller/shared/reused preservation and same-owner retry assertions. Temporary
normal close now invokes shared helper after attachment cleanup (Arc-cloned state
lock retains same authority while borrowing self); anonymous-close-red2 running
session35847 should expose the helper's lost failed name. Next refactor that shared
helper to Result preserving first typed source, retaining failed created names and
removing only successfully deleted names from anonymous_volumes. Boot-failure caller
uses .is_ok for existing best-effort boolean; normal close propagates source. Keep
persistent/preserve-rootfs and reused volumes intact. No new state/config/dependency.
No037 source freeze/native build/VM start yet;036 archive remains accepted.

Latest receipt checkpoint:anonymous-close-red2 actual0/1 proves lost created names;
shared helper now Result preserving original ConfigError, retains failed names,
removes only successful names. anonymous-close-green1 1/0 and all close-controls1
6/0 pass. Completion observation refined to completed_cleanup:Option<bool>
(success + exact rootfs-preserve policy), so preserving stop cannot suppress a
later full ephemeral remove. Idempotence/one-stop metric and persistent missing
Guest-handoff retry refusals are tested. No duplicate execution state/config.

Socket shutdown-replacement red1 actual0/1 proves caller replacement file deletion;
shutdown now compares bound socket dev/inode/type using the existing Engine pattern
before unlink, returning an error and preserving replacement. cli-shutdown-green2
3/0; Clippy1 and vm-only1 pass. Initial bind's old unconditional unlink/bootstrap
listener failure race remain distinct next local work, not claimed fixed here.

Owned cache prepared714 files; source1 frozen2fb83f23…/1726 inputs, candidate1
archive71b56441… built/installed23414236/44191744 bytes,11 checksums/source fields.
Actual verify1 2321/6/52 exit101 (same six disk failures), isolation4/workspace/fmt
pass; CLI938/0,contract library24/0,installer8/0, freeze-client1 pass. Full contract
118 remains preceding036 unchanged-contract evidence, not a new037 full run.
No037 native VM/client execution yet; candidate1 retained but no longer final.

Final caller review found the BG maintenance shutdown-mid-replenish branch still
discarded its just-ready manager after a stop error. It now uses the shared actual
retaining bulk helper (no new abstraction); late-replenish1 running. This Rust
change requires source freeze2/rebuild/install2/relevant checks before native.
Do not use candidate1 or its test client as final source. Foreground acquire/
release/boot-error/restore/postboot local managers still have separate pre-admission
owner-loss paths; required continuation after bounded published-close acceptance.
All earlier test/package sessions through44865 terminal; no live037 native resource.

Current final-candidate checkpoint: late-replenish1 56/0/2 after using retained
bulk for shutdown-mid-replenish Ready VMs; removed now-unused mut. Listener is
bound synchronously into BoundPoolListener (actual listener/identity/existing
control channel) before prewarm can create VMs. Startup failure signals that exact
owning server and awaits its identity-checked unlink; no blind post-abort socket
unlink. cli-bound-owner1 all41 pool tests pass, including existing real startup-
prewarm-error socket cleanup. Initial bind stale/input-object safety is still a
separate next task; no arbitrary same-UID namespace-race guarantee.

Shutdown admission exposed an existing PEP gap: watch flag was not checked by
acquire/queued boot. close-admission-red1 first failed its fixture queue precondition
(default boot concurrency exceeded one held permit); no product-red claim for
that first run. Correct single-permit fixture retained every assertion; actual
close-admission-red2 0/1 proves queued boot enters despite shutdown. New shared
ensure_admission_open at acquire/under-idle-lock/after boot permits fixes it;
close-admission-green1 1/0. No isolated VM starts in these tests.

Source2 frozen dde4c61ac1c2348111de6c8ca72d512836d47637cd7bacc6744fc20cb6bdd33d,
1726 compiled/package files. Candidate2 build running session15137. Final checks
(Clippy2, verify2, CLI-full2, freeze-client2) running session41660. All earlier
sessions terminal. No037 native VM run yet. On completion copy the actual test
executable from freeze-client2 JSON into R42/native-client-final2, update context,
check-final-identities2.py, then native_stop2.py (new published-pool entry already
adapted), AF runner + final2 context with image=baseImage, then pool_stress.py.
R42 scripts and714-file owned image cache prepared; baseline retained. Never use
obsolete candidate1/source2fb83…/archive71b564… to qualify current source.

After all actual counts/source/native/cleanup proof, write037 outcome/docs, scoped
commit, then continue startup socket binding/foreground pre-admission/legacy owner
loss. User Git authorization stands; latest committed86a6bf08, old baseline merged,
remote account/DNS still blocked, no force/publish. Full goal stays active.

Native-start checkpoint: final2 package6326607e… built/installed23415039 compressed/
44191744 installed bytes; all11 files and signatures/arch/source/Agent identities
pass. Actual verify2 2322/6/52 exit101, same six disk failures; isolation4/workspace/
fmt/support pass; Clippy2 and CLI-full2 938/0 pass. Contract library24/0 and installer8/0
actual this phase, preceding unchanged-contract full118 not rerun. Source dde4c61a…
unchanged. Native client6f75291a… frozen from actual compiler-artifact JSON.

Native fault2 actual1/0/0 passes against final2 installed shim/guest-init/libraries,
with loader proof. Actual VM writes continue after cancelled synthetic control wait
and one-shot refusal, exact allocation/handler/template retained, actual retry stop
removes created anonymous volume and no late writes; real OS failure/snapshot not
claimed. AF final2 running session42857 (3Python+2actualRust target); owned final
pool matrix not run yet. R42/final-audit.py prepared from accepted036 audit without
nonexistent old-fault1 assumptions. Execute only after all native matrix results;
cover P42/R42/allcheck homes/verifier short roots/loader sockets/immutable cache,
retain superseded candidate1 and prior archives. All other sessions terminal.

Final native matrix checkpoint: AF final2 all5 rows PASS, actual2Rust+3Python with
loader proof; together with fault2=3Rust+3Python. Pool-wave-final2 passes4 daemons,
16 initial authenticated Ready VMs,4 Guest commands, exact4 pre-existing template
identities retained (one unsupported-Mac-snapshot cold control). All matrix tasks
terminal. Final-audit1 running (tool session); no further Runtime source changes
after dde4c61a… final2. Fresh Git probe again fails UID501 and DNS, parked; baseline
branch ancestry still0/merged. Complete audit/docs/diff and task-only commit,
continue next known local gaps; do not end whole goal.

Final-audit1 passes52 owned homes,35 VM socket directories,146 exact sockets,
2 engines,713 immutable cached files and both verifier short roots; no own managed
VM/volume/process/mount/disk/FD/socket remains. Caller sources/4 template canaries,
prior and candidate1 archives retained; audit deletes nothing. Final outcome/docs
reflect source dde4c61a…/Box6326607e…/client6f75291a…, native3Rust+3Python and
4daemon/16initialReady/4commands. All tool sessions through23710 terminal.
Task-only stage/commit next, then R42/git-closeout.json records actual clean main/
source/archive/commit identity. No remote equality/push or whole-GA claim.

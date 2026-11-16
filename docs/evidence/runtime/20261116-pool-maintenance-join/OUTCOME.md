# Cancelled maintenance join ownership

Bounded043 is accepted on this existing macOS26.6.2 arm64 host. Both public drain
methods retain the exact unfinished maintenance JoinHandle across cancellation;
retry waits for actual worker completion before reporting drained. Full W4,
default OCI Host Service cutover/live continuity/enterprise GA stay open.

## Actual failure and shared implementation

join-red1 actually exits101 with0pass/2fail/0ignored. Each production drain is
polled and cancelled while a real controlled Tokio task awaits an owned oneshot.
The actual unfinished handle is gone from the pool (original assertion fails).
The catcher releases and waits for only the fixture worker before resuming the
original failure. No native VM or synthetic OS stop failure in this hermetic red.

The existing replenish_handle mutex now keeps the handle in its slot while a
mutable borrow is awaited; the slot clears only after completion. Both callers
reuse this small shared join method. Cancellation drops the wait, retaining the
actual join owner; concurrent drains serialize on the existing async mutex. Task
error handling and all other shutdown/admission, typed refusal/retry, template,
ExecutionManager authority, routes/generation and resource contracts remain.
No new lifecycle/config/dependency/workspace/kill loop/native source changes.

join-green1 actually passes2/0/1ignored (new native entry is ignored here).
Pool-focused1:76/0/7ignored; CLI-full1:949/0; Runtime/CLI/contracts all-target
Clippy passes. scripts/verify-commit.sh passes isolation4/support/workspace-
all-targets/fmt then exits101 at Runtime2341pass/6fail/57ignored, with the same
six external RAM/APFS/disk-service failures. No green default gate or skipped
failing assertion. Every command records fresh owned K_HOME/short TMPDIR,
stdout/stderr/exit. Unchanged038 contracts106 top-level(+12 child receipts) and
installer8 are retained history, not new043 passes. Actual commands include:

```sh
cargo test --locked --offline -p kbox-runtime --lib cancelled_maintenance_join
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Source, archive and installed effects

Artifact root `artifacts/pool-maintenance-join-723bc11bc46c4d4d9c19fffa6f8e8022`;
private root `/private/tmp/kbox-pool-join-723bc11bc46c`.
Frozen compiled/package source `1353d5cf2cfed86722f8a85fa74507781438ca1acc5e08aaff8e0603c91146c5`,1729 inputs,
based on main2dd75501 plus043 changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.
Frozen actual compiler-artifact test client `0468cfff3a7f5993f1c995b1afd1461bb22d0001bc209e85deb7466039d8b6ea`.

Final archive `final-dist-pooljoin-1/kbox-3.4.3-macos-arm64.tar.gz` under the artifact root;
SHA256 `ab31000598e9fc9fe2a3fc617181eb8995930f9300e86570f8b5d24fe4722b64`;
23427185 compressed bytes /44208128 installed bytes.
Fresh prefix `installed-pooljoin-final-1`; all11 manifest/source/signature/
architecture/native dependency/embedded OCI Agent checks pass. CLI/shim Mach-O
arm64, guest-init Linux aarch64, strict ad-hoc signatures, actual loader paths/
hashes match installed companions. No development/user installation substitution,
notarization or publication. Parent libkrun3c2889c7/libkrunfwfc3ea068 gitlinks
remain clean/unchanged. Separate unchanged OCI archived5f59995… keeps original
sourced2b442f1…/owning-repository native patch provenance, not relabeled043.
Previous archives and original failures remain.

- New native entry actually executes1pass/0fail/0ignored, covering both public
  drains with separate real VMs/allocations. A controlled maintenance task owns
  an actual registered manager; cancellation retains the same actual JoinHandle,
  manager/handler, PID, allocation JSON/dev/inode and continuing Guest writes.
  The retry is actually polled Pending while the worker remains gated and VM
  lives. Releasing only that owned completion gate lets the worker physically
  finalize the actual VM; retry then completes. Caller data is retained and no
  late writes/remaining volumes/boxdirs follow cleanup. This is a synthetic
  completion gate, not a fabricated OS stop error or production readiness claim.
- Existing042 stats-wait native assertions remain in the shared fixture's
  separate entry; they are historical native acceptance, not newly run043 tests.
- AF-final1 passes5 actual rows:3 Python scenarios (OCI lifecycle, Host RO attacks/
  restart, engine process flow) and2 Rust native tests (prewarm/directory identity
  and generation fencing), each Rust1/0/0. Host RO survives privileged RW remount,
  new mount/child attacks with RW control; streams/replay/no-late-write remain.
- Separate4-daemon/16-initial-authenticated-Ready/4-Guest-command matrix passes.
  All4 canonical pre-existing caller template directories retain dev/inode/data.

Current total3 Rust native tests +3 Python AF scenarios, plus the independent
4daemon/16Ready/4commands. Two variants inside one entry count as one Rust test.
Actual stdout/exit/result/loader evidence is checked, not just PASS/ignored flags.

## Owned cleanup and continuing work

Final audit covers25 owned homes,35 observed VM socket directories,
146 exact socket paths,2 engines,797 immutable image cache files
(bytes/UID/GID/mode), both roots and the bound verifier short root. No remaining
managed executions/volumes, owned VM/shim/engine/Host processes, mounts/disk
images, FD holders or sockets. Caller data/prior archives remain; only mutable
image catalog changes; audit deletes nothing. Runtime RSS unmeasured. Historical
default-gate user-home nonmodification remains unestablished as035 corrected;
current explicit isolated native acceptance is valid.

Next bounded candidate is legacy provider.start result publication: it currently
awaits handler destination before storing the returned actual handler, unlike
the selected original-owned path's pending_handler retention. Trace/reproduce/
reuse canonical ownership and close; do not claim this fix closes provider-start
internal cancellation, pool boot/allocation/template/unpublished/legacy owner
loss or capacity/publication. Full W4/default OCI Host Service cutover/live
continuity/GA, original15s boot cause, six disk/prior SDK gates, historical RAM
loss, fresh-host/other-platform/conformance/security/soak remain open.

## Explicitly not claimed

No green whole gate/full W4/GA/default OCI cutover/live continuation, native Linux
snapshot/fresh-host/other platforms/actual OS stop or unmount fault/hostile same-
UID namespace qualification/runtime memory measurement/platform promotion.
Windows ARM64 dependencies unsupported, no native run. No KRun business coupling,
real ~/.kbox operation, native submodule edit or publication.

User permits scoped ordinary commit/merge/push; old handoff no-commit superseded.
Baseline is already integrated locally. Git remote synchronization still needs
UID501 lookup/GitHub DNS recovery per retained039 prerequisite proof; no unchanged
failed retry, force operation, remote equality/merge or push success claimed.
Scoped authorized commit and verified incremental bundle follow source/ref/diff
review, actual refs/SHA in ignored artifact git-closeout.json. Continue next local
ownership gap immediately; continuous goal remains active.

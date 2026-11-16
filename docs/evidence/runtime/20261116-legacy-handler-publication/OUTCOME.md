# Legacy returned-handler publication

Bounded044 is accepted on this existing macOS26.6.2 arm64 host. A returned legacy
provider handler is retained in the existing pending slot before waiting for
installation. Cancellation cannot discard that actual handler while its caller
retains the VmManager. Default finalizers adopt pending ownership before cleanup;
conflicts retain both actual owners and working data. Full W4/default OCI Host
Service cutover/live continuity/enterprise GA remain open.

## Actual failure, callers and implementation

publication-red1 actually exits101 with0pass/1fail/0ignored: a real provider-return
boundary is polled with the handler destination locked, then cancelled. The old
manager has pendingNone rather than the returned actual PID14884. A behavior-
preserving extraction of the original legacy block lets the test call the actual
production helper. Its bounded owned OS child is stopped by fixture Drop on the
original failure; this is real child lifetime evidence, not a native VM claim.
Original assertions/logs remain; publication-green1 now executes1/0.

Ordinary boot and rootfs-maintenance start share the same provider acquisition
helper. It refuses repeat acquisition while a handler is pending, stores the
actual returned handler synchronously, then awaits installation. The existing
pending_handler is reused; no new lifecycle/owner collection/config/dependency/
workspace/drop killer. Destination conflict returns typed StateError while both
actual handlers remain retained. Default destroy and boot-failure cleanup adopt
pending handlers before state/cache/resource deletion. A stale completed-cleanup
observation is invalidated by actual pending ownership; stop-refusal reinsertion
also stores pending before waiting. Existing selected original-owned scope/
acquisition semantics remain unchanged, as do ExecutionManager authority,
persisted routes/generation fencing, guest/storage contracts and default backend.

The extra controls cover default/failed-boot cleanup despite a stale stopped
cache, and conflict retaining both live owned children/PIDs/data until explicit
fixture resolution and actual cleanup. owned-focused1 actually9/0/1ignored;
VM-focused1:246/0/3ignored (overlapping suites, not additive unique test counts).
CLI949/0 and Runtime/CLI/contracts all-target Clippy pass. Every check has fresh
owned K_HOME/short TMPDIR/stdout/stderr/exit. scripts/verify-commit.sh passes
isolation4/support/workspace-all-targets/fmt, then exits101 at Runtime2344pass/
6fail/58ignored, retaining the same six external RAM/APFS/disk-service failures.
No green whole gate, ignored failure or reduced assertion. Unchanged038 full
contracts106 top-level(+12 child receipts)/installer8 remain history, not new044
passes. Commands/counts are in checks/test-counts-final.json, including:

```sh
cargo test --locked --offline -p kbox-runtime --lib returned_legacy_child_survives_install_waiter_cancellation
cargo test --locked --offline -p kbox-runtime --lib vm::owned_vm_lifecycle_tests
cargo test --locked --offline -p kbox-runtime --lib vm::
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Frozen archive and installed native effects

Artifact root `artifacts/legacy-handler-publication-dd7d5776d97d4be9b34355c9d1fc4073`;
private root `/private/tmp/kbox-handler-dd7d5776d97d`.
Frozen compiled/package source `27b4f58eba9b28592fd55c41b3b2c1d200bdc1efa365ea64cb90634dc544a8ea`,1730 inputs,
based on main38824146 plus044 changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.
Actual compiler-artifact frozen Runtime client `69e8b3bc6e8b29389aca37ec0dd197388085b671a67a920f472c5ef8e99ebf61`.

Final archive `final-dist-handlerpub-1/kbox-3.4.3-macos-arm64.tar.gz` under the artifact root;
SHA256 `d24cb38d756dd3990cabccf192514813956e22b1786db166d2e80ce9bc750dfb`;
23428487 compressed bytes /44208128 installed bytes.
Fresh prefix `installed-handlerpub-final-1`; all11 manifest/source/strict ad-hoc
signature/architecture/native dependency/embedded OCI Agent checks pass. CLI/shim
are Mach-O arm64, guest-init Linux aarch64; actual loaded paths/hashes match the
installed companions. No development/user installation substitution, native
submodule edit/notarization/publication. Parent libkrun3c2889c7/libkrunfwfc3ea068
remain clean/unchanged. Separate OCI archived5f59995… retains original source
d2b442f1… and owning-repository native patch provenance, not relabeled044. Prior
archives and original failure remain.

- New native entry actually1pass/0fail/0ignored. It initially boots a real VM
  through installed native companions, then a controlled one-shot provider
  returns a proxy for that SAME actual handler. With destination locked, real
  legacy installation is polled/cancelled. Pending owner, actual PID, allocation
  JSON/dev/inode and continuing Guest writes remain; repeat acquisition refuses
  without another provider call. Default product cleanup adopts and physically
  stops that actual handler (observed stopCalls>0), removes created allocation/
  working directory, retains caller data and proves no late writes.
- The fixture keeps rescue access to this exact actual handler. Any assertion
  failure would physically close it before host cleanup; current PASS requires
  rescueUnused and product finalization. This is controlled returned-handler
  publication, not a second fresh native provider launch or synthetic OS failure.
  Initial normal native boot and AF/pool exercise the real start adapter too.
- AF-final1 passes5 actual rows:3 Python scenarios (OCI lifecycle, Host RO attacks/
  restart, engine process flow), plus2 Rust native tests (prewarm and directory/
  identity/generation fencing), each Rust exactly1/0/0. Host RO survives privileged
  remount/new mount/child attempts with RW control; streams/replay/fencing remain.
- Separate4-daemon matrix passes16 initial authenticated Ready VMs and4 actual
  Guest commands; all4 canonical pre-existing caller template dev/inode/data stay.

Current total3 Rust native tests +3 Python AF scenarios, plus the independent
4daemon/16Ready/4commands matrix. Actual stdout/exit/result/loader records are
checked; no compile/no-run/ignored/default flag is treated as native execution.

## Scoped cleanup, Git and continuing work

Final audit covers26 owned homes,35 observed VM socket directories,
146 exact socket paths,2 engines,811 immutable image cache files
(bytes/UID/GID/mode), both owned roots and bound verifier short root. No remaining
managed executions/volumes, owned VM/shim/engine/Host processes, mounts/disk images,
FD holders or sockets. Caller data/prior archives remain; only mutable image
catalog changes; audit deletes nothing. Runtime RSS unmeasured. Historical
user-home nonmodification for old default gates remains unestablished as035
corrected; current explicit isolated native qualification is valid.

Fresh read-only Git prerequisites at08:45:09Z still show UID501 getpwuid KeyError
and github.com DNS errno8; [actual receipt](checks/git-prerequisites-closeout1.json).
Baseline ancestry already integrated in local main. User authorizes scoped
ordinary commit/merge/push, superseding old handoff no-commit. Task-only local
commit/verified incremental bundle follow source/ref/diff review; post-commit
refs/SHA in ignored artifact git-closeout.json. No unchanged failed push retry,
force operation, fresh remote equality/merge or publication claim.

Next045: register actual managers inside boot_or_restore before first async boot,
return existing CompletedBoot to both published foreground/batch callers, preserve
restore-refusal ownership before cold fallback. Current fix only protects an
already returned handler while its VmManager itself is retained. Internal boot/
allocation/template/unpublished/legacy manager drop, provider-internal cancel,
from_pid reattachment and capacity/publication remain open. Full W4/default OCI
Host Service cutover/live continuity/GA, original15s boot cause, six disk/prior
SDK gates, historical RAM loss, fresh-host/other-platform/conformance/security/
soak stay open. Continue the next local gap; continuous goal remains active.

## Explicitly not claimed

No whole green gate/full W4/GA/default OCI cutover/live continuation, native Linux
snapshot/fresh-host/other platforms/actual OS stop or unmount fault/hostile same-
UID namespace qualification/runtime memory measurement/platform promotion.
Windows ARM64 current dependencies unsupported, no native run. No KRun business
coupling, real ~/.kbox operation or native submodule content edit.

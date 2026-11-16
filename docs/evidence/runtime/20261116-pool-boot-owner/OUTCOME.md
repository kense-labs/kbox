# In-progress published pool boot ownership

Bounded045 is accepted on this existing macOS26.6.2 arm64 host. Actual cold and
restore managers register in the existing owner collection before cancellable
boot waits and return existing CompletedBoot guards to both consumers. A cancelled
published-pool boot keeps its actual manager and closes existing admission.
Full W4/default OCI Host Service cutover/live continuity/enterprise GA stay open.

## Actual failure and shared implementation

boot-owner-red1 actually exits101 with0pass/1fail/0ignored. A behavior-preserving
cold-path extraction is polled with a real VmManager state lock held, then dropped.
The original actual synthetic handler drops1 rather than0, without stop. The
fixture uses actual VmManager/working data with a synthetic stop handler; it does
not orphan a native VM. Original data/counter/refusal/retry assertions remain;
boot-owner-green1 executes1/0 after only necessary helper-signature adaptation.

Cold creation registers before template claim/boot/auth/bootstrap waits. Restore
creation registers before its boot/auth waits. The existing guard/actual manager
collection is reused, with no copied resource state/new workspace/config/dependency
or global drop killer. Both foreground boot_new_vm and BootBatch consume already
registered CompletedBoot values without second registration. The heap-indirection
assertion remains. Exact exclusive transfer, metrics/events, identity/routes/
generation, template ownership, selected/all cleanup and default backend remain.

Restore close uses the existing actual-owner finalizer; a typed stop refusal
propagates before any cold fallback. Cold errors retain unclosed actual managers
and close admission; proven prelaunch or observed fully completed cleanup can
finalize/remove the exact owner while preserving the original error/admission.
cleanup_is_complete exposes the existing successful physical/host-cleanup
observation; BoxState::Stopped alone is insufficient. The new prelaunch malformed
snapshot control passes: original typed ConfigError, no retained owner, pool
admission remains open. Selected unsupported Mac snapshot behavior stays explicit;
this phase does not qualify Linux native snapshot execution.

Pool-focused2 actually78/0/8ignored, CLI949/0 and Runtime/CLI/contracts all-target
Clippy pass. pool-focused1 is a retained compile failure: unwrap_err required
Debug for actual VmManager/ForegroundHandoff; no test executed. It is replaced
with matches(Err(error) if is_cancelled), preserving the same cancellation
assertion without new Debug implementations or a weaker test. Every command uses
fresh task-owned K_HOME/short TMPDIR with stdout/stderr/exit. The actual default
scripts/verify-commit.sh passes isolation4/support/workspace-all-targets/fmt, then
exits101 at Runtime2346pass/6fail/59ignored, retaining the same six external RAM/
APFS/disk-service failures. No green whole gate or skipped failing assertion.
Unchanged038 contracts106 top-level(+12 child receipts)/installer8 and044 VM246
are history, not new045 executions. Actual commands include:

```sh
cargo test --locked --offline -p kbox-runtime --lib cancelled_in_progress_cold_boot_keeps_the_actual_pool_owner
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Frozen source/archive and installed native effects

Artifact root `artifacts/pool-boot-owner-da5ff89af2324e6eb09e07a6b6b515c4`;
private root `/private/tmp/kbox-pool-boot-da5ff89af232`.
Frozen compiled/package source `8aec11912bdc6195f6b57df29fc17aa2b51487abd737b2575e1353d4a9186571`,1731 inputs,
based on mainb42c2869 plus045 changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.
Actual compiler-artifact frozen Runtime client `1582eb4944e0ea241c08aa8936ceda8b37f8d75437971da9edaf50b13b9f4f78`.

Final archive `final-dist-poolboot-1/kbox-3.4.3-macos-arm64.tar.gz` under the artifact root;
SHA256 `d4a99066d3ff742ac8523d98cb8dbd7369e54e96841ac0621c5609a77ece593a`;
23428693 compressed bytes /44208128 installed bytes.
Fresh prefix `installed-poolboot-final-1`; all11 manifest/source/strict ad-hoc
signature/architecture/native dependency/embedded OCI Agent checks pass. CLI/shim
Mach-O arm64, guest-init Linux aarch64, actual loaded paths/hashes bind installed
companions. No development/user installation substitution/native submodule edit/
notarization/publication. Parent libkrun3c2889c7/libkrunfwfc3ea068 are unchanged/
clean. Separate OCI archived5f59995… retains original sourced2b442f1… and owning-
repository native patch provenance, not relabeled045. Previous archives remain.

- New native entry actually1pass/0fail/0ignored. A test-only gate waits BEFORE
  delegating actual VmController.start. The caller locks destination after boot
  precheck, then permits real fresh VM launch. There is no new fixture await
  holding a returned actual handler;044 synchronously stores it pending. Only
  after actual pool registration count1 does the test abort the boot worker.
- The exact manager/handler-slot identity, actual live shim PID/config, allocation
  JSON/dev/inode and continuing Guest writes survive. Existing watch closes,
  idle count stays0, BoxState staysCreated; cancelled boot never invents Ready.
  Product drain adopts pending ownership, physically stops this actual VM,
  removes created allocation/box directory and proves caller retention/no late
  writes. A failed pre-cancel assertion releases its gate and awaits the still-
  owning worker instead of deliberately orphaning a baseline VM. No synthetic
  OS stop fault, mock native VM or second lifecycle owner is used.
- AF-final1 passes5 actual rows:3 Python scenarios (OCI lifecycle, Host RO attacks/
  restart, engine process flow) plus2 Rust native tests (prewarm and directory/
  identity/generation fencing), each Rust1/0/0. Host RO survives privileged RW
  remount/new mount/child attempts with RW control; replay/streams/fencing remain.
- Separate4-daemon matrix passes16 initial authenticated Ready VMs and4 actual
  Guest commands; all4 pre-existing caller template dev/inode/data identities stay.

Current total3 Rust native tests +3 Python AF scenarios, plus the independent
4daemon/16Ready/4commands. stdout/exit/result/loader files are checked; compile/
no-run/ignored counts are not native execution. Old fault cases are retained
history, not newly run045 passes.

## Scoped cleanup and continuing work

Audit covers28 owned homes,35 observed VM socket directories,
146 exact socket paths,2 engines,825 immutable image cache files
(bytes/UID/GID/mode), both roots and bound verifier short root. No remaining
managed executions/volumes, owned VM/shim/engine/Host processes, mounts/disk images,
FD holders or sockets. Caller data and prior archives remain; only mutable image
catalog changes; audit deletes nothing. Runtime RSS unmeasured. Historical old
default-gate user-home nonmodification remains unestablished as035 corrected;
current isolated native qualification is valid.

Ownership here relies on a surviving published pool collection. Unpublished
WarmPool::start failure/cancel can still drop that collection and needs caller/
startup ownership work. Shutdown admission after queued template preparation,
provider-internal cancellation, legacy/from_pid/unpublished/template allocation
and capacity/publication remain local work. Full W4/default OCI Host Service
cutover/live continuity/GA, original15s boot cause, six disk/prior SDK gates,
historical RAM loss, fresh-host/other-platform/conformance/security/soak stay open.

User permits scoped ordinary commit/merge/push. Baseline is already local in main;
latest04408:45:09Z proof still shows UID501 missing/GitHub DNS errno8. No unchanged
push retry, force operation, fresh remote equality/merge or publication claim.
Scoped authorized commit/verified incremental bundle follow source/ref/diff
review; actual refs/SHA in ignored artifact git-closeout.json. Continue the next
local admission/startup ownership gap; continuous goal remains active.

## Explicitly not claimed

No green whole gate/full W4/GA/default OCI cutover/live continuation, native Linux
snapshot/fresh-host/other platforms/actual OS stop or unmount fault/hostile same-
UID namespace qualification/runtime memory measurement/platform promotion.
Windows ARM64 current dependencies unsupported, no native run. No KRun business
coupling, real ~/.kbox operation or native submodule content edit.

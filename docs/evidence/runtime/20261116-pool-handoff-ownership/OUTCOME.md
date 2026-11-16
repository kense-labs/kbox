# Foreground pool handoff ownership

Bounded039 is accepted on this existing macOS26.6.2 arm64 host. Published
foreground pool acquire/release now retains actual VmManagers through health,
statistics, cancellation and refused finalization. Allocation/boot producers,
unpublished pools and legacy callers remain continuing work. This does not close
full W4/default OCI Host Service cutover/live continuity or enterprise GA.

## Actual failures and implementation

`release-owner-red1` runs1 test,0pass/1fail: public release propagates stop refusal
but drops the actual handler (drop count1 instead of0). `release-owner-green1`
passes1/0 after retaining the exact owner. No assertion is removed or skipped.

The existing retained-owner collection holds actual Arc/Mutex<VmManager> handles;
it has no copied execution lifecycle. Registration happens before async health/
stat waits. Idle stats are locked before pop, preserving member/count if that
wait is cancelled. Exclusive exact-owner transfer happens synchronously after
all awaits into the caller or idle list. A competing finalizer prevents transfer.
Successful operations update counters; a refused close returns the original typed
error, retains owner/data and closes existing admission. Converted foreground
paths no longer use ambient-home orphan reap. Boot result/stat tails are covered;
internal boot/allocation lifetimes are explicitly outside this bounded slice.

Intermediate failures remain: `foreground-focused1`4pass/1fail and
`cancel-foreground2`0/1 expose maintenance eviction sweeping the entire retained
collection, including a foreground health wait. It consumes the injected refusal
before explicit drain, giving unexpected drainOk/stopcalls2. PoolCloseScope now
selects only actual eviction/rollback/invalidation/late-fill inputs; explicit drain
still closes all retained handles. It selects cleanup work, not execution state.

`cancelled-admission-red3`0/1 proves retention alone still permits more launches
beside a cancelled in-flight VM. ForegroundHandoff is a scope guard with a weak
reference to the actual owner; if it remains registered on cancellation/error,
it signals the existing shutdown watch. Successful transfer/finalization leaves
no matching registration and keeps admission open. This does not add a global
VM Drop killer or interfere with intentional detached ownership. Actual retained
VMs are reported live until finalization; cancellation alone is not a stopped
claim. The original cancellation/admission/refusal assertions remain.

The final focused suite includes7 new hermetic cases: refused release, refused
idle acquire without replacement boot, cancelled pre-pop stats wait, cancelled
acquire/release health, competing transfer refusal, same-manager exclusive
transfer and selected-maintenance isolation. All previous template/finalizer tests
remain. The native037 fixture is reused with a separate foreground entry; both
entries are executed below because their shared helper changed.

## Checks and frozen assets

Every Cargo/test command uses a fresh task-owned K_HOME and short temporary root
through the recorded `run-check.py`. Actual commands/exits/counts are retained
in `checks`; no real user-home scan or repair occurs. Earlier default-gate home
nonmodification remains unestablished as corrected by035.

- Pool-focused1:64pass/0fail/3ignored. The ignored native cases are not passes.
- CLI-full1:949pass/0fail/0ignored. Runtime/CLI/contracts all-target Clippy passes.
- `scripts/verify-commit.sh`:isolation4/support/workspace-all-targets/fmt pass,
  then Runtime2329pass/6fail/53ignored, exit101. The same six external RAM/APFS/
  disk-service failures remain. No green default gate or skipped assertion.
- Full contract106 top-level(+12 successful filtered child receipts) and
  installer8 are unchanged038 evidence, not new039 runs. New archive install/
  manifest/signature checks below execute against039 assets.

Key commands:

```sh
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

Artifact root `artifacts/pool-handoff-ownership-6b342296abe142e18c1b80b5da4aba9f`;
owned private root `/private/tmp/kbox-pool-xfer-6b342296abe1`.
Final compiled/package source
`87247f938bd5e8fe1eef03ad154adc72a6e8942d368708d69b518c55c9520355`,1726 inputs,
based on main5b8d68f7 plus recorded039 source changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.

Final archive:
`artifacts/pool-handoff-ownership-6b342296abe142e18c1b80b5da4aba9f/final-dist-poolxfer-1/kbox-3.4.3-macos-arm64.tar.gz`

SHA256 `1223c6b17c875d6b725f166768a3d8227c1f03f563abdd5dc2d73a750926559e`;
23424953 compressed bytes /44208128 installed bytes. Fresh installed prefix is
`installed-poolxfer-final-1` under that artifact root. All11 manifest files/source
provenance, CLI/shim ARM64 strict ad-hoc signatures, Linux aarch64 guest-init,
native library identities and embedded OCI Agent match. No notarization or
publication claim. Prior Box782cd1ae and all older archives remain retained.

Separate unchanged OCI archive
`d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79` retains original
source `d2b442f175f89b34d6ba9b63e6d2df4a478dc2a6c3493f8ca647a761e9012871` and owning-
repository native patch provenance; it is not relabeled039. Parent gitlinks
libkrun3c2889c7/libkrunfwfc3ea068 are unchanged/clean. Frozen current Runtime
client SHA256250f42e3feececcade438576e194f636f27579151e775a745635c3f0b3df05d7.

## Final installed native and failure records

Native drivers clear development loader overrides, copy companions from the
fresh install, and capture actual loaded shim/dylib paths and hashes. No ignored,
zero-test, cross-compiled or old-asset result counts as a current native pass.

- New foreground Rust native entry:exactly1/0/0. Actual healthy public release/
  acquire returns the same execution and correct counts. Deliberately stalled
  health is cancelled; the actual VM/PID, writes, anonymous allocation identity
  and template input remain. Public release receives a synthetic one-shot stop
  refusal, retaining the exact owner. Actual drain retry stops the VM, removes
  its created volume/template/sockets and leaves no late writes or caller loss.
- Original published-shutdown Rust native entry:exactly1/0/0, preserving its
  guest-control cancellation/refusal/live-resource/actual-finalization assertions.
  These refusals/stalls are synthetic; VM, writes, identities, finalization and
  cleanup are real. No actual OS stop-failure or Linux snapshot claim.
- AF-final1:all5 rows pass,3 Python scenarios (OCI lifecycle, Host RO attacks/
  restart, engine process flow) plus2 Rust native tests (prewarm and directory/
  generation fencing), each Rust exactly1/0/0. Host RO checks14 actual denials
  on first/restart probes after privileged RW remount/new mount/child attempts,
  with RW control; identity, stream/replay/no-late-write and stale generation
  assertions remain. Total current phase:4 Rust native tests +3 Python AF scenarios.
- Separate4-daemon matrix:16 initial authenticated Ready VMs and4 Guest commands
  pass. All4 pre-existing canonical caller template directory dev/inode/bytes
  remain. Mac unsupported-snapshot control retains the existing explicit cold
  optimization in the same backend, not native snapshot support.

One evidence wrapper fails after the foreground native test succeeds: outer and
inner driver used the same receipt/log stem. The wrapper refuses to overwrite the
native receipt (exit1); actual native exit0/stdout1/0/0/proof/cleanup remain intact.
`native-wrapper-collision1.json` records this helper failure, and independent
`foreground-native-receipt-check1.json` verifies the original native data. Later
wrappers use distinct check names and pass. The VM test is not pointlessly rerun,
and the wrapper failure is not presented as a pass. Raw unit failures and all
native/packaging commands remain retained.

## Cleanup, Git and remaining scope

`final-cleanup.json`/home/FD receipts audit32 private homes, both owned roots,
the bound verifier short root,35 observed VM socket directories,146 exact socket
paths,2 engines and741 pre-existing immutable cache files including UID/GID/mode.
Managed executions/volumes, owned VM/shim/engine/Host processes, mounts/disk images,
FD holders and sockets are empty. Only the mutable image catalog changes. Caller
sources, old archives and evidence remain. Audit deletes nothing; RSS unmeasured.

Internal allocation/boot_or_restore/template/batch cancellation, unpublished pool
lifetime and legacy CLI boot/compose ownership still require actual producer/
consumer work. The old helper still exists only in those unconverted boot paths;
this phase does not claim their orphan-reap/lifetime issues solved. Full W4,
default OCI Host Service cutover/live continuity/GA remain incomplete. Original
15s boot cause, six default disk/prior SDK gates, earlier unexplained RAM loss,
fresh-host/other-platform/OCI conformance/security review/soak remain open.
Windows ARM64 dependencies unsupported/no native run; platform levels and the
default libkrun/BoxVm route stay unchanged. Arbitrary hostile same-UID namespaces,
actual OS stop/unmount failures and native Linux snapshots are not qualified.

Local baseline ancestry in main is proved; fresh remote equality/push still needs
UID501 lookup and GitHub DNS recovery ([proof](../20261116-pool-shutdown-ownership/checks/git-prerequisites1.json)).
After source/diff/task-only commit closeout, create/verify a local incremental
Git bundle for transport, recording its exact commit/prerequisite/SHA. A bundle
is not a remote push or publication. Then continue the existing boot-producer
and legacy owner gaps; the continuous goal stays active.

### Actual Git transport checkpoint

Accepted code/evidence is committed on main85eebd65d50198c3ee8f547c836eb79a6d40cd81,
tree3e5004b4dcc7353d74c9c2472b04752e9040fc02. Its compiled inputs exactly match
the final archive. The local incremental `accepted-main-incremental.bundle` in
the artifact root is1919973 bytes, SHA256
4427bf7e63ed5b8ccac5a1dcb7bf62309ddd0f7d1b9a6a0549b9143f90650fd8.
Git bundle verify succeeds; it includes13 unpublished commits relative to cached
origin/main9d279a1cd0a11d1fb035e5cd845a15f5d70a2967. That base is a prerequisite,
not a fresh remote observation. [Fresh prerequisite check](checks/git-prerequisites-closeout1.json)
still fails getpwuid501 and GitHub DNSerrno8. No push is attempted against those
unchanged failures. This documentation-only checkpoint is committed separately;
`accepted-main-closeout.bundle` and its actual final-ref receipt are then created
in the artifact root. The earlier bundle is retained. Neither bundle means push,
external merge or publication. Continue the existing local producer gap.

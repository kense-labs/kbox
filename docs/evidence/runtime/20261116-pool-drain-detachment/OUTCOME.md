# Drain detachment ownership

Bounded042 is accepted on this existing macOS26.6.2 arm64 host. Both public drain
variants and template invalidation now wait for statistics before detaching idle
VMs, and register actual selected managers before any subsequent asynchronous
wait. Cancelled stats acquisition leaves actual idle membership/count/identity
intact. Full W4/default OCI Host Service cutover/live continuity/GA remain open.

## Actual failure and smallest shared fix

`detachment-red1` actually exits101 with0pass/3fail/0ignored. Each real operation
is polled while the actual stats mutex is held, then the future is lexically
dropped. All three old implementations drop the exact handler (actual drops1,
expected0), without physical stop. Fixtures use real VmManager/filesystem data
with synthetic stop handlers; they do not launch a native VM or fake OS failure.
Original failing assertions/logs remain. These tests cover drain, drain_idle and
template invalidation independently, rather than assuming one caller covers all.

The minimal fix acquires idle then stats before detaching and updates the count
in that critical section. There is no await between detachment and existing
`destroy_vms` registration. Typed refusal, all/selected close scope, retry,
ExecutionManager authority, routes/generation, template cleanup and guest/storage
contracts remain unchanged. Eviction already registers before its later stats
wait and is unchanged. No new state machine/config/dependency/workspace/native
source edit or drop killer. Maintenance joining is a separate remaining gap.

Original three regressions now pass3/0, including unchanged data/owner/counter
assertions and exact typed stop-refusal followed by successful retry. Final pool
74/0/6ignored, CLI949/0, Runtime/CLI/contracts all-target Clippy pass. Every test
uses a fresh task-owned K_HOME/short TMPDIR with stdout/stderr/exit recorded.
`scripts/verify-commit.sh` passes isolation4/support/workspace-all-targets/fmt,
then exits101 at actual Runtime2339pass/6fail/56ignored: the same six external
RAM/APFS/disk-service failures. No green default gate or ignored failing assertion.
Unchanged contracts106 top-level(+12 child receipts)/installer8 are038 history,
not new042 tests. Commands/counts are in `checks/test-counts-final.json`, including:

```sh
cargo test --locked --offline -p kbox-runtime --lib cancelled_idle_detachment
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Frozen source, archive and actual installed native results

Artifact root `artifacts/pool-drain-detachment-c7d4914f49e840e681a3793ce5443d7a`;
private root `/private/tmp/kbox-pool-det-c7d4914f49e8`.
Final compiled/package source `f522f929ddce4c89c0101f457ac44a1debc8b289e3f7c9399cb6d00d71c4f772`,1729 inputs,
based on main65a81966 plus042 changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.
Frozen actual compiler-artifact Runtime client `049c5ed4578ec7b503b5efc6ab1b5e8f17029a086250e69d8cbf7aab9faf2ae7`.

Final archive `final-dist-pooldetach-1/kbox-3.4.3-macos-arm64.tar.gz` under the artifact root;
SHA256 `f5f41897d02b0b2bf467d65330afdcc8477655c069e32bc31a99980d623cb50e`;
23427053 compressed bytes /44208128 installed bytes.
Fresh prefix `installed-pooldetach-final-1`. All11 manifest/source/architecture/
strict ad-hoc signature/native dependency/embedded OCI Agent checks pass. Actual
loader paths/hashes bind companion binaries to this isolated install. CLI/shim
are Mach-O arm64, guest-init Linux aarch64. Parent native gitlinks3c2889c7/fc3ea068
remain clean and unchanged; no parent submodule edit/notarization/publication.
Separate unchanged OCI archived5f59995… retains sourced2b442f1… and owning-repo
native patch provenance, not relabeled042. Previous archives are retained.

- New native entry actually executes1pass/0fail/0ignored, covering both public
  drain variants with separate real VM processes/allocations. After cancelling
  the actual stats wait, the pool still contains the exact manager/handler, same
  actual PID, same allocation JSON/dev/inode and coherent idle count. Admission
  is closed; Guest writes continue. Retrying the actual corresponding drain
  physically stops it and removes its created allocation/box directory/sockets.
  Both cases keep caller data and prove no late writes after owned cleanup.
- AF-final2 passes all5 rows:3 Python scenarios (OCI lifecycle, Host RO attacks/
  restart, engine process flow) plus2 actual Rust native tests (prewarm and
  directory/identity/generation fencing), each Rust exactly1/0/0. Host-enforced
  RO remains after privileged remount/new mount/child attempts, with RW control.
- Separate4-daemon matrix passes16 initial authenticated Ready VMs and4 Guest
  commands. All4 canonical pre-existing caller template identities/data remain.

Current total3 Rust native tests +3 Python AF scenarios, plus the independent
4-daemon/16Ready/4-command matrix. Counts come from stdout/exit/result/loader
files; two drain variants within one entry are not counted as two Rust tests.
Invalidation is hermetic only, not a Linux native snapshot qualification. Old
fault controls are retained history, not newly run042 passes.

## Failures, scoped cleanup and continuing work

AF-check-final1 exits1 before launching any native subprocess: the external
qualification runner expected its evidence directory pre-created. Original log/
stderr/empty root remain; `af-final1-setup-failure.json` records actual0 native
cases/resources. Correct preparation uses freshAF-final2 root/checks, unchanged
source/client/archive/assertions, and all5 actual scenarios pass. There is no
relabeling of failed final1 or unnecessary source rebuild for fixture setup.

Final audit covers26 owned homes,35 observed VM socket directories,
146 exact socket paths,2 engines,783 immutable cache files
(bytes/UID/GID/mode), both roots and bound verifier short root. No remaining
managed executions/volumes, owned VM/shim/engine/Host processes, mounts/disk images,
FD holders or sockets. Caller sources/data and prior archives remain. Only mutable
image catalog changes; audit deletes nothing. Runtime RSS is unmeasured.
Historical default-gate user-home nonmodification remains unestablished as035
corrected; explicit current isolated native evidence remains valid.

Next043: cancelled maintenance join currently takes/drops the actual JoinHandle,
so retry can return before the worker physically finishes. Reproduce/fix both
public drain callers before proceeding internal boot/allocation/template,
unpublished pool, legacy lifetime and capacity/publication work. Full W4/default
OCI Host Service cutover/live continuity/GA, original15s boot cause, six disk/prior
SDK gates, historical RAM loss, fresh-host/other-platform/conformance/security/
soak remain open. Git baseline is already in local main; remote sync remains
blocked by UID501 lookup/GitHub DNS, per retained039 prerequisite proof. User
allows scoped ordinary local commits/merge/push; no unchanged failed push retry,
force operation, remote equality/merge or publication claim. Task-only commit
and verified incremental bundle follow review; actual refs/SHA in ignored
artifact `git-closeout.json`. Continue043; continuous goal remains active.

## Explicitly not claimed

No green whole gate/full W4/GA/default OCI cutover/live continuation, fresh-host/
other-platform/Linux snapshot native, actual OS stop/unmount failure, hostile
same-UID namespace qualification, runtime memory measurement or platform level
promotion. Windows ARM64 dependencies remain unsupported/no native run. No KRun
business coupling or real ~/.kbox operation.

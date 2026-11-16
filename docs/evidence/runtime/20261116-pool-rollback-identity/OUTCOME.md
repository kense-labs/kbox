# Exact pool rollback identity

Bounded041 is accepted on the existing macOS26.6.2 arm64 host. Rollback now
selects exact current BoxIds under one idle lock and registers selected actual
owners before any subsequent asynchronous wait. Only selected idle resources
are finalized; already acquired or unrelated resources retain their identity.
Full W4/default OCI Host Service cutover/live continuity/enterprise GA stay open.

## Failure, implementation and tests

`rollback-identity-red1` actually exits101 with0pass/1fail/0ignored: FIFO idle-lock
scheduling changes membership between the original index snapshot and removal.
The old implementation closes unrelated C and deletes its real working-data
canary (ENOENT), rather than target B. These hermetic fixtures use actual
VmManager handles and filesystem effects with synthetic stop handlers; this is
not a native OS stop-failure claim. Failure cleanup closes only fixture owners
before resuming the original assertion failure. Original log/assertions remain.

The shared implementation takes idle and stats locks before selection, compares
the actual BoxId while removing within that same critical section, updates the
actual count there and performs no asynchronous wait before existing selected
owner registration. Existing typed close/refusal/retry and selected-vs-all scope
remain unchanged. There is no new lifecycle state/config/dependency/workspace,
route/generation change or native submodule content edit.

The original regression now passes1/0. Two added controls cover cancellation
before stats admission and duplicate/absent/already removed IDs. The broad
rollback filter executes21pass/0fail/1ignored, including unrelated existing
TEE/resource cases; it is not21 new rollback tests. Pool-focused1:71/0/5ignored;
CLI-full1:949/0; Runtime/CLI/contracts all-target Clippy passes. Every command has
a fresh owned K_HOME and short TMPDIR, recorded with stdout/stderr/exit.

`scripts/verify-commit.sh` passes isolation4/support/workspace-all-targets/fmt,
then exits101 at actual Runtime2336pass/6fail/55ignored. The same six external
RAM/APFS/disk-service failures remain; no green full gate or skipped failure
is claimed. Unchanged contracts106 top-level(+12 child receipts) and installer8
are038 history, not current041 tests. Actual commands/counts are in
`checks/test-counts-final.json` and individual receipts, including:

```sh
cargo test --locked --offline -p kbox-runtime --lib rollback
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Frozen final archive and native effects

Artifact root `artifacts/pool-rollback-identity-c67c2a3047e341a6a8e3aee3ee494673`;
private root `/private/tmp/kbox-pool-rb-c67c2a3047e3`.
Compiled/package source
`c0552f3ed494fdad67e7669b25b9640a00b66b15d24894cf9a6a5ad0067a3f6f`,1728 inputs,
based on main15d6fa88 plus041 changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.
Frozen actual Runtime test client:
8ee2845e12fbac759217867dabd4fd71b5414ecadf99d33389d3fc28c8448c4d.

Final archive under that root:
`final-dist-poolrb-1/kbox-3.4.3-macos-arm64.tar.gz`

SHA256 `5588756009f175db37c023f5e6aac0f5efed11f6520a40fecf0629e761c058b5`;
23427222 compressed bytes /44208128 installed bytes. Fresh prefix
`installed-poolrb-final-1`. All11 manifest files, source provenance, CLI/shim
ARM64 strict ad-hoc signatures, Linux aarch64 guest-init, native dependencies
and embedded OCI Agent match. Parent libkrun3c2889c7/libkrunfwfc3ea068 gitlinks
remain unchanged/clean. Native loaded paths/hashes prove installed companions;
no development/user installation substitution. No notarization/publication.
Separate unchanged OCI archived5f59995… keeps original sourced2b442f1… and native
owning-repository patch provenance; it is not relabeled041. Old archives remain.

- New rollback native entry executes exactly1pass/0fail/0ignored. Two real VM
  processes have independent anonymous allocations. Real FIFO membership
  mutation leaves selected B stopped with handler/box/allocation removed; C
  retains the same actual PID, allocation JSON/dev/inode and continues Guest
  writes. Owned final drain stops C, empties volumes/boxdirs and keeps caller
  data/no-late-writes. No synthetic native stop fault or deliberate orphan.
- AF-final1 actually passes5 rows:3 Python scenarios (OCI lifecycle, Host RO
  attacks/restart, engine process flow), plus2 Rust native tests (prewarm and
  directory/identity/generation fencing), each Rust exactly1/0/0. Privileged
  remount/new mount/child RO attempts remain rejected by Host, with RW control.
- Separate4-daemon matrix passes16 initial authenticated Ready VMs and4 Guest
  commands; all4 pre-existing caller template directories retain dev/inode/data.

Current total is3 Rust native tests +3 Python AF scenarios, plus that independent
4-daemon/16Ready/4-command matrix. Unchanged040 fault controls are historical,
not new041 passes. Counts come from actual stdout/exit/result/loader files.

## Cleanup, failures and remaining work

`checks/final-cleanup.json`/inventories/FD receipts audit26 owned homes, both roots,
the bound verifier short root,36 observed VM socket directories,150 exact socket
paths,2 engines and769 immutable cache files (bytes/UID/GID/mode). No remaining
managed execution/volume, owned VM/shim/engine/Host process, mount/disk image,
FD holder or socket. Only mutable image catalog changes. Caller data and prior
archives remain; audit deletes nothing. Runtime RSS is unmeasured.

`final-audit-check1` exits2 because the invocation misspelled the artifact path;
no audit code/native test ran. Original stderr/receipt retained. Correct unique
`final-audit-check2` executes the unchanged audit and passes. The deterministic
wrong-target deletion red is retained separately. Historical default-gate
user-home nonmodification remains unestablished as corrected by035; current
explicit isolated native acceptance remains valid.

Next local042: drain/drain_idle detach idle managers before awaiting stats and
before registering them; cancellation can lose actual owners. Reproduce and fix
that shared boundary, including invalidation/eviction caller review. Internal
incomplete boot/allocation/template, unpublished pool, legacy lifetime and
capacity/publication work remain. Full W4/default OCI Host Service cutover/live
continuity/GA, original15s boot cause, six disk/prior SDK gates, historical RAM
loss, fresh-host/other-platform/conformance/security/soak remain open.

## Explicitly not claimed

No whole-product/GA/green default gate, full OCI cutover/live continuation,
fresh-host/other-platform/Linux snapshot native, actual OS stop/unmount failure,
arbitrary hostile same-UID namespace qualification, runtime memory measurement
or platform support promotion. Windows ARM64 current dependencies unsupported,
with no native run. No KRun business coupling or real ~/.kbox operation.

Local baseline ancestry is already integrated in main. Latest user authorization
allows scoped ordinary commits/merge/push; old handoff no-commit is superseded.
Remote equality/push still needs UID501 lookup/GitHub DNS recovery
([latest prerequisite proof](../20261116-pool-handoff-ownership/checks/git-prerequisites-closeout1.json)).
Task-only local commit and verified incremental bundle follow source/diff review;
actual post-commit refs/SHA remain in the ignored artifact receipt. No unchanged
push retry, force operation, remote merge or publication is claimed. Continue042
immediately after this bounded acceptance; continuous goal stays active.

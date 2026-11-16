# Actual CLI pool startup ownership

The bounded047 owner fix is verified on this existing macOS26.6.2 arm64 host.
Final archive acceptance is INCOMPLETE: the first primary/directory startup
timeouts and failed automatic cleanup remain unresolved. Diagnostic reruns pass
without source, timeout or assertion changes; that is not a root-cause fix.
The user now requests convergence/commit/merge/push rather than another feature.
CLI registers its
actual pool owner before cancellable first-ready initialization; cancellation/
retry/drain keep the same entry. Shared Runtime preparation is synchronous and
initialization borrows an already retained owner. Full W4/default OCI Host Service
cutover/live continuity/enterprise GA remain open.

## Actual red, API and caller

startup-registry-red1 actually0pass/1fail/0ignored, exit101. Actual get_or_create
is polled while the daemon boot limiter is0: first fill is pending, but the old
registry has no owner. No network/VM is launched in this hermetic baseline.
The original registry/same-Arc/retry-Pending/drain assertions remain and pass1/0.

Existing validation/configuration/construction are factored into synchronous
prepare_with_metrics_and_boot_limiter; it starts no VM or maintenance task.
Borrowed initialize_first_ready uses the EXISTING replenish-handle mutex to
serialize initialization/retry/shutdown and starts maintenance once. The actual
initial-fill metric tests inert preparation, one initial fill across repeat
calls and typed refusal after drain. Existing compatibility constructors retain
Self-return and unit-fill semantics; they do not gain arbitrary cancellation/
Drop safety. CLI uses the new owned preparation API, not the old async constructor.

Under the existing key lock, CLI prepares and publishes the exact Arc under the
existing registry map lock BEFORE awaiting initialization. Existing entries also
resume unfinished initialization; closed/unfinalized entries remain available for
truthful cleanup retry. Draining admission is checked under that same map lock
before publication. No new registry/lifecycle state/config/dependency/workspace/
kill loop; ExecutionManager authority/routes/generation/default isolation stay.

Pool-focused1 actually81/0/10ignored. Final CLI-full2:950/0; all-target Runtime/
CLI/contracts Clippy2 passes. Both verify1/verify2 actually exit101 at Runtime
2349pass/6fail/61ignored after isolation4/support/workspace-all-targets/fmt, with
the same six external RAM/APFS/disk-service failures. Runtime inputs are unchanged
between source1 and2 (only the CLI fixture below changes). No green whole gate,
skipped failure or lowered assertion. Every test uses fresh owned K_HOME/short
TMPDIR/stdout/stderr/exit. Unchanged038 contracts106 top-level(+12 child receipts)/
installer8 remain history, not new047 runs.

## Retained CLI failure and fixture ordering correction

CLI-full1 actually949pass/1fail, with the new owner test passing. Existing
shutdown_socket_cleanup_does_not_unlink_a_replacement_object fails reconnecting
the renamed socket (errno61). Its readiness loop could connect immediately after
OS bind, before asynchronous chmod/metadata/ownership-receipt/from_std completion;
the replacement attack was not ordered after the completed-bind boundary that
this SHUTDOWN-cleanup test intends. The fixture now waits for BoundPoolListener::
bind to return, then performs the SAME rename/replacement/moved-path connection/
Stop/error/caller-bytes assertions. No product path, attack or assertion is removed.

Original log, failed compiled CLI client/hash, source copy and candidate1 are
retained. Exact failing internal syscall was not captured; no kernel defect or
hostile-same-UID initial-bind race is claimed fixed. A separate native OS rename
probe succeeds40/40 (with/without a probe connection), so that isolated operation
did not reproduce the original failure. Corrected target passes1/0 and20 further
actual executions across4 concurrent processes, each fresh K_HOME, with original
assertions. Those are20 executions of ONE case, not20 distinct features.
Full CLI950/0 andClippy2 follow. Initial binding under hostile same-UID mutation
remains an explicit unqualified/open boundary, not hidden by this fixture change.

Source1/candidate1 sha88b25d40… are retained and unqualified, not overwritten.
Only crates/box/cli/src/commands/pool/tests/shutdown.rs changes from source1 to2,
recorded in source-candidate1-to2-delta.json. Final source/affected package are
regenerated and installed into a new prefix. Commands/counts in checks include:

```sh
cargo test --locked --offline -p kbox-cli --lib cancelled_initializer_retains_the_same_registered_pool_for_retry_and_drain
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Frozen final2 archive and actual installed native effects

Artifact root `artifacts/pool-startup-owner-307fbf4272cd40e4ae20156ebb6e067e`;
private root `/private/tmp/kbox-pool-start-307fbf4272cd`.
Final source `d38e6a13e9199070fc38d0122eee25cd5867cdb9bfc9638c930e18779c80a9be`,1732 inputs, based on main20c72422
plus047 changes. Rustc1.97.1;
Cargo.lock8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734.
Actual compiler-artifact frozen Runtime client `2ae79a6c5cc56db9ab041e23d71790035600216cb3e5b09a1ab67270fca21b62`.

Final archive `final-dist-poolstart-2/kbox-3.4.3-macos-arm64.tar.gz` under the artifact root;
SHA256 `f6ae774aff6e58df48c663ae154c5074b391f5259dfa13912338edc38689bcd4`;
23427926 compressed bytes /44208128 installed bytes.
Fresh prefix `installed-poolstart-final-2`; all11 manifest/source/architecture/
strict ad-hoc signatures/native dependency/embedded OCI Agent checks pass. CLI/
shim Mach-O arm64, guest-init Linux aarch64, actual loaded paths/hashes bind final
installed companions. No development/user installation substitution. Parent
libkrun3c2889c7/libkrunfwfc3ea068 remain clean/unchanged. Separate unchanged OCI
archived5f59995… keeps sourced2b442f1… and owning-repository native patch provenance,
not relabeled047. No native submodule edit/notarization/publication; old archives
and failed candidate remain.

- New prepared-startup native entry executes exactly1/0/0 (53.56s). A caller-owned
  Arc exists BEFORE first-ready initialize, with publication held at actual stats
  lock. A real authenticated Ready VM is registered while initial fill awaits
  publication. Cancelling the initializer keeps the same actual manager/handler,
  PID, allocation JSON/dev/inode and continuing Guest writes. Watch closes,
  maintenance has not started and reinitialization refuses that closed owner.
  Actual drain physically stops the VM/removes its created allocation/working
  directory; caller data and no-late-write assertions pass. Fixture waits the
  actual publication-lock condition, not just a scheduling assumption. No mock VM,
  synthetic OS stop error or arbitrary registry-Drop safety claim.
- Production CLI composition is checked by its real registry hermetic red/green;
  native Runtime ownership uses the same public prepared/borrowed API. Existing
  compatibility constructor cancellation is explicitly outside this bounded proof.
- AF-final2 actually passes OCI, Host RO/restart and prewarm, but FAILS primary
  startup (120s) and directory (Rust0/1,60s control timeout). Both failed states
  persist Starting without an original PID; directory leaves a live shim.
  The final audit2 correctly fails this gate. Fresh failed-only AF-final3 with
  INFO logs, SAME installed assets/client/timeouts/assertions and no other pool
  matrix running passes primary (68.41s including no-late waits) and directory
  (Rust1/0/0,16.07s). Original rootfs artifacts appear about61/59s after creation;
  diagnostic primary prepares its first rootfs in1.76s. Original internal phase
  timings were not captured, so neither concurrent load nor rootfs code is proven
  causal. Do not claim those startup failures fixed or a reliable cold-start SLA.
- Selected successful records therefore span AF-final2 andAF-final3:3 Python
  scenarios plus2 Rust native tests, each Rust1/0/0. First-run failures stay in
  native-results.json, stdout/stderr and failed-run-quarantine; they are not
  replaced by a green summary. Host RO survives privileged RW remount/new mount/
  child attempts with RW control. This does not close whole-archive acceptance.
- Separate4-daemon matrix passes16 initial authenticated Ready VMs and4 actual
  Guest commands against current CLI, preserving all4 caller template identities.

Current total3 Rust native VM tests +3 Python AF scenarios, plus the independent
4daemon/16Ready/4commands matrix. The20 socket executions/40 OS probes are separate
host/fixture controls. Counts come from stdout/exit/result/loaders, not compile/
no-run/ignored/default flags. No runtime-memory or latency target is claimed.

## Scoped cleanup, integration and remaining work

Audit covers63 owned homes (including all20 matrix homes), both verifier
homes/short roots,35 observed VM socket directories,147 exact
socket paths,3 engines and853 immutable cache files (bytes/UID/GID/mode).
Passing cases clean their managed resources normally. Failed-run rm -f refuses
missing original PID for BOTH records; this safety check is preserved. Operator
cleanup verifies the exact directory shim PID79002/start/complete config, captures
lsof and a native stack sample, sends SIGTERM and verifies exit, no mount/disk/FD
holders. Only its exact owned socket directory is removed. Both failed homes are
RENAMED into failed-run-quarantine, preserving records, disks, volume data and
logs; original homes are fresh empty directories. These TWO quarantined metadata
records remain forensic evidence, not automatically removed product executions.
failed-owned-cleanup.json records every command/failure/signal/identity/move.
Audit3 independently verifies all current owned homes empty, no owned processes,
mounts/disk images, FD holders or sockets and both quarantined state hashes.
It explicitly sets operatorCleanupUsed=true, failedRunAutomatedCleanupPassed=false,
firstStartupRootCauseResolved=false, wholeArchiveAcceptanceComplete=false. No
automatic-cleanup success or full acceptance is claimed. Caller sources, failed
client/candidate, prior archives and failures remain; audit itself deletes nothing. Runtime RSS unmeasured. Historical default-gate user-home
nonmodification remains unestablished as035 corrected; current explicit isolated
native evidence is valid.

Arbitrary compatibility-constructor cancellation/entire-registry lifetime, initial
bind publication under hostile same UID, provider-inner/legacy/from_pid/template
allocation/capacity and other local ownership work remain. This phase closes the
actual production CLI's unpublished initialization gap, not universal Drop safety.
Full W4/default OCI Host Service cutover/live continuity/GA, original15s boot cause,
six disk/prior SDK gates, historical RAM loss, fresh-host/other-platform/conformance/
security/soak remain open. Latest user instruction is convergence/commit/merge/
push; no new optional feature stage is started. Overall goal stays unachieved.
The next required product task is a deterministic startup-timeout/cancellation
reproduction and original-owner cleanup, using the preserved failed records.

User authorizes scoped ordinary commit/merge/push. Baseline already local in main;
fresh04710:40:16Z failure-environment2-results.json still shows UID501 missing/
GitHub DNS errno8. No unchanged
push retry, force operation, fresh remote equality/merge or publication claim.
Current refs/source/diff review precedes authorized commit/verified incremental
bundle; actual post-commit refs/SHA in ignored artifact git-closeout.json.

## Explicitly not claimed

No green whole gate/full W4/GA/default OCI cutover/live continuation, native Linux
snapshot/fresh-host/other platforms/actual OS stop or unmount fault/hostile same-
UID namespace qualification/runtime memory measurement/platform promotion.
Windows ARM64 current dependencies unsupported, no native run. No KRun business
coupling, real ~/.kbox operation or native submodule content edit.

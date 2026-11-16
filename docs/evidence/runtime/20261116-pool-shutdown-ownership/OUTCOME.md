# Outcome037: registered pool shutdown ownership

Bounded registered-close implementation, final source2 installed native matrix
and owned-resource audit pass. Whole-release/GA remains incomplete.

## Actual failures and shared fixes

drain-red1 proves a published idle VM whose backend refuses stop is discarded
while drain returns success (0pass/1fail). Bulk teardown now registers actual
Arc/Mutex VmManager handles before yielding, removes them only after successful
finalization, propagates the original failure and retries the same owners on a
later drain. The collection has no execution-state copy. Idle statistics remain
zero for failed finalization. Fill rollback, template invalidation, eviction and
shutdown-mid-replenish use the same retaining teardown. Admission closes through
the existing watch channel; acquire and queued cold boots enforce it.

CLI red0/2 proves both early Stop ACK and removal of a leased entry/permit before
close. The registry now keeps entries/permits until success, checks exact Arc
identity before removal and applies this rule to release/expiry too. Failed TTL
cleanup is not counted as success. drain_all returns failure. Stop uses the
existing error field, waits for prior requests, excludes itself from that barrier
and asks the daemon to exit only after successful close/response. Signal/close
failure keeps the owning registry/listener available for retry. Startup is counted
in the request barrier and late initialization publishes its actual owner even
when admission has closed.

cancel-handler-red1 proves that retaining VmManager alone is insufficient: its
handler was taken into a local variable across cancellable Guest-control waits,
so cancellation actually dropped the handler. The shared destroy path now keeps
the actual handler in its existing owner slot until physical finalization.
cancel-handler-green1 passes without reducing assertions.

host-cleanup-red1 proves a rootfs cleanup refusal still wiped working data.
After gating deletion on cleanup success, red2 reaches the second defect:
Stopped caused retry to skip incomplete host cleanup. The manager now records
successful cleanup and its exact rootfs retention policy; this is a VM-local
resource observation, not another execution lifecycle. Boot resets it; success
publishes it. Rootfs/attachment cleanup refusal keeps the directory, incomplete
cleanup retries, preserving-stop can later become full ephemeral removal and
successful repeat close remains idempotent. Persistent missing Guest handoff
proof still refuses on retry. Stop metrics/event are counted once.

Anonymous-close red1 proves normal ephemeral close ignored its newly-created
anonymous allocations; temporary wiring then red2 proves the old boot-failure
helper forgot failed allocation identities. One shared Result-returning helper
now keeps failed names, removes only successfully finalized names and preserves
the first typed error. It runs only for the VM's own newly-created ephemeral
allocations after attachment cleanup; reused/caller/preserved allocations stay.
Boot-failure cleanup keeps its original best-effort caller/error semantics.

Socket replacement red1 proves shutdown unlinked a caller replacement file.
Shutdown now compares the bound socket type/dev/inode using the existing Engine
pattern and preserves/refuses changed identity. The actual listener/identity/
existing shutdown channel are bound before prewarm in BoundPoolListener; startup
error signals that exact server and awaits its checked cleanup. Initial binding
still has an older unowned-path/stale-endpoint gap, listed below for continuation.

## Actual checks, retained failures and candidates

Seven Runtime shutdown controls and three CLI shutdown controls pass, including
cancel/retry, allocation binding, host cleanup failure, retention policy, missing
persistent handoff and queued admission. The first admission fixture failed its
queue precondition (default permits exceeded one held permit); corrected explicit
single-permit setup retains every assertion, actual red2 reproduces admission and
green1 passes. This first fixture failure is not counted as product-red evidence.
One intermediate host-cleanup green1 was a compile failure after moving a local
variable; raw receipt retained, not counted as a pass. No assertion was removed.

Final source2 checks actually executed:

- Clippy2 for Runtime/CLI all targets passes; workspace all-target compile,
  formatting, support matrix and verification isolation4 pass.
- verify2 executes Runtime2322pass/6fail/52ignored, exit101. Same2 RAM/Secret and4
  APFS/provider disk-service failures remain. The default gate is not green.
- Final CLI938/0. Contract library24/0 and installer8/0 also ran this phase;
  full118 contract checks are unchanged036 evidence, not a new037 full run.
- vm-only compile passes. Earlier VM filter243/0/2 and pool controls are retained;
  final default Runtime also exercises the current assertions.

All commands/exit codes/private K_HOME/TMPDIR and raw results live under checks/.
The verifier allocates its own private roots; test-counts.json and
verification-roots.json preserve exact counts/owner binding. No real user-home
scan, cleanup or repair. Historical unisolated default-gate claims stay corrected
by035, with old evidence retained.

Source1/candidate1 were built and tested, then superseded after final caller
review found shutdown-mid-replenish dropping a just-ready manager. Its archive
71b5644133b7596bf11487fceab714cfa5ea06ee683b7c4a3836f51e9b03f2a1 remains;
no native VM ran against candidate1. Source2 adds that retaining caller,
bound-listener ownership and actual admission enforcement. Every affected final
artifact/check is rebuilt; candidate1 does not qualify current source.

## Final source and installed native evidence so far

Final compiled/package input digest
`dde4c61ac1c2348111de6c8ca72d512836d47637cd7bacc6744fc20cb6bdd33d`,1726 inputs,
based on local main86a6bf08 plus recorded changes. Frozen Rust client SHA256
`6f75291a6869e67f3f096e18d11d10ad76f189a691676bd27d2a2a763a299cde`.
Final Box archive
`artifacts/pool-shutdown-ownership-fe886a631080491db66abe1b4f2deac8/final-dist-poolstop-2/kbox-3.4.3-macos-arm64.tar.gz`,
SHA256 `6326607e094f4390c561eedf11b15ea6719b36769506e73aa4eebf59c6b1dafe`,
23415039 compressed bytes /44191744 installed bytes. Fresh prefix is
`artifacts/pool-shutdown-ownership-fe886a631080491db66abe1b4f2deac8/installed-poolstop-final-2`.
All11 manifest files/source provenance, CLI/shim ad-hoc signature/arm64,
guest-init Linux aarch64, native libraries and embedded OCI Agent identities pass
final-identities-2.json. No notarization/publication or runtime RSS measurement.

Separate OCI source/payload unchanged: archive
d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79,
original source d2b442f175f89b34d6ba9b63e6d2df4a478dc2a6c3493f8ca647a761e9012871.
Original owning-repository native patch provenance remains; parent gitlinks stay
libkrun3c2889c7/libkrunfwfc3ea068. No native submodule source edit in this parent.
Previously accepted Boxd5393321 and older archives/checksums remain retained.

Native-fault2 actually passes1/0/0 using the frozen client, installed companions
and captured loaded library identities on this macOS arm64 host. An actual VOLUME
VM keeps writing after cancellation of a deliberately withheld test control ACK,
then after one synthetic backend stop refusal. Exact handler/PID/allocation/
directory identity and template input remain. Actual stop retry removes VM,
created anonymous volume, sockets and owned input, with no late writes/caller
loss. The control stall/one-shot refusal are synthetic; VM, writes, allocation
identity, actual finalization and cleanup are real. Cold-boot input guard, not a
native Linux snapshot mapping/restore claim. The Rust stdout/count and exit code
are checked alongside result.json; internal result alone is insufficient.

The final installed AF matrix passes all5 rows:3 Python scenarios (OCI lifecycle,
Host RO attacks/restart and engine process flow) plus2 actual Rust native tests
(process prewarm and directory grants/fencing). Together with the new fault test,
actual final counts are3 Rust native +3 Python scenarios. RO checks include real
Host14 denials on both first/restart probes after guest RW remount/fresh mount and
child attempts, with RW control intact. Native stdout counts/exit and complete
installed-loader maps are checked, not only PASS result fields.

The separate CLI matrix passes4 daemons,16 initial authenticated Ready VMs and4
Guest commands. All4 pre-existing image-hash template directories preserve their
exact dev/inode/bytes, and installed loader identities are captured. One Mac
unsupported-snapshot request retains its existing same-backend cold optimization
path; this is not Linux snapshot functionality or an isolation fallback.

Final-cleanup.json and scoped inventories/FD receipts verify52 private homes,
both owned roots,
both verifier short temporary roots,35 observed VM socket directories,146 exact
socket paths,2 engines and713 pre-existing immutable cached files. Owned managed
executions/volumes, VM/shim/engine/Host process checks, mounts/disk images, FD
holders and sockets are empty. Only the mutable image catalog changes. Caller
sources/canaries and prior/candidate1 archives remain. Audit deletes nothing.
Raw final commands/results and copied native proof live in checks/native-case,
checks/af-final2 and checks/pool-wave-final2; actual counts in test-counts.json.

Commands use a fresh task-owned K_HOME through the recorded run-check.py:

```sh
cargo test --locked --offline -p kbox-runtime --lib pool::warm_pool
cargo test --locked --offline -p kbox-cli --lib commands::pool
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli --all-targets --no-deps
cargo check --locked --offline -p kbox-runtime --no-default-features --features vm
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

The recorded artifact root contains source_snapshot.py, source-frozen-2.json,
build-box-final2.py, check-final-identities2.py, native_stop2.py,
qualification-runner4.py, pool_stress.py and final-audit.py. They install only
archive payloads, isolate homes, clear loader overrides and preserve failed raw
results. Package command sets K_BUILD_LIBKRUN=1; native source is unchanged.

## Explicitly not claimed / continuing local work

Pre-admission foreground boot/restore/postboot/health transfer and legacy CLI
boot/compose error paths can still drop local managers; this bounded phase covers
registered pool/lease teardown and its shared finalizer, not every manager lifetime.
Initial socket bind still blindly replaces paths and needs a separate owned/stale
endpoint boundary. Arbitrary same-UID hostile namespace races, actual OS stop or
unmount failure, Linux snapshot native, Windows native, full W4/default OCI Host
Service cutover/live continuity and enterprise GA are not qualified. Windows ARM64
current dependencies remain unsupported/no native run. Support levels/default
libkrun route stay unchanged; RSS unmeasured. Original15s boot cause, six disk
gates/prior SDK failure, historical unexplained RAM loss and other-host/fresh-host/
conformance/security/soak remain open. Git UID501/GitHub DNS still block remote
synchronization; only local commits/previous baseline ancestry are proved.

After current final native/audit/source/diff/commit closeout, continue the next
local owner/bind gap. Goal remains active; there is no whole-project completion.

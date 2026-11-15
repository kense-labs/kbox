# Real-time process limits: accepted

Spec026 carries existing Box rtprio/rttime intent through standard OCI limits
and the unchanged Guest executor. Only two standard enum mappings and the Box
qualification text change production. CLI/UID defaults, per-UID counter guards,
OCI/Agent/schema/native sources/dependencies remain unchanged. Reverse patch
matches025 production exactly. [Source delta](checks/minimal-production-delta.json).

Final source2:9365453fbda6b3b8bd4758ae24c6a7b2817b5a0c130ec90724e75d3e46b3e676,
1701 inputs. Box2 archive SHA256
fb44d46e0fdd8f70d56c88a46cfdb6bc005778eadf7ce109dc968fb9119e6bc4,
23,392,959compressed/44,171,264installed bytes,11 companion checksums.
Unchanged OCI022 is freshly extracted:37009193a985f12eddced69d1ee5214650ae2e7e1d449ba5787802195bb381e4,
33,421,132compressed/114,950,144installed bytes; original source4f9dbe71…,
11 checksums. [Install](checks/install-realtime-final2.json),
[OCI provenance](checks/reused-oci-install1.json),
[architecture/signature/native/embedded Agent identities](checks/final-identities-2.json).
Environment:macOS26.6.2(25G83),M5Max arm64,euid501,Rust1.97.1.
[Environment](checks/host-environment.json),[all owned paths](run-context.json).

Red admission/completion:0passed/2failed/0ignored; after mapping20passed/0failed/
1ignored. Verify2 exits0 in208.41s:Runtime2259passed/42ignored,CLI935,installer8,
workspace all-target check/fmt/scoped Clippy. Verify1 remains separate historical
result. [Red](checks/realtime-before1.json),[focused](checks/realtime-after1.json),
[final verify](checks/verify2.json).

Native3 passes1/0/0 in277.55s native/278.74s wrapper. Three generations have15 observations across
init/capture/child/stdin-stream/realPTY; explicit1001:2002 also passes. FIFO/RR5
succeeds,6returnsEPERM1; child/thread inheritance, range reuse, hard-raise refusal
and entered-root/mapped-nonroot namespace boundaries pass. Continuous real-time
CPU produces actualSIGXCPU24/SI_KERNEL128 near100000us andSIGKILL9 near300000us;
blocking resets allow400000us aggregate and SCHED_OTHER survives360000us.
Infinity actually reaches99 and survives360000us; zero priority refuses FIFO/RR.
Independent zero RTTIME actually receivesSIGKILL9 after6600us measured child
CPU. Every control owns a distinct caller-file inode; children are reaped and
threads joined. Public/CLI root defaults and default nofile1024 remain.
Replay repeats one actual effect, changed requests/stale generations refuse,
normal restart and stopped Host recovery retain identity and advance generation.
CLI healthy, explicit nonroot, realPTY exec and attach pass the installed2 binary.
[Actual native counter](checks/box-native-realtime-attempt-3.json).

Final matrix totals **5 outer Rust native tests +3 Python native scenarios**,
all pass, with no new Guest unit run. Current installed2 Host Service mixed
routes/foreign-rootfs/capability/RO/replay/fault/recovery and ordinary-file Host
PEP each execute1/0/0 (385.23s and43.27s wrappers). A–F all5 pass in137.48s:
2Rust(prewarm/directory)+3Python(OCI lifecycle/build/export/commit, actual Host
RO attacks/restart, process flow). [Host](checks/box-native-host-service-attempt-2.json),
[files](checks/box-native-files-attempt-2.json),[A–F](checks/af-final1/native-results.json),
[strict asset/source/effect/count review](checks/final-review2.json).
Unchanged resource/Secret/Guest matrices retain their actual024/025 identities;
they are not counted as new026 executions. Exact production delta proof and
the complete new default baseline cover the bounded two-mapping change.

Failures retained:Native1 stopped at musl sched_getparam ENOSYS38; installed
Zig0.16 musl's three POSIX process scheduler wrappers are unconditional stubs.
C2 directly invokes the Linux syscalls with identical arguments/assertions.
Native2 readiness used absent/bin/test; fixture now calls the existing shell
builtin and still requires actual success. Rust fixture source/client/Box2 and
verify2 were regenerated. Every runtime payload matches1 except build-info.
Diagnostic2 independently captured actual signals/CPU and successful effects
from installed1; it is a diagnostic, not a final matrix substitute.
[Failures](FAILURES.md),[stub source proof](checks/libc-scheduler-stub-proof.json),
[diagnostic](checks/realtime-diagnostic2-result.json),
[C2 source/ELF/CLI hashes](checks/probe-identities2.json),
[payload equivalence](checks/production-equivalence1-to2.json).

Primary pinned kernel sources:Linux6.12.76
[scheduler permission checks](https://github.com/gregkh/linux/blob/v6.12.76/kernel/sched/syscalls.c),
[RT watchdog](https://github.com/gregkh/linux/blob/v6.12.76/kernel/sched/rt.c),
[per-thread CPU timers](https://github.com/gregkh/linux/blob/v6.12.76/kernel/time/posix-cpu-timers.c).
The actual native configuration has HZ100; limits are accounted in clock ticks,
not guaranteed exact microsecond termination. Source copies and hashes retained
in [primary-source record](checks/official-kernel-source.json).

Artifact root:artifacts/oci-realtime-b71041f587734cec8ffe7d63a68940ca.
Private root:/private/tmp/kbox-realtime-b71041f58773. Actual commands, logs,
exit codes and loader/descriptor identities are retained in checks. Drivers:
build-box-final2.py,check-final-identities2.py,run-native.py,
qualification-runner-final1.py,diagnose-realtime1/2.py,final-resource-audit.py,
final-review2.py. Original source/C1/C2/ELFs/diagnostics/failed attempts/candidate1
and prior archives/checksums remain. [Prior artifacts](checks/prior-artifacts-retained.json).

Final cleanup audits12 homes including both failures and both diagnostics,
93 observed socket paths, all owned processes/mounts/runtime disks/FD holders
and all7 initial RAM devices. Zero managed executions/volumes/owned processes/
mounts/disks/sockets/FD holders; no new RAM device. Audit deletes nothing and
leaves the unrelated mounted image untouched. All23 native and367 A–F baseline
cache blobs/owner/mode persist unchanged; index-only last_used/owned path updates
and one exact owned A–F build are validated against its result/current CLI SHA.
[Cleanup](checks/final-cleanup.json),[FDs](checks/final-fd-holders.json),
[disks](checks/final-runtime-disks.json),[cache](checks/final-cache-audit.json).

## Explicitly not claimed

No CPU reservation/fairness, aggregate lifetime/cgroup budget, normal-policy
CPU or wall deadline is supplied by these standard limits. Runtime RSS is
unmeasured. Full W4/default Host Service cutover/live continuity/fresh-host or
other-platform native matrices and platform/readiness promotion remain open.
macOS default remains BoxVm; HVF recovery remains stopped-only. Windows ARM64
dependencies unsupported/no native run. No VMM/firmware port, real Secret,
real ~/.kbox mutation, unrelated cleanup, commit/push/publication.

Next existing local W4 slice: managed Secret with raw anonymous volumes. Existing
Box/SDK refusal and versioned transport boundaries must be preserved while
qualifying any new combination; this acceptance does not end the continuous task.

# Nonroot process-count limits: accepted

Bounded spec022 is accepted on this existing macOS arm64 host. Box now carries
nonroot nproc intent through OCI RLIMIT_NPROC and the shared Guest executor.
Admission requires an explicit numeric nonzero UID, empty capability sets and
no_new_privileges. Every init/exec/stream/PTY entry must retain that UID; root,
implicit-root, different-UID and privileged combinations refuse before entry.
This is the kernel's real-UID process/thread budget, independent of cgroup pids.
Public Exec/Pty user=None still means root0; the CLI explicitly supplies the
configured UID only for authoritative persisted OciSdk nproc records, including
health and attach. Identity, route, replay and generation fencing are retained.

The first native run found a real root-helper bypass: setuid checked the old
target-UID count before accounting the joining process, allowing one additional
valid exec at a full quota. Shared Agent setup now reserves one temporary fork
slot, performs an actual trusted fork/reap after credentials and capabilities,
then restores/readbacks the exact requested soft/hard values before workload
entry. Zero refuses, the last legitimate one-process slot remains available,
and admission failure refuses launch. No workload PID replacement or second
process counter is introduced. The kernel source used for diagnosis is retained
with identity in the artifact root, including [Linux6.12.76 setuid](https://github.com/gregkh/linux/blob/v6.12.76/kernel/sys.c)
and [fork accounting](https://github.com/gregkh/linux/blob/v6.12.76/kernel/fork.c).

Required Host nproc-admission v1 and Agent manifest v10 prevent use of older
unenforcing assets. v10 binds its original v1–v9 transport schema; old transport
bytes and validation retain their meaning. HVF and KVM producers both use the
shared wrapper. Old Host, shim and Agent refuse without workload Start or route
fallback; committed pending Create cleanup resumes that exact operation without
Start, then ForceDeletes its exact binding.

Actual CLI health also exposed a shared bare-command lookup defect: HVF has no
materialized Host rootfs for PATH lookup. Capture, stream and PTY now delegate
bare names to the existing Guest resolver with the effective image/default PATH;
explicit PATH and the legacy materialized-root path/request bytes remain intact.
The health fixture still uses bare sh. Existing path escape checks remain.

## Frozen installed assets

Environment: macOS26.6.2(25G83), Apple M5Max arm64, euid501, Rust1.97.1.
Final global product/test source5 is
`cc9e1c37a0012ce51169262970c27f90d110ef47648fea8a373952fbcd32d4d0`,1697 inputs.
All uncommitted work is retained. [Context/absolute paths](run-context.json),
[final source/count/identity review](checks/final-review5.json).

| Installed archive | SHA-256 | Compressed bytes | Installed bytes |
| --- | --- | ---: | ---: |
| Box5 kbox-3.4.3-macos-arm64.tar.gz | f190440c72f88159c6c33f40050f4a5dbe556e6e369e3b85f45805e116b34c0a | 23,392,791 | 44,171,264 |
| OCI3 koci-hvf-0.3.6-macos-arm64.tar.gz, fresh extraction5 | 37009193a985f12eddced69d1ee5214650ae2e7e1d449ba5787802195bb381e4 | 33,421,132 | 114,950,144 |

Box uses source5 above. OCI retains original source3
`4f9dbe71f19a9d018e40b0a58e5a62e2dd2cde5d07c9698f7e710c9a8f2a648b`;
every OCI/Agent source is unchanged since that freeze. All11 companion hashes
per installation, arm64 Mach-O/static arm64 ELF identities, strict ad-hoc code
signatures, native dependencies and embedded/standalone Agent match. Actual
loader observations use the installed prefixes, with distinct Box/OCI libraries.
Guest Agent SHA is76e1bc15…; Box CLI40e99305…; shim296e2ebd…; guest-initf2b6d327… .
The full hashes, signature/architecture commands and image/native identities are
in [final identities](checks/final-identities-5.json),
[Box install](checks/install-nproc-final5.json),
[OCI original build](checks/oci-nproc-final3-install.json) and
[fresh OCI5 extraction/provenance](checks/source-freeze5-oci-reuse.json).
Native repositories remain clean at libkrun3c2889c7… /libkrunfwfc3ea068…;
Cargo.lock remains0f82d0ea… . No parent submodule content edits.

## Actual execution counts and effects

Final matrix totals **12 outer Rust native tests,13 Guest unit tests,3 Python
native scenarios and1 separate native VM decoder probe**, all passed. Nested
libtest subprocess probes and earlier calibration passes are not counted twice.
Each selected ignored native Rust test is checked against --list, actually
executes1passed/0failed/0ignored and must produce result.json and empty cleanup.

The final nproc entry executes1/0/0 in236.06s. Fifteen observations cover init,
captured exec, child, stdin streaming and real PTY across generations1/2/3.
Actual fork/thread EAGAIN11, reaping/joining and budget reuse, hard-raise EPERM1,
numeric1001:2002, zero capabilities/NNP and bounded forks inside an entered,
mapped user namespace all pass. A separate unlimited execution forks32 children
and creates32 threads using a separate owned probe inode. Full-budget valid
exec, concurrent exec, stream and PTY refuse with actual EAGAIN and no workload
marker; invalid root/None/other-UID entries refuse before RPC. Replay has one
side effect; conflicts/stale generations refuse; normal restart and Host-death
stopped recovery retain limits. CLI health is healthy, exec -t and attach -it
both run on real TTYs, including CLI restart. Guest rlimit units execute5/0/0;
their zero/one-slot child is truly single-threaded (launcherThreads2/probeThreads1).
[Nproc actual receipt](checks/box-native-nproc-attempt-5.json),
[main wrapper/evidence](checks/nproc-native5.json).

Old Host/shim/Agent each execute1/0/0, with no marker, no fallback and zero Starts
in exact pending-Create cleanup. The separate actual installed VM worker probe
captures the exact old Agent's unknown nprocTransportSchema error and waitpid
exit1 before token consumption. This uses a clearly marked **test-only image**:
the unchanged old Agent ELFfa5e1d36… plus a tiny stderr-to-console/wait launcher;
it is not claimed as the original archive image. The original-image public-API
refusal is separately tested and linked by old Agent/original manifest hashes.
Probe worker is reaped and its exact socket directory removed.
[Old Host](checks/box-native-nproc-old-host-attempt-5.json),
[old shim](checks/box-native-nproc-old-shim-attempt-5.json),
[old Agent](checks/box-native-nproc-old-agent-attempt-5.json),
[native decoder/fixture identities](checks/old-agent-direct6.json).

Final5 mixed Host Service/recovery/foreign-rootfs/capabilities/replay/fault and
single-file Host PEP entries each execute1/0/0. Secret ENV executes1Rust plus
8Guest units, all process paths, old-asset refusal, three generations and owned
RAM detach. A–F executes2Rust+3Python: OCI lifecycle/build/export/commit, actual
Host readonly attacks/restart, process flow, prewarm and directory grants.
[Host Service](checks/box-native-host-service-attempt-5.json),
[files](checks/box-native-files-attempt-5.json),
[Secret ENV/RAM cleanup](checks/environment-positive-5/result.json),
[A–F actual counts/results](checks/af-final5/native-results.json).

CPU, data/stack/mlock and nofile/fsize/core each execute1/0/0 using installed4
in323.09s,195.94s and194.62s respectively. They retain their actual4 source/
archive identities. Source4→5 changes only the negative-probe fixture; every
packaged runtime payload is byte-identical except build-info.json, proven by
[file-level equivalence](checks/production-equivalence4-to5.json).
These unchanged successful checks are retained, not relabelled as new5 runs.

Final scripts/verify-commit.sh exits0 in221.00s: Runtime2251passed/38ignored,
CLI935passed, installer8; all-target workspace check, fmt and scoped Clippy pass.
Owning OCI Runtime438passed/2ignored, AgentProtocol70passed, SDK227library plus
18integration passed. Linux Agent cross-build/Clippy and actual Guest tests pass;
cross-build itself is not native platform evidence.
[Verify command/log](checks/verify5.json), [verify log](checks/verify5.log),
[OCI owning](checks/oci-runtime-owning1.json),
[protocol/SDK owning](checks/protocol-owning2.json),
[Agent Clippy](checks/agent-linux-clippy3.json).

Artifact root: artifacts/oci-nproc-bcf6f36c2d7d42a0bd0fc9ea6dcec09e.
Reproduction commands are retained in each JSON and its stdout/stderr; actual
driver scripts include build-box-final5.py, package-oci-final3.py,
check-final-identities5.py, run-native.py, run-memory-native.py,
qualification-runner-final1.py, probe-old-agent-direct6.py,
final-resource-audit.py and final-review5.py. Prior archives and all failed
source/probes/images/logs remain. [Failure/root-cause ledger](FAILURES.md)
includes the kernel bypass, CLI bare-PATH bug and test-fixture calibration.

## Owned cleanup

The strict audit checks29 persisted/prepared homes and210 observed socket paths,
including all failed attempts and diagnostic roots. Executions, volumes, owned
VM/shim/engine/Host processes, workload/runtime disks, mounts, FD holders and
sockets are zero. ENV's exact owned RAM disk is unmounted/detached; all7 initial
RAM devices and the unrelated mounted image remain. The audit deletes nothing.
All23 native and297 A–F baseline cache files retain content/immutable metadata;
only owned-path/last_used index changes and two exact owned A–F build additions
are admitted, each bound to its original result digest and CLI hash.
[Cleanup](checks/final-cleanup.json), [FDs](checks/final-fd-holders.json),
[disks](checks/final-runtime-disks.json), [cache audit](checks/final-cache-audit.json).

## Explicitly not claimed

This is per-UID process/thread accounting, not aggregate Host PIDs, cgroup pids
replacement or privileged/root enforcement. Raw public callers still must
supply the qualified UID. Old assets explicitly refuse; refusal is not parity.
Runtime RSS is unmeasured. Complete OCI Host Service/default cutover and full
W4 remain incomplete; macOS default stays BoxVm, HVF owner death stays stopped
only. Fresh-host, other-platform native matrices, conformance/security/soak,
live continuity and platform/readiness promotion remain open. Windows ARM64
current dependencies remain unsupported without native validation. No port,
real ~/.kbox mutation, real Secret, unrelated cleanup, commit/push/publication
or fabricated missing historical evidence.

Continue directly to the next existing W4 process-limit gap; this accepted
bounded slice does not end the continuous task.

# POSIX message-queue byte limits: accepted

Bounded spec024 is accepted on this existing Mac arm64 host. Existing Box
msgqueue intent reaches OCI RLIMIT_MSGQUEUE and the existing Guest executor.
The common numeric nonroot UID/empty-capability/NNP boundary and CLI authoritative
default UID now include this standard type. Init/capture/stream/PTY retain the
configured UID; public None remains root0 and refuses. Other defaults, nproc/
sigpending behavior, persistent identity/route/replay and generation fencing
remain intact. Only the Box mapping/guard and existing CLI typed match change
in production; no OCI/Agent/native source, schema/capability, config, dependency,
ABI, workspace or resource state is added.

The standard byte budget includes queue capacity and kernel accounting overhead;
message receive or unlink alone does not release a still-open queue's capacity.
The probe distinguishes mq_open EMFILE from ordinary FD availability and from
per-queue send EAGAIN. Primary contract: [Linux6.12.76 mqueue allocation/eviction](https://github.com/gregkh/linux/blob/v6.12.76/ipc/mqueue.c).
Pinned aarch64 configuration enables POSIX_MQUEUE, IPC_NS and USER_NS. The syscall
probe uses the namespace's internal MQ filesystem without adding any Host or
workload mount. This is POSIX MQ accounting, not a generic IPC/memory quota.

## Installed assets

Environment: macOS26.6.2(25G83), M5Max arm64, euid501, Rust1.97.1.
Final source1 is
`b91eb406769969d757789ed8ff09ae31e91b4ae1b3687a4cc3dea89b7afbceef`,1699 inputs.
All original uncommitted work and failed artifacts remain.
[Actual environment](checks/host-environment.json), [absolute paths](run-context.json),
[source delta](checks/source-freeze1.json), [final receipt/effect/count review](checks/final-review2.json).

| Final installed archive | SHA-256 | Compressed bytes | Installed bytes |
| --- | --- | ---: | ---: |
| Box1 kbox-3.4.3-macos-arm64.tar.gz | 6b0e49a9a045621d367de3c6e0608f75e578ba752612ee63452afc47758d6709 | 23,392,838 | 44,171,264 |
| Unchanged OCI022, fresh extraction | 37009193a985f12eddced69d1ee5214650ae2e7e1d449ba5787802195bb381e4 | 33,421,132 | 114,950,144 |

OCI retains original source
`4f9dbe71f19a9d018e40b0a58e5a62e2dd2cde5d07c9698f7e710c9a8f2a648b`;
all OCI/Agent inputs are unchanged from accepted023. All11 companion hashes per
installation, arm64 Mach-O/static arm64 ELF, strict ad-hoc signatures, native
links and embedded/standalone Agent match. Actual native loaders bind workers
and libraries to these installed prefixes. Parent native source repositories
remain clean at libkrun3c2889c7… /libkrunfwfc3ea068…; Cargo.lock0f82d0ea… .
[Box install](checks/install-msgqueue-final1.json),
[OCI fresh install/original provenance](checks/reused-oci-install1.json),
[all file/Agent/image/dependency/signature identities](checks/final-identities-1.json),
[clean submodules](checks/native-submodules.json),
[prior archives and independent SHA preservation](checks/prior-artifacts-retained.json).

## Actual counts and effects

Final totals: **11 outer Rust native tests +13 Guest unit tests +3 Python native
scenarios**, all passed. Every selected ignored Rust native test is verified
against --list, executes1passed/0failed/0ignored and emits PASS result.json plus
empty cleanup. No nested helper test, diagnostic, failure or historical pass is
counted again.

MQ2 executes1/0/0 in247.08s native/248.82s wrapper. Fifteen observations span
init/captured exec/child/stdin stream/realPTY across generations1/2/3. With an
actual2048-byte RLIMIT_MSGQUEUE, one four-slot/256-byte queue succeeds; creating
another returns EMFILE24 while /dev/null still opens. Full per-queue send returns
EAGAIN11; payload bytes, priorities and equal-priority FIFO order are read back.
Draining messages does not release the reserved capacity. Unlinked names are
actually ENOENT while descriptors remain usable. Dup and fork-inherited FDs
retain quota until the last close; same-UID parent/child admissions share it and
reuse succeeds after close/reap. A fully UID/GID-mapped user namespace and new
IPC namespace still return EMFILE24 at full parent quota, with child reaped.
[Main actual receipt](checks/box-native-msgqueue-attempt-2.json).

Two parallel same-UID valid exec probes at a held full allocation cannot obtain
a fresh budget. Root/implicit/other-UID capture/stream/PTY refuse before entry;
an actual guest !-e && !-L verifies no canary. Separate infinity control holds
four queues; zero permits startup/ordinary FDs but no MQ allocation. Hard raise
is EPERM1; default nofile1024 remains. CLI exec/healthy health/realTTY/attach/
restart pass; replay has one effect, conflicts/stale generations refuse, and
normal restart/stopped Host recovery retain limits. Every control uses a distinct
probe inode and the zero control explicitly asserts persisted OciSdk routing.
[Final C2 source/ELF/CLI driver hashes](checks/probe-identities2.json).

Current installed archive also passes nproc1Rust+5Guest units, sigpending1Rust,
CPU1Rust, data/stack/mlock1Rust, nofile/fsize/core1Rust, mixed Host Service/
foreign-rootfs/capability/RO/replay/fault/recovery1Rust, single-file Host PEP1Rust,
and Secret ENV1Rust+8Guest units with owned RAM detach. A–F passes2Rust+3Python
in155.92s: OCI lifecycle/build/export/commit, Host RO attacks/restart, process
flow, prewarm and directory grants. Guest unit ELFs are reused unchanged with
their original hashes and actually execute on the current installed OCI VM.
[Nproc](checks/box-native-nproc-attempt-1.json), [sigpending](checks/box-native-sigpending-attempt-1.json),
[CPU](checks/box-native-cpu-time-attempt-1.json), [memory](checks/box-native-memory-limits-attempt-1.json),
[old limits](checks/box-native-ulimits-attempt-1.json), [Host](checks/box-native-host-service-attempt-1.json),
[files](checks/box-native-files-attempt-1.json), [Secret/actual Guest8/RAM cleanup](checks/environment-positive-1/result.json),
[A–F actual counts/results](checks/af-final1/native-results.json).

Before1 admission/completion executes0/2/0 and CLI0/1/0. Final focused limits
execute16passed/0failed/1ignored; CLI1passes, including metadata authority and
all other route/default cases. Zero/infinity/u64, exact enum/soft/hard/defaults,
UID/capability/NNP, idempotence, malformed/duplicate/inverted/overflow and other
profile refusals are covered. scripts/verify-commit.sh exits0 in205.62s:
Runtime2255passed/40ignored, CLI935, installer8; workspace all-target compile,
fmt and scoped owning Clippy pass. Unchanged OCI/SDK/Agent owning results remain
historical [accepted022 evidence](../20261115-oci-nproc/OUTCOME.md), not new024 runs.
[Focused](checks/msgqueue-after2.json), [CLI](checks/cli-after1.json),
[verify command](checks/verify1.json), [actual log](checks/verify1.log).

## Failures, commands and cleanup

Shared test-helper extraction first retained a Sigpending expected enum; its
explicit Msgqueue argument now supplies the independent expectation. All
existing resource/entry assertions remain. Native1 exits80 before readiness.
The installed CLI diagnostic obtains the exact C stderr: namespace MQ create
returns EOVERFLOW75 because only UID was mapped before new IPC inode creation.
C2 maps its own GID after setgroups=deny and checks getgid0; the original exact
EMFILE24/ordinary-FD/payload/lifetime expectations stay intact. All15 complete
mapping observations pass afterward. C1 source/ELF, failed native1, diagnostic
and cleanup records remain; only external C2/CLI test assets regenerate.
Product source/client/archive and successful verify stay exact.
[Failure ledger](FAILURES.md), [installed diagnostic](checks/msgqueue-diagnostic1-result.json).

Artifact root: artifacts/oci-msgqueue-e437302b77104ac0a5c447e1455be2c9.
Commands, arguments, counts, stdout/stderr and loader/descriptor observations
are in checks JSON/logs. Executed drivers include build-box-final1.py,
check-final-identities1.py, run-native.py, run-memory-native.py,
qualification-runner-final1.py, diagnose-msgqueue1.py,
final-resource-audit.py and final-review2.py. Archives/checksum files retained.

Final audit covers15 isolated homes including failed native1 and diagnostic1,
and129 observed socket paths including owned private state. Executions, volumes,
owned VM/shim/Host/engine processes, mounts, workload/runtime disks, sockets and
FD holders are zero; no live Guest remains to retain MQ objects. Probe queue
names are unlinked and descriptors closed/reaped. Secret RAM12 unmount/detach
exits0; all7 initial RAM devices and the unrelated mounted image remain. Audit
deletes nothing. All23 native and339 A–F baseline cache files preserve blobs,
owner and mode; index changes only update last_used/owned paths plus one exact
owned A–F build bound to its result digest and CLI hash.
[Cleanup](checks/final-cleanup.json), [FDs](checks/final-fd-holders.json),
[disks](checks/final-runtime-disks.json), [cache](checks/final-cache-audit.json).

## Explicitly not claimed

This standard limit controls POSIX MQ capacity accounting, not SysV IPC, all
IPC traffic, aggregate memory/RSS or disk usage. Root/privileged combinations
remain unqualified. Runtime RSS is unmeasured. Complete OCI Host Service/default
cutover/full W4/live continuity, fresh-host/other-platform native matrices,
conformance/security/long soak and platform/readiness promotion remain open.
macOS default stays BoxVm; HVF owner death stopped-only. Windows ARM64 current
dependencies remain unsupported without native validation. No VMM/firmware port,
real ~/.kbox mutation, real Secret, unrelated cleanup, commit/push/publication
or fabricated missing historical evidence.

Continue directly to existing nice-limit intent. This accepted slice does not
end the continuous task.

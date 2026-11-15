# Data, stack and locked-memory limits: accepted

Bounded spec021 is accepted on this existing Mac arm64 host. Box carries its
existing data/stack/memlock byte limits through standard OCI process.rlimits
and the existing Agent setrlimit/getrlimit executor. The shared parser retains
zero/infinity/u64 and duplicate/inverted/malformed rejection semantics. Existing
CPU/nofile/fsize/core, default request bytes, routes, identity, replay and
generation fencing remain intact. Only three standard enum mappings are added
in production; no config, ABI, schema, dependency, resource state or native
source change. These are process controls, separate from cgroup/VM memory/RSS.

Final product source is
`e02147be72c10c307955ad416a5daa41a3dc5b60cad7c682c405ce0b3dc4ffcc`,1694 inputs.
Exactly five Box runtime files differ from accepted020; every OCI/Agent/Host/
storage source input remains unchanged. [Source delta proof](checks/source-freeze1.json),
[final receipt/identity/count review](checks/final-review1.json).

| Final installed archive | SHA-256 | Compressed bytes | Installed bytes |
| --- | --- | ---: | ---: |
| Box1 kbox-3.4.3-macos-arm64.tar.gz | 79c97ef7dd7462375270db78ab736e8b72c12a7c900ed01d2b0de860769a9ce3 | 23,390,507 | 44,154,880 |
| Unchanged OCI019 koci-hvf-0.3.6-macos-arm64.tar.gz | 542bfe43dd6f2837ad55c3ba79416832584e9032a9b6e1b2bfa4d75bc94203bf | 33,417,962 | 114,925,568 |

Box uses the source above. OCI is freshly extracted in this run and retains
its original019 provenance
`03a62c3e7d246fa4f8eb1e758d338aa3fc9bb25a75ee6f6ce94259c736986b35`.
All11 companion hashes per install, actual architectures/signatures/native
library dependencies and embedded/standalone Agent match. Native VM loader
proof uses these installed prefixes, with distinct Box/OCI libraries. Native
repositories remain clean at libkrun3c2889c7… /libkrunfwfc3ea068…; Cargo.lock
stays0f82d0ea… . [Box install](checks/install-memory-limits-final1.json),
[OCI install/original provenance](checks/reused-oci-install1.json),
[full file/Agent/image/dependency identities and commands](checks/final-identities-1.json),
[paths/environment](run-context.json). Prior archives and failed probe binaries
remain retained; development/runtime user installations are not borrowed.

Focused before tests execute0passed/2failed/0ignored at admission/completion.
After the mappings,29passed/0failed/4ignored cover exact byte values including
zero/infinity, unchanged CPU/default nofile values, malformed/duplicate/overflow
rejection before allocation and other-platform refusal. Final verify1 exits0
in231.66s: Runtime2242passed/36ignored,CLI934 and installer8. All-target compile,
fmt and scoped Clippy pass with retained warnings.
[Before](checks/memory-before1.json), [after](checks/memory-after1.json),
[verify command](checks/verify1.json), [actual counters](checks/verify-counts1.json).

The final memory entry executes1passed/0failed/0ignored in180.27s. Across init,
captured exec, child, streaming stdin and real PTY in generations1/2/3, all15
observations show4MiB data/512KiB stack/64KiB locked-memory limits, malloc/mmap
and excess mlock ENOMEM12, successful64KiB lock plus unlock/reuse, actual stack
SIGSEGV11/SEGV_MAPERR on an alternate signal stack (125–126 recursive frames),
and three hard-limit EPERM results. A simultaneous independent execution with
all three limits at infinity successfully allocates/maps8MiB, locks above64KiB
and reports513 actual recursive frames. The main limits/effects remain intact.
Replay has one side effect, changed requests/stale generations refuse, normal
restart/stopped Host recovery and CLI/restart pass. Controls use distinct exact
probe inodes, preserving the caller-file alias boundary.
[Actual native count](checks/box-native-memory-limits-attempt-2.json),
[effects and unlimited control](checks/memory-effects-final1.json).

Native1 remains a failed0/1/0. Its init exited16 without OOM; diagnostic1 through
the installed CLI verified exact limits and zero capability masks. Emitted code
proved the probe's unobservable malloc/free calls were optimized away, so its
claimed successful allocation was not a kernel operation. Probe4 uses
-fno-builtin-malloc/-fno-builtin-free, retains every original effect assertion
and emits two actual malloc calls. Original/native1/diagnostic3 source and ELFs,
commands, failed events and assembly remain. Only this external probe was
regenerated; product source, client and archives stayed exact.
[Root cause](checks/optimizer-root-cause.json), [actual calls](checks/probe-actual-allocation-calls4.json),
[final fixture identities](checks/probe-identities2.json), [failures](FAILURES.md).

Current-archive CPU and prior nofile/fsize/core native entries each execute1/0/0
in311.20s and182.66s. A–F all five pass in141.00s: actual OCI lifecycle/build/
export/commit, Host readonly attacks+restart, process flow, prewarm and directory
grants. Prewarm/directory each execute1/0/0; the remaining three each complete
one Python native scenario. Final totals are **5 Rust native tests +3 Python
native scenarios**. No new Guest unit run is counted. Unchanged020 full OCI Host
PEP/rootRO/tmpfs/Secret evidence remains historical, bound to the exact unchanged
OCI assets/source, and is not relabelled as a new run.
[CPU](checks/box-native-cpu-time-attempt-1.json), [prior limits](checks/box-native-ulimits-attempt-1.json),
[A–F results/counters](checks/af-final1/native-results.json),
[unchanged020 evidence](../20261115-oci-cpu-time/OUTCOME.md).

Final audit checks10 existing/prepared isolated homes and92 observed socket
paths, including failed native1 and diagnostic1. Executions, volumes, owned
VM/Host/shim/engine processes, workload/runtime disks, mounts, sockets and FD
holders are zero; all7 initial RAM mounts remain unchanged. The audit deletes
nothing. All23 native and283 AF baseline files retain blobs/immutable metadata;
index changes only update last_used/owned paths plus one exact owned AF build
bound to the current CLI and actual result digest.
[Cleanup](checks/final-cleanup.json), [FDs](checks/final-fd-holders.json),
[runtime disks](checks/final-runtime-disks.json), [cache](checks/final-cache-audit.json).

## Explicitly not claimed

These standard limits are not Host RSS, aggregate execution memory, a disk quota
or revocation of explicitly granted Linux privileges. Tested effects use
cap_drop ALL. Zero limits can intentionally prevent startup/allocation; no
infinity fallback is introduced. Runtime RSS is not measured (a diagnostic cat
status snapshot is not an overall runtime measurement). Full OCI Host Service/
default cutover, full W4, live continuity, fresh-host/conformance/security/soak
and platform/readiness promotion remain open. macOS default remains BoxVm and
HVF owner death stopped-only. Windows ARM64 dependencies remain unsupported
without native validation. No VMM/firmware port, real ~/.kbox mutation, real
Secrets, unrelated cleanup, commit/push/publication or fabricated historical
missing evidence.

Next bounded local W4 item is nonroot per-UID process-count (nproc) intent through
the same existing executor, including explicit refusal of root/privileged
bypasses. Continue directly; this accepted slice does not end the task.

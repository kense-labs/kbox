# Nice-priority limits: accepted

Bounded spec025 is accepted on this existing Mac arm64 host. Existing Box nice
intent reaches standard OCI RLIMIT_NICE and the existing Guest executor. The
production delta is one enum mapping plus qualification text. Reverse-patching
those two edits reproduces the exact accepted024 source hash. No CLI/OCI/Agent/
native source, schema/capability, dependency/config/ABI/state changes. Public
None and CLI defaults remain root0; explicit nonroot requests carry the same
per-process limit. The existing per-UID counter boundary stays unchanged.
[Exact minimal delta proof](checks/minimal-production-delta.json).

Nice25 permits future priority improvements to-5; it does not set initial nice
or reserve CPU. Kernel [can_nice](https://github.com/gregkh/linux/blob/v6.12.76/kernel/sched/syscalls.c)
and [setpriority](https://github.com/gregkh/linux/blob/v6.12.76/kernel/sys.c) enforce
this process limit or CAP_SYS_NICE. The Mac profile refuses SYS_NICE,
SYS_RESOURCE and privileged combinations; native effects use all capabilities
empty and NNP1. Root0 with empty capabilities is not exempt from this limit.

## Installed assets

Environment: macOS26.6.2(25G83), M5Max arm64, euid501, Rust1.97.1.
Final global source2 is
`7aa7feebab7594c99f4d9866d23050e164bf6e6551ed4c7378f5ecfd19c22661`,1700 inputs.
All original uncommitted work and failed artifacts remain.
[Environment](checks/host-environment.json), [paths/context](run-context.json),
[final source/effect/count review](checks/final-review2.json).

| Installed archive | SHA-256 | Compressed bytes | Installed bytes |
| --- | --- | ---: | ---: |
| Box2 kbox-3.4.3-macos-arm64.tar.gz | 15baefe0145e030046f16b1cdb4c2af72434e5b3bd581488b620f1fe2db76dfd | 23,393,022 | 44,171,264 |
| Unchanged OCI022, fresh extraction | 37009193a985f12eddced69d1ee5214650ae2e7e1d449ba5787802195bb381e4 | 33,421,132 | 114,950,144 |

OCI retains original source
`4f9dbe71f19a9d018e40b0a58e5a62e2dd2cde5d07c9698f7e710c9a8f2a648b`.
All11 companion hashes per installation, actual arm64 Mach-O/static arm64 ELF,
strict ad-hoc signatures, native links and embedded/standalone Agent match.
Every actual VM worker/shim/native library is bound to these installed prefixes.
Box1 remains retained; every runtime payload is identical to2 except build-info,
which honestly records each source freeze. Source1→2 only fixes a native fixture
prerequisite. Native repos remain clean at libkrun3c2889c7… /libkrunfwfc3ea068…;
Cargo.lock0f82d0ea… . [Box2 install](checks/install-nice-final2.json),
[OCI original provenance/fresh install](checks/reused-oci-install1.json),
[all identities/signature/architecture commands](checks/final-identities-2.json),
[payload equivalence](checks/production-equivalence1-to2.json),
[submodules](checks/native-submodules.json), [prior archives/SHA](checks/prior-artifacts-retained.json).

## Actual counts and effects

Final matrix: **5 outer Rust native tests +3 Python native scenarios**, all
passed. No new Guest unit run is counted. Nice2 executes1/0/0 in220.74s native/
225.09s wrapper. All15 init/capture/child/stdin-stream/realPTY observations across
generations1/2/3 show actual initial nice0, successful changes to-5, denied-6
EACCES13, correct readback after refusal, lowering/reusing the allowed range and
hard-raise EPERM1. Forked children and pthreads inherit-5, cannot improve beyond
it, close/reap/join correctly and leave the parent's priority unchanged.
[Actual native count](checks/box-native-nice-attempt-2.json).

The entered root user namespace cannot map parent UID0 without CAP_SETFCAP:
actual map EPERM1 is preserved, then priority syscalls still exercise and refuse
-6 inside that entered unmapped namespace. Explicit1001:2002 exec and CLI requests
successfully map their own UID and retain the same-5/EACCES boundary. No capability
is added to make a fixture work. Zero allows startup and lowering priority to19
but refuses improvement to18; a separate infinity execution actually reaches
-20. Distinct source inodes preserve the caller-file alias boundary. CLI exec,
healthy health, unchanged root defaults, explicit nonroot, realTTY/attach/restart,
replay/one effect/conflict/stale generation, normal restart and stopped Host
recovery all pass. Default nofile1024 remains.
[C2 source/ELF/CLI driver hashes](checks/probe-identities2.json).

Current installed2 mixed Host Service/recovery/foreign-rootfs/capability/RO/
replay/fault and single-file Host PEP each execute1/0/0. A–F all5 pass in146.45s:
2Rust(prewarm/directory)+3Python(OCI lifecycle/build/export/commit, actual Host RO
attacks/restart and process flow). [Host](checks/box-native-host-service-attempt-2.json),
[files](checks/box-native-files-attempt-2.json), [A–F counts/results](checks/af-final1/native-results.json).
Other resource/Secret/Guest checks retain their actual [accepted024](../20261115-oci-msgqueue/OUTCOME.md)
identity and counts, not new025 executions: only the new Nice mapping changes
production; all their code/OCI/Agent/assets remain unchanged. The source proof
above and owning baseline verify that bounded delta; unchanged tests are not
gratuitously repeated or relabelled.

Focused before1 executes0passed/2failed/0ignored; after1 executes18passed/0failed/
1ignored. Exact zero/infinity/u64/enum/soft/hard/defaults/idempotence, malformed/
duplicate/inverted/overflow, capability/privileged and other-profile refusals
are covered. Final scripts/verify-commit.sh exits0 in200.12s: Runtime2257passed/
41ignored, CLI935, installer8; all-target workspace check, fmt and scoped owning
Clippy pass. Verify1's earlier227.50s pass remains separate historical evidence.
[Before](checks/nice-before1.json), [focused](checks/nice-after1.json),
[final verify](checks/verify2.json), [actual log](checks/verify2.log).

## Failure, commands and cleanup

Native1 exits before readiness. The installed CLI diagnostic independently
confirms actual25:25, UID/GID0:0, empty capabilities/NNP and pinpoints uid_map
EPERM. The fixture incorrectly assumed root mapping would succeed without
CAP_SETFCAP. Pinned [verify_root_map](https://github.com/gregkh/linux/blob/v6.12.76/kernel/user_namespace.c)
requires it. C2/Rust/CLI fixtures now assert the real root-map refusal, continue
all priority-effect assertions in the entered namespace, and independently
require successful nonroot mapping. No Nice/child/thread/permission assertion
is removed, skipped or weakened; no product capability changes. Root/nonroot
observations both pass. C1/source/ELF/failed native/diagnostic/candidate1 remain.
Source2/client2/Box2 are regenerated after that fixture-source edit and actually
retested; all production payloads remain identical.
[Failure ledger](FAILURES.md), [installed diagnostic](checks/nice-diagnostic1-result.json),
[kernel root-map proof](checks/root-map-kernel-proof.json).

Artifact root: artifacts/oci-nice-581649d8a124446983e5432838103584.
Actual commands/args/counters/stdout/stderr/loader and FD observations are in
checks JSON/logs. Executed drivers include build-box-final2.py,
check-final-identities2.py, run-native.py, qualification-runner-final1.py,
diagnose-nice1.py, final-resource-audit.py and final-review2.py.

Final audit checks10 homes including failure/diagnostic and95 observed socket
paths including private state. Executions, volumes, owned VM/shim/Host/engine
processes, mounts, workload/runtime disks, sockets and FD holders are zero. All7
initial RAM devices and the unrelated mounted image remain untouched; no new
RAM device is used. Audit deletes nothing. All23 native and353 A–F baseline
cache files preserve blobs/owner/mode; index changes only update last_used/owned
paths plus one exact owned A–F build bound to its result digest/current CLI hash.
[Cleanup](checks/final-cleanup.json), [FDs](checks/final-fd-holders.json),
[disks](checks/final-runtime-disks.json), [cache](checks/final-cache-audit.json).

## Explicitly not claimed

This controls future nice improvements, not initial nice, CPU reservation,
fairness/throughput guarantees or real-time scheduling. Runtime RSS is unmeasured.
Complete OCI Host Service/default cutover/full W4/live continuity, fresh-host/
other-platform native matrices, conformance/security/soak and platform/readiness
promotion remain open. macOS default stays BoxVm; HVF owner death stopped-only.
Windows ARM64 dependencies remain unsupported without native validation. No
VMM/firmware port, real ~/.kbox mutation, real Secret, unrelated cleanup,
commit/push/publication or fabricated missing historical evidence.

Continue existing real-time scheduling controls (rtprio/rttime). This accepted
slice does not end the continuous task.

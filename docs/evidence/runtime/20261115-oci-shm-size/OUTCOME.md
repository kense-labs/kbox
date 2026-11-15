# Shared-memory size: accepted before integration merge

Spec030 reuses the existing CLI → policy.shm_size → managed record →
VmLocalExecutionBackend /dev/shm:size=N → OCI MountPlan path. Mac preflight now
validates the generated entry before claims. The exact /dev/shm target admits
size-only, page-aligned4KiB..512MiB, with rw,nosuid,nodev,noexec,mode1777 fixed.
Bare targets, extra flags/owners, invalid sizes, overlapping mounts and combined
policy/explicit declarations refuse. Other profiles retain their qualification
guard. Omitted shm_size retains the Guest's existing synthesized mount.

No second state/configuration, dependency, wire contract or native source change.
ExecutionManager, immutable request, routes, generation fencing, cap_drop ALL,
and029 capability compiler/init/exec application remain unchanged. OCI/SDK/Agent
sources are byte-identical to029. [Scope proof](checks/source-scope1.json).

## Frozen source and installed assets

macOS26.6.2(25G83), M5 Max arm64, euid501, Rust1.97.1.
[Environment](checks/host-environment.json), [paths/context](run-context.json).
Source1 `c7e0c86543bd475639bc5b56261d34fac6d115f0deed5f0c11fdb02683a8d1dd`,1709 inputs;
client1 `e040d1e253b9910661a174be61f0f92666ba4398c649b186d5333d035dbe9f2b`.
Root: `artifacts/oci-shm-size-c4f98908041047b2b2af040555b77d89`;
private: `/private/tmp/kbox-shm-size-c4f989080410`.

| Archive under artifact root | SHA256 | Compressed / installed bytes |
| --- | --- | --- |
| `final-dist-shm-size-1/kbox-3.4.3-macos-arm64.tar.gz` | `ad8d6f67d06e1e949ed94659606a7ab4bdf8a57d961e768b389075e081c264b3` | 23,396,001 / 44,171,264 |
| `oci-reused-dist/koci-hvf-0.3.6-macos-arm64.tar.gz` | `30eb5e29bbe0e3676a55a6c67f95b20d2752e5aacbf791a2cd70222e9f4f8d62` | 33,424,560 / 114,954,240 |

Box was freshly built/installed; unchanged OCI freshly extracted. Each has11
verified companions. Actual architectures, ad-hoc signatures, dylib links,
source manifests and standalone/embedded Agent equality pass.
OCI retains original source
`c07659de97c45cd962d08dd29f49e882ac9266bed570226b116af767bbc1734c`;
Agent `a7159a45271c2843901f8cc958bbaf83355ed8ebac5b5bf24e921e79194fd566`.
[Box install](checks/install-shm-size-final1.json),
[OCI provenance](checks/reused-oci-install1.json),
[identities](checks/final-identities-1.json),
[probe/client/driver hashes](checks/probe-client-driver-identities1.json).
The two native bundles remain distinct. libkrun gitlink
`3c2889c76460d81e3fb16d6144580fd31eb1bae3` and libkrunfw
`fc3ea06895301f1df6a274dad75fbd9317b3bfee` stay clean. OCI native archive
`d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`
and Cargo.lock `0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb`
are unchanged. [Retained prior assets](checks/prior-artifacts-retained.json).

## Actual tests and effects

Admission red1 executes2passed/2failed/0ignored. After implementation,
production34,focused5,persisted caller1 and completion18 pass.
`scripts/verify-commit.sh` exits0 in233.77s: Runtime2274passed/46ignored,
CLI935,installer8,workspace all-target/fmt/scoped Clippy/contracts.
[Red](checks/shm-admission-before1.json), [production](checks/shm-admission-after1.json),
[caller](checks/shm-caller1.json), [completion](checks/box-completion1.json),
[default gate](checks/verify1.json).

Final matrix: **4 outer Rust native tests +3 Python native scenarios**, all pass.
No Guest unit suite is counted. Shared-memory1 executes1/0/0 in101.78s driver /
103.00s wrapper. Its25 recorded observations cover init,capture,child,stdin stream
and realPTY across three generations, a separate2MiB execution and an explicit
size-only tmpfs control. Kernel statfs/statvfs proves exact1/2MiB capacities and
1777/rw/nosuid/nodev/noexec. Capabilities remain empty and NNP1.
[Actual counter](checks/box-native-shm-size-attempt-1.json).

Each full probe uses actual shm_open,ftruncate,MAP_SHARED,fork and child
read/write visibility. shm_unlink removes the name while the retained FD/mapping
remains usable. Real writes reach ENOSPC28, unlink releases capacity and a synced
new page succeeds. Actual execve on a0755 script fails EACCES13. Child/thread
probes repeat effects and are reaped/joined. UID1002 cannot delete UID1001's0600
file in the sticky mount but can create/remove its own file. A separate tmpfs
has another device identity; the readonly root still permits shared-memory writes.

The4KiB and512MiB boundary controls inspect real capacity/flags without filling
the maximum. Omitted size actually yields252,522,496bytes on this kernel;
this observed filesystem capacity is neither a fixed product default nor RSS.
Independent instances do not share data. CLI --shm-size, persisted OciSdk,
capture and two realPTY/attach cases pass. Replay/changed requests and stale
generations retain existing guards. Normal restart and authenticated
Host-SIGKILL/stopped recovery advance1/2/3, retain identity/route and recreate
empty generation-local shared memory.

Affected existing tmpfs matrix executes1/0/0 in165.90s driver/167.49s wrapper,
retaining actual EROFS/ENOSPC/noexec, ownership, storage,CLI and recovery behavior.
A–F all5 pass in175.70s: two Rust1/0/0 (prewarm,directory) and three Python native
scenarios (OCI lifecycle,Host RO/restart,process flow). Native library loader
paths/hashes and scenario result files are checked.
[Tmpfs](checks/box-native-tmpfs-attempt-1.json),
[A–F](checks/af-final1/native-results.json),
[strict final review](checks/final-review1.json).

## Retention and cleanup

[FAILURES.md](FAILURES.md) retains the actual initial refusal; no native,
build or acceptance failure was hidden. All earlier candidates and failures stay.
Audit checks7 actual homes: zero executions,volumes,owned processes,mounts,
workload disks or FD holders;86 observed socket paths are absent. Initial7 RAM
devices and unrelated mounts remain. All35 native/451 A–F baseline files retain
bytes/UID/GID/mode, with validated index updates and one exact owned A–F image.
All7 probe-source files retain pre-grant bytes/hash,UID/GID/mode,device/inode,
size and single-link identity. Audit deletes nothing.
[Cleanup](checks/final-cleanup.json), [FDs](checks/final-fd-holders.json),
[cache](checks/final-cache-audit.json), [sources](checks/final-caller-sources.json).

## Explicitly not claimed

Logical tmpfs capacity is not reserved Host RAM, measured RSS, live resize or
immunity from an explicitly granted Guest SYS_ADMIN remount. Full W4/default
OCI Host Service cutover/live continuity/fresh-host/conformance/soak/platform
promotion remain open. Default macOS stays BoxVm; HVF recovery stays stopped-only.
Windows ARM64 dependencies remain unsupported/no native run. No real ~/.kbox,
real Secret, native-submodule edit or unrelated cleanup. The user's later
instruction authorizes convergence, commit, merge and push; no new W4 feature
stage is started. Remote main advanced to e4a6809, so integration checks and
affected rebuilt assets require their own subsequent record; these pre-merge
results are not relabeled as validation of merged source.

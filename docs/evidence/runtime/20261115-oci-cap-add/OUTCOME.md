# Existing portable capability additions: accepted

Spec029 removes the Mac adapter's blanket refusal of the existing shared
13-name cap_add allowlist. It reuses the closed compiler and its normalization,
default10 and drop precedence. The separately qualified explicit SYS_ADMIN
branch remains. Unknown capabilities refuse before resource claims; cap_drop ALL
still clears every set, including explicit additions. Other profiles retain
their qualification limits. No shared allowlist or default privilege expands.

Init and exec now share apply_workload_capabilities. For the actual resolved
nonroot OCI UID, only explicitly added, still-permitted names enter inherited
and ambient sets, so Linux exec retains the caller's additions. Default and
dropped names never enter ambient. Root inherited/ambient stay empty. The
existing Agent credential/capability executor is unchanged; nonroot per-UID
nproc/sigpending/msgqueue still require empty capabilities and no_new_privileges.

## Final assets and actual checks

Existing host: macOS26.6.2(25G83), M5 Max arm64, euid501, Rust1.97.1.
[Environment](checks/host-environment.json), [context](run-context.json).
Source3 SHA256
`b34ab4b90e5b334d12ae08625a3fa60024ce9b455084830863fee34c07a6ab36`,1707 inputs;
client3 `d85971b7dcd293e437f9775504f092544b9f1273a9ecba65d316c7f0455052ac`.
Artifact root: `artifacts/oci-cap-add-05c0ae87abb246feb912f52b32af137a`.
Private root: `/private/tmp/kbox-cap-add-05c0ae87abb2`.

| Final archive under artifact root | SHA256 | Compressed / installed bytes |
| --- | --- | --- |
| `final-dist-cap-add-3/kbox-3.4.3-macos-arm64.tar.gz` | `b1964bfc6ca149932f870ae951e6aceca67a99486a3e2fd5a4d0f5bfbddb6cd6` | 23,394,400 / 44,171,264 |
| `oci-reused-dist/koci-hvf-0.3.6-macos-arm64.tar.gz` | `30eb5e29bbe0e3676a55a6c67f95b20d2752e5aacbf791a2cd70222e9f4f8d62` | 33,424,560 / 114,954,240 |

Box3 was freshly built and installed; unchanged OCI was freshly extracted.
Both have11 verified companions. OCI/SDK/Agent source remains identical to028,
retaining original OCI source
`c07659de97c45cd962d08dd29f49e882ac9266bed570226b116af767bbc1734c`.
Standalone/embedded Agent remains
`a7159a45271c2843901f8cc958bbaf83355ed8ebac5b5bf24e921e79194fd566`.
CLI/shim/guest-init/Host/Agent/system image, architecture, native links, ad-hoc
signatures and build manifests pass. [Box install](checks/install-cap-add-final3.json),
[OCI install/provenance](checks/reused-oci-install1.json),
[actual identities](checks/final-identities-3.json).

Native gitlinks remain clean: libkrun
`3c2889c76460d81e3fb16d6144580fd31eb1bae3`, libkrunfw
`fc3ea06895301f1df6a274dad75fbd9317b3bfee`.
OCI native archive stays
`d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`;
Box and OCI retain their distinct native bundles. Cargo.lock remains
`0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb`.
[Previous archives/checksums](checks/prior-artifacts-retained.json),
[probe/client/driver hashes](checks/probe-client-driver-identities3.json).

Admission red1 actually failed4/4; final focused capability4, completion17,
production30 and backend79 pass. Nonroot completion red1 failed1/1 before the
ambient fix. `scripts/verify-commit.sh` exits0 in224.75s: Runtime2269passed,
45ignored,CLI935,installer8, workspace all-target/fmt/scoped Clippy/contracts.
Ignored tests are not native passes. [Focused](checks/cap-add-after2.json),
[completion](checks/box-completion3.json), [backend](checks/box-backend3.json),
[final default gate](checks/verify3.json).

Final installed matrix: **3 outer Rust native tests +3 Python native scenarios**,
all pass. No new Guest unit suite is counted. Capability native3 actually
executes1/0/0 in165.44s driver/166.63s wrapper, using installed3/client3 and the
fresh OCI assets. [Command/count](checks/box-native-cap-add-attempt-3.json).
Its45 recorded observations cover init,capture,child,stdin stream and realPTY
across three root generations and six independent root/nonroot controls.
Each probe also forks/reaps a child and creates/joins a thread.

Actual kernel capget/bounding/ambient readback matches exact sets. MKNOD creates
the requested Guest char node and confirms its owner/type/device; nodev still
denies opening it. SETFCAP writes/reads/removes an actual security.capability
xattr. Specific-drop and ALL controls return EPERM without leaving those
objects; normal file writes succeed. UID1001 retains only explicit additions
after exec, and dropping those additions leaves no effective permissions despite
the default bounding set. Explicit nonroot SYS_ADMIN performs and removes a
real tmpfs mount; the corresponding no-SYS_ADMIN controls get EPERM.
AUDIT_WRITE is established by exact kernel bit readback, not a full audit service.

The existing nonroot NPROC16 control retains its hard limit and empty sets.
Combined root SYS_ADMIN plus additions still permits Guest remount/fresh mounts
while the ordinary-file Host PEP records16 denials, child refusal, isolated
parent views and live RW writeback in each of two generations. CLI cap-add flags,
persisted OciSdk route, captured nonroot exec and two realPTY/attach cases pass.
Replay produces one actual side effect; changed requests and stale generations
refuse. Normal restart and authenticated Host-SIGKILL/stopped recovery advance
generations1/2/3 while retaining route/identity. Ephemeral root effects reset.

A–F3 all5 pass in155.11s: OCI lifecycle,Host RO/restart,process flow,prewarm and
directory grants. Prewarm/directory each execute1/0/0; the other three are actual
Python native scenarios. Actual loaded library paths/hashes are retained.
[A–F counters/results](checks/af-final3/native-results.json),
[strict asset/source/effect/counter review](checks/final-review4.json).

## Failures and cleanup

[FAILURES.md](FAILURES.md) preserves admission and nonroot red tests, a mistaken
drop-test polarity, actual native2 exec failure, archive/identity filename errors
and inventory preparation assumptions. Native2 exposed a real omitted caller:
init applied ambient values while exec still used the root compiler. Both now
use the same implementation. No assertion, quota, timeout or security gate was
weakened. Old candidates, native failures and their logs remain. Candidate2's
identity JSON was overwritten by a driver filename error; its corrected JSON is
explicitly a later read-only revalidation, not the original byte-for-byte file.
Its archives/source/native logs and receipts were unaffected.

Audit checks all18 actual execution homes from three attempts and three A–F
runs, including failed native2. Zero executions,volumes,owned processes,mounts,
workload disks or FD holders remain; all235 observed socket paths are absent.
Initial7 RAM devices and unrelated mounts remain. All35 native and409 A–F
baseline cache files retain bytes/UID/GID/mode, with validated index metadata
updates and exactly three owned A–F images. No audit cleanup deletes resources.
All21 probe-source copies retain exact pre-grant UID/GID/mode/device/inode/size,
bytes/hash and single-link identity. [Cleanup](checks/final-cleanup.json),
[FDs](checks/final-fd-holders.json), [cache](checks/final-cache-audit.json),
[pre-grant sources](checks/final-caller-sources.json).

## Explicitly not claimed

This qualifies the existing portable allowlist and explicit additions on this
Mac profile, not arbitrary Guest capabilities, Host devices, audit subsystem
availability or complete Box parity. Full W4/default OCI Host Service cutover,
live continuity,fresh-host/conformance/soak and platform promotion remain open.
Default macOS stays BoxVm and HVF recovery remains stopped-only. Windows ARM64
dependencies remain unsupported with no native run; runtime RSS is unmeasured.
No native-submodule edits, real ~/.kbox, real secrets, unrelated cleanup or
commit/push/publication. Historical missing evidence remains missing.

Continue the existing W4 shared-memory policy gap: policy.shm_size is already
exposed by CLI and stored, but the Mac OCI profile still refuses it. Trace and
reuse the existing Guest /dev/shm mount executor; qualify actual capacity and
failure effects without adding a configuration language or changing Host memory
claims. This bounded acceptance does not finish the continuous task.

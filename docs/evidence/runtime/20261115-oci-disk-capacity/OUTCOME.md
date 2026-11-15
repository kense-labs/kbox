# Custom disk capacity: accepted

Spec028 carries existing Box `resources.disk_mb` through the explicit persisted
macOS OciSdk profile. The Mac admission reuses the existing ext4 constructor's
16 MiB..64 GiB range before resource planning/claims. The old default-size guard
remains on other profiles. Only these two production lines change; the reverse
patch matches027 exactly. No new option, dependency, schema, state or resize API.
[Production proof](checks/minimal-production-delta.json).

The existing builder already passes disk_mb to rootfs and image-VOLUME anonymous
disks; rootfs-image/raw-storage descriptors, pinned Host files, Guest serial/size
verification and VolumeStore ownership are reused. Default4096MiB, onevCPU/512MiB,
all other unsupported controls, identity/route/generation and cap_drop ALL remain.
Invalid capacities refuse before claims. A valid but too-small disk fails image
preparation and cleans its own partial state without entering a VM.

## Assets, source and environment

macOS26.6.2(25G83), M5 Max arm64, euid501, Rust1.97.1.
[Environment](checks/host-environment.json), [paths/context](run-context.json).
Source4 SHA256:
`a3d30ebc663f422439f44fa5dd14e46fa89db23cee84b7ba6d81eb5332cb919b`,1705 inputs.
Client4 SHA256:
`30cd1883d4d18961d8db133c0d406b9e37529f6d7e09705348f8b750bad0adbc`.

Artifact root: `artifacts/oci-disk-capacity-64a2d8b501a94159b8bc63c58861bb0a`.
Private root: `/private/tmp/kbox-disk-capacity-64a2d8b501a9`.

| Final archive under artifact root | SHA256 | Compressed / installed bytes |
| --- | --- | --- |
| `final-dist-disk-capacity-4/kbox-3.4.3-macos-arm64.tar.gz` | `39e6b5b34b6a84455669ef8d5598062aac7444dc866a18e122b4de4a01b92c82` | 23,393,934 / 44,171,264 |
| `oci-reused-dist/koci-hvf-0.3.6-macos-arm64.tar.gz` | `30eb5e29bbe0e3676a55a6c67f95b20d2752e5aacbf791a2cd70222e9f4f8d62` | 33,424,560 / 114,954,240 |

Box4 was built and installed to a new isolated prefix; unchanged OCI027 was
copied and freshly extracted. Each has11 checked companions. OCI/SDK/Agent code
is unchanged and retains original source
`c07659de97c45cd962d08dd29f49e882ac9266bed570226b116af767bbc1734c`.
Standalone/embedded Agent remains
`a7159a45271c2843901f8cc958bbaf83355ed8ebac5b5bf24e921e79194fd566`.
CLI, shim, guest-init, Host, Agent, system image, architecture, ad-hoc signatures,
library links and source manifests are verified.
[Box install](checks/install-disk-capacity-final4.json),
[OCI provenance](checks/reused-oci-install1.json),
[identities](checks/final-identities-4.json).

Native sources remain clean: libkrun
`3c2889c76460d81e3fb16d6144580fd31eb1bae3`, libkrunfw
`fc3ea06895301f1df6a274dad75fbd9317b3bfee`.
OCI native archive stays
`d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`;
lib `7dcaf94032e22d2044aed36b7c43f17dd311fed65369f0f292c2e65337856ddf`,
firmware `841bc9d5eecbc2aeeb6098fbc75d484427680d7503f5ed9bcdfe9d072a9420d4`,
kernel `b1180b50148ed14f5fbeadf17288ce8abcf245daa468255b7ff41113bbf01199`.
Box's separate libraries are retained. Cargo.lock remains
`0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb`.
Previous archives/checksums and all four candidates remain.
[Prior archives](checks/prior-artifacts-retained.json).

## Actual checks and installed native results

Admission red1:2passed/1failed/0ignored; after1:3/0/0. Owning production26,
rootfs152 and backend79 pass. An initial backend filter executed zero tests,
was rejected as verification, and was corrected using the frozen binary's actual
test list. Final `scripts/verify-commit.sh` exits0 in248.80s:
Runtime2264passed/44ignored,CLI935,installer8,workspace all-target check/fmt/scoped
Clippy/contracts. [Admission](checks/disk-admission-after1.json),
[production](checks/box-production1.json), [rootfs](checks/box-rootfs1.json),
[backend](checks/box-backend2.json), [final verify4](checks/verify4.json).

Final matrix: **6 outer Rust native tests +3 Python native scenarios**, all pass.
No new Guest unit suite is counted. Native capacity4 executes1/0/0 in154.06s
native/155.40s wrapper, using installed4/client4 and the freshly extracted OCI.
[Actual counter](checks/box-native-disk-capacity-attempt-4.json).

It proves actual128MiB root and three anonymous disks, with private600 Host
files, correct owner/allocation identity, numeric Guest ownership/mode/links,
and three generations. Each generation has8 recorded observations: init,
capture,child,stdin stream,realPTY and all three anonymous mounts. Guest statfs
and sysfs identify four distinct block devices with exact134,217,728-byte
capacity; actual filesystem capacity accounts for ext4 metadata. Root and one
anonymous volume are independently written until ENOSPC28, unlinked and reused
with a successful synced1MiB write. No cap is added for these effects.
[24 observations plus2 full-disk probes are checked against their result files](checks/final-review4.json).

An independent256MiB instance has separate root/allocation/inode identities;
actual installed CLI inspect/exec/restart with migration policy off retains its
persisted OciSdk route. Root and anonymous Guest capacities remain256MiB before
and after CLI restart. The8GiB control has an actual8,589,934,592-byte Host/Guest
device,8,388,009,984-byte filesystem and only10,076,160 allocated Host bytes at
the recorded observation. These are disk measurements, not process memory.
The20MiB synthetic COPY-only image cannot fit a16MiB disk and fails before VM
entry; invalid0/15/65537/u32::MAX requests leave no claimed executions/volumes.
[Fixture build](checks/oversized-image-build1.json),
[probe/client source and ELF hashes](checks/probe-client-identities4.json).

The existing [spec008 lifetime contract](../../../../specs/008-oci-rootfs/contracts/rootfs-image.md)
remains explicit: root writes work in their current generation, and restart
rebuilds the ephemeral root from the image. Anonymous data/owner/links/capacity
persist across normal restart and authenticated Host-SIGKILL/stopped recovery;
root markers disappear. Replay preserves one identity; changed requests/stale
generations refuse. Transport blockId follows the new attachment digest;
allocation/source/mount/access/ownership/cleanup fields remain exactly equal.
The test asserts both changed block IDs and unchanged remaining fields. Kill
retains owned raw data; remove deletes only the recorded allocations.

The affected anonymous,Host Service and ordinary-file matrices each execute1/0/0
from installed2/client2 (471.22s,418.41s,47.73s wrappers). A–F all5 pass in170.85s:
2Rust(prewarm/directory)+3Python(OCI lifecycle,Host RO/restart,process streams).
These retain their actual installed2/source2 identities. Only the unused
capacity-test sibling changed afterward; every product runtime payload2→3→4
is byte-identical, with only build-info provenance regenerated. They are not
relabelled as installed4 executions or needlessly rerun.
[Anonymous](checks/box-native-anonymous-attempt-2.json),
[Host](checks/box-native-host-service-attempt-2.json),
[files](checks/box-native-files-attempt-2.json),
[A–F](checks/af-final2/native-results.json),
[source-only change](checks/fixture-only-source3-to4.json),
[payload2→3](checks/production-equivalence2-to3.json),
[payload3→4](checks/production-equivalence3-to4.json).

## Retained failures and resource cleanup

[FAILURES.md](FAILURES.md) records all failures and corrections: real admission
gap, optimized C assertions disabled by NDEBUG (strict compiler caught it),
missing stream enum arm, zero-test filter, missing A–F output directory, root
DAC/persistence/blockId test assumptions, audit GID and OsString format errors.
No product capability, timeout, upload limit, assertion or acceptance gate was
weakened to pass. The corrected native checks follow actual existing contracts.
All candidates, failed native roots, logs, old passes and diagnostic data remain.

Audit4 checks13 actual K_HOME inventories including failed attempts and the
COPY-only build home. Zero executions/volumes/owned processes/mounts/disks/FDs;
107 observed socket paths absent. Initial7 RAM devices and unrelated mounted
image remain unchanged. No new native RAM device was required. All23 native
and395 A–F baseline cache files retain bytes/UID/GID/mode; validated index updates,
one exact owned synthetic image and one exact A–F image are the only additions.
[Cleanup](checks/final-cleanup.json), [FDs](checks/final-fd-holders.json),
[cache](checks/final-cache-audit.json), [owned fixture](checks/final-owned-fixture-build.json).

Nine synthetic caller-probe copies retain their pre-entry file IDs and bytes;
UID/mode match fs::copy and GID matches native creation in each owned parent.
Four standalone native Rust copy controls prove the parent-group inheritance and
remove only their own new control files. The artifact ELF has GID20; these
private/tmp parents/copies have GID0. Per-copy pre-admission GID snapshots were
not captured and are not invented. [Sources](checks/final-caller-sources.json),
[native copy controls](checks/copy-group-control1.json),
[strict final asset/source/count/effect review](checks/final-review4.json).

Drivers under the artifact root: build-box-final4.py,check-final-identities4.py,
build-oversized-image1.py,run-native.py,qualification-runner-final1.py,
final-resource-audit4.py,final-review4.py. Actual commands, environments,
counts/exit codes, loader maps and logs remain under checks.

## Explicitly not claimed

disk_mb is logical filesystem capacity, not writable-layer quota, guaranteed
free bytes, live resizing or durable root writes. Existing CLI run has no new
disk-size option; this slice validates its operations on existing typed custom
requests. Other platforms/custom CPU or VM memory/lifetime/ephemeral quotas,
full W4/default OCI Host Service cutover/live continuity/fresh-host/conformance/
soak and platform promotion remain open. Default macOS remains BoxVm; recovery
remains stopped-only. Windows ARM64 dependencies unsupported/no native run;
runtime RSS unmeasured. No native-submodule edit, real ~/.kbox mutation, real
Secret, unrelated cleanup, commit/push/publication. Missing historical evidence
remains missing.

Next existing local W4 slice: the Mac Box cap_add guard rejects even the shared
portable allowlist. Trace and qualify those existing guest permissions through
the same compiler/executor; preserve cap_drop ALL, unknown-cap refusal, per-UID
limit boundaries and Host PEP. This acceptance does not finish the continuous task.

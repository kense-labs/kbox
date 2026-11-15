# Spec012 image-declared anonymous volumes

Status: accepted bounded anonymous-volume slice on this existing macOS arm64
host, with final installed archives, actual CLI/native matrices and owned cleanup.
Full OCI Host Service/default cutover, W4 and enterprise GA remain open.

## Implemented behavior

Box ExecutionManager persists the existing deterministic image VOLUME plan before
materialization, then claims allocations through the existing VolumeStore. A
private mount-free projection of the existing layered Guest tree seeds each
allocation's `ext4-v1` artifact. Numeric UID/GID, modes, hardlinks, symlinks,
xattrs and whiteouts stay Guest-native; admission never performs Host foreign
chown. Existing disks are validated by allocation UUID/capacity and reused,
never reseeded. Caller/named mounts override the same image destination.

Existing SDK storage-v2 entries remain Caller/DetachOnly: Box owns allocation,
retention and deletion. Required `dev.kbox.raw-storage-images` v1 with exact token
`raw-ext4-v1` and explicit Agent attachment manifest v7 combines raw disks with
workload rootfs/directories/files. Previous v1–v6 meanings and restrictions stay.
Mac Host/shim reuse raw verification, no-follow private single-link RO/RW pins,
OS writer/shared-reader locks, native block attachment and exact-file Seatbelt.
Object/allocation/mount/serial collisions, aliases, access or inode/size drift,
unsupported capacities and missing capabilities refuse. Inventory is bounded
at128 including workspace/rootfs. No new store/state machine/config/dependency.

Normal stop retains volumes; canonical remove deletes only its owned allocations.
Ephemeral cleanup holds the existing Box state lock and checks generation,
lifecycle, route, creation identity, OCI binding and exact names before deletion;
explicit restart preserves data. Mac OciSdk anonymous records reuse spec010's
atomic directory fence so the measured unaware old CLI cannot reinterpret them.
Other legacy no-file/BoxVm roots retain the flat format. Arbitrary rollback,
manual flattening and newer-to-older deployment remain unqualified.

## Failures retained and corrected

- Planner before3 ran0/1: no image anonymous plan. Shared planner now reserves
  normalized/deduplicated names, honors explicit overrides and changes no catalog.
  Earlier fixture compile/route errors are retained too.
- Clean ext4 UUID and artifact mismatch regressions each ran0/1. Shared resume
  validator now checks clean UUID too; strict open still refuses dirty journals,
  explicit resume allows existing Guest journal recovery. ext4 focused18passed;
  subtree builder10passed with foreign metadata/links/xattrs and escape refusals.
- Raw transport before0/1 treated a raw image as a directory. The existing KVM
  verifier was moved and reused, not reimplemented; original KVM caller retained.
- Two normal catalog callers quarantined corrupt authority into empty state
  (before0/2). Both now use strict reads and preserve exact bad bytes. State fence
  before0/1 is fixed with the existing layout; compatibility11passed.
- Anonymous focused3 ran32/0/0. Atomic publication, reuse, wrong owner, nonce
  change, busy catalog and retained legacy data tests pass. Cleanup3passed.
- Raw sandbox attempts1/2 accidentally selected0tests and are not evidence.
  Corrected native child proves12actualHostEPERM mutations, thread/execEPERM,
  retained RO FD EBADF and successful exact RW data writes. Full pin fixture1
  wrongly assumed sorted attachment position; exact-ID lookup fixed it, with
  inventory/replace tests passing. Assertions were retained.
- RawRO2 failed before VM because entry rescanned `/dev/fd` after confinement.
  Scan now occurs at open and again in production confine before policy install;
  entry keeps pinned hash/inode/size checks without expanding sandbox access.
  Its pending Creating record was retired by exact SDK Create replay followed by
  forced Delete without Start (cleanup1 actual1/0/0). All inputs/failure receipts
  survive. Post-fix focused13/0/1 plus explicitly launched child1/0/0 passed.
- OldHost1 fixture requested an unqualified custom disk size. It now uses the
  unchanged4GiB policy, with no profile limit relaxed. RawRO1 used the v1 handoff
  helper for v2; the caller now uses the existing v2 helper.
- RawRO4 lacked `/sbin` in its test PATH. RawRO5/6 incorrectly assumed BLKROSET
  clears the hardware feature; actual BLKROGET remains1. RawRO7 expected EROFS
  on a raw device, but actual raw write is EPERM. Correct prerequisites require
  exact Guest device254:32/sysfs readback, retained hardware RO, zero copied
  bytes and explicit EPERM; remount, valid-device fresh mount and child writes
  refuse. See [primary-source explanation](checks/raw-ro-probe-assumption.md).
  These Guest errors alone are not counted as Host enforcement.
- FinalRawRO9's first FD sample preceded native open. Its incomplete snapshot and
  failed additional receipt assertion remain. Sampler now retains both initial
  and later actual native snapshots; final10 requires retained/native RO FDs
  all read-only, RW FDs writable. [Sampling record](checks/raw-ro-descriptor-sampling.md).

All failed/candidate assets remain; no skipped failed assertion, native source
edit, workload-specific exception or fallback was used.

## Frozen final assets

HEAD405c8b89f151e9e2879438d9e457dd60bfd18b90 remains dirty. Source-input SHA256
`dada28865aa4e06e0e3462d046f179eaea58d6cad73195af424efbdf1dba1046` covers1661code inputs in
`artifacts/oci-anonymous-dc919588a1694bb6875ff6b1c6e1dacf/source-frozen-1.json`.
The53changed inputs include one moved existing helper; source inventory retained.
Native test client SHA256 is
`bd39f7b6758e7980ebadcaa0f270401dfcf2924af855457fa0ee380fab44ace8`.

| Final archive | SHA256 | Compressed bytes | Installed disk bytes |
| --- | --- | ---: | ---: |
| Box3.4.3 final1 | `6e2e961d5da356beeadd1875fedace6dfc3c91615f10dd30eb7fe61f2431371e` | 23362761 | 44113920 |
| OCI0.3.6 final1 | `df4b882c40f607b56e098b9f971552ad879f8a156fdc35703cc79fd8faf71a2a` | 33345582 | 114847744 |

[Context](run-context.json) gives exact archive/prefix paths. Previous global
archive, accepted11Box/OCI and their original checksum sidecars were copied into
`artifacts/oci-anonymous-dc919588a1694bb6875ff6b1c6e1dacf/previous-dist/`; original accepted archives remain unchanged.
Both final prefixes are fresh isolated installs, verified11file hashes each.
CLI/shims Mach-Oarm64, static guest-init/Agentaarch64, native signatures/dylib
links and ext4-embedded Agent equality were checked in
[final identities](checks/final-identities-1.json). Product workers loaded only
those installed native/Agent/system assets, with per-process loader maps.

OCI native archive remains qualified009
`d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`:
libkrun7dcaf940…, firmware841bc9d5…, kernelb1180b50…; full values/source sidecar
are in final identities and included build-info/native-source.json. BoxVm keeps
its independent baseline libraries. Parent native trees/gitlinks are unchanged.

## Actual commands and results

Existing macOS26.6.2(25G83), M5Max arm64, euid501, Rust1.97.1. Homes and caches
are isolated owned paths; no real ~/.kbox. Exact commands/exit/time/stdout/stderr
are in checks and the retained run scripts under `artifacts/oci-anonymous-dc919588a1694bb6875ff6b1c6e1dacf`.

- `cargo test --locked -p koci-sdk -p koci-agent-protocol -p koci-runtime -p koci-kbox --all-targets`:
  exit0,362.66s; protocol65, Runtime438/2ignored, SDK215 plus public integrations;
  initial Kbox80/4ignored, binary18/3ignored and3integration passed. Post-Sandbox
  change has its focused native rerun and final owning rerun:80lib/4ignored,
  18binary/3ignored and3integration, exit0 in6.65s.
- Owning OCI Clippy all-targets/no-deps exit0. Final fmt checks exit0.
- `bash scripts/verify-commit.sh`: exit0,281.15s; Box2222/24ignored, CLI933,
  all contract suites, workspace all-targets, fmt, scoped Clippy and installer8
  passed. The final ignored Guest attack fixture edits were subsequently compiled
  and actually executed; product source is unchanged since that gate. No ignored
  or0test target is counted as native execution. [Separate summaries](checks/actual-counts-source1.json).
- Final native source client invokes each exact ignored entry with `--ignored
  --exact ... --nocapture`; actual counts and result/cleanup files are checked:

| Installed-asset matrix | Actual counts | Seconds | Checked CLI |
| --- | --- | ---: | --- |
| Anonymous2: metadata, isolation, overrides, normal/stopped recovery, ephemeral cleanup | 1/0/0 | 430.72 | true |
| Full1: rootfs/files/mixed routes, process/FS/resources/faults/Host recovery | 1/0/0 | 379.74 | true |
| Named3: live growth, shared references, stable IDs and recovery | 1/0/0 | 162.16 | true |
| FailedCreate2: exact reconciliation/delete with no added workload Start | 1/0/0 | 40.67 | true guard |
| RawRO10: actual Guest attacks, unchanged Host image and native/retained FD proof | 1/0/0 | 7.93 | public SDK |
| OldHost3: missing capability/unaware CLI refuse before allocation or workload | 1/0/0 | 1.78 | archived old CLI negative |

Anonymous and Full matrices respectively repeat3 and5 exact-file probes, each
requiring16Hostdenials plus child/fresh-mount checks. Named matrix repeats3
HostRO14denial probes. Actual Host syscall proof and actual installed VMM FD
proof are independent of Guest RO flags; [receipt review](checks/final-receipt-review.json)
checks both, identities, archive/source hashes and all six result files.
A–F final1 ran all5scenarios (OCI, readonly/restart, primary process/engine,
prewarm, directory). All passed with installed companions and loader proof;
prewarm/directory explicitly ran1/0/0 each.
[Actual A–F counts/results](checks/af-final1/native-results.json).

## Cleanup and continuing W4

[Final audit](checks/final-cleanup.json) covers23homes:0managed executions/volumes,
0owned processes/mounts/execution disks/temporary builders,117observed socket
paths all absent. Seven prior RAM devices and unrelated mounted image were
preserved, with baseline provenance recorded. Fifteen main-cache blob files and
81preexisting A–F blob files remain byte-identical. Caller raw images/files,
parent permissions and canaries survive; only explicit allocation/fixture owners
remove their own data. Audit itself deleted nothing.
[Descriptor audit](checks/final-descriptor-audit.json) found no owned runtime/test
FD holder. [Source review](checks/final-source-review.json) confirms empty index,
unchanged parent native trees/gitlinks, preserved archives and frozen input hash.

Bounded spec012 is closed. Tracing the queued tmpfs intent exposed an existing
Guest rootfs readonly fail-open; that confirmed safety gap takes priority in
[spec013](../../../../specs/013-rootfs-readonly/tasks.md). Tmpfs stays queued.
No external blocker remains for this slice. The continuous task is still active.

## Explicitly not claimed

Full Host Service/default cutover, complete Box parity, Secrets/other controls,
pools/snapshots/TEE/devices/custom networks, live-session continuity, full OCI
conformance/security review/long soak, fresh-host or other-platform releases,
support/readiness promotion or enterprise GA. Windows ARM64 current dependencies
are unsupported and no native run is claimed. Guest BLKROSET cannot clear this
hardware feature; no Guest-to-Host EIO bypass proof or hostile-Host-admin defense
is claimed. Host enforcement is separately measured. No aggregate disk quota or
runtime RSS measurement; declared Guest memory is not a measured runtime size.
No native submodule source change, staging, commit, push or publication occurred.

# Guest-native OCI workload rootfs: final acceptance

Status: **complete within spec008**, 2026-11-14. No commit, push or publication.
The explicit Box ExecutionManager → persisted OciSdk → public SDK → independent
macOS HVF Host Service path now preserves foreign numeric UID/GID in an actual
Linux workload. Full Host Service cutover remains incomplete.

## Environment and retained state

macOS 26.6.2 (25G83), Apple M5 Max, arm64, host euid501;
rustc1.97.1 / LLVM22.1.6. Native claims name this existing host only.
[Environment](checks/final-host.json), [compiler](checks/initial-compiler.log).
All initial dirty work, source hashes, mount/process/device inventories and old
archive copies/checksums were retained. Native submodule working trees remain
clean; their source was not edited. No real `~/.kbox` was used.

Artifact root: `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647`. Private native root: `/private/tmp/kbox-rootfs-1a469ffdee77`.
[Run context](run-context.json), [bounded tasks](../../../../specs/008-oci-rootfs/tasks.md).

## Implemented contract

Reused verified OCI layer application, the bounded LogicalRootfs namespace,
mkext4 and atomic publication. Numeric owners, modes, links, whiteouts and
supported xattrs remain filesystem metadata. No foreign host chown, sudo,
ownership normalization or Box guest-init injection is involved.

SDK `dev.kbox.rootfs-image` v1 is required automatically for its descriptor.
Agent manifest v5 binds initial disk SHA256/capacity/OCI manifest, exact target,
generation/configuration and the writable file inode. The Host transfers a
private single-link disk outside its exported share. The shim retains a pinned
writable FD and verifies content/identity before attaching the second virtio-blk
device. The Agent verifies serial/size/access before mounting ext4. System image
and workload root remain separate. Existing directory admission and v1–v4
manifests retain their meaning; Exec v1–v6/v7 encoding/replay is unchanged.

Each operation builds a distinct writable disk from immutable OCI blobs; no
writable ext4 cache is shared. Stopped-only recovery preserves the authenticated
record; explicit restart advances generation and rebuilds the ephemeral root.
The existing handoff owns cleanup. Path restrictions, capability ceiling,
ExecutionManager authority, persisted routes and default BoxVm selection remain.
The installed CLI's pristine diff baseline is now derived from the logical tree
and published through the existing first-writer installer before launch.
[Wire/lifetime contract](../../../../specs/008-oci-rootfs/contracts/rootfs-image.md).

## Actual verification

| Check | Actual result / evidence |
| --- | --- |
| Legacy directory foreign-UID rejection | 2 passed; `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/baseline-foreign-uid.log` |
| Workload builder + baseline/layers/metadata/corruption/capacity | 9 passed; `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/cli-baseline-rootfs-2.log` |
| Host rootfs transport/cleanup/limits | 9 passed; `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/hvf-directory-final.log` |
| Shim pin/replacement/permissions (including original cases) | 7 passed; `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/rootfs-shim-pin-2.log` |
| Native sandbox child enforcement | Parent1 passed,1 child entry ignored by top-level harness and invoked by parent; `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/rootfs-sandbox-1.log` |
| Box Runtime / CLI | **2175 / 933 passed**, Runtime14 existing ignored; [final gate](checks/verify-commit-candidate2.log) |
| OCI owning full suites | Agent24, Protocol62, Kbox71 (2 existing ignored), SDK210 passed in `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/oci-full-3.log`; Runtime**430 passed** in `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/oci-runtime-final.log`. Final CLI fix changes only five Box Runtime files; OCI source/dependency/ABI inputs are identical |
| fmt, workspace all-target check, owning Clippy, contract/race/install checks | `scripts/verify-commit.sh` **exit0**, `verify-commit: OK`; Box/CLI follow-up Clippy exit0 in `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/cli-baseline-clippy.log`; OCI owning Clippy exit0 in `/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/rootfs-clippy-final.log` |
| Final installed independent-HVF matrix | **1 passed / 0 failed / 0 ignored**, 256.90s; [command/count/client identity](checks/box-native-attempt-3.json); raw stdout/stderr beside it; `cliChecked=true` in private attempt3/result.json |
| Final installed A–F regressions | **5/5 scenarios PASS**: OCI lifecycle, readonly, primary process flow, prewarm, directory grants; prewarm and directory each executed exactly1 Rust test (1/0/0); [all commands/counts/results](checks/archive-final2/native-results.json) |
| Targeted installed CLI baseline smoke | Actual CLI/OciSdk start + 529 pristine baseline entries + guest stat1001:2002:600 + removal/empty list; `/private/tmp/kbox-rootfs-1a469ffdee77/cli-baseline-check/commands.json` and result.json |

Filtered zero-test output from the exploratory `oci_production` filter is not
counted as verification. Default ignored tests do not become native passes.

Pinned workload image:
`docker.io/library/kbox-hvf-ownership@sha256:a6845bea2508837efa7717113f0715fa3cf86303497c9c39816a59a83d9708fa`.
It contains the preserved Alpine arm64 base and two real numeric-metadata OCI
layers; [builder/identity](/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/ownership-image-identity.json).

Actual guest stat returns root0:0, directory/file1001:2002 with0700/0600,
GID-only0:3003 with0770/0640, and root-only0600. UID1001 writes succeed; other
UID1002 reads/writes are denied; GID3003 reads and directory writes succeed while
file append is denied. Real hardlink/symlink and whiteout/opaque results match.
Two executions have distinct writable identities and no write cross-talk.
Normal stop/restart and authenticated Host-SIGKILL/stopped recovery/restart
preserve ownership semantics and reject old generations/input. Initial image
content is restored for the new ephemeral generation.

Private attempt3 files retain `foreign-ownership.json` (including exact disk
identities), `ownership-after-stopped-recovery.json`, `faults.json`,
`committed-response-loss.json`, `cli.json`, `behavior.json`, `cap-drop-all.json`,
`recovered-sdk.json` and both `readonly-probe-*.json`. Each readonly probe records
14 actual host denials, child denial and writable-remount/fresh-mount attacks;
all five cap_drop ALL kernel masks are zero. Actual library mappings are in
[8 mixed-route workers](checks/loader-attempt-3.json) and
[19 A–F workers](checks/archive-final2/native-loaded-libraries.json).
[7 OCI worker FD inventories](checks/rootfs-fds-attempt-3/inventory.json) show the
actual opened generation disks outside guest shares. These are execution effects,
not configuration/log-only claims.

## Final installed artifacts

Source commit identifies the retained base, `405c8b89f151e9e2879438d9e457dd60bfd18b90`;
uncommitted inputs are captured by source SHA256
`a54f07ddea510d319125b0deec7c1aeb48c2e94f6c2af70e74f1911b251069c3`. Complete per-file hashes are in
`/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/source-frozen.json` and archive build-info; Cargo.lock SHA256 is
`0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb`.

| Artifact | SHA256 | Compressed bytes | Installed bytes (`du -sk` ×1024) |
| --- | --- | ---: | ---: |
| [Box archive](/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/final-dist-2/kbox-3.4.3-macos-arm64.tar.gz) | `34e2a8f09f7c029319e8a86495c1e9607dfb83941fe6a0752ab4972aa617f889` | 23309401 | 44044288 |
| [Separate OCI assets](/Users/roylin/code/kense/kbox/artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647/oci-final2-dist/koci-hvf-0.3.6-macos-arm64.tar.gz) | `fb1bc50474bea3802325352e1400d0811ed6177c42597a53a78b252acbcadba5` | 33124162 | 113782784 |

[Box build/install commands and provenance](checks/install-final2.json),
[OCI archive/extraction/checks](checks/oci-final2-install.json).
Box11 and OCI10 installed file checksums verified. Actual native execution uses
`installed-final-2` and `oci-final2-unpacked/koci-hvf-0.3.6-macos-arm64` beneath
the artifact root. Test clients are separately frozen qualification executables,
not an installed Rust SDK library; their hashes/build records are retained.
Runtime companions/libraries and the actual CLI come from the installed archives.

[Final identities](checks/final-identities.json) records CLI/shim/guest-init/Agent,
ELF/Mach-O architecture, signatures, dylib dependencies and the actual system
image/manifest. `debugfs` extracted system-image `/usr/bin/koci-agent` and its SHA
matches the installed Agent (`53e9becb96133e6f0af8b85ad2d4e2c516c16f3183d3692453087e4fd613143d`).
Installed Mach-O signatures and architectures checked successfully (ad hoc, not notarized publication).

Box actual libraries: libkrun`9ab3eefe28555dc193cec04f1cc2df03c30b8fda5c5bfd80290bfcbc7a518ad0`,
firmware`ca7bba4e17038ae73587db1f01a1e1c8e70ca565cae901b5106232e79acded49`.
`K_BUILD_LIBKRUN=1` source build is recorded in package-final2.log. The wrapper
uses the pinned Boxlite-ai firmware5.3.0 kernel.c tar SHA
`12b9401d7735d1682450e4d025273c5016ec2237dcbfb76b2f0a152be6e606d6`;
the recorded root firmware gitlink is not a claim that its checkout was compiled.
Root gitlinks remain libkrun3c2889c76460d81e3fb16d6144580fd31eb1bae3 /
libkrunfwfc3ea06895301f1df6a274dad75fbd9317b3bfee.

OCI keeps its separate fixed native archive
`5486f38e91eb4da0e58888b543c93fe669c918ad4b84dd495f0d1dfdffc43b56`:
libkrun`c5353f9cbd91564ce26eceaf1bdc33341097b43280fe029203ccca02807c082d`,
firmware`841bc9d5eecbc2aeeb6098fbc75d484427680d7503f5ed9bcdfe9d072a9420d4`.
Its system image SHA is
`5f03ac4dd9326463ab23e104bff80cfb960048f3b4254b63e032d55abd9b34b4`;
manifest SHA`b10bbf6a996397d1eaffda609e0a70d93d7950116331d931c981985f7ee0eab0`.
No Box/OCI native library or firmware selection was mixed.

## Failures retained and corrected

- Initial builder test failed on absolute process paths passed to relative layer
  normalization; fixed at the shared process-view boundary. builder-1 failure
  retained; builder-2 and subsequent9-test runs pass.
- Contract-check-1 used an array for a Vec capability API; SDK-tests-1 used the
  wrong capability test method; native-client-build-1 used a nonexistent stop
  method. Their logs remain; owning API fixes use required-capability validation
  and public kill_with_options(SIGTERM). No assertion was removed.
- OCI full-1 exposed two sandbox test callers with the old signature. Both were
  updated and the native sandbox asserts writable-file data plus denied deletion,
  rename and parent creation. A test-file naming collision briefly replaced an
  existing test file; initial status/hash were checked and it was fully restored,
  with new tests moved to macos_workload_rootfs_tests.rs. full-2 compile failure
  (duplicate module) and shim-pin-1 failure remain; full-3 / shim-pin-2 pass.
- attempt1 Rust native1/0/0 passed, but its wrapper exited1 on the unchanged
  final-CLI assertion because this diagnostic had no new installed CLI. It is
  not final archive acceptance.
- **Candidate1 installed attempt2 failed0/1/0** on actual CLI creation: no diff
  baseline existed and the independent Agent does not run Box guest-init.
  Root cause fixed in the logical rootfs/Box preparation caller, with the existing
  baseline contract/limits/atomic installer. [Failure](checks/box-native-attempt-2.json)
  and raw stderr remain. Candidate1 Box/OCI archives and full hashes remain in
  final-dist / oci-final-dist; they are superseded by candidate2, not overwritten.
  Focused9-test regression, installed CLI smoke, new verify gate and full native
  attempt3 all pass afterward.

## Owned cleanup

[Final audit](checks/final-cleanup.json): **14 private homes**, all execution and
volume lists empty; zero owned Host/VM/shim/engine/log/host-Agent processes,
mounts or workload disks; **175 observed socket paths absent**. Guest Agents end
with their reaped VM workers. Eight immutable OCI source blobs retain identical
bytes. Initial seven RAM mounts and the unrelated mounted image remain untouched;
there are no new RAM devices. No audit-wide deletion or user-data cleanup was
needed. Per-home CLI commands, final process/mount/plist data and socket origins
are retained beside the audit. A read-only global scan also found12 socket nodes
in three unrelated Box IDs; their 2026-11-07 birth times and initial process
record prove they predate this run. They were left untouched; see
[preexisting socket identities](checks/preexisting-global-sockets.json).

## Explicitly not claimed

Runtime memory was **not measured**; package size, installed usage and configured
guest memory are separate values. No fresh-host or other-platform native pass,
full OCI conformance/security/long-soak promotion, live-session continuity,
default cutover or release readiness is claimed. Windows ARM64 remains unsupported
by current dependencies and has not been run natively here. The two known missing
historical evidence links (`crates/oci-runtime/docs/evidence/2026-10-18-guest-command-status/OUTCOME.md`
and `docs/evidence/code-harness/20261018-macos-guest-owner/OUTCOME.md`) remain recorded
as missing in prior evidence; no
replacement content was invented. Unsupported file/named/anonymous/secret mounts,
custom network/devices, pools/snapshots and other profile exclusions still refuse.

[W4 ledger](../20261114-oci-host-service/W4-GATES.md) closes only the foreign
rootfs slice. Next bounded local task: admit single-file caller grants through
the existing SDK/Host PEP without exposing the parent directory. No work on that
separate task, VMM/firmware ports or Windows host waiting was started.
[Final source/reference review](checks/final-review.json) records the unchanged
source freeze, preserved initial files, clean native repositories and resolved
new evidence references.

# Exact single-file caller grants

Status: qualified for the bounded explicit macOS arm64 profile with the final
installed Box6/OCI7 archives below. Continuous W4 work remains active; full Host
Service cutover is incomplete. Completed UTC: 2026-11-14T15:49:48.688017+00:00.
Spec: [009](../../../../specs/009-oci-file-grants/spec.md),
[contract](../../../../specs/009-oci-file-grants/contracts/file-grants.md),
[tasks and all failure checkpoints](../../../../specs/009-oci-file-grants/tasks.md).

## Behavior and boundary

ExecutionManager reserves exact caller file device/inode identities in its
existing durable resource plan. The persisted OciSdk route, execution ID and
generation remain authoritative. SDK required `dev.kbox.file-grants` v1 and
Agent attachment manifest v6 carry ordinary Caller/DetachOnly grants. Rootfs-image
v1 / manifest v5 and Exec v1–v6/v7 retain their previous meanings. An old Host
missing the required capability refuses before activation.

The Host creates one generation-owned private hardlink projection per file,
outside the Guest-writable runtime share. It exports that projection, never the
caller parent and never a copied file. Source and projection are pinned and
reverified; RO sources cannot have an inherited writable descriptor. RO is
forced by Host Seatbelt; RW additionally permits data write/truncate, while
link/unlink/rename/chmod/chown and sibling creation remain denied. Projection
link-count/ctime changes are intentional admission/revocation effects; setup
and cleanup do not change caller content, owner or permissions. Source content
is live in both directions. Explicit Host replacement of the pathname does not
replace the active inode grant, and restart refuses the new object.

Same-filesystem, canonical, caller-owned unaliased ordinary files are required.
Existing aliases, competing grants of one inode, control-directory overlap,
foreign-owned files and special devices refuse. Cross-filesystem refusal is a
safety boundary, not implementation of that combination. Named/anonymous/secret
mount allocation remains outside this slice.

Product VM lifetime now follows the existing exact-parent watchdog and driver
shutdown. The internal macOS `--runtime-owner` flag is explicit; standalone
CLI/library diagnostics keep their 90s deadline. No startup/request timeout was
increased. Bindingless Killing must prove Host absence before product cleanup;
an unfinished Create cannot be forgotten or silently rebound to a current
runtime. Terminal preparation errors clean only their exact target/config
handoff; cleanup failure retains retryable ownership. A still-pending Create
requires reconciliation of its original SDK request before deletion; general
in-flight Create cancellation is not claimed.

## Frozen installed artifacts

Source-input SHA256 (existing package algorithm):
`3d07d7318f277094813eb973e746339c8fce5934bf95bcf1021a9011a7e54ca5`; root commit `405c8b89f151e9e2879438d9e457dd60bfd18b90` plus retained dirty
source. The algorithm covers Rust/TOML/proto/plist, Cargo/toolchain and package
scripts; [supplemental input hashes](checks/supplemental-source-identities.json)
bind runtime-assets/source JSON, system-image builder and C/Python fixtures.
No commit, staging, push or publication was performed.

| Artifact | SHA256 | Compressed bytes | Installed disk bytes (`du -sk`) |
| --- | --- | ---: | ---: |
| Box6 | `a35388f45b1c326d6fd549a30fb3545e30176194e3b8f7bdb088a9016df313bb` | 23,332,309 | 44,060,672 |
| OCI7 | `ce2b65c4b93b8843b9d4a9b897e5b37e217a78895458ae5fd7f2e58586fd9239` | 33,333,578 | 114,847,744 |

[Box archive](../../../../artifacts/oci-files-38d1ee741be94955907fc6c8b3fad645/final-dist-files-6/kbox-3.4.3-macos-arm64.tar.gz)
and [OCI archive](../../../../artifacts/oci-files-38d1ee741be94955907fc6c8b3fad645/oci-files-final7-dist/koci-hvf-0.3.6-macos-arm64.tar.gz)
were each extracted/installed to a new prefix. Each has 11 verified checksum
entries. Commands, prefixes and build provenance:
[Box install](checks/install-files-final6.json), [OCI install](checks/oci-final7-install.json),
[architectures/signatures/native dependencies/embedded Agent](checks/final-identities-6.json).
Native tests use installed companions with cleared loader overrides; test clients
are separately recorded qualification executables. No developer/user installation
supplies a runtime dependency.

The OCI native archive is `d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`;
libkrun `7dcaf94032e22d2044aed36b7c43f17dd311fed65369f0f292c2e65337856ddf`, firmware `841bc9d5eecbc2aeeb6098fbc75d484427680d7503f5ed9bcdfe9d072a9420d4`,
kernel `b1180b50148ed14f5fbeadf17288ce8abcf245daa468255b7ff41113bbf01199`
(22,740,992 bytes, load/entry `0x80000000`). Firmware and kernel are the exact
retained OCI bytes, not the different Box companions. Box dependency identities
remain in its install/build and final identity records.

The new generic per-export uncached API selects Never data caching and zero
attribute/entry TTL only for file exports. It is from a separate linked worktree
of the existing libkrun repository at base `3c2889c76460d81e3fb16d6144580fd31eb1bae3`.
The native source is **uncommitted**, not a landed parent gitlink:
source `c89d545b25021c02c7739d300a746405772d889fd422935d9bf8a8ba2b5817bb`, patch `394f321494615786c4f6c3aa7f3cb181d054ac87a3e034df6056145533f5acdd`,
full source archive `2097d9bea8640f5d2e04ce92f814e356a710d281e0970c94dfdce957ebbcfe80`. See
[native source sidecar](../../../../crates/oci-runtime/crates/kbox/runtime/macos-aarch64/uncached-source.json)
and [exact build/tool/ABI checks](checks/native-cache-bundle-v2.json).
It uses Rust1.97.1, arm64 and the prior Mach-O minimum11.0; only the host below
was run. Parent libkrun/libkrunfw source worktrees and gitlinks remain unchanged.
No firmware or VMM architecture port was started.

## Actual verification

Host: macOS26.6.2 (25G83), Apple M5 Max arm64, euid501, Rust1.97.1;
[environment](checks/initial-environment.log). Guest is Linux arm64; Agent and
native attack probe are static aarch64 ELF. No Windows or other-platform native
result is inferred.

| Check | Actual result | Evidence |
| --- | --- | --- |
| SDK / Agent protocol | 213 /63 passed | [log](checks/file-sdk-protocol-full-1.log) |
| Box Runtime / CLI, final default gate | 2179 passed,18 ignored /933 passed | [verify log](checks/verify-commit-files-5.log) |
| OCI Runtime full | 437 passed,2 ignored | [log](checks/file-runtime-full-4.log) |
| OCI Agent | 25 passed (unchanged Agent source/bytes) | [owning full log](checks/file-owning-full-1.log) |
| Shim all targets | 75 lib passed/3 ignored;18 binary passed/3 ignored;3 integration passed | [log](checks/file-shim-full-9.log) |
| Shared create/session cleanup | 40 passed | [log](checks/shared-create-cleanup-2.log) |
| Bindingless kill / terminal preparation | 2 /2 passed | [kill](checks/unbound-kill-after.log), [prepare](checks/terminal-prepare-after-2.log) |
| Native dependency | devices61 +utils21 +FFI1 passed; Clippy/release pass | [full](checks/native-eventfd-full-3.log), [FFI](checks/native-libkrun-full-1.log), [build](checks/native-checks-4.json) |
| fmt / owning Clippy / workspace / verify-commit | all exit0 | [commands and status](checks/verify-files-5.json) |

[Exact test counts including ignored/zero doc tests](checks/actual-test-counts-source6.json)
are retained; ignored entries are not counted as execution. Projection cancellation,
partial publication, locking, identity drift, malformed bounds, alias/overlap and
capability rejection have focused real-implementation coverage. Cross-filesystem
HFS→APFS admission executed1/0/0 and refused EXDEV without copying; its owned RAM
device was unmounted/detached ([result](checks/crossfs-native-2.json)). System-image
manifest generation parser tests pass2, including5 negative subcases
([log](checks/macos-runtime-builder-tests-1.log)).

Final installed full matrix executed **1 passed/0 failed/0 ignored**, 384.13s;
[result and command](checks/box-native-attempt-14.json),
[installed loader maps](checks/loader-attempt-14.json). It includes two independent
file executions, empty RW and Unicode/space RO paths, exact destinations, live
Host growth after a prior Guest read, Guest writeback, normal restart, stale
input, source replacement refusal and independent cleanup. Five file proof
reports each require16 EPERM/EACCES Host denials after the Guest readonly
superblock is explicitly cleared, plus fresh RO/RW tag mounts and fork-child
refusal. EROFS is not accepted as Host proof. The same matrix repeats directory
RO, file RO/RW and foreign UID/GID rootfs checks after Host SIGKILL/stopped
recovery, plus mixed BoxVm/OciSdk routes, cap_drop ALL, I/O, resource controls,
exact bindings, response-loss replay/concurrency and the actual installed CLI.

Final installed failure/reconciliation matrix executed **1/0/0**, 40.42s
([result](checks/box-native-attempt-15.json), [loader](checks/loader-attempt-15.json)).
A copy of the installed shim intentionally loses only its HVF entitlement.
Failed admission preserves the Box reservation and caller identities; the actual
installed CLI refuses unsafe bindingless kill. Restoring the real shim and
replaying the exact original SDK Create permits exact forced deletion without
Start. Caller files remain intact. [Installed CLI proof](checks/installed-cli-unbound-kill.json).
The >90s owned lifetime test also executed1/0/0 with installed Box5/OCI6;
commands span97.755s ([result](checks/box-native-attempt-12.json)). That owner
implementation and native payload are unchanged in OCI7; the final full matrix
also retains its long-lived main VM through CLI and recovery operations.

A–F five scenarios were all rerun using **Box6 installed companions**:
OCI lifecycle, RO/restart, primary process streams, prewarm and directory/fencing.
Prewarm and directory each execute1/0/0; the other three each report one actual
completed native scenario. [Commands/results](checks/af-final6/native-results.json),
[loader proof](checks/af-final6/native-loaded-libraries.json). No old result is
substituted for this changed Box binary.

## Retained failures and corrections

All logs, rejected candidates and prior archives remain. The checkpoint file
records compile/fixture mistakes and exact subsequent tests. Material failures:

- A preopened Darwin writer survived sandbox installation. Admission now rejects
  matching dev/inode writer descriptors; no revocation of existing writers is claimed.
- /dev/fd hardlinking returned EPERM; private pathname links are verified against
  pinned objects before VM publication. Cross-filesystem copying is forbidden.
- Cancelled blocking preparation initially left late aliases; an owned TempDir
  result and fs2 lock now cover cancellation, publication and cleanup.
- The first Guest probe accepted EROFS; strengthened probe2 requires a writable
  Guest superblock and actual Host EPERM/EACCES. Original probe is retained.
- Final2 failed immediate live-length visibility. Native uncached exports fix
  both data and metadata caching; the strict first-read assertion remains.
- Native full tests exposed event counter semantics; after fixing them a queue0
  MMIO notification wrote0 and stalled boot. Before/after regression proves the
  caller now signals1. The speculative bootstrap edit was withdrawn; TMPDIR was
  already inside the pinned runtime share. No GIC fallback or firmware swap occurred.
- Minimum-OS14 candidate was rejected; minimum11.0 and Rust1.97.1 were restored.
  Signing-neutral pin and hardcoded old system-image native identity were fixed;
  mutation rejection and exact provenance checks remain.
- Final3/4 matrix aborted when a product VM inherited the diagnostic90s lifetime.
  Explicit owner mode fixes the shared cause; direct diagnostics remain bounded.
- Cleanup audit found4 retained retryable Create disks/projections despite empty
  Box lists. Bindingless Killing now proves Host absence. Shared terminal prepare
  cleanup is identity-bound and retains ownership on cleanup failure. Old5–7
  settled their exact Create failures through the repaired candidate Host
  boundary; old4's terminal record was already quarantined and was repaired via
  the same cleanup API under an exclusive durable-root lock, matched failed
  receipt/config/attachments, and no active process. No lost workspace was
  recreated and no caller file was deleted. [Old4 repair](checks/terminal-repair4-1.json),
  [old5](checks/pending-attempt5-1.json), [old6](checks/pending-attempt6-1.json),
  [old7](checks/pending-attempt7-1.json).

## Owned cleanup

[Final audit](checks/final-cleanup.json):25 private homes,0 managed executions,
0 volumes,0 owned Host/VM/shim/engine/Agent processes,0 owned mounts/workload
disks;215 observed socket paths absent. [Descriptor audit](checks/final-descriptor-audit.json)
finds0 same-user owned runtime descriptor holders. Guest Agents ended with reaped
VM workers. Eight immutable source blobs keep exact bytes; initial7 RAM identities
and the unrelated mounted image remain unchanged. The audit deleted no resource.
[Caller-source inventory](checks/final-file-source-cleanup.json) records retained
files, parent mode/owner, canaries and one remaining link per source. Earlier
partial audit failure, after-repair snapshot and exact recovery commands remain.

## Explicitly not claimed

Full W4/OCI Host Service/default cutover; fresh-host or other-platform runtime
qualification; production/readiness promotion; live-session continuity after
HVF Host death; general in-flight Create cancellation; downgrade/rollback safety
of the new Box caller-file metadata; unsupported allocations/controls; Developer
ID/notarization; runtime memory measurement. Windows ARM64 is unsupported by
current dependencies and has no native run. Native changes remain uncommitted
in their owning linked worktree. Two preexisting missing historical evidence
paths remain missing, as recorded in spec007/008; none was fabricated.

Next bounded local W4 correctness task: validate and fence old Box rollback that
may ignore caller-file metadata. Do not end continuous work at this stage.

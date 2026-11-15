# Caller-file metadata compatibility

Status: qualified for the bounded explicit macOS arm64 profile using final
Box2 and unchanged OCI7 installed assets. Continuous W4 / enterprise GA work
remains active. Completed UTC: 2026-11-14T16:40:54.396024+00:00.
[Spec](../../../../specs/010-oci-file-rollback/spec.md),
[layout contract](../../../../specs/010-oci-file-rollback/contracts/state-layout.md),
[all checkpoints/failures](../../../../specs/010-oci-file-rollback/tasks.md).

## Actual failure and resulting behavior

Actual accepted spec008 CLI started a file-bearing reservation after its source
pathname was replaced by a directory. The real HVF guest read the directory's
canary and the old writer discarded caller_files (1→0). Before1 executed
**0 passed /1 failed /0 ignored**; cleanup completed. Its [command/counts](checks/box-native-attempt-1.json),
[state/canary observation](/private/tmp/kbox-rollback-ccc80622aec3/attempt-1/rollback-observation.json), raw state
before/after, CLI output and native loader proof remain retained.

The same BoxStateStore now publishes file-bearing roots as an actual directory
at logical boxes.json, containing the single authoritative records.v1.json.
The sibling lock, record identities, creation digests, persisted routes,
operations and generations retain their meanings. macOS RENAME_SWAP atomically
exchanges a complete synced private directory with an old flat file. New
reservations publish before backend effects; normal current loads adopt old
flat file-bearing records under the lock with a fresh read. Readonly snapshots
do not migrate. Legacy no-file roots remain flat; adopted roots stay fenced
when empty after cleanup. This adds no second lifecycle authority or config.

Old readers fail I/O before their quarantine-to-empty fallback, and old flat
rename writers cannot replace the actual directory. Missing, corrupt, future
or symlinked directory records fail closed, including record loss after path
resolution. Managed transitions cannot drop/replace caller-file bindings.
Secret nondisclosure evidence still scans actual physical bytes; fixture cleanup
handles only its exact owned store. Caller files/parents are not changed by
store adoption. Backups must preserve the complete layout. Arbitrary rollback
or manual flattening is unsupported.

## Final installed artifacts and environment

Actual environment is retained in /Users/roylin/code/kense/kbox/artifacts/oci-rollback-ccc80622aec3439491048d99d16b2897/environment.txt and mounts-before.txt:
macOS26.6.2 (25G83), Apple arm64, same-UID501. Rust1.97.1 is pinned. This is
existing-host acceptance; no fresh-host/minimum-OS/other-platform runtime claim.
Root source commit405c8b89f151e9e2879438d9e457dd60bfd18b90 plus dirty inputs.
Source3 SHA256: **b60caefc24eb123281d699c8d268cdb6ddc9605d25a3487676fee5aa82645689**. Full source file inventory is
/Users/roylin/code/kense/kbox/artifacts/oci-rollback-ccc80622aec3439491048d99d16b2897/source-frozen-3.json; excluded validation/fixture inputs are separately
[hashed](checks/supplemental-source-identities.json).

| Asset | Identity / size |
| --- | --- |
| Box2 archive | /Users/roylin/code/kense/kbox/artifacts/oci-rollback-ccc80622aec3439491048d99d16b2897/final-dist-rollback-2/kbox-3.4.3-macos-arm64.tar.gz |
| Box2 SHA256 | `9856cab0edc2bbe4ff2f0f2da1e0537d2e65ee73a4f44e358656e04a44918078` |
| Compressed / installed | 23,333,369 / 44,060,672 bytes |
| Isolated installed prefix | /Users/roylin/code/kense/kbox/artifacts/oci-rollback-ccc80622aec3439491048d99d16b2897/installed-rollback-final-2 |
| Immutable native test client | `5eed111dac5ca0dfdc87f03173b3c94b9425b7ab91b932ab117c99d3efbe3611` |
| Reused OCI7 archive | `ce2b65c4b93b8843b9d4a9b897e5b37e217a78895458ae5fd7f2e58586fd9239` |

[Package command](checks/package-rollback-final2.json),
[exact installer/checksums](checks/install-rollback-final2.json) and
[complete file/native/signature/architecture identities](checks/final-identities-2.json)
verify11 checksum entries, actual arm64 CLI/shim/libraries, static Linux/aarch64
guest-init/Agent, signatures and the embedded Agent matching standalone bytes.
All identity commands exit0. Ad-hoc signing is not Developer ID/notarization.

The [OCI source inventory](checks/unchanged-oci-source3.json) proves its source
and selected assets are unchanged. The actual accepted installed OCI7 Host,
shim, Agent, system image, native library/FW are reused, not claimed rebuilt.
Its source digest remains3d07d7318f277094813eb973e746339c8fce5934bf95bcf1021a9011a7e54ca5;
33,333,578compressed /114,847,744installed bytes. Exact dependency/source/patch
provenance and the uncommitted owning native worktree are in
[spec009](../20261114-oci-files/OUTCOME.md). Parent native worktrees/gitlinks
remain unchanged. All old archives and failed candidates remain retained.
Runtime memory was not measured; configured guest memory is not RSS.

## Commands and actual results

| Check | Actual result / evidence |
| --- | --- |
| New compatibility tests |10 pass: migration, no-file compatibility, immutable authority, empty-after-cleanup fence, corruption/missing/future/symlink data, concurrency/stale writes, eager adoption and concurrent record loss; [focused log](checks/record-loss-after.log) |
| Frozen-source full gate |scripts/verify-commit.sh exit0: Box2189 pass/19 ignored, CLI933 pass; all workspace targets, fmt, scoped Clippy, contract suites and installer8 actual tests; [stdout](checks/verify-commit-3.log), [stderr](checks/verify-commit-3.stderr), [command](checks/verify-commit-3.json), [all exact result rows](checks/actual-counts-source3.json) |
| Final installed compatibility6 |1/0/0 in1.78s; actual new CLI adopts an exact legacy flat fixture without changing records; old start/inspect/rename/run all fail EISDIR; no Host container or VM worker, state unchanged; [counts](checks/box-native-attempt-6.json), [result](/private/tmp/kbox-rollback-ccc80622aec3/attempt-6/result.json) |
| Final installed full7 |1/0/0 in379.89s; actual installed Box2/OCI7, mixed routes, reconstructed clients/policy changes, rootfs UID/GID/links, exec/streams/files, resource/cap_drop ALL controls, relay response-loss/concurrency, file/directory RO/restarts and authenticated Host SIGKILL stopped recovery/new generation; [counts](checks/box-native-attempt-7.json), [result](/private/tmp/kbox-rollback-ccc80622aec3/attempt-7/result.json), [loader proof](checks/loader-attempt-7.json) |
| Final installed failedCreate8 |1/0/0 in41.34s; owned no-entitlement shim refuses, actual installed CLI retains bindingless Killing authority, exact original SDK Create reconciliation/delete without Start, caller files preserved; [counts](checks/box-native-attempt-8.json), [result](/private/tmp/kbox-rollback-ccc80622aec3/attempt-8/result.json), [loaders](checks/loader-attempt-8.json) |
| Final installed A–F2 |all5 actual scenarios pass: OCI lifecycle, Host RO/restart, process streams, prewarm1/0/0 and directory/fencing1/0/0; [results](checks/af-final2/native-results.json), [loaded companions](checks/af-final2/native-loaded-libraries.json) |

The five final full7 file probes each require16 Host EPERM/EACCES denials,
child refusal and two fresh mounts after clearing Guest read-only flags. EROFS
is not accepted. Live Host updates/RW writeback, replacement identity refusal,
independent grants and source-preserving cleanup repeat through restart and
stopped recovery. The compatibility refusal test alone is not RO/VM support.
The raw full7 proof reports remain beside its result file.

No zero-test/ignored/build result is called a native pass. Unchanged OCI/SDK/
protocol/Agent full suites from spec009 are historical and not counted as rerun.

## Retained failures and repairs

- Before native1 proved real directory exposure and authority loss; before
  focused2 was1pass/6fail. Initial wrong sibling-module path failed compilation,
  and an incorrect SDK List return-field reference also failed compilation;
  both corrected with failure logs kept.
- A JSON version parse refusal cannot fence an old quarantine-to-empty CLI.
  A trial symlink directory was withdrawn because stale flat rename writers can
  replace it. The [actual atomic probe](checks/atomic-layout-probe.json) proves
  mixed file/directory exchange and stale flat writer EISDIR.
- After2 had8pass/1fail due a stale read_link fixture assertion from that
  withdrawn layout. The corrected test still asserts future/missing refusal
  and retained future bytes; after3 all9 passed. No refusal assertion removed.
- Candidate1 (source2 /archive29e4f65e81f83e0fdc2c87e0db2d76a92d0737d22b55c00fbff205e6e20306e1)
  passed verify2, native3/4/5, A–F1 and its11-home/89-socket cleanup. Those results
  and bytes remain historical, not the final source3 asset.
- Final review's concurrent record-loss test then failed0/1/0 in0.98s:
  [before](checks/record-loss-before.log). The shared missing-file branch could
  synthesize empty after directory resolution. One condition now limits
  missing-as-empty to legacy flat stores. All10 tests pass; Box2 was rebuilt,
  verify3 and all affected final installed matrices were actually repeated.

## Final owned cleanup

[Final resource audit](checks/final-cleanup.json):**20 private homes**,0 managed
executions/volumes,0 owned Host/VM/shim/engine/Agent processes,0 owned mounts or
workload disks;**177 observed socket paths absent**. [Same-UID descriptor audit](checks/final-descriptor-audit.json)
finds0 owned runtime descriptor holders. Eight immutable source blobs match;
initial7 RAM identities and the unrelated mounted image remain unchanged.
Audit deleted no resource. Runtime removal used the canonical manager/SDK APIs.

[File inventory](checks/final-file-source-cleanup.json) records retained caller
files, source nlink1, unchanged parent owner/mode and canaries. [Rollback source
inventory](checks/rollback-caller-cleanup.json) retains the deliberately renamed
originals and replacement-directory canaries unchanged. Empty directory-format
state roots intentionally remain as the compatibility fence, with no execution
resources. Candidate1 audit copies are retained separately.

## Explicitly not claimed

Complete W4/OCI Host Service or default cutover; enterprise GA/readiness or
support-level promotion; live-session continuity; arbitrary binary downgrade or
manual state flattening; unsupported allocation/control parity; general
in-flight Create cancellation; fresh-host/other-platform/minimum-OS runtime;
Developer ID/notarization, power-cut installation/migration testing or runtime
memory measurement. Windows ARM64 is dependency-unsupported and has no native
run. No commit, push or publication occurred. Two previously missing historical
evidence paths remain missing; none was fabricated.

Next immediate bounded W4 local task: named VolumeStore mounts through existing
Caller/DetachOnly transport, with actual lifecycle/restart/source-preserving
cleanup. Anonymous and Secret allocation remain separate open slices. Continue
work; this bounded acceptance is not the continuous task's end.

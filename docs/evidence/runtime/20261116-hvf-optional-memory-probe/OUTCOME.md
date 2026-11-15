# Optional RAM capability failure no longer disables ordinary HVF execution

The independent installed Host now runs ordinary workloads when its optional
RAM-backing probe returns FailedPrecondition. Discovery explicitly reports
`memory_secret_files=unavailable-memory-backing-not-verified` and retains the
bounded OS cause. All memory-secret extension versions remain absent; an actual
public SDK Create request receives Unsupported before a container is admitted.
No Secret backing, permission, isolation or backend fallback is introduced.

This closes the bounded availability defect, not the failed full release gate.
The six actual RAM/APFS default-test failures remain external prerequisites.

## Source and checks

`crates/oci-runtime/crates/runtime/src/hvf_driver.rs` isolates only the optional
probe's FailedPrecondition. Unexpected errors and task-join failures still abort
driver opening. Positive RAM proof still advertises exactly versions1/2/3;
physical backing still omits them. Per-request Shim backing checks and durable
required-extension checks are unchanged. A durable record requiring a missing
Secret extension still refuses reopening; its contract is never stripped.
DedicatedVm isolation, experimental readiness, default BoxVm routing, resource
authority, identity and generation fencing remain unchanged.

| Actual check | Result |
| --- | --- |
| Focused `CARGO_NET_OFFLINE=true cargo test --locked -p koci-runtime --lib hvf_driver::tests -- --nocapture` before fix | 2 passed /1 failed /0 ignored; [red receipt](checks/red1.json) |
| Same command after fix | 3 passed /0 failed /0 ignored; [receipt](checks/focused-green1.json) |
| `CARGO_NET_OFFLINE=true cargo test --locked -p koci-runtime --lib` | 441 passed /0 failed /2 ignored;398.28s, [receipt](checks/runtime-all1.json), [actual output](checks/runtime-all1.log) |
| `CARGO_NET_OFFLINE=true cargo clippy --locked -p koci-runtime --all-targets --no-deps` | Pass; [receipt](checks/clippy1.json) |
| `CARGO_NET_OFFLINE=true bash scripts/verify-commit.sh` | Workspace check/fmt pass; Runtime2271 passed /6 failed /46 ignored; exit101, [receipt](checks/verify1.json), [actual output](checks/verify1.log) |

The original installed failure is retained in the
[preceding phase](../20261116-native-tool-diagnostics/OUTCOME.md). Direct
[diskutil still exits1](checks/diskutil1.json) with DiskManagement unavailable.
No test, assertion, ignored status or release gate is changed to hide that error.

## Current installed native execution

Two actual ignored-by-default Rust tests were selected by exact name and each
completed1 passed /0 failed /0 ignored with the newly installed assets:

- Single-file Host PEP: live host updates and guest writeback,16 real Host
  denials, child denial, writable remount and two fresh mounts.
  [Command/count/result receipt](checks/box-native-files-attempt-1.json).
- Independent Host lifecycle: mixed persisted BoxVm/OciSdk, reconstructed
  clients, ownership, two-execution file isolation and normal restart,
  response-loss/replay, CLI, dropped capabilities, Host death/stopped recovery,
  old-generation refusal and repeated file/directory Host denials.
  [Command/count/result receipt](checks/box-native-host-service-attempt-1.json),
  [actual final result](checks/native-host-service/result.json).

Six file PEP result rows each observe16 Host denials, child denial, writable
remount and two fresh mounts; directory probes observe14 denials before and
after stopped recovery. Actual result files and counters are retained in
[native-proof-counts.json](checks/native-proof-counts.json). Runtime loader maps
bind the actual Box/OCI workers and libraries to the installed prefixes.

One additional standalone Rust client uses the real public SDK against a fresh
installed Host. Discovery retains the actual DiskManagement cause, omits all
three Secret versions, and still advertises the same experimental DedicatedVm
driver. A valid required memory-secret Create receives
`Unsupported/create: runtime does not support required extension
dev.kbox.memory-secret-files version 1`; inventories are empty before and after,
and caller file identity/mode/owner/bytes remain equal.
[Native API result](checks/native-capability-result2.json),
[exact launch/exit/cleanup](checks/native-capability-launch2.json),
[client source](checks/native-capability-client.rs).

The client initially used a nonexistent DriverKind string method, then selected
the v1 handoff helper for a v2 external-storage request. These probe errors are
retained in [compile1](checks/native-capability-client-build1.json) and
[native attempt1](checks/native-capability-launch1.json). The source was corrected
to the existing typed enum and v2 storage-handoff helper; assertions are retained.
[Compile3](checks/native-capability-client-build3.json) and native attempt2 pass.
The failed candidate binary and all inputs remain in the artifact directory.

## Artifacts and identities

Frozen compiled inputs SHA256
`25b4cf5f312a998a6132b2ead24b65286ff10bdb848d6e2e780c6861aa8215ef`,1710 files;
build base `440e179630c4432e5a4c580bcb4d1cb4f7ab6ee2` plus this recorded diff.
Cargo.lock SHA256 remains
`3cec984fb77851fcf5e6bbd3838cad56b71c2249cb29e5ec01ec19c36918adf8`.

All paths below are under
`artifacts/hvf-optional-memory-probe-ed388b4f5a644712b93b24521bf10b9f/`.

| Final archive | SHA256 | Compressed / installed bytes |
| --- | --- | --- |
| `final-dist-probe-1/kbox-3.4.3-macos-arm64.tar.gz` | `619cf47eeb958eb918f4b21ab7ea19a68af515a80b8f17b3ddbcd3ef8ca19496` | 23,398,689 /44,175,360 |
| `oci-merged-final1-dist/koci-hvf-0.3.6-macos-arm64.tar.gz` | `f014049494a8f9f3a16f7f40ceed49f92d73160e6093d6087f8e046a25205423` | 33,427,110 /114,954,240 |

Fresh prefixes: `installed-probe-final-1` and
`oci-merged-final1-unpacked/koci-hvf-0.3.6-macos-arm64`. All11 companions per
archive, architecture/signatures, source provenance, native dependencies and
embedded/standalone Agent equality are verified.
[Box receipt](checks/install-probe-final1.json),
[OCI receipt](checks/oci-merged-final1-install.json),
[full identities](checks/final-identities-1.json).

The frozen Box native test client remains
`ee668a98491db72810b2a1e8b672ca5c3191ccbfeda1bd0637c6ea99186baa6d`.
All five actual Box runtime companion hashes equal those used by the preceding
five A–F scenarios; that evidence is reused by exact byte identity and is **not**
reported as five newly executed scenarios.
[Binding](checks/unaffected-box-native-binding.json).

Parent native gitlinks remain libkrun3c2889c7… and libkrunfwfc3ea068…; no native
submodule source is edited. OCI native bundle SHA256 remains
`d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`.
[Environment/toolchain/gitlinks](checks/environment-closeout.json).
Runtime RSS is unmeasured; installed disk bytes are not running memory usage.

## Cleanup, Git and next action

Scoped audit passes all4 actual native homes, including the failed API-probe
attempt: zero managed executions/volumes, owned processes, mounts, workload
disks or open descriptors. Nine observed socket directories and36 socket paths
are absent. All31 pre-existing immutable cloned-cache files retain hash, owner
and mode; index metadata is the expected owned-cache change. Caller inputs and
both preceding final archives are retained. The audit deletes nothing.
[Scoped cleanup](checks/final-cleanup.json),
[actual CLI inventories](checks/final-home-inventories.json).

Local UID501 lookup and github.com DNS still fail in an independent
[prerequisite observation](checks/git-prerequisites-observation1.json). The
previous real push failure remains authoritative; no duplicate remote attempt,
credential/DNS change, daemon restart or force push is performed here.
After recovery, fetch the actual remote head, integrate without overwriting
local work, revalidate affected source/assets and push normally.

The active continuous goal resumes local W4 work after the prior local Git
convergence. Next bounded gap: existing custom VM CPU/memory controls through
the actual independent OCI VM worker, required capability negotiation and
native hardware readback. Keep current refusal until that full chain exists.

## Explicitly not claimed

Full default verify, final release acceptance and enterprise GA remain open.
Actual positive RAM Secret/APFS operation, original intermittent parallel warm
boot, the unknown removal actor for seven historical RAM mounts, fresh-host,
full conformance/security review, long soak and other-platform matrices remain
unresolved. The historical strict RAM-preservation failure is retained.
Linux OpenSSL compilation/native TLS still needs the uncached official crate
and working registry DNS. Windows ARM64 dependencies remain unsupported and
have no native run. Full W4/default OCI cutover and live-session continuity are
not completed; platform support levels and stopped-only HVF recovery do not
change. No release is published.

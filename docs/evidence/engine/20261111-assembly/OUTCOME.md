# Directory grants, shared base and guest installation

Date: 2026-11-11. Native host: macOS / arm64, HVF; guest: Linux / aarch64.
Result: A01 implemented and verified. A02 guest tooling/base implemented;
complete MateHarness orchestration and the remaining M2 gates are unfinished.

## Product boundary

Ordinary OCI retains its existing independent entry points. ACP packages,
KRun, assembly receipts and the native Harness profile are optional consumers
or extensions. Directory grants also work for ordinary OCI specifications and
do not select a Harness profile. ExecutionManager retains actual resource
authority; MRC retains operation receipts.

`Directory` is an explicit mutable reference, enabled only by a caller-owned
port. Immutable Artifact inputs retain their read-only contract. Engine grants
bind exact source paths, permissions, accepted specifications and generations.
They retain teardown authority for older attempts without authorizing future
ones. Caller data survives removal, including when the source has been moved.

The native test initially failed because libkrun sanitized Unicode guest mount
paths to underscores. Mount controls now encode paths needing an ASCII-safe
transport, with a paired guest decoder; ordinary ASCII controls keep their wire
form. Both volume and tmpfs callers use the same codec.

## Native results

- [Directory proof](30adbf3a2f5f40d9af526f1f84b09802/result.json): PASS. Actual
  canonical Unicode/space workspace path, external directory, 16 MiB binary
  write/rename/hash, child process, live host changes, unmounted canary,
  old-generation refusal, moved-source cleanup, zero private executions.
  The [initial failure](6c7a1bfdad4345e2a55db0e6372f61c1/native.stderr) is retained;
  its failed execution was stopped/removed through KBox.
- [Independent KRun SDK proof](../../../../../krun/docs/evidence/2026-11-11-assembly-engine-native/OUTCOME.md):
  PASS across three actual environment generations, directory bindings and two
  raw process cases. No host Harness execution.
- [Ordinary OCI proof](../20261111-assembly-oci-private-build/55a13cdabc814d56b823e1406d32afe0/result.json):
  PASS. Pull, COPY + RUN build in a private build pool, run/exec/child,
  Unicode/space files, named volume, real HTTP, snapshot/restore, same ordinary
  execution after engine SIGKILL/restart, native profile/direct baseline and
  zero private executions. The ordinary runtime prefix contains no KRun binary.
- [Shared base build](base-toolchain-c66967fb55c14982b1b89be50645efc9/result.json):
  PASS, `krun-base@sha256:4d054fc12eef66d0e1435d3b084988c38bf57b1ea31f3b9ba38e823f5e3f5f51`.
  Fixed official Node/Debian tooling plus the trusted guest installer; no Harness
  or credentials are installed in the base.
- [Three ZIPs through Rust installer](../../../../../krun/docs/evidence/2026-11-11-guest-installer-native/OUTCOME.md):
  PASS for Aster 0.1.0, Dsh 0.1.0 and Codex 0.1.1, ACP initialize and EOF.

The exact isolated runtime prefix and companion hashes are retained in
[artifact metadata](checks/assembly-native-prefix.json). KBox executable SHA256:
`07d0998bbf5dcb866ebdb9e67d85f29df52e9deb91db1b8c41a43f6293f35234`;
paired guest-init: `31fcbd8783323a0cf3eb06d1ad0650075fea172efb09d615ea17ce3f52237440`.

## Checks and failures

Commands and raw logs are retained in `checks/`:

| Check | Result |
| --- | --- |
| `cargo test --locked -p kbox-runtime engine::` | 161 PASS; filter includes OCI build engine tests |
| `... kbox_contract_driver::directory::tests` | 4 PASS |
| `... kbox_contract_driver::artifact_tests` | 7 PASS, including both port composition orders |
| `... engine::directory::tests` | 3 PASS |
| `... kbox_contract_driver::engine_tests` | 10 PASS |
| `cargo test --locked -p kbox-core guest_exec` | 8 PASS |
| Runtime Mounts conformance requirements | 5 PASS |
| `cargo check --workspace --all-targets --locked` | PASS |
| Owning-package Clippy | PASS with existing warnings |
| Scoped Rust formatting | PASS |
| Full workspace formatting | FAIL; 54 files also occur in the recorded 55-file baseline, no added failing path |

The expanded contract run fails one MCP fixture identity check. Remaining
targets expose two protocol golden failures. All three failures reproduce on
unchanged HEAD `0304123f562ce7264954a8a104dced9e19b3d483` in an isolated worktree;
[baseline log](checks/assembly-contract-baseline-tests.log). No fixture, expected
digest or validation gate was changed. The 10,001-request persistence test
passed in the expanded run. Full contract-suite success is not claimed.

Two attempted Debian `RUN apt-get` builds failed with an incomplete response
frame, including a private 2 GiB helper pool. Both attempts are retained under
`base-4a6f22b37636450489d3c747e1fdd760` and
`base-private-3fb3eeb3a4d84ab0af7280d02dc2840a`. Root cause remains unresolved;
the private pool was stopped. The successful base instead reuses a pinned
generic toolchain image. This does not establish that the failing build path
has been fixed.

## Explicitly not claimed

- Complete KBox MateHarness host-to-guest archive streaming/assembly receipts,
  prewarm handoff, memory credentials/native refresh or formal KRun CLI.
- Real-model, five-capability, long-task, cancellation/resume or performance
  acceptance of the three Harnesses. Initialization is a limited probe.
- Linux KVM or Windows WHPX native qualification, or general host read-only
  virtio-fs enforcement on macOS. Unsupported RO admission remains refused.
- A clean whole-repository test/format baseline, or resolved networking/package
  installation build failures. No commit or push was performed.

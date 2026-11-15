# Tasks: Windows ARM64 feasibility

- [x] F1 Verify official virtualization, compiler/ABI and dependency support.
- [x] F2 Trace native host, guest and packaged-asset architecture admission.
- [x] F3 Reproduce/fix justified capability and refusal gaps with focused tests.
- [x] F4 Run available builds/tests/Clippy and `scripts/verify-commit.sh`.
- [x] F5 Record explicit conclusions and exact native handoff/host prerequisites.
- [x] F6 Review final scoped diff and evidence; preserve A–D/E and user changes.

## Final continuation state

All locally actionable F delivery is complete. Current complete-stack conclusion:
**当前依赖不支持**; native Windows ARM64 behavior **缺少原生验证**. The API route
is **可实现** as an official-source/QEMU-based inference, not this host's VM proof.

- SDK 207/207; libksys build-support 7/7; affected agent-session 18/18; corrected
  OCI Runtime full suite 424/424; affected scoped Clippy pass.
- Six actual build-entry checks pass their expected refusal/stub assertions.
  ARM core/SDK all-targets, ARM OCI Runtime all-targets, ARM OCI libraries,
  x64 OCI libraries and x64 libksys compile. ARM libksys explicitly refuses.
- `bash scripts/verify-commit.sh` passes, exit 0, 273.71 s: Runtime 2165/0/12
  ignored; CLI 933/0; all default contracts, check, fmt, Clippy, installer 7/7.
- Scoped source review and resource inventory pass; both native repositories
  and gitlinks unchanged, nothing staged. All original failure logs retained.

Exact commands/logs/identities and limits:
`docs/evidence/runtime/20261114-windows-arm64/OUTCOME.md` and `checks/*.json`.
The only failed build left is combined ARM shim all-targets: target lzma-sys
needs absent Windows C/CRT headers. Do not rerun without that external toolchain.

Next executable task: on an eligible native Windows ARM64 host with current
ARM64 MSVC/SDK, run `NATIVE-HANDOFF.md`'s architecture, capability and reversible
partition-object probe; retain HRESULTs, binary identity and cleanup. Then port
the selected VMM/firmware and ARM64 bundle in their owning repositories before
attempting guest/lifecycle qualification. This session has neither host nor
native dependencies. No further local change can qualify that runtime.

## Investigation history (earlier next steps are superseded above)

Queued after E. Existing `docs/architecture/windows-arm64-plan.md` contains
research to recheck, not native acceptance. Available host is macOS arm64;
installed Rust Windows target initially only x86_64-pc-windows-msvc.

Official Microsoft documentation and headers explicitly expose ARM64 WHP and
WHV_CAPABILITY_FEATURES.Arm64Support; QEMU ARM source uses the public WHv APIs.
The previous Vid-vs-public-API hypothesis is superseded. Rust ARM64 Windows MSVC
is Tier 1 with host tools. Current selected libkrun Windows vCPU implementation,
Windows firmware and both pinned DLL bundles remain x64 (PE Machine 0x8664).
No native dependency repository changed. Overall current-stack conclusion:
当前依赖不支持; actual API/VM behavior additionally 缺少原生验证 on a Windows ARM64 host.

Observed baseline: real cached libksys build script accepts Windows ARM64 in
stub mode; source dispatches by build-host cfg and falls back to x64 archives.
Baseline command/exit and archive hashes are under F checks.

Implemented shared SDK IsWow64Process2 admission for Box create/launch, Box shim,
OCI features/smoke, and native OCI context. Unsupported/emulated/unknown native
architecture refuses before VM mutation; capability status is Unsupported.
Implemented target/ABI rejection before stubs and Windows target dispatch in
libksys. Focused SDK architecture matrix (one test with nine refusal cases)
and native-target matrix (one test) pass on macOS. ARM64 Windows std installed.
Next: Windows ARM64/x64 compilation checks, actual build-entry rejection checks,
official-source report/native handoff, affected full tests/Clippy, final verify.

Available cross checks now pass: ARM core/SDK all-targets; ARM OCI runtime
all-targets; ARM OCI shim/runtime libraries; Windows x64 OCI libraries and
libksys. Native ARM libksys check intentionally exits 101 before assets/stubs.
The combined ARM shim all-targets check is externally blocked only by target
lzma-sys needing the absent Windows C/CRT headers (stdlib.h); failed log retained.
Do not repeatedly retry this toolchain absence or claim all-targets success.

SDK full unit suite 207/207 and libksys build-support 7/7 pass. Expanded owning
OCI Runtime suite found two stale fixtures: valid Mac report omitted the
existing required macos_host_sandbox_enforced field. Production validator is
unchanged. Valid fixture repaired; added missing/false/wrong-type refusal
coverage; existing digest assertion retained. Original full failure is
oci-runtime-full.log (421 pass / 2 fail). Next: focused agent_session tests,
full OCI rerun, scoped Clippy, actual build-entry self-checks and final verify.
NATIVE-HANDOFF.md and research.md contain concrete native API/partition steps.

Continuation: fixed agent_session suite passes 18/18; scoped affected Clippy
passes. Full OCI Runtime rerun and final scripts/verify-commit.sh are running;
their logs are oci-runtime-full-fixed.log and verification-entry.log. Actual
current cached build-script checks all match: ARM64 (with/without stub),
ARM64EC and x64 GNU refuse before link directives; x64 MSVC enters only the
explicit metadata stub; mismatched Unix host/target refuses before native work.
build-entry-admission.json records binary/source identities and all six exits.
Next: collect both full results, fix any failure, record final resource/diff
inventory, and finish F4–F6. Native Windows and the missing C/CRT remain external.

Full OCI Runtime rerun now passes 424/424 (371.57 s test time). Final default
verify remains in the contract 10000-replay durability test. Scoped review
recorded 19 file identities and the cumulative diff, no staged changes, clean
native repositories and unchanged gitlinks. No new production edit is pending.
After the default gate: inventory owned native resources, finalize evidence and
tasks; do not rerun the absent Windows C/CRT toolchain or claim native Windows.

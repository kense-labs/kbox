# Windows ARM64 feasibility — F

Date: 2026-11-14. Status: bounded F feasibility, local guards and checks complete;
remaining native qualification/port prerequisites are external.
Host: macOS 26.6.2 / arm64, rustc 1.97.1. No Windows host was used.
Base: 405c8b89f151e9e2879438d9e457dd60bfd18b90 plus retained A–D and E.

## Conclusion

**Current complete stack: 当前依赖不支持. Native Windows ARM64 behavior:
缺少原生验证.** The platform route is 可实现 as an inference from Microsoft's
ARM64 WHP contract and QEMU's public-WHv ARM implementation. The missing VMM,
Windows firmware/ABI-matched native assets and host evidence cannot be replaced
by Rust cross compilation or x64 emulation.

The prior hypothesis that ARM WHPX needs an unavailable public partition API
or a separate Vid route is superseded. Microsoft documents Arm64Support and
publishes ARM64/GIC definitions; QEMU uses those public WHv functions. See
[Microsoft WHP](https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/hypervisor-platform),
[QEMU WHPX](https://www.qemu.org/docs/master/system/whpx.html) and the
[layer-by-layer research](../../../../specs/005-windows-arm64/research.md).

## Changes and source findings

- Shared SDK IsWow64Process2 admission checks actual native machine, process
  machine and compiled architecture. ARM64, ARM64EC, x64-on-ARM emulation and unknown
  combinations refuse with Unsupported; query failure refuses Unavailable.
- Box create/launch, direct Box shim, OCI discovery/smoke and native OCI context
  use that same guard. OCI unsupported hosts advertise no isolation class and
  candidate opening preserves Unsupported. No VM mutation precedes admission.
- libksys checks Cargo target/ABI before stubs or archive discovery. Windows
  target dispatch no longer follows the build host's cfg, and other targets
  cannot silently consume kwindows-x64. Unix native-source builds explicitly
  refuse a mismatched host OS/CPU rather than selecting host binaries.
- The Mac-only SDK qualification example now has a real unsupported-platform
  entry on Windows; its Unix implementation is compiled only on Unix. This
  fixes a real ARM Windows all-targets compile failure without removing a test.
- Both native bundles have x64 PE machines (0x8664), and their firmware blobs
  expose x86_64 ELF headers (0x003e). The Windows vCPU source uses X64 registers,
  APIC/CPUID/MSR; current windows-sys Hypervisor bindings lack ARM64 structures.
- Native gitlinks unchanged: libkrun 3c2889c76460d81e3fb16d6144580fd31eb1bae3;
  libkrunfw fc3ea06895301f1df6a274dad75fbd9317b3bfee.

Evidence: `baseline-build-admission.json`, `native-archive-architectures.json`,
`firmware-guest-architectures.json`, `official-source-observations.json` under
`checks/`. The old actual build-script entry returned 0 for an ARM Windows stub;
that was compilation scaffolding, never native execution evidence.

## Actual checks

- SDK architecture regression: 1 executed hermetic test, nine refusal cases and
  a synthetic native-x64 tuple as positive control. `sdk-architecture-first.log`.
- Native target/ABI regression: 1 executed test. `native-target-admission-first.log`.
- `rustup target add aarch64-pc-windows-msvc`: success for the pinned toolchain.
- ARM Windows koci-core/SDK all-targets: pass after fixing the Mac example.
- ARM Windows koci-runtime all-targets: pass independently.
- ARM Windows koci-runtime/koci-kbox library check: pass, existing warnings.
- ARM Windows klibksys check: intentional exit 101 with the explicit unsupported
  dependency reason before archive/native work. This is refusal coverage.
- Windows x64 klibksys check: pass, selecting its Windows bundle on this Mac
  build host. Compilation only, no native Windows execution.
- Windows x64 SDK/koci-runtime/koci-kbox library check: pass, compilation only.

Exact commands, exits and log names are in `checks/cross-results.json`,
`checks/cross-results-after-example.json`, `checks/available-cross-results.json`
and `checks/final-focused-results.json`; failures remain separate from passes.

Original compile failures remain in `arm64-core-sdk.log` (Unix API in a Mac
example) and `arm64-oci-runtime.log` (target C/CRT toolchain missing). Full
combined all-targets koci-runtime/koci-kbox compilation is blocked by target
lzma-sys: clang cannot find Windows stdlib.h. The narrower independent library
check does not erase or count as a pass for that all-targets command. No retry
of the missing C toolchain is being used as a substitute for a real environment.

The actual current build-script entry now passes six assert-based checks:
ARM64 with/without stub, ARM64EC and x64 GNU refuse before link directives;
native-x64 ABI enters the explicit metadata stub; mismatched Unix source target
refuses before native work. These are build admission checks on macOS, not
Windows execution. `checks/build-entry-admission.json` includes exact build-script
and source hashes, target inputs, expected/actual exits and raw logs.

Full SDK unit tests pass 207/207 and libksys build-support tests pass 7/7.
Expanded OCI Runtime tests exposed two stale valid-report fixtures missing the
existing mandatory Seatbelt field (421 pass / 2 fail). The production validator
and digest assertions are unchanged; the fixture now includes the required
field and a new test rejects missing/false/wrong-type values. All 18 affected
agent-session tests and scoped Clippy pass. The full OCI rerun passes 424/424
(371.57 s test time). Original
failures remain recorded in `checks/oci-runtime-full.log`; the corrected suite
is `checks/oci-runtime-full-fixed.log`, exact commands/exits/times in
`checks/post-fixture-results.json`.

Final `bash scripts/verify-commit.sh` **PASS**, exit 0, **273.71 s**:

- Runtime **2165 pass / 0 fail / 12 ignored**, CLI **933 pass / 0 fail**.
- All default contract tests, including the more-than-10000 exact replay gate.
- Workspace all-targets compilation, format and scoped Clippy; existing warnings
  remain visible, with no new lint suppression or assertion removal.
- Support matrix **4 targets / 0 supported / 4 rejected / 1 required**, unchanged
  readiness; archive installer **7/7**.

Full log: `checks/verification-entry.log`; command, UTC start, elapsed time and
exit: `checks/verification-entry.json`. Shared macOS platform/shim tests and
real HVF regressions remain in [E's outcome](../20261114-macos-readonly/OUTCOME.md).
F changes Windows admission/target selection and evidence fixtures; it does not
claim another native macOS binary qualification or any native Windows run.

`checks/scoped-source-review.json` hashes the 19 touched source/docs files,
records the cumulative dirty diff review, clean native repositories, unchanged
gitlinks and no staged changes. `checks/resource-inventory.json` confirms empty
private execution homes, no owned VM process and no added RAM device after the
native default-test portion. The seven earlier foreign fixture devices remain
untouched. Source trees, outputs and frozen qualification artifacts are retained.

## External prerequisites and next executable task

The current session has no eligible native Windows ARM64 host. A new VMM/firmware
port and an ARM64 Windows native runtime bundle must be implemented/tested in
their owning repositories before this parent can select a fifth bundle.
Full Windows C-dependency builds also need a Windows SDK/CRT and native MSVC
toolchain; this Mac has the Rust target but no such target sysroot.

Next: run the capability/partition probe and retain its exact executable,
compiler/OS/CPU, raw output, HRESULTs and cleanup on an eligible native ARM host,
then port the dependency with that host's feedback. See [native handoff](NATIVE-HANDOFF.md).

## Explicitly not claimed

No Windows ARM64 or Windows x64 native runtime result, guest boot, supported
readiness promotion, native ARM64 DLL/kernel bundle, emulation validation or
alternate-backend implementation. No native submodule edit, commit, push,
publication or real ~/.kbox mutation. E remains separately qualified on macOS.

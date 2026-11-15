# Implementation Plan: Windows ARM64 feasibility

Spec: `spec.md`. Execute after E; reuse existing platform policy and support matrix.

## Constitution check

I/V/VIII: typed native architecture refusals, no emulation/backend fallback.
II: no resource or lifecycle ownership change.
III/IV/X/XI/XV: official/source evidence, adversarial tests, retain failed builds;
cross checks never count as native runtime evidence.
VI/VII/IX/XIII: same workspace/KCL, exact identities, synthetic fixtures.
XII/XIV: native repositories remain unchanged; native host required to qualify.
All principles pass; absence of a host bounds claims, not local progress.

## Scope and sequence

1. Recheck current official Microsoft/API, Rust/toolchain and libkrun sources.
2. Trace capability and build admission through Box, libksys and OCI.
3. Reproduce architecture gaps; fix shared causes and callers with small tests.
4. Run available host/cross checks, scoped Clippy and full verification.
5. Record a layer-by-layer conclusion, remaining prerequisites and executable
   native handoff. Update existing architecture documents/matrix as necessary.

No new runtime bundle or dependency is admitted without native architecture
and provenance evidence. Existing Windows x64 assets remain x64 only.

## Implemented boundary and current verification

Reuse `koci-sdk::admit_windows_whpx_host` across Box host class/launch, direct
shim entry and OCI capability/context admission. It uses the existing Windows
dependency's typed IsWow64Process2 binding; no product config or lifecycle state
is added. Target/ABI admission in libksys precedes stubs/assets and uses Cargo
target values rather than build-host cfg. Unix source cross builds fail closed.
The SDK export, Windows SystemInformation feature, all callers, architecture
matrix tests, build-entry tests and the Mac qualification example's platform
guard are in scope. Existing routes and generation/receipt authority remain intact.

ARM Windows core/SDK all-targets and OCI libraries compile. The combined shim
all-targets check reached a real target lzma-sys C dependency and is externally
blocked by the absent Windows C/CRT sysroot. Its failed log is retained; other
independent checks continue. No C dependency or assertion is removed to pass it.
Native dependency target rejection and native x64 target selection are verified
separately. The official-source/native handoff is in the F evidence directory.

## Final acceptance

Shared SDK tests 207/207, libksys build-support 7/7, corrected OCI Runtime full
suite 424/424 and scoped affected Clippy pass. All six actual build-entry cases
match their expected refusal/stub outcomes. Final `scripts/verify-commit.sh`
passes (273.71 s): Runtime 2165/0/12 ignored, CLI 933/0, all default contracts,
workspace all-targets, format, scoped Clippy and installer 7/7. Source review and
native resource inventory pass; native gitlinks/repositories remain unchanged.

The locally actionable F deliverable is complete. Further Windows ARM64 work
needs an eligible native host, SDK/CRT/MSVC and a VMM/firmware/bundle port in the
owning native repositories. The missing target C sysroot remains a failed
combined shim all-targets check; it is not reclassified as a pass. Follow the
retained native handoff rather than repeating that absent-toolchain build.

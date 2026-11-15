# Feature Specification: Generic MicroVM release baseline

Created: 2026-11-14. Status: baseline implemented and verified on macOS arm64. Owning paths: crates/box, crates/runtime, scripts.

## User scenarios and testing

1. Resource updates preserve each requested value and leave unrelated OCI controls unchanged. Existing resource_update_contract tests plus malformed-limit cases are the independent gate.
2. Capability discovery never promises unsupported MicroVM StateSnapshot operations. Explicit unsupported configuration is refused before any resource or filesystem mutation.
3. Default Runtime/CLI/contract tests and the checked-in verification entry point pass without reviving removed host-Sandbox product semantics.
4. A release archive contains only matching, identified runtime companions and installs into an isolated prefix. Failed installation preserves the previous prefix.
5. Final archive native checks prove lifecycle, streaming, prewarm, generation refusal and zero managed resource leaks on the actual qualified host.

## Requirements and ownership

ExecutionManager remains the sole resource authority. Runtime owns durable operation receipts. Caller-defined profiles stay opaque. Application installers, schemas and workload classification remain outside this repository.
Invalid limits, unsupported features, stale identity, damaged archives and missing companions fail before unsafe effects. Existing state and evidence are never overwritten to obtain a pass.
The root workspace and KCL policy contract remain unchanged. No new backend, dependency framework or product configuration is required.

## Success criteria

Default Runtime/CLI/contract tests have zero failures; workspace all-targets check, Clippy, format and support matrix pass. The final archive has retained companion hashes and isolated-prefix installation evidence. Native claims name their actual host and image.

## House checklist

- Hermetic tests accompany behavior changes and include refusal cases.
- Existing lifecycle locks, exact execution identity and generation fencing are preserved.
- Native dependencies remain in their owning repositories; no submodule edit is planned.
- Evidence: docs/evidence/runtime/20261114-release-baseline/OUTCOME.md.

## Explicitly not claimed

No automatic v1 journal migration, shared-host-kernel Box product, application-specific integration, memory snapshot implementation, external publication or unsupported-platform promotion.
macOS read-only integration and Windows ARM64 feasibility are separate subsequent milestones.

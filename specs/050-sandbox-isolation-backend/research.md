# Research: 050 sandbox isolation backend

## Upstream scout

- Designated Apache-2.0 upstream at commit `2d6c2d6` (local scout checkout used
  only for fusion; not a second product).
- Size: ~18 MB tree, ~77k LOC Rust across crates; edition 2024 (matches kbox).
- Model: Docker Engine API daemon → vsock → guest agent (PID 1) → `clone3`
  namespaces/cgroups. macOS: Apple Virtualization.framework
  (`kbox-sandbox-vmm`). Linux: agent on host kernel (no VM). Windows: WSL2 +
  host bridge binary.

## kbox today

- Runtime `IsolationLevel::{Process,Container,Sandbox,Confidential}` exists.
- Box historically mapped Sandbox → ordinary MicroVM (`TeeConfig::None`).
- `ExecutionBoundary` becomes a real selector under Spec050 (`Sandbox` | `Microvm`).
- `ManagedRuntimeRoute::{Unspecified,BoxVm,Sandbox,OciSdk}`.
- CLI historically defaulted to MicroVM isolation.

## Fusion decisions

| Decision | Choice | Why |
| --- | --- | --- |
| Import shape | Vendor into `crates/box/sandbox/*` | One lockfile |
| Default flip timing | After sandbox can run OCI smoke | Fail-closed |
| Brand | Zero upstream product-brand strings in code/docs | GOALS + user R5 |
| VMM stack | Virtualization.framework for sandbox; libkrun for MicroVM | Isolation grades |
| Authority | `LocalExecutionBackend` under ExecutionManager | Constitution II |

## Risks

- Duplicate Docker/OCI surfaces — route by isolation.
- Lockfile dependency collisions — resolve in root lock only.
- Unrelated dirty tree: `049-pool-oneshot-discard` — do not edit those files.

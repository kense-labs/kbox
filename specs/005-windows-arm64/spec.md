# Feature Specification: Windows ARM64 feasibility and honest admission (F)

Created: 2026-11-14. Status: bounded feasibility/local guards complete and verified;
native qualification blocked by the documented external prerequisites.
Owners: platform probes,
`crates/box/deps/libksys`, OCI adapters, support documentation.

## User scenarios and independent tests

1. Windows ARM64 discovery and launch never report x64 WHPX assets as native
   ARM64 support. Unsupported CPU/ABI combinations refuse before resource use.
2. An implementer can follow source- and official-reference-based findings
   covering native virtualization API, VMM, firmware/guest, ABI, tools and DLLs.
3. A future Windows ARM64 operator has exact native handoff commands and
   expected evidence. Cross builds remain explicitly distinct from native runs.

## Requirements and ownership

Conclude `可实现`, `当前依赖不支持` or `缺少原生验证` from actual evidence for
each layer and for the current stack. Implement locally justified architecture
checks, probes/refusals and tests. Preserve Windows x64 behavior and all A–D/E
gates. No backend switch, emulation-as-native claim or submodule source edits.

## House checklist and success criteria

Use the root workspace, existing typed capabilities and ExecutionManager;
no duplicate lifecycle/configuration. Each behavior change has focused tests.
Official sources have URLs and access date; local findings name gitlinks and
source lines. Run available build checks and `scripts/verify-commit.sh`.
Evidence: `docs/evidence/runtime/20261114-windows-arm64/OUTCOME.md`.

## Explicitly not claimed

No Windows ARM64 runtime support without actual eligible host evidence.
No native VMM/firmware port or fake ARM64 archive made from x64 dependencies.

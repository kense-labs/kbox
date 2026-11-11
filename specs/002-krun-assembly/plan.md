# Implementation Plan: KRun environment assembly

**Spec**: [spec.md](spec.md)
**Owning path**: Box Runtime, shared Runtime contracts, independent KRun.

## Constitution Check

I/V/VIII: typed references, explicit capability discovery and refusal; II/X:
ExecutionManager effects and MRC observations stay authoritative; III/IV/XV:
retain failed evidence and run owning/native gates without weakening assertions;
VI/XII: existing workspace, no KRun production path dependency or native gitlink
change; VII: KCL; IX: secret references and memory-only delivery; XIII: identities
come from receipts/manifests; XIV: claim only native-tested configurations.

## Research and Reuse

- `kbox_contract_driver/artifact.rs`: keep immutable Artifact admission; add
  separate directory port using existing storage-path validation/mount wiring.
- `ExecutionSessionManager::start_process`: stream archive bytes to trusted
  guest tooling; do not add a second exec provider or buffer archives in IPC JSON.
- KRun `package.rs`: reuse bounded ZIP/checksum/link validation in guest extraction.
- `engine/`: generation-bound assembly intent/receipt and original MRC lease.
- Existing pool/ExecutionManager paths: compatible prewarm and dirty-generation
  retirement; never execute Harness on host or fallback to Docker/OrbStack.

## Delivery Sequence

1. Directory reference contract, capability, authorized port and native grant test.
2. Shared base and KRun guest installer; archive-private atomic extraction tests.
3. Engine MateHarness orchestration/receipt, replay and stop/replace fencing.
4. Compatible prewarm and secret delivery/refresh ownership.
5. Formal CLI, three package/model closures and ordinary OCI regression.

## Verification

Run owning focused tests, fmt and Clippy; workspace check for contract changes.
Native HVF tests use isolated homes/prefixes and retained artifact hashes. KRun
keeps its own independent test/evidence output. Linux KVM/WHPX remain unclaimed
until their native tests run. Known baseline test/format failures stay visible.

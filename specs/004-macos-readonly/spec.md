# Feature Specification: macOS host-enforced read-only mounts (E)

Created: 2026-11-14. Status: implemented and qualified on the named macOS arm64 host. Owners: `crates/box/runtime`,
`crates/box/shim`, `crates/oci-runtime/crates/kbox`.

## User scenarios and independent tests

1. A macOS MicroVM reads a caller directory declared `:ro`; the host denies
   create, overwrite, append, truncate, unlink, mkdir, rename and replacement.
   A privileged guest remounting the share writable must still receive a host
   denial. A separate `:rw` directory and the host owner's own writes still work.
2. Symlinks, directory replacement, overlapping grants and child execution
   cannot widen the write boundary. Native disposable-process tests exercise
   the enforcement primitive independently of guest mount flags.
3. New and retained executions use the same platform enforcement. Persisted
   BoxVm/OciSdk routes, execution identities, recovery and generation fencing
   remain authoritative. Routing tests cover mixed retained records and errors.

## Requirements and ownership

- Reuse OCI's macOS confinement and platform adapters at the actual libkrun
  process boundary; do not invent another resource authority or state store.
- Admit `:ro` only when native host enforcement is installed before VM entry.
  Unsupported combinations return typed errors, with no writable/backend fallback.
- Keep live caller directories live and caller-owned; no recursive chmod,
  disk-image conversion, copy presented as a live directory, or caller cleanup.
- Validate actual grants, aliases and resource paths at the execution boundary.
- Preserve A–D changes and evidence. No submodule content changes, commits,
  publication, real `~/.kbox` access or business-specific behavior.

## House checklist

- Hermetic/adversarial tests and native host tests are required.
- ExecutionManager, durable route and generation remain unchanged.
- Closed typed input/error contracts; KCL remains the product configuration.
- No real secrets; native fixtures use synthetic data in isolated directories.
- Evidence: `docs/evidence/runtime/20261114-macos-readonly/OUTCOME.md`.

## Success criteria

Native host denial and real HVF guest bypass tests pass with source/artifact
identity and clean resource inventories. Focused affected-package tests and
`scripts/verify-commit.sh` pass. Failures remain in evidence.

## Explicitly not claimed

No supported-readiness promotion, new isolation backend, automatic migration of
retained records, or Windows/Linux native qualification. Sharing the platform
enforcement boundary does not by itself claim a full OCI service cutover.

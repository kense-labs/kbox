# Bounded memory-backed Secret files on explicit Mac OCI

Created: 2026-11-15. Status: bounded phase accepted; installed evidence in OUTCOME.md.
Owning paths: Box Runtime,OCI SDK/Runtime/Kbox. Existing user W4 scope.

## User scenarios and testing

P1: An already-authorized Box-managed regular Secret file on a validated macOS
ram:// volume reaches one exact readonly Guest file through ExecutionManager,
persisted OciSdk,public SDK and independent Host. No parent/sibling content,
no plaintext disk fallback,source/route/generation identity retained. Reuse
existing Box transient store and live file grants. Preserve actual source mode
and numeric ownership; do not invent ownership translation. No Secret bytes in
configuration,receipts,Debug or evidence. Synthetic fixtures only in tests.

P1: Unsupported backing or old Host refuses before workload entry. Required
versioned memory-file extension binds exact RO file grant indices. Source and
file-projection backing are verified at actual Host/shim boundary. Existing
runtime-root configuration must be on the same RAM filesystem for this bounded
profile; ordinary file v1 behavior and physical-disk refusal remain unchanged.
Restart/stopped recovery preserve exact identities; cleanup detaches runtime
aliases and lets existing Box Secret owner retire only its own source.

## Requirements and adversarial cases

Shared required extension,strict bounded parsing,exact file grants,readonly only,
RAM provenance verified from actual mounts/disk-image identity,old Host refusal,
disk source/projection rejection,links/path replacement/alias/cross-filesystem
and conflicting classifications refuse. Test actual Host RO remount/child effects,
Guest access as actual owner,independent instances,reconstructed clients,CLI
where applicable,replay/fencing,normal and stopped recovery,failed preparation
and source-preserving cleanup. Final archives must be rebuilt/installed and
verified with actual test counts and owned RAM/device/FD/process cleanup.

Environment Secret projection is separate remaining W4 work and must refuse
rather than pass its manifest untreated to the OCI workload. This slice does not
claim all Secret parity,full W4/default cutover,live continuity or promotion.

## Ownership and house checks

ExecutionManager remains resource authority; public SDK carries classifications
only. Box owns authorization/materialization/retirement. No new state/config,
workspace/dependency/native source changes. Constitution I–XV apply without
waiver; hermetic adversarial tests and actual installed native effects required.
Evidence: docs/evidence/runtime/20261115-oci-secrets/OUTCOME.md.

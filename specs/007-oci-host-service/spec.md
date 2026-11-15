# Specification: Box to independent macOS OCI Host Service

Status: accepted for the explicit bounded profile (2026-11-14). Scope: the explicit macOS arm64 DedicatedVm migration path.
Start from retained A–F/final-archive changes; no default route/readiness promotion.

## Acceptance

Real chain: ExecutionManager -> persisted OciSdk -> public SDK -> independent
same-UID macOS Host Service -> HVF. Retain a real BoxVm record beside OciSdk;
reconstructed clients and changed create policy preserve exact route, binding,
identity and generation. Wrong/missing bindings and old generations refuse.

Reuse service recovery/owner-replacement gates for SIGKILL, committed response
loss, duplicate/concurrent requests and client disconnect. A lost session may
be explicitly invalidated by the existing stopped-only protocol; do not invent
continuity. Check real I/O, file/directory grants, host RO attacks, mounts,
resource admission and cleanup. Unsupported operations refuse before effects.

Record fixed executables/native assets/system image/manifest, actual test counts,
failures, cleanup and final affected installation-package qualification. Preserve
all history, one resource authority and current isolation/readiness semantics.

## Not in scope

Windows host waiting, VMM/firmware ports, shared-host-kernel revival, default enum
flip, business special cases, user ~/.kbox, native source edits, new workspace,
unnecessary dependencies, commit/push/publication or global W4 certification.


## Accepted profile and limitations

The final scope is same-UID Linux arm64 directory rootfs, one vCPU/512 MiB,
network none, plain caller directories, typed resource limits and explicit guest
SYS_ADMIN. Other combinations fail closed; refusal does not claim feature parity.
New Exec operation receipts use schema v7; legacy v1–v6 read/validation rules
remain. Host owner death recovers stopped state, explicitly invalidates old
sessions and permits a new-generation restart. Default BoxVm is retained.

Actual final counts, assets, failures and cleanup are in
[OUTCOME](../../docs/evidence/runtime/20261114-oci-host-service/OUTCOME.md).

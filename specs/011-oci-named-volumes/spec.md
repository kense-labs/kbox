# Named VolumeStore mounts through explicit HVF

Status: accepted bounded slice; final installed Box1/OCI1, native matrices and
scoped cleanup recorded in the outcome. Spec010 history retained.
Owner: Box Runtime. Existing W4 allocation parity slice, not full W4 or GA.

Support Box-owned named VolumeStore directory mounts through persisted OciSdk,
public storage Caller/DetachOnly attachments and independent macOS HVF Host.
Use exact named storage identities and existing Box attach/detach ownership;
runtime deletion must retain volume metadata/data and other executions.

Independent acceptance:
1. Existing named volume RO/RW mounts start and persist data across normal
   restart, reconstructed clients, CLI use and stopped Host recovery. Verify
   storage identities, routes and exact generation rejection in real assets.
2. Two executions share one volume safely; removing one keeps other references
   and data. Named volume remains after all execution cleanup. Only the test
   owner may remove its explicitly created final fixture through VolumeStore.
3. RO denies actual Host writes after privileged remount/fresh mount/child,
   including after restart. Live data/RW writeback are real, not staging copies.
4. Missing/corrupt/inconsistent catalog, wrong name/path/identity, aliases,
   unbound declared volume, quotas/driver overrides that cannot be enforced,
   stale generation and preparation failure refuse safely with preserved data.

Reuse VolumeStore, ExecutionResourceGuard and oci_storage_attachments. No new
allocation state machine, config, workspace/dependency or native source edit.
Anonymous image VOLUME and Secret lifetimes stay separate open slices. Retain
all default/legacy route/Exec/rootfs/file-grant behavior and evidence. No default,
support/readiness promotion, real ~/.kbox, commit/push/publication or Windows port.

Success: before regression, focused/owning gates and verify-commit; rebuild and
isolate-install affected assets, actual native named/file/rootfs/Host/A–F replay,
owned resource audit and outcome under
docs/evidence/runtime/20261114-oci-named-volumes/OUTCOME.md; then next W4 local item.

# Spec044: retain returned legacy handlers before installation wait

Legacy boot and rootfs-maintenance start await handler.write before storing the
actual returned provider handler. Cancellation there drops the returned owner;
selected original-owned start already uses pending_handler. Reuse that existing
actual pending slot for legacy start/publication, refuse conflicting occupied
slots without overwrite, and ensure default destroy/boot-failure cleanup adopts
pending actual handlers before physical finalization or resource deletion.

Keep scope bounded to an already returned handler in a retained VmManager. No
claim for provider-internal cancellation, outer pool/legacy/unpublished manager
drop, from_pid reattachment or complete lifecycle cutover. No new state/config/
dependency/workspace/native source edit; preserve authority/routes/generation.
Acceptance: real production helper red with bounded owned child, cancellation/
reentry/conflict/data/physical close controls, owning/full checks, frozen rebuilt
fresh installed native VM handoff/actual cleanup+AF/pool/audit, current evidence
and scoped authorized commit/bundle, then next required local ownership gap.

# Spec041: exact pool rollback identity

Owning path: Runtime pool/warm_pool.rs::remove_idle_vms and its fill caller.
Current rollback snapshots matching indices under one idle lock, releases it,
then removes those indices under a later lock. Concurrent membership changes can
retarget cleanup to another actual VM and delete its working data. Select/remove
only requested exact BoxIds in one critical section, keep counts coherent and
register actual selected owners before any later asynchronous wait. A VM already
handed to another caller is no longer an idle rollback candidate. Preserve typed
close failures/retry and selected-close isolation; no lifecycle/config/dependency
/workspace/route/generation changes.

First establish deterministic actual implementation/data-deletion red using real
VmManager handles and FIFO mutex scheduling, then fix shared selection. Native
installed fixture must prove intended target closes and unrelated actual VM/
allocation/caller data stays live until owned final cleanup; run affected owner/
default/Clippy/fmt/frozen archive/AF/pool/owned audit. No mocks as native proof.

Scope is selected fill rollback. General drain/eviction/template detach-to-
registration waits, internal boot/allocation/unpublished/legacy lifetime and
publication/capacity race remain continuing work. Full W4/default OCI cutover/
live continuity/GA/other-host/nativeWindows/Linuxsnapshot/RSS are not claimed.

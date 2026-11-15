# Bounded custom disk capacity on macOS OciSdk

Status: accepted on this existing macOS arm64 host; [final installed evidence](../../docs/evidence/runtime/20261115-oci-disk-capacity/OUTCOME.md).

Carry existing Box `resources.disk_mb` through the macOS OciSdk writable root
disk and image-VOLUME anonymous allocations. Reuse the existing ext4 capacity
range (16 MiB..64 GiB), builder, rootfs-image/raw-storage descriptors and owner.
No new configuration, resize API, quota mechanism, native dependency or state.

Acceptance: invalid capacities refuse before preparation/claims; valid values
preserve exact bytes through the builder, pinned Host disk and Guest block/FS.
Actual bounded Guest ENOSPC and unlink/reuse prove capacity behavior, with
distinct writable instances, anonymous seeds, replay, CLI, restart and stopped
Host recovery/generation fencing. An image that cannot fit must fail and clean
only its own partial resources, retaining input/cache bytes. Other platforms,
CPU/memory sizing, lifetime timeout, ephemeral quotas and other controls retain
their existing refusal. Default capacity/serialization/route stays unchanged.

Freeze/build/fresh install affected Box assets; retain the unchanged OCI/Agent
with honest original source identity and fresh extraction. Run focused/owning/
default and affected installed native/A–F gates, verify actual counts/results,
audit exact homes/disks/volumes/processes/mounts/sockets/FDs and source preservation.
Full W4/default cutover/live continuity/platform promotion stay open; Windows
ARM64 dependencies unsupported/no native run; RSS unmeasured. Continue next W4.

Existing lifetime contract: [spec008](../008-oci-rootfs/contracts/rootfs-image.md)
rebuilds the ephemeral root at each generation. Root writes must work before
restart and be absent afterward; anonymous-volume data/owner/links/capacity must
persist. This slice does not change that contract or claim durable root writes.

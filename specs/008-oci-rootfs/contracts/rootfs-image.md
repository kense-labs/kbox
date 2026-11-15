# Rootfs image transport contract

## Public SDK

`dev.kbox.rootfs-image` v1 is a required extension automatically derived from
the same OCI annotation. Its bounded JSON value contains `schemaVersion: 1`,
`sha256` (lowercase algorithm-prefixed SHA256), `size` (existing 16 MiB–64 GiB,
4096-aligned ext4 capacity policy), and `imageDigest` (verified OCI manifest).
The ordinary OCI root remains the exact writable relative `rootfs` directory.
Old Hosts lacking the extension refuse before VM launch; no directory fallback.
The fixed operation input is `rootfs.ext4`, a private single-link regular file.

## Host / shim / Agent

Agent manifest `koci.agent-vm-attachments.v5` adds `rootfsImage` with that image
descriptor and `sourceIdentity` (`device`, `rawDevice`, `inode`, `size`). Existing
manifest target, generation, config digest and attachment digest bind the disk.
It consumes the existing storage count bound. Earlier v1–v4 manifests omit the
field and keep their wire encoding. Exec v1–v6/v7 storage/replay is unchanged.

Existing handoff ownership transfers the file to `<generation>.rootfs.ext4`, a
private sibling outside the exact generation's exported share. Both input and
transferred file existing is a conflict. Hash/size/link/owner/type or manifest
identity drift refuses; a pre-manifest interruption can be revalidated or
cleaned. Existing bounded manifest reads and identity-bound removal are reused.
There is no additional lifecycle store or shared writable artifact cache.

The shim pins a writable no-follow descriptor, verifies the content and exact
inode before VM entry, and calls existing krun_add_disk2 for a distinct raw
workload device. Seatbelt grants only file data access, with no writable parent
or virtio-fs export. Agent requires exactly one matching writable serial/size,
then mounts ext4 on the pinned empty OCI root with nodev/nosuid. System-image,
Agent and Box guest-init identities stay separate. Capability ceiling and Host
directory PEP are unchanged. This profile requires an authorized caller directory
(Box supplies workspace); raw caller storage, network and reusable sessions refuse.

## Lifetime and CLI

Each operation assembles a fresh instance from verified immutable OCI blobs.
An explicit restart advances generation and rebuilds the ephemeral workload root.
Stopped recovery authenticates the retained record; it does not revive sessions.
Cleanup checks target/config/disk identity and removes only the reaped generation.

The existing `rootfs_snapshot.json` diff baseline is derived from the validated
logical namespace before launch. CPU work occurs in the blocking builder; only
its awaited result is published through the existing first-writer-wins installer.
This is the existing mode/size/directory map, not a new state or config schema.

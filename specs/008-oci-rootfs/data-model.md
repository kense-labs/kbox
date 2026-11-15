# Data model

`RootfsImage` v1 describes the initial operation disk (`rootfs.ext4`), SHA256,
capacity and original OCI manifest digest. The OCI config digest binds the
required extension. Agent manifest v5 binds the exact target/generation/config,
private disk source identity and libkrun-derived guest serial. It is separate
from raw caller storage and immutable system boot assets.

The existing bundle handoff transfers this private operation disk to a sibling
outside its guest share. There is no retained immutable ext4 seed or new cache:
verified OCI blobs are the immutable source; each operation builds a distinct
writable disk. Mutable guest bytes are not mistaken for pristine input on replay.
A pre-manifest interruption is validated/recovered; a changed or replaced disk
is refused. Existing bounded readers and identity-bound cleanup remove only the
reaped owner. The CLI baseline uses the existing rootfs_snapshot.json map and
first-writer installer, not a new lifecycle record.

v1–v4 manifests omit the new field and retain encoding/meaning. Persisted BoxVm
and directory OciSdk records keep exact identities/routes. Exec v1–v6/v7 is
unchanged. Restart creates a fresh ephemeral workload root in a new generation;
Host recovery is authenticated stopped-only. Default routing/readiness do not change.

# Ordinary caller file grant v1 (implementation pending native acceptance)

Required SDK extension `dev.kbox.file-grants` v1 binds exact OCI mount index,
allocation ID, source device/inode, access, Caller ownership and DetachOnly cleanup.
Annotation and work are bounded; old Hosts must refuse. Agent manifest v6 carries
file grants cumulatively with directory/rootfs transport; v1-v5 meanings remain.
The file tag derives from the complete OCI configuration digest and allocation;
manifest additionally binds target/generation and full SDK attachment digest.

Box reserves source IDs through its existing resource plan in managed metadata,
with no second lifecycle authority. Restart retains the original source IDs and
never automatically authorizes a new file occupying the pathname. A source path
replacement during preparation refuses; after entry the original inode remains
the live granted object. Its new pathname is not automatically granted.

An exact private tempfile directory owns unpublished aliases until its blocking
result is adopted. Nonblocking flock prevents retry or cleanup from adopting an
in-flight preparation; cancellation drops the temporary directory and its links.

The runtime creates one hardlink named `file` inside one private projection per
grant, outside the Guest writable runtime share. Source must be a canonical plain
caller-owned regular file, initially one link, on the same filesystem. Existing
aliases, overlapping directory grants and competing source grants refuse. No
copy fallback. Setup/revocation changes link count and ctime but does not chmod,
chown or alter source bytes. Only explicitly authorized RW/host operations alter
data. Revoke removes private aliases, never resolves/deletes caller pathnames.

Shim independently pins source and projection, requires exact IDs and two links,
verifies again at VM entry and retains descriptors until exit. Source pins are
readonly; existing writable descriptors for RO source IDs refuse admission.
Darwin does not reliably revoke pre-sandbox open writer authority; rejection of
that combination is a required safety boundary, not a revocation claim.

Host PEP: RO permits file reads; RW additionally data write/truncate. Caller
parent grants metadata traversal only, no listing/read/create/unlink/rename.
Private projection listing exposes only the alias. Link/unlink/rename/chmod/chown
and sibling creation remain denied even on writable Guest remounts/fresh tags.
Guest mount flags are defense in depth; actual Host denial is required evidence.

Not claimed: cross-filesystem grants, existing aliases, simultaneous grants of
one source inode, unsafe overlap, special devices, foreign-owned host files,
unrestricted metadata writes, live HVF recovery, complete W4 or default cutover.

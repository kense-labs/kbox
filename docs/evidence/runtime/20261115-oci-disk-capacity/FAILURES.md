# Spec028 retained failures

No acceptance claim before actual tests and installed-asset verification.

## Admission red1

`cargo test --locked -p kbox-runtime --lib custom_disk_capacity -- --nocapture`
ran3 tests:2 passed/1 failed/0ignored (23.53s wrapper). The valid-capacity test
failed on the existing custom-disk blanket refusal. Boundary/control/other-profile
tests passed. Logs and receipt are retained in checks/disk-admission-before1.*.
The shared Mac validation now reuses Ext4ArtifactOptions::from_disk_mib; the old
nondefault-capacity refusal remains on other profiles and all other controls.

## C probe build1: optimized assertions disabled

Zig cc -O2 defines NDEBUG; strict -Werror caught uninitialized/unused values
when standard assert expressions were removed. No probe ELF or native claim
was produced. Source1 and failure are retained. The synthetic test probe now
explicitly undefines NDEBUG before assert.h, so validation and syscall effects
always execute under the same optimization. No warning or assertion disabled.

## Native fixture draft1: missing FlushAck arm

The Rust compiler refused the new stream collector because ExecEvent also
contains FlushAck. The collector now explicitly consumes that existing protocol
ack while retaining chunk routing and exact exit0/timeout checks. The compiler
failure is preserved; no native test or resource had started.

## Backend filter1 executed zero tests

The file-name-based filter local_execution::oci_backend_tests:: matched no
Rust module names: exit0,0/0/0. This is retained as a failed verification attempt,
never a passing regression claim. The frozen test binary --list and previous
actual receipt identify the owning module before the corrected run.

## Native1: foreign-owner helper assumed root DAC capabilities

Actual0/1/0,24.74s native/26.33s wrapper. Raw128MiB root+3anonymous sources,
all five process paths, actual ENOSPC28 for root and anonymous volume and
unlink/reuse passed before the reused foreign-rootfs helper tried to stat
UID1001-owned0700 content as empty-capability root. The kernel correctly
returned EACCES. No capability is added. The bounded fixture now verifies the
original owner UID1001:2002 read/write/mode/links, another UID denial, foreign
GID sharing and root-with-empty-caps refusal explicitly; recovery retains the
same owner checks. Cleanup-boxes is[], candidate1/logs and Guest observations
are retained. Rust fixture/source/client/Box2 and verify2 must be regenerated.

## A–F launch2: missing evidence directory

The runner caller omitted mkdir for checks/af-final1; it failed before spawning
the first OCI scenario (zero scenarios). The empty af-final1/work artifact root
and failure receipt are preserved. A fresh af-final2/checks path and context3
are created before retry; no product source, assertion, archive or gate changes.

## Native2: incorrect persistent-root test expectation

Actual0/1/0,41.05s native/45.28s wrapper. Capacity, ENOSPC/reuse, explicit owner/
other-UID/foreign-GID/root-without-caps checks and independent256MiB creation
passed. The CLI restart correctly rebuilt the ephemeral root, so its temporary
root marker disappeared. This is the explicit spec008 rootfs-image lifetime
contract (and OUTCOME008 lines36–38,74–76), not a product regression.
No data-retention feature is silently added and no old semantics change.
Fixture3 must verify root data before restart, exact root reset afterward and
real anonymous-volume persistence as a separate positive control, including
independent256MiB anonymous disks/CLI and primary128MiB three generations.
The failed persistent-root assumption and candidate2/logs are retained;
cleanup-boxes is[]. Source/client/Box3/verify3 required after Rust fixture edits.

## Native3: generation-local block transport identity

Actual0/1/0,123.68s native/127.92s wrapper. CLI root reset+anonymous persistence
and8GiB sparse/Guest capacity controls passed. Comparing all raw manifest fields
across restart incorrectly included blockId, which is derived by the protocol
from the new attachment digest and immutable allocation ID. Actual g1/g2 JSON
diff shows only blockId changed; source device/inode/size, allocation ID,
mount digest/order, access and ownership are identical. Fixture4 now asserts
blockId MUST change, and ALL remaining fields MUST match exactly. This retains
strong exact-identity checks and adds the proper generation-local fence; no
product or protocol changes. Source3/candidate3/failure preserved, cleanup[].
See agent-protocol/src/vm_attachment/storage.rs derive_block_id/validate.

## Audit3: copied-file GID expectation

All13 actual home/volume inventories, processes/mounts, cache and runtime disk
checks passed. The added probe-copy audit incorrectly expected fs::copy-created
files under private/tmp to inherit the artifact ELF group20. Actual destination
parents and copies have group0, owner501 and mode0755; byte/inode/link checks
passed. A standalone native Rust fs::copy control on each exact sentinel-owned
parent records this creation behavior and removes only its new control file.
Audit4 keeps UID/mode/hash/pre-entry inode/nlink checks and checks GID against
the observed native creation parent. No product source or test result changed.
No per-copy pre-admission GID snapshot was captured; this audit does not invent
one. Original synthetic ELF metadata and all file-grant regression evidence
remain separate. The failing audit/partial observations are retained.

## Review3: OsString evidence representation

The actual Rust CLI evidence serializes std::ffi::OsString as the Serde Unix
byte-array variant, not a JSON string. Review3 incorrectly passed that object
to pathlib.Path and failed. Review4 asserts exactly the Unix variant and exact
installed4 CLI path bytes; all actual native/asset/count/cleanup checks remain.
No product source, artifact, test or acceptance behavior changed.

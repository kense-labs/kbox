# Reuse current volume and disk contracts

Current gap: dedicated Mac planner only captures caller files; ext4 preparation
rejects image VOLUME; OCI completion refuses extra mounts. Existing VolumeStore
claim_anonymous, deterministic plan_anonymous_volumes, durable BoxRecord plan,
resource cleanup and preparation identity validation already define authority.
Existing SDK storage-v2/AgentVmStorageAttachment/raw ext4/serial verifier/native
krun_add_disk2 are implemented for other transports. Mac hvf_directory currently
assumes all remaining grants are directories; shim explicitly rejects storage.

Use anonymous allocation directories only as Box-owned containers for the raw
ext4 data file. Derive its pristine contents from existing OciWorkloadRootfs
logical tree/subtree and ext4 builder. No Host materialization/chown of guest
metadata, tar parser, guest copy framework, new schema meaning or dependencies.
Reused allocations are never reseeded; preparation stages disk output privately
before publishing to exact owned allocation. Check cancellation/error cleanup.
Mac transport must reject ambiguous overlap/aliases/identity/access and retain
no-follow file descriptors until VM exit. Disk RO enforced in native backend
and sandbox, not just guest mount flags. Existing named/ordinary file grants
and rootfs transport meanings remain.

Reach: Box oci_production/resourceplanning/vm oci_microvm/oci_hvf_spec,
rootfs/oci_ext4 tree projection, storage classification and owning native tests;
OCI runtime hvf_directory + existing disk transport helpers, Mac attachment/
context/sandbox; reuse current Agent raw-device serial/mount code. Trace actual
callers and helpers before changing, add before regressions first.

Constitution I/II/V/VIII failclosed, one authority and typed data; III retained
failures; IV/X actual Host native effects; VI/VII no new workspace/config/deps;
IX source/data/credential preservation; XII unchanged native boundaries;
XIII exact persisted values; XIV existing-host Mac-only evidence; XV retained
assertions/gates. No waiver. Every platform absence remains unclaimed.


Contract inspection correction: existing Agent raw-disk storage entries are
reusable, but v5 explicitly forbids raw storage with workload rootfs and v6
explicitly forbids it with files. Preserve these old meanings. The combination
needs explicit new manifest v7 plus a required typed SDK capability token for
raw ext4 images, using existing extension registration. OldHost/shim must refuse
this combination; do not simply relax v5/v6 validation. No new storage shape,
resource state machine or native API/dependency is required. Raw-only existing
storage-v2 legacy paths keep their exact behavior.

Disk layout reuses current Ext4Artifact directory (artifact.json + rootfs.ext4)
under the owned anonymous allocation, published atomically from private staging
on the same filesystem. Existing open_ext4_artifact_for_resume validates mutable
Guest filesystems/journal recovery. New evidence must verify clean image UUID
against allocation metadata as well as crash-recovery UUID; inspection found
clean branch omitted this check, and a before regression is running. No manual
superblock parser or private metadata format. Capacity derives from existing
request disk policy; do not silently impose or claim a global volume quota.

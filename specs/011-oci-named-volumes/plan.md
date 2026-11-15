# Plan: reuse Box volume authority

Trace: CLI resolves named source in VolumeStore and records policy.volume_names;
ExecutionManager persists it; OCI preflight currently refuses any names before
resource claims. ExecutionResourceGuard already attaches/detaches references.
The unused production helper attach_box_owned_volume_storage already classifies
names as Caller/DetachOnly storage. Mac's current builder instead gives generic
bind IDs. Wire the existing helper and exact validated store lookup; do not
merely delete the qualification check or silently change old ordinary bind IDs.

Reach: oci_production.rs + sibling tests; oci_storage_attachments.rs + actual
callers; VolumeStore/resource guard only if safety validation requires it;
native oci_hvf test module and final installed CLI; owning/release/ledger/docs.
Inspect catalog corruption and attach/remove races before relaxing admission.
Reject unenforced driver/quota/path/anonymous/Secret combinations explicitly.

Constitution: I/V/VIII typed fail-closed boundaries; II existing manager/store;
III retained failures/archives; IV/X hermetic + actual installed Host PEP;
VI/VII one workspace/no dependency/config; IX preserve shared caller data;
XII no submodule edit; XIII exact IDs from stored values; XIV actual Mac-only
claim; XV all assertions and gates retained. PASS; no waiver needed.

Sequence: current refusal baseline -> caller/implementation/negative tests ->
focused + owning gates -> frozen installed assets/native matrix -> cleanup/
evidence/ledger -> next existing W4 slice. Do not end at this bounded phase.


Observed implementation decisions: existing helper raw volume name is not an
allocation identity. A timestamp-derived identity also failed when recreating
from a copied prior config. Reuse the existing VolumeConfig managed-label
mechanism for a store-issued UUIDv4; protect it on mutation and change it on
new materialization. Existing no-label catalogs derive a stable legacy identity
without read-time rewrite. No new Box metadata field or state layout is needed;
older label-map roundtrips retain the managed key. Arbitrary old-library metadata
updates remain outside full rollback qualification.

Before relaxing admission, tests proved VolumeStore traversal/corrupt-catalog
mutation issues; single-component validation and strict normal resource APIs
now close these shared boundaries. Explicit legacy load recovery remains only
for compatibility. New normal APIs preserve malformed files and report errors.
Reuse existing atomic durable writer. Quotas/custom drivers and more than one
bind of a named allocation per execution remain explicit profile refusals.

Actual native1 proved named Host RO14denials+child, then caught stale directory
file length after Host growth. Use the already-pinned generic uncached native
API for all caller directory exports in macOS shim, keeping runtime-control
share behavior. No native/FW/kernel edits or silent fallback. New signed OCI
candidate1 is source-bound; full named Native2 must pass before final archives.

# Reuse the existing bounded disk path

Entry: OciBundleProvider preflight calls validate_dedicated_vm_qualification;
the current shared guard refuses disk_mb !=default. Mac VmManager already sends
the requested disk_mb to Ext4ArtifactOptions::from_disk_mib and anonymous staging.
RootfsImage validates the identical16MiB..64GiB byte range and the Host/Guest
verify exact disk identity/capacity. The sparse publication fix from027 applies.

Add a small Mac-only early call to the existing Ext4ArtifactOptions constructor,
and limit the old custom-capacity refusal to other profiles. Preserve all
unrelated controls. Sibling unit tests cover boundaries, rejected values and
other-profile/control refusals before mutation. Native sibling reuses the existing
manager/Host/recovery/anonymous/CLI helpers and a bounded synthetic Linux probe.
Trace and verify actual source/descriptor/block/filesystem capacities and cleanup,
including too-small-image failure, without inventing a second store or parser.

Constitution Check: ExecutionManager still owns resources and generation;
existing typed config/data contracts only, no schema reinterpretation/fallback;
existing builders/pins/limits retained; no native source edits; actual effects
and preserved failures under docs/evidence, installed final artifacts and default
gate required. No waiver. Source and dependency identities remain explicit.

Completed: two production lines, focused/owning/default gates, frozen Box4 and
fresh unchanged OCI, actual custom-capacity/CLI/three-generation matrix and
13-home audit. Native test assumptions were corrected against spec008 lifetime
and typed transport semantics without changing product behavior. [Acceptance](../../docs/evidence/runtime/20261115-oci-disk-capacity/OUTCOME.md).
Next: existing Mac cap_add rejects even shared portable allowlist requests.

# Plan: guest-native OCI workload disk

Status: implemented; final installed archives and owned cleanup accepted in [OUTCOME](../../docs/evidence/runtime/20261114-oci-rootfs/OUTCOME.md).

## Constitution check

ExecutionManager remains resource authority; existing OciSdk durable route and public SDK/handoff own execution. Versioned typed contracts refuse unsupported transports; existing limits/atomic publication and exact-owned cleanup are reused. Native submodules, capability ceiling, default routing and platform claims stay unchanged. Failures and native results live in docs/evidence/runtime/20261114-oci-rootfs.

## Smallest complete path

Reuse LogicalRootfs and verified OCI layer assembly plus mkext4 publication, without Box guest-init or boot-control injection. Add an explicit SDK workload-rootfs extension and versioned Agent manifest field. Independent HVF attaches a separate pinned raw workload disk using its existing krun_add_disk2 binding; Agent mounts it at the OCI bundle root before executor preparation. Keep the system image and workload root independent. Preserve legacy directory records and their admission gate.

Reach: vm/oci_microvm, layout image-pull/spec/process resolution; rootfs/oci_ext4; oci_production preparation/capability admission; SDK attachment exports/capabilities; Host handoff/HVF preparation; Agent manifest/bootstrap/block inventory; shim pinned disk/sandbox; build assets and native tests/fixtures; archive/release evidence and exact-owned cleanup. No new lifecycle or cache.

Sequence: baseline -> workload builder tests -> typed SDK/protocol and HVF/Agent disk path -> Box caller integration -> hermetic adversarial checks -> frozen-assets native debugging -> full gates -> affected archive rebuild/install/native replay -> cleanup and W4 evidence.

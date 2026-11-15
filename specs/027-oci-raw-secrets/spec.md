# Managed memory Secrets with raw anonymous volumes

Status: accepted on this existing macOS arm64 host; [final installed evidence](../../docs/evidence/runtime/20261115-oci-raw-secrets/OUTCOME.md).

Permit an explicitly versioned combination of existing managed RAM RO files
(optionally environment bindings) and existing image-VOLUME raw anonymous disks
on the explicit macOS OciSdk route. Reuse exact file/raw/rootfs identities,
Host RAM provenance, Guest pre-namespace source pins and existing allocation/
cleanup/recovery. Do not merely remove the existing combination guards.

Keep memory filesv1/environmentv2 and transportv7/v8/v9/v10 bytes/meaning intact;
old combinations still refuse. Require a new memory capabilityv3 and combined
transportv11; old Host/shim/Agent refuse before workload entry or fallback.
NPROC's existingv10 envelope may bind the new transport explicitly and must
retain its entry guard. No new product config/resource state/native dependency.

Prove exact synthetic Secret bytes by hashes only, actual Host RO attacks after
remount/fresh mount/child, protected env sources after visible replacement,
raw disk seed/write/isolation/lifetime identities, all process paths/CLI,
three generations/replay/fencing/normal restart/stopped recovery and exact
RAM/disk/volume/source-preserving cleanup. Reject mismatched versions/bindings,
disk Secret sources/projections, aliases/overlap and absent capabilities.

Freeze/build/freshly install affected Box+OCI+Agent assets, verify actual
architecture/signatures/native/embedded Agent identity, execute counts/results
and affected/A–F/default regression gates before acceptance. Preserve failures.
Full W4/default cutover/live continuity/platform promotion stay open; Windows
ARM64 dependencies unsupported/no native run, RSS unmeasured. Continue local W4.

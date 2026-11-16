# kbox-runtime

## 概述

MicroVM runtime engine — VM lifecycle, OCI images, attestation, networking

Local MicroVM and shared Kbox Runtime provider implementation for Kbox Box.

## Overview

This package provides the actual runtime implementation for Kbox Box, including:

- **VM Management**: MicroVM lifecycle management with libkrun
- **OCI Image Support**: Pull, store, and extract OCI container images
- **Rootfs Builder**: Construct guest root filesystems from OCI layers
- **gRPC Communication**: Guest agent health checking over Unix socket
- **Filesystem Operations**: virtio-fs mount management
- **Metrics Collection**: Runtime metrics and monitoring
- **Kbox Runtime Provider**: generation-fenced Tasks and Services over Linux
  MicroVM backends (Box-libkrun and DedicatedVm OCI)
- **Transient Secrets**: caller-authorized environment and file Secret
  materialization without durable plaintext

## Architecture

The runtime package builds on top of `kbox-core` which provides foundational types:

```
┌─────────────────────────────────────┐
│         kbox-runtime             │
│  (VM, OCI, Rootfs, gRPC, etc.)     │
└─────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────┐
│          kbox-core               │
│  (Config, Error, Event)            │
└─────────────────────────────────────┘
```

## Shared Kbox Runtime provider

`BoxRuntimeDriver` implements the shared `kbox_contract` contract. It maps
digest-pinned OCI Tasks and Services onto the canonical local execution manager
instead of creating a second Box lifecycle path. The driver owns provider
identity, generation fencing, recovery, Service endpoints, health observations,
logs, exec, resource controls, and cleanup.

The provider consumes the local workspace `kbox_contract` 0.8 contract and
advertises its atomic `ServiceLifecycle` feature. `RuntimeUnitSpec::health` remains readiness for
traffic admission. `RuntimeServiceLifecycle::liveness` independently drives
recovery according to the declared restart policy, while
`shutdown_grace_seconds` is persisted as an exact `SIGTERM` grace interval
before Box forces termination. Apply, inspect, and exec use independent
threshold trackers for the two probes, refresh the canonical execution after
every in-flight probe, and apply at most one liveness restart per public
operation. The durable Box execution generation remains the sole restart
counter and lifecycle authority.

The lifecycle-aware provider timeout reserves the complete declared grace plus
the normal control-plane budget. Source-level coverage proves threshold reset,
one-restart generation advancement, exact policy mapping, and timeout
reservation. Advertising the feature also activates the real-provider
readiness/liveness separation, liveness transition, graceful stop, and forced
deadline cases; those ignored-by-default cases require the dedicated Linux
KVM certification environment.

Box implements the shared Runtime contract on MicroVM isolation.
`NetworkMode::None` and `NetworkMode::Service` remain loopback-only and do
not grant workload egress: MicroVM keeps explicit vsock IPC without libkrun
TSI socket interception for those modes. `NetworkMode::Outbound` maps to
default TSI socket-proxy egress and does not grant Service loopback
publication.

Callers that need Runtime Secrets compose the driver with exactly one
`BoxSecretMaterializer`:

```rust
use std::sync::Arc;

use kbox_core::ExecutionBoundary;
use kbox_runtime::{BoxRuntimeDriver, BoxRuntimeDriverConfig};

let config = BoxRuntimeDriverConfig {
    secret_root: "/run/kbox/runtime-secrets".into(),
    ..Default::default()
};
let driver = BoxRuntimeDriver::new_with_boundary(config, ExecutionBoundary::Microvm)?
    .with_secret_materializer(Arc::new(node_secret_materializer));
```

The configured `secret_root` must already be a canonical provider-owned `0700`
Linux tmpfs mount. Box never creates it and never falls back to disk. The caller
owns reference authorization and transport; Box owns only node-local
materialization, deterministic isolation-specific read-only mounts, recovery
validation, log redaction, and lifecycle cleanup.

On macOS arm64, the Box and OCI VM workers share a host Seatbelt read-only
boundary. Read-only directories are descriptor-pinned; overlapping writable
grants, pre-existing hard-linked files, trees above 100000 entries and pre-opened
writable source descriptors refuse before VM entry. Caller-owned writes stay
live. A validated `ram://` root proves memory-backed Secret storage only; it
does not by itself qualify a Secret-bearing workload. The retained BoxVm route
refuses ordinary read-only single-file mounts. The explicit OciSdk profile below
supports exact single-file grants without exporting their parent. See the
[host enforcement evidence](../../../docs/evidence/runtime/20261114-macos-readonly/OUTCOME.md).

`BoxTransientSecretStore` exposes that same validated tmpfs owner to Box's
local Compose adapter. Compose creates a private scope only after the parent
tmpfs passes the provider checks; it supplies zeroizing values and receives
only mount paths plus the guest manifest. For MicroVM single-file mounts, the
virtio-fs directory staging copy stays inside that tmpfs scope. The ordinary
disk-backed `.filemounts` fallback is never used for managed Secret sources.

Environment, file, and registry credential targets are supported. Environment
values use a non-sensitive `K_BOX_SECRET_ENV_V1` binding manifest that Guest
Init validates and consumes immediately before process creation. File material
uses the exact requested mode. A registry credential is resolved only when the
image is absent from the local cache, handed through a per-execution in-memory
broker, consumed at the image-pull boundary, and zeroized before rootfs
preparation continues. Cache hits do not call the resolver. With no registry
target the Runtime driver supplies explicit anonymous auth, preventing fallback
to persistent Box-local or process-environment credentials.

Secret bytes never enter durable Box records, creation intent, OCI
configuration, logs, cursors, or the image credential store. Stopping retains
environment and file material for an explicit restart; removal and stale-
generation retirement remove it. A reconstructed driver validates the existing
live generation without rematerializing it. Every log read reauthorizes only
workload-visible references, redacts exact values longest-first, and hashes only
redacted content into its cursor. Registry credentials are never injected into
the workload and therefore never become log-redaction inputs.

MicroVM providers currently refuse `BoxRuntimeDriverConfig.state_snapshot=true`
with typed `UnsupportedCapabilities` before creating state. They do not advertise
Runtime `StateSnapshot`. CLI rootfs/image snapshots have their separate existing
contract; Runtime live filesystem and memory capture are not implemented.

## Local execution engine

`kbox engine serve` exposes a same-user, owner-only Unix control endpoint over
`ManagedRuntimeClient` and `ExecutionManager`. Runtime owns operation receipts;
ExecutionManager owns actual VM, process, mount, network and teardown effects.
The engine accepts OCI Service definitions and opaque process input. Package
installation, application readiness, guest dependency checks and application
protocols belong to the caller.

### Versioned contracts

Discovery advertises `kbox.engine.v2`. The request/response envelopes remain
`kbox.engine.request.v1` and `kbox.engine.response.v1`; intents now use
`kbox.engine.intent.v2` and the durable journal uses `kbox.engine.journal.v2`.
Service intent has no workload classification or package-assembly field.
Unknown fields and unsupported schemas are refused.

Existing v1 journals are retained and refused rather than rewritten or reset.
Before upgrading an existing engine home, stop its workloads and unclaimed
reservations with the previous executable, and retain the old home for rollback.
Use a fresh `K_HOME` for the new engine. The ordinary Box CLI storage contract
is unchanged. This cleanup does not migrate external clients.

### Explicit guest policy

`EngineServiceTemplate.semantics_profile_digest` passes the caller's explicit
Runtime profile through unchanged. Omitting it keeps the provider's ordinary
security defaults. The optional `kbox.native-guest.v1` profile requires an
independent-kernel Sandbox, enables guest capabilities, and disables guest
seccomp and `no-new-privileges`. It grants no host mounts, devices or services.
No application type selects privileges automatically.

The canonical LF bytes pinned by `NATIVE_GUEST_PROFILE_DIGEST` are:

```text
kbox.native-guest.v1
boundary=microvm
seccomp=unconfined
capabilities=all-guest
no-new-privileges=false
resources=explicit
```

### Live directory grants

`RuntimeMountSource::Directory` names a mutable caller-owned directory.
`directory-grants.v1` binds exact paths and permissions to the accepted Runtime
specification and generation. Missing or moved paths do not prevent retirement
of an already bound execution. No parent directory is implicitly exposed;
removal retains caller data. Immutable Artifact inputs remain read-only.

### Process streams

`process-stream.primary.v1` opens an exclusive primary process per environment
generation. Control and one-use data attachments preserve raw stdout/stderr,
stdin half-close and one exact exit. Disconnected streams are not replayed or
resumed; Runtime retires the old generation before preparing another attempt.
Other exec, PTY and file APIs keep their contracts.

`process-stream.bootstrap.v1` forwards a bounded opaque launch input through
guest stdin before admitting the data stream. Bootstrap bytes stay out of the
serialized request, its fingerprint and the journal. The caller owns their meaning.
Process launch checks the actual execution identity and generation through the
existing driver; the engine does not inspect application output or receipts.

### Clean environment prewarm

`environment-prewarm.v1` is selected through the intent's optional
`prewarmBindingsDigest`, independently of any package or application. A prewarmed
Service must be an independent-kernel Sandbox running `/bin/sleep infinity`,
with no process environment, Secrets, health check or restart policy. Either the
ordinary guest policy or an explicitly selected guest profile may be used.

The compatibility key includes the pinned image, boot configuration, resources,
exact directory identities, caller bindings digest, platform, runtime assets and
engine authority. ExecutionManager owns reservations and atomically advances
the generation when claiming one. Unclaimed environments reject workload access.
After a running Service opts in, the engine maintains one compatible spare.
Stop stays `CLEANUP_PENDING` until owned unclaimed reservations are removed.

Current change verification and limits are recorded in
[generic engine evidence](../../../docs/evidence/engine/20261114-generic-engine/OUTCOME.md).
See [local engine boundaries](../../../docs/architecture/local-engine.md).

## Cloud image-build boundary

`BoxBuildPlan` is the closed Kbox ACL admission contract for Cloud-owned image
build intent. It validates one `build "oci"` block, produces canonical ACL
bytes and a stable SHA-256 digest through `kcl`, confines both the context
and build file to one canonical source root, and compiles into the existing
`BuildConfig`. The recorded-build API adds supervision by extending the same
native engine, receipt journal, and `ImageStore`; it does not introduce another
build backend, cache, queue, scheduler, lifecycle store, workspace registry, or
image authority. Content-changing build arguments are deliberately absent from
version 1; adding them requires a new closed schema so they cannot alter output
behind an unchanged plan digest.

```kcl
build "oci" {
  cache = "content-addressed"
  context = "."
  file = "Dockerfile"
  network = "none"
  platform = "linux/amd64"
  schema = "kbox.build-plan.v1"
}
```

The admitted cache values are `content-addressed` and `disabled`. Network is
closed to `none` or `outbound`. `none` rejects remote URL `ADD`, binds the
policy into every `RUN` cache identity, and adds a private Linux network
namespace to every native `RUN`. Base-image and external-stage resolution stay
host-side OCI operations; they are not executed inside the build rootfs.
Network-none plans reject the warm RUN pool until that provider can prove an
equivalent isolated network boundary.

```rust
use std::path::Path;
use std::sync::Arc;

use kbox_core::OperationId;
use kbox_runtime::{
    execute_recorded_build_plan, BoxBuildPlan, BuildOperationIdentity,
};

let plan = BoxBuildPlan::parse_kcl(plan_kcl)?;
let identity = BuildOperationIdentity::new(
    OperationId::new("cloud-build-018f")?,
    source_artifact_digest,
)?;
let result = execute_recorded_build_plan(
    &identity,
    &plan,
    Path::new("/srv/kbox/source"),
    true,
    Arc::clone(&image_store),
)
.await?;
```

`start_recorded_build_plan` starts without blocking;
`inspect_recorded_build_status` returns running, cancelling, cancelled, failed,
or a revalidated successful output; and `cancel_recorded_build_plan` durably
requests cancellation through that same journal. A crash-released execution
lease is liveness evidence, not a second state store. Linux `RUN` uses one
asynchronous subprocess boundary with kill-on-drop, a parent-death signal, a
private PID namespace, and a journaled PID/start-time identity. Recovery fences
that process tree before removing the hash-derived operation workspace.
The existing journal lock serializes cancellation with ImageStore publication;
content-addressed plans use that same permit to publish one operation-owned
Box-native OCI cache artifact before the ImageStore commit. One trace records
hits and stores from the existing `BuildCache`; there is no export cache,
publisher, or cache lifecycle store alongside it. Export layers are copied
into an immutable snapshot so receipt validation and cleanup cannot mutate the
live cache authority through shared inodes.
Native OCI config and history records use the canonical epoch because a build
plan carries no creation clock, so identical inputs preserve the exact manifest
descriptor across rebuilds and parent-cache hydration.

The cache key length-binds the source digest, canonical plan digest, platform,
and cache schema profile. The v2 supervised operation record persists the
admitted cache policy: a new `content-addressed` operation must commit cache
evidence, while a `disabled` operation must not. Legacy v1 operation and output
records remain readable without cache evidence. Replay rejects extra,
missing, symlinked, or changed cache entries and revalidates the manifest,
config, layers, descriptor sizes, exact blob inventory, and byte count before
returning the cache receipt.

`hydrate_recorded_build_cache` accepts that existing `RecordedBuildCache`
type, repeats the same complete artifact validation, and publishes entries
through the sole `BuildCache` lock and blob/key write boundary. A valid local
key with different content fails the entire preflight before an imported key
is written; repeated imports are idempotent. Imported layers are copied, the
complete imported set is retained while unrelated old blobs are pruned, and a
successful return carries the exact revalidated receipt.

`assemble_recorded_build_outputs` accepts two to eight
`BuildOutputAssemblyInput` values after their single-platform native builds
complete. `BuildOutputAssembly` requires one source digest, identical context,
build file, target, network, and cache intent, and one unique plan-bound receipt
per platform. It sorts the inputs before constructing the OCI image index,
copies shared content-addressed blobs once, revalidates every input and the
complete assembled graph, then enters the same ImageStore commit boundary as a
direct build. It has no execution backend, queue, scheduler, receipt journal,
manifest store, or publisher.

```rust
use std::sync::Arc;

use kbox_runtime::{
    assemble_recorded_build_outputs, BuildOutputAssembly, BuildOutputAssemblyInput,
};

let assembly = BuildOutputAssembly::new(
    "registry.example/kbox/service:build-018f",
    source_artifact_digest,
    vec![
        BuildOutputAssemblyInput::new(amd64_plan, amd64_result.receipt),
        BuildOutputAssemblyInput::new(arm64_plan, arm64_result.receipt),
    ],
)?;
let image_index =
    assemble_recorded_build_outputs(&assembly, Arc::clone(&image_store)).await?;
```

This completes the Box-owned `BX0.4` supervision, portable cache receipt,
typed cache hydration, and multi-platform OCI assembly slices. Cloud Node Agent
consumption through its existing Fleet queue, Artifact publication, and
end-to-end SPDX/SLSA signing evidence remain separate integration gates.

## Components

### VM Manager

Manages the microVM lifecycle:

```rust
use kbox_runtime::VmManager;
use kbox_core::{BoxConfig, EventEmitter};

let config = BoxConfig::default();
let emitter = EventEmitter::new();
let vm = VmManager::new(config, emitter);

// Boot the VM (lazy initialization)
vm.boot().await?;

// Check health
let healthy = vm.health_check().await?;

// Destroy when done
vm.destroy().await?;
```

## VM States

The runtime manages the following VM states:

- **Created**: Config captured, no VM started
- **Ready**: VM booted, agent initialized, health check passing
- **Busy**: A session is actively processing
- **Compacting**: A session is compressing its context
- **Stopped**: VM terminated, resources freed

## gRPC Communication

The runtime communicates with the guest agent over Unix socket (bridged to vsock port 4088):

- Health checks

Agent-level operations (sessions, generation, skills) are handled by the
kbox crate, not the Box runtime.

## Explicit macOS OCI Host Service

On macOS arm64, `LocalExecutionManager::with_macos_hvf_oci_qualification`
connects the public SDK to an independently started `koci
macos-hvf-host-service`. Box retains execution/resource authority; the service
owns the HVF worker and authenticated guest transport. The service must use its
fixed signed shim, pinned native bundle, system image and manifest.

CLI discovery uses `K_BOX_OCI_HVF_ENDPOINT=/absolute/service/runtime.sock` and
`K_BOX_OCI_MIGRATION=microvm`. The runtime root is the sibling `runtime`
directory (`K_BOX_OCI_HOST_ROOT`, if supplied, must match it). Use the same
endpoint with `K_BOX_OCI_MIGRATION=off` to create BoxVm executions while keeping
stored OciSdk routes usable. Missing explicit endpoints or unsupported modes
refuse; macOS with no migration input keeps its existing BoxVm default.

The bounded profile requires Linux arm64 and `network=none`. VM hardware accepts
1/2/4/8 vCPUs and 128..8192 MiB in 128 MiB steps. Custom pairs require the exact
`dev.kbox.vm-resources` v1 attachment; absence retains 1 vCPU / 512 MiB. See
[actual Guest hardware, recovery and installed-archive acceptance](../../../docs/evidence/runtime/20261116-oci-vm-sizing/OUTCOME.md).
It admits plain caller directory workspace/binds,
host-enforced directory `:ro`, typed cgroup limits and explicit guest SYS_ADMIN
(with cap_drop precedence). Init and exec share this admitted capability
ceiling. Secret/raw-disk combinations, custom devices/network,
snapshots/pools and unsupported overrides refuse instead of changing isolation or silently ignoring intent.

Existing named VolumeStore allocations support one explicit RO or RW bind per
allocation per execution. Box retains allocation/deletion authority and reference
counts; the Host receives Caller/DetachOnly storage with a stable allocation
identity. Normal restart and stopped recovery retain data and identity; deleting
an execution only detaches its reference. Caller directory exports use uncached
live data, including Host content growth after a guest has read a file.
Corrupt catalogs, path/name aliases, missing or duplicate bindings, custom drivers
and unenforced quotas refuse. New allocations receive a protected UUID in the
existing labels map; legacy catalogs retain their bytes and stable identity.
Normal volume APIs preserve malformed metadata instead of quarantining it into
an empty catalog. See [named-volume implementation and current acceptance](../../../docs/evidence/runtime/20261114-oci-named-volumes/OUTCOME.md).

Image-declared VOLUME allocations use the existing durable anonymous plan,
VolumeStore owner and protected allocation UUID. Required
`dev.kbox.raw-storage-images` v1 / Agent attachment manifest v7 carries the
allocation's private ext4 image alongside the existing rootfs, directory and
ordinary-file transports. The seed comes from the layered Guest namespace,
preserving numeric ownership, modes, links and xattrs without Host chown.
Explicit caller/named mounts override the same image destination. Restart and
stopped recovery reuse the same allocation/data; new executions get independent
allocations. Normal remove and ephemeral stop delete only their owned anonymous
volumes; explicit restart retains them. Unenforced quotas, legacy nonempty data
without a valid artifact, identity/size drift and missing Host capability refuse.
Raw disks retain exact RO/RW descriptors and OS locks; Host confinement grants
file data only, and Runtime cleanup remains Caller/DetachOnly. See
[spec012 implementation and acceptance status](../../../docs/evidence/runtime/20261115-oci-anonymous-volumes/OUTCOME.md).

New explicit HVF executions require `dev.kbox.rootfs-image` v1 and carry the
OCI workload in guest-native ext4. Verified OCI layers preserve numeric UID/GID,
modes and links without host foreign chown or Box guest-init injection. The
Host transfers a private operation disk into one exact generation outside its
guest-exported runtime share; no two executions share a writable disk. The
immutable VM system image remains separate. Old directory records retain their
transport and foreign-UID refusal; a Host missing the new capability refuses
new preparation. These workload roots are ephemeral: explicit restart builds a
fresh root in the new generation. Caller directory lifetime is unchanged.
See [spec008](../../../specs/008-oci-rootfs/spec.md) and its
[rootfs evidence](../../../docs/evidence/runtime/20261114-oci-rootfs/OUTCOME.md).

Ordinary caller-owned single files use required `dev.kbox.file-grants` v1 and
Agent manifest v6. Box retains their device/inode identities across restart;
replacing a source pathname refuses reuse. Host-pinned private live projections
export only the granted file. RO remains enforced by the Host after privileged
remount, fresh mount and child-process attacks. RW permits data writes/truncation,
without caller-parent enumeration, sibling access, link, rename or deletion.
Sources must be unaliased regular files on the projection filesystem; unsafe
overlaps, writable inherited descriptors and cross-filesystem projection refuse.
Cleanup removes runtime aliases and retains caller files. See
[spec009 and installed-archive evidence](../../../docs/evidence/runtime/20261114-oci-files/OUTCOME.md).
Caller-file and macOS OciSdk anonymous-volume state roots use a versioned
directory at `boxes.json`, containing
`records.v1.json`; it remains the same BoxStateStore and uses the same sibling
lock. Normal current loads adopt earlier flat file-bearing records atomically
on macOS; read-only snapshots do not migrate. Other legacy roots keep their flat format. Older readers fail I/O before their quarantine fallback, and stale
flat writers cannot replace the directory. Missing, corrupt or future directory
records fail closed; adopted roots stay in that format after cleanup. Backups
must preserve the whole state directory. Manual flattening or arbitrary old-
binary rollback is unsupported. See
[compatibility work and current acceptance status](../../../docs/evidence/runtime/20261114-oci-file-rollback/OUTCOME.md).

Host SIGKILL uses authenticated **stopped-only** recovery; active process
sessions explicitly fail. Reconstructed clients retain persisted routes and
exact bindings. An explicit restart advances the generation; old input cannot
reach the replacement. Terminal reconciliation releases the exact OCI runtime
binding while retaining Box identity, route and generation history.

See [the bounded task](../../../specs/007-oci-host-service/spec.md) and
[native evidence](../../../docs/evidence/runtime/20261114-oci-host-service/OUTCOME.md).
This opt-in integration does not close full OCI Host Service cutover or raise
platform support levels.

## License

MIT

Guest rootfs read-only enforcement now uses one fallible init/exec/PTY helper:
failed mounts/stat/readback refuse launch; already read-only roots are idempotent
and inner mounts keep their explicit permissions. Installed BoxVm root EROFS,
exec/PTY/restart and real Linux Guest failure paths are covered in
[spec013 evidence](../../../docs/evidence/runtime/20261115-rootfs-readonly/OUTCOME.md).
Masked/readonly-path setup is closed in the spec014 slice below; this does not
qualify full OCI control parity or change the default backend.

The shared Box Guest masked/readonly-path guard now refuses setup errors before
exec/PTY launch, pins target components, verifies actual masks and recursively
sets readonly mount attributes. Unsafe symlink components refuse; private empty
readonly file sources retain original data and avoid unlinked-device attachment
failures. Existing managed K_SEC_* filtering and generation authority are unchanged.
[Spec014 final installed evidence](../../../docs/evidence/runtime/20261115-guest-path-guards/OUTCOME.md)
records actual Guest protocol/EROFS/restart results and retained fixture corrections.
These Guest path controls require updated booted companions and do not establish
Host inode RO, full OCI control parity or a default backend change.

The explicit Mac HVF profile now carries bounded Box tmpfs intent through the
existing OCI Guest executor: disjoint targets,RO/RW,size/mode/UID/GID and common
safety flags. The shared Core parser keeps mount flags out of kernel data;
BoxVm keeps its advanced data options. Image VOLUME exact-target overrides do
not allocate hidden anonymous disks; unqualified/overlapping targets refuse.
[Spec015 installed effects and cleanup](../../../docs/evidence/runtime/20261115-oci-tmpfs/OUTCOME.md)
record actual tmpfs/EROFS/ENOSPC/noexec,CLI,generation reset and stopped recovery,
with caller/named data retained. Kernel nosuid/nodev readback is recorded
separately from independent privilege probes. Full W4/default cutover and OCI
Secret parity remains open; no platform support promotion.

The explicit Mac profile also maps Box read_only to the existing OCI root view.
It requires rootfs-image v2; v1 remains writable-only with identical bytes.
Old Host capability refusal occurs before preparation and does not prove the
feature. Actual init/exec/stream/PTY/File/FS EROFS,inner RW,restart/stopped
recovery,explicit OCI writable control and CLI are qualified in
[spec016 evidence](../../../docs/evidence/runtime/20261115-oci-root-readonly/OUTCOME.md).
The physical private boot disk remains writable for runtime boot; this does not
revoke explicitly granted Guest SYS_ADMIN or replace caller Host RO. No default
cutover,promotion or live continuity; Secret/unqualified controls/other parity remains open.

Bounded Mac nofile/fsize/core now map through standard OCI process.rlimits for
init/exec/stream/PTY/children, retaining omitted defaults and fingerprints.
Malformed legacy Shim input refuses through the shared Core parser. Managed
BoxVm and OciSdk CLI process sessions both use ExecutionManager security and
generation fencing; raw sockets no longer bypass stored cap_drop ALL.
[Spec017 final installed evidence](../../../docs/evidence/runtime/20261115-oci-ulimits/OUTCOME.md)
records kernel EMFILE/EFBIG/EPERM,real attach/spoof controls,three generations,
CLI and owned cleanup. This is bounded process enforcement,not a management
filesystem quota or full W4/default cutover. Secret environment transport is next local work.

Bounded managed Secret files now use the existing readonly file grant mechanism
with required memory-secret-files v1 / Agent manifest v8. Both the source and
projection must be on actual validated RAM storage, including APFS only when
every physical store is a hdiutil ram:// image. The Host advertises capability
only for a RAM runtime share; source numeric mode/owner and exact file identity
remain unchanged. Old assets, physical runtime roots, raw disk combinations and
Secret environment manifests refuse. [Spec018 installed evidence](../../../docs/evidence/runtime/20261115-oci-secrets/OUTCOME.md)
qualifies actual Host denials, init/exec/stream/PTY, independent files, CLI,
three generations and exact owned RAM cleanup. Full Secret parity, W4/default
cutover and Windows ARM64 remain unqualified; continue environment delivery.

Managed Secret environment projection is now qualified on the explicit Mac OCI
route through memory-secret-files v2 / Agent manifestv9. The existing manifest
is validated against exact RO RAM grants, consumed before process launch and
omitted from workload env. Sources are pinned in the protected Agent namespace
before workload root entry, so replacing a visible path cannot redirect an
exec read. Only metadata enters private plans; rejected bindings are validated
before plan writes. Files-onlyv1/v8 and legacy native Secret checks remain intact.
[Spec019 final installed evidence](../../../docs/evidence/runtime/20261115-oci-secret-environment/OUTCOME.md)
qualifies actual Guest8/process paths/CLI/compatibility/three generations and
19-home cleanup. Raw-disk combinations, other parity and full W4 remain open;
next bounded local work is nonroot nproc process-count limits.

Spec020 qualifies Box CPU-time intent on the explicit Mac OCI route through the
existing process.rlimits executor. Zero refuses; omission/default bytes and
other limits stay intact. Actual SIGXCPU/SIGKILL, measured CPU usage, idle and
unlimited controls, init/exec/child/stream/realPTY, CLI/replay/three generations
and owned cleanup pass the final installed archives. [Final evidence:8 outer
Rust native,8 Guest units,3 Python scenarios and20-home cleanup](../../../docs/evidence/runtime/20261115-oci-cpu-time/OUTCOME.md).
CPU-time is per-process, separate from wall deadlines and aggregate cgroups.
Full W4/default cutover/live continuity/platform promotion stay open; Windows
ARM64 dependencies remain unsupported without a native run. Next local work:
data/stack/memlock process limits, qualified by spec021 below.

Spec021 qualifies standard data/stack/memlock byte limits through the existing
Mac OCI executor. Actual malloc/mmap/mlock ENOMEM, stack SIGSEGV on alternate
stack, unlock/reuse, hard-raise EPERM, unlimited controls, all process paths/CLI,
replay and stopped recovery pass the final installed archives. [Evidence:5 Rust
native tests +3 Python scenarios and10-home cleanup](../../../docs/evidence/runtime/20261115-oci-memory-limits/OUTCOME.md).
No VM/cgroup/RSS or privileged-bypass claim; full W4/default cutover/platform
gates remain open. Nonroot nproc is qualified by spec022 below.

Spec022 qualifies nonroot per-UID nproc through all process paths/CLI/health/real
PTY/attach, three generations and actual fork/thread EAGAIN/reaping/reuse. Shared
Agent admission closes the native-observed root-helper equality bypass; required
Host nproc-admission v1 and manifest v10 bind prior transports and refuse old
Host/shim/Agent before workload entry. Legacy root defaults remain unchanged.
The same native matrix found and fixed HVF bare-command PATH resolution in the
shared session caller. [Final installed evidence:12 Rust native,13 Guest units,3
Python scenarios plus1 native decoder probe and29-home cleanup](../../../docs/evidence/runtime/20261115-oci-nproc/OUTCOME.md).
Full W4/default Host Service cutover/live continuity and external platform gates
remain open. Runtime RSS unmeasured; Windows ARM64 dependencies unsupported/no
native run. Continue the next unqualified existing process-limit type.

Spec023 qualifies nonroot sigpending through the same per-UID entry boundary
and existing OCI executor. Actual realtime sigqueue EAGAIN, ordered delivery/
drain/reuse, shared UID budgets, mapped userNS refusal at full quota, zero and
infinity controls, ordinary kill semantics, all process paths/CLI/health/realTTY/
attach/replay/recovery pass. [Final installed evidence:10 Rust native +13 Guest
units +3 Python scenarios and14-home cleanup](../../../docs/evidence/runtime/20261115-oci-sigpending/OUTCOME.md).
Nproc/defaults/identity/routes/fencing remain intact. Full W4/default cutover/
live continuity/platform gates remain open; RSS unmeasured, Windows ARM64
dependencies unsupported/no native run. Continue existing msgqueue intent.

Spec024 qualifies nonroot msgqueue through the same UID/capability/NNP boundary
and existing OCI executor. Actual MQ capacity EMFILE with ordinary FDs available,
message-capacity EAGAIN/priority+FIFO, unlinked/dup/inherited-FD lifetime, shared
UID quota/reuse, mapped user+IPC namespace refusal, zero/infinity and all process
paths/CLI/health/realTTY/attach/replay/recovery pass. [Final installed evidence:
11 Rust native +13 Guest units +3 Python scenarios and15-home cleanup](../../../docs/evidence/runtime/20261115-oci-msgqueue/OUTCOME.md).
No OCI/Agent/schema/native/dependency change. Full W4/default Host Service
cutover/live continuity/platform gates remain open; RSS unmeasured, Windows
ARM64 dependencies unsupported/no native run. Continue existing nice intent.

Spec025 qualifies existing nice intent through the standard OCI executor without
changing CLI/root defaults or the per-UID counter boundary. Actual -5 success,
-6 EACCES, inherited child/thread and namespace boundaries, zero/infinity,
all process paths/CLI/health/realTTY/attach/replay/recovery pass. [Final installed
evidence:5 Rust native +3 Python scenarios and10-home cleanup](../../../docs/evidence/runtime/20261115-oci-nice/OUTCOME.md).
Only one production enum mapping and its qualification text change; OCI/Agent/
other resources remain unchanged. Full W4/default Host Service cutover/live
continuity/platform gates remain open; RSS unmeasured, Windows ARM64 dependencies
unsupported/no native run. Continue existing rtprio/rttime intent.

Spec026 qualifies existing rtprio/rttime through the unchanged OCI executor:
FIFO/RR5 allowed/6 EPERM, inherited child/thread and namespace boundaries,
actual SIGXCPU/SIGKILL with measured CPU, blocking reset/normal-policy controls,
zero/infinity, all entries/CLI/health/realTTY/attach/replay/recovery. [Final
installed evidence:5 Rust native +3 Python scenarios and12-home cleanup](../../../docs/evidence/runtime/20261115-oci-realtime/OUTCOME.md).
No CLI/default/UID/schema/native/dependency change; current kernel tick granularity
is explicit. Full W4/default Host Service cutover/live continuity/platform gates
remain open; RSS unmeasured, Windows ARM64 dependencies unsupported/no native run.
Continue the existing managed Secret/raw anonymous-volume combination.


Spec027 qualifies managed RAM Secret files/environment with raw anonymous volumes
through required memory-secret-filesv3 / Agent transportv11, including the existing
NPROC v10 envelope. Old versions retain meaning and refuse incompatible intent;
old Host/shim/Agent and disk projections actually refuse. A real cross-volume
sparse rootfs publication bug is fixed by reusing the existing extent copier.
[Final installed evidence:12 Rust native +21 top-level Guest units (plus2 isolated
child test runs) +3 Python scenarios and17-home/8-owned-RAM cleanup](../../../docs/evidence/runtime/20261115-oci-raw-secrets/OUTCOME.md).
Box2 f9f48ab5…:23,393,755compressed/44,171,264installed bytes; OCI1 30eb5e29…:
33,424,560/114,954,240. Verify2261/43ignored,CLI935,installer8. Full OCI Host
Service/default cutover/live continuity/platform gates remain open; Windows
ARM64 dependencies unsupported/no native run, support levels unchanged and RSS
unmeasured. Continue the existing W4 custom writable-root disk-capacity gap.


Spec028 qualifies existing disk_mb on the explicit macOS OciSdk route using the
existing16MiB..64GiB ext4/rootfs-image/raw-storage contracts. Actual128MiB root+
anonymous ENOSPC/reuse, independent256MiB CLI controls,8GiB sparse disk, too-small
image refusal, replay/fences/normal+stopped three generations pass. Root remains
per-generation ephemeral; anonymous data persists. [Final installed evidence:
6 Rust native +3 Python scenarios,13-home/107-socket cleanup](../../../docs/evidence/runtime/20261115-oci-disk-capacity/OUTCOME.md).
Box4 39e6b5b3…:23,393,934compressed/44,171,264installed bytes; OCI30eb5e29…
unchanged/freshly extracted with honestc07659de… source. Verify2264/44ignored,
CLI935,installer8. Full W4/default Host Service cutover/live continuity/platform
gates remain open; Windows ARM64 dependencies unsupported/no native run, support
levels unchanged and RSS unmeasured. Continue the existing Mac cap_add allowlist gap.

Spec029 qualifies the existing Mac portable cap_add allowlist with explicit
nonroot inherited/ambient values shared by init and exec. Default10, root empty
ambient, drop ALL, unknown-cap refusal, per-UID budgets and Host PEP remain.
Actual MKNOD/SETFCAP effects, SYS_ADMIN mount/drop controls,45 observations,
CLI/realPTY,three generations and Host RO pass the final installed Box3/unchanged
OCI assets. Final3Rust native+3Python scenarios pass; verify Runtime2269/45ignored,
CLI935,installer8. Audit18 homes/235 absent sockets/21 pre-grant source identities
and zero owned resources passes. [Final assets, failures and evidence](../../../docs/evidence/runtime/20261115-oci-cap-add/OUTCOME.md).
Full W4/default Host Service cutover/live continuity/platform gates stay open;
Windows ARM64 dependencies remain unsupported/no native run and RSS unmeasured.
Continue the existing W4 policy.shm_size gap; platform support levels stay fixed.

Spec030 qualifies existing shm_size and size-only /dev/shm on Mac: page-aligned
4KiB..512MiB,1777/rw/nosuid/nodev/noexec,exact POSIX mapping/lifetime,1/2MiB
ENOSPC/reuse,noexec/sticky controls,CLI,three generations and7-home cleanup.
Final4Rustnative+3Python pass; verify Runtime2274/46ignored CLI935installer8.
[Pre-merge assets and evidence](../../../docs/evidence/runtime/20261115-oci-shm-size/OUTCOME.md). Full W4/default Host Service
cutover/live continuity/platform gates stay open; Windows ARM64 dependencies
unsupported/no native run and RSS unmeasured. The user requests convergence and
integration now; subsequent merged-source validation is recorded separately.

[Merged-source installed acceptance](../../../docs/evidence/runtime/20261116-integration-closeout/OUTCOME.md)
records rebuilt Box/OCI archives,5 Rust native tests plus3 Python scenarios and
zero remaining owned resources across9 homes. A parallel OCI warm-boot timeout
remains unresolved despite an unchanged isolated retry passing. Default BoxVm,
stopped-only HVF recovery, full W4/platform gates and Windows ARM64 limitations
remain unchanged; runtime RSS is unmeasured.

[Current native-tool diagnostics and installed assets](../../../docs/evidence/runtime/20261116-native-tool-diagnostics/OUTCOME.md)
retain bounded OS failure details and five passing installed native scenarios.
The full gate still fails six disk-service cases on this session; independent
OCI Host startup also requires DiskManagement recovery. Scoped six-home cleanup
passes. This does not qualify the whole release or change default routing.

Failed-boot diagnostics retain flat, descriptor-confined single-link logs under
`diag/failed-boots/<id>/`:64 files,1MiB tails and8MiB total, with a private observed
limit/skip report. Links and unsafe prior target trees are refused; cleanup waits
for blocking retention before removing the working directory. This is best-effort
evidence, not an atomic whole-tree snapshot or global retention quota.
[Source, native attacks and current archive](../../../docs/evidence/runtime/20261116-boot-diagnostics-boundaries/OUTCOME.md).

If failed-boot backend finalization refuses, VmManager retains the exact handler
and working/anonymous resources, with bounded diagnostics, for a later cleanup
retry. A process exit alone does not finalize its backend. Callers must retain
the manager until successful close; outer pool/drop-on-error lifetime remains an
open task. [Native effects and limitations](../../../docs/evidence/runtime/20261116-boot-stop-preservation/OUTCOME.md).

Pool template cleanup now uses remembered private descriptor-pinned allocations,
with VM consumer references released only after actual backend finalization or
proven prelaunch failure. Failed template sources remain owned for cleanup retry;
independent pools do not share image-hash RAM paths. Cleanup/rewrite refuses
active/unproven consumers and observed replacement, preserving unrelated caller
directories. [Installed Mac cold-pool, filesystem and cleanup evidence](../../../docs/evidence/runtime/20261116-pool-template-ownership/OUTCOME.md)
does not qualify Linux snapshot restore or close outer pool/legacy owner-loss paths.

Registered idle-pool teardown now retains the actual manager until backend and
host cleanup succeed, including after a cancelled Guest-control wait. Failed
rootfs cleanup preserves working data; failed created anonymous allocations keep
their exact retry identity. Stopped workload state alone does not prove host
cleanup, and a successful preserving stop still permits later ephemeral removal.
Pool/lease Stop errors keep owners available for retry; closed pools refuse new
and queued acquisition. [Final installed Native effects and limits](../../../docs/evidence/runtime/20261116-pool-shutdown-ownership/OUTCOME.md)
cover registered close, not initial endpoint binding or all pre-admission/legacy
manager lifetimes. Those remaining local gaps and whole GA stay open.

Pool CLI initial binding now preserves caller/live endpoints and requires exact
versioned listener ownership for stale recovery. On this Mac, a full live queue
returns ECONNREFUSED, so legacy/unproven endpoints refuse rather than being
unlinked. The [038 installed qualification](../../../docs/evidence/runtime/20261116-pool-socket-ownership/OUTCOME.md)
records endpoint/autostart/pool/AF proofs and53-home cleanup. This does not close
pre-admission/legacy manager lifetime, OCI pool parity/default cutover/W4/GA.

Published foreground pool acquire/release now registers actual owners through
health/stat waits and transfers only exclusive handles. Cancellation with an
unclosed registered owner closes existing admission; selected maintenance cannot
finalize unrelated handoffs. [039 installed evidence](../../../docs/evidence/runtime/20261116-pool-handoff-ownership/OUTCOME.md)
records real healthy handoff/cancel/refusal/actual retry,4Rust+3Python and32-home
audit. Internal boot/allocation/template/batch/unpublished/legacy ownership and
full W4/default OCI Host Service cutover/GA remain open.

Completed warm boot workers now return registered actual owners plus cancellation
guards; both batch consumers wait for idle/stat/admission before exact transfer,
and late close keeps actual owners for retry. [040 installed evidence](../../../docs/evidence/runtime/20261116-pool-batch-ownership/OUTCOME.md)
records native0/1->1/0,5Rust+3Python/16Ready and31-home audit. Internal incomplete
boot/template/unpublished/legacy and rollbackselection remain open, as do full
W4/default OCI Host Service cutover/GA.

Selected fill rollback now removes exact current BoxIds under one idle lock,
updates actual counts and registers selected managers before any later wait.
An already acquired or unrelated idle VM cannot become a stale-index target.
[041 installed evidence](../../../docs/evidence/runtime/20261116-pool-rollback-identity/OUTCOME.md)
records actual working-data deletion0/1->1/0,3Rust+3Python/16Ready/26home audit.
Drain detachment cancellation, incomplete boot/template/unpublished/legacy and
full W4/default OCI Host Service cutover/GA remain open.

Drain detachment cancellation

[042 installed evidence](../../../docs/evidence/runtime/20261116-pool-drain-detachment/OUTCOME.md)
Both public drain variants/template invalidation wait stats before detaching
actual idle owners. Actual0/3 becomes3/0 with typed refusal/retry intact.
Pool74/0/6,CLI949/Clippy; default2339/6/56 retains six disk failures. Final
Boxf5f41897…/sourcef522f929… is23427053 compressed/44208128 installed bytes.
Actual3Rust+3Python/4daemon/16Ready/4commands/26home audit pass. AF setup
failure before any native subprocess is retained. Cancelled maintenance join,
internal boot/template/unpublished/legacy/full OCI cutover/W4/GA remain open.
WindowsARM64 unsupported/no native run; levels unchanged; RSS unmeasured.
Git remote remains UID501/DNS blocked; local commit/bundle is not publication.

Cancelled maintenance join ownership

[043 installed evidence](../../../docs/evidence/runtime/20261116-pool-maintenance-join/OUTCOME.md)
Cancelled maintenance joins retain actual handles through both drain variants;
actual0/2->2/0. Pool76/0/7,CLI949/Clippy; default2341/6/57 keeps six disk failures.
Final Boxab310005…/source1353d5cf… is23427185 compressed/44208128 installed bytes.
Actual3Rust+3Python/4daemon/16Ready/4commands/25home audit pass. Legacy
handler publication/internal boot/template/unpublished/other owner/full OCI
cutover/W4/GA remain open. WindowsARM64 unsupported/no native run; support levels
unchanged, RSS unmeasured. GitUID501/DNS blocks remote sync, no publication.

Returned legacy handler ownership

[044 installed evidence](../../../docs/evidence/runtime/20261116-legacy-handler-publication/OUTCOME.md)
Returned legacy provider handlers survive installation cancellation in the
existing pending slot; boot/maintenance/default finalizers share safe adoption,
repeat acquisition/conflict refuse without owner/data loss. Actual0/1->1/0;
VM246/0/3,CLI949/Clippy; default2344/6/58 keeps six disk failures. Final
Boxd24cb38d…/source27b4f58e… is23428487 compressed/44208128 installed bytes.
Actual3Rust+3Python/4daemon/16Ready/4commands/26home audit pass; controlled
real handler handoff uses no rescue on PASS. Internal boot/unpublished/legacy/
from_pid/full OCI cutover/W4/GA remain open. WindowsARM64 unsupported/no native
run; support unchanged, RSS unmeasured. Fresh GitUID501/DNS still blocks push.

In-progress published pool boot owners

[045 installed evidence](../../../docs/evidence/runtime/20261116-pool-boot-owner/OUTCOME.md)
Published pool cold/restore managers register before boot waits and return
existing CompletedBoot values to foreground/batch callers; no double registration.
Actual0/1->1/0;Pool78/0/8,CLI949/Clippy;default2346/6/59 keeps six disk failures.
Final Boxd4a99066…/source8aec1191… is23428693 compressed/44208128 installed bytes.
Actual3Rust+3Python/4daemon/16Ready/4commands/28home audit pass; cancelled
fresh native boot retains actual manager/PID/allocation/writes, never inventsReady,
then real drain finalizes. Unpublished/queued shutdown/providerinner/legacy/
from_pid/full OCI cutover/W4/GA remain open. WindowsARM64 unsupported/no native
run; support unchanged, RSS unmeasured. GitUID501/DNS remains external.

Queued boot shutdown admission

[046 installed evidence](../../../docs/evidence/runtime/20261116-pool-boot-admission/OUTCOME.md)
Closed cold entry/queued template wait refuse before new claim/boot; restore
returns before fallback on closure. Canonical EM predicates unchanged. Actual
2pass/2fail->4/0;Pool80/0/9,CLI949/Clippy;default2348/6/60 keeps six disk failures.
Final Box0da9ec21…/source190148b3… is23428828 compressed/44224512 installed bytes.
Actual3RustVM+1Rustprelaunch-negative(0VM)+3Python/4daemon/16Ready/4commands/
27home audit pass. Unpublished/providerinner/legacy/from_pid/full OCI
cutover/W4/GA remain open. WindowsARM64 unsupported/no native run; support unchanged,
RSS unmeasured. GitUID501/DNS remains external; no publication.

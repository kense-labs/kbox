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

The provider pins `kbox_contract` 0.5.0 at
`aeb1dd96b5d3f823464d51fcabe99b6052874aee` and advertises its atomic
`ServiceLifecycle` feature. `RuntimeUnitSpec::health` remains readiness for
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

On macOS, a validated `ram://` root proves memory-backed storage only. Guest
environment/file Secrets still require host-enforced read-only virtio-fs
exports, which the current backend does not provide. Managed MicroVM creation
now refuses those mounts during preflight, before execution reservation or
container Secret resolution. A RAM-root capability probe is not proof that a
Secret-bearing workload can run. See the
[actual processing admission evidence](../../../../docs/evidence/desktop/20261017-paperloom-secret-readonly-admission/OUTCOME.md).

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

## Local engine and native guest profile

### Optional live directory grants

`RuntimeMountSource::Directory` names a mutable caller-owned directory through a
reference, independently of OCI image or ACP workload type. A plain
`BoxRuntimeDriver` does not expose it. Callers opt in with `BoxDirectoryPort`;
Box still owns the actual share, execution generation and teardown. Immutable
Artifact inputs keep their existing read-only contract.

The local engine advertises `directory-grants.v1` and accepts explicit grants
in the immutable intent. References bind exact paths; a grant must match every
Directory mount and cannot expand a read-only authorization to write access.
Resolution checks the full accepted Runtime specification and generation,
including previous attempts needed for teardown. Missing or moved caller paths
do not prevent retiring an already bound execution. No parent directory is
implicitly exposed, and removal retains caller data.

MicroVM mount controls encode non-ASCII and quoted paths across libkrun's
ASCII-only kernel command line; ordinary ASCII controls preserve their wire
form. Package the matching guest-init with the host runtime. The native macOS
directory and ordinary OCI results are recorded in
[assembly evidence](../../../docs/evidence/engine/20261111-assembly/OUTCOME.md).

`kbox engine serve` adds a same-user, owner-only Unix control endpoint over
`ManagedRuntimeClient` and `ExecutionManager`. It persists accepted intent and
immutable operation references; Runtime retains its receipts and Box retains
actual execution resources. Ordinary OCI CLI commands and Runtime consumers do
not require this endpoint, ACP, or KRun.

The explicit `kbox.native-guest.v1` policy is pinned by
`NATIVE_GUEST_PROFILE_DIGEST` in the Runtime specification's
`semantics_profile_digest`. It requires independent-kernel Sandbox isolation,
selects full guest capabilities, disables guest seccomp filtering and
`no-new-privileges`, and preserves explicitly requested resource, mount and
network policy. It grants no host mounts, devices or services. Other specs keep
the existing provider security defaults. The engine selects this policy for an
ACP Harness environment; it does not inspect ACP traffic or impose task-loop
limits. Credentials remain references and this profile does not authorize
inline secret storage.

The canonical LF bytes hashed by that identity are:

```text
kbox.native-guest.v1
boundary=microvm
seccomp=unconfined
capabilities=all-guest
no-new-privileges=false
resources=explicit
```

Native control-plane, OCI regression and guest-policy comparison evidence is
recorded in [OUTCOME](../../../docs/evidence/box/20261110-krun-engine/OUTCOME.md).
The separately discovered `process-stream.primary.v1` capability opens an
exclusive primary process per environment generation. An owner-only control
connection and a one-use data attachment keep disconnect detection independent
of data backpressure. Raw stdout / stderr, explicit stdin half-close and one
exact exit preserve the native process transport. Lost streams are not resumed
or replayed: their receipt and the next empty environment attempt are published
atomically, and Runtime retires the old resources before preparing the next
generation. This primary policy does not prohibit other execution sessions;
ordinary command / PTY / file APIs and unary Runtime `Exec` retain their contracts.

`process-stream.bootstrap.v1` adds a bounded opaque launch input on the
authenticated primary control connection. A client sets `bootstrap` in the
open request, receives its ticket, and sends a little-endian 32-bit byte count
and 1 byte to 4 MiB of data before attaching. The engine receives the complete
frame before launch and forwards it through the generation-bound guest stdin
before admitting the separate data stream. Bootstrap bytes stay out of the
serialized request, its fingerprint and the journal. The caller and trusted
guest launcher own the meaning of those bytes.

### Optional Harness package assembly

`harness-assembly.v1` is explicitly selected by an ACP Harness intent. The
engine verifies a bounded immutable ZIP copy on the host and streams it through
the existing generation-fenced process API to the trusted guest installer.
Package contents and hooks never execute on the host. The receipt binds the
accepted operation, Runtime specification and generation, package digest and
actual Box execution identity and generation. A running VM alone is not an
assembly receipt. Primary startup checks that binding again at the actual
execution boundary; superseded assembly cannot publish readiness.

An assembly may require a named guest protocol. The engine checks it against
the installer response before publishing the bound receipt. Existing assembly
plans without the field use the original readiness check.

`harness-assembly.guest-protocols.v1` adds the optional
`requiredGuestProtocols` list. Every requested protocol must appear in the
installer response. The original singular field remains readable and empty
lists are omitted from legacy wire data. A digest- and protocol-bound
`krun.guest-installation-error.v1` response can report a missing runtime
requirement. KBox records `RUNTIME_REQUIREMENTS_UNMET` and KRun reports failed
assembly. The trusted guest installer owns component version and file checks.

This optional path reserves `/opt/krun` for its trusted installer and published
packages, and uses an indefinite bootstrap process. Those requirements do not
apply to ordinary OCI or to intents that do not select package assembly. Legacy
intents omit the new optional field and keep their existing wire identity.
KRun stores plans and operation references; it does not own another VM registry.

Native three-package streaming, public KRun CLI and ordinary OCI regression
results are recorded in
[assembly evidence](../../../docs/evidence/engine/20261111-mate-assembly/OUTCOME.md).
That evidence does not qualify VM prewarm, memory credentials, arbitrary package
dependencies, real-model tasks or other native platforms.

### Canonical environment prewarm handoff

The local engine advertises `environment-prewarm.v1` for assembly intents
that opt in. ExecutionManager owns each idle reservation. The compatibility key
covers the pinned image, boot configuration, resource policy, exact directory
identities, capability bindings, platform, runtime asset generations and engine
authority. Compatible Harness packages can use the same idle environment.

After an opted-in instance becomes available, the engine keeps one compatible
spare and starts it through a generation-bound worker. The worker holds the
canonical lifecycle lock through startup and can finish if the engine exits.
Runtime apply claims a compatible running spare or uses the normal create path.
Drivers without prewarm composition and ordinary OCI requests use normal create.

The idle VM starts the trusted bootstrap. Unclaimed reservations permit
inventory and lifecycle teardown. The manager and direct CLI entry points
require claim before workload access, including exec, PTY, file and port
sessions. Claim atomically transfers ownership and advances the Box generation
while retaining the same physical runtime and exact OCI binding. The claimed
VM follows normal workload teardown, with its prewarm provenance kept for fencing.
For an instance with prewarm history, engine stop stays `CLEANUP_PENDING`
until its unclaimed reservations are removed, even after the workload is gone.

The explicit driver composition exercises HVF identity reuse, cross-unit claim,
external files and teardown. [Native handoff evidence](../../../docs/evidence/engine/20261111-prewarm-native/OUTCOME.md)
The production CLI test also exercises an engine restart during prewarm, a real
Aster-to-Dsh handoff, unclaimed access rejection and an ordinary OCI workload
running throughout the handoff. [Engine prewarm evidence](../../../docs/evidence/engine/20261111-prewarm-schedule/OUTCOME.md)

The independent OCI regression uses a prefix with only KBox and its runtime
assets. It covers pull, build, run, external files, volumes, HTTP, live export,
commit and run, snapshot restore and engine restart. [Native OCI evidence](../../../docs/evidence/engine/20261111-credential-bootstrap/OUTCOME.md)

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

## License

MIT

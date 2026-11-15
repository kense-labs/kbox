# Add one explicit combined contract

Existing Box plan_create_resources refuses managed Secret+anonymous raw volumes;
SDK MemorySecretFiles::from_bundle independently refuses raw storage. Protocol
chooses exclusivev7/raw,v8/files,v9/environment, optionally enclosed byv10/NPROC.
Core Secret intent and raw allocation identity already exist. Current Mac Host
pins memory source/projection and raw FDs independently; Guest source planning
already supports both inventories and protected optional environment bindings.

First red-test SDKv3 combination and protocol transport downgrade. Add a bounded
v3 constructor/exports and require actual raw intent; v1/v2 retain refusal. Use
v11 only for the combined inventory; retain exact old manifests and NPROC
envelope semantics. Advertise3 only from the existing RAM-capable HVF driver.
Update Box completion and preallocation capability checks; preserve request,
route/storage identities, generation and rollback cleanup under the manager.

Affected: SDK attachment module/exports/tests, Agent protocol manifest/raw
constructor/tests/exports, HVF capability probe, Box caller/spec/preparation/
tests and one native sibling/harness. Reuse existing Secret/file/raw/CLI native
helpers; do not add another manager, parser, configuration, dependency or native
source patch. Prove effects from regenerated installed Box/OCI/Agent bundles,
including old Host/shim/Agent refusal and unchanged legacy file/env/raw flows.

Run focused/owning/Guest/native/default checks with actual counters/results,
precise RAM mount ownership and source/storage cleanup; retain every failure,
candidate and source digest. OUTCOME/release/ledger closure precedes next W4.

Constitution Check: single ExecutionManager resource authority/fencing; typed
closed versions and fail-closed negotiation, KCL/one workspace, no plaintext
real Secret/evidence, positive Host PEP and native observations, clean native
submodules, no default/platform promotion. No waiver.

Native1 exposes a required correctness fix: disk-home to RAM handoff rootfs
publication used generic COW copy's dense cross-filesystem fallback, expanding
4GiB logical sparse capacity and failingENOSPC before VM launch. Mac publisher
now reuses existing rootfs/ext4_cache::copy_sparse_file (extent preserving,
create_new, bounded buffer, fails closed without sparse extent support). Other
platform paths retain existing behavior. Add a small actual publication test
for byte hash/holes/capacity/source/mode and repeat the installed cross-volume
native case at the same RAM capacity; do not inflate RAM to mask the bug.

Completed: explicit3/11, capability-before-allocation callers, sparse handoff
fix, installed Box2/OCI1 matrix/default gate, strict source/count review and
17-home/8-RAM audit. See [acceptance](../../docs/evidence/runtime/20261115-oci-raw-secrets/OUTCOME.md).
No external blocker for027; continue existing custom root disk-capacity slice.

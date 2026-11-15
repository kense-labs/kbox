# Implementation findings

Existing same-UID directory gate fails foreign UID before launch. Baseline command `cargo test --locked -p kbox-runtime admit_same_uid_rejects_foreign_uid`: 2 passed, 0 failed (2026-11-14); raw log retained in this run artifact root. This demonstrates safe refusal, not ownership support.

LogicalRootfs already retains numeric uid/gid/mode and links in a bounded Linux namespace, uses opaque host spool files and verified layer digests, and publishes ext4 atomically. Existing Box provider injects guest-init and must not be used as-is for OCI workload root.

macOS Host already uses krun_add_disk2 for its immutable system image. Agent already inventories virtio block serial/size/readonly state and validates exact target/config attachment manifests. Reuse these boundaries for a distinct workload disk. Old directory semantics remain explicit.

Prior archives and source hashes are saved before edits. No external blocker remained. The second workload disk was measured through actual HVF execution, numeric stat/permissions, independent writes, restart and stopped recovery. Final installed attempt3 passed 1/0/0 with CLI enabled; A–F replay2 passed all five scenarios. Compilation alone was never counted as native acceptance.

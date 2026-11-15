# Caller-file metadata compatibility

Status: qualified for the bounded macOS arm64 profile using final installed
Box2/OCI7; [outcome](../../docs/evidence/runtime/20261114-oci-file-rollback/OUTCOME.md).
Owner: Box Runtime.
Prerequisite: spec009 final installed acceptance is retained and complete.

An older Box reader may ignore caller_files, reclassify a replaced file pathname
as a directory and discard the stored file authority when rewriting the record.
Actual accepted spec008 CLI reproduced directory exposure and binding loss;
the final installed matrix proves the repaired boundary.

User stories / independent tests:
1. A file-bearing reservation must refuse an unaware older lifecycle reader
   before VM activation or authority loss. Run the actual old installed CLI in
   a fresh isolated home against current independent Host assets; preserve the
   original file, replacement directory and canary, state bytes and Host objects.
2. Current readers retain existing spec009 identities, routes, generations and
   replay while installing an explicit compatibility fence atomically. Hermetic
   tests cover forward adoption, no-file legacy records, malformed/missing
   bindings, retry, restart and forbidden authority changes.

Scope: existing metadata, BoxStateStore/ManagedExecutionStore, resource planning,
owning tests and installed native archive replay. Reuse existing atomic state
locking and version checks. No new authority/state machine, configuration,
dependency, route/default change or native submodule edit. Unsupported rollback
must be recorded honestly; arbitrary immutable old binaries cannot be repaired
by pretending they understand a new field.

Success: retained before/after reproduction; focused and full owning gates,
verify-commit; rebuilt affected installed archive with actual old/new reader
matrix and source-preserving owned cleanup. Evidence:
docs/evidence/runtime/20261114-oci-file-rollback/OUTCOME.md.

House checks: hermetic + native tests; ExecutionManager owns resources;
compatibility is explicit and fail closed; no credential/DataRef/config change;
actual Host effects distinct from error responses; unchanged platform claims.
Not claimed: full rollback compatibility, live continuity, complete W4,
fresh-host/other-platform qualification, Windows ARM64, release promotion.

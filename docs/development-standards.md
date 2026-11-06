# kbox Development Standards

`AGENTS.md` is the actionable entry point. `.specify/memory/constitution.md` defines the non-waivable baseline. This document explains how changes are designed, verified and released.

## 1. Change design

1. State the requested behavior and the owning path.
2. Inspect the entry point, direct callers and current failure behavior.
3. Reuse existing contracts and adapters.
4. Record assumptions, non-goals and unsupported combinations.
5. Implement the smallest complete change with its tests.
6. Review the final diff and preserve unrelated work.

A bug fix starts with a failing test, reproduction or log. A feature starts with a typed contract and acceptance scenario. A migration includes source provenance, compatibility rules and a rollback path.

## 2. Ownership boundaries

Box ExecutionManager owns VM processes, mounts, networks and execution resources. ManagedRuntimeClient owns Task／Service receipts and observations. Runtime code must reconcile through provider inspection rather than infer state from a local copy.

ACP Session Manager owns ACP session mappings. Lifecycle Controller converts ResourceSpec into provider operations. Service Registry owns endpoint health. Capability Broker selects ProviderBinding only after schema, policy and health checks.

Data Lineage Controller owns DataRef relationships and deletion state. Actual Providers execute deletion and return receipts. PEPs execute intervention and return effects.

## 3. Error and admission contracts

- Validate external input once at the entry boundary.
- Use enums, newtypes and builders for admitted states.
- Refuse unknown security-critical values.
- Return typed errors with sources and stable codes.
- Do not coerce an unsupported request into a lower-privilege success.
- Do not convert a missing structured event into plain text success.
- Bound request bytes, decompression, object counts and work before allocation.

## 4. Rust rules

- Preserve public Rust APIs unless the task requires a change.
- Keep sensitive fields private and redact custom formatters.
- Avoid production panics for input, I/O and cancellation failures.
- Document every handwritten `unsafe` invariant with `SAFETY:`.
- Use structured tracing and avoid high-cardinality labels.
- Keep blocking operations off async executor threads.
- Put unit tests in sibling files and public API tests in `tests/`.
- Do not copy implementation logic into a test oracle.

## 5. Configuration and data contracts

Product behavior is configured in ACL parsed by `kacl`. Image manifests, AI-BOM, event schemas and ProviderBinding records are versioned data, not a second configuration format.

New enums implement a closed parse and representation contract. Schema compatibility rules state which optional fields can be ignored and which unknown fields must fail. Secrets are represented by references only.

## 6. Data security

- Assign an independent identity to each workload, Run and Replica.
- Scope credentials to principal, task, capability, DataRef and generation.
- Narrow permissions during delegation and revoke them at task completion.
- Treat external content as untrusted until an authorized transition.
- Encrypt memory and checkpoints by tenant and retain source provenance.
- Enforce data classification, residency, retention and training use before Provider routing.
- Track derived data through summaries, embeddings, indexes, memory and artifacts.
- Return actual deletion evidence from each Provider.
- Keep full reasoning traces, raw prompts and sensitive payloads out of default evidence.

## 7. Observability and intervention

Use typed events with tenant, run, replica, generation, trace and DataRef correlation. Validate event ownership and ordering before storage.

Authorization PDP makes synchronous access decisions. Data Security PDP computes data decisions and obligations. Behavioral decision logic evaluates observed events. Only a PEP performs the resulting action.

`Decision.block` never substitutes for an actual PEP Result. Unknown intervention results are queried, not retried blindly.

## 8. Cross-platform rules

Use platform path and process types. Preserve target CPU, ABI, minimum-OS, libc and linker baselines. Isolate OS-specific code with `cfg` and existing adapters.

Use UTF-8 without BOM and LF for normal source text. Windows command-file checkouts follow CRLF rules. Binary fixtures and evidence remain exact. Do not renormalize a whole file to hide unrelated edits.

Linux KVM, macOS HVF and Windows WHPX claims require separate native evidence. A successful build on one host does not establish support on another.

## 9. Test requirements

| Change | Required tests |
| --- | --- |
| Contract or state machine | Hermetic unit and serde fixtures |
| Caller or public API | Hermetic integration through the real entry point |
| Backend lifecycle | Focused fake-driver tests plus native backend tests for claims |
| ACP or Plugin protocol | Real protocol fixtures with cancellation and unsupported events |
| Data security | Adversarial cases for injection, delegation, secrets, routing and deletion |
| Recovery | SIGKILL, restart, partial operation and stale-generation cases |
| Release | Isolated-prefix installation and final-archive smoke |

Required commands for handwritten Rust:

```bash
cargo fmt --all -- --check
cargo test --locked -p <owning-package> <focused-filter>
cargo clippy --locked -p <owning-package> --all-targets --no-deps
```

Workspace integration changes also run:

```bash
cargo check --workspace --all-targets --locked
```

## 10. Evidence

Every behavior change records `docs/evidence/<domain>/<date-slug>/OUTCOME.md` with:

- Result and scope.
- Actual commands and counts.
- Toolchain, OS, architecture, backend, features and image digest where relevant.
- Logs and artifact locations.
- Compatibility and migration results.
- Explicitly not claimed limitations.

Do not report an unrun check as passed. Do not use evidence from another backend or an older artifact.

## 11. Performance and size

Measure before optimizing. Record cold start, authenticated execution, Plugin readiness, call latency, cancel cleanup, recovery, branching, memory, disk and evidence overhead as applicable.

Comparisons use the same host, image digest, backend, network and persistence settings. Include raw results and P50／P95／P99 where the sample supports them.

The 40–50 MB target covers only the minimal Runtime bootstrap package. Workload images, kernel, rootfs, models, indexes and caches are reported separately.

## 12. Submodules

Change native sources in `kense-labs/libkrun` or `kense-labs/libkrunfw`. Test the source change there, update the parent gitlink, then run the parent build.

Do not commit submodule working-tree files in the parent. Keep the packaged source fallback, checksum and provenance intact when a checkout without submodules must remain supported.

## 13. Release

Before release:

1. Run formatting, focused tests, Clippy and applicable workspace checks.
2. Run native tests for every platform named in the support matrix.
3. Check all required runtime and native companions in the final archive.
4. Install into an isolated prefix and run real smoke scenarios.
5. Verify cancel, restore, cleanup and rollback behavior.
6. Record support limits, migration steps and Explicitly not claimed items.

Do not commit unless the user asks. Stage only the task's files and inspect the staged list before committing.

## 14. Spec-driven development

Bounded epics use `docs/spec-driven-development.md` and `.specify/templates/`. Routine fixes do not create planning scaffolding. Planning and acceptance are separate records.

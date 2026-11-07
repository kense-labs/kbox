# kbox Constitution

This document is the enforceable engineering baseline for the repository. Actionable implementation rules live in `AGENTS.md` and `docs/development-standards.md`. A principle marked **NON-NEGOTIABLE** cannot be waived by a spec, plan, task or deadline.

## Regimes

### Execution regime

Paths: `crates/box`, `crates/oci-runtime`, `crates/runtime`.

New behavior has a hermetic test in the owning package. Backend claims also need the relevant native execution test. ExecutionManager remains the actual resource authority, and every restore or retry preserves generation fencing.

### Policy regime

Paths: `../kcl`.

Policy changes have deterministic tests for allow, refuse and unsupported cases. Security tests include an adversarial case. Configuration stays in KCL and behavioral decisions remain separate from actual enforcement.

### Native dependency regime

Paths: root `libkrun` and `libkrunfw` gitlinks.

Source changes land in the owning repository. The parent records only the tested gitlink update. The parent build must prove that the selected source or packaged fallback is complete and compatible.

### Supporting-crate regime

Paths: other `crates/*` directories.

Reuse the owning package's public API and existing test style. Do not move packages, create a second workspace or expand a dependency solely for convenience.

## Principles

### I. Fail Closed

Refused conditions have typed reasons. Missing required values refuse activation. Unsupported protocols, backends, capabilities and interventions return explicit errors. No silent fallback increases privilege, exposure or compatibility claims. **NON-NEGOTIABLE.**

### II. One Resource Authority

Box ExecutionManager owns actual execution resources. ManagedRuntimeClient owns operation receipts and observations. ACP sessions, Plugin endpoints, Provider bindings and data lineage have separate named owners. No second execution lifecycle store may contradict the backend. **NON-NEGOTIABLE.**

### III. Evidence Retained

A behavior change records `docs/evidence/<domain>/<date-slug>/OUTCOME.md` with commands, counts, environment and an Explicitly not claimed section. Evidence is not deleted to improve a result. **NON-NEGOTIABLE.**

### IV. Tests Follow the Regime

Execution and policy changes use small hermetic tests. Native runtime claims require native tests on the named host. Cross-compilation, mocks, ignored tests and startup alone do not establish runtime support. **NON-NEGOTIABLE.**

### V. Closed Sets and Typed Contracts

Admitted values use enums and invariant-bearing types. Unknown security-critical values are refused at the boundary. Errors are matched by type or stable code. **NON-NEGOTIABLE.**

### VI. One Workspace and Local APIs

The root Cargo workspace and lockfile are shared. Internal `k*` dependencies resolve locally and remain `publish = false`. Cross-crate calls use public APIs. Submodule repositories keep their own build contracts. **NON-NEGOTIABLE.**

### VII. One Configuration Language

KCL parsed by `kcl` is the only product configuration language. Data schemas and manifests are versioned data contracts and cannot alter permission discovery or provider selection by themselves. **NON-NEGOTIABLE.**

### VIII. Replaceable Backends and Transports

Consumers depend on typed capabilities, not transport details. Provider errors do not trigger another backend, isolation level, residency zone or trust domain. A capability that cannot be enforced is rejected.

### IX. Data and Credential Protection

Credentials are scoped to principal, task, capability, DataRef and generation. Delegation narrows permission. Sensitive payloads do not enter logs, prompts, memory, checkpoints or evidence. Provider routing enforces classification and residency before availability. **NON-NEGOTIABLE.**

### X. Decisions Are Not Effects

ObservedEvent, Decision, Intervention and PEP Result are distinct records. Only the actual execution point can establish an intervention effect. Audit does not replace isolation or enforcement. **NON-NEGOTIABLE.**

### XI. Independent Judgment and Adversarial Review

State contradictions with reasoning. Try to break the change before completion. A confirmed unresolved finding names its exact follow-up and coverage limit. Pressure does not replace evidence. **NON-NEGOTIABLE.**

### XII. Repository and Submodule Boundaries

New code lands in this repository unless the user explicitly authorizes another repository. Native dependency changes land in the corresponding submodule repository before the gitlink changes. Parent commits never include submodule working-tree files. **NON-NEGOTIABLE.**

### XIII. Names and Meaning Are Not Guessed

Identifiers come from typed models, quoted source text or exact stored values. Regexes and word lists do not infer user intent, identity, authorization or entity names. Mechanical parsing remains fail closed. **NON-NEGOTIABLE.**

### XIV. Native Compatibility Claims

Supported CPU, ABI, minimum-OS and backend baselines stay unchanged unless the task explicitly changes them. Every affected native target has its own evidence and limitations. **NON-NEGOTIABLE.**

### XV. Verification Gates Only Tighten

Tests, security policies and release gates are not weakened to obtain a pass. Existing allowlists used as debt ratchets may only shrink. A failing gate is fixed or recorded as a limitation. **NON-NEGOTIABLE.**

## Constraints

- Rust toolchain is fixed by `rust-toolchain.toml`.
- Production `unwrap()` and `expect()` require a construction-level infallibility argument.
- Credential comparisons use constant-time equality where applicable.
- Blocking I/O and long CPU work stay off async executor threads.
- Generated files are regenerated from source, not hand-edited.
- UTF-8/LF is the default source format. Windows command files follow `.gitattributes` CRLF rules.
- Commits require explicit user authorization. Stage only the task's files and inspect `git diff --cached --name-only`.

## Development Workflow

Routine fixes land with focused tests and required evidence. Bounded epics use `specs/<NNN-slug>/` with the house templates. Planning artifacts stay in `specs/`; acceptance evidence stays in `docs/evidence/`.

Constitution changes follow observed engineering evidence. A phase that requires a NON-NEGOTIABLE violation stops for review rather than editing the gate to pass.

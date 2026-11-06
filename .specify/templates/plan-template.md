# Implementation Plan: [FEATURE]

**Branch**: `[###-feature-name]` | **Spec**: `specs/[###-feature-name]/spec.md`

**Owning path**: [Crate, application or submodule]

## Summary

[Describe what lands and which existing module is extended.]

## Constitution Check

*GATE: Re-read `.specify/memory/constitution.md`. A NON-NEGOTIABLE violation stops the plan.*

| Principle | Status | Evidence in the plan |
| --- | --- | --- |
| I. Fail closed | [PASS/VIOLATES] | [Refusal paths] |
| II. One resource authority | [PASS/VIOLATES] | [Owner and reconciliation] |
| III. Evidence retained | [PASS/VIOLATES] | [OUTCOME target] |
| IV. Tests follow the regime | [PASS/VIOLATES] | [Hermetic and native targets] |
| V. Closed sets and typed contracts | [PASS/VIOLATES] | [Types and schemas] |
| VI. One workspace and local APIs | [PASS/VIOLATES] | [Packages and dependencies] |
| VII. One configuration language | [PASS/VIOLATES/N-A] | [ACL changes] |
| VIII. Replaceable backends and transports | [PASS/VIOLATES/N-A] | [Capability and routing surface] |
| IX. Data and credential protection | [PASS/VIOLATES/N-A] | [Identity, DataRef and redaction] |
| X. Decisions are not effects | [PASS/VIOLATES/N-A] | [PEP result evidence] |
| XII. Repository and submodule boundaries | [PASS/VIOLATES] | [Files and gitlinks] |
| XIII. Names and meaning are not guessed | [PASS/VIOLATES/N-A] | [Name source] |
| XIV. Native compatibility claims | [PASS/VIOLATES/N-A] | [Native targets] |
| XV. Verification gates only tighten | [PASS/VIOLATES] | [Affected gates] |

## Project Structure

### Documentation

```text
specs/[###-feature-name]/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── contracts/
└── tasks.md
```

### Source Code

| Layer | Path | Changes |
| --- | --- | --- |
| Execution | `crates/box/` or `crates/oci-runtime/` | [Files] |
| Runtime contracts | `crates/runtime/` | [Files] |
| Policy | `crates/acl/` | [Files] |
| Supporting crate | `crates/<name>/` | [Files] |
| Native submodule | `libkrun` or `libkrunfw` gitlink | [Owning commit and pointer] |

## Research and Reuse

For each decision, name the existing file or contract reused. Explain why a new pattern is necessary if no suitable prior art exists.

## Contracts and Failure Behavior

- [Typed inputs and outputs]
- [Identity, generation and ownership]
- [Unsupported combinations and stable errors]
- [Cancellation, restart and reconciliation]

## Verification Plan

- Hermetic tests: [package and filter]
- Protocol or integration tests: [fixtures]
- Adversarial tests: [cases]
- Native tests: [host, backend and image]
- Workspace checks: [commands]
- Evidence: `docs/evidence/<domain>/<date-slug>/OUTCOME.md`
- Explicitly not run or claimed: [limits]

## Delivery Sequence

1. [Contracts and tests]
2. [Implementation and callers]
3. [Native qualification and evidence]
4. [Final diff and support-matrix review]

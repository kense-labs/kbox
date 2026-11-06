# Spec-Driven Development

kbox uses an adapted spec-kit workflow for bounded epics. House templates and the constitution live in this repository. Do not run an upstream installer.

## Artifacts

```text
.specify/
├── memory/constitution.md
└── templates/
    ├── spec-template.md
    ├── plan-template.md
    └── tasks-template.md

specs/<NNN-slug>/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── contracts/
└── tasks.md

docs/evidence/<domain>/<date-slug>/OUTCOME.md
```

`specs/` stores planning. `docs/evidence/` stores acceptance evidence.

## Phases

| Phase | Result | Required artifact |
| --- | --- | --- |
| constitution | Establish or amend engineering constraints | `.specify/memory/constitution.md` |
| specify | Define user scenarios, requirements and acceptance | `specs/<NNN>/spec.md` |
| clarify | Resolve material ambiguity and record decisions | Marker-free `spec.md` |
| plan | Choose implementation and prove the Constitution Check | `plan.md`, `research.md`, contracts |
| tasks | Break work into ordered, test-bearing tasks | `tasks.md` |
| analyze | Check consistency without editing artifacts | Findings table |
| implement | Land code, tests and evidence | Source and `OUTCOME.md` |

Host command names may use `/speckit-<phase>` or `$speckit-<phase>`. Resolve all paths from the repository root and read `AGENTS.md` before editing.

## When to use the workflow

Use all phases for work that spans sessions, crates, native backends or security boundaries. Use `specs/<NNN-slug>/` as the epic's durable planning memory.

Do not create a spec directory for a one-slice fix. Routine fixes use focused tests and required evidence.

Constitution amendments follow observed behavior and evidence. A deadline does not waive a NON-NEGOTIABLE principle.

## Phase rules

### Specify

- Name the owning crate or application.
- Give each user story an independent test.
- Include refusal and adversarial scenarios.
- State unsupported platforms and explicit non-goals.

### Clarify

Resolve only ambiguity that changes implementation or acceptance. Record assumptions and rejected alternatives. Do not leave clarification markers in the final spec.

### Plan

- Read `.specify/memory/constitution.md`.
- Complete the Constitution Check with evidence links.
- Name existing code reused by the implementation.
- Define resource ownership, data contracts and failure behavior.
- Include native verification for platform claims.

A NON-NEGOTIABLE violation stops the plan.

### Tasks

- Include exact paths and test obligations in every behavior task.
- Mark parallel work only when files and dependencies are independent.
- Include callers, configuration, fixtures and exports in scope.
- Keep evidence in the task that produces the behavior.

### Analyze

Review spec, plan and tasks read-only. Check contradictions, missing refusal paths, ownership drift, incomplete tests and unsupported claims. Do not edit artifacts during analysis.

### Implement

Follow `AGENTS.md` and the owning path regime. Run focused tests, Clippy and integration checks. Record actual results and limitations in `docs/evidence/`.

Do not commit unless the user asks. Stage only the task's files and inspect `git diff --cached --name-only` before committing.

## Non-waivable constraints

Fail closed. Preserve one resource authority. Retain evidence. Use typed closed sets. Keep one workspace and ACL. Protect data and credentials. Separate decisions from effects. Preserve submodule boundaries. Require native evidence for native claims. Never weaken a gate to pass.

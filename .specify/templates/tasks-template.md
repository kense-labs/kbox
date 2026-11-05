---
description: "Task list template for feature implementation"
---

# Tasks: [FEATURE NAME]

**Input**: Design documents from `specs/[###-feature-name]/`

**Prerequisites**: `spec.md` and `plan.md`, including a passing Constitution Check

**House rule**: every behavior task includes its tests and required evidence in the same task.

## Format

`[ID] [P?] [Story] Description with exact paths`

- **[P]**: Independent files and dependencies allow parallel work.
- **[Story]**: User story served by the task.

## Path Conventions

- Execution: `crates/box/`, `crates/oci-runtime/`
- Runtime contracts: `crates/runtime/`
- Policy: `apps/gateway/`, `crates/sentry/`, `crates/acl/`
- Native dependencies: root `libkrun` and `libkrunfw` gitlinks
- Evidence: `docs/evidence/<domain>/<date-slug>/OUTCOME.md`

## Phase 1: Contracts and refusal tests

- [ ] T001 [US1] Add typed contracts and hermetic acceptance tests in `[paths]`
- [ ] T002 [P] [US1] Add malformed, unsupported and adversarial cases in `[paths]`

## Phase 2: Implementation and wiring

- [ ] T003 [US1] Implement the behavior and necessary callers in `[paths]`
- [ ] T004 [US1] Verify ownership, generation fencing, cancellation and cleanup in `[paths]`
- [ ] T005 [US1] Verify data redaction, Provider constraints and PEP effect evidence in `[paths]`

## Phase 3: Native verification and evidence

- [ ] T006 [US1] Run native tests for each claimed target and record results in `OUTCOME.md`
- [ ] T007 Run formatting, focused tests, Clippy and applicable workspace checks
- [ ] T008 Record commands, counts, environment and Explicitly not claimed in `OUTCOME.md`
- [ ] T009 Review the final diff and inspect `git diff --cached --name-only` before any user-requested commit

## Final Verification Gate

- [ ] The owning package's focused tests pass
- [ ] `cargo fmt --all -- --check` passes for Rust edits
- [ ] Applicable native and workspace checks pass
- [ ] Internal dependencies stay on local paths
- [ ] Product configuration remains ACL
- [ ] No decision is reported as an actual effect without PEP evidence
- [ ] Support claims match actual native results

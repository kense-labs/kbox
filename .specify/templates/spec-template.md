# Feature Specification: [FEATURE NAME]

**Feature Branch**: `[###-feature-name]`

**Created**: [DATE]

**Status**: Draft

**Input**: User description: "$ARGUMENTS"

**Owning path**: [Crate, application or submodule]

## User Scenarios & Testing *(mandatory)*

### User Story 1 - [Title] (Priority: P1)

[Describe the user journey.]

**Why this priority**: [Value and priority.]

**Independent Test**: [Package, test target and assertion that proves this story.]

**Acceptance Scenarios**:

1. **Given** [initial state], **When** [action], **Then** [observable result]
2. **Given** [unsupported state], **When** [action], **Then** [typed refusal]

### User Story 2 - [Title] (Priority: P2)

[Describe the user journey.]

**Independent Test**: [Standalone test description.]

**Acceptance Scenarios**:

1. **Given** [initial state], **When** [action], **Then** [expected result]

### Edge and Adversarial Cases

- [Unsupported protocol, backend or capability]
- [Stale generation, cancellation or partial completion]
- [Injection, excessive permission, secret exposure or data-residency violation]

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST [capability and observable result]
- **FR-002**: System MUST refuse [abuse case] with [typed reason]

### Ownership and Entities

- **[Entity]**: [Meaning and authoritative owner]
- **[Binding]**: [Identity, generation, schema and compatibility rules]

## House Checklist *(mandatory)*

- [ ] New behavior has a small hermetic test in the owning package
- [ ] Resource ownership and generation fencing are explicit
- [ ] New admitted values use closed typed sets
- [ ] Product configuration, if any, is KCL parsed by `kcl`
- [ ] Secret and DataRef flows have redaction and lineage tests
- [ ] Decisions and actual PEP effects are separate
- [ ] Native claims have native test targets
- [ ] Evidence target is `docs/evidence/<domain>/<date-slug>/OUTCOME.md`

## Success Criteria *(mandatory)*

- **SC-001**: [Focused suite and required count]
- **SC-002**: [End-to-end or native scenario and observable result]

## Assumptions

- [Assumption]

## Explicitly Not Claimed

- [Unsupported platform, protocol, backend or performance claim]

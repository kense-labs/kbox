# Spec050: sandbox isolation backend (fused upstream engine, debranded)

## Goal

Make KBox isolation-graded: default **sandbox** runs a fused, debranded engine
vendored from a designated Apache-2.0 upstream; optional higher isolation remains
**libkrun / MicroVM**. Same KBox product surface. Code and docs contain **zero**
upstream product-brand strings. 「做成」= daily installable with default sandbox
working. Gate green / exit 77 ≠ Pass.

## User stories

1. **Default run**: After install on a primary platform, `kbox run` (or equivalent
   Docker-shaped path) completes an OCI workload on the **sandbox** backend
   without selecting MicroVM.
2. **Opt-up**: Caller can select MicroVM / libkrun isolation for a stronger
   boundary; provider failure must not silently downgrade isolation.
3. **Zero brand residue**: Source, docs, evidence, NOTICE headers, CLI help,
   package names, domains, and paths contain no upstream product brand.
4. **One authority**: Box ExecutionManager remains the resource owner; fused
   engine does not create a second lifecycle store that contradicts it.

## Requirements

- R1: `IsolationLevel::Sandbox` routes to the fused sandbox engine.
- R2: Default isolation for CLI/product paths is Sandbox once the engine can run.
- R3: MicroVM/libkrun remains available as higher isolation (not deleted).
- R4: Source is fused into this workspace (no second Cargo.lock / workspace).
- R5: Apache-2.0 provenance retained (LICENSE/NOTICE per vendored crate) without
  upstream product-brand wording.
- R6: `python3 scripts/check-sandbox-brand.py` exits 0.
- R7: Evidence under `docs/evidence/runtime/20261117-sandbox-isolation-backend/`.

## Non-goals

- Shipping a separate upstream-branded app/UI/menu-bar product.
- Expanding into Kubernetes / Finder integration unless later GOALS expand.
- Deleting libkrun or claiming GA Pass from compile-only.
- Touching unrelated in-flight pool-oneshot-discard work.

## Acceptance

- Hermetic tests: isolation routing Sandbox vs MicroVM; refuse silent downgrade.
- Native (primary platform): install/smoke that default sandbox runs an OCI
  hello-path; MicroVM still selectable.
- Brand scan: `scripts/check-sandbox-brand.py` clean.
- OUTCOME.md with commands, counts, Explicitly not claimed.

## Platforms

- Primary claim: macOS (Virtualization path as provided by fused engine) and
  Linux host path as upstream supports.
- Windows/WSL: record limitation until native evidence exists; no silent claim.

# Implementation Plan: 050 sandbox isolation backend

**Branch**: `main` (P17: no private branch) | **Spec**: `specs/050-sandbox-isolation-backend/`

**Owning path**: `crates/box/sandbox/*`, `crates/box/core`, `crates/box/runtime`, `crates/box/cli`

## Summary

Vendor a debranded upstream Apache-2.0 container engine under
`crates/box/sandbox/`, add `ManagedRuntimeRoute::Sandbox` and make
`ExecutionBoundary` a real selector (`Sandbox` | `Microvm`), wire a
`LocalExecutionBackend` adapter owned by ExecutionManager, then flip
CLI/product default to Sandbox with evidence. Code/docs must stay brand-clean
(`scripts/check-sandbox-brand.py`).

## Constitution Check

| Principle | Status | Evidence in the plan |
| --- | --- | --- |
| I. Fail closed | PASS | No default flip until sandbox run works; refuse silent isolation downgrade |
| II. One resource authority | PASS | Adapter under ExecutionManager; no second lifecycle store |
| III. Evidence retained | PASS | `docs/evidence/runtime/20261117-sandbox-isolation-backend/OUTCOME.md` |
| IV. Tests follow the regime | PASS | Hermetic routing tests + native smoke when claimed |
| V. Closed sets and typed contracts | PASS | Enum variants for boundary/route; unknown refused |
| VI. One workspace and local APIs | PASS | Members under root workspace; path deps |
| VII. One configuration language | N-A | No new product config language |
| VIII. Replaceable backends | PASS | Isolation selects backend; no silent fallback |
| IX. Data and credential protection | PASS | Preserve redaction |
| X. Decisions are not effects | N-A | |
| XII. Repository boundaries | PASS | Vendor into this repo; Apache NOTICE retained without brand |
| XIII. Names not guessed | PASS | Brand replacements from explicit maps |
| XIV. Native compatibility claims | PASS | Platform claims only with native evidence |
| XV. Gates only tighten | PASS | Brand scan is additive; no weaken tests |

## Ordered slices

1. Spec/research (this dir).
2. Scaffold workspace members + mechanical rebrand (compile) + brand gate.
3. Route types + hermetic tests (still MicroVM default).
4. Adapter + minimal run path.
5. Flip default + brand scan + native smoke evidence.

## Conflict

- Do **not** modify `049-pool-oneshot-discard` dirty files.
- Stay on `main`; no private worktree (P17).

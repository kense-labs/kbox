# Feature Specification: KRun environment assembly

> Historical scope, superseded by the generic engine boundary in `docs/architecture/local-engine.md`. Product-specific engine behavior and package assembly have been removed.


**Created**: 2026-11-11
**Status**: In progress
**Owning path**: `crates/box/runtime`, `crates/runtime`; independent client/guest tooling in `../krun`.
**Input**: KBox prewarms shared KRun environments and dynamically assembles versioned ACP ZIP packages; Harness capabilities and ordinary OCI remain intact.

## User Scenarios & Testing

### US1 — Explicit live directories (P1)

A caller grants its workspace and selected external directories to an environment.
Mutable directories use a dedicated typed mount capability, not an immutable Artifact.
The actual Box generation mounts the exact grant, preserving native path semantics.

Independent test: Runtime contract and Box directory-port tests plus a native VM
writing/renaming/hash-checking files under Unicode/space paths and an unmounted canary.
An unconfigured port or unenforceable read-only grant must refuse before VM creation.

### US2 — Shared base and private package assembly (P1)

KRun submits a fixed base, package digest/path and explicit bindings. KBox starts
or leases the compatible environment, streams the package to its trusted guest
assembler, and publishes a generation/provider-bound assembly receipt. A real
ACP process starts only after assembly succeeds. Package hooks never run on host.

Independent test: three versioned ZIPs use the same base through `up/connect/down`;
package replacement, unsafe links, interrupted transfer, stale generation and
incomplete extraction never produce an available receipt.

### US3 — Native credentials and lifecycle (P1)

Credentials enter guest RAM/tmpfs through a separate generation-bound control
operation. Secret references alone enter durable plans. Native refresh has an
explicit owner. EOF/down retires private resources; persistent user data remains.
Prewarm keys include bindings/resources/trust and exclude Harness identity.

## Requirements

- FR-001: ExecutionManager owns physical resources; MRC owns Runtime receipts.
- FR-002: Directory references are resolved only by a caller-authorized port;
  no default arbitrary-host-path access and no relaxation of Artifact read-only rules.
- FR-003: Reuse process streaming for large archives, with digest verification
  and private atomic extraction. Assembly receipt must bind the actual generation.
- FR-004: KRun CLI exposes the existing lifecycle contract and reports available
  only with matching Running and successful assembly observations.
- FR-005: No task/turn/tool/PID caps are introduced; native arguments, extensions,
  filesystem, subagents and Harness loops remain under the Harness.
- FR-006: Ordinary OCI remains usable without any assembly, ACP or KRun component.
- FR-007: Product configuration remains KCL; ZIP manifests and IPC are data contracts.
- FR-008: Optional `harness-assembly.capability-packages.v1` accepts an ordered,
  digest-fixed set of capability archives. Every archive uses the same bounded
  streaming and guest validation path; no aggregate receipt is published until
  every package belongs to the same actual Box execution and generation. KBox
  does not interpret Skill, MCP, tool, plugin, model or vendor configuration.

## Edge and Adversarial Cases

Wrong/unknown capability, changed directory reference, source symlink, invalid
package/hash/platform, partial frame, lost receipt, concurrent replace/down,
late old-generation result, secret leakage and shared-service ownership conflict.

## House Checklist

- [ ] Focused contract, provider, guest and CLI tests
- [ ] Generation-fenced effects and receipt identity
- [ ] Closed input types and redaction
- [ ] Native HVF proof and ordinary OCI regression
- [ ] Evidence: `docs/evidence/engine/20261111-assembly/OUTCOME.md`

## Success Criteria

Three native ZIPs complete the formal CLI flow and one real-model task each,
with correct live external-file results and zero private execution leftovers.
The wider M3–M5 acceptance gates remain in the independent KRun plan.

## Explicitly Not Claimed

Planning does not establish implementation, platform support, complete Harness
acceptance or performance. Existing unsupported RO mounts are not downgraded.

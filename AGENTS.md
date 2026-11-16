# kbox

kbox is one Rust workspace for OCI execution, MicroVM resources, Agent Runtime contracts, model traffic, and runtime security. Root `libkrun` and `libkrunfw` directories are Git submodules with their own repositories.

## Execution and completion

- Define completion from the request and the owning path's contracts. Include callers, tests, fixtures, configuration, exports, and evidence required by that change.
- Read this file, `.specify/memory/constitution.md`, and the owning crate's documentation before editing. Load additional standards only when their subject affects the task.
- Start with the entry point and direct dependencies. Expand a search only to answer a concrete unresolved question.
- Reuse existing code and conventions. Implement the first approach that satisfies the contract, then verify it.
- For a bug, establish the failure with a focused test, reproduction, or log before fixing the shared cause.
- Proceed with reasonable assumptions when they do not change the outcome. Ask only when missing information materially changes implementation or verification.
- Preserve unrelated work. Do not commit unless the user asks. Stage only this task's files and inspect `git diff --cached --name-only` before committing.
- Finish after the requested behavior, necessary related changes, checks, evidence, and final diff review are complete. Do not start an optional improvement pass.

## Repository boundaries

| Path | Responsibility | Required care |
| --- | --- | --- |
| `crates/box` | Execution resources, MicroVM lifecycle, images, volumes, networks, snapshots and credentials | Preserve ExecutionManager ownership and generation fencing |
| `crates/oci-runtime` | OCI execution and guest/host integration | Preserve backend routing and platform-specific behavior |
| `crates/runtime` | Provider-neutral Task／Service contracts and lifecycle clients | Do not create a second execution state machine |
| `kense-labs/kcl` (external, pinned git rev) | KCL parser, generator and canonical digest | Keep product configuration in KCL; kbox-side closed schemas live in `crates/box/core/src/compose/` and `crates/box/runtime/src/oci/build/plan.rs` |
| `libkrun` | Native virtualization source submodule | Edit in `kense-labs/libkrun`, test, then update the gitlink |
| `libkrunfw` | Guest kernel bundle submodule | Edit in `kense-labs/libkrunfw`, preserve version and provenance, then update the gitlink |
| `specs` | Bounded-epic planning | Planning is not acceptance evidence |
| `docs/evidence` | Commands, counts, platform results and limitations | Never delete evidence to support a claim |

### Resource authority

- Box ExecutionManager is the authority for VM processes, mounts, networks and actual execution resources.
- ManagedRuntimeClient owns Task／Service operation receipts and the last provider observation. It does not duplicate Box state.
- ACP Session Manager owns ACP session mappings. RunBinding binds sessions to an execution generation.
- Service Registry owns Plugin endpoints and health bindings.
- ProviderBinding owns local/cloud routing constraints.
- Data Lineage Controller owns DataRef relationships and deletion status.
- The actual PEP owns intervention effects. A decision or audit record does not prove enforcement.
- Provider failure never causes a silent backend, isolation, data-residency or permission downgrade.

## General rules

- Do not create another Cargo workspace or root lockfile. Keep internal `k*` dependencies on local paths and `publish = false`. First-party crates share exactly one root `[workspace]`, one `Cargo.lock` and one `rust-toolchain.toml`; `scripts/check-workspace-layout.py` enforces this in CI (vendored trees, third_party and submodules are excluded).
- Do not edit submodule content in the parent repository. Land and test the change in the owning repository, then update the parent's gitlink.
- Do not create a repository for a new library without explicit user authorization.
- Validate external input once at the entry boundary and pass typed values inward.
- Reuse existing platform adapters for process cancellation, PTY behavior, locking, permissions, symlinks and atomic replacement.
- Do not hand-edit generated files. Change the source and regenerate them.
- kbox and krun are separate repositories with no shared crate, so versioned wire schema identifiers are duplicated by hand. `scripts/wire-contracts.json` is the declared contract: update it in the same change that adds, renames or removes any `.v1` wire identifier. `scripts/check-wire-contracts.py` verifies the kbox side in CI; run `python3 scripts/check-wire-contracts.py --krun ../krun` for any wire change. Undeclared identifiers fail the gate rather than being silently accepted.
- KCL comes from `kense-labs/kcl` at a pinned `git rev` in the root `Cargo.toml`, exactly as krun does; do not restore a sibling `../kcl` path dependency or pin a `branch`/`tag` instead of a `rev`. A KCL rev or semantics change is a cross-repository change: update both manifests and lockfiles together, and verify with `python3 scripts/check-kcl-source.py --krun ../krun` plus the KCL-consuming tests (`cargo test --locked -p kbox-core --lib compose` and `cargo test --locked -p kbox-runtime --lib oci::build::plan`). `scripts/check-kcl-source.py` enforces the kbox side in CI. Historical release notes, test plans and evidence logs that mention `../kcl` stay unchanged.
- Keep tests hermetic by default and mocks minimal. Test the real implementation rather than copying business logic into tests.
- Record unrelated findings without fixing them unless they block the requested task.
- Never hardcode evaluation answers, weaken a test, or change a grader to obtain a pass.

## Rust implementation and review

- Use enums and invariant-bearing newtypes for admitted values. Validate in fallible constructors and keep invariant-bearing fields private.
- Return typed recoverable errors at library boundaries and preserve sources. Match error types or stable codes, not rendered text.
- Input, I/O and cancellation failures are not panics. Production `unwrap()` or `expect()` requires a nearby construction-level infallibility argument.
- Prefer item-scoped `#[expect(lint, reason = "...")]` for an expected lint. Do not broaden workspace lint allows.
- Every member inherits the root lints with `[lints] workspace = true`, except the documented local exceptions in `scripts/check-workspace-layout.py`. Do not lower an inherited `deny` to make the gate pass; fix the code or keep the narrower local contract.
- Public types implement useful `Debug`. Credentials, header values and sensitive payloads stay out of `Debug`, `Display` and logs. Test redaction with synthetic secrets.
- Use structured `tracing` fields. Reserve stdout for intended CLI or protocol output.
- Prefer safe Rust and existing wrappers. Every handwritten `unsafe` block or impl needs a nearby `SAFETY:` argument.
- Every `unsafe fn` has its own `# Safety` rustdoc section stating the caller's obligations, even when private or target-gated. Do not add a blanket `allow(clippy::missing_safety_doc)`: that lint only inspects exported items, so binary crates and `cfg`-only functions escape it. `scripts/check-unsafe-safety-docs.py` is the visibility-independent gate in CI.
- Inside an `unsafe fn`, wrap each FFI call in its own `unsafe { ... }` block with a nearby `SAFETY:` comment; the workspace keeps `unsafe_op_in_unsafe_fn` at deny. A block comment does not satisfy `clippy::undocumented_unsafe_blocks` — use a `// SAFETY:` line above or inside the block.
- Every `unsafe` block has its own concrete `// SAFETY:` argument, not a shared boilerplate line; the root clippy baseline keeps `undocumented_unsafe_blocks` at deny and CI runs clippy on both the Linux host and the `x86_64-pc-windows-msvc` target, because clippy cannot see `cfg`-excluded Windows blocks from a Linux run. Verify with `K_DEPS_STUB=1 cargo clippy --locked -p kbox-shim --target x86_64-pc-windows-msvc --all-targets --no-deps` when touching target-gated unsafe code.
- Bound input bytes and work at entry. Keep blocking I/O and long CPU work off async executor threads and do not hold blocking locks across `.await`.
- Keep public re-exports explicit. Unit tests live in sibling files and public API integration tests live in `tests/`.
- Measure hot paths before changing allocators, hashers or ownership layouts.

## Configuration, identity and data security

- KCL is the only product configuration language, parsed by `kcl`. Manifests, evidence and protocol schemas are data contracts, not a second configuration system.
- Use closed enums with `parse()` and `as_str()` for admitted values. Unknown security-critical values are refused.
- Bind workload credentials to the principal, Run, Capability, DataRef and generation. Delegated permissions may only narrow.
- Keep real secrets out of images, prompts, memory, checkpoints, events, logs and evidence. Store only typed references.
- Treat external content as untrusted data. Do not promote it to instructions or long-term memory without an explicit authorized transition.
- ProviderBinding must enforce data classification, residency, retention and training-use constraints before routing.
- Preserve DataRef lineage through summaries, embeddings, memory, checkpoints, artifacts and deletion receipts.
- Record ObservedEvent, Decision, Intervention and PEP effect separately. `Decision.block` is not proof of rejection.

## Cross-platform compatibility

- Keep changes compatible with supported Linux KVM, macOS HVF and Windows WHPX targets. Isolate OS-specific code and dependencies with `cfg`.
- Use `Path`, `PathBuf`, `OsStr` and `OsString` for filesystem and process values. Do not concatenate paths or force them to UTF-8.
- Pass executables, arguments, working directories and environment through process APIs. Invoke a shell only when its semantics are required.
- Use UTF-8 without BOM for source text unless a contract says otherwise. Follow `.gitattributes` for LF and Windows command-file line endings.
- Preserve target CPU, ABI, minimum-OS, libc and linker baselines. Do not add machine-local `target-cpu=native` settings.
- Use portable temporary directories in tests. Cover spaces and non-ASCII paths where path handling changes.
- Cross-compilation proves compilation only. Native runtime claims require native tests on the affected target.

## Verification

For each handwritten Rust behavior change, run:

Before any local test invocation, set `K_HOME` to a fresh task-owned isolated
directory. This includes targeted `cargo test` commands. `scripts/verify-commit.sh`
allocates and reports its own private home, retained for audit on either result.
An unset `K_HOME` uses the real user installation and is not an isolation boundary.

```bash
cargo fmt --all -- --check
cargo test --locked -p <owning-package> <focused-filter>
cargo clippy --locked -p <owning-package> --all-targets --no-deps
```

Run `cargo check --workspace --all-targets --locked` for workspace integration changes. Run affected native backend tests for platform claims.

- Unit tests cover contracts, state machines, validation and errors.
- Hermetic integration tests cover callers, drivers, ACP, Plugin and CLI behavior.
- Malformed-input tests cover truncation, excessive sizes, path escape and invalid combinations.
- Crash tests cover SIGKILL, restart, partial operations, stale generations and reconciliation.
- Native tests cover only the actual backend, image and host configuration being claimed.
- Documentation-only changes require reference checks and `git diff --check`. Unsafe-doc changes also run `python3 scripts/check-unsafe-safety-docs.py` (with a negative test when the gate itself is new).

A behavior change records `docs/evidence/<domain>/<date-slug>/OUTCOME.md` with actual commands, counts, environment, logs and an **Explicitly not claimed** section. Never claim a check without its actual result.

## Release gates

- Check final archives for required runtime, guest-init, shim, ACP, native library, kernel and rootfs companions.
- Install the final archive into an isolated prefix and run real Box, ACP and Plugin smoke scenarios.
- Verify cancel leaves no late writes, restore rejects old generations, and cleanup leaves no managed resources.
- Keep platform support matrices limited to native tested configurations.
- Record compressed package size, installed disk usage and runtime memory separately. The 40–50 MB goal applies only to the minimal Runtime bootstrap package.

## Spec-driven development

Use `docs/spec-driven-development.md` and `.specify/templates/` for bounded epics. Routine fixes do not need a spec directory. A NON-NEGOTIABLE principle in `.specify/memory/constitution.md` cannot be waived by a spec, plan, task or deadline.

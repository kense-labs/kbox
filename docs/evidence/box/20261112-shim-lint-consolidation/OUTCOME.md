# Eliminate the kbox-shim workspace-lint exception

Date: 2026-11-12
Repository: `kbox`, HEAD `405c8b8` (working tree uncommitted)
Result: PASS for the static-contract, compile, lint, formatting and
whitespace scopes below. No native runtime claim is made.

## Problem

The workspace consolidation left `crates/box/shim` as a local lint exception:
it had **no `[lints]` table at all**, so none of the root workspace rust or
clippy rules applied to it. The stated blocker was the inherited
`unsafe_op_in_unsafe_fn = "deny"`, which turned bare FFI calls inside its
`unsafe fn`s into E0133 errors.

Re-measurement corrected the recorded number. Forcing the deny via clippy on
both targets produced **50 E0133 primary spans on the host** and **37 on the
Windows cross target**, a union of **56 unique byte spans** — not the 49 from
the earlier single-target snapshot. Distribution: 31 in
`src/kbox/context.rs`, 23 in `src/vm_launch.rs`, 2 in `src/main.rs`; 7 spans
are multi-line calls. The Windows-only ones are invisible to a host build, so
a host-only fix would have left the exception unable to pass CI.

`configure_and_start_vm` spans roughly lines 86–800 of `vm_launch.rs`.
Wrapping each entire `unsafe fn` body in one big `unsafe { ... }` would have
re-indented hundreds of lines, so the per-call-site approach was chosen.

## Pre-flight experiments

A throwaway crate in `/tmp/unsafe_probe` was used to settle two questions
before touching 56 sites:

- With `-D unsafe_op_in_unsafe_fn`, an explicit `unsafe { ... }` inside an
  `unsafe fn` does **not** trigger `unused_unsafe` (clippy exit 0).
- `clippy::undocumented_unsafe_blocks` accepts a `// SAFETY:` line above the
  block or on the first line inside it. A `/* SAFETY: ... */` block comment
  placed before the `unsafe` keyword is **not** accepted.

## Changes

- **56 call sites** wrapped in `unsafe { ... }`, each with a nearby
  `// SAFETY:` line. The changes were driven by rustc/clippy **JSON
  diagnostics** (`--message-format=json`, primary `byte_start`/`byte_end`)
  from both targets and applied from the highest offset downwards, so
  cfg-only call sites and multi-line calls were covered without guessing.
  A pre-flight assertion checked every span was a complete, balanced call
  expression. No line contained two spans.
- The `SAFETY:` lines deliberately point back to the owning function's
  `# Safety` rustdoc section (written in the previous pass) instead of
  restating the same argument 56 times. `configure_and_start_vm`'s previously
  thin `# Safety` section was expanded into an explicit caller-obligation
  contract (shim subprocess only, borrowed resources valid for the VM
  lifetime, serialized context access, one-time logging init) so its 23
  references have real content.
- The two libc `getrlimit`/`setrlimit` sites got their own concrete SAFETY
  arguments (live stack-local, valid resource, soft limit within hard limit)
  rather than the generic contract reference.
- `crates/box/shim/Cargo.toml` now has `[lints] workspace = true`. The root
  deny therefore applies automatically; no command-line `-D` is needed.
- `scripts/check-workspace-layout.py`: `crates/box/shim` removed from
  `LOCAL_LINT_EXCEPTIONS`, and a new **stale-exception** check fails when an
  exception member already inherits workspace lints. This prevents a future
  fix from being hidden by a leftover entry.
- Root `Cargo.toml` comment updated (the "49 FFI calls" text is gone).
  `AGENTS.md` gained the rule that each FFI call inside an `unsafe fn` needs
  its own `unsafe { ... }` with a `// SAFETY:` line, plus the note that block
  comments do not satisfy `undocumented_unsafe_blocks`.

The only remaining exception is `crates/box/third_party/mkext4`, which keeps
its own *stricter* `unsafe_code = "forbid"`. That is an intentional narrower
contract, not a relaxed gate.

## Verification

See `checks/verification.txt`. All 11 commands exited 0, including full
workspace `check`/`clippy`, host and Windows cross clippy **without any
manual `-D`**, per-file `rustfmt --edition 2021 --check`, and
`git diff --check`.

- Forced `-D` clippy after the change: 0 E0133 on both targets.
- Layout gate: 19 members, **18 inherit root lints (was 17)**, **1 exception
  (was 2)**, PASS.
- Unsafe-doc gate: 35 `unsafe fn`, 0 missing `# Safety`, 0 blanket allows.
- `kbox-cri` was excluded locally because `protoc` is absent; the CI `rust`
  job installs `protobuf-compiler` and checks the full workspace.
- All three touched Rust files were rustfmt-clean before this change, so
  running `rustfmt --edition 2021` on them introduced no historical
  formatting churn.

## Negative tests

See `checks/negative-tests.txt`. Sources were backed up and restored with a
shell trap; the final state is PASS.

1. Unwrapped one call (`let ctx = kcreate_ctx();`) and ran **plain** clippy:
   `error[E0133]` and exit 101 — the inherited deny is genuinely active.
2. Re-added `crates/box/shim` to `LOCAL_LINT_EXCEPTIONS`: layout gate exit 1
   with `FAIL: stale local lint exception (already inherits workspace lints)`.
3. Restored: layout PASS (18/1) and clippy exit 0.

## Explicitly not claimed

- **No runtime behavior change.** Every edit either adds an `unsafe { ... }`
  boundary around an existing expression, adds a comment, or changes lint
  configuration. The wrapped expression text is byte-for-byte the compiler's
  own span; rustfmt only re-indented the resulting blocks.
- **No soundness improvement claimed.** These blocks make pre-existing
  unsafe operations explicit and documented; they do not add new checks or
  fix any memory-safety defect.
- **7 pre-existing undocumented unsafe blocks remain** on the host target
  (`context.rs` `Drop` impl and `main.rs` libc `geteuid`/`flock`/`fcntl`
  calls). They predate this change and are outside its scope;
  `undocumented_unsafe_blocks` is not a root lint, so they do not affect the
  gate. All 56 newly added blocks are recognized as documented.
- **No Windows native runtime test.** Cross-compilation proves compilation
  only. Windows-only call sites were fixed from the cross-target JSON
  diagnostics and verified by cross-target check/clippy, not by execution.
- **No fix for pre-existing warnings** elsewhere in the workspace, no
  physical crate relocation (`M-CRATES-FLAT-FOLDER` flat-folder shape is
  still not fully achieved), and no change to KCL dual-source, cross-repo
  wire-schema sync, or krun's host-ARCH assumption.
- **Historical evidence was not rewritten.** The earlier
  `20261112-workspace-consolidation` evidence still records its original
  "49" count; this document records the corrected dual-target measurement
  separately.
- **No commit, push, or CI run on GitHub.** The workflow file was edited but
  not executed remotely.

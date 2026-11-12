# Unsafe function safety documentation + a real gate

Date: 2026-11-12
Repository: `kbox`, HEAD `405c8b8` (working tree uncommitted)
Result: PASS for the static-contract, compile, lint, formatting and
whitespace scopes below. No native runtime claim is made.

## Problem

`crates/box/shim/src/kbox/context.rs` carried a file-level
`#![allow(clippy::missing_safety_doc)]` and no per-function `# Safety` sections
for most of its 30 `unsafe fn`s. The previous optimization pass removed that
blanket allow and inserted sections, but the insertion placed them in the
middle of several multi-line rustdoc descriptions, splitting sentences in half
(e.g. "Attach a typed raw auxiliary disk without probing its" / "contents.").

A second, more important issue was discovered while validating the gate plan:
`-D clippy::missing_safety_doc` is a **vacuous gate** for this crate.
`clippy::missing_safety_doc` only inspects *exported* items; `kbox-shim` is a
binary crate with a private `mod kbox;` in `src/main.rs`, so `KContext` is not
exported. In a controlled negative test, deleting a `# Safety` heading caused
cargo to recompile `kbox-shim` yet clippy still exited 0. Windows-only
`unsafe fn`s are also invisible to a macOS/Linux host build. The original
blanket allow was therefore suppressing a lint that did not fire here anyway.

## Changes

Source documentation (no behavior/code changes):

- `crates/box/shim/src/kbox/context.rs`
  - Module-level `# Safety` contract states the four shared invariants: a live
    context id from `KContext::create`, serialized calls per context, borrowed
    resources staying valid for the VM's lifetime, and the process-global
    effect of `init_logging`.
  - Each of the 30 `unsafe fn`s has its own `# Safety` section, moved to the
    **end** of its rustdoc block so descriptions and `# Arguments` / `# Returns`
    sections stay intact. Special requirements are named per method (rootfs
    paths, open fds, sockets, named pipes, TEE config, kernel format).
  - Windows stubs get an explicit note that they perform no FFI call but keep
    the shared `unsafe` signature.
  - Removed `#![allow(clippy::missing_safety_doc)]` and the
    `#[allow(unsafe_op_in_unsafe_fn)]` on `create()`. The four remaining
    `#[allow(dead_code)]` are unrelated.
- `crates/box/shim/src/main.rs` — added `# Safety` to `apply_user_config`.
- `crates/oci-runtime/crates/agent/src/executor/hook/watchdog.rs` — added
  `# Safety` to `watchdog_intermediate` and `watchdog_main` (fork/exec
  constraints, raw fd ownership, never returning into Rust).
- `crates/box/shim/src/vm_launch.rs` and
  `crates/box/deps/libksys/src/lib.rs` already had `# Safety`; unchanged.

Gate and rules:

- `scripts/check-unsafe-safety-docs.py` (new, pure stdlib, read-only): scans
  all 1452 first-party `.rs` files, checks all 35 `unsafe fn` declarations for
  an attached `# Safety` section regardless of visibility or `cfg`, and forbids
  any blanket `allow(clippy::missing_safety_doc)`. It is Python 3.9-compatible.
- `.github/workflows/verify.yml` — `layout` job renamed to "Static repository
  contracts" and gained a `scripts/check-unsafe-safety-docs.py` step. Existing
  clippy steps are unchanged; a comment explains why clippy alone is not
  sufficient.
- `AGENTS.md` — added the per-function `# Safety` rule and the note that
  clippy's lint only covers exported items; verification section now requires
  the static gate for unsafe-doc changes.

## Verification

See `checks/verification.txt`. All 8 commands exited 0:

| Check | Exit |
| --- | --- |
| `python3 scripts/check-unsafe-safety-docs.py` | 0 |
| `python3 scripts/check-workspace-layout.py` | 0 |
| `K_DEPS_STUB=1 cargo check --locked --workspace --exclude kbox-cri --all-targets` | 0 |
| `K_DEPS_STUB=1 cargo clippy --locked --workspace --exclude kbox-cri --all-targets --no-deps` | 0 |
| `K_DEPS_STUB=1 cargo clippy --locked -p kbox-shim --all-targets --no-deps -- -D clippy::missing_safety_doc` | 0 |
| `K_DEPS_STUB=1 cargo check --locked -p kbox-shim --target x86_64-pc-windows-gnu --all-targets` | 0 |
| `K_DEPS_STUB=1 cargo clippy --locked -p kbox-shim --target x86_64-pc-windows-gnu --all-targets --no-deps -- -D clippy::missing_safety_doc` | 0 |
| `git diff --check` | 0 |

Static gate counts: 1452 first-party `.rs` files, 35 `unsafe fn`, 0 missing
`# Safety`, 0 blanket allows. Layout gate: 1 workspace, 19 members, 17
inheriting root lints, 2 documented exceptions (unchanged).

Formatting was checked per crate edition (all four touched Rust files are
edition 2021): `rustfmt --edition 2021 --check` on `context.rs`, `main.rs`,
`watchdog.rs`, `driver.rs` all exited 0. Using `--edition 2024` produces
spurious import-order diffs; the repository's historical rustfmt differences
were not reformatted.

`kbox-cri` was excluded from local check/clippy because `protoc` is not
installed on this host; the CI `rust` job installs `protobuf-compiler` and runs
the full workspace. `check-workspace-layout.py` needs `tomllib`, so locally it
must be run as `python3` (3.13.12), not `/usr/bin/python3` (3.9.6); CI Ubuntu
`python3` satisfies this.

## Negative tests

See `checks/negative-tests.txt`. Sources were backed up and restored via a
shell trap; the final gate run after restoration is PASS.

1. Removed one `# Safety` heading from `set_vm_config` and ran clippy with
   `-D clippy::missing_safety_doc`: crate recompiled, **clippy exit 0** —
   confirms the lint cannot serve as the gate for this binary crate.
2. Same missing heading plus a reintroduced
   `#![allow(clippy::missing_safety_doc)]` in `watchdog.rs`: static gate
   **exit 1**, naming both exact locations
   (`context.rs:134 unsafe fn set_vm_config`, `watchdog.rs:1` blanket allow).
3. Restored sources: static gate **exit 0**, 0 missing / 0 allows.

## Explicitly not claimed

- **No runtime behavior change.** All edits are rustdoc, comments, a new
  Python checker, one CI step and two AGENTS.md lines. A scripted comparison
  of `context.rs` before/after the documentation reorder confirmed the
  code-and-attribute line sequence is byte-identical.
- **No claim that clippy is a safety-doc gate.** It was measured to be vacuous
  for this crate; the remaining `-D clippy::missing_safety_doc` invocations are
  recorded as passing but are not the protection.
- **No Windows native runtime test.** Cross-compilation proves compilation
  only; the Windows cfg'd functions were verified by source scan and
  cross-target check/clippy, not by execution.
- **No fix for the 49 pre-existing E0133 `unsafe_op_in_unsafe_fn` sites** in
  `kbox-shim`, and no change to the two documented workspace-lint exceptions.
  That remains the separate technical-debt item recorded in the workspace
  consolidation evidence.
- **No fix for pre-existing warnings** (unused imports, dead code, deprecated
  `AsyncFd::new`, clippy style warnings). They are present at HEAD and were not
  introduced here.
- **No commit, push, or rewrite of historical evidence.**

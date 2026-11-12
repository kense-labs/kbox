# Outcome: document every `unsafe` block and make the deny lint real

Date: 2026-11-12 18:50 +0800
Repository: `/Users/roylin/code/kense/kbox` at HEAD `405c8b8`. No commit, no push.

## Problem

Earlier work documented all 35 `unsafe fn`s and wrapped 56 FFI call points in
explicit `unsafe { ... }` blocks. Seven `// SAFETY:` comments were still missing
in the shim, and the root `[workspace.lints.clippy]` table did **not** contain
`undocumented_unsafe_blocks`, so nothing prevented the backlog from growing.

## Measurement

Forcing the lint with clippy JSON diagnostics on `-p kbox-shim`:

```
host (aarch64-apple-darwin)          : 7 unique blocks
windows (x86_64-pc-windows-gnu)      : 3 unique blocks
union                                : 8 unique blocks  (1 Windows-only)
```

Extending the measurement to the whole workspace revealed a much larger backlog.
Clippy aborts a crate on its first error, so early passes only saw the crates
that happened to compile first. Each fix pass exposed another crate:

```
pass 1 (initial -D, aborted early)    :  2 blocks  (kbox-contract, klibksys example)
pass 2 (root deny added)              : 33 blocks  in 13 files
pass 3                               : 10 blocks  in 10 files
pass 4                               :  3 blocks  in  2 files
pass 5                               : 19 blocks  in  6 files
pass 6 (--keep-going, complete)       : 14 blocks  in  3 files
Windows-only pass (--keep-going)      : 40 blocks  in 11 files
```

Total addressed in this task: **129 `unsafe` blocks** (8 shim + 2 adjacent +
33 + 10 + 3 + 19 + 14 + 40). `cargo clippy --keep-going` was the key to a
complete inventory: without it the build stopped at the first failing crate.

A finding worth keeping: several files already had a `SAFETY:` comment, but two
or more lines above the `unsafe` token (for example above an `assert_eq!`
invocation, or above the first of several consecutive `unsafe` statements).
Clippy requires the comment adjacent to each `unsafe` block; those blocks were
genuinely undocumented to the compiler even though a human reader saw a
nearby explanation.

`kbox-cri` cannot be compiled locally (`prost-build` needs `protoc`, absent on
this host). A static scan found **0 `unsafe {` blocks** in `crates/box/cri/src`,
so the CI `rust` job, which installs `protobuf-compiler`, should not expose new
ones. This is a static argument, not a compiled result.

## Changes

* Added a concrete `// SAFETY:` argument to each of the 129 blocks. Comments are
  specific to the call (pointer lifetime, fd ownership, scalar-only libc calls,
  `MaybeUninit` initialization, async-signal-safety of `pre_exec` / `fork`,
  Win32 handle ownership), not one shared boilerplate line.
* `Cargo.toml`: added `undocumented_unsafe_blocks = "deny"` to
  `[workspace.lints.clippy]`; 18 of 19 members inherit it (`mkext4` keeps its
  own stricter `unsafe_code = "forbid"` and has no unsafe blocks).
* `.github/workflows/verify.yml`: added `Clippy kbox-shim on Windows target`
  to the `windows-cross` job. `cargo check` alone cannot see clippy lints, and a
  Linux clippy run cannot see `cfg`-excluded Windows code.
* `AGENTS.md`: added the rule that every unsafe block needs its own concrete
  `// SAFETY:` argument, that the root baseline denies the lint, and that
  target-gated unsafe code needs the Windows-target clippy command.
* Two pre-existing `SAFETY:` comments in
  `crates/oci-runtime/crates/runtime/src/state/filesystem/platform.rs` were
  split so `assume_init` and `transmute_copy` each have their own argument.

No executable statement was changed by this task. A diff audit shows the only
non-comment `.rs` lines relative to HEAD come from earlier tasks in this
session (rename cleanup, `driver.rs` mock, `OwnedHandle`, E0133 call-site
wrappers); this task added and replaced comment lines only.

## Verification

```
# host, whole workspace, lint forced to deny
K_DEPS_STUB=1 cargo clippy --keep-going --locked --workspace --all-targets \
  --no-deps --exclude kbox-cri --message-format=json \
  -- -D clippy::undocumented_unsafe_blocks
  -> rc=0, 0 error-level messages, 0 undocumented blocks

# host, no -D: the inherited root deny alone is sufficient
K_DEPS_STUB=1 cargo clippy --keep-going --locked --workspace --all-targets \
  --no-deps --exclude kbox-cri                        -> rc=0

# Windows target, whole workspace
K_DEPS_STUB=1 cargo clippy --keep-going --locked --workspace --all-targets \
  --no-deps --exclude kbox-cri --target x86_64-pc-windows-gnu
  -> 0 undocumented blocks; remaining errors are pre-existing Windows
     compilation failures: E0599 x53, E0277 x14, E0433 x2, E0432 x1

# CI windows-cross step
K_DEPS_STUB=1 cargo clippy --locked -p kbox-shim \
  --target x86_64-pc-windows-gnu --all-targets --no-deps   -> rc=0

K_DEPS_STUB=1 cargo check --locked --workspace --all-targets --exclude kbox-cri -> rc=0
K_DEPS_STUB=1 cargo test --locked -p kbox-core --lib compose        -> 35 passed
K_DEPS_STUB=1 cargo test --locked -p kbox-runtime --lib oci::build::plan -> 8 passed
K_DEPS_STUB=1 cargo test --locked -p kbox-contract --lib             -> 24 passed
git diff --check                                                     -> rc=0
python3 scripts/check-workspace-layout.py                            -> PASS
python3 scripts/check-unsafe-safety-docs.py                         -> PASS
python3 scripts/check-wire-contracts.py                             -> PASS
python3 scripts/check-kcl-source.py                                  -> PASS
python3 scripts/check-kcl-source.py --krun ../krun                   -> PASS
ruby -ryaml YAML.load_file(".github/workflows/verify.yml")          -> 3 jobs,
  layout 5 steps / windows-cross 4 steps
```

Formatting: 53 changed `.rs` files checked with `rustfmt --edition 2021
--check`; 51 clean, 2 have diffs that already exist at HEAD
(`linux_kvm_oci_qualification.rs`, `local_execution/vm_backend.rs`),
**0 new regressions**. Both pre-existing diffs are unrelated import-order and
method-chain formatting; none of them touches an added `SAFETY:` line.

Negative test (`/tmp/negative_safety_gate.py`): removed one `// SAFETY:` line
from `crates/runtime/src/state/file/fs.rs`, ran plain
`cargo clippy --locked -p kbox-contract --lib --no-deps` (no `-D`), got rc=101
with `error: unsafe block missing a safety comment`, then restored the file
byte-exactly (SHA-256 verified) and re-ran clippy -> rc=0. The inherited deny is
a real gate, not a vacuous one.

## Explicitly not claimed

* No commit, no push, no remote GitHub Actions run. The new `windows-cross`
  clippy step has not been executed by a real runner.
* `kbox-cri` was not compiled locally (no `protoc`); its zero-unsafe-block
  status comes from a static scan only. The CI `rust` job does compile it.
* The Windows cross build of the **whole** workspace still fails with 70
  pre-existing `E0599` / `E0277` / `E0433` / `E0432` errors. Those are unrelated
  to this change and were present before the lint was introduced; CI therefore
  only runs the Windows clippy step on `kbox-shim`.
* `K_DEPS_STUB=1 cargo test -p kbox-shim` cannot link (`ld: library 'krun' not
  found`): stub mode intentionally skips building libkrun and is valid for
  check/clippy only. Shim behavior was not executed here.
* No claim about native Windows runtime behavior, MicroVM, ACP or any platform
  matrix. Cross-compilation proves compilation only.
* No behavior change is claimed or intended: this task added and rewrote
  comments plus one lint level and one CI step.
* The 33 parallel uncommitted krun directory changes were not touched; only
  `kbox` files were modified in this task.

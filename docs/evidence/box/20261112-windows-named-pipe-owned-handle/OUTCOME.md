# Windows named-pipe handle ownership and unsafe documentation

Date: 2026-11-12. Host: macOS/arm64. Toolchain: repository `rust-toolchain.toml`
channel 1.97.1 (clippy/rustfmt components). Cross target:
`x86_64-pc-windows-gnu`, installed during this session because the pinned
toolchain previously had only `aarch64-apple-darwin` and
`aarch64-unknown-linux-musl`. FFI crate: `windows-sys 0.48.0`, where
`Win32::Foundation::HANDLE` is `isize` and `RawHandle` is `*mut c_void`.

Result: PASS for the compile-time, formatting and lint scopes below. No Windows
runtime claim is made.

## Scope and motivation

`crates/box/shim/src/windows_port_forward.rs` is
`#![cfg(target_os = "windows")]`, so a normal macOS `cargo test` never compiles
it. The Microsoft Pragmatic Rust Guidelines review of this workspace flagged
`NamedPipeServer` as the highest-risk item: it stored a raw `HANDLE`, closed it
in a handwritten `Drop`, declared `unsafe impl Send` / `unsafe impl Sync` with
no soundness argument, and had no `SAFETY:` comments anywhere in the file.
`kbox/AGENTS.md` requires a nearby `SAFETY:` argument for every handwritten
unsafe block or impl.

Changes made only in that file:

- `NamedPipeServer.handle` is now `std::os::windows::io::OwnedHandle`. The
  standard library supplies `Send`/`Sync` and the single `CloseHandle`; both
  handwritten `unsafe impl`s were deleted.
- `raw()` borrows the scalar handle value for FFI only. `create()` now rejects
  both `0` and `INVALID_HANDLE_VALUE` before `from_raw_handle` (the `OwnedHandle`
  contract), then transfers unique ownership.
- `Drop` now calls only `DisconnectNamedPipe`. Rust drops fields after
  `Drop::drop` returns, so the client still observes a broken pipe before the
  `OwnedHandle` field closes the handle; the original ordering is preserved.
- `spawn_parent_watchdog` also wraps its `OpenProcess` handle in `OwnedHandle`,
  removing the last manual `CloseHandle` call in the file.
- All 14 `unsafe` blocks have a nearby `SAFETY:` argument.
- A regression test pins `NamedPipeServer: Send + Sync` and
  `Arc<NamedPipeServer>: Send + Sync` at compile time without a handwritten impl.

## Commands and actual results

| # | Command | Result |
| --- | --- | --- |
| 1 | `rustup target add x86_64-pc-windows-gnu --toolchain 1.97.1` | PASS (`downloading component rust-std`) |
| 2 | `rustc --target x86_64-pc-windows-gnu --edition 2021 --emit=metadata -o /tmp/probe_owned_handle.rmeta /tmp/probe_owned_handle.rs` (one-off probe, `RUSTUP_TOOLCHAIN=1.97.1`) | PASS: `OwnedHandle` is `Send + Sync`, `HANDLE`/`RawHandle` casts and `from_raw_handle` type-check. First probe attempt failed with `E0790` (trait associated function needs an impl type); fixed with a fully qualified path. |
| 3 | `K_DEPS_STUB=1 RUSTUP_TOOLCHAIN=1.97.1 cargo check --manifest-path <workspace>/Cargo.toml --locked -p kbox-shim --target x86_64-pc-windows-gnu --all-targets` | Before: rc=0, `Finished dev profile in 10.26s`. After: rc=0, `Finished ... in 0.82s`. |
| 4 | `K_DEPS_STUB=1 RUSTUP_TOOLCHAIN=1.97.1 cargo clippy --manifest-path <workspace>/Cargo.toml --locked -p kbox-shim --target x86_64-pc-windows-gnu --all-targets --no-deps -- -W clippy::undocumented_unsafe_blocks` | Before: rc=0, 18 unique locations. After: rc=0, 3 unique locations. See `checks/lint-before.txt` and `checks/lint-after.txt`. |
| 5 | `K_DEPS_STUB=1 RUSTUP_TOOLCHAIN=1.97.1 cargo check --manifest-path <workspace>/Cargo.toml --locked -p kbox-shim --all-targets` (host target) | After: rc=0, `Finished ... in 11.58s`. Confirms the cfg-gated change does not disturb the macOS build. |
| 6 | `RUSTUP_TOOLCHAIN=1.97.1 rustfmt --check --edition 2021 crates/box/shim/src/windows_port_forward.rs` | rc=0 |
| 7 | `git diff --check` (repository) | rc=0 |

The `--all-targets` check type-checks the unit test target as well. The existing
`thread::spawn` call sites that move or share `NamedPipeServer` /
`Arc<NamedPipeServer>` also type-check, which is the direct evidence that
`OwnedHandle` supplies the required `Send`/`Sync` bounds.

## Counts

| Metric | Before | After |
| --- | --- | --- |
| Handwritten `unsafe impl Send`/`Sync` in the file | 2 | 0 |
| Manual `CloseHandle` call sites in the file | 2 | 0 |
| `unsafe {` blocks in the file | 13 | 14 |
| `SAFETY:` comments in the file | 0 | 14 |
| `undocumented_unsafe_blocks` unique locations, whole crate | 18 | 3 |
| ... of those in `windows_port_forward.rs` | 15 | 0 |

The unsafe block count rises by one because two `from_raw_handle` calls were
added while the separate `CloseHandle` block in the watchdog was removed.

## Retained failures and detours

- The first cross `cargo check` failed in `klibksys` build.rs (exit 101) with
  `Failed to resolve Rust's macOS deployment target`, after printing
  `K_BUILD_LIBKRUN set: forcing build from source`. No `K_*` variable was set in
  the environment. The warning text is misleading: build.rs line 144 is the
  `else` branch of `env::var("K_BUILD_LIBKRUN").is_err() && !force_vendored`,
  and on a macOS host `force_vendored` is true because `K_USE_SYSTEM_LIBKRUN`
  is absent. The repository's documented `K_DEPS_STUB=1` mode (build.rs line 99)
  skips the native build and emits stub link directives; `cargo check` does not
  link, so all checks above used it.
- An earlier attempt piped cargo output through `tail` without `pipefail`, so a
  failed cargo run appeared as exit code 0. All recorded results above capture
  the cargo exit code directly.
- Three `undocumented_unsafe_blocks` locations remain outside the changed file:
  `kbox/context.rs:664`, `main.rs:262`, `vm_launch.rs:864`. They are recorded,
  not fixed.
- `.github/workflows` contains no Windows job, so this cfg-gated module had no
  CI compilation coverage. Adding one is a separate change.

## Explicitly not claimed

- No Windows native runtime test was run. The module's existing tests
  (including `named_pipe_disconnect_flushes_the_complete_response`) were
  type-checked, not executed. Per `AGENTS.md`, cross-compilation proves
  compilation only; native runtime claims require native tests on Windows.
- The new regression test was type-checked by `cargo check --all-targets`, not
  executed on a Windows host.
- This change does **not** eliminate the pre-existing concurrency race between
  `HostWriter::shutdown()` -> `disconnect()` and a concurrent `read()` /
  `write_all()` on the same `Arc<NamedPipeServer>`. `OwnedHandle` is `Sync`, so
  `DisconnectNamedPipe` can still run alongside `PeekNamedPipe`, `ReadFile` or
  `WriteFile`. The handle is not closed early (the `Arc` keeps it alive), but
  I/O may fail or land on a later client. Fixing that needs a serialization
  change and was out of scope; it is recorded above, not fixed.
- No claim that the original `unsafe impl Send`/`Sync` caused a demonstrated
  memory-safety bug. A Win32 kernel handle is process-wide and usable across
  threads, so the impls were defensible in principle. The defects were the
  unargued impl, manual handle ownership, absent `SAFETY:` documentation and
  zero compile coverage for the module.
- No commit, push or CI configuration change was made.

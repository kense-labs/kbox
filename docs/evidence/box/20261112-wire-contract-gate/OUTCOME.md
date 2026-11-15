# Make cross-repository wire contracts explicit and gated

Date: 2026-11-12
Repositories: `kbox` HEAD `405c8b8`, `krun` HEAD `e06c9c2` (both uncommitted)
Result: PASS for the contract, regression and whitespace scopes below. No
runtime or cross-repo CI claim is made.

## Why this item

The architecture review flagged three remaining cross-repo risks:

1. **KCL dual source** — kbox uses `kcl = { path = "../kcl" }`
   (`Cargo.toml:117`) while krun pins
   `git rev = 76bf580454f1d69d7b826223ac50161efa685f85` (`Cargo.toml:15`).
   The local `kcl/` directory is **not a git repository**, so the two sides
   cannot even be compared. Unifying this is a product decision (path vs
   pinned rev vs patch override) and was not made here.
2. **Wire schema identifiers hand-synchronised across two repos, no gate.**
3. **krun derives guest architecture from host `std::env::consts::ARCH`**
   (`src/contract/package.rs`). Fixing it means editing krun, which currently
   carries a large parallel, uncommitted directory refactor (33 changed
   paths). That work was deliberately left untouched.

Item 2 is the one that can be improved without a product decision, without
touching krun source, and without new CI credentials — so this change does
exactly that.

## Measurement (read-only)

A whole-repository scan of versioned `.v1` string literals in `.rs` sources
(excluding `target`, `vendor`, `third_party` and submodules; skipping `//`
comment lines) found 137 distinct literals in kbox and 48 in krun. Filtering
to the wire namespaces (`kbox.engine.*`, `krun.guest*`,
`harness-assembly*`, `process-stream.*`, `krun.capability*` plus seven bare
protocol names) gives:

| | count |
| --- | --- |
| kbox wire identifiers | 28 |
| krun wire identifiers | 28 |
| **shared** (present in both) | **26** |
| kbox-only | 2 |
| krun-only | 2 |

The four one-sided identifiers are legitimate local schemas, each recorded
with a reason in `wire-contracts.json`:
`kbox.engine.journal.v1` / `kbox.engine.prewarm.v1` (kbox-internal records),
`krun.capability-package.v1` / `krun.guest-runtime.v1` (krun-owned schemas).

**Conclusion: there is no current drift; what was missing is a gate that
prevents future drift.**

### Two measurement pitfalls worth remembering

- Scanning only `kbox/crates/box/runtime/src/engine/` produced a false alarm
  that `kbox.engine.endpoint.v1` exists only in krun. It is actually produced
  at `crates/box/cli/src/commands/engine.rs:40`. Cross-repo contract checks
  must scan whole repositories.
- A naive `"...v1"` regex misses byte strings such as
  `b"kbox.engine.directory.v1\0"`. Allowing an optional escape tail recovered
  `kbox.engine.directory.v1` and raised the shared count 25 -> 26.

The earlier architecture-review claim that "krun has not adopted
`engine-diagnostics.v1`" is **outdated**: `krun/src/engine/unix.rs:93` checks
it today. Historical evidence was not rewritten; this document supersedes
that observation only.

## Changes

- **`scripts/wire-contracts.json`** (new): version-1 machine-readable
  contract with 26 shared identifiers, 2 kbox-only and 2 krun-only entries
  (each with a reason), plus the namespace prefixes used for classification.
- **`scripts/check-wire-contracts.py`** (new): pure stdlib, read-only,
  Python 3.9+. Default mode scans this repository and requires the wire
  identifier set to equal `shared ∪ kbox_only`; missing identifiers and
  **undeclared** new identifiers both fail. `--krun <path>` adds the same
  check against a local krun checkout (`shared ∪ krun_only`). Because
  `shared` is in both expected sets, every shared identifier is required on
  both sides. One scan per side.
- **`.github/workflows/verify.yml`**: the `layout` job gained a
  `Wire schema contract` step running the default single-repo mode.
- **`AGENTS.md`**: new rule that any wire identifier change must update
  `wire-contracts.json` in the same change and run the `--krun` mode.

No product source, Cargo manifest, lockfile or krun file was modified.

## Verification

See `checks/verification.txt`. All 7 commands exited 0: wire gate (single
and `--krun`), workspace-layout gate, unsafe-doc gate, full workspace
`cargo check`/`cargo clippy` (excluding `kbox-cri`, which needs `protoc`
locally), and `git diff --check`. Workspace layout remains 19 members /
18 inheriting root lints / 1 exception; unsafe-doc gate remains 35
`unsafe fn` / 0 missing.

The script also runs under `/usr/bin/python3` (3.9.6) — slower (~21 s due to
this machine's file-I/O behaviour) but PASS. `python3` here is 3.13.12.

## Negative tests

See `checks/negative-tests.txt`. kbox source was backed up and restored with
a shell trap; the krun probe used a **copy** of `krun/src` in `/tmp`, never
the real checkout.

1. Renamed one shared identifier in kbox (`endpoint.v1` -> `.v2`): CI mode
   exit 1, `declared contract missing from source: kbox.engine.endpoint.v1`.
2. Added an undeclared identifier: exit 1,
   `undeclared wire identifier kbox.engine.probe.v1 (in ...engine.rs)`.
3. Dropped `kbox.engine.discovery.v1` from the krun copy: cross mode exit 1,
   `krun: declared contract missing from source`.
4. Restored kbox and used the pristine krun checkout: PASS, 28/28 on both.

## Explicitly not claimed

- **CI does not verify the krun side.** The default `GITHUB_TOKEN` cannot
  check out a second private repository; fully automatic cross-repo gating
  would need a configured secret / deploy key. `--krun` is a local,
  developer-run verification step recorded in `AGENTS.md`.
- **No shared crate was introduced.** The two repos remain separate; this
  change only makes the duplicated contract explicit and checkable.
- **No claim of current drift fixed.** The measurement found the two sides
  already consistent; the value is regression prevention.
- **KCL dual-source and krun host-ARCH inference remain unfixed** (product
  decision / parallel uncommitted work respectively), as does physical crate
  flattening (`M-CRATES-FLAT-FOLDER`).
- **No krun file was modified.** Its 33 pending uncommitted paths and HEAD
  `e06c9c2` are unchanged; the probe used a `/tmp` copy.
- **No runtime behavior change, no native Windows test, no GitHub CI run,
  no historical evidence rewrite, and no commit or push.**

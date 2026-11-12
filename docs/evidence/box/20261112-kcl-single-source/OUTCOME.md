# Outcome: converge kbox and krun onto one pinned KCL revision

Date: 2026-11-12 18:10 +0800
Repository: `/Users/roylin/code/kense/kbox` at HEAD `405c8b8`, plus the sibling
`/Users/roylin/code/kense/krun` checkout. No commit, no push.

## Problem

kbox declared KCL as a sibling path dependency
(`kcl = { package = "kense-kcl", path = "../kcl" }`) while krun pinned
`kense-labs/kcl` at `76bf580454f1d69d7b826223ac50161efa685f85`.
`/Users/roylin/code/kense/kcl` is not a git checkout: it is a local stand-in
exported on 2026-11-09 from kbox history `591e1a5^ / crates/acl/src` when the
real repository was inaccessible. Its README still says `# kacl`. The two
products therefore used different KCL implementations under the same version
number `0.3.0`.

## Compatibility precheck (read-only)

Symbols imported from the external `kcl` crate (18 unique, in 3 files):

* `crates/box/core/src/compose/kcl.rs`: `Block, Document, Lexer, Token, Value`, `parse`
* `crates/box/runtime/src/oci/build/plan.rs`: `canonical_bytes_with_schema`,
  `canonical_digest_with_schema`, `parse_with_limits`, `validate_document`,
  `AttributeSchema`, `Block`, `BlockSchema`, `CanonicalError`, `Cardinality`,
  `Document`, `ParseLimits`, `Schema`, `SchemaDiagnosticCode`, `Value`, `ValueSchema`
* `crates/box/runtime/src/oci/build/plan/tests.rs`: `SchemaDiagnosticCode`

All 18 are re-exported by remote `src/lib.rs`; the remote surface is a strict
superset (adds `parse_kcl` / `generate_kcl`, keeps `parse_acl` / `generate_acl`
as `#[doc(hidden)]` aliases).

Byte comparison of the 9 `src/*.rs` files between the local stand-in and the
cargo git checkout `~/.cargo/git/checkouts/kcl-52282599ad9d9fe0/76bf580`:

```
ast.rs 7  canonical.rs 12  diagnostic.rs 22  generator.rs 13  lexer.rs 3
lib.rs 32  parse_limits.rs 6  parser.rs 10  schema.rs 30   TOTAL 135 changed lines
```

Every production-code difference is one of:

1. doc comments / doctest crate names `ACL` → `KCL` (`kacl` → `kcl`);
2. stable wire error-code strings `acl.*` → `kcl.*` in
   `DiagnosticCode::as_str` (7), `SchemaDiagnosticCode::as_str` (12) and
   `CanonicalError::code` (2) — 21 literals;
3. `Option::is_none_or` → `map_or(true, ...)` in `schema.rs` (equivalent,
   lowers MSRV);
4. new `parse_kcl` / `generate_kcl` entry points plus hidden aliases.

Remaining differences are test-only (fixture file suffix `.acl` → `.kcl`,
`#[allow(clippy::approx_constant)]`, assertion style, numeric constants).
`canonical.rs` digest/bytes logic is byte-identical apart from the two error
code literals and doc comments, so build-plan identity does not change.

kbox does not reference any `acl.*` literal (`git grep` over tracked files: 0
hits) and does not call `.as_str()` / `.code()` directly. `plan.rs` matches on
`SchemaDiagnosticCode` enum variants. The one observable behavior change is
that `BoxBuildPlanError::Schema`'s `#[error("... {path}: {code}")]` renders
`kcl.schema.*` instead of `acl.schema.*`, because `Display` delegates to
`as_str()`. No kbox source, test or fixture asserts that rendered text.

Resolution check: `git ls-remote https://github.com/kense-labs/kcl.git HEAD`
returned `76bf580454f1d69d7b826223ac50161efa685f85` without credentials, so
GitHub Actions can fetch the dependency with the default checkout; no secret
or deploy key is needed. `cd ../krun && cargo metadata --offline --locked
--format-version 1` returned rc=0.

Baseline tests with the old `path = "../kcl"` dependency:

```
K_DEPS_STUB=1 cargo test --locked -p kbox-core --lib compose
  -> 35 passed; 0 failed; 445 filtered out
K_DEPS_STUB=1 cargo test --locked -p kbox-runtime --lib oci::build::plan
  -> 8 passed; 0 failed; 2168 filtered out
  (includes canonical_kcl_and_digest_are_stable and
   parser_applies_a_bounded_document_limit)
```

## Changes

* `Cargo.toml`: replaced the `path = "../kcl"` dependency with the same
  `git` + `rev` declaration krun uses, plus a comment forbidding a local
  fallback.
* `Cargo.lock`: `kense-kcl` now carries
  `source = "git+https://github.com/kense-labs/kcl.git?rev=76bf580...#76bf580..."`.
  Relative to HEAD the lock diff is `+990 / -72`, but 989 of those insertions
  are the earlier workspace-consolidation change; this task added exactly one
  line (`git diff Cargo.lock | grep -c '^+.*kcl'` = 1).
* `cargo metadata` resolves the package to
  `~/.cargo/git/checkouts/kcl-52282599ad9d9fe0/76bf580/Cargo.toml` (no `/Users/roylin/code/kense/kcl` path).
* Live specification files updated (all mention the external repo instead of a
  sibling working tree): `AGENTS.md`,
  `.specify/memory/constitution.md`,
  `.specify/templates/plan-template.md`,
  `.specify/templates/tasks-template.md`, and `../krun/AGENTS.md`.
  `AGENTS.md` gained a general rule with the cross-repository change procedure
  and the two KCL-consuming test commands.
* New `scripts/check-kcl-source.py` (standard library, read-only, Python 3.9+):
  verifies the manifest declares `package/git/rev` with the exact pin and no
  `path`/`branch`/`tag`; verifies the `kense-kcl` lock source; scans first-party
  manifests for any `../kcl` path dependency; `--krun <path>` additionally
  verifies the krun manifest and lockfile. `--root` supports negative tests.
* `.github/workflows/verify.yml`: new `Pinned KCL dependency contract` step in
  the `layout` job. YAML parsed successfully (ruby `YAML.load_file`), layout
  steps are now: checkout, Workspace contract, Unsafe safety documentation
  contract, Wire schema contract, Pinned KCL dependency contract.

## Verification

Commands (all rc=0 unless noted):

```
# after switching the dependency
K_DEPS_STUB=1 cargo test --locked -p kbox-core --lib compose
  -> 35 passed; 0 failed; 445 filtered out
K_DEPS_STUB=1 cargo test --locked -p kbox-runtime --lib oci::build::plan
  -> 8 passed; 0 failed; 2168 filtered out
  (canonical_kcl_and_digest_are_stable: ok; parser_applies_a_bounded_document_limit: ok)
K_DEPS_STUB=1 cargo check --locked --workspace --all-targets --exclude kbox-cri   -> rc=0
K_DEPS_STUB=1 cargo clippy --locked --workspace --all-targets --no-deps --exclude kbox-cri -> rc=0
git diff --check -> rc=0

python3 scripts/check-workspace-layout.py               -> PASS
python3 scripts/check-unsafe-safety-docs.py            -> PASS
python3 scripts/check-wire-contracts.py                -> PASS
python3 scripts/check-kcl-source.py                    -> PASS (19 manifests)
python3 scripts/check-kcl-source.py --krun ../krun     -> PASS (19 + 1 manifests)
/usr/bin/python3 scripts/check-kcl-source.py           -> PASS (Python 3.9.6)
ruby -ryaml -e 'YAML.load_file(".github/workflows/verify.yml")' -> jobs: layout, rust, windows-cross
```

`kbox-cri` remains excluded locally because `prost-build` needs `protoc`, which
is not installed on this host; the CI `rust` job installs `protobuf-compiler`
and checks the full workspace. `K_DEPS_STUB=1` is required for check/clippy on
this macOS host (see the existing workspace-consolidation evidence).

Negative tests for the new gate (`/tmp/kcl_gate_negative_test.py`, copies the
real manifests into a temp tree and mutates one thing per case):

```
[sanity]                     rc=0 PASS  (unmutated copy)
[path-dep-regression]        rc=1 PASS
[rev-drift]                  rc=1 PASS
[lock-source-missing]        rc=1 PASS
[crate-path-override]        rc=1 PASS
[krun-rev-drift-cross-mode]  rc=1 PASS
6 cases, 0 unexpected results
```

## Explicitly not claimed

* No commit, no push, no remote GitHub Actions run. CI changes are untested
  against a real runner.
* `/Users/roylin/code/kense/kcl` was **not deleted, renamed or modified**. It
  is now unreferenced but sits outside the kbox repository; removing it is a
  separate, destructive action left to the user. The gate prevents re-linking
  it rather than relying on its absence.
* Historical documents mentioning `../kcl` were deliberately not rewritten:
  `dist/RELEASE_NOTES.md`, `docs/e2e-dsh-acp/TEST-PLAN.md`, all
  `docs/evidence/**` logs and `OUTCOME.md` files. They record what was true at
  the time.
* krun's 33 parallel uncommitted directory changes were preserved. Only
  `../krun/AGENTS.md` (an untracked file created earlier in this session) was
  edited; no krun product code or `Cargo.lock` was touched.
* Cross-repository KCL checking is not automated in CI: the `layout` job only
  verifies the kbox side, because krun is a second checkout. `--krun` must be
  run locally when the pin changes.
* The user-visible rendered diagnostic code prefix changed from `acl.schema.*`
  to `kcl.schema.*` (and the equivalent `parse`/`limit`/`canonical` codes).
  This is a consequence of adopting the canonical KCL implementation, not a
  kbox code change; no downstream contract in this repository depends on the
  rendered text.
* No claim about native VM, MicroVM, ACP or runtime behavior. No platform
  matrix was re-tested; cross-compilation and native runtime claims are out of
  scope here.

## Erratum (2026-11-12, appended; original text above is preserved)

The phrase "KCL dual source" used above and in earlier evidence documents is
imprecise and should not be read as "there were two KCL projects". KCL is a
**standalone project**, `kense-labs/kcl`, with its own Git history, CI,
`CHANGELOG.md`, `LICENSE`, examples and fixtures. The two sources were:

1. That standalone project, pinned by `krun` at
   `76bf580454f1d69d7b826223ac50161efa685f85` — the canonical implementation.
2. `/Users/roylin/code/kense/kcl`, a local, non-Git stand-in exported from
   kbox's historical `crates/acl` on 2026-11-09 when the organisation
   repository was unreachable. Its README is still titled `# kacl`. It was a
   development workaround, never the KCL project and never a second product
   implementation of it.

Nothing in this erratum changes the recorded commands, counts, lockfile
contents or conclusions above. It corrects terminology only. On 2026-11-12 the
standalone project was additionally cloned to
`/Users/roylin/code/kense/kcl-project` (HEAD equal to the pinned rev), given
its own `AGENTS.md`, and the legacy stand-in's README was marked deprecated.
The stand-in itself was still not deleted, renamed or otherwise removed.

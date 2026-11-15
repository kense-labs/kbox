# Generic local execution engine

Date: 2026-11-14. Host: macOS-26.6.2-arm64-arm-64bit. Rust: rustc 1.97.1 (8bab26f4f 2026-07-14).
Base commit: `405c8b89f151e9e2879438d9e457dd60bfd18b90`. Changes are uncommitted.

## Result and scope

Removed the product-specific package installer, reserved installation paths,
guest installation schemas, assembly receipts and journal entries, assembly
commands/capabilities/errors, and workload classification. Application type no
longer selects guest privileges. Installation and application readiness belong
to external callers using ordinary execution APIs.

Existing generic directory grants, process/bootstrap streams, generation fencing,
Runtime receipts, ExecutionManager ownership, and clean environment prewarm remain.
The Service template passes an explicitly supplied `semantics_profile_digest`.
Prewarm uses top-level `prewarmBindingsDigest`; default and explicit guest policies
are both eligible and their compatibility keys remain distinct.

Current source/tests/scripts have zero matches for the product-coupling patterns
recorded in `checks/product-coupling-scan.json`. Virtualization libkrun/libkrunfw,
native API symbols, historical research/specs and prior evidence are retained.
The current engine documentation is `docs/architecture/local-engine.md`.

## Compatibility and rollback

Discovery advertises `kbox.engine.v2`; intent and journal schemas are v2.
Request/response envelopes and generic stream protocols retain their versions.
Unknown product fields and the removed assembly command are rejected.
Older journal data is preserved and refused, never reset or rewritten.

Before upgrading an existing engine home, use the prior executable to stop its
workloads and unclaimed reservations. Retain that executable and home for
rollback, and use a fresh `K_HOME` with the new engine. External client migration
is outside this repository change. No existing user home or live execution was
modified during verification. Pre-existing deletions under `dist/` are unrelated.

## Verification

| Actual command / check | Result |
| --- | --- |
| `cargo test --locked -p kbox-runtime engine` | 182 passed, 2 ignored |
| `cargo test --locked -p kbox-runtime prewarm` | 11 passed, 1 ignored |
| `cargo test --locked -p kbox-runtime profile_tests` | 3 passed |
| Final `cargo test --locked -p kbox-runtime engine::prewarm` | 2 passed; includes default/explicit guest policy compatibility separation |
| `cargo test --locked -p kbox-runtime exec_integration_tests` | 2 passed |
| `cargo test --locked -p kbox-cli missing_console_log_message_mentions_recovery_commands` | 1 passed |
| `cargo clippy --locked -p kbox-runtime -p kbox-cli --all-targets --no-deps` | Passed with warnings; final rerun also passed |
| `cargo check --workspace --all-targets --locked` | Passed with warnings |
| `cargo fmt --all -- --check` | Failed on 51 unchanged files; each verified byte-identical to HEAD |
| `rustfmt --check --edition 2021 --config skip_children=true <19 changed Rust files>` | Passed |
| Python compile of the three affected native test drivers | Passed |
| New/current documentation link comparison | No new broken links; 13 pre-existing failures retained |
| Product-coupling source scan | 0 matches |
| `git diff --check` and final source/caller diff review | Passed |
| Built CLI control smoke | Passed; generic discovery, empty v2 journal, removed command refused, clean shutdown |

Test counts above overlap between filters. The actual CLI executable was launched
with a temporary empty home containing no application artifacts. No VM was started.
`checks/control-smoke.json` records its hash and observed protocols.
Detailed commands, times and exit codes are in `checks/results.json`.

The first test compilation found a private test-helper import. The test now reuses
the existing engine fixture; the rerun passed. The initial full log was overwritten;
the observed diagnostic is retained in `checks/test-engine-initial-error.txt`.

## Explicitly not claimed

- No native HVF/KVM/WHPX workload or VM prewarm qualification was run.
- No external client was migrated or tested against the new intent schema.
- No automatic v1 state migration or old-home startup compatibility is claimed.
- No full-workspace formatting pass is claimed; unchanged baseline differences remain.
- Historical research, planning and evidence may still name previous clients and APIs.

# Rename krun to kbox — Outcome

## Scope

Renamed the product, CLI, Rust packages and modules, OCI companion assets, configuration paths, documentation, local checkout and GitHub repository from `krun` to `kbox`.

- CLI: `kbox`; contract crate: `kbox-contract` / `kbox_contract`; OCI crate: `koci-kbox`; companion directory: `koci-kbox-runtime`.
- Default home and protected control directory: `.kbox`.
- Local checkout: `/Users/roylin/code/kense/kbox`.
- Remote: `git@github.com:kense-labs/kbox.git`; GitHub default branch remains `main`.
- `libkrun` and `libkrunfw` remain independent submodules at `fd95e4842b17eb8f691e5c124ec8a43ed5f4ea94` and `fc3ea06895301f1df6a274dad75fbd9317b3bfee`; their source and gitlinks were not changed.
- Native `krun_*` symbols, `KRUN_*` native environment keys, `krun.dll`, `krun.lib`, `init.krun*` and native runtime provenance fields remain intact.
- Historical evidence and changelog entries were not rewritten.
- The root `Cargo.lock` and standalone OCI lockfile were regenerated through Cargo. Three stale lockfiles without an active owning workspace were removed: `crates/box/Cargo.lock`, `crates/runtime/Cargo.lock`, and `crates/updater/Cargo.lock`.
- The standalone OCI workspace now resolves `klibksys` from the existing local path rather than a nonexistent registry package.

## Corrections found during verification

- Restored mechanically damaged native dynamic symbols, native asset schema fields and native guest environment keys before running native tests.
- Replaced remaining `koci_krun` entry-point and test references with `koci_kbox`.
- Corrected the runtime asset schema constant to match `koci.kbox-runtime-assets.v1`.
- Restored a distinct negative rootfs path assertion after the positive and negative cases had become identical.
- Relinked the CLI before checking its version. The final output is `kbox 3.2.7`.
- Removed remaining old repository links from the native binding crate metadata and build instructions.

## Environment

- Host: macOS, arm64, native HVF execution.
- Tests used the renamed binaries and isolated `K_HOME` directories where required by the existing suites.
- TEE was excluded.

## Commands and observed results

| Command | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked -j 8` | Passed; repeated successfully after moving to `/Users/roylin/code/kense/kbox` |
| `cargo check --manifest-path crates/oci-runtime/Cargo.toml --workspace --all-targets -j 8` | Passed |
| `cargo test --locked -p kbox-cli --lib -j 8` | **938 passed, 0 failed** |
| `cargo test --locked -p kbox-cli --tests -j 8` | **33 default integration checks passed, 0 failed**; native-only tests and 3 TEE tests ignored |
| `cargo test --locked -p kbox-cli --test command_coverage -j 8` | **16 passed, 0 failed** |
| `cargo test --locked -p kbox-cli --test local_e2e -j 8 -- --nocapture --test-threads=1` | **6 passed, 0 failed** |
| `cargo test --locked -p kbox-cli --test registry_e2e -j 8` | **2 passed, 0 failed**, including Compose pull |
| `cargo test --locked -p kbox-contract -p kbox-core -p klibksys --lib -j 8` | **23 + 486 + 12 passed, 0 failed** after correcting the rootfs test fixture |
| `cargo test --locked -p kbox-runtime volume::store::tests -j 8` | **37 passed, 0 failed** |
| `cargo test --locked -p koci-kbox --tests -j 8` | **67 + 17 + 3 passed, 0 failed**; 4 tests ignored by their existing design |
| `cargo test --locked -p kbox-cli --test host_smoke test_real_vm_command_matrix -j 8 -- --ignored --nocapture --test-threads=1` | **1 passed, 0 failed**; included `export → import → run`, snapshots, networks and image lifecycle |
| `cargo test --locked -p kbox-cli --test nginx_integration -j 8 -- --ignored --nocapture --test-threads=1` | **4 passed, 0 failed** |
| `cargo test --locked -p kbox-cli --test scale_api_process reconciles_real_replicas_and_rejects_stale_revisions -j 8 -- --ignored --nocapture --test-threads=1` | **1 passed, 0 failed**; replicas reconciled `0→2→1→0`, stale revision rejected |
| `cargo test --locked -p kbox-cli --test guest_native_rootfs_integration -j 8 -- --ignored --nocapture --test-threads=1` | **3 passed, 0 failed** |
| `cargo clippy --locked -p kbox-cli -p klibksys -p kbox-runtime --all-targets --no-deps -j 8` | Passed with warnings; no denied lint failure |
| `cargo fmt --all -- --check` | Passed |
| `git diff --check` | Passed |
| `target/debug/kbox --version` | `kbox 3.2.7` |
| `gh repo view kense-labs/kbox --json nameWithOwner,url,defaultBranchRef` | Confirmed `kense-labs/kbox`, default branch `main` |

Counts describe suite invocations and are not deduplicated. The 33 default integration checks comprise 16 command-coverage, 2 default core-smoke, 3 multi-platform-load, 6 local-e2e, 2 registry-e2e, 1 scale-process and 3 wait checks.

## Final repository state

- Existing uncommitted migration work was preserved. No reset, commit or push was performed.
- `git submodule status` confirms both native submodule revisions listed above.
- The checkout builds successfully from its new absolute path.

## Explicitly not claimed

- **TEE was not tested**, including simulated TEE, attestation, sealing and secret injection.
- **Linux KVM was not tested on this host.** Linux-only source and scripts were reviewed for naming but not natively executed.
- **Windows WHPX was not tested.** Windows-only source and scripts were reviewed for naming but not natively executed.
- **External registry delivery was not verified.** Registry tests used local fixtures.
- **Final release archives were not verified.** No archive completeness, isolated-prefix installation or release smoke test was performed.
- The full 22-test native `core_smoke` matrix and the remaining ignored `host_smoke` scenarios were not rerun for this rename; the focused native scenarios listed above were run instead.
- Existing compiler and Clippy warnings remain. These checks do not establish exhaustive correctness, performance, soak or production readiness.

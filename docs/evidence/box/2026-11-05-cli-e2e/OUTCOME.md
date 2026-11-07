# CLI end-to-end outcome — 2026-11-05

## Scope

Implemented and executed the planned CLI end-to-end matrix on the current host. TEE was explicitly excluded by the requester. Tests used the real `krun` binary, isolated `K_HOME` directories, local OCI fixtures and registries, and native MicroVM execution where required.

## Environment

- Host: macOS 26.6.2, `arm64`, Apple HVF.
- Toolchain: pinned Rust `1.97.1-aarch64-apple-darwin`.
- Workspace: `/Users/roylin/code/kense/krun`.
- Native sources: `libkrun` `fd95e4842b17eb8f691e5c124ec8a43ed5f4ea94`; `libkrunfw` `fc3ea06895301f1df6a274dad75fbd9317b3bfee`.
- Guest init: `aarch64-unknown-linux-musl`, statically linked and stripped.
- Test images: Docker Hub Alpine and Nginx; digest-pinned PostgreSQL for the native SCRAM scenario; synthetic OCI images and a loopback registry for hermetic cases.

## Changes exercised

- Dynamic Clap command-tree coverage: 59 top-level entries, 43 third-level commands, and the `container-prune` alias.
- Shared CLI process helper with isolated Home, real exit status, stdout/stderr bytes, timeout, streaming interruption and optional Home retention.
- Real local OCI import, audit, events, registry, network prune and snapshot prune scenarios.
- Real MicroVM lifecycle, Nginx HTTP, exported-rootfs import and run, and Scale API `0 → 2 → 1 → 0` reconciliation with stale-revision rejection.
- Named-volume creation rollback only removes volumes created by the failed call; existing volumes are retained.
- Unified libkrun FFI symbol prefix and current block-root environment key; retained startup diagnostics before Box directory removal.
- Corrected stale test fixtures: Nginx binary discovery, structured label assertions, Compose KCL service syntax, and PostgreSQL startup username.

## Actual commands and results

| Command / scenario | Observed result |
| --- | --- |
| `cargo test --locked -p kbox-cli --lib -j 8` | **938 passed, 0 failed** |
| `cargo test --locked -p kbox-cli --tests -j 8` | **33 default integration checks passed, 0 failed**; 3 TEE tests ignored; native-only tests ignored |
| `cargo test --locked -p kbox-cli --test command_coverage -j 8` | **16 passed, 0 failed**; no dynamic command-tree omissions |
| `cargo test --locked -p kbox-cli --test local_e2e -j 8 -- --nocapture --test-threads=1` | **6 passed, 0 failed**; import artifacts, invalid import, real audit/events, network prune and snapshot prune |
| `cargo test --locked -p kbox-cli --test registry_e2e -j 8` | **2 passed, 0 failed**; local pull/push/login/logout/compose-pull with digest and manifest-byte checks |
| `cargo test --locked -p kbox-cli --test nginx_integration -j 8 -- --ignored --nocapture --test-threads=1` | **4 passed, 0 failed**; Alpine lifecycle, exec, environment/labels, Nginx guest HTTP |
| `cargo test --locked -p kbox-cli --test scale_api_process -j 8 -- --nocapture --test-threads=1` | **1 passed, 1 native test ignored**; process restart and idempotent replay |
| `cargo test --locked -p kbox-cli --test scale_api_process reconciles_real_replicas_and_rejects_stale_revisions -j 8 -- --ignored --nocapture --test-threads=1` | **1 passed, 0 failed**; two real replicas created, reduced to one and zero, stale revision returned HTTP 409 without changing inventory |
| `cargo test --locked -p kbox-cli --test wait_integration -j 8` | **3 passed, 0 failed** |
| `cargo test --locked -p kbox-cli --test load_multi_platform -j 8` | **3 passed, 0 failed** |
| `cargo test --locked -p kbox-cli --test host_smoke test_real_vm_command_matrix -j 8 -- --ignored --nocapture --test-threads=1` | **1 passed, 0 failed**; includes `export → import → run` and `/etc/os-release` verification |
| Remaining seven `host_smoke` tests, run individually with `--ignored --nocapture --test-threads=1` | Seven process results successful; **6 executable scenarios passed**; `test_real_packages_service_push` printed a skip because `K_BOX_PUSH_TEST_REF` was unset |
| `cargo test --locked -p kbox-cli --test core_smoke -j 8` | **2 PostgreSQL wire tests passed, 0 failed**; 22 native tests ignored |
| Twenty-two native `core_smoke` tests with `--ignored --nocapture --test-threads=1` | **22 completed, 0 failed**. A 300-second whole-group invocation was stopped after 13 successful tests; the remaining 9 were then run individually and passed. No timed-out aggregate result is claimed |
| `cargo test --locked -p kbox-cli --test guest_native_rootfs_integration -j 8 -- --ignored --nocapture --test-threads=1` | **3 passed, 0 failed** |
| `cargo test --locked -p kruntime -p kbox-core -p klibksys --lib -j 8` | **23 + 486 + 12 passed, 0 failed** |
| `cargo test --locked -p kbox-runtime volume::store::tests -j 8` | **37 passed, 0 failed** |
| `cargo check --workspace --all-targets --locked -j 8` | Passed |
| `cargo clippy --locked -p kbox-cli -p klibksys -p kbox-runtime --all-targets --no-deps -j 8` | Passed with existing warnings; no denied lint failure |
| `cargo fmt --all -- --check` | Passed |
| `git diff --check` | Passed |

Counts above describe suite invocations, not a deduplicated total; several focused suites were also included in `--tests` or run again as final regression.

## Representative evidence

- Imported image execution returned Alpine `/etc/os-release`, proving that `import` produced a bootable image rather than only an OCI artifact.
- Nginx returned `Welcome to nginx` through `wget` executed inside the guest.
- `network prune` removed only the unused fixture; the endpoint-attached network remained.
- `snapshot prune --keep 2` removed only `snap-e2e-0`; `--max-bytes` then removed `snap-e2e-1`, retaining the newest snapshot.
- Scale API returned `actual_replicas` values 2, 1 and 0 across real reconciliations; the stale request returned `stale_revision` and left the one-replica inventory unchanged.
- Final process inspection found no leftover `kbox-shim`, test `krun`, `core_smoke`, `nginx_integration` or `scale_api_process` processes.

## Incidents resolved during qualification

- The legacy Nginx helper derived a nonexistent target directory and fell back to PATH. It now uses the shared binary/helper and isolated Home.
- A label assertion expected Docker-style `app=test` text inside JSON. It now parses and checks `labels.app` and `labels.env`.
- The Scale API fixture initially used YAML-style `services`; it now uses the supported KCL `service "api"` block. Server stderr is retained for diagnosis.
- The PostgreSQL wire fixture expected a stale username instead of the `kcloud` value passed by the test; corrected and rerun successfully.
- A host-wide transient prevented newly linked C and Rust executables from leaving `_dyld_start`. After it cleared, the minimal executables, `krun --version`, and all reported test suites were actually executed; no timeout is counted as a pass.

## Explicitly not claimed

- **TEE was skipped**, including simulated TEE, seal/unseal, secret injection, hardware attestation and hardware trust claims.
- **External registry delivery was not verified.** `test_real_packages_service_push` skipped because no external push reference was configured; its zero-second success is not counted as external-registry coverage.
- **Linux KVM was not verified on this host.** Linux Sandbox, Linux `port-forward`, Linux chroot build and Linux-only `real_core_virtiofs_tar_closes_every_source_file_cleanly` were not executed.
- **Windows WHPX was not verified.** Cross-compilation or static checks are not runtime evidence for Windows.
- **Final release archives were not verified.** No final archive completeness check, isolated-prefix installation or release smoke test was performed.
- Existing compiler and Clippy warnings remain; lint success is not a claim that all warning classes or all repository behavior are exhaustively verified.
- This outcome is limited to the observed macOS HVF environment and the commands above. It is not a blanket cross-platform, performance, soak or production-readiness claim.

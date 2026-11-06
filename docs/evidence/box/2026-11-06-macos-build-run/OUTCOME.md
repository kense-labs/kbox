# macOS isolated Dockerfile RUN repair — 2026-11-06

## Result and scope

Repaired macOS `kbox build` through the complete path from CLI routing to OCI layer publication, then built and ran `dsh-acp` end to end.

Changes exercised:

- The default warm-pool rootfs mount moved from reserved `/run/kbox/build-rootfs` to `/mnt/kbox-build/rootfs`; the reserved mount guard remains unchanged.
- macOS defaults `RUN` to the existing isolated VM pool when no executor is explicit. Linux keeps its previous default, and explicit pool options and the unsafe host experiment retain their existing behavior.
- The CLI Tokio runtime uses an 8 MiB worker stack, matching existing complex-runtime test practice. A Node pool build passes without `RUST_MIN_STACK`.
- Context cache hashes preserve symlink targets without following directory links and honor `.dockerignore` for context `COPY`/`ADD`. Workspace `node_modules` cycles no longer recurse or consume unbounded memory.
- OCI layers encode long symlink targets with GNU longlink entries instead of failing at 100 bytes.
- `dsh-acp` no longer removes required workspace runtime dependencies with root `pnpm prune --prod`; its image clears the inherited Node `CMD` with `CMD []`.

## Environment

- Host: macOS 26.6.2 (25G83), `arm64`, Apple HVF.
- Rust: `1.97.1 (8bab26f4f 2026-07-14)`.
- Guest init: `aarch64-unknown-linux-musl`, statically linked and stripped, built with `cargo zigbuild`.
- Base images: `docker.io/library/alpine:latest`, `node:24-bookworm-slim`.
- Final application image: `dsh-acp:kbox-e2e-fixed`, `linux/arm64`, 7 layers, digest `sha256:8ad17d2fde5561ed9c47042b063ad17edf7fa8d149bc0bba95785f275155575d`.

## Actual checks and counts

- `cargo test --locked -p kbox-cli commands::build::tests -- --nocapture`: 20 passed.
- `cargo test --locked -p kbox-runtime build_run_pool -- --nocapture`: 8 passed, including failed-RUN lease release and cache rollback.
- `cargo test --locked -p kbox-runtime oci::build::cache::tests -- --nocapture`: 14 passed, including symlink cycles and `.dockerignore`.
- `cargo test --locked -p kbox-runtime oci::build::layer -- --nocapture`: 26 passed, including GNU longlink.
- `cargo test --locked -p kbox-cli --test host_smoke test_real_build_run_pool_smoke -- --ignored --nocapture`: 1 native HVF test passed; shell/exec RUN, cache mounts, multi-stage COPY, non-root execution, and image launch were exercised.
- `cargo test --locked -p kbox-cli --test host_smoke test_real_macos_default_build_run_pool_smoke -- --ignored --nocapture`: 1 native HVF test passed with a Node base and no pool CLI flags; final rerun also passed without `RUST_MIN_STACK`.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked -p kbox-cli -p kbox-runtime --all-targets --no-deps`: passed with pre-existing warnings.
- `cargo check --workspace --all-targets --locked`: passed with pre-existing warnings.
- `git diff --check` for the touched source files: passed.

The default rootfs regression test failed before the mount-path fix and passed afterward. The longlink regression test failed with `provided value is too long when setting link name` before the encoder fix and passed afterward.

## Full application evidence

The original multi-stage `Containerfile` was built by the release CLI with no `--run-pool` flag and 4 GiB helper memory. Logs:

- `.e2e-dsh-final2.log`: successful isolated image build before application metadata fixes.
- `.e2e-dsh-production.log`: successful rebuild after removing the dependency-breaking prune.

The final build completed `pnpm install --frozen-lockfile`, native addon generation, Typert generation, TypeScript compilation, and assembly of 172 packages. The runtime image reported 169.5 MB in the builder log; `image-inspect` confirmed an empty `Cmd`, the intended ACP `Entrypoint`, and user `node`.

ACP initialization executed inside the final running image returned:

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentInfo":{"name":"deepseek-harness-acp","version":"0.0.1"},"agentCapabilities":{"mcpCapabilities":{"http":true},"promptCapabilities":{"image":false,"audio":false,"embeddedContext":false},"sessionCapabilities":{"close":{},"list":{},"resume":{}}},"authMethods":[]}}
```

Test boxes and build-pool daemons were stopped or removed after validation.

## Investigation notes

- A pool failure initially presented as `early eof`; native diagnostics showed Tokio worker stack overflow in the OCI async pull path.
- A later full-context build showed thousands of recursive calls and approximately 91.4 GB physical footprint. Open descriptors identified workspace package symlinks, and `collect_files` was confirmed to follow them with `Path::is_dir()`.
- The first valid image build then exposed a pnpm long-symlink encoding failure, a production dependency pruning failure, and an inherited base-image `CMD` argument. Each was fixed and rerun.
- The `dsh-acp` source directory was removed externally during testing. The source copy retained by the earlier running test box was restored to the same workspace path before continuing.

## Explicitly not claimed

- This is not a claim that the complete macOS test matrix or every kbox feature passes.
- Linux KVM and Windows WHPX behavior was not tested.
- `run -i` and `attach -i` were not established as ACP stdio transports. Current foreground/attach code streams logs but does not forward host stdin; the successful ACP handshake used `kbox exec` inside the running image.
- Production dependency minimization with `pnpm deploy` was not completed. Package `files` allowlists omit required runtime modules, so the functional full workspace dependency closure is retained.
- No model API call, API-key validation, session prompt, or end-user ACP client integration was tested; the protocol check covered `initialize` only.

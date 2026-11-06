# Process-prewarmed dsh-acp ACP attach — 2026-11-06

## Outcome

`kbox pool start --deferred --prewarm` starts the image's OCI `ENTRYPOINT + CMD` in a separate MicroVM and advertises an idle slot only after the application writes `kbox-prewarm-ready-v1\n` to stderr. `kbox pool attach` consumes one ready VM and initialized process, forwards ACP stdio in both directions, returns the application's exit status, destroys that VM on disconnect, and lets the daemon replenish the pool.

dsh-acp starts model input capability resolution while its ACP instance mounts. `initialize` awaits the same operation instead of resolving model information again. The ready marker is emitted only after both the ACP stdio connection and this capability resolution are ready. ACP stdout remains JSON-RPC only.

The public tested image is:

```text
ghcr.io/kense-labs/dsh-acp:latest
sha256:4237b8ac59d8ad1fb89ce065877fdd8d02f9114c17412cca8163d4276242fea5
linux/arm64, 169.5 MB, 7 layers
```

GitHub reports package visibility `public`. An anonymous GHCR token followed by a manifest HEAD request with the OCI manifest media type in `Accept` returned HTTP 200 and the same `Docker-Content-Digest`. Two earlier HEAD attempts omitted `application/vnd.oci.image.manifest.v1+json` from `Accept` and returned 404; the corrected request is the successful result above.

## Environment and revisions

- Host: macOS 27.2 (26B123), arm64.
- CLI: kbox 3.2.7 release build.
- kbox base revision: `aef1fd7c9101ab9ebcb2330396b541d916220415`, plus this task's working-tree changes.
- dsh-acp base revision: `5881fcb1b5e1f39eab798809c3af9c8af1b52021`, plus this task's working-tree changes.
- Live API selection: provider `deepseek-official`, model `deepseek-flash`, reasoning effort `low`.
- The API key was read from the owner's local configuration into the process environment. It was not written to the image, benchmark output, daemon log evidence, or this file.

## Implementation checks

The following checks completed successfully before the documentation-only edits:

### dsh-acp

```sh
cd /Users/roylin/code/kense/dsh-acp
node node_modules/vitest/vitest.mjs run --config vitest.acp.config.ts packages/acp/acp/tests/prewarm.spec.ts
# 2 tests passed

node node_modules/vitest/vitest.mjs run --config vitest.acp.config.ts
# 12 test files, 155 tests passed (includes the two prewarm tests)

node node_modules/typescript/bin/tsc -b tsconfig.host.json
# passed
```

The readiness test checks the exact stderr marker and opt-in behavior. The mount test checks repeated `initialize` calls against the pre-resolved capability operation.

### kbox

```sh
cd /Users/roylin/code/kense/kbox
cargo test --locked -p kbox-runtime pool::attach
# 4 passed

cargo test --locked -p kbox-runtime pool::prewarm
# 3 passed

cargo test --locked -p kbox-cli commands::pool::tests
# 38 passed

cargo fmt --all -- --check
cargo clippy --locked -p kbox-core -p kbox-runtime -p kbox-cli -p kbox-guest-init --all-targets --no-deps
cargo check --workspace --all-targets --locked
cargo build --release --locked -p kbox-cli -p kbox-shim
# all passed; only existing warnings were emitted

sh -n dist/bench-pool-dsh-acp.sh
# passed; the embedded Python was also parsed with ast.parse after adding model/effort selection
```

The benchmark accepts `DSH_MODEL` as either `model` or `provider/model`, and `DSH_REASONING_EFFORT` as an advertised ACP value. It applies both through `session/set_config_option`, reads back the selected values, and only then sends the default greeting `你好！请用一句话介绍你自己。`.

## Image build and publication

```sh
./target/debug/kbox build --run-pool-memory 4g --run-pool-cpus 4 \
  -t ghcr.io/kense-labs/dsh-acp:latest /Users/roylin/code/kense/dsh-acp
# Successfully built ghcr.io/kense-labs/dsh-acp:latest (7 layers, 169.5 MB, linux/arm64)

gh auth token | ./target/release/kbox login ghcr.io -u ZhiXiao-Lin --password-stdin
./target/release/kbox push ghcr.io/kense-labs/dsh-acp:latest
# Pushed manifest sha256:4237b8ac59d8ad1fb89ce065877fdd8d02f9114c17412cca8163d4276242fea5

gh api /orgs/kense-labs/packages/container/dsh-acp --jq .visibility
# public
```

The GitHub package version tagged `latest` reported the same digest at `2026-11-06T09:06:23Z`.

## Live end-to-end benchmark

One release-binary smoke session completed first:

- CLI spawn to initialize response: 23.620 ms.
- Initialize request RTT: 22.425 ms.
- Model prompt RTT: 1452.832 ms.
- Returned nonempty Chinese model text, `deepseek-flash`, `low`, `end_turn`, exit code 0.

The five-run command was equivalent to the following, with the API key already exported and not echoed:

```sh
RUST_LOG=kbox_cli=debug \
POOL_LOG=.e2e-prewarm-release-pool.log \
DSH_MODEL=deepseek-flash \
DSH_REASONING_EFFORT=low \
RUNS=5 sh dist/bench-pool-dsh-acp.sh ./target/release/kbox
```

Each iteration waited for a ready VM and process, attached, performed `initialize`, `session/new`, optional model and reasoning configuration, and `session/prompt`, then closed stdio. All five returned real model text, the requested model and effort, `stopReason: end_turn`, and exit code 0. Five sequential iterations also demonstrated destruction and replenishment between attachments.

### Raw samples

```json
{"run":1,"spawn_to_initialize_ms":19.367749999219086,"initialize_rtt_ms":17.64220900076907,"prompt_rtt_ms":1092.6203749986598,"agent":"deepseek-harness-acp","provider":"deepseek-official","model":"deepseek-flash","reasoning_effort":"low","stop_reason":"end_turn","reply":"你好！我是 DeepSeek 智能体，一个可以读取文件、运行命令、编写代码并帮你分析问题的 AI 编程助手，很高兴为你服务！","exit_code":0}
{"run":2,"spawn_to_initialize_ms":15.06620899453992,"initialize_rtt_ms":13.822208995406982,"prompt_rtt_ms":1608.1352920009522,"agent":"deepseek-harness-acp","provider":"deepseek-official","model":"deepseek-flash","reasoning_effort":"low","stop_reason":"end_turn","reply":"你好！我是由 DeepSeek 驱动的 AI 编程助手，可以帮你阅读代码、排查问题、编写和修改程序。","exit_code":0}
{"run":3,"spawn_to_initialize_ms":20.50887500081444,"initialize_rtt_ms":19.215584004996344,"prompt_rtt_ms":1676.0675409968826,"agent":"deepseek-harness-acp","provider":"deepseek-official","model":"deepseek-flash","reasoning_effort":"low","stop_reason":"end_turn","reply":"你好！我是 DeepSeek Harness 驱动的编程智能体，可以帮你读代码、写代码、调试问题并执行开发任务。","exit_code":0}
{"run":4,"spawn_to_initialize_ms":28.954542001883965,"initialize_rtt_ms":27.501750002556946,"prompt_rtt_ms":676.8831250010408,"agent":"deepseek-harness-acp","provider":"deepseek-official","model":"deepseek-flash","reasoning_effort":"low","stop_reason":"end_turn","reply":"你好！我是 DeepSeek 驱动的人工智能助手，能够协助你处理编程、分析、写作与信息检索等任务。","exit_code":0}
{"run":5,"spawn_to_initialize_ms":17.71333400392905,"initialize_rtt_ms":16.143250002642162,"prompt_rtt_ms":1436.4989579989924,"agent":"deepseek-harness-acp","provider":"deepseek-official","model":"deepseek-flash","reasoning_effort":"low","stop_reason":"end_turn","reply":"你好！我是 DeepSeek Harness 中的 AI 编程助手，可以帮你阅读代码、调试问题、执行命令并完成开发任务。","exit_code":0}
```

| Metric | Min | Median | Max |
| --- | ---: | ---: | ---: |
| Complete CLI spawn → initialize response | 15.066 ms | 19.368 ms | 28.955 ms |
| Initialize request RTT | 13.822 ms | 17.642 ms | 27.502 ms |
| Model prompt RTT | 676.883 ms | 1436.499 ms | 1676.068 ms |

The daemon logged `attach_setup_us` values of 313, 254, 232, 251, and 222 microseconds:

| Internal-only metric | Min | Median | Max |
| --- | ---: | ---: | ---: |
| Daemon attach setup | 0.222 ms | 0.251 ms | 0.313 ms |

This internal setup figure excludes CLI process startup, IPC/client overhead, ACP serialization, and the initialize response. It is not an end-to-end startup measurement.

For context only, two pre-optimization live sessions using the earlier image (`sha256:dba89cabaa38b0c5ade3939612396af0c54f6b57566fb4436126a9263f71ac33`) measured CLI-to-initialize values of 283.159 and 275.939 ms, and initialize RTT values of 281.498 and 274.400 ms. These two samples are not combined with the new five-run distribution and are not used to claim a formal speedup percentage.

## Explicitly not claimed

- Complete 1 ms CLI startup or 1 ms ACP `initialize` was **not** achieved. The measured complete CLI-to-initialize range was 15.066–28.955 ms; only the daemon-internal attach setup was below 1 ms.
- Model network latency is not startup latency. Prompt RTT depends on DeepSeek and network conditions.
- Native runtime coverage is limited to macOS arm64 with a linux/arm64 image. Linux KVM, Windows WHPX, other architectures, and cross-platform packages were not tested here.
- This was a text greeting test. Tool execution, MCP, image prompts, session persistence, project-volume behavior, and a specific third-party TUI/GUI were not exercised.
- Anonymous access was verified by an unauthenticated-token OCI manifest request, not by a full image pull on a freshly provisioned clean host.
- `latest` is mutable. Reproduce this exact test with the digest recorded above.
- Snapshot-fork is not part of this macOS process-prewarm result. No claim is made that a live ACP stdio connection can be restored from a VM snapshot.

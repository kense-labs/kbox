# Interactive main-process stdin — 2026-11-06

## Result and scope

Implemented Unix main-process stdin forwarding for `run -i` and `attach -i` using the existing libkrun split-console input descriptor.

The shim now creates a mode-`0600` Unix socket beside the exec socket and a connected socketpair. One side is passed to libkrun as the console input; the other forwards one interactive client. The CLI connects to the same short socket path, avoiding macOS `SUN_LEN` failures seen with the box-log path. Closing the interactive client closes the forwarding stream and produces a natural EOF for the workload.

`InstanceSpec.stdin_open` carries the existing configuration into the shim. The default is `false` and preserves non-interactive behavior. The implementation does not add a second process lifecycle or a new control protocol.

## Environment

- Host: macOS 26.6.2 (25G83), `arm64`, Apple HVF.
- Rust: `1.97.1`.
- Guest init: static `aarch64-unknown-linux-musl`.
- Application image: `dsh-acp:kbox-e2e-fixed`.

## Actual validation

- `cargo test --locked -p kbox-cli --test host_smoke test_real_interactive_stdin_smoke -- --ignored --nocapture`: 1 native test passed. The test drives the same Alpine `read` workload through `run -i` and detach + `attach -i`, and asserts both returned strings.
- `cargo check --workspace --all-targets --locked`: passed with pre-existing warnings.
- `cargo clippy --locked -p kbox-core -p kbox-shim -p kbox-cli -p kbox-runtime --all-targets --no-deps`: passed with pre-existing warnings.
- `cargo fmt --all -- --check`: passed.

Foreground ACP initialization through `run -i` returned protocol version `1` and agent `deepseek-harness-acp`:

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentInfo":{"name":"deepseek-harness-acp","version":"0.0.1"},"agentCapabilities":{"mcpCapabilities":{"http":true},"promptCapabilities":{"image":false,"audio":false,"embeddedContext":false},"sessionCapabilities":{"close":{},"list":{},"resume":{}}},"authMethods":[]}}
```

Detach + `attach -i` returned the same successful ACP result with request ID `2`. Both boxes exited after the client closed stdin and were removed.

## Failure investigation

- The initial FIFO design deadlocked because the shim waited for a writer while the CLI waited for exec-server readiness.
- The first socket path under box logs exceeded macOS `SUN_LEN`; moving the socket beside `exec.sock` fixed it.
- `run -i` then returned `stdin-ok:hello-stdin` and `attach -i` returned `attach-ok:from-attach`, proving byte forwarding before the ACP checks.

## Explicitly not claimed

- Windows interactive stdin is not implemented; the new channel is Unix-only.
- No TTY framing, resize, or PTY semantics were added. This is a byte-stream channel for protocols such as ACP, not a terminal emulator.
- No concurrent interactive clients are supported. One forwarding client is accepted per interactive channel.
- No model API call or prompt/session test was performed; ACP coverage is `initialize` only.

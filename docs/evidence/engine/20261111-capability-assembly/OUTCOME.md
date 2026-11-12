# Optional capability package assembly

2026-11-11, macOS 26.6.2 arm64 / HVF, Linux/aarch64 guest. The optional
`harness-assembly.capability-packages.v1` path passed native SDK and formal KRun
CLI qualification on the same KBox binary. The SDK run took 1142.15 seconds;
the CLI run took 462.99 seconds.
[Native results, commands and artifact identities](../../../../../krun/docs/evidence/2026-11-11-capability-assembly-native/OUTCOME.md)

Aster, Dsh and Codex each received the same five capability packages on their
actual execution and generation. A receipt is published only after every package
passes immutable-copy, digest, guest protocol and installation receipt checks.
The journal validates the complete ordered package set on load and primary start.
After ready-state engine SIGKILL, the aggregate receipt and provider identity
survived with every source archive temporarily unavailable.

Wrong digest, wrong package class, missing runtime dependency and an old guest
without the required protocol all refused without an aggregate receipt. Both
native runs finished with zero private executions and volumes, including an
independent observation after engine exit. No capability entrypoint was executed.

Owning verification commands:

```text
cargo test --locked -p kbox-runtime engine:: --lib
cargo test --locked -p kbox-runtime kbox_contract_driver::engine_tests:: --lib
cargo clippy --locked -p kbox-runtime -p kbox-cli --all-targets --no-deps
cargo check --locked --workspace --all-targets
cargo build --locked --release -p kbox-cli --bin kbox
```

The two test filters passed 172 and 13 tests respectively. Clippy, workspace
check, release and scoped rustfmt passed. Whole-workspace rustfmt still reports
the same 50 pre-existing paths; none were added or removed.
[Logs and exact comparison](../../../../../krun/docs/evidence/2026-11-11-capability-assembly-native/checks/checks-summary.json)

The same signed binary, guest-init, shim and native libraries passed independent
ordinary OCI qualification from a prefix without a host KRun binary, taking
186.11 seconds. Pull, build, run, exec, external files, volumes, networking,
export, commit, snapshot and engine restart remained available; cleanup reached
zero resources.
[OCI result](../20261111-capability-assembly-oci/e0553e837405413d8bf4a96b6b96d683/result.json)

## Explicitly not claimed

Archive delivery does not prove native Skill, MCP, tool, plugin or model use.
KBox adds no vendor configuration parser or Harness task loop. Native adapters,
service ownership, in-flight fault coverage, full Harness acceptance and
performance sampling remain open in KRun's implementation plan.

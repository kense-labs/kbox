# T5 CLI `--isolation` flag

Date: 2026-11-17  
≠Pass

## Behavior

- `CommonBoxArgs.isolation` default `sandbox`
- `--isolation microvm` sets `ExecutionBoundary::Microvm`
- TEE/confidential forces Microvm regardless of flag

## Hermetic

```text
cargo test --locked -p kbox-cli --lib test_build_box_config_defaults_to_sandbox
cargo test --locked -p kbox-cli --lib test_build_box_config_honors_explicit_microvm
→ 1 passed each
```

## Explicitly not claimed

Native MicroVM `kbox run --isolation microvm` smoke this pulse.

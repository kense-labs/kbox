# Native HVF probes

These Linux arm64 Guest probes and bounded CLI PTY drivers are the exact source
bytes used by the accepted capability/shared-memory qualification. They require
the existing opt-in native harness, fixed workload images, a sentinel-owned test
root and separately installed Box/OCI archives. They do not use the real K_HOME.
Final commands, identities, counters and cleanup are recorded in
[integration evidence](../../../../../docs/evidence/runtime/20261116-integration-closeout/TASKS.md).

Build each C probe with the existing Zig toolchain:

```sh
zig cc -target aarch64-linux-musl -static -O2 -Wall -Wextra -Werror -pthread \
  -o /absolute/owned-output/probe /absolute/path/to/probe.c
```

The capability fixture consumes `K_BOX_TEST_CAPABILITY_PROBE` and
`K_BOX_TEST_CAPABILITY_CLI_SCRIPT`; the shared-memory fixture consumes
`K_BOX_TEST_SHM_PROBE` and `K_BOX_TEST_SHM_CLI_SCRIPT`. Both use the existing
`K_BOX_TEST_HVF_*` installed-asset inputs and exact ignored test names in
`oci_hvf_native_capabilities.rs` / `oci_hvf_native_shm.rs`. The Python drivers take
the installed CLI, owned root, execution ID and new output directory as arguments.
Assertions stay active in optimized C builds. Rebuilding from a different source
path may change assertion-location bytes in the ELF; retain its new identity.

These tests establish only the named installed assets on the actual native host;
cross-compilation, ignored tests and configured VM memory do not establish a
runtime result or RSS. See each OUTCOME's explicit limitations.

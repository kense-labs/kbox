# Native generic-engine qualification

Use the installed binaries and companions from a fixed release archive. Supply
an isolated, writable test image store; these drivers pull/build images and must
not point at a real user's image cache. Each driver owns fresh private homes and
retains failed runs and cleanup results.

Image prerequisites:

- `native_smoke.py --base`: Linux image containing python3 and bubblewrap.
- `native_primary.py --image`: Linux image containing python3.
- `native_prewarm.py --image`: Linux image containing Node.js (`node`).
- `native_readonly.py --image`: macOS arm64 host and a Linux/aarch64 image
  containing Python. It grants guest mount privileges, proves successful
  writable remount/new mount followed by host `EPERM`, checks links/children,
  live host updates, retained-route restart, explicit overlap refusal and cleanup.

An image containing Python, Node and bubblewrap can serve all three. Pin the
resolved OCI digest. The prewarm driver also takes `--test-binary`, the executable
reported by `cargo test --locked -p kbox-runtime --lib --no-run --message-format=json`.
This client drives the same-source library using the archive's actual runtime
companions. It does not prove release CLI scheduler performance.

Pass `--kbox /isolated-prefix/bin/kbox`, `--image-store /private-cache/images` and
`--output /private-results` to each driver. A successful process alone does not
establish native acceptance: inspect result.json and cleanup inventories.

Current exact commands, archive/companion/source hashes, generic image digest,
actual test counts, native loader maps and cleanup:
[final A–F archive evidence](../../docs/evidence/runtime/20261114-final-archive/OUTCOME.md).
The final acceptance runs outside the developer cwd with the installed prefix
and system tools on PATH. The prewarm and directory clients each must execute
one test and produce a checked PASS result; a zero-test exit is not acceptance.
The same-source frozen test client is a driver, not an additional shipped binary.
Historical [A–D baseline](../../docs/evidence/runtime/20261114-release-baseline/OUTCOME.md)
and [E source qualification](../../docs/evidence/runtime/20261114-macos-readonly/OUTCOME.md)
remain separate. These scenarios do not claim a full OCI Host Service cutover
or increase platform readiness.

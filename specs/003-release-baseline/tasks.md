# Tasks: Generic MicroVM release baseline

- [x] A1 Reproduce default Runtime/CLI/contract tests; retain initial working-tree identity and failures.
- [x] A2 Fix host_fs resource conversion and necessary callers with boundary and partial-update tests.
- [x] A3 Refuse unsupported StateSnapshot configuration and preserve truthful capability discovery.
- [x] A4 Resolve MicroVM-only routing, ownership, process identity and snapshot recovery regressions without weakening adversarial assertions.
- [x] B1 Repair installer source ownership, archive inputs and fixed companion provenance in scripts/package-macos-arm64.sh.
- [x] B2 Test isolated installation, missing/wrong-architecture companions, checksum corruption and rollback; repair support-matrix references.
- [x] C1 Apply mechanical formatting separately; strengthen scripts/verify-commit.sh to full default tests and required checks.
- [x] C2 Run the full verification entry point and record exact final results.
- [x] D1 Verify the final installed archive through existing native CLI/stream/prewarm checks on eligible hosts.
- [x] D2 Record artifact identities, upgrade/refusal checks, cleanup inventories and remaining host limitations.
- [x] Review the final scoped diff; preserve prior changes; do not commit or publish.

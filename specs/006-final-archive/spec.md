# Feature Specification: final A–F macOS arm64 archive acceptance

Created: 2026-11-14. Status: final archive acceptance and cleanup complete on
the named existing macOS arm64 host. Owners: release scripts, Box Runtime
qualification clients, release documentation/evidence.

## Scenarios and acceptance

1. Preserve the previous archive, checksum and staging tree, then build current
   CLI, shim, Linux/aarch64 guest-init and matching native libraries. Verify
   source/companion identities, architectures and the shim's entitlement/signature.
2. Install the exact resulting archive in a new private prefix. Native VM workers
   load that prefix's companions, without developer-tree or user-install fallback.
3. Reuse native OCI-image/engine, read-only remount/restart, process stream,
   prewarm and directory grant clients. Check actual test counts, PASS artifacts,
   identity/generation fencing and exact private cleanup, retaining failures.
4. Run `scripts/verify-commit.sh`. A source fix regenerates affected artifacts
   and repeats affected native checks before acceptance.

## Boundary

ExecutionManager, route/identity/recovery and generation ownership stay intact.
No new backend, product configuration, dependency, native-source edit, business
special case, user-home mutation, commit or publication. Support readiness stays
unchanged. Full OCI Host Service cutover remains incomplete; Windows ARM64
remains unsupported by current dependencies and unverified natively.

## Evidence

`docs/evidence/runtime/20261114-final-archive/OUTCOME.md` records actual commands,
source/native/companion hashes, compressed bytes, installed usage, failures,
counts and cleanup. Runtime memory is not reported without measurement.

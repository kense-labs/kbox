# Exec readiness deadline and retained warm-pool investigation

The total admission deadline is now enforced by the existing shared VM readiness
loop. This does not claim that the original intermittent parallel startup timeout
is fixed, or that enterprise GA/full W4 cutover is complete.

## Source and implementation

Starting main was69fffadb. While this task was running, a separate local action
committed the same source/evidence as70df92be (the recorded commit has a Cursor
co-author). This controller did not issue that commit. No work was overwritten.
The1709 compiled inputs remain byte-identical to the frozen source digest
`d4cc47ffa43d69bfe7e762ae8fc6b7f4c4973ac209f73fc169d00c8dd113d20e`.
[Commit/content binding](checks/source-commit-binding.json).

`vm/ready.rs` previously gave every attempt its full500ms Unix/3s Windows
budget, then accepted a heartbeat before checking the total deadline. A final
attempt could therefore admit Ready after the total cap or outlive the remaining
budget. Each attempt and poll now uses only the remaining total budget; successful
responses recheck the deadline before retaining readiness proof. Default15000ms,
terminal-status reconciliation and existing platform ceilings remain unchanged.
An internal timeout argument also removes the existing test's process-global
environment mutation. No new configuration, dependency or resource authority.

The red run actually executes4 tests:2 existing pass,2 new fail (late heartbeat
returns Ok; wedged attempt exceeds40ms). The repaired VM subset passes214 and
pool subset83. A positive real Unix Frame heartbeat control also proves Ready
publication remains available. [Red](checks/ready-red1.json),
[red failures](checks/ready-red1.stderr), [VM](checks/vm-green1.json),
[pool](checks/pool-green1.json).

`bash scripts/verify-commit.sh` passes in297.90s: Runtime2277/0/46ignored,
CLI935/0/0, contract suites, scoped Clippy and8 installer tests. No assertion or
timeout was relaxed. [Gate](checks/verify1.json), [stdout](checks/verify1.log),
[stderr](checks/verify1.stderr).

## Installed assets and actual native results

Existing macOS26.6.2 arm64 host; no fresh-host/platform promotion. Source was
frozen before building, and the previous archive is retained. Final Box archive:
`artifacts/warm-pool-readiness-bfa18104e9e9434b9b89e85e425ff763/final-dist-readiness-1/kbox-3.4.3-macos-arm64.tar.gz`.
SHA256`fd20baccb4cbfe793a1d98483f8e1a4406fda0022125ac28952d544ebbace2ea`;
23,400,527 compressed/44,175,360 installed bytes. The new isolated prefix is
`installed-readiness-final-1` under that root. All11 companions, architecture,
signatures, links, build provenance and unchanged native dependencies are checked.
The fresh frozen native client SHA256 is
`a43a28334444de309018fb686fed239efb498359f097b854d1895e00eef195ed`.
[Install](checks/install-readiness-final1.json),
[identities](checks/final-identities-1.json), [client](checks/frozen-native-client.json).

Before repair, wave1 passes4 concurrent OCI/engine Python scenarios; pool-wave2
passes4 daemons with16 authenticated idle VMs and4 real Guest commands. These
bounded diagnostic waves do not reproduce the original failure.
After repair, actual final installed results are:

- wave3:4 concurrent OCI/engine Python scenarios pass,18 shims observed and137
  live log snapshots retained. [Results](checks/wave-3/native-results.json).
- pool-wave4:4 daemons,16 authenticated idle VMs and4 real Guest commands pass,
  with shutdown/empty inventories checked. [Results](checks/pool-wave-4/native-results.json).
- A–F:all5 scenarios pass, specifically2 Rust tests actually running1/0/0 each
  and3 Python scenarios. OCI lifecycle, Host RO/remount/child/restart, primary
  process streams, prewarm reuse, directory grants and generation fences remain.
  [Actual rows/counts](checks/af-final1/native-results.json).

No generic summary line, filtered test, configured VM memory or cross build is
used as native runtime evidence. The OCI assets are unchanged from the prior
integrated archive and retain their original provenance.

## Resource audit and unresolved external state

The owned-resource audit passes21 actual homes:zero managed executions/volumes,
owned processes/mounts/workload disks/FD holders,106 observed runtime socket
directories absent and441 observed socket paths absent. All443 original cloned
image blobs compare equal. The audit deletes no resources.
[Home inventories](checks/final-home-inventories.json),
[FD holders](checks/final-fd-holders.json), [audit](checks/final-cleanup.json).

**The overall historical RAM preservation gate remains FAIL.** The previous
phase recorded7 unrelated RAM mounts; current hdiutil reports none. This run did
not issue their detach commands. Their removal actor is not established; the
user has been asked whether another agent/cleanup ran. The failing assertion and
all three audit attempts are retained. No unrelated mount is recreated or cleaned
to make the comparison pass. [Observed delta](checks/ram-comparison-failure.json),
[strict final audit failure](checks/final-audit3.json).

Audit1 also caught the concurrent Git HEAD change. Comparing every compiled
input proves the source unchanged; the new commit is a descendant of the actual
build base. This metadata change is recorded rather than rebuilding identical
source or rewriting provenance. [Failures](FAILURES.md).

## Explicitly not claimed

Original intermittent parallel warm-pool startup cause remains unresolved. Its
old retained failed-boot directory contains only two empty shim logs, no console.
New diagnostic waves retain live snapshots; passing retries do not erase the
original failure. Full OCI Host Service/default cutover, complete feature parity,
live continuity, upstream conformance/fresh-host/long-soak/security gates and
enterprise GA remain open. Windows ARM64 dependencies remain unsupported with
no native run. Runtime RSS is unmeasured; no platform level is raised.

Next executable local slice: patch the existing openssl lock entry covering8
official advisories. Cargo's index DNS and the official static crate endpoint
currently fail to resolve; retain those failures, use available verified cached
metadata/build checks, and continue other local work. No VMM/firmware port or
release publication is started. [Current task](TASKS.md), [context](context.json).

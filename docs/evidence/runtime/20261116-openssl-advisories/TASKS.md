# Existing OpenSSL dependency advisories

Enterprise GA remains the overall active goal. This slice addresses the8 actual
root Cargo.lock alerts through the existing native-tls/reqwest callers. All
current behavior/source/artifacts remain, including concurrently committed70df92.

- [x] Read8 official GitHub advisory records and actual target-specific call tree.
- [x] Verify upstream latest patch covering this set0.10.80, official cached
  checksum a45fa2aa…, rust_version1.80,yanked=false. Host Rust1.97.1 meets it.
- [x] Retain index/static crates.io DNS failures. Do not repeat blocked endpoints.
- [x] Precisely update existing openssl lock resolution offline; inspect all
  dependency/version/checksum changes and prove all8 known ranges excluded.
- [x] Run available compile/registry/TLS callers, default gate and source review; retain actual failed/blocked target results.
  Missing crate download/native target evidence is an explicit external limit.
- [x] Rebuild/install affected Box archive, compare exact payload/dependency
  identities, relevant native acceptance and only owned cleanup.
- [x] Record OUTCOME and continue the next executable W4/GA gap. No readiness
  promotion, remote advisory dismissal, VMM/firmware port or publication.

Warm-readiness task has a separate strict historical RAM preservation failure
pending user clarification. Its own21 homes/processes/mounts/106 socket dirs/
441 paths are clear, but original intermittent startup remains unresolved.
Do not overwrite those failures or infer enterprise GA from local passes.

## Current checkpoint

Lock3cec984f… uses2 security version patches and one justified tempfile resolver
edge,573 packages/no additions/removals. Sourcef3392138…/1709 inputs, base9d279a1c.
Archive3b68798d… rebuilt and freshly installed with11 verified companions.
Actual ordinary CLI/VM/Guest exec control passes and owns no remaining resource.
Available CLI935,contract,Clippy and installer8 pass. Full verify remainsFAIL:
Runtime2271/6failed/46ignored; direct diskutil reports unavailable DiskManagement,
and immutable old source also fails. Linux check needs uncached OpenSSL0.10.80;
index/static DNS unavailable. CUA service startup also fails. No more live jobs.

Do not rewrite these failures as accepted. Network/system-service restoration
and historic RAM removal actor are external pending conditions, but local error
diagnostics can be fixed and tested independently. Next task: actual native-tool
status/program/bounded stderr at the existing SDK/rootfs boundary. Afterwards
continue the next W4/GA source gap without claiming full GA or platform promotion.

Native-tool diagnostics are now implemented and checked in the
[next phase](../20261116-native-tool-diagnostics/OUTCOME.md). Its rebuilt Box/OCI
archives replace this intermediate candidate for current-source use. Disk-service
and Linux dependency checks remain externally blocked. The latest user requests
Git convergence before another feature stage; see that phase's Git closeout.

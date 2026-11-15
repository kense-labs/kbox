# Existing OpenSSL dependency advisories

Enterprise GA remains the overall active goal. This slice addresses the8 actual
root Cargo.lock alerts through the existing native-tls/reqwest callers. All
current behavior/source/artifacts remain, including concurrently committed70df92.

- [x] Read8 official GitHub advisory records and actual target-specific call tree.
- [x] Verify upstream latest patch covering this set0.10.80, official cached
  checksum a45fa2aa…, rust_version1.80,yanked=false. Host Rust1.97.1 meets it.
- [x] Retain index/static crates.io DNS failures. Do not repeat blocked endpoints.
- [ ] Precisely update existing openssl lock resolution offline; inspect all
  dependency/version/checksum changes and prove all8 known ranges excluded.
- [ ] Run applicable compile/registry/TLS callers, default gate and source review.
  Missing crate download/native target evidence is an explicit external limit.
- [ ] Rebuild/install affected Box archive, compare exact payload/dependency
  identities, relevant native acceptance and only owned cleanup.
- [ ] Record OUTCOME and continue the next executable W4/GA gap. No readiness
  promotion, remote advisory dismissal, VMM/firmware port or publication.

Warm-readiness task has a separate strict historical RAM preservation failure
pending user clarification. Its own21 homes/processes/mounts/106 socket dirs/
441 paths are clear, but original intermittent startup remains unresolved.
Do not overwrite those failures or infer enterprise GA from local passes.

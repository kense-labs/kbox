# Outcome032: cold pool admission failure cleanup

Bounded default-libkrun correction accepted on this existing macOS arm64 host.
The full GA goal remains active. Original intermittent15s startup cause remains
unlocated; this is a separate proven resource leak after boot already succeeded.

## Change and behavioral regression

Cold pool boot serves on-demand misses and background fill. After boot(), its
independent exec availability check and Ready publication used early returns.
VmManager has no Drop reaper, so failed post-boot admission discarded a running
handler without teardown. Existing process-prewarm and restored branches already
requested cleanup. The common cold block now routes every post-boot error through
existing destroy_manager_or_reap before returning the original error. Valid
admission/prewarm remain; no timeout, backend, identity/generation, configuration,
dependency or lifecycle store changes. Owning paths: pool/warm_pool.rs and sibling
pool/warm_pool/post_boot_tests.rs.

First compile caught a missing test-only Path import. Corrected red4 actually
runs2pass/2fail: missing exec path and vanished channel leave the handler live.
Positive real Frame authentication and old prewarm-refusal cleanup controls pass.
Green4/0/0, then full pool87/0/1ignored. Unit tests exercise real VmManager
filesystem teardown/caller-data retention, not a claimed native fake-handler VM.

## Checks and installed assets

- Pool full:87pass/0fail/1ignored.
- scripts/verify-commit.sh:exit101; workspace all-target compile and fmt pass;
  Runtime2282pass/6fail/49ignored. Same2 Secret/RAM and4 APFS/provider disk-service
  failures remain verbatim in checks/verify1.log. No assertion/ignore/default or
  threshold was weakened; the full gate is not green.
- Runtime/CLI all-target Clippy passes. Separate CLI935/0/0 and contract library
  24/0/0 follow-through pass. Unchanged installer retains prior8-case evidence;
  current package self-install and all checksum/companion checks actually ran.

Frozen source `241de04e39157c00593a913209d70757b4ccdc27c736ec9af70b15c758c94fa5`,1718 inputs, build base
`ffd92b00da154229970910c2c1ab091517003877` with scoped uncommitted change; lock/native gitlinks unchanged.
New Box archive: `/Users/roylin/code/kense/kbox/artifacts/pool-postboot-cleanup-0b717b3c20f543679b0d676ed54a1294/final-dist-postboot-1/kbox-3.4.3-macos-arm64.tar.gz`.
SHA256 `fb8ca51fe430a20514b173a39d71bfc3ebc5c3c984d68609c156e989dc8e18c6`;
23404277 compressed bytes / 44175360 installed disk bytes.
Prefix `/Users/roylin/code/kense/kbox/artifacts/pool-postboot-cleanup-0b717b3c20f543679b0d676ed54a1294/installed-postboot-final-1`. All11 companions, architecture, signatures, dynamic
paths, source/lock binding and native dependencies checked. Frozen native owner
client SHA256 `5ddd92472a0e7083ca871f4d751b6c100bb439c96bba063ee866edbd3d8dcd16`.

The fault test invokes the production pool helper from this frozen owner client
with actual installed shim/guest-init/native payloads. A-F/concurrent pool use
the actual installed CLI. No development target runtime dependency or real user
installation is borrowed. OCI source is unaffected: R36 archive reused with its
original provenance, not newly rebuilt. OCI SHA256
d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79,
source d2b442f175f89b34d6ba9b63e6d2df4a478dc2a6c3493f8ca647a761e9012871.
Both previous archives remain unchanged. No submodule source edited; clean native
gitlinks retained. Runtime RSS was not measured.

## Actual native effects

Exact test pool::warm_pool::post_boot_tests::native_lost_post_boot_channel_stops_real_vm_and_late_writes
executes1pass/0fail/0ignored. A real VM first authenticates and performs actual
caller writes; only its exact owned exec socket is removed. The unchanged120s
post-boot check refuses. Measured check+cleanup is 120.085317084s.
The real handler is finalized, VM stops, private box/socket directories disappear;
1137 permitted bytes remain unchanged after another500ms.
Caller canary stays intact. Actual PID/config and loader hashes/live logs are
retained before cleanup. This proves this cleanup, not the original15s cause.

Installed A-F5 rows pass:3 Python scenarios (OCI lifecycle, Host readonly attacks
and restart, primary streams) plus2 exact Rust1/0/0 tests (prewarm identity and
directory authorization/fencing). One separate concurrency matrix passes4 pool
daemons,16 authenticated idle VMs and4 real Guest commands with clean shutdown.
Totals are3 Rust native tests +3 A-F Python scenarios and one separate concurrency
matrix; no generic printed summaries are counted as additional tests.

Audit passes10 actual homes, including the raw-owner home with no managed catalog:
zero managed execution/volume, owned VM/shim/engine/process/mount/disk/FD holder;
2 scoped socket directories absent and
9 expected/recorded socket paths absent. All632 original
immutable cache files retain bytes/UID/GID/mode; mutable catalog delta recorded.
Caller data and previous archives remain; audit deletes nothing. See
checks/final-cleanup.json, final-home-inventories.json, native-fault1.json,
checks/af-final1/native-results.json and checks/pool-wave-1/native-results.json.

## Remaining work and Explicitly not claimed

Previous goal turn was actual progress (031 source/archive/native/cleanup and
local main commits), not a wait/status-only turn. Current .kstudio metadata is
bootstrap/unbound, not product completion. Project k-yolo entry is absent;
/k-init is its repair workflow, not automatically run. No fake executor/resource/
check binding or new orchestration was created. Existing repository workflow
continues under explicit continuous-development authorization.

Original intermittent warm-boot cause, full W4/default Host Service cutover,
live continuity, fresh hosts/conformance/security/soak and enterprise GA stay
open. Existing destroy-stop-failure ownership/orphan fallback edge cases are not
claimed solved by this successful-stop correction. Linux/Windows native evidence
is absent; Windows ARM64 dependencies remain unsupported/no native run. Prior
seven-RAM disappearance remains a failure. Six default disk-service failures and
prior SDK failure remain external. Git UID501/DNS slices stay parked, without
repeated push under the unchanged blocker. No real ~/.kbox, unrelated resources,
native submodule source or publication touched.

Next local causal gap from this trace: boot_diag.rs claims bounded retention but
copies arbitrary recursive logs without byte/entry/depth limits and follows
symlinks. Fix a small adversarial regression and harden it before another startup
reproduction wave. Stop-failure owner retention remains separately open. Neither
is replaced by a GA claim.

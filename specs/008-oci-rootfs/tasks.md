# Tasks: OCI guest-native rootfs

Status: **complete for this bounded slice**; final acceptance and cleanup in [OUTCOME](../../docs/evidence/runtime/20261114-oci-rootfs/OUTCOME.md). Earlier checkpoints below are historical.

- [x] T01 Preserve dirty tree, old archives/checksums, native gitlinks and initial process/mount/device inventories; read handoff/contracts; establish directory foreign-UID refusal baseline (2 actual tests).
- [x] T02 rootfs/oci_ext4: workload-only ext4 entry point and logical process inspection; reuse layer fixtures to verify root/foreign UID/GID, links/modes/whiteouts/corruption/bounds/partial publication.
- [x] T03 SDK/agent-protocol: versioned rootfs extension, exact identity/manifest compatibility, unsupported admission and adversarial tests.
- [x] T04 runtime/kbox/Agent: handoff/pinned second disk, guest mount, sandbox boundaries and exact-owned cleanup; focused tests for corruption/source drift/stale target/independent instances/interruption.
- [x] T05 Box vm/oci_production: wire new transport through explicit persisted OciSdk path and shared process resolution; retain old directory rejection and mixed-route/recovery/fencing tests.
- [x] T06 Build/freeze separate OCI native assets and Box archive; native foreign UID/GID fixture, permissions, two instances, stop/restart/stopped recovery/RO/Exec/cap_drop ALL; retain every failure/count/identity.
- [x] T07 Full owning suites, Clippy/fmt/integration and scripts/verify-commit.sh; rebuild/reinstall/replay affected assets after fixes.
- [x] T08 Confirm only owned resources cleaned; final source/artifact/dependency hashes, sizes and counts, OUTCOME and W4 row; final diff/reference review.

## Continuation (2026-11-14, implementation + first native complete)

Root artifacts: artifacts/oci-rootfs-1a469ffdee774403b7bae4b58f254647. Private root: /private/tmp/kbox-rootfs-1a469ffdee77. Evidence: docs/evidence/runtime/20261114-oci-rootfs. `run-context.json` contains current asset/cache pointers and nested `prior`; prior assets/evidence were not changed.

Implemented: workload-only LogicalRootfs ext4 (no guest-init), shared process-view resolver, Box Mac explicit preparation/capability check, auto-required SDK rootfs-image v1 descriptor, Agent manifest v5, generation-owned sibling disk moved outside guest share, pinned writable FD + exact-file Seatbelt permission, Agent block inventory/mount, handoff-owned cleanup. Immutable input is verified OCI blobs; each operation builds a private disk, ownership transfers to the runtime generation. No shared writable ext4 cache. Older directory transport UID gate remains.

Actual checks: baseline 2 refusal tests; workload builder 8 passed (later added capacity test); SDK 2 passed; protocol 12 passed; HVF directory 8 passed (includes 3 new disk cases); Agent VM attachment 7 passed (later added exact disk inventory case). Runtime full: **2175 passed, 0 failed, 14 ignored**, box-runtime-full-1.log. OCI full suites and Clippy are running in artifact logs. The `oci_production` filter executed zero tests and is **not** counted as validation; full Runtime suite supersedes it.

Failures retained: builder-1 path normalization failed 1 test, fixed trim at ProcessRootfs boundary, builder-2 passed 8. contract-check-1 Vec argument compile error fixed; contract-check-2 passed. SDK-tests-1 used wrong capability test API, fixed SDK-tests-2 passed 2. native-client-build-1 called nonexistent stop API; fixed via public kill_with_options(SIGTERM) and build-2 passed.

**First actual HVF run (diagnostic fixed assets): native test 1 passed / 0 failed / 0 ignored**, 241s; attempt-1/result.json, foreign-ownership.json, behavior.json, cap-drop-all.json, committed-response-loss.json, faults.json, ownership-after-stopped-recovery.json, cleanup-boxes.json retained under private root. Guest uid1001/gid2002, foreign gid3003 permissions, wrong uid denial, two instances, graceful stop/restart gen2, old generation, host SIGKILL/stopped recovery/restart, readonly attacks, Exec replay, mixed BoxVm/OciSdk all passed. Native harness exited 1 afterward because its archive-CLI assertion remained enabled while this preliminary run intentionally had no new archive CLI. **Do not call this final archive acceptance**; final repeat must use newly installed Box CLI and satisfy the unchanged CLI assertion. Raw test counts and loader identities recorded in checks/box-native-attempt-1.* / loader-attempt-1.json.

Fixed ownership image: docker.io/library/kbox-hvf-ownership@sha256:a6845bea2508837efa7717113f0715fa3cf86303497c9c39816a59a83d9708fa. Built in artifacts/build-ownership-fixture.py from preserved pinned Alpine plus two numeric-metadata layers; new image cache is isolated. Installed OCI diagnostic prefix: oci-workload-installed-1; its Host and Agent were built before the most recent SDK auto-required extension, v5 resource-bound refinements, inventory test and unused import change. **Rebuild all affected final OCI assets** before final acceptance. Original OCI pinned native libraries stay distinct from Box libraries.

Latest modifications after first run: blocking jobs retain TempDir guards across cancellation (no late orphan spool); capacity/corruption tests strengthened; SDK tests moved to sibling file; v5 consumes existing storage bound; native rootfs test now records exact disk inode/descriptor and asserts removal. Need rebuild native client after these edits. Full gates/archives/installed replay/A-F regressions/owned cleanup inventory/docs/W4 still outstanding. No confirmed external blocker. Next: inspect running OCI full suite + Clippy outputs, fix root causes, then full verify and rebuild/install exact Box/OCI assets and replay native with CLI enabled. Do not stop at these preliminary passes.


## Latest checkpoint: installed CLI regression fixed, candidate 2 required

Candidate1 final source was e49866c8c91be49dc19b8f3cd5f8dc50d06e8a521c4f53d3f2bb3e125a6e05ff. Box archive SHA33a075d88f766104cfbe865a2d186ef006869c56cbf7b494ae76eba837ac19d6; OCI archive SHA536c529a99497e852b4f62d879824f9f2ed7a6901a73f69f1c3153baadcf50de. Both remain retained at final-dist / oci-final-dist and isolated installed-final / oci-final-unpacked. Candidate1 archive A–F **all five passed** (OCI/readonly/primary/prewarm/directory), counts/results/real library identities in checks/archive-final and archive-final-runner.log. Both verify-commit-1 and verify-commit-final passed; final-source OCI runtime **430 passed**, other full suites Agent24/Protocol62/Kbox71+2ignored/SDK210; Box2175+14ignored. Clippy-final passed. Source changed afterward for the following genuine acceptance failure, so candidate1 is not accepted as final.

**attempt-2 archive native failed 0 passed / 1 failed / 0 ignored (208.5s)**: foreign owner/GID/isolation/restart and RO/cap_drop checks passed, then actual installed Box CLI creation hit `refusing to invent a rootfs diff baseline` because new disk preparation had not populated rootfs_snapshot.json. The source rootfs exists only in the guest and independent Agent never runs Box guest-init. Root cause fixed by deriving the existing pristine diff map directly from LogicalRootfs (same mode/size/link/path semantics and core byte/entry limits), returning it from blocking preparation and publishing through the existing first-writer-wins baseline installer before bundle handoff. No CLI assertion removed and no boot asset injected. Focused **cli-baseline-rootfs-1 / -2 logs: 9 passed**. Full archive retry remains necessary. Initial test file collision was corrected: original macos_vm_attachment_tests.rs fully restored (git diff empty); new tests live in macos_workload_rootfs_tests.rs.

New source freeze is in artifacts/source-frozen.json; prior frozen map saved as source-frozen-candidate1.json. New candidate scripts prepared: build-box-final2.py (new final-dist-2/installed-final-2), rebuild-oci-final5.py (new prefix5), then adapt package-oci-final.py to unique `oci-final2-*` destinations and prefix5. OCI code itself unchanged by CLI baseline fix, but rebuild/provenance repack will use the new whole-source snapshot. Need rerun verify-commit, Box focused/full + clippy, native client build; preserve logs with new suffixes. Then full installed native attempt3 **with finalBoxPrefix enabling the unchanged CLI assertion**, A–F repeat as needed, and final cleanup/docs/W4.

Native attempt2 did perform owned cleanup: cleanup-boxes.json exists; final global inventory not yet done. Actual workload FD sampling caught **3 owned OCI workers**, checks/rootfs-fds-attempt-2/inventory.json plus lsof records. First-run and attempt2 cache contents/provenance remain retained. No external blocker. No source/workspace/native submodule changes beyond the scoped files. Next action: build candidate2 and replay the exact installed CLI path; do not end here.


## Candidate 2 now in final native replay

Source SHA a54f07ddea510d319125b0deec7c1aeb48c2e94f6c2af70e74f1911b251069c3. Only five Box Runtime files differ from candidate1; OCI source/ABI/native assets unchanged. Guest baseline map is computed in the blocking builder; existing atomic first-writer installer runs only after the awaited builder returns, so a cancelled build cannot publish a late per-execution baseline. Preparation TempDirs are owned by the blocking jobs under the private K_HOME, outside removable execution directories.

Box archive final-dist-2/kbox-3.4.3-macos-arm64.tar.gz SHA34e2a8f09f7c029319e8a86495c1e9607dfb83941fe6a0752ab4972aa617f889; compressed23309401, installed44044288, 11 checksums verified, prefix installed-final-2. OCI archive oci-final2-dist/koci-hvf-0.3.6-macos-arm64.tar.gz SHAfb1bc50474bea3802325352e1400d0811ed6177c42597a53a78b252acbcadba5; compressed33124162, installed113782784, 10 verified, prefix oci-final2-unpacked/koci-hvf-0.3.6-macos-arm64. Both include the same source SHA. OCI system image SHA5f03ac4dd9326463ab23e104bff80cfb960048f3b4254b63e032d55abd9b34b4, manifest SHAb10bbf6a996397d1eaffda609e0a70d93d7950116331d931c981985f7ee0eab0. Prior candidates preserved, no commits or submodule edits.

Focused new baseline test passed9; cli-baseline-clippy passed; verify-commit-candidate2.log ended `verify-commit: OK`. The new installed CLI **targeted native smoke passed**: /private/tmp/kbox-rootfs-1a469ffdee77/cli-baseline-check/result.json, route OciSdk,529 baseline entries, guest stat1001:2002:600, managed cleanup[]. All actual commands and Host logs retained in that owned root.

Running sessions: full installed Host/Box native attempt3 (exec session32153), A–F five-scenario archive replay2 (89342), exact rootfs FD collector3 (91188). Raw logs checks/box-native-attempt-3.*, checks/archive-final2-runner.log, checks/rootfs-fds-attempt-3/. Final native client frozen from checks/native-client-candidate2.jsonl, path clients/kbox-runtime-test-final2. Task must continue until these pass or root causes are fixed and affected assets rebuilt again.

Need final: inspect native3 result and CLI assertion, A–F five results/counts, cache byte hashes unchanged; Box installed signatures/architectures and actual guest-init/system-Agent identities; every owned private home execution/volume inventory, all own Host/VM/shim/engine/log processes, socket paths from loader/FD commands, mounts and RAM devices vs initial seven (do not clean baseline resources). Then rewrite final OUTCOME/commands/failures/artifact/source/dependency summary and W4 foreign-rootfs row only, keep full parity/live/default/fresh/other-platform gates open, Windows ARM64 dependency-unsupported. Runtime memory unmeasured. Final source hash must still match source-frozen.json; final diff/reference checks; mark tasks complete only then.


## Completion checkpoint

All T01–T08 delivered. Final native attempt3 passed **1/0/0**, CLI assertion true,
256.90s. Final installed A–F replay2: **5/5 PASS** with actual result files,
prewarm1/0/0 and directory1/0/0. Source and archive hashes are candidate2 values
above and in OUTCOME. Source freeze unchanged after builds. Complete identities,
signatures, architectures and extracted embedded Agent verified. The final
verify-commit-candidate2 gate and Box/CLI Clippy pass; OCI source is unchanged
from the 430/210/62/71/24 full owning passes. All failures/candidate1 archives remain.

Final resource audit: **14 owned homes**, zero execution/volume records, own
Host/VM/shim/engine/log processes, workload disks or mounts; **175 observed
socket paths absent**. Eight source blobs unchanged. Initial seven RAM devices
and unrelated mounted image untouched; no broad deletion performed. Final actual
FD capture has seven OCI VM workers; A–F has19 verified loader mappings and the
mixed matrix8. See checks/final-cleanup.json and final-identities.json.

No external blocker remains for spec008. Broader complete parity, live sessions,
default cutover, fresh-host/other-platform/release gates stay open; Windows ARM64
is dependency-unsupported with no native evidence. Runtime memory unmeasured.
Next separate local task: single-file caller grants through the existing SDK/Host
PEP without a writable/readable parent-directory grant. Do not begin it within
this completed scope. No commit/push/publication or native-submodule edit occurred.


Final read-only global socket audit found12 nodes from three Box IDs created on
2026-11-07 and present in the initial process record. They are unrelated and were
left untouched, like the initial seven RAM mounts. See preexisting-global-sockets.json.
Final reference audit resolves all34 checked local targets; HEAD/native repositories
and source SHA remain unchanged. All initial hashed source files still exist and
only scoped files changed. No remaining work in spec008; broader gates are explicit.

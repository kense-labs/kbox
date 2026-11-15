# Spec010 tasks

- [x] Read handoff/rules/owning docs, close009 actual evidence and preserve dirty state/old archives.
- [x] T001 Reproduce actual old installed CLI ignoring file authority in oci_hvf_native_rollback.rs; retain state/canary/Host observations and cleanup.
- [x] T002 Fix shared metadata/store/resource-plan/replay boundary with hermetic malformed/adversarial/legacy tests; preserve exact ID/route/generation and no-file compatibility.
- [x] T003 Focused tests, full Box/CLI checks, Clippy/fmt/workspace and scripts/verify-commit.sh; retain exact counts and failures.
- [x] T004 Freeze/rebuild affected final archive, isolated install and actual old/new CLI/native replay; source/dependency checks and owned cleanup.
- [x] T005 Evidence/owning/release/ledger update, diff review and immediately continue W4 next local item.

Evidence/context: docs/evidence/runtime/20261114-oci-file-rollback/run-context.json.
Initial dirty status/binary diff, mount baseline, environment and isolated roots
are saved under that context's artifact root. No stage/push/commit/publication.
Current: add real old-CLI reproduction before choosing a fence. Prefixing an
already-validated creation digest may reject strict old managers, but old CLI
quarantine fallback must also be examined; no blanket safety claim from parsing.


Checkpoint: native before1 executed0pass/1fail (14.41s), actual old008 start exit0,
callerFiles became0, directory canary exposed through real HVF. Cleanup[] and
Host socket removed. Full before state/output/loader evidence retained.
Focused before2 executed1pass/6fail; initial before1 was a test module path
compile failure, not a test pass. Trial symlink store passed7tests but was
withdrawn after stale-writer analysis. Native atomic-layout-probe proves real
macOS file/directory exchange and stale rename EISDIR. Final directory layout
focused after2 was8pass/1fail due a stale read_link fixture assertion from the
withdrawn layout; corrected to assert retained future records bytes, keeping
missing/version refusal checks. Focused after3 all9pass. Current all BoxState
focused and native-after4 client compile running; actual old-command matrix next.
No native dependency/Agent/SDK change, so retain exact installed OCI7 bytes.
After prototype passes run owning/full gates and freeze/rebuild Box archive;
final new installed CLI must also adopt an exact legacy flat fixture unchanged.
Then full installed file/Host/rootfs/mixed-route/recovery matrix + A–F and final
owned audit, evidence/ledger, continue W4 next local item.


Prototype native2 PASSED1/0/0 in1.78s: actual old008 start/inspect/rename/run
all refuse the directory store; bytes unchanged, caller binding1 retained,
Host listempty and noVMworker loaded. Cleanup[]/socketremoved. This run does
not claim current installed-CLI adoption (cliAdopted=false); finalarchive rerun
must require cliAdopted=true. Full-before identity/canary exposure remains.
Newactualinstaller regression passed8tests (versionedstate inode/bytes retained
on install and forced installed-executable rollback). verify1 progressing:
Box2188/19ignored andCLI933 passed; long KContract journal test still running.

Caller review found raw-state Secret nondisclosure evidence and conformance
fixture cleanup need the new layout. They now read actual physical bytes via
existing BoxStateStore resolution and remove only their owned fixture store;
no serialization that could hide leaked unknown fields. Native raw-byte helper
uses same resolution. Source frozen2 is
12cd38dc44997121340fcd05d057b449f775a1768ea3a8d7d104aba7dcd575af.
Prior freeze1 retained, not an accepted asset. Box rollback-final1 build started
from frozen2; all previous dist archived. OCI7 byte-identical/unchanged.
Next verify2 against frozen2, compile final nativeclient, finalinstalled adoption
and full file/Host/rootfs/mixed/CLI/recovery matrix, A–F and owned finalaudit.


Candidate1 source2 complete: verify2 passed (2188/19ignored;CLI933;installer8),
installed compatibility3 1/0/0 withcliAdopted=true, failedCreate5 1/0/0 in45.38s,
A–F5/5, fullmatrix4 PASS/cleanup[]. All results retained with candidate1 archive.
Final review found a real fail-closed race: after directory data_path validation,
its record file could disappear before exists(), returning empty instead of error.
New concurrent-loss test reproduced0pass/1fail (0.98s). Fixed shared missing-file
branch to allow missing-as-empty only for legacy flat stores; fenced file reads
now surface actual I/O errors. No deletion/skip/lowered assertion.
Source3 focused10tests running, then freeze3 +newBoxcandidate2, verify3 and
newinstalled compatibility/full/failedCreate/A–F repetitions. OCI7 unchanged.
Do not label source2/candidate1 as final accepted after this behavior change.


Source3 focused all10compatibility tests PASSED after record-loss correction.
Frozen3 b60caefc24eb123281d699c8d268cdb6ddc9605d25a3487676fee5aa82645689.
Candidate1 fullnative4 finished1/0/0 in384.51s, still historical source2 proof.
Candidate1 resource audit PASSED11homes/89absent sockets/0resources; preserved
candidate1-final-* copies before next wave. No broad cleanup. Source3 Box2
build running via build-box-final2.py (session55035); verify3 and then final3
nativeclient build running via session17603. Both started after freeze3.
After Box2 install: prepare source3 final identity script/new AF2 context;
compile metadata native-client-final3.log, setcontextnativeClient to immutable
copy, run rollback/adoption6, fullmatrix7, failedCreate8, AF2fivecases usingBox2.
Do not reuse source2 nativeclient/final1AF results as final source3 acceptance.
Finalaudit must add all newhome scopes, caller original/canary checks and lsof
owned descriptors. Docs/release/ledger update only after final acceptance.
Next W4 local named-volume slice already has reusable Box VolumeStore/SDK
Caller+DetachOnly classification in oci_storage_attachments.rs. CurrentHVF
profile refuses policy.volume_names, genericbind IDs differ from namedidentity;
need a focused refusal baseline, exact store lookup/identity/cleanup/restart
qualification and existing helper reuse, not merely delete the profile check.
Anonymous imageVOLUME/Secret lifetimes remain separate later bounded slices.
User active goal is enterpriseGA; still no commit/push/publish/default or
support promotion, and no ending while W4 local work remains.


Frozen3 Box2 built/isolated-installed successfully (101.58s package command; JSON authoritative): archive9856cab0edc2bbe4ff2f0f2da1e0537d2e65ee73a4f44e358656e04a44918078,
23,333,369compressed/44,060,672installed bytes;11checksum entries; identities2
checks signatures/arm64/staticaarch64/embeddedAgent. Source3 OCI source3inventory
proves unchangedOCI7 source/assets/archive. Nativeclient-ready3/frozen3 immutable
copy SHA5eed111dac5ca0dfdc87f03173b3c94b9425b7ab91b932ab117c99d3efbe3611.
Native6 installed new-CLI adoption +old4commands running (session50932);
full7(session42060), failedCreate8(session21529) andAF2(session51429) launched
separately.
Full7/failed8/AF2 must complete beforeaudit/finalacceptance. Verify3longjournal
stillrunning; finalnative-client-final3 metadata scheduled afterit. Source frozen3
muststayunchanged untilalltheseacceptancecommands complete. No blockers so far.


Final accepted checkpoint: source3 b60caefc24eb123281d699c8d268cdb6ddc9605d25a3487676fee5aa82645689;
Box2 archive9856cab0edc2bbe4ff2f0f2da1e0537d2e65ee73a4f44e358656e04a44918078.
verify3 exit0:Box2189/19ignored;CLI933;installer8;fmt/workspace/Clippy/contracts.
Final installedcompat6 1/0/0 (cliAdopted=true),full7 1/0/0 in379.89s,
failedCreate8 1/0/0 in41.34s;AF2all5PASS. Nativeclient frozen3 SHA5eed111dac5ca0dfdc87f03173b3c94b9425b7ab91b932ab117c99d3efbe3611.
Finalaudit20homes/177absent sockets:0executions/volumes/processes/mounts/disks/
owned runtime descriptorholders.8immutableblobs unchanged;initial7RAM devices
and unrelatedmount preserved. Caller sources/canaries/parents retained;directory
state fence intentionallykept. Candidate1audit11/89 andallfailedcandidateproofkept.
Finalreview-source3 checks unchangedfreeze/supplemental hashes/references/archive
retention/cleanparentsubmodules/emptyindex/diffcheck. See finalOUTCOME.

Spec010 accepted; continuousW4/enterpriseGAgoalNOTcomplete. Immediate nextlocal
work is boundednamedVolumeStoremountslice (preflight/lookup/storageID/HostPEP/
restart/cleanup), reusingexistingoci_storage_attachments andBoxVolumeStore.
Create011spec/plan/tasks andrefusalbaseline, thenimplementwithactualnativeassets.
No userconfirmationneeded; no externalblockpreventingthatnextwork. Do notfinal.

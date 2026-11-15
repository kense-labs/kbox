# Spec029 retained failures

No acceptance claim before actual tests and installed-asset verification.

## Red admission/compiler1

Actual0passed/4failed/0ignored in79.20s wrapper. All failures identify the
existing Mac non-SYS_ADMIN blanket guard, including valid shared additions,
specific/all drops, preallocation admission and per-UID effective-empty requests.
The existing closed compiler now receives non-special additions; its allowlist,
defaults, validation and drop precedence remain unchanged.

## Focused after1: drop assertion polarity

Actual3/1/0. The new test accidentally asserted that a specifically dropped
capability remained present. It now asserts absence for each of MKNOD,SETFCAP,
AUDIT_WRITE,SYS_ADMIN, retaining exact default count/drop ALL/unknown checks.
The production compiler correctly removed all four; no production workaround
or capability/default change was needed. Failed test/log retained.

## Nonroot completed-bundle red1

Actual0/1/0,12.33s wrapper: resolved UID1001 with MKNOD/SETFCAP/CHOWN had
empty inheritable/ambient sets. The existing Agent applies capability sets before
exec and carries only declared ambient values across a nonroot ordinary exec.
Completion now uses the actual resolved OCI UID and carries only explicitly
requested, still-permitted additions. Defaults and dropped values never enter
ambient; UID0 behavior, drop ALL and per-UID capability-free budgets remain.
No new wire contract/executor/parser/dependency is introduced. Native2 adds
UID1001 positive and specific-drop cases; source/client/Box/verify regenerated.

## Installed native2: exec caller still used the root capability compiler

Actual0/1/0,63.29s wrapper. Initial UID1001 probe completed, but the following
captured exec failed exact effective-capability readback (exit134). Root added,
specific-drop, ALL and nonroot NPROC controls had completed all five paths.
The init completion applied the explicit inheritable/ambient subset, while
oci_session::launch still called workload_capabilities directly. Extract that
existing completion into apply_workload_capabilities and use it before rlimits
in both init and exec, so stream/PTY share the same actual UID rule. No assertion
was removed. Native2 cleanup-boxes is[], source pre/post snapshots match; its
failed root, installed2, logs and partial observations are retained. Rebuild
affected Box/client and rerun native/default/affected gates before acceptance.

## Third-candidate archive driver destination

Review caught that build-box-final3.py retained the literal final-dist-cap-add-2
destination. Its mkdir cannot replace that existing candidate and will refuse
after building. Preserve that failed command and original driver. The separate
install-box-final3.py requires a successful package-cap-add-final3 receipt and
exact current/frozen source equality before archiving the freshly built package
under a new final-dist-cap-add-3 directory and installing a new prefix. No prior
archive is moved, overwritten or relabeled; unchanged source needs no redundant
compiler run. Final acceptance still checks the actual new archive/build-info.

## Final-review3 evidence filename and home inventory preparation

Strict-review3 refused missing final-identities-3.json. The generated identity3
driver had retained the literal final-identities-2 output name and replaced that
JSON with actual candidate3 data. Preserve the original driver and failed review;
move the verified candidate3 data to its proper new filename, then revalidate
the retained candidate2 archives/installed files/signatures/embedded Agent against
their own frozen2 build-info. final-identities-2.json now explicitly records
revalidatedAfterEvidenceFilenameCollision=true; it is a later read-only
revalidation, not the original byte-for-byte JSON. Candidate2 native logs,
receipts, source freeze and artifacts were unaffected. Strict-review4 checks
the actual candidate3 identities, not a relabeled older artifact.

Home inventory preparation also initially counted three package K_HOME environment
paths that the build never created. The actual catalog roots match exactly three
native homes plus three complete five-home A–F runs (18). The planning assertion
was corrected from this observed receipt-derived set; no execution is excluded.
package-homes-unused.json records those absent paths. Source audit checks all21
pre-grant snapshots, including the failed native2 attempt.

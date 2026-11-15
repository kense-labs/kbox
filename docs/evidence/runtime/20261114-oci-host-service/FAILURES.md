# Retained failures and corrections

Every attempt has a fresh owned root and immutable client copy. Failed logs and
artifacts remain; later passes do not turn earlier attempts into passes. Native
attempt 3 was basic composition only; attempt 12 was the complete library path;
attempt 14 additionally exercises the archive-installed CLI.

| Attempt / check | Actual failure | Correction / evidence |
| --- | --- | --- |
| Native 1 | Portable directory was an empty staging root when Box selected direct guest ext4 assembly | Use the existing case-sensitive APFS provider for OCI preparation only; BoxVm retains direct ext4. `box-native-attempt-1.*`, `apfs-directory-transport.*` |
| Native 2 | Image contained foreign UIDs, which unprivileged directory transport cannot preserve | Keep the admission gate; assert this refusal, then use a pinned all-UID0 Alpine fixture. No ownership normalization. `foreign-uid-refusal.json` in each later root |
| Initial Host matrix | Generated Unix endpoint exceeded SUN_LEN | Shorten the owned work parent; no socket validation changed. `host-service-matrix.*`; second matrix passes |
| Native 4 | External-storage worker refused config.json mode 0644 | Publisher creates it at 0600. Original worker validator unchanged. `private-config-red.*`, `private-config-green.*` |
| Native 5–6 | Guest probe printed PASS but host writable file was absent | Zig -O2 defines NDEBUG; disabled assertions had removed actions/checks. Fixture now undefines NDEBUG. **These probe outputs are invalid evidence.** Preprocessed corrected source retained in `readonly-probe-assertions-fixed.preprocessed.c` |
| Native 7 | Corrected probe could not remount because exec discarded the admitted guest capabilities | Initial process and HVF exec now share the existing bounded capability compiler; other drivers unchanged |
| Native 8 | Active record without binding was admitted by current-runtime discovery | Fail closed for an active unbound record; interrupted-start discovery stays bounded to Starting/RestartStarting. `missing-binding-fix.*` |
| Native 9 | Successful Exec response was dropped; replay falsely conflicted | Capability HashSet order changed the request digest. Shared process serializer sorts only capability sets; Host v7 fingerprints normalize Exec. v1–v6 retain their encoding rules. `caps-order-*`, `operation-schema-compatibility.*` |
| Native 10 | Test process aborted by stack overflow after authenticated stopped recovery, before restart completed | Reuse the repository's 32 MiB native runner / 8 MiB worker stack pattern. Exit -6, no successful test count or cleanup claimed |
| Maintenance 10 first attempt | Canonical remove refused pending restart_starting | Reconcile original operation, reload generation, then kill/remove. `native-cleanup-10-2.*` passes exactly 1/0/0 and cleans that root |
| Native 11 | Projection worker died between readiness and drain | Handle typed Unavailable from either stage; require exact worker terminal confirmation; retain unknown Box exit status and do not forge drain evidence. `projection-drain-race-fix.*` |
| Native 13 | CLI fixture omitted required `--` before the guest command | Correct the caller syntax. Existing stored-route CLI inspect/exec had passed; new create was not counted as passed |
| Final review after candidate2 | ` all ` / `CAP_ALL` could re-add guest SYS_ADMIN because the HVF extension used a different spelling check | Red regression reproduced the bug; shared normalization reused for add/drop precedence. Four spelling variants pass; native15 verifies all five kernel capability masks zero in the replacement candidate. `cap-drop-all-red.*`, `cap-drop-all-green.*` |
| First archive runner | Loader assertion assumed the installed shim lay below the scenario output directory | Accept the exact archive-installed shim or its owned copy, while retaining hash equality and exact library-prefix checks. Original five runner failures remain in `checks/native-results.json`; fresh final runs are under `checks/archive-2/` |

Compile-only failures are also retained: missing test trait/import/default-field
assumptions (`native-client-first-build.*`), unsupported generated MutGetters
(`hvf-storage-resources-first.*`), test API argument mistakes
(`hvf-behavior-client-build.*`), tuple Generation access/imports
(`hvf-fault-client-build.*`), and the new service test's missing macro import and
unreleased exclusive state lock (`caps-order-runtime*`). Each was corrected;
no assertion, fault registry, default test or admission gate was removed.

The first archive's companions remain byte-identical to the final archive's
companions. The archive was regenerated after the test-source correction so
its source-input digest matches the final checkout. Both archives and both
isolated installations remain available.

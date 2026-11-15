# Reuse file grants and require actual memory backing

Trace: BoxTransientSecretStore/immutable managed_secret_root→preflight and exact
caller-file reservation→OCI bind/FileGrants v1→SDK required memory-file extension
and Secret extension classification→Host existing hvf_files hardlinks→shim
pinned readonly file→existing Agent file bind. Existing same-filesystem contract
requires a RAM runtime share for Secret files; do not copy to disk or reinterpret
ordinary v1 grants. Reuse/move existing macOS ram:// validation into owning Host
adapter for Box and actual shim checks; avoid duplicated filesystem policy.
No new Agent mounter. Compatibility analysis and a real red protocol test proved
that an old shim can ignore unknown annotation constraints in an otherwise v6
manifest. Memory files therefore require new manifest v8,with existing file
fields but explicit changed semantics; v1–v7 retain their bytes/meaning. Both old
Host capability refusal and new-Host/old-shim refusal need actual native proof.
Raw anonymous disks + memory files remain explicitly unqualified in this slice.

Establish failing preflight/required contract tests,implement SDK validation and
capability/Host checks,Box callers and tests,then native RAM controls/failures,
freeze/build/install final assets,impacted regressions/verify/audit/evidence.
The runtime boundary must revalidate actual memory backing,not merely trust the
Box policy label. Secret environment manifests remain explicitly unqualified.

Actual HFS RAM calibration cannot hold the fixed4GiB logical image in512MiB.
An owned APFS RAM probe proved4GiB logical/0 allocated bytes and cleanup. The
shared OS backing helper now follows the exact APFS volume to every physical
store and verifies all are hdiutil ram:// devices; mixed/disk/unknown mappings
refuse. Place this shared host-platform helper in SDK (existing serde_json/std
dependencies),re-export fromKbox for existing Box callers,and use it in Host
capability probing plus actual shim PEP. A physical runtime share must not
advertise memory-file capability. This avoids duplicated inspection or a new
Runtime→native-Kbox dependency. No default route/readiness/platform promotion.

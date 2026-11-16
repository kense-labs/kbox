# Spec036: owned pool template artifacts

clear_snapshot_template unconditionally derives a directory from ambient home+
image hash and deletes it even for a pool that never allocated it. build_template
also uses that shared image-hash path across independent pools and configurations;
a build flock does not provide ownership over another live consumer's artifacts.
Fix actual allocation/cleanup ownership, preserving user/caller/unrelated inputs.
Use existing filesystem/temporary-directory adapters and one actual resource
owner; do not infer ownership from TemplateState or an image hash. Bound source
VM and active consumer lifetime before deleting owned templates. Failed stop or
partial creation must retain exact resources for retry, never publish false clean.
No duplicate execution state/config/workspace/new dependency or native source edits.

Establish real isolated filesystem red before implementation. Preserve existing
owned-directory deletion assertions; make fixture ownership explicit and add
unowned/cross-pool/path-replacement/failure controls. Do not delete assertions or
skip tests to preserve unsafe old canonical cleanup. Rebuild affected assets and
qualify actual Host filesystem/Mac cold-pool behavior using installed companions;
Linux snapshot-native evidence is an external gate, not supplied by this host.
Use private K_HOME for every test; verifier now enforces private K_HOME/short TMPDIR.
No real ~/.kbox cleanup or historical repair. Full GA/outer caller lifetime remains
active until actual contracts and remaining local work close.

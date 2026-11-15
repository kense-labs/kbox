# Retained failures

SDK before1 actual0passed/1failed/0ignored establishes unsupported memoryv3.
After1 actual7passed/1failed/0ignored reaches the new path but the new fixture
tries to register the same capability twice; SDK correctly rejects duplicates.
After2 uses separate old/new capability inventories, retaining old-host refusal
and all exact version/binding/raw/RO checks. No production guard is weakened.

Protocolbefore1 actual0/1/0 establishes missing combined transport; after1
actual21/0/0 after explicitv11, NPROCv10 binding and downgrade checks.
Boxbefore1 actual0/1/0 establishes the original always-refuse combination guard.
Boxafter1 now checks required memoryv3+rawv1 from the existing SDK capabilities
before returning planned identities; direct tests include each missing capability.

Native client drafts1/2 compile-fail E0308: the new fixture retained create's
ExecutionReservation where restart returns ExecutionLease. It now retains the
actual start() lease after separately asserting reservation/replay/no-volume
materialization. No assertion is removed. Both compiler JSON/logs are retained.

Native raw-positive1 actual0/1/0 before VM launch: disk home to RAM handoff
fails ENOSPC28. Package identities all pass. Own1GiBRAM /dev/disk14 unmount/
detach both0, owned runtime processes[]. The dense-copy call and existing
sparse ext4 copy path are under investigation; no acceptance or external-block
claim and no RAM-size workaround. Failed sources/packages/receipts retained.

Native raw-positive2 actual0/1/0 reaches real VM/raw manifest+seed validation;
sparse disk/RAM copy succeeds at unchanged1GiBRAM. Guest unit ELF upload then
correctly refuses78615168bytes >33554432decoded FileAPI bound. Driver confirms
actual home executions[]/volumes[] and ownedprocesses[]; ownRAM unmount/detach0.
No bound/test change. Zig objcopy strip-debug returned unimplemented (zero output
retained). Existing toolchain llvm-objcopy strip-debug still49MB; strip-all yields
24916008bytes. All SHF_ALLOC sections (code/rodata/data/unwind/TLS/BSS), machine,
entrypoint and assertions/test descriptors are identical to original compiled
ELF; independent section/hash proof retained. Debug/symbol sourceELF preserved.
New test fixture fits unchanged32MiB transport; actual8/13native counts still
required. No Rust/product/source2/package change or relabelled identity.

## Final audit2 catalog-format error

The audit treated the archived `boxes.json` directory as a flat JSON file and
failed with `IsADirectoryError`. The actual spec010 layout is defined by
`box_state_layout.rs`: `boxes.json/records.v1.json`. Audit3 retains both the
actual installed CLI zero-execution/zero-volume checks before RAM detach and
validates the archived catalog as a regular, non-symlink empty records file,
with no additional directory entries. No data or resources were mutated by
this audit failure; audit2 and its logs are retained.

## Final review1 raw descriptor shape error

Review1 incorrectly indexed raw storage entries using the ordinary-file
`attachment.accessMode` shape. Actual `VmStorageAttachment` entries have direct
`accessMode`, `ownership`, `cleanup`, and `sourceIdentity` fields. Review2 uses
that protocol shape and additionally checks caller/detach-only and the exact
4 GiB logical disk capacity; no runtime or acceptance assertion is removed.
The failing review receipt is retained.

## Final review2 nested test-count assumption

The NPROC Guest parent harness reports exactly5 tests; two of those isolated
admission paths launch a single-test child harness each (zero and last-slot).
Review2 incorrectly expected a single test-result line. The retained stdout
contains exact successful counts1/0/0,1/0/0,5/0/0 in order. Review3 asserts
all three lines and retains the parent receipt5, explicitly reporting the two
child runs separately. The stage total is21 top-level Guest tests plus2 nested
child test executions, not29. No test/filter/assertion changed.

# Plan: preserve file authority across readers

Read AGENTS/constitution and Box/OCI owning docs. Reuse ManagedExecutionMetadata,
BoxStateStore atomic lock/write, immutable creation identity and current SDK
generation fences. Start with actual spec008 binary + current installed Host
using the existing native harness; test first, choose the smallest complete fence
from observed behavior. Inspect legacy CLI quarantine semantics as well as the
strict manager so an invalid version cannot masquerade as an empty collection.

Reach: box_record.rs, local_execution/backend.rs and creation/replay consumers;
box_state.rs and managed_execution_store.rs only as required; sibling unit tests,
oci_hvf_native_rollback.rs and actual installed CLI harness; owning README,
release/ledger/spec/evidence. No SDK/Guest/native changes expected.

Constitution check: I/V/VIII fail-closed explicit version; II existing owner;
III retained failure/candidate evidence; IV/X real old/new CLI and Host objects;
VI/VII one workspace/no dependency/config; IX preserve caller data; XII no
submodule source or gitlink edits; XIII exact persisted IDs; XIV unchanged host
claims; XV assertions and required gates retained. All PASS for this plan.

Sequence: reproduce -> shared cause and compatibility contracts -> tests/full
checks -> freeze/rebuild affected archive -> isolated installed-native matrix ->
owned cleanup/evidence/ledger -> next existing local W4 gap. Any immutable-old-
consumer limit must remain an explicit unsupported release gate.


Chosen from reproduction: a digest/version parse error is unsafe in the old CLI
because its quarantine path moves the entire collection and starts empty.
A symlink to a directory rejects fresh old reads, but a stale old writer can
replace the symlink with a flat file. The final fence is a real directory at
logical boxes.json, with records.v1.json as its sole authoritative record file.
macOS renamex_np(RENAME_SWAP) atomically exchanges an existing flat file and a
fully fsynced private directory. The old flat inode is removed only from the
exact owned staging path after publication; failed preparation drops its owned
TempDir. New stores use ordinary same-parent directory rename. No fallback to a
flat file on unsupported exchange/platform; the existing state lock is retained.

Normal current load adopts legacy009 file-bearing records under the lock with a
fresh read; readonly load stays side-effect-free. No-file legacy stores stay
flat. Once adopted, empty-after-cleanup stores stay fenced. Missing/future or
corrupt directory data never invokes quarantine-to-empty. Managed transitions
cannot remove or replace caller_files. Creation digests/IDs/routes/generations,
replay and wire/protocol schemas are unchanged. This changes a persisted store
layout, not resource ownership or the number of authorities.

Native atomic probe proved mixed file/directory exchange succeeds and a stale
regular-file rename over the resulting directory fails EISDIR. Prefix/symlink
trial implementation was withdrawn before final assets. Nine focused tests
cover migration, no-file compatibility, corruption/missing/future/symlink data,
concurrency, old-writer refusal, eager adoption and immutable file authority.


Final record-loss correction: directory resolver verifies records exist, but the
old shared missing-file check could race a subsequent loss and synthesize an
empty store. The new 4000-iteration concurrent-loss regression failed0/1/0
before the one-condition correction and all10compatibility tests now pass.
Missing-as-empty is retained only for legacy flat roots. Fenced reads surface
actual I/O/parse errors, including loss after resolution. Source3 supersedes
candidate1 and requires a rebuilt archive and related installed matrix replay.

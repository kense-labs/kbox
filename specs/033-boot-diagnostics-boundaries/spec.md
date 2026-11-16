# Spec033: bounded descriptor-confined boot diagnostics

Owner: kbox-runtime vm/boot_diag.rs, lifecycle retention caller and existing
workspace cap-std/cap-fs-ext filesystem adapters. Goal: failed default-libkrun
startup must retain useful bounded evidence without following untrusted paths or
blocking unrelated async work. Original intermittent15s cause remains unknown.

## Actual defects and required behavior

Current recursive std::fs copy follows symlink files/directories, has no byte,
entry or depth bounds and recursively deletes the prior target. Bind exact home /
box id / box directory before side effects. Pin each source/destination child
without following links; snapshot only flat regular single-link log files.
Prioritize known console/shim logs, inspect at most64 general entries plus one bound witness and the
four known names, copy at most64 files,1MiB tail per file and8MiB total. Skip and
report links/special/nested inputs and truncation explicitly; do not claim a full
snapshot from a limit. No recursive tree walking or unsafe target truncation.

Preserve existing flat latest-slot location and ordinary retry behavior. Target
ancestors/reparse aliases refuse; bound inspection before clearing a previous
slot, preserving unsupported legacy nested/oversized targets instead of deleting
an unbounded tree. New snapshot files are fresh owner-private files. Caller data
and every historical docs/evidence artifact remain. Best-effort diagnostic failure
must not replace the original boot error or bypass teardown. Move blocking disk
copy off the async executor while preserving await-before-directory-removal.

Tests establish genuine failures for source file/directory/root symlinks,
hardlinks, destination ancestry and excessive files/bytes, plus old ordinary and
retry positives. Add FIFO/nonblocking and descriptor path-replacement probes.
Current installed native VM plant source aliases/flood where supported; prove
real Host snapshot excludes canary and retains bounded tail, then exact owned
cleanup. Rebuild affected archives, run actual counts/checks and retain failures.

Use already pinned workspace cap-std/cap-fs-ext rather than new unsafe Unix/NT
wrappers. No new third-party package/version, workspace, config language, wire
contract or execution state owner. Platform compile proof is separate from native
support. No VMM/firmware edits, real ~/.kbox, unrelated cleanup, publication or
GA/default-route/platform promotion. External disk/Git conditions stay parked.

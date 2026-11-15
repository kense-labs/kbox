# Reuse the existing Guest read-only root helper

Trace all init, exec and PTY callers before editing. Establish actual before
failure at the callable setup boundary. Existing library exec helper already
implements idempotent recursive bind/remount; prefer one fallible shared owner
with post-mount readonly verification over two divergent implementations.
Propagate its error at all callers before spawning any child. Preserve writable
inner mount semantics, cleanup ownership, output/error separation and existing
resource fences. Use typed OS errors, no swallowed stat/mount errors.

Constitution I/II/V/X/XV failclosed/oneauthority/typedeffects/no weakenedgates;
III/IV retained before/finalnativeproof; VI/VII no workspace/config/deps;
XII/XIV no native source edits/support claims. No waiver.

Next inspect prepare exec helper/tests, before LinuxGuestbinary path and public
native harness. Then minimal implementation, targeted/owning/defaultchecks,
rebuild/freeze/install assets and real readonly/init/exec/PTY negatives/positives,
unchanged A–F/OCI/anonymous behavior and owned audit. Record unrelated best-effort
masked/readonly path guards for the next W4 gap; do not hide them as fixed.

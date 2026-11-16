# Spec037: published pool shutdown ownership

An actual published WarmPool drain discards VmManagers when finalization refuses
and returns success. The CLI drops leased entries before close and acknowledges
Stop before cleanup. Retain actual managers/leases under existing owners until
successful finalization, surface failures and allow exact-owner retry. No backend
substitution, duplicate execution state machine/config/dependency. Keep idle stats
truthful; an unclosed manager is not an authenticated idle VM. Cancellation of a
teardown waiter must not discard its actual owner. Stop must not report success
while managed resources/in-flight operations remain; failure keeps the owning
registry/socket available for retry. Preserve all036 private template guarantees.

First establish an actual isolated unit red. Complete shared bulk shutdown,
callers, tests, default/Clippy/fmt and rebuilt installed Mac native close/retry/
pool/caller preservation plus scoped audit. Linux/other-host/fresh-host/GA gates
remain open. Startup/unpublished-pool, foreground boot/legacy error-path ownership
are continuing tasks, not silently closed by a published-shutdown fix.

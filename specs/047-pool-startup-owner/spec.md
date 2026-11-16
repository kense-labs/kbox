# Spec047: retain the actual CLI pool before first-ready initialization

CLI currently awaits WarmPool startup before inserting its actual Arc in the
existing PoolRegistry. Cancelling initialization can lose its only actual pool
collection. Prepare synchronously, insert that exact owner before async fill,
and initialize through a borrowed owner under the existing key/maintenance locks.
Cancelled safe prelaunch initialization can resume the same entry; unclosed/
closed owners stay for truthful drain/retry. No second registry/state machine/
config/dependency/workspace; preserve identity/routes/generation/native boundaries.

Keep existing async constructor APIs compatible. This bounded production-caller
fix does NOT make arbitrary library constructor cancellation or dropping an entire
registry safe; those remain explicit follow-up. Source actually initial-fills
with a unit return, not a typed startup error; do not misdescribe old behavior.
Acceptance: real CLI registry red(before global permit/newVM), cancellation/retry/
same-owner/cleanup controls, shared Runtime prepared initialization checks, native
prepared actualVM/cancel/data/realdrain, rebuilt installed assets/AF/pool/audit/
counts/currentevidence/authorizedcommit/bundle. FullW4/defaultOCIcutover/GA open.

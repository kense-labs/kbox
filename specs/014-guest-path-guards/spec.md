# Fail closed Guest masked and readonly path enforcement

Status: accepted bounded slice; W4 priority1, following accepted013. Shared Guest path guard
is void and logs errors; exec/PTY can launch with requested restrictions absent.
Its st_dev difference heuristic can mistake an unrelated mount for a mask.
Scope: existing CRI K_SEC path intent, shared guard, its two actual callers,
small real tests and native Guest/install asset acceptance. No separate init path
consumer was found; do not invent one or claim it exists. Preserve absent-path
semantics only for actual NotFound; propagate other I/O/mount errors, reject
unsafe paths instead of skipping, verify effects and safe idempotence. Pins or
explicit refusal must prevent symlink/parent escapes; arbitrary mounted targets
cannot count as a valid mask. No new dependencies/state/config/native source,
no route/support/default change; authority and generation remain Box-owned.

Accept: real before exec/PTY permission failure and mounted-target mask bypass;
then refusal before command/fork (framed PTY Error, no marker), invalid/absent
path/error distinctions, real successful directory/file masking and RO writes,
idempotence, caller/sibling retention and containment. Rebuild affected Box
archive, fresh install, source-bound Linux tests on actual HVF Guest, impacted
BoxVm/A–F checks, default verify and scoped owned resource audit. OCI source is
unchanged unless later evidence demands otherwise; retain original provenance.

Acceptance evidence docs/evidence/runtime/20261115-guest-path-guards/OUTCOME.md.
Continued task remains active after bounded closure. OCI tmpfs is queued.
External fresh-host/platform/Windows dependency/GA gates unchanged.

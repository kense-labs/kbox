# Pass existing additions to the existing compiler

Entry: Box preflight -> oci_hvf_spec::validate -> workload_capabilities; bundle
completion calls the same compiler. The current non-SYS_ADMIN blanket refusal
blocks existing common permissions. Filter only the existing special SYS_ADMIN
name out of added/dropped vectors and pass other additions to the shared
oci_portable_spec::compile_workload_capabilities. Its13-name closed parser,
default10, unknown rejection and drop ALL precedence remain authoritative.
Root/default ambient/inheritable stay empty; resolved nonroot additions follow
the explicit subset rule below. Keep the existing SYS_ADMIN branch unchanged.

Affected: vm/oci_hvf_spec.rs, sibling production tests and native test module/
probe; existing preflight/complete callers, CLI, per-UID limits and executor
require no parallel implementation. Validate those callers through actual
installed process paths. Reuse existing manager/Host/stream/PTY/CLI/fencing and
owned cleanup helpers. Capture synthetic caller-copy UID/GID/mode/inode/hash
before grants; keep ephemeral-root and generation-local transport semantics.

Constitution Check: one ExecutionManager resource authority and exact generation;
typed existing config/allowlist only; explicit caller additions, default/drop
semantics preserved; Host PEP unaffected; actual native permission effects and
negative controls; no dependency/workspace/config/state/native source addition;
evidence and final installed artifacts before acceptance. No waiver.

Scope review found a required nonroot completion fix: use the actual resolved
OCI UID, not a guessed config username. Select only explicit normalized names
from the already validated permitted set, and use those as inheritable/ambient
for nonroot. Defaults and dropped names are never carried. Reuse the existing
Agent credential/ambient executor unchanged; no new parser or capability name.
Add completion red/positive tests plus actual UID1001 permission and specific
drop controls and explicit CLI --user capture. Root/default/all-drop/per-UID
budget behavior stays unchanged. Regenerate affected Box/client/default gates.

Final caller correction: completion and oci_session::launch both call the same
apply_workload_capabilities before rlimits. Native2 proved the omitted exec
caller; native3 proves actual nonroot permissions across all five entry paths.
Final evidence and retained driver failures are in the spec029 OUTCOME.

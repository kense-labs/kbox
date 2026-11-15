# Reuse the existing policy-to-tmpfs caller

Entry: CLI common.shm_size → request.policy.shm_size → immutable managed metadata
and BoxRecord → VmLocalExecutionBackend::new_manager_from_metadata appends
/dev/shm:size=N → oci_hvf_spec::validate/complete → existing Guest MountPlan.
Agent DefaultFilesystemPlan supplies omitted /dev/shm and never replaces an exact
configured destination. Portable Box OCI spec has only /proc initially, so a
single explicit /dev/shm mount overrides that synthesized default without a
duplicate mount. No Guest/Host/SDK ABI extension is required.

Mac preflight validates a clone with the same generated size entry, so invalid
sizes and conflicting explicit tmpfs reject before image planning/claims. Other
profiles keep the policy guard. oci_hvf_tmpfs::mounts admits only exact /dev/shm
with one size option, uses the existing bounded size parser and MountBuilder,
and fixes rw,nosuid,nodev,noexec,mode1777. All other reserved-target checks remain.
Use the existing builder unchanged; no alternate state or config is introduced.

Constitution Check: fail closed before claims; existing typed policy/config and
one manager authority; exact binding/fencing; no external content/secrets/native
edits/new dependency; real Guest effects and source-preserving cleanup; no test,
support or release gate weakened. Existing /dev/shm bare refusal stays tested.

Scope: oci_production.rs guard/preflight, oci_hvf_tmpfs.rs branch, sibling tests,
native harness registration/sibling/probe and evidence. Callers/CLI/record/Agent
need verification, not duplicate implementations. Validate behavior before
freezing source/client and building final assets; changed source means affected
rebuild/retest. Inspect actual counts and result schemas before asserting them.

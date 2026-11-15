# Add only the missing mappings in the existing process path

Callers are unchanged: CLI ResourceLimits -> stored Box request -> Mac
preflight oci_hvf_rlimits::limits -> OCI init completion and session
apply_workload_rlimits -> SDK/independent HVF Host -> Agent RlimitPlan.
The Linux Agent already maps DATA/STACK/MEMLOCK and applies/readbacks before
credential reduction for init and exec. No firmware change is required.

First fail two focused tests on current Mac admission/completion. Add three
standard enum mappings in the shared Mac helper. Test malformed values,
unchanged previous limits/defaults and non-Mac refusal. Use the existing native
limits harness and a static Linux probe with real allocation, recursion and
mlock effects. Drop all capabilities for the limit and positive controls.
Use distinct probe inodes for simultaneous executions, as required by the
existing caller-file identity boundary; do not weaken that boundary.

Owning Runtime/CLI checks and scoped Clippy/fmt/verify, then frozen-source Box
build/isolated install. Reuse OCI019 only with exact unchanged-input proof and
its original03a62c3e… provenance; fresh extraction/companion/signature/Agent
checks remain required. Final native matrix, CPU/prior-limit and A–F regressions, exact cleanup and
evidence precede bounded acceptance. Source analysis confirms no OCI/Agent/
storage/recovery source changes: keep accepted020 Host PEP/rootRO/tmpfs/Secret
evidence with its exact unchanged OCI identities, without relabelling it as a
new run. The memory matrix uses the existing readonly-root/tmpfs/file path.

# Plan032

Extract the existing cold post-boot admission block without changing behavior,
then add a sibling test establishing that its early exec error leaves the handler
and private resources. Fix the single common cold branch by routing every error
through existing destroy_manager_or_reap, preserving original error and existing
restored-path behavior. Reuse Frame heartbeat, VmManager teardown, VolumeStore,
and existing native archive / A-F harnesses. No product-level injection flag.

Test missing and vanished channels, authenticated success and prewarm refusal;
verify caller isolation and original error. Preserve cleanup failures as failures.
Freeze corrected source, rebuild/install affected Box archive, bind reused OCI
assets by exact prior provenance, execute real native failure + pool/A-F controls,
audit all owned resources and record outcome before continuing next GA gap.

Constitution: one owner/workspace/KCL; actual PEP/native evidence, typed failures,
source-bound archives, sibling tests and no weakened deadlines/claims. No waiver.

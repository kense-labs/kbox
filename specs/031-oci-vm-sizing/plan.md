# Plan031: bind existing VM resources through the actual worker

## Reused components and contract

- Reuse existing VmConfig and shared Core hardware_vm validation rather than
  adding another resource model. Move the pure validated type to SDK and retain
  koci-kbox's public re-export/constructor semantics, including Windows x64's
  one-vCPU guard. Preserve native smoke minima and legacy1/512 defaults.
- SDK parses the bounded, closed version1 annotation and binds its exact value
  with the existing required RuntimeExtensionAttachment/config digest. Existing
  manifests without the annotation retain their previous bytes/meaning.
- Box validates requested hardware before preparation, retains record/request
  equality, projects the annotation for custom values and requires the Host
  capability through existing preflight. Non-Mac qualification remains unchanged.
- HVF advertises the extension only on the implemented path, reads the immutable
  bundle's typed value and passes it through the existing AgentVmConnectOptions,
  shim CLI, AgentVmHandoff and private macOS worker handoff to set_vm_config.
  Other native paths retain fixed agent-VM defaults. New arguments are paired,
  validated before entry, and absent for legacy Host-to-Shim requests.
- Preserve the worker report schema; typed paired arguments carry the exact
  configuration to the native call and exact shutdown report validation, retaining
  the admitted value in AgentVmSession through finish. Reports remain diagnostic, and native Guest
  hardware readback is the acceptance authority. Reuse current signed image/native
  bundle and existing generation/recovery machinery; no Agent ABI/schema bump.

## Tests and final artifacts

First fix red unit tests for the missing contract/Box refusal. Cover malformed
annotation and manifest substitution, unknown/old capability, worker argument
pairing/bounds, argument/record intent drift and immutable update behavior.
Update prior fixed-profile assumptions only where the newly implemented contract
actually supersedes them; retain unsupported and identity-negative assertions.

Run focused owning tests, complete affected package checks/Clippy/fmt and the full
verify-commit gate. Keep genuine native-disk prerequisite failures unchanged.
Freeze source/lock/native identities, rebuild affected Box/OCI archives, install
fresh prefixes, and run current installed assets with exact test/result counts.
Probe real Guest CPU and boot RAM, compare legacy/changed values and verify CLI,
old Host/Shim refusal, restart/stopped recovery/fencing, Host RO and owned cleanup.
Do not use reserved/configured memory as RSS; measure separately if ever claimed.

## Constitution Check

One resource authority and workspace; typed fail-closed admission; one KCL product
configuration; namespaced annotations are versioned runtime data; no extra native
or third-party dependency. Legacy identities/routing/recovery stay authoritative.
All failed checks/assets are retained under docs/evidence/runtime/20261116-oci-vm-sizing.
Native claims require the real macOS arm64 host and Guest effects. External gates
and unsupported platforms stay explicit. No non-negotiable principle is waived.

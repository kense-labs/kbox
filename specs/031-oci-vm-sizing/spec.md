# Spec031: explicit HVF VM CPU and memory sizing

## Scope and current failure

Continue the existing W4 custom VM sizing gap after the optional RAM-capability
fix, local main e01bd356. Current Box OCI qualification intentionally rejects
anything outside1 vCPU/512MiB. `agent_vm_smoke` constructs that same fixed
configuration and the isolated macOS worker independently uses fallback_config.
This is an explicit missing feature, not evidence of an already admitted silent
downgrade. Complete this chain before lifting the Box guard.

Owning paths: Box runtime projection/admission; public koci-sdk attachment
contract; koci-runtime HVF driver/session launcher; koci-kbox parent/private
worker and existing native context. No native submodule source changes.

## User stories and acceptance

1. Existing Box CPU/memory requests configure the dedicated VM's actual hardware.
   Reuse the shared hardware-vm profile:1/2/4/8 vCPUs;128..8192MiB aligned to128.
   Admit only the qualified macOS arm64 path here. Ordinary defaults remain
   unchanged; other platform qualification guards remain fixed.
   Native proof reads Guest online/present CPUs and actual boot RAM mapping,
   accounting explicitly for any documented reserved segments. Configuration,
   logs, cgroup values and top-level reports alone do not prove VM hardware.
2. The request's immutable SDK attachment contract binds hardware intent. Add
   required `dev.kbox.vm-resources` version1, annotation JSON
   `{"schemaVersion":1,"vcpus":2,"memoryMib":768}`. Reject wrong versions,
   types, ranges, unknown/duplicate fields, oversized values and an omitted or
   mismatched required extension before admission. No new configuration language.
3. Absence retains the original1/512 semantics. Box omits the annotation for
   that legacy pair; explicit annotations require support even if they name the
   legacy pair. Old Hosts reject a required extension; old Shims reject new
   explicit sizing arguments before VM/workload entry. No argument retry without
   sizing, backend/isolation fallback or rewritten identity is permitted.
4. CPU/memory boot intent, persisted record counts, immutable OCI config and
   execution/generation remain exact. Initial requests, client reconstruction,
   restart/stopped recovery and stale-generation refusal retain that intent.
   Existing cgroup resource updates remain separate from immutable boot sizing;
   this epic does not add live VM hotplug or memory resizing.
5. Native tests cover multiple CPU/memory values, minima/maxima, CLI and ordinary
   SDK/ExecutionManager paths, concurrent independent executions, old-asset
   refusal, unchanged legacy/default behavior, restart/stopped recovery, stale
   generations and source-preserving scoped cleanup. Inspect actual test counts
   and result files. Unsupported combinations explicitly refuse; refusal is not
   reported as hardware sizing implemented.

## Invariants and non-goals

ExecutionManager remains the only Box resource authority. Persisted BoxVm/OciSdk
identity/routing, generation fencing, libkrun default backend, Host RO, Exec
replay, Guest capability/rlimit policy and caller storage semantics are retained.
No workspace, dependency, state machine, installer, workload special case or
VMM/firmware port is added. Guest Agent/attachment manifest ABI need not change:
VM hardware is configured by the host worker before Guest boot.

No default cutover, new platform support, Windows ARM64 support, live-session
continuity, fresh-host/conformance/soak/GA claim. Existing DiskManagement/RAM
positive tests, Linux OpenSSL download and Git UID/DNS prerequisites remain
external unresolved gates; continue independent work without repeated retries.

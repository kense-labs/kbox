# Primary bootstrap control and OCI regression

2026-11-11, macOS 26.6.2 arm64 with HVF and Linux/aarch64 guests. The optional
primary bootstrap travels on the owner-verified control socket. The engine
receives the complete bounded frame before launch, forwards it through the
bound execution, and admits the independent data socket. Durable records
retain the request and generation identity. The payload lives in the control
and guest process path. [KRun native evidence](../../../../../krun/docs/evidence/2026-11-11-credentials-native/OUTCOME.md)

The hermetic engine suite recorded 183 passes and two ignored native cases.
The focused prewarm suite recorded 11 passes and one ignored native case.
It covers frame bounds, incomplete bootstrap with no guest launch, exact
bootstrap and raw stream ordering, journal exclusion, and refusal when an
installer omits the required guest protocol. The first fixture incorrectly
expected an empty guest environment. The corrected oracle verifies ordinary
environment delivery and the absence of the credential marker in the launch
request. [Checks](checks/credentials-final-kbox-checks.json) · [First run](checks/credentials-kbox-engine-first.log)

The first three-package run found a stopped spare after the CLI reported
`down`. Inspection now projects `CLEANUP_PENDING` until the driver confirms
that the instance owns no unclaimed spare. The regression leaves a separate
owner's reservation in place. The unchanged native zero-resource check then
passed with all three ZIPs. [Failing unit test](checks/credentials-down-cleanup-before.log) · [Passing engine tests](checks/credentials-final-kbox-engine.log) · [Three-package evidence](../../../../../krun/docs/evidence/2026-11-11-credentials-harnesses-native/OUTCOME.md)

Workspace all-target checks, scoped clippy and release build passed. The
workspace fmt check still reports the same 50 existing paths. [Format comparison](checks/credentials-final-kbox-fmt-comparison.json) · [Release log](checks/credentials-final-kbox-release.log)

The final KBox binary and signed libkrun passed the independent native OCI
regression in 57.78 seconds in a prefix with only KBox and its runtime assets.
The test covers image pull, COPY/RUN build, run and exec, child processes,
external files, named volumes, HTTP, live export, commit and run, snapshots,
runtime identity across engine restart and empty final execution inventory.
[OCI result](../20261111-credential-bootstrap-oci/4f4b0e56ea4d4986b874bb76532d5c5e/result.json) · [Invocation](checks/oci-export-final-oci-retry-command.json) · [Binary hashes](checks/oci-export-prefix.json)

The expanded test exposed a live-archive regression in the prior libkrun
Unix backpressure change. The fix drains the write queue before local read
shutdown can trigger a macOS hangup. One intermediate retry exported and
committed successfully, then the fixture omitted `--` before the committed
image's command. The final invocation used the existing CLI separator and
verified the committed rootfs contents. [First export failure](../20261111-credential-bootstrap-oci/c49aa0b6caf9481a8509a3b7fbb3a7c3/008.stderr) · [Intermediate CLI fixture](../20261111-credential-bootstrap-oci/72842754d7e34b0b93777571a7aaec69/010.stderr) · [Library evidence](../../../../libkrun/docs/evidence/20261111-unix-shutdown-tail/OUTCOME.md)

## Explicitly not claimed

Native scope is macOS HVF. Further gates cover native auth refresh and
credential ownership across generations, live provider tasks, KVM/WHPX,
release archives and repeated performance sampling.

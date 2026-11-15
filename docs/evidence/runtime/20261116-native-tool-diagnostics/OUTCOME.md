# Native tool diagnostics and Git convergence checkpoint

The bounded error-reporting fix is implemented and verified. The final installed
Box archive completes five native A–F scenarios. Overall acceptance remains
**incomplete**: the full gate has six native-disk failures, the SDK has one,
and the independent OCI Host cannot start while this session cannot use
DiskManagement. The failed single-file invocation is not a passed native test.

## Change and checked scope

The shared SDK memory-backing inspector now retains the program, exit status
and bounded OS stderr (first4096 bytes, at most1024 characters). Command
arguments and stdout are omitted. `FailedPrecondition` and the existing
`verify-macos-memory-backing` operation remain unchanged. Both callers still
execute the original hdiutil/diskutil checks and fail closed.

Legacy APFS create/attach errors now include hdiutil's exit status, including
when stderr is empty. No resource lifecycle, authorization, backend default,
wire format, manifest, dependency, generation or recovery policy changes here.

## Commands, counts and retained failures

| Actual command / scenario | Result |
| --- | --- |
| `CARGO_NET_OFFLINE=true cargo test --locked -p koci-sdk --lib macos_memory::tests::failed_native_tool -- --nocapture` before fix | 0 passed /2 failed; [red receipt](checks/red1.json) |
| Same focused command after fix | 2 passed /0 failed /0 ignored; [receipt](checks/focused-green1.json) |
| `CARGO_NET_OFFLINE=true cargo test --locked -p koci-sdk --lib` | 229 passed /1 failed /0 ignored; actual diskutil prerequisite failure, [receipt](checks/sdk-all1.json), [output](checks/sdk-all1.stdout) |
| `CARGO_NET_OFFLINE=true bash scripts/verify-commit.sh` | Workspace check/fmt pass; Runtime2271 passed /6 failed /46 ignored; exit101, [receipt](checks/verify1.json), [output](checks/verify1.log) |
| `CARGO_NET_OFFLINE=true cargo clippy --locked -p koci-sdk -p kbox-runtime --all-targets --no-deps` | Pass, [receipt](checks/sdk-clippy1.json) |
| Installed Box OCI lifecycle, host directory RO with restart, primary process/stream lifecycle | 3 Python native scenarios pass |
| Installed Box prewarm and directory grants | 2 Rust native tests, each1 passed /0 failed /0 ignored |
| Installed independent OCI single-file test | Started1, completed0: Host startup fails before workload admission; test process aborts on a second shutdown assertion, exit-6, no result file |

All five completed native rows, their exact installed commands, checked counters
and result paths are retained in [native-results.json](checks/af-final1/native-results.json).
The RO driver checks14 actual Host denials before and after restart, child denial,
guest remount/fresh writable mount and a writable control. Runtime loader maps
prove the archive-installed shim and native libraries were used.

The single-file Host stderr now identifies the actual boundary:
`/usr/sbin/diskutil returned exit status: 1`, followed by the OS explanation
that DiskManagement is unavailable. [Host stderr](checks/files-attempt-1-host-1.stderr),
[test receipt](checks/box-native-files-attempt-1.json),
[unmodified abort output](checks/box-native-files-attempt-1.stderr).
The six full-gate failures remain the two RAM Secret and four legacy APFS/rootfs
cases; no assertions, test selection or release gates were weakened.
[Prior old-binary control and independent OS probes](../20261116-openssl-advisories/OUTCOME.md)
reproduce this host prerequisite failure. The supported guest-native ext4 Box
path can still run, as the five successes demonstrate.

## Final artifacts and provenance

Build base `9d279a1cd0a11d1fb035e5cd845a15f5d70a2967`; frozen compiled inputs
SHA256 `fce46075b91dabe6f8a7f0a9437d2e5c4cc4b1ec10184e72e3b49cbbe9ce39d6`
over1709 files. Final audit compares every compiled input against this snapshot.
Cargo.lock SHA256 is
`3cec984fb77851fcf5e6bbd3838cad56b71c2249cb29e5ec01ec19c36918adf8`.
Both packages are newly built from these inputs; neither substitutes the older
integration archive. Historical packages and failed results remain retained.

All paths below are under
`artifacts/native-tool-diagnostics-a0dcc02256ac4fd2a15a1dc22c8e5c03/`.

| Artifact | SHA256 | Compressed / installed bytes |
| --- | --- | --- |
| `final-dist-diagnostics-1/kbox-3.4.3-macos-arm64.tar.gz` | `d3ddf08ab914668b01fb0da79ecfc7e3b5009a46dc93192a165bef3e79ead36a` | 23,398,692 /44,175,360 |
| `oci-merged-final1-dist/koci-hvf-0.3.6-macos-arm64.tar.gz` | `8cb0c6bf2a2fb898417a5c0148b633eeb54058bf837944c0dfbc3aaae7d53da2` | 33,426,888 /114,954,240 |

Fresh prefixes are `installed-diagnostics-final-1` and
`oci-merged-final1-unpacked/koci-hvf-0.3.6-macos-arm64`. Eleven companions per
package, executable/library architectures, code signatures, package source
metadata and embedded/standalone Agent equality are checked.
[Box install](checks/install-diagnostics-final1.json),
[OCI install and native dependency identities](checks/oci-merged-final1-install.json),
[full file/hash/signature inventory](checks/final-identities-1.json).
The frozen native test client SHA256 is
`ee668a98491db72810b2a1e8b672ca5c3191ccbfeda1bd0637c6ea99186baa6d`.

Native dependency gitlinks remain libkrun
`3c2889c76460d81e3fb16d6144580fd31eb1bae3` and libkrunfw
`fc3ea06895301f1df6a274dad75fbd9317b3bfee`; submodule content was not edited.
The OCI native bundle SHA256 remains
`d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`.
The exact macOS/toolchain environment is in
[environment-closeout.json](checks/environment-closeout.json). Runtime RSS was
not measured; installed bytes and configured VM memory are not RSS.

## Cleanup and handoff

The audit derives all six homes from actual result/failed-command paths,
including prewarm's `k-home`. Actual installed CLI inventories show zero
executions and volumes; no owned workload disks, VM/shim/engine/Host processes,
mounts or open descriptors remain. Eighteen observed runtime socket directories
and73 socket paths are absent. All635 pre-existing immutable cache files retain
their hash/ownership/mode; the AF image catalog has the expected build-cache
metadata change. The audit deletes nothing.
[Scoped cleanup](checks/final-cleanup.json),
[actual home inventories](checks/final-home-inventories.json).

This audit does not resolve the earlier strict preservation failure: seven
historical RAM mounts disappeared under an unknown actor. That failure and the
pending user clarification remain in the
[readiness phase](../20261116-warm-pool-readiness/OUTCOME.md).

The latest user instruction requests commit/merge/push convergence before
another W4 feature. Current changes are integrated on local `main`; no optional
VM-sizing stage is opened. Remote synchronization and local commit identities
are recorded separately in [GIT-CLOSEOUT.md](GIT-CLOSEOUT.md).

## Explicitly not claimed

This is not a green full gate, completed final release acceptance or enterprise
GA. Full W4/OCI Host Service default cutover and live-session continuity remain
incomplete; macOS default BoxVm and stopped-only HVF recovery remain unchanged.
The original intermittent parallel warm-boot cause is unresolved. Fresh-host,
full conformance/security review, long soak and other-platform matrices remain
open. Windows ARM64 dependencies remain unsupported and have no native run;
support levels are unchanged.

Linux OpenSSL0.10.80 compilation/native TLS remains blocked by its missing
cached crate and registry DNS. No advisory is dismissed and no platform is
promoted. Restore DiskManagement/session prerequisites, rerun the six failed
Runtime tests, the SDK test and installed single-file scenario, then run the
full gate. Restore Git user lookup/network access and execute the exact Git
handoff commands after fetching the real remote head. No release was published.

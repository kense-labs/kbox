# Existing OpenSSL advisory patch: local implementation, qualification incomplete

Root Cargo.lock now selects openssl0.10.80 and openssl-sys0.9.118, excluding the
affected ranges in the8 retained official GitHub alerts. The
[upstream advisory](https://github.com/rust-openssl/rust-openssl/security/advisories/GHSA-phqj-4mhp-q6mq)
names0.10.80 as the last required patch in this set. This is a lock-resolution
claim, not native Linux crypto certification or remote alert closure.
[Official records](checks/official-github-alerts-before.json),
[exact generated delta/ranges](checks/precise-lock-delta.json).

`cargo update --offline -p openssl --precise0.10.80` updates exactly two package
versions. Cargo also changes tempfile3.27.0's existing getrandom edge from0.3.4
to already-locked0.4.3, within its published >=0.3,<0.5 constraint. Its version
and checksum are unchanged. All573 packages remain; no package is added/removed.
No manifest, feature or alternate dependency source is introduced.
Lock SHA256`3cec984fb77851fcf5e6bbd3838cad56b71c2249cb29e5ec01ec19c36918adf8`.

The current all-target call tree is openssl←native-tls←reqwest/oci-distribution←
kbox-runtime/CLI. macOS uses its existing platform TLS implementation; selecting
this lock does not mean OpenSSL itself was compiled or exercised on this host.
Cached official metadata matches checksum
`a45fa2aa886c42762255da344f0a0d313e254066c46aad76f300c3d3da62d967`,
minimum Rust1.80 and yanked=false; current pinned Rust is1.97.1.

## Checks and actual failures

`CARGO_NET_OFFLINE=true bash scripts/verify-commit.sh` fails, retained unchanged:
Runtime2271 pass/6 fail/46 ignored. The failures are two RAM Secret backing
checks and four legacy APFS/rootfs checks. No test is removed, filtered out of
the full gate or converted to an expected pass. [Full failure](checks/verify1.json),
[actual output](checks/verify1.log).

Focused old/new binaries reproduce the same failures, including the previously
passing immutable readiness client. Direct diskutil exits1 with
“unable to use the DiskManagement framework”; launchctl's system-domain query
exits141. hdiutil info alone and a standalone Rust main/thread query pass, so
those narrow successes do not substitute for actual diskutil/APFS operation.
Native CUA startup also fails (Sky Computer Use service startup request failed).
The current host/session disk-service prerequisite is unavailable; this is not
proven to be an OpenSSL regression. No system daemon, unrelated mount or network
configuration is changed. [Old control](checks/old-ram-control1.json),
[diskutil](checks/diskutil-info.json), [service](checks/disk-arbitration-service.json),
[Rust probe](checks/rust-hdiutil-probe.stdout).

Independent applicable checks pass: CLI935; the complete kbox-contract command;
scoped Runtime/CLI/contract Clippy; installer8. They do not turn the failed full
gate green. [CLI](checks/cli1.json), [contract](checks/contract1.json),
[Clippy](checks/clippy1.json), [installer](checks/installer1.json).

Linux ARM64 library check stops before compilation because openssl0.10.80's crate
is not cached. Both official index/static endpoints fail DNS; no repeated
network retry or fake/stub crypto artifact is used. [Linux failure](checks/linux-check1.json),
[index failure](checks/openssl080-info.stderr),
[static download failure](checks/openssl080-download-command.json).

## Rebuilt installed artifact and bounded native proof

Compiled source SHA256
`f3392138916733048e58f9130260ce90bb76e7e1049d9bbca9a2544c9996db91`,1709 inputs,
current build commit9d279a1c. A concurrent local session committed the existing
lock/source and added KStudio workflow files; all its work is preserved.

Archive:
`artifacts/openssl-advisories-9f921204fdec43cb87f3f63669a793e2/final-dist-openssl-1/kbox-3.4.3-macos-arm64.tar.gz`.
SHA256`3b68798deb01a883f44acb1fb383cdab25de086ad8be97b4a409da688243856c`;
23,398,364 compressed/44,175,360 installed bytes. Fresh isolated prefix:
`installed-openssl-final-1`. All11 companions, architecture/signatures/source
and unchanged native libraries are checked. This remains an unqualified local
candidate while the full default/native-target gates above fail or are blocked.
[Install](checks/install-openssl-final1.json),
[identities](checks/final-identities-1.json).

The actual installed CLI starts a real HVF VM from the cached digest-pinned OCI
image, receives a Guest exec heartbeat and executes one nonce command. It uses
the guest-native ext4 layer path; this does not prove legacy APFS or RAM Secret
availability. [Actual commands/result](checks/native-installed1-commands.json).
Owned home cleanup reports zero executions/volumes, owned processes/mounts/FDs,
and removal of the observed runtime socket directory. The audit removes nothing.
[Scoped cleanup](checks/native-cleanup1.json).

## Explicitly not claimed

Full verify/enterprise GA is not achieved. Linux OpenSSL compilation/native TLS,
Windows runtime, remote advisory closure, macOS cold APFS/RAM Secret operation,
fresh-host/full conformance/long soak and full W4/default OCI Host Service cutover
remain unqualified. Windows ARM64 dependencies remain unsupported without a
native run; support levels stay fixed. Runtime RSS is unmeasured.

The original intermittent parallel warm-boot cause is still unresolved. The
readiness phase's historical7 RAM mounts are absent from both current hdiutil
and kernel mount observations; the removal actor remains unknown and the strict
preservation failure is retained, pending the already asked user clarification.

Next local work: preserve the actual OS-tool program/exit status and bounded
stderr at the existing memory-backing/rootfs error boundary, with failing then
passing hermetic tests. After environment recovery, rerun the6 actual failed
tests and the full default gate; fetch the official crate and run the Linux
checks/native matrices. No VMM/firmware port, reset, real ~/.kbox write, remote
advisory dismissal or release publication was performed.

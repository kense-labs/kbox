# Spec013 Guest rootfs read-only enforcement

Status: accepted bounded fix on this existing macOS arm64 host. The continuous
W4 task remains active; full OCI Host Service/default cutover and GA are incomplete.

## Behavior and retained failures

One fallible Guest helper now owns init, exec and PTY rootfs read-only setup.
Invalid/missing/non-directory roots, stat failures, bind/remount failures and a
writable post-check refuse before launch. Already read-only roots are idempotent.
Init propagates the error; exec returns failure output; PTY sends its existing
Error frame before openpty/fork. The previous init path explicitly continued
writable and the exec/PTY helper silently returned on errors. No new dependency,
config, state machine or native source change was introduced.

Actual before NativeGuest1 and2 ran0/1/0 (34.00s/33.93s), proving real Linux
command preparation and PTY continuation after mount EPERM, not missing fixtures.
Their failed logs/binaries remain. After3/4 passed during development. Linux
Clippy1 failed because aarch64-linux-musl-gcc was absent; existing cargo-zigbuild
CC/CXX/AR wrappers supplied the compiler for successful2/3, with no installation.
Installed readonly scenario1 failed only on its diagnostic's nonexistent backend
field; runtime_route is the stored contract. Its record was removed, sources
retained; scenario2 asserts the actual route and all effects. Nothing was skipped
or weakened. All candidate/failure evidence remains.

## Source and final assets

HEAD405c8b89f151e9e2879438d9e457dd60bfd18b90 remains dirty and unstaged. Source
input SHA256 `87966c0e77c51d463f8a1e31c7847002f3ef38a63ce1993257ad85472a948a4b` covers1664inputs; only8inputs changed from accepted012,
in Guest Init and its Box native harness. [Source review](checks/final-source-review.json)
and [context](run-context.json) retain exact commands/paths/hashes. Old global and
accepted012 archives/checksum sidecars were saved before packaging; originals remain.

| Archive | SHA256 | Compressed bytes | Installed disk bytes |
| --- | --- | ---: | ---: |
| Box3.4.3 final1 | `ee2c103c56675350b28db18c6c0f7627a417095bbd1d281ac1ab63d824e0a9d5` | 23362328 | 44109824 |
| OCI0.3.6 accepted012, fresh extraction | `df4b882c40f607b56e098b9f971552ad879f8a156fdc35703cc79fd8faf71a2a` | 33345582 | 114847744 |

Box was rebuilt from frozen013source and installed in a new prefix. OCI production
inputs did not change; its immutable012archive retains original source
`dada28865aa4e06e0e3462d046f179eaea58d6cad73195af424efbdf1dba1046`, freshly
extracted under this stage. It is not relabeled as a013build. Both prefixes passed
11file hashes, arm64 CLI/shims/native libs, static aarch64 Guest/Agent, signatures,
dylib links and ext4 embedded-Agent equality. [Final identities](checks/final-identities-1.json)
contains full identities and successful command exits; real worker loader maps
prove only those installed dependencies were used. Guest Init SHA256
`a852930e651da8a380d77bb5860f2deb3048e835610bd14cbae323cfb6ab003a`.

BoxVm retains its independent baseline native libraries. OCI native bundle stays
spec009 `d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`,
libkrun7dcaf940…/firmware841bc9d5…/kernelb1180b50… with original provenance.
Parent gitlinks/working trees unchanged. No native submodule edit.

## Commands and actual counts

Existing macOS26.6.2(25G83), M5Max arm64, euid501, Rust1.97.1. All homes/caches
are isolated; no real ~/.kbox. Exact argv, stdout/stderr, exits and durations are
in checks and retained scripts under `artifacts/rootfs-ro-a4e38db72f6b41dc9fe5be63ef11dc48`.

- `cargo test --locked -p kbox-guest-init --all-targets`:156passed on macOS.
  Linux-only tests are excluded here; this is not their runtime proof.
- Guest all-target Clippy passed on macOS and Linux-musl with existing Zig wrappers;
  fmt passed. Cross `cargo zigbuild --locked --profile release-dist --target
  aarch64-unknown-linux-musl -p kbox-guest-init --tests --message-format=json`
  built frozen final test binaries, not counted as native pass.
- `bash scripts/verify-commit.sh`: exit0,250.51s; Box2222passed/25ignored,
  CLI933passed, contract suites, workspace all-target check, fmt, scoped Clippy,
  support matrix and installer8passed. The extra ignored Box native test is
  explicitly executed below, not substituted for by this baseline.
- FinalGuest5: actual1/0/0,39.29s using frozen test client SHA256
  `122943917ba032e10b61177e85eb52b74b3224f5a1d75b295cd960358198cf6e`, new
  installed Box companions and freshly extracted OCI assets. Real HVF Linux
  Guest reports3/0/4 plus two child1/0/0, then explicitly invoked alreadyRO,
  positive enforcement and init refusal each1/0/0. Exec and PTY fail before
  launch under missing mount authority; init refuses after real UID change;
  no workload marker. Actual root writes return EROFS, inner tmpfs remains
  writable, repeated calls leave mountinfo unchanged. [Outer receipt](checks/box-native-guest-attempt-5.json)
  and Guest logs under the context privateRoot retain all counts.
- Installed normal BoxVm readonly scenario2:1completed scenario,3.15s; init,
  captured exec and real PTY all run with root writes rejected as read-only;
  caller bind and explicit tmpfs remain writable. Restart preserves ID/BoxVm
  route, generation1→2, clears tmpfs and retains caller data. Caller canary,
  directory UID/GID/mode survive removal. [Receipt](checks/rootro-2-result.json).
- A–F final1: all5scenarios passed in141.66s: OCI lifecycle/build/snapshot/engine,
  Host directoryRO attacks and restart, primary streams/engine recovery, prewarm,
  directory grants/identity/generation. Prewarm and directory each actual1/0/0;
  other drivers each check1complete scenario result. [Counts and results](checks/af-final1/native-results.json).

## Owned cleanup and continuation

[Audit](checks/final-cleanup.json): 12homes, no managed executions/volumes,
owned processes/mounts/workload disks/temporary builders; 82observed socket
paths absent. No owned descriptor holder remains ([descriptor audit](checks/final-descriptors.json)).
Seven prior RAM mounts and unrelated disk unchanged. Source blob files15main and
91AF remain unchanged. Audit deleted nothing. Retained failure/native inputs and
caller canaries remain outside runtime resource directories.

Next bounded local task: fix confirmed masked/readonly path setup errors that
still warn/continue in Guest exec/PTY/init, then continue queued OCI tmpfs intent.
This stage does not claim those adjacent guards are fixed.

## Explicitly not claimed

No full W4/cutover/enterprise GA, live HVF session continuity, fresh-host result,
platform support promotion, registry/readiness change, arbitrary rollback,
masked/readonly-path completion, or OCI tmpfs/Secret parity. Windows ARM64
remains current-dependency unsupported and lacks native runtime validation.
No runtime RSS measured. No commit, push or publication. Two previously missing
historical evidence files remain missing; no evidence fabricated.

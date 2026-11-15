# Tasks: final A–F archive acceptance

- [x] G1 Read constraints/A–F, inventory host/resources and preserve old dist.
- [x] G2 Build current archive; verify provenance, architecture/signatures/load paths.
- [x] G3 Install exact archive in fresh prefix; freeze client and private image cache.
- [x] G4 Run OCI lifecycle, RO attack/restart, streams, prewarm and directory/fencing.
- [x] G5 Run final `scripts/verify-commit.sh`; regenerate/requalify after source fixes.
- [x] G6 Inventory owned executions/processes/mounts and finalize size/source/evidence.
- [x] G7 Update release references, review scoped changes and final acceptance limits.

## Final continuation state

Final archive acceptance **PASS**, cleanup **PASS**. Full verify exit 0,
231.88 s: Runtime 2165/0/12 ignored, CLI 933/0, all default contracts, compile,
format, Clippy, matrix and installer 7/7. All five native scenarios PASS; the two
Rust native filters each execute exactly one passing test with checked result.
Nineteen loader samples cover all five scenarios and use installed native libs.
Seven private homes/prefixes have zero executions/volumes; no owned VM/shim/engine
process/mount/disk image remains, 18 sampled Box identities have no nonempty
socket dir, and seven earlier foreign RAM devices are unchanged. Archive and
installed companion hashes remain unchanged. No product source change this round.

Frozen package and SHA256, source/dependency identities, exact commands, counts,
failed extraction diagnostic and cleanup:
`docs/evidence/runtime/20261114-final-archive/OUTCOME.md` and `checks/`.
Compressed 22496416 bytes; installed du 42528768 bytes; runtime memory unmeasured.
Release docs and harness references updated; root native repositories/gitlinks
and product readiness unchanged. Nothing staged, committed, pushed or published.

No acceptance item remains blocked. Next bounded development task: qualify the
native OCI Host Service path and retained OciSdk/BoxVm identity/recovery/fencing
before default route cutover. Full cutover is still incomplete; Windows ARM64
current dependencies remain unsupported and no Windows native run is claimed.

## Investigation history (earlier next steps are superseded above)

Current run root:
`artifacts/final-archive-cf1a062d97174e699fd16a7cdf6a981c`.
Evidence: `docs/evidence/runtime/20261114-final-archive/`.
Old dist copied to `previous-dist`, including staging tree. Archive checksum
8a405ba0ddd4d2653b2e3caa85918830949034dd143d4d869ba78e8908fb8005 verified.
Initial git status, environment, process/mount and seven foreign RAM fixtures
retained in checks. Root native repositories clean and gitlinks unchanged.
Next: build, inspect actual firmware/native source and loader provenance, install
into a new prefix; run every native case with archive-installed VM companions.
No Windows research/port or host wait is in scope.

Current archive build PASS (128.37 s), with K_BUILD_LIBKRUN=1 and ambient native
overrides removed. SHA256:
1289af08712159ef5dc70c8240b72388af32e1699589c3f798388738fbdceb4c.
Installed into the new run root's `installed`; all archive hashes match. Host
arm64, static Linux/aarch64 guest, strict signatures, Hypervisor entitlement and
relative rpaths pass. Actual libkrunfw is locally compiled from checksum-pinned
v5.3.0 kernel.c bundle; root firmware gitlink is not represented as its input.
Source input digest: 2e6124f1fd17c4b11991893c5a899477f5d681c4e19d9fe2f94a2296cecc9839.
`checks/native-provenance.json` records exact paths, tar/kernel hashes and the
single native build matching installed library bytes.

Retained non-product failure: Python extraction diagnostic used unavailable
tarfile filter API; fixed with member/link-path validation and supported API,
then the actual installer succeeded. No product source fix or binary change.
Fresh image cache copied from our prior owned fixture cache. Frozen same-source
Runtime client has only system dynamic dependencies. Native runner executes
outside the developer cwd with prefix/system PATH and no user/native loader
override; actual lsof samples require installed native libraries and shim hashes.
OCI and RO/restart scenarios PASS; primary/prewarm/directory queued/running in
one sequential runner (session 58439). Actual counters/results checked by
`qualification-runner.py`, exact commands/results in checks/native-results.json.
Next: collect all five results, run full verify, inventory and finalize docs.

All five installed-archive native scenarios PASS: OCI 34.94 s, RO 4.35 s,
primary 66.29 s, prewarm 15.15 s, directory 16.79 s. Prewarm/directory each ran
exactly 1 pass / 0 fail / 0 ignored and emitted the checked PASS artifact. Every
scenario has actual lsof proof of loaded installed native libraries; shim bytes
match the archive. No native/test failure occurred. Full default verify is now
running (session 38750; verification-entry.log/json), with an explicit private
verification K_HOME. Next: retain its result,
inventory all owned homes, process/mount identities and finalize docs/review.

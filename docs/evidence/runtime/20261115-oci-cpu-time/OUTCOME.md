# CPU-time ulimits: accepted installed assets

Bounded spec020 is accepted on this existing macOS arm64 host. Box now admits
`cpu=SOFT:HARD` through the existing Core parser, standard OCI process.rlimits
and Agent setrlimit/getrlimit executor. Positive seconds and the existing -1
infinity value retain their values; zero refuses before preparation instead of
using Linux's implicit one-second interpretation. Existing nofile/fsize/core,
omitted request bytes/defaults, stored routes, identities, replay and generation
fencing remain intact. No ABI, schema, dependency, state machine or native source
change. This is per-process CPU time, including inherited child limits; it is
separate from wall deadlines and an execution's aggregate cgroup CPU controls.

Final source is `ccc6c32ff1d7823cbb6f80aa5e03e799a0f839f170c6844372f5e7341bde77f7`
with1693 recorded inputs. Only five Box runtime files differ from accepted019;
all OCI source inputs remain identical. The final fixture correction changes
only a Rust native test relative to candidate1. [Frozen1 delta](checks/source-freeze1.json),
[final2 delta](checks/source-freeze2.json), [final receipt review](checks/final-review2.json).

| Final installed archive | SHA-256 | Compressed bytes | Installed bytes |
| --- | --- | ---: | ---: |
| Box2 kbox-3.4.3-macos-arm64.tar.gz | 57d38d06f5d637b4e2127b1e8967bab37e3a94c83922426e9f616e2e8413d15e | 23,390,475 | 44,154,880 |
| Unchanged OCI019 koci-hvf-0.3.6-macos-arm64.tar.gz | 542bfe43dd6f2837ad55c3ba79416832584e9032a9b6e1b2bfa4d75bc94203bf | 33,417,962 | 114,925,568 |

Box2 uses the final source above. The OCI archive is freshly extracted in this
run and retains its original019 source provenance
`03a62c3e7d246fa4f8eb1e758d338aa3fc9bb25a75ee6f6ce94259c736986b35`;
it is not relabelled as a new OCI build. All11 companion hashes in each install
pass. Actual CLI/shim/native library architecture, signature and loader checks
pass; Linux aarch64 CLI companions and the embedded/standalone Agent match.
Box and OCI native library identities remain distinct. Native repositories
remain clean at libkrun3c2889c7… and libkrunfwfc3ea068…; Cargo.lock stays
0f82d0ea… . [Box build/install](checks/install-cpu-time-final2.json),
[OCI installation/provenance](checks/reused-oci-install1.json),
[exact per-file/Agent/image/dependency identities and commands](checks/final-identities-2.json),
[environment](checks/environment-final2.json), [artifact paths](run-context.json).
Prior archives, candidate1, all failed receipts and their native outputs remain.

Focused before testing executes0passed/2failed/0ignored at the actual admission
and completion boundaries. After the shared mapping fix,27pass/0fail/4ignored,
including unchanged defaults, previous grants/Secret/tmpfs/rootRO checks,
malformed/duplicate/overflow/inverted/zero refusals before allocation and other
platform refusals. Final verify-commit2 exits0 in237.00s: Runtime2240passed/
35ignored, CLI934passed and installer8passed; workspace all-target compile,
formatting and scoped Clippy pass with retained warnings.
[Before](checks/cpu-before1.json), [focused after](checks/cpu-after1.json),
[verify command](checks/verify2.json), [actual counts](checks/verify-counts2.json).

The final CPU native entry executes1passed/0failed/0ignored in330.69s. It checks
init, captured exec, child, stdin stream and real PTY in generations1/2/3:15
actual measured observations. Default SIGXCPU24 arrives at0.995171–1.015951 CPU
seconds; catching SIGXCPU still ends with SIGKILL9 at1.998736–2.014952 CPU seconds.
Direct exec exits152/137 with start markers. Hard-limit raise returns EPERM and
readback stays1:2. Idle2.2 wall seconds consumes0.000121–0.000539 CPU seconds and
survives; separate omitted/infinity control executions each consume at least
2.5 CPU seconds. They use distinct owned probe inodes while the main execution
retains1:2. CLI/restart, exact stored route/identity, replay's one side effect,
changed-request refusal, stale fencing, normal restart and stopped Host recovery
pass. [Final CPU command/count](checks/box-native-cpu-time-attempt-2.json),
[measurements](checks/cpu-measurements-final2.json).

Final affected native entries each execute1/0/0: existing nofile/fsize/core,
rootRO, tmpfs, complete Host Service mixed BoxVm/OciSdk routing/foreign ownership/
caller files/actual Host directory RO/replay/recovery, and managed Secret ENV.
The ENV case additionally executes8/0/0 actual Linux Guest units, all process
paths/CLI/three generations,16 Host-denied writes and2 fresh RW remounts per
wave, protected source replacement, no plaintext metadata and old Agent v9
refusal. Its exact RAM unmount and detach both exit0.
[Affected process chain](checks/process-regressions2.json),
[service chain](checks/service-regressions2.json),
[Host Service result/count](checks/box-native-host-service-attempt-2.json),
[ENV result and RAM cleanup](checks/environment-positive-2/result.json).

Final A–F all five scenarios pass in175.70s through the final installed Box2:
OCI lifecycle/build/export/commit, Host readonly attacks+restart, process flow,
prewarm and directory grants. Prewarm/directory each execute1/0/0; the other
three each complete one native Python scenario. Totals for final acceptance are
**8 outer Rust native tests +8 actual Linux Guest units +3 Python native scenarios**.
Zero-test and candidate1 results are excluded from these totals.
[A–F results/counts](checks/af-final3/native-results.json), [failures/corrections](FAILURES.md).

The final owned audit checks20 existing/prepared isolated homes and208 observed
socket paths, including failed/prepared candidates and both AF runs. There are
zero executions, volumes, owned VM/Host/shim/engine processes, remaining workload
or Host runtime disks, mounts, sockets or FD holders. Reclaimed RAM cases keep
logical cleanup/ownership receipts; all7 initial RAM mounts remain unchanged.
The audit deletes nothing. All23 native-cache and255 AF baseline files are
checked: original blobs/immutable metadata stay unchanged; index changes only
update last_used/owned paths plus two exact owned AF image builds, each bound
to its own candidate CLI/result digest. [Cleanup](checks/final-cleanup.json),
[cache](checks/final-cache-audit.json), [FDs](checks/final-fd-holders.json),
[runtime disks](checks/final-runtime-disks.json).

## Explicitly not claimed

Full OCI Host Service/default cutover, full W4, live-session continuity,
fresh-host/conformance/security/soak and platform/readiness promotion remain
open. Default macOS remains BoxVm and HVF owner death remains stopped-only.
Windows ARM64 current dependencies are unsupported and have no native run.
Cross-builds are preparation, not another platform's native evidence. No VMM/
firmware port, real Secret, real ~/.kbox use, unrelated cleanup, commit, push
or publication. Runtime RSS is not measured. CPU time is not wall time or an
aggregate execution limit. Other unqualified process controls still refuse;
known historical missing evidence is not reconstructed.

Next bounded local W4 work is data/stack/memlock process limits through the same
existing executor. Continue directly; acceptance of020 does not end this task.

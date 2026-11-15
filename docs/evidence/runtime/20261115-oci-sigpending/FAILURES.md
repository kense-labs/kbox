# Failures retained

Admission/completion red1:0passed/2failed; CLI red1:0passed/1failed. Both
failures exercise the missing SIGPENDING mapping/default identity, not a build
or zero-test issue. Logs and actual result counters are under checks. Shared
helper/mapping and CLI typed match fix these without changing public root
defaults or prior nproc policy. After1:14passed/0failed/1ignored, CLI1pass.

Exploratory source reads guessed non-existent sibling paths; rg --files and
actual #[path] declarations resolved them. An unexecuted fixture edit initially
inserted sigpending into a fixed-size memory-limit test array; it was corrected
before the red test build, preserving every memory test assertion. No failed
native run at this checkpoint. Probe binaries1/2/3 retained; only probe3 is
selected for final native execution. Static builds are not native evidence.

Native1:0passed/1failed/0ignored in48.04s wrapper. Actual5path probes all passed
16queued/drained,EAGAIN,hardEPERM,sameUIDchild-sharing and mapped userNS-at-full
EAGAIN. Both parallel full-budget valid exec probes passed with0queued/EAGAIN.
The canary check then incorrectly unwrapped FileDownload(ENOENT), expecting a
false success field. That API returns typed Unavailable for this actual missing
file. Do not accept a generic error as absence proof: use an allowed exact-UID
Guest shell to assert both ! -e and ! -L and require exit0. Original absence
assertion remains; no kernel/quota/permission check weakened. Native1 cleanup
boxes[]; owned holder is stopped with its VM by the existing failure cleanup.
Only the native fixture changes; rebuild Box metadata/client/source and rerun
main2 before affected regressions. Verify1exit0/203.15s retained as actual1, not
final2. All native1 logs/candidate1 remain.

Native2:0passed/1failed/0ignored in186.22s wrapper. All15process observations,
fullbudget guards/parallel requests, replay, normal restart/stopped recovery
and unlimited control pass. zero_control incorrectly uses the reconstructed
LegacyOnly client's new-create policy after recovery, so its new control record
is explicitly BoxVm and refuses a single-file RO bind before VM start. This is
a fixture caller error, not silent backend fallback. Existing main bindings
remain OciSdk. Pass the already-existing MicrovmViaOci positive-control client
to zero_control and assert the new control's persisted OciSdk route. Keep exact
zero/EAGAIN/ordinary-kill/hardEPERM assertions; do not alter production routing.
Native2 failure cleanup boxes[]; all failed records and logs retained.
Currentinstalled2 regressions nproc,files,memory,oldlimits,Secret+Guest8,A–F
havepassed; CPU/Hoststillrunning at this checkpoint. Verify2exit0/226.02s.
Only the ignored Sigpending fixture changes for3, not product payloads. Rebuild
Box3/client/source; prove payload equivalence and retain the actual2 regression
identities without unnecessary reruns, then main3+verify3/audit/docs.

Final main3 passes1/0/0,224.71s(native)/228.91s(wrapper), includingzeroCLIhealth/
realTTY/attach. Finalverify3passes198.75s; strict audit14homes/130sockets0owned
resources and final-review3 PASS. An exploratory count tried read_text on the
modern boxes.json directory; corrected by actual CLI inventory in the audit.
No failed result or candidate is overwritten.

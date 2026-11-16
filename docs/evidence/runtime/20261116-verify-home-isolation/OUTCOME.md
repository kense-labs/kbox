# Outcome035: isolate verification from the user installation

Verification boundary corrected on this macOS arm64 host. The full GA goal
remains active. This phase changes AGENTS.md and the verification script/test;
no compiled Runtime source or packaged payload changed.

## Correction to prior evidence

The invocation environment was confirmed K_HOME unset. dirs_home() falls back to
the real user installation; default tests include direct create/remove operations
on image-hash pool template directories. Prior default verification wrappers
inherited the environment without isolating it. Therefore earlier blanket
statements that real ~/.kbox was untouched are **not established for those default
gate commands**. Native archive qualifications with explicit isolated K_HOME,
loader identities and scoped cleanup remain valid. No existing user directory
was cleaned, restored or modified as part of this correction/audit. We cannot
certify historical user-home nonmodification from the old logs.

Prior logs and outcomes remain; append-only notes in033/034 point here. No attempt
to delete presumed historical test leftovers from real ~/.kbox was made.

## Implementation and actual regression

verify-commit.sh now allocates a fresh0700 private K_HOME before checking tools,
unconditionally replacing inherited/unset K_HOME within its child process. It
prints/retains the exact directory for audit. It also allocates a short0700 native
temporary directory, binds its owner marker to the exact private verification
root, records the reciprocal path and exports TMPDIR within that child. Existing
commands and failure statuses remain; no skip/bypass flag. AGENTS now requires a
fresh task-owned K_HOME for targeted tests too. HOME/CODEX_HOME were not changed.

The script runs the new isolation self-check. Controlled subprocess tools exercise
its environment/exit boundary and preserve a caller canary; each case also binds
an actual Host AF_UNIX socket in the provided private short temporary directory.
These are four shell/Host-filesystem checks, not four Runtime or VM tests.

Original isolation-red1:3fail. Initial correction passes3 cases. Actual real
verify1 then reports2300pass/8fail/51ignored: two existing exec tests fail with
SUN_LEN under the inherited long TMPDIR. Those exact failure logs remain. The
short-temp red adds stronger checks (4fail), then green4/0 passes inherited/unset
homes, tool exit42 and a long Unicode/space temporary base with real socket bind.
All original assertions remain; Runtime tests/thresholds were not changed.

## Actual real gate and owned effects

Final verify2 executes the actual repository script with real Cargo/Python/tools,
not the controlled tools used by its unit self-check. It reports:

- Isolation4pass/0fail; workspace all-target compile, support matrix and fmt pass.
- Runtime2302pass/6fail/51ignored, script exit101. Both SUN_LEN tests now pass.
- The same2 RAM/Secret and4 APFS/provider disk-service failures remain. No full
  green-gate claim. CLI/contract/Clippy/installer commands after Runtime were not
  reached by this invocation; unchanged034 evidence remains935/24/Clippy/8, and
 033 separately retains full contract118. These are historical, not new runs.

Recorded final K_HOME `/private/tmp/kbox-verify-iso-634baeeab45e/real-gate-temporary/kbox-verify.AAmimw/home`;
short TMPDIR `/private/tmp/kv.DIFGdE`. Modes/owner markers/cross-binding pass.
The inherited task-owned caller home retains exactly its canary; the parent
process environment remains unchanged. The long-temp failing run and final run
are retained together. Actual scoped audit finds no owned VM/shim/engine process,
mount/disk image, FD holder or socket; 146 forensic filesystem
entries remain, with no audit deletion. No real user-home content was scanned or
cleaned to support a nonmodification claim. See final-audit.json, owned-file-
inventory.json, fd-*.json and verify1/verify2 receipts/raw logs.

## Version and unchanged archive

Runtime compiled inputs `629e2bac05b4102ad569ae699a45814de438cf873dfa0d70a0c98867acf30a42` match all1720 frozen034
inputs byte-for-byte. Current archive remains `/Users/roylin/code/kense/kbox/artifacts/boot-stop-preservation-3f488eca50004416875e318fc753940a/final-dist-bootstop-1/kbox-3.4.3-macos-arm64.tar.gz`,
SHA256 `4093b9d508588359e25012206deb5767053c41c0fdde3960705d70fd1a796b2e`,23411039 compressed/44191744 installed bytes.
All11 installed manifest files were rechecked. The verifier/test are outside the
archive; an unaffected payload was not rebuilt. No VM native scenario was newly
executed; retained034 qualification is3 Rust native +3 Python scenarios, including
one synthetic stop error around a real VM. No actual OS stop-error claim.
New workflow input hashes are separately recorded in workflow-inputs2.json;
base local main is c634d76e800d2ae24af38dabc9788c6dd0751032.

Actual commands and exit codes:

```sh
python3 scripts/test-verify-isolation.py
bash scripts/verify-commit.sh
```

The real gate runner supplies an isolated inherited caller K_HOME and a long owned
TMPDIR, then records the script's independently allocated private roots; see
verify2.json. Evidence includes all reds and the first actual8-failure run.

## Explicitly not claimed / next work

Full release/GA, historical ~/.kbox nonmodification, Linux/Windows native runtime,
Windows ARM64 support, arbitrary overlong AF_UNIX address support, RSS or a new
Runtime native pass are not claimed. Windows ARM64 dependencies remain unsupported
and have no native run; platform levels unchanged. Full OCI Host Service/default
cutover/live continuity, original15s boot cause, six disk-service gates/prior SDK
failure, historical RAM loss and external fresh-host/conformance/security/soak
remain open. Git UID501/GitHub DNS remote synchronization remains blocked/unproved.

Next required local work remains pool/template ownership: cleanup can delete an
unowned canonical directory, builders use a shared image-hash directory, and
outer pool/legacy error paths can discard a retained manager. Existing tests that
expect deletion are part of that contract audit; no deletion assertion was
silently weakened here. Continue that work after this prerequisite, keeping the
same resource authority and owned-resource-only boundary.

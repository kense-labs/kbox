# Spec035: private verification home

Confirmed invocation environment has no K_HOME; verify-commit.sh and ad-hoc gate
wrappers inherit it. Default tests use dirs_home() and directly create/remove
image-hash template fixtures. That does not support prior statements that the
real ~/.kbox was untouched. Preserve those logs and append a clear correction;
no deletion, restoration or cleanup of real user directories.

The actual verification script must allocate a fresh0700 private root/home before
any checking tool, unconditionally override inherited/unset K_HOME only within
its own subprocess, print exact ownership path and retain it on success/failure
for audit. Allocate a separate short0700 owned temporary root for native Unix
sockets and bind the two roots with ownership markers; keep all existing commands/assertions/checks and exit statuses. Do not
repurpose HOME/CODEX_HOME, use another workspace/config or add dependencies.
Add small standard-library subprocess regressions with isolated fake tools to
verify environment confinement/failure propagation; these are shell-harness
checks, not Runtime/native test passes. Run the real script on this actual host,
inspect actual counts and prove its known fixture artifacts are in the owned
home. Preserve failure logs. No process or user data cleanup outside owned roots.

Runtime source is unchanged from034; prove compiled-input equality and retain
its exact installed/archive identity/native evidence rather than rebuild an
unaffected payload. New script/test digests are separate workflow inputs. Correct
release/W4 evidence limitations and previous033/034 outcomes by append-only note.
No GA/default route/platform promotion; full task and template/outer pool defects
remain required next. Remote Git/disk/platform blockers remain parked.

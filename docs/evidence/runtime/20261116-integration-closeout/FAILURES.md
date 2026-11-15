# Retained integration failures

- The first accepted-work commit emitted the existing missing-lefthook warning
  but succeeded. Required verification is run manually; no hook is disabled.
- Merge conflicts in shim main.rs and vm_launch.rs combine incoming unsafe
  contracts with the accepted Host RO prepare/confine/reverify path. The retired
  duplicate parse_ulimit function is not restored; the shared typed parser stays.
- layout1 used the host default Python3.9, which lacks tomllib. Use the already
  installed /opt/homebrew/bin/python3.13 for that incoming Python3.11+ gate.
- wire1 found17 actual kbox/28 consumer identifiers. Eleven removed business
  contracts were still marked shared by the incoming manifest. Move them to
  consumer-only with an explicit retirement reason; the runtime implementation
  and the fail-closed inventory checker are unchanged. Cross-repo recheck required.
- verify1 stopped before compilation on incoming Markdown hard-break spaces and
  raw Windows evidence formatting. Ordinary docs use equivalent CommonMark
  backslash hard breaks. Captured evidence retains its original bytes through
  narrow historical path attributes, as existing repository evidence does.
  No source assertion or validation is relaxed. Original Git diff failures/logs
  remain under checks; the large initial staging whitespace log remains under
  the pre-merge artifact root. Product tests have not yet run on merged source.

- windows-clippy1 correctly refused incoming CI's x86_64-pc-windows-gnu ABI.
  Accepted spec005 intentionally supports only the current Windows x64 MSVC
  native bundle, including in stub mode, and still refuses ARM64/other ABIs.
  Change CI and its matching documented command to x86_64-pc-windows-msvc; do
  not bypass the native target guard. This remains compile/Clippy evidence only.
- workspace-clippy1 found one missing concrete SAFETY comment at the actual
  readonly-FD pwrite denial test. Add the live-FD/one-byte-buffer argument
  without altering its EBADF assertion or broadening a lint allowlist.

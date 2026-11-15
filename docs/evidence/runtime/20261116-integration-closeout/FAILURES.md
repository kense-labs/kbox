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

- OCIcandidate1 Host/Agent builds and signing passed, then the shared system-image
  helper failed because upstream removed crates/oci-runtime/Cargo.toml. Read the
  actual Agent package manifest/version instead of root workspace version0.1.0.
  Keep the failed log and empty output directory. Compiled Rust/Box assets are
  unaffected; regenerate the affected image/package with the helper SHA recorded
  in build-info, verify Agent version0.3.6 in actual generated metadata, and rerun
  relevant installed native acceptance. No nested workspace is restored.

- A–F1's OCI RUN build failed acquiring a warm-pool VM: the shim stayed alive
  but Guest exec readiness exceeded the unchanged15000ms deadline. Other four
  A–F cases and all three independent OCI matrices passed. The final aggregate
  correctly failed; an earlier progress statement incorrectly assumed OCI had
  passed and was corrected. Original002.stderr/commands and aggregate failures
  are retained. Per-box logs were removed by the existing finally cleanup.
  Reproduce only this affected OCI case in a fresh root, with bounded live
  snapshots of owned console/shim logs; retain the same asset hashes, image and
  boot deadline. Do not rerun passed cases or relax readiness/assertions.

- Audit preparation initially assumed every existing test K_HOME has boxes.json.
  The failed OCI build home exists but never created a managed catalog. Include
  it explicitly among all9 actual test homes; do not omit it based on catalog
  discovery. The attempted audit1 failed before inventory because that planned
  file was not yet written. Audit2 uses receipt-derived homes including this
  failure, with the same zero-resource checks.

- Final resolutions: verify3 passes after the shared image-builder fix; the
  regenerated OCI image metadata names Agent0.3.6. The fresh OCI-only retry
  passes36.04s unchanged, but does not prove the original warm-boot cause fixed.
  Audit2 passes all9 actual homes with99 absent sockets and no owned resources.
  The initial failures above remain part of the result, not overwritten passes.

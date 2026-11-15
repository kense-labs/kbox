# Implementation Plan: Generic MicroVM release baseline

Spec: spec.md. Work begins from the existing uncommitted generic-engine cleanup.

## Constitution check

I/V/VIII: validated resource limits and typed unsupported refusals; no backend fallback.
II/X: retain ExecutionManager effects and Runtime receipt authority.
III/IV/XI/XV: retain failing baselines, add small adversarial regressions, strengthen full default gates, verify real effects and preserve evidence.
VI/VII: reuse the root workspace, installed dependencies and KCL; no new product config.
IX/XIII: synthetic fixtures only; exact runtime and manifest identities, no guessed identifiers.
XII/XIV: no commits/publication/submodule mutation; platform claims require the corresponding native test.
All principles pass for this scope. A missing native host remains an explicit unqualified item.

## Research and reuse

Resource compilation uses host_fs::oci::OciResources and the existing OCI partial-update adapter. Snapshot refusal uses RuntimeError::UnsupportedCapabilities at driver admission. Existing routing, lifecycle, ownership and snapshot recovery tests are updated to the MicroVM-only contract without weakening fencing.
Packaging reuses scripts/package-macos-arm64.sh, the existing download/install split, and a source-owned archive installer. Existing e2e-engine drivers exercise final installed binaries.

## Delivery sequence

A. Reproduce and fix resource/capability defects; classify remaining default-test failures.
B. Repair package inputs and provenance; verify isolated installation and rejection cases.
C. Repair mechanical formatting and strengthen scripts/verify-commit.sh to full default checks.
D. Run final-artifact native checks on eligible available hosts and retain evidence.

## Verification and failure behavior

Run owning focused and default tests, scoped Clippy, workspace all-targets check, full formatting, support matrix and git diff --check. Use fresh private test homes and preserve failed runs. No promotion from mocks, compilation or startup alone.
Evidence records actual commands, exit codes, counts, source/artifact identity, native cleanup inventories and Explicitly not claimed.

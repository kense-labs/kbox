# Retained failures and corrections

- cpu-before1: actual0 passed/2 failed/0 ignored. Existing Mac profile refused
  RLIMIT_CPU in admission and process completion. Fixed once in the shared Mac
  mapping; cpu-after1 actual27 passed/0 failed/4 ignored.
- af-native1: exit1 before any native scenario/test was started. The isolated
  AF parent directory was not created before work.mkdir. No product or asset
  defect and no runtime resource was created. Keep this failed receipt. The
  fresh runner explicitly validates an artifact-scoped absolute path and creates
  its new root before work; retry uses af-final2 and separate checks. No assertion
  is removed or reduced, and the failed attempt is not counted as native pass.

- cpu-native1: actual0passed/1failed/0ignored after all15 measured CPU process
  observations across three generations and direct signal exits passed. The
  simultaneous unlimited control reused the main execution's active caller-file
  probe inode. That inode intentionally has one exact runtime alias, and the
  existing single-file PEP refuses a new unaliased-file admission. Preserve the
  refusal. Fixed the fixture by copying identical probe bytes to a distinct owned
  inode for each control; no assertion, source identity check or safety boundary
  is reduced. Failed case cleanup-boxes is[], and its Host/workers are reaped.
  This changes a Rust test input, so freeze2, rebuild Box provenance/client and
  replay final installed-asset checks; source1 results remain calibration only.

- host-service-native1: outer runner refused the actual0passed/0failed/0ignored
  result. The newly added top-level test selector accidentally inserted a second
  namespace separator. No Host or execution started; prepared home remains for
  final inventory. Fixed the exact selector and added a --list membership assert
  before starting any native case. The zero-test result remains failed and is
  never counted as acceptance. Retry will use final2 assets and attempt2.

# Retained failures and qualification limits

- ready-red1 executes2 failing regressions and2 existing passes before the shared
  fix. Late heartbeat incorrectly returns Ok; wedged heartbeat exceeds the total
  cap. vm-green1 subsequently passes214 tests, pool-green1 passes83, and verify1
  passes the full default gate. Original red output is retained.
- final-audit1 compares the whole snapshot including Git HEAD and fails after a
  separate local actor commits70df92be. All1709 compiled input bytes and their
  digest remain unchanged. The source/commit binding records both identities;
  no source assertion or artifact provenance is rewritten.
- final-audit2 finds all7 previous-phase RAM mounts absent. final-audit3 retains
  that assertion at the end so every independent owned-resource observation is
  collected:21 homes,106 socket directories,441 paths,443 unchanged image blobs,
  no owned runtime resources. Overall RAM preservation still fails; the unknown
  removal actor is pending user clarification. No detach or restore is attempted.
- `cargo info --locked openssl@0.10.80 --verbose` fails because index.crates.io
  does not resolve. One distinct official static.crates.io download attempt also
  fails DNS. Do not repeatedly retry either endpoint. Cached official sparse
  metadata exists for the fixed version and has checksum
  a45fa2aa886c42762255da344f0a0d313e254066c46aad76f300c3d3da62d967,
  rust_version1.80 and yanked=false; no crate download is claimed.
- The original parallel warm-boot timeout remains unreproduced/unfixed despite
  bounded before/after native waves passing. That original result remains FAIL
  in the integration-closeout record; this deadline fix is a separate proven
  defect found while tracing the same shared readiness loop.

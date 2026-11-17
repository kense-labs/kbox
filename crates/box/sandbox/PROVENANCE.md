# Sandbox engine provenance

Vendored Apache-2.0 sandbox engine sources at commit `2d6c2d6`, fused into kbox
as the sandbox isolation backend. Each crate under this directory retains a
`LICENSE` and third-party `NOTICE` (headers rewritten for kbox sandbox naming).

Excluded from the vendor copy: upstream marketing brand assets, packaging
installer chrome, and the native window UI (stubbed under `engine/src/ui`).

Do not treat this note as a product Pass claim.

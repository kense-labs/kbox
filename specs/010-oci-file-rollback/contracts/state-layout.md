# File-authority state layout

- The canonical logical store path and sibling lock remain unchanged.
- Legacy no-file roots retain the flat JSON array. The record schema, creation
  digest, operation ID, route, target and generation keep their exact meanings.
- A root with caller_files is published as an actual directory containing
  records.v1.json, the single authoritative JSON array. Cleanup does not flatten
  an adopted empty root. This is a store version, not a second lifecycle owner.
- Current normal load adopts old flat file-bearing records using a fresh read
  under the existing exclusive lock. Readonly load never changes the layout.
- macOS prepares synced bytes in an owned private directory, then atomically
  swaps it with an existing flat file. No gap permits an empty root. Unsupported
  publication returns an I/O error without a writable compatibility fallback.
- Unaware readers and old regular-file rename writers fail EISDIR. They must
  not quarantine the new directory or replace it with a flat snapshot.
- Missing/corrupt/future/symlinked directory data refuses, including file loss
  after path resolution. Missing-as-empty applies only to legacy flat roots.
- A managed transition cannot replace or discard caller_files. Adopting the
  layout does not change caller files, their identities or resource authority.
- Persisted-format changes that add security meaning require a new layout
  version understood by readers; ignoring unknown fields cannot be a fence.
- Backups/restoration preserve the complete layout. Manual flattening/arbitrary
  old-binary rollback is unsupported. No power-cut migration test is claimed.

Native claims are limited to the actual macOS arm64 host/filesystem and pinned
assets in the outcome; no Windows ARM64 or cross-platform promotion follows.

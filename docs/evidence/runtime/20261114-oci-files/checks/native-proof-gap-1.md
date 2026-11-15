# Candidate 1 RO proof gap

Candidate probe accepts EROFS, and actual RO failures return errno=30 after a
successful bind remount and new mount. A fresh mount may share the original
readonly virtio-fs superblock. Therefore these RO errors do NOT yet prove Host
PEP denial. RW metadata/parent denials return EPERM. Preserve attempt-1 results
and probe-1 source/binary; require a writable Guest superblock and EPERM/EACCES
for final Host RO acceptance. Product file-only Seatbelt native child proof is
independent and remains valid, but does not replace full installed-VM evidence.

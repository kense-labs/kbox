# Recursive Guest readonly contract

Primary documentation checked during implementation:
- [Linux mount_setattr manual](https://man7.org/linux/man-pages/man2/mount_setattr.2.html): AT_EMPTY_PATH selects the pinned descriptor, AT_RECURSIVE applies to the mount tree, repeated identical attributes are idempotent. The implementation sets only READONLY and clears nothing. Unsupported/permission errors propagate; no weaker remount fallback.
- [Linux kernel mount API](https://docs.kernel.org/filesystems/mount_api.html): filesystem source is reflected in mount information. A dedicated source name helps distinguish this guard's masks from caller tmpfs, but is combined with actual tmpfs/readonly/empty readback. It alone never proves enforcement.

These are Guest mount restrictions, not Host inode-wide RO or protection from explicitly granted Guest SYS_ADMIN. Native tests establish the actual bundled Guest kernel's behavior. No older/fresh platform support is inferred.

# Spec 008: guest-native OCI workload rootfs

Status: implemented and accepted on the recorded existing macOS arm64 host. Owners: kbox-runtime; koci-sdk, agent-protocol, runtime, kbox and Agent.

## Acceptance

1. Explicit macOS arm64 independent Host Service admits foreign numeric UID/GID through a guest-native ext4 workload disk. Real guest stat and non-root positive/negative permission checks establish preservation for files/directories/root, modes and links.
2. Verified OCI layers retain existing overwrite, whiteout, hardlink, symlink, xattr and input-limit semantics. No host foreign chown, sudo, ownership normalization or Box guest-init injection.
3. Versioned capability/manifest binds immutable source hash, disk inode, config, execution ID and generation. Each execution gets a distinct writable disk; source/caches stay unchanged. Corruption/replacement/stale generation/partial publication fail closed and exact-owned cleanup retains other executions.
4. Existing directory transport refusal, persisted BoxVm/OciSdk identities, stopped-only recovery, Exec encoding/replay, cap_drop ALL and host readonly enforcement remain intact.
5. Focused and full owning checks, verify-commit, rebuilt isolated Box/OCI archives, native lifecycle/recovery/isolation and owned cleanup have command/count/artifact evidence.

## Non-goals

No live recovery, default route change, fresh-host/platform promotion, VMM/firmware changes, Windows ARM64 enablement, new state manager/config/workspace, commit or publication. W4 only changes the foreign-rootfs row after real acceptance.

## Adversarial scenarios

Bad layer digest/metadata/path/size; disk source swap/corruption/link alias; old generation/binding; prepare cancellation; independent writable instances; guest SYS_ADMIN fresh mounts; old Host lacking the extension. Existing UID gate remains the old directory transport oracle.

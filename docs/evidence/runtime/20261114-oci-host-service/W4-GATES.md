# W4 gate ledger

This bounded epic qualifies an explicit macOS arm64 integration. Full OCI Host
Service cutover remains incomplete. The authoritative broader requirements are
[ROADMAP W4](../../../../crates/oci-runtime/ROADMAP.md) and
[macOS promotion gates](../../../../crates/oci-runtime/docs/macos-hvf.md).

| Gate | Result / remaining condition |
| --- | --- |
| ExecutionManager → persisted OciSdk → public SDK → independent HVF Host | Qualified on this existing macOS host with fixed assets; no direct BoxVm result substitutes for OciSdk |
| Mixed BoxVm/OciSdk, reconstructed clients and new-create policy changes | Qualified; exact provider binding, route, identity and generation retained; missing/wrong binding and stale generations refuse |
| Host death and recoverability | Authenticated stopped/SIGKILL recovery, explicit session invalidation and new-generation restart qualified; **live-session continuity is not implemented by this HVF profile** |
| Ambiguous committed response, replay, concurrency and disconnect | Qualified with a framed public-SDK byte relay to the actual independent Host and real host side-effect counters; no fake execution service |
| Host directory RO, files, directory grants, resource enforcement | Qualified for plain directory binds, remount/fresh-mount attacks and child process, file/FS APIs, cgroup readback/update; actual host denial repeated after restart |
| Foreign UID/GID workload rootfs | Qualified on this macOS arm64 host with the final installed archives through required rootfs-image v1 / Agent manifest v5: actual numeric ownership, permissions, links/layers, isolated writable disks, restart and stopped recovery. Legacy directory refusal retained. [Spec008 evidence](../20261114-oci-rootfs/OUTCOME.md) |
| Ordinary single-file caller grants | Qualified on this host using exact installed Box6/OCI7 assets: live RO/RW files, Host EPERM/EACCES after remount/fresh mount/child, no caller-parent content access, source identity/restart/stopped recovery, failed-create reconciliation and source-preserving cleanup. Unsafe alias/overlap/cross-filesystem combinations refuse. [Spec009 evidence](../20261114-oci-files/OUTCOME.md) |
| Named VolumeStore mounts | Qualified bounded existing allocation RO/RW binds through installed Box1/OCI1: live Host growth/writeback, shared references, stable allocation identities, CLI, restart/stopped recovery and retained data after detach. One bind per allocation/execution; aliases, bad catalogs, missing binds and unenforced quotas/drivers refuse. [Spec011 evidence](../20261114-oci-named-volumes/OUTCOME.md) |
| Image VOLUME anonymous allocations | Qualified bounded Guest-native ext4 seeds, exact Box allocation identities, isolation, caller/named overrides, normal/stopped recovery, CLI and owned/ephemeral cleanup through installed Box1/OCI1. Required raw v1 / Agent manifest v7; old Host refuses. Host syscall and actual native FD evidence retained. [Spec012 evidence](../20261115-oci-anonymous-volumes/OUTCOME.md) |
| Box tmpfs intent | Qualified bounded Mac targets,RO/RW,size/mode/UID/GID and safety flags through existing OCI Guest executor; actual EROFS/ENOSPC/noexec,independent instances,CLI,restart/stopped recovery and no hidden image allocations. Final archive/source/13-home cleanup in [spec015](../20261115-oci-tmpfs/OUTCOME.md). This is Guest mount control,not Host inode RO or arbitrary Guest SYS_ADMIN revocation |
| Box root read_only intent | Qualified existing Mac Guest root view via required rootfs-image v2; init/exec/stream/realPTY/File/FS EROFS,inner RW,OCI positive control/CLI/normal+stopped recovery and18-home cleanup in [spec016](../20261115-oci-root-readonly/OUTCOME.md). v1 bytes/access retained; old Host safely refuses2. Physical boot disk stays writable; Guest SYS_ADMIN revocation is not claimed |
| Box process ulimits | Qualified bounded Mac nofile/fsize/core,CPU-time and data/stack/memlock,nice,rtprio/rttime and nonroot per-UID nproc/sigpending/msgqueue through existing OCI executor; actual EMFILE/EFBIG/EPERM,init/exec/child/stream/realPTY,CLI/replay/three generations and21-home cleanup. Shared parser refuses malformed legacy input; managed BoxVm CLI sessions now retain existing manager security/fencing. [Spec017 evidence](../20261115-oci-ulimits/OUTCOME.md). CPU-time adds actual SIGXCPU/SIGKILL, independent measured controls, all process paths/CLI/recovery and20-home cleanup in [spec020](../20261115-oci-cpu-time/OUTCOME.md). Data/stack/mlock actual ENOMEM/SIGSEGV/EPERM and10-home cleanup in [spec021](../20261115-oci-memory-limits/OUTCOME.md). Nonroot nproc actual fork/thread EAGAIN, admission-bypass fix, all entries/CLI/realTTY, old-asset refusal and29-home cleanup in [spec022](../20261115-oci-nproc/OUTCOME.md). Sigpending actual realtime queue EAGAIN/order/drain/reuse, same-UID sharing, namespace refusal, CLI/recovery and14-home cleanup in [spec023](../20261115-oci-sigpending/OUTCOME.md). POSIX msgqueue actual EMFILE/FD control, message EAGAIN/order, shared/inherited-FD lifetime, namespace refusal, CLI/recovery and15-home cleanup in [spec024](../20261115-oci-msgqueue/OUTCOME.md). Nice actual -5 success/-6 EACCES, child/thread/userNS, zero/infinity, CLI/recovery and10-home cleanup in [spec025](../20261115-oci-nice/OUTCOME.md). Real-time FIFO/RR priority ceiling and continuous CPU SIGXCPU/SIGKILL, measured/zero/infinity/blocking/normal controls, CLI/recovery and12-home cleanup in [spec026](../20261115-oci-realtime/OUTCOME.md) |
| Managed Secret file transport | Qualified bounded existing Box-owned RAM regular RO files via required memory-secret-files v1 / Agent manifest v8. Actual Host denials, numeric owner/mode, independent material, three generations, old assets/disk-root refusal and exact RAM cleanup through final installed archives. Raw-disk combinations require the explicit bounded v3/v11 contract in027 below; environment v2 is qualified in019 below. [Spec018 evidence](../20261115-oci-secrets/OUTCOME.md) |
| Managed Secret environment delivery | Qualified required memory-secret-files v2 / Agent manifest v9 through protected pre-namespace source pins and the existing init/exec consumer. Actual data stays correct after visible-path replacement; all process paths/CLI, Guest8, compatibility, replay/recovery and owned cleanup pass final installed archives. Files-onlyv1/v8 bytes/meaning retained. [Spec019 evidence](../20261115-oci-secret-environment/OUTCOME.md) |
| Managed Secret/raw anonymous-volume combination | Qualified bounded memory-secret-files v3 / Agent transport v11, including optional protected ENV and existing NPROC v10 envelope. Actual Host RO attacks, raw seeds/identity/writes, old assets/disk projection refusal, three generations, CLI,21 Guest tests plus2 isolated child runs and17-home/8-owned-RAM cleanup. Legacy versions retain semantics. [Spec027 evidence](../20261115-oci-raw-secrets/OUTCOME.md) |
| Custom root/anonymous disk capacity | Qualified bounded existing disk_mb16MiB..64GiB on this Mac profile: actual128MiB ENOSPC/reuse,256MiB CLI,8GiB sparse disk, undersized-image refusal, restart/stopped recovery and13-home cleanup. Root remains ephemeral; anonymous data persists. No new schema/resize/quota. [Spec028 evidence](../20261115-oci-disk-capacity/OUTCOME.md) |
| Existing portable Guest cap_add | Qualified shared13-name allowlist plus previously explicit SYS_ADMIN; default10 unchanged, root ambient empty, nonroot carries only explicit undropped additions through shared init/exec mapping. Actual MKNOD/SETFCAP/mount effects, drop/ALL/per-UID/Host RO controls,45 observations,CLI,three generations and18-home cleanup. [Spec029 evidence](../20261115-oci-cap-add/OUTCOME.md) |
| Shared-memory size | Qualified page-aligned4KiB..512MiB size-only /dev/shm, preserved1777/safety flags, actual POSIX mapping/lifetime,1/2MiB ENOSPC/reuse,sticky/noexec,CLI,three generations and7-home cleanup. [Spec030 pre-merge evidence](../20261115-oci-shm-size/OUTCOME.md) |
| Complete Box feature parity | **Open.** This profile is Linux arm64, one vCPU/512 MiB, network none and bounded directory/ordinary-file grants. Unqualified process limits, custom VM CPU/memory sizing and disk quotas, arbitrary guest-capability additions, pools, snapshots, TEE, devices/custom network and other unsupported controls refuse; only the bounded Secret/raw combination in027 is qualified. Refusal is a boundary, not implemented parity |
| Default Box cutover / no remaining direct VM path | **Open.** macOS default remains BoxVm; explicit migration only. Both persisted routes remain available. Do not flip an enum or remove the retained backend to claim W4 |
| Fresh KVM x86_64 + AArch64 and WHPX native release matrices | **External hardware/attestation gate.** This existing macOS host cannot supply those matrices. No Windows waiting or VMM/firmware port started |
| Fresh-host macOS, full OCI conformance/security review and long soak | **Open.** Existing-host qualification and 25-wave smoke do not replace these release gates |
| Upgrade/rollback | Forward reading of operation v1–v6 retained; new writes use v7. Old Hosts do not understand v7 state; rollback on such a state root is not qualified. Spec010 qualifies atomic adoption of existing caller-file records and an old-reader/stale-flat-writer refusal fence, with source identity/route/generation intact; arbitrary old-binary rollback/manual flattening remains unsupported. [Installed evidence](../20261114-oci-file-rollback/OUTCOME.md). Legacy nonempty-capability Exec receipts retain their original order-sensitive fingerprint, never a weakened fallback |
| Public registry/readiness/default supervised create | Unchanged. No support-matrix promotion, no `promotes_readiness=true`, no B2 single-report self-certification |
| Windows ARM64 | Current dependencies unsupported; no native run, cross-compilation or emulation claim in this epic; retained F evidence remains authoritative |

The foreign-rootfs slice is closed by [spec008](../../../../specs/008-oci-rootfs/tasks.md),
and ordinary file grants by [spec009](../../../../specs/009-oci-file-grants/tasks.md),
including final installed-archive replay and owned cleanup. The next bounded local
W4 shared-memory gap is closed by [spec030](../20261115-oci-shm-size/OUTCOME.md). The user now requests convergence, commit/merge/push rather than another local feature stage; merged-source validation is required separately. Portable capability additions are closed by [spec029](../20261115-oci-cap-add/OUTCOME.md). Custom disk capacity is closed by [spec028](../20261115-oci-disk-capacity/OUTCOME.md). The bounded Secret/raw combination is closed by [spec027](../20261115-oci-raw-secrets/OUTCOME.md). Real-time limits are closed by [spec026](../20261115-oci-realtime/OUTCOME.md). Nice is closed by [spec025](../20261115-oci-nice/OUTCOME.md). Msgqueue is closed by [spec024](../20261115-oci-msgqueue/OUTCOME.md). Sigpending is closed by [spec023](../20261115-oci-sigpending/OUTCOME.md). Nonroot nproc is closed by [spec022](../20261115-oci-nproc/OUTCOME.md).
The Guest masked/readonly-path fail-open is closed by
[spec014 installed evidence](../20261115-guest-path-guards/OUTCOME.md);
rootfs readonly init/exec/PTY enforcement is closed by
[spec013 installed evidence](../20261115-rootfs-readonly/OUTCOME.md); tmpfs intent
is closed by [spec015](../20261115-oci-tmpfs/OUTCOME.md). OCI root read_only intent
is closed by [spec016](../20261115-oci-root-readonly/OUTCOME.md),separate from
Host raw-disk RO and the accepted BoxVm guard fixes.
Anonymous allocations are closed by [spec012](../../../../specs/012-oci-anonymous-volumes/tasks.md). Named mounts are
closed by [spec011](../../../../specs/011-oci-named-volumes/tasks.md), following
the [spec010 compatibility fence](../../../../specs/010-oci-file-rollback/tasks.md);
full feature parity, live continuity, default cutover and external fresh-host
matrices remain open above.

Bounded process limits and the managed CLI raw-socket security bypass are closed
by [spec017 final installed evidence](../20261115-oci-ulimits/OUTCOME.md). Full
W4 and default cutover remain open; unqualified resource types still refuse.

User-authorized Git convergence is validated separately by the
[merged-source installed closeout](../20261116-integration-closeout/OUTCOME.md):
5 Rust native tests plus3 Python scenarios and9-home/99-socket cleanup. The first
parallel OCI warm-boot readiness timeout is retained as a failure; its isolated
retry passes unchanged, without resolving the startup risk. Next technical work
is owned-log reproduction of that timeout. This does not close default cutover,
full parity, long soak or external platform qualification.

The [later diagnostic/Git checkpoint](../20261116-native-tool-diagnostics/OUTCOME.md)
rebuilt both final archives and passed five installed A–F scenarios. Overall
acceptance remains incomplete: six Runtime disk-service failures, one SDK
failure and independent OCI Host startup require DiskManagement recovery.
Current-phase six-home cleanup passes; the earlier unknown-actor RAM
disappearance remains a retained failure. The user's latest request is Git
convergence before another local feature; no W4 or platform gate is promoted.

After that local convergence, the active continuous goal resumes bounded local
gaps while Git prerequisites remain blocked. The
[optional memory-capability fix](../20261116-hvf-optional-memory-probe/OUTCOME.md)
restores ordinary independent Host availability with explicit Secret absence and
real required-extension refusal. Current installed file/mixed-route/recovery
tests and four-home cleanup pass; full default disk-service/RAM-positive gates
stay open. Next local parity gap remains typed custom VM CPU/memory sizing.

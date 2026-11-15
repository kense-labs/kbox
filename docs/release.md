# Release packaging and qualification

`install.sh` downloads a versioned archive and checksum. The archive carries
`scripts/install-package.sh` as its local installer. `dist/` contains generated
archives and staging trees, not installer or release-documentation source.

On macOS arm64, run `scripts/package-macos-arm64.sh` with the pinned Rust
toolchain and installed cargo-zigbuild/Zig. The script builds Linux/aarch64
static guest-init and the host CLI/shim from the current checkout, packages only
the native libraries staged by that build, and signs the shim with the checked-in
Hypervisor entitlement. It does not reuse the user's installed runtime.

Each archive retains `VERSION`, `build-info.json` and `SHA256SUMS`, including
source identity, native source commits and exact companion checksums. The local
installer verifies companions and architecture before changing runtime files.
Failed verification rolls back bin/lib and preserves workload data. Unrecoverable
rollback retains its backup path. Ad-hoc signing is not Developer ID signing or
notarization. SIGKILL/power-loss transactional installation is not claimed.

The current product support matrix remains authoritative for readiness;
packaging and installation alone do not promote a platform. Fresh-host native
qualification, OCI semantic conformance, security review, upgrade/rollback and
longer soak remain release gates. The current A–F archive acceptance is recorded
in [the final archive outcome](evidence/runtime/20261114-final-archive/OUTCOME.md).
It rebuilt the current dirty source, installed the exact archive in a new prefix,
and qualified OCI-image/engine lifecycle, host read-only attacks/restart, process
streams, prewarm and directory/generation behavior using the installed companions.
Actual loader maps, test counts, cleanup, source/dependency identities and separate
package/disk measurements are retained there; runtime memory was not measured.

This local candidate still uses version 3.4.3; its source digest and archive
checksum distinguish it from the preserved A–D archive. It has not been published.
Full OCI Host Service cutover remains incomplete. Windows ARM64 remains unsupported
by the current native dependencies and has no native run in this acceptance.
Existing-host macOS acceptance does not promote any platform's support level.
Historical [A–D](evidence/runtime/20261114-release-baseline/OUTCOME.md),
[E](evidence/runtime/20261114-macos-readonly/OUTCOME.md) and
[F](evidence/runtime/20261114-windows-arm64/OUTCOME.md) records remain unchanged.


The subsequent [Box/independent OCI Host Service acceptance](evidence/runtime/20261114-oci-host-service/OUTCOME.md)
tracks the rebuilt candidate after explicit macOS SDK integration. Its Box
archive and separately frozen OCI Host/shim/Agent/system-image assets have
distinct dependency identities; the OCI worker's pinned native bundle is not
replaced with the Box archive's libraries. Follow that outcome for the current
source/archive hashes, counts, installed-prefix replay and cleanup inventory.
The prior A–F archive remains frozen at its original hash and path above.


Frozen spec007 Box candidate SHA-256:
`1f12f1fb296394f151538dd15073500d0383cd935fb966ada1ac143b993622ad`.
Compressed size 23,293,906 bytes; isolated installed usage 44,027,904 bytes.
Source-input digest:
`c24d5a84c2e955c2789f0be1d8b7946f63f8d9b8ad8743145ff42d82ed3f9243`.
The separately qualified OCI asset archive SHA-256 is
`17ffecf4643a3ec16da398e6db865c87824b70a2a68cc1ca028c2c829682385b`.
Both exact installed archives pass the bounded native matrix and retain full
checksums/source/dependency inventories in the linked outcome. Runtime memory
was not measured. Explicit HVF integration still has the profile and stopped-only
recovery limits in that outcome; full Host Service cutover remains incomplete,
Windows ARM64 remains dependency-unsupported/unverified natively, and support
levels are unchanged.


The current spec008 candidate includes guest-native OCI workload rootfs and the
installed CLI diff-baseline fix. [Final acceptance](evidence/runtime/20261114-oci-rootfs/OUTCOME.md)
records the final commands, real test counts, failures, dependency identities and
cleanup. Source-input SHA256 is
`a54f07ddea510d319125b0deec7c1aeb48c2e94f6c2af70e74f1911b251069c3`.
Box archive SHA256 is
`34e2a8f09f7c029319e8a86495c1e9607dfb83941fe6a0752ab4972aa617f889`
(23,309,401 compressed bytes; 44,044,288 installed bytes). The separate OCI asset
archive SHA256 is
`fb1bc50474bea3802325352e1400d0811ed6177c42597a53a78b252acbcadba5`
(33,124,162 compressed bytes; 113,782,784 installed bytes). Both are unpublished
local artifacts; all earlier archives and failure evidence remain retained.
Runtime memory was not measured. Full OCI Host Service cutover is still incomplete;
Windows ARM64 remains dependency-unsupported and unverified natively. Platform
support levels and default routing are unchanged.


The spec009 single-file candidate is accepted through the exact installed
Box6/OCI7 assets in [its outcome](evidence/runtime/20261114-oci-files/OUTCOME.md).
Source-input SHA256 is
`3d07d7318f277094813eb973e746339c8fce5934bf95bcf1021a9011a7e54ca5`.
Box archive SHA256 is
`a35388f45b1c326d6fd549a30fb3545e30176194e3b8f7bdb088a9016df313bb`
(23,332,309 compressed bytes; 44,060,672 installed bytes). Separate OCI archive
SHA256 is
`ce2b65c4b93b8843b9d4a9b897e5b37e217a78895458ae5fd7f2e58586fd9239`
(33,333,578 compressed bytes; 114,847,744 installed bytes). Native dependency
source/patch provenance, all failed candidates, actual test counts, final A–F
replay and cleanup are retained there. The new OCI native source is uncommitted
in its owning linked worktree, not a landed parent gitlink or published release.
Runtime memory was not measured. Full OCI Host Service cutover, caller-file
rollback and broader W4 remain open; Windows ARM64 remains dependency-unsupported
and unverified natively. Existing-host acceptance does not raise support levels.


The subsequent spec010 candidate fences file-bearing Box state against unaware
older readers and stale flat writers, and fixes concurrent record loss being
mistaken for an empty store. [Final acceptance](evidence/runtime/20261114-oci-file-rollback/OUTCOME.md)
records the reproduced exposure, actual old/new installed CLI tests, complete
native/A–F replay and cleanup. Source-input SHA256 is
`b60caefc24eb123281d699c8d268cdb6ddc9605d25a3487676fee5aa82645689`.
Box archive SHA256 is
`9856cab0edc2bbe4ff2f0f2da1e0537d2e65ee73a4f44e358656e04a44918078`
(23,333,369 compressed bytes; 44,060,672 installed bytes). Its actual installed
OCI7 dependency archive remains
`ce2b65c4b93b8843b9d4a9b897e5b37e217a78895458ae5fd7f2e58586fd9239`,
with unchanged OCI source/assets and the retained owning native provenance.
Backups must preserve the versioned state directory; arbitrary old-binary
rollback/manual flattening remains unsupported. Earlier archives and all failed
candidates are retained. Runtime memory was not measured. Full OCI Host Service
cutover, W4 and enterprise GA remain incomplete; Windows ARM64 remains dependency-
unsupported and unverified natively. Support levels/default routing are unchanged.


Spec011 closes bounded named VolumeStore mounts through explicit OciSdk and the
independent HVF Host, including live caller directory growth, protected allocation
identities, shared references and stopped recovery. [Final installed acceptance](evidence/runtime/20261114-oci-named-volumes/OUTCOME.md)
records actual new CLI/native/A–F counts, all failures and source/data preservation.
Source-input SHA256 is
`07ccdf500f00db25334d538ebed0dd1bcf10dd11e3dfaf417ca45f79cad2ef35`.
Box final1 archive SHA256 is
`d4209a7ff4048690770089126c0026bfc09abe0cc26d5b074992cf8fc83595cc`
(23,344,608 compressed bytes; 44,081,152 installed bytes). OCI final1 archive is
`5f9578d20cf4db524b7fb83223443f83d0aca257688370b854772d191c478288`
(33,334,546 compressed bytes; 114,851,840 installed bytes). Both rebuilt archives
were installed in fresh owned prefixes, and native workers loaded those included
dependencies. Native dependency source/firmware/kernel remain the qualified009
bundle; parent native gitlinks unchanged. Old archives and checksums retained.
No runtime memory measurement. Full OCI Host Service cutover, anonymous/Secret
parity, remaining W4 controls and enterprise GA remain incomplete. Windows ARM64
current dependencies unsupported and no native run; existing-host acceptance
does not raise platform support levels or change default routes.


Spec012 closes bounded image VOLUME anonymous allocations on this existing
macOS arm64 host, retaining Box ownership, Guest metadata and normal/stopped
recovery data through required raw v1 / Agent manifest v7. [Final installed
acceptance](evidence/runtime/20261115-oci-anonymous-volumes/OUTCOME.md) records all
failures, six actual native matrices, CLI/A–F, Host syscall/native FD proof and
23-home zero-resource cleanup. Source-input SHA256 is
`dada28865aa4e06e0e3462d046f179eaea58d6cad73195af424efbdf1dba1046`.
Box final1 archive is `6e2e961d5da356beeadd1875fedace6dfc3c91615f10dd30eb7fe61f2431371e`
(23,362,761 compressed;44,113,920 installed bytes); OCI final1 is
`df4b882c40f607b56e098b9f971552ad879f8a156fdc35703cc79fd8faf71a2a`
(33,345,582 compressed;114,847,744 installed bytes). Both prefixes were freshly
installed from those archives; source, signatures, architecture, embedded Agent
and included native dependencies match the retained identity records. Native
source/FW/kernel selection is unchanged from009; old archives/sidecars remain.

Runtime memory was not measured. Full OCI Host Service/default cutover, Secret
and remaining control parity, full W4 and enterprise GA are incomplete. Windows
ARM64 current dependencies remain unsupported and have no native runtime pass.
This acceptance does not raise platform support levels or publish any artifact.

Spec013 closes the Guest rootfs read-only fail-open in init, exec and PTY using
one shared fallible guard. The new installed Box archive rejects actual root
writes, retains writable inner mounts and preserves restart identity/generation;
real Linux Guest refusal tests and all five affected A–F scenarios passed.
[Final source/assets/counts/owned cleanup](evidence/runtime/20261115-rootfs-readonly/OUTCOME.md)
retain prior archives and explicitly reuse unchanged OCI012assets with their
original provenance. Adjacent masked/readonly-path guards and queued OCI tmpfs
remain local work. Full OCI Host Service/default cutover and enterprise GA are
incomplete; support levels are unchanged and Windows ARM64 remains dependency-
unsupported without native validation. Runtime RSS was not measured.

Spec014 closes the shared Box Guest masked/readonly-path fail-open at both trusted
exec and PTY protocol entry points. Final Box2 actual server hashes, negative
markers, mask/EROFS effects, restart/fencing, real HVF Guest cases, all five A–F
scenarios and owned cleanup passed.
[Source/assets/counts and retained failures](evidence/runtime/20261115-guest-path-guards/OUTCOME.md)
explain the corrected test caller and private source lifecycle. Managed K_SEC_*
filtering/cap_drop ALL remain unchanged. Existing running Guest servers must
restart with updated companions; these Guest mount controls do not replace Host
RO. OCI tmpfs/read_only/Secret parity and full Host Service/default cutover remain
open, as do enterprise GA and external promotion gates. Windows ARM64 remains
dependency-unsupported without native validation; runtime RSS was not measured.

Spec015 closes bounded Box tmpfs intent through the existing OCI Guest executor
on this existing Mac host. Shared parsing,Mac admission,standard OCI mounts and
image VOLUME overrides retain caller/volume identity and generation fencing.
Final Box archive `1f81ad37106ed31275027bd5c135a1b9f72bcfd755d5af252d58e75dac9e1f30`
(source `31aeb63a32b8b388fe5526eddf9f3172eb3c577656fd8e1b6342309a17fbcb16`) is
23,370,310 compressed /44,122,112 installed bytes. Unchanged OCI012df4b882c…
is freshly installed with its original dada2886… provenance.
[Commands,actual counts,effects and retained failures](evidence/runtime/20261115-oci-tmpfs/OUTCOME.md)
include CLI,real tmpfs/EROFS/ENOSPC/noexec/UIDGID,three generations,independent
instances,HostRO/file/named/anonymous/A–F regressions and 13-home zero-resource
cleanup (106 absent sockets;seven original RAM devices preserved).
`verify-commit.sh` passed Box2228/28ignored,CLI933,installer8. No runtime RSS
measurement. Full OCI Host Service/default cutover,read_only/Secret parity,
full W4 and enterprise GA remain incomplete. Windows ARM64 remains dependency-
unsupported without native validation; platform levels stay unchanged.

Spec016 closes bounded OCI Guest root read_only through required rootfs-image
v2 and the existing Agent finalizer. v1 bytes/writable semantics stay unchanged;
old Host refusal remains a boundary. Final installed Box2
`52418165c730223d4284daf4a7c5050a2f9475c128cd5715076460f6d6d57721`
(23,370,870 compressed/44,122,112 installed bytes) and OCI2
`c7db5a96c1acb2621872154a4f946b8c565fe26ece8222453319df22d63e67cc`
(33,355,655 compressed/114,860,032 installed bytes) share source
`3ac0fbd8f65db2518b9586038f1d13ef00bde5b9aefdae60c069ef29455b9c7a`.
[Actual native counts,failures,source/assets and18-home cleanup](evidence/runtime/20261115-oci-root-readonly/OUTCOME.md)
include init/exec/stream/realPTY/File/FS EROFS,inner RW,normal/stopped recovery,
explicit OCI writable control/CLI,old Host rejection,volume/file/HostRO/A–F
regressions and107 absent sockets. Verify passed Box2231/30ignored,CLI933,
installer8; OCI owning SDK217,protocol66,Runtime438/2ignored,Kbox80/4ignored.
No runtime RSS measurement,physical boot-disk RO or SYS_ADMIN revocation claim.
Full OCI Host Service/default cutover,W4/GA and other control parity remain
incomplete; Windows ARM64 dependencies unsupported without native validation.
Platform levels unchanged. No commit,push or publication.

Spec017 closes bounded Mac OCI nofile/fsize/core process intent and a managed
BoxVm CLI session security bypass. Final Box2
`c61f0a5a90567ee9e0a2a2cde627b6b990b647821bb35dd79dc087c6c592d0c5`
(source `8c687c06d486c721afcc33605d83ec208cd9b92f3a72d7732529ebd2d7ddd736`)
is23,374,170 compressed/44,138,496 installed bytes. Unchanged OCI016c7db5a96…
is freshly installed with original3ac0fbd8… provenance.
[Actual commands,counts,failures,source/loader identities and21-home cleanup](evidence/runtime/20261115-oci-ulimits/OUTCOME.md)
include kernel EMFILE/EFBIG/EPERM,init/exec/stream/realPTY/child,three generations,
replay,CLI,real attach and spoof controls,rootRO and all five A–F scenarios.
Verify passed Box2236/31ignored,CLI934,installer8;184 observed sockets absent,
seven pre-existing RAM devices preserved,zero owned resources/FD holders.
Full OCI Host Service/default cutover,W4/GA and other parity remain incomplete.
Windows ARM64 dependencies unsupported without native validation; platform
levels unchanged. Runtime RSS unmeasured. No commit,push or publication.

Spec018 qualifies bounded actual RAM-backed managed Secret files on this existing
macOS arm64 host through final Box3 `9ad22c54…` /OCI3 `3331cbc9…`, source
`e1aa0d07bea7f853f81ff91e7f9cad778035a132c765cfab17bb3a4adca15585`. Compressed
bytes23,384,813 /33,372,500; installed44,154,880 /114,892,800. Required SDK
memory-secret-files v1 /Agent manifest v8 preserves older versions and numeric
source ownership/mode. [Actual commands, failures, identities,14 native Rust
tests +3 Python scenarios and16-home cleanup](evidence/runtime/20261115-oci-secrets/OUTCOME.md)
include actual Host denials, CLI, three generations, old assets/disk-root refusal
and A–F. Verify passed2238/33ignored,CLI934,installer8;134 observed sockets absent,
zero owned resources/FD holders and seven baseline RAM mounts preserved.
Secret environment/raw-disk combinations and full OCI Host Service/default
cutover remain incomplete. Windows ARM64 current dependencies remain unsupported
and not natively run; no platform support level is promoted. Runtime RSS was not
measured. No commit, push or publication. Next local W4 slice: Secret environment.

Spec019 qualifies bounded managed Secret environment on this existing Mac arm64
host using Box2 `b9b77584…` /OCI2 `542bfe43…`, source
`03a62c3e7d246fa4f8eb1e758d338aa3fc9bb25a75ee6f6ce94259c736986b35`. Compressed
23,390,394 /33,417,962 bytes; installed44,154,880 /114,925,568. Required
memory-secret-filesv2 /Agent manifestv9 preserves prior versions. Protected
sources survive visible-path replacement; no material enters private plans.
[Actual15 outer Rust native +8 Guest unit +3 Python scenarios, failures,
identities and19-home cleanup](evidence/runtime/20261115-oci-secret-environment/OUTCOME.md)
include all process paths/CLI, three generations, compatibility, A–F and owned
RAM/FD/process cleanup. Verify passed2238/34ignored,CLI934,installer8;175 sockets
absent,7initialRAMunchanged. Full OCI Host Service/default cutover remain
incomplete; Windows ARM64 current dependencies unsupported/no native run;
platform levels unchanged. Runtime RSS unmeasured. No commit/push/publication.
CPU-time is qualified in spec020 below.

Spec020 qualifies bounded CPU-time ulimits on this existing Mac arm64 host using
Box2 `57d38d06…`, source
`ccc6c32ff1d7823cbb6f80aa5e03e799a0f839f170c6844372f5e7341bde77f7`.
Box is23,390,475 compressed/44,154,880 installed bytes. OCI019 `542bfe43…`
is freshly installed unchanged with original03a62c3e… provenance;33,417,962
compressed/114,925,568 installed bytes. [Actual8 outer Rust native +8 Guest unit
+3 Python scenarios, failures, identities and20-home cleanup](evidence/runtime/20261115-oci-cpu-time/OUTCOME.md)
include SIGXCPU/SIGKILL and CPU-time measurements, all process paths/CLI,
independent controls/replay/recovery, Host RO, rootRO/tmpfs/Secret and A–F.
Verify passed2240/35ignored,CLI934,installer8;208 observed sockets absent,
zero owned resources/FD holders and7baselineRAM preserved. Complete OCI Host
Service/default cutover remains incomplete; Windows ARM64 current dependencies
unsupported/no native run; platform levels unchanged. Runtime RSS unmeasured.
No commit,push or publication. Those limits are qualified by spec021 below.

Spec021 qualifies data/stack/memlock on this existing Mac arm64 host through
Box1 `79c97ef7…`, source
`e02147be72c10c307955ad416a5daa41a3dc5b60cad7c682c405ce0b3dc4ffcc`.
Box is23,390,507 compressed/44,154,880 installed bytes; unchanged OCI019
`542bfe43…` retains03a62c3e… provenance after a fresh isolated installation.
[Actual5 Rust native +3 Python scenarios, failures/optimizer diagnosis, source/
asset identities and10-home cleanup](evidence/runtime/20261115-oci-memory-limits/OUTCOME.md)
include real ENOMEM/SIGSEGV/EPERM,15 process observations,unlimited controls,
CLI/replay/recovery,prior CPU/limits and A–F. Verify2242/36ignored,CLI934,installer8;
92 sockets absent,zero owned resources/FDs,7baselineRAMunchanged. Unchanged020
OCI/Agent/Host/storage checks retain their historical identity and are not
counted again. Full OCI Host Service/default cutover remains incomplete;
Windows ARM64 dependencies unsupported/no native run; support levels unchanged.
Runtime RSS unmeasured. No commit/push/publication. Nonroot nproc is qualified by spec022 below.

Spec022 accepts bounded nonroot nproc on this existing Mac arm64 host, including
the real kernel root-helper admission fix and shared HVF bare-command PATH fix.
Final sourcecc9e1c37… /Box5f190440c… /OCI37009193…(original4f9dbe71…).
Box23,392,791compressed/44,171,264installed bytes; OCI33,421,132/114,950,144.
[Actual12 Rust native +13 Guest +3 Python +1 native VM decoder probe, all asset
identities, failures and29-home cleanup](evidence/runtime/20261115-oci-nproc/OUTCOME.md).
Verify2251/38ignored,CLI935,installer8;210 sockets absent,zero owned resources/FDs,
7baselineRAMpreserved. Three successful limits regressions retain installed4
identities with proven payload equivalence to5. Complete OCI Host Service/default
cutover remains incomplete; Windows ARM64 current dependencies unsupported/no
native run; platform support levels unchanged. RSS unmeasured. No commit/push/
publication. Continue the next existing W4 process-limit gap.

Spec023 accepts nonroot sigpending on this existing Mac arm64 host, through the
existing shared per-UID boundary and OCI executor. Final sourceb0d1c272… /Box3
a6f6b371… /OCI37009193…(original4f9dbe71…). Box23,392,573compressed/44,171,264
installed bytes; OCI33,421,132/114,950,144. [Actual10 Rust native +13 Guest +3
Python, two retained fixture failures, identities and14-home cleanup](evidence/runtime/20261115-oci-sigpending/OUTCOME.md).
Successful2 regressions keep their original identities with strict payload
equivalence to3. Verify2253/39ignored,CLI935,installer8;130 sockets absent,zero
owned resources/FDs,7baselineRAMunchanged. Complete OCI Host Service/default
cutover remains incomplete; Windows ARM64 dependencies unsupported/no native
run; support levels unchanged. RSS unmeasured. No commit/push/publication.
Continue existing W4 msgqueue intent.

Spec024 accepts nonroot POSIX msgqueue limits on this existing Mac arm64 host.
Sourceb91eb406… /Box16b0e49a9… /OCI37009193…(original4f9dbe71…). Box23,392,838
compressed/44,171,264installed bytes; OCI33,421,132/114,950,144.
[Actual11 Rust native +13 Guest +3 Python, retained namespace fixture failure,
identities and15-home cleanup](evidence/runtime/20261115-oci-msgqueue/OUTCOME.md).
Verify2255/40ignored,CLI935,installer8;129 sockets absent,zero owned resources/
FDs,7baselineRAMunchanged. Product assets/source remain frozen after external C
fixture correction. Complete OCI Host Service/default cutover remains incomplete;
Windows ARM64 current dependencies unsupported/no native run; platform support
levels unchanged. RSS unmeasured; no commit/push/publication. Continue W4 nice.

Spec025 qualifies existing nice intent through the standard OCI executor without
changing CLI/root defaults or the per-UID counter boundary. Actual -5 success,
-6 EACCES, inherited child/thread and namespace boundaries, zero/infinity,
all process paths/CLI/health/realTTY/attach/replay/recovery pass. [Final installed
evidence:5 Rust native +3 Python scenarios and10-home cleanup](evidence/runtime/20261115-oci-nice/OUTCOME.md).
Only one production enum mapping and its qualification text change; OCI/Agent/
other resources remain unchanged. Full W4/default Host Service cutover/live
continuity/platform gates remain open; RSS unmeasured, Windows ARM64 dependencies
unsupported/no native run. Continue existing rtprio/rttime intent.

Spec026 qualifies existing rtprio/rttime through the unchanged OCI executor:
FIFO/RR5 allowed/6 EPERM, inherited child/thread and namespace boundaries,
actual SIGXCPU/SIGKILL with measured CPU, blocking reset/normal-policy controls,
zero/infinity, all entries/CLI/health/realTTY/attach/replay/recovery. [Final
installed evidence:5 Rust native +3 Python scenarios and12-home cleanup](evidence/runtime/20261115-oci-realtime/OUTCOME.md).
No CLI/default/UID/schema/native/dependency change; current kernel tick granularity
is explicit. Full W4/default Host Service cutover/live continuity/platform gates
remain open; RSS unmeasured, Windows ARM64 dependencies unsupported/no native run.
Continue the existing managed Secret/raw anonymous-volume combination.


Spec027 qualifies managed RAM Secret files/environment with raw anonymous volumes
through required memory-secret-filesv3 / Agent transportv11, including the existing
NPROC v10 envelope. Old versions retain meaning and refuse incompatible intent;
old Host/shim/Agent and disk projections actually refuse. A real cross-volume
sparse rootfs publication bug is fixed by reusing the existing extent copier.
[Final installed evidence:12 Rust native +21 top-level Guest units (plus2 isolated
child test runs) +3 Python scenarios and17-home/8-owned-RAM cleanup](evidence/runtime/20261115-oci-raw-secrets/OUTCOME.md).
Box2 f9f48ab5…:23,393,755compressed/44,171,264installed bytes; OCI1 30eb5e29…:
33,424,560/114,954,240. Verify2261/43ignored,CLI935,installer8. Full OCI Host
Service/default cutover/live continuity/platform gates remain open; Windows
ARM64 dependencies unsupported/no native run, support levels unchanged and RSS
unmeasured. Continue the existing W4 custom writable-root disk-capacity gap.


Spec028 qualifies existing disk_mb on the explicit macOS OciSdk route using the
existing16MiB..64GiB ext4/rootfs-image/raw-storage contracts. Actual128MiB root+
anonymous ENOSPC/reuse, independent256MiB CLI controls,8GiB sparse disk, too-small
image refusal, replay/fences/normal+stopped three generations pass. Root remains
per-generation ephemeral; anonymous data persists. [Final installed evidence:
6 Rust native +3 Python scenarios,13-home/107-socket cleanup](evidence/runtime/20261115-oci-disk-capacity/OUTCOME.md).
Box4 39e6b5b3…:23,393,934compressed/44,171,264installed bytes; OCI30eb5e29…
unchanged/freshly extracted with honestc07659de… source. Verify2264/44ignored,
CLI935,installer8. Full W4/default Host Service cutover/live continuity/platform
gates remain open; Windows ARM64 dependencies unsupported/no native run, support
levels unchanged and RSS unmeasured. Continue the existing Mac cap_add allowlist gap.

Spec029 qualifies the existing Mac portable cap_add allowlist with explicit
nonroot inherited/ambient values shared by init and exec. Default10, root empty
ambient, drop ALL, unknown-cap refusal, per-UID budgets and Host PEP remain.
Actual MKNOD/SETFCAP effects, SYS_ADMIN mount/drop controls,45 observations,
CLI/realPTY,three generations and Host RO pass the final installed Box3/unchanged
OCI assets. Final3Rust native+3Python scenarios pass; verify Runtime2269/45ignored,
CLI935,installer8. Audit18 homes/235 absent sockets/21 pre-grant source identities
and zero owned resources passes. [Final assets, failures and evidence](evidence/runtime/20261115-oci-cap-add/OUTCOME.md).
Full W4/default Host Service cutover/live continuity/platform gates stay open;
Windows ARM64 dependencies remain unsupported/no native run and RSS unmeasured.
Continue the existing W4 policy.shm_size gap; platform support levels stay fixed.

Spec030 qualifies existing shm_size and size-only /dev/shm on Mac: page-aligned
4KiB..512MiB,1777/rw/nosuid/nodev/noexec,exact POSIX mapping/lifetime,1/2MiB
ENOSPC/reuse,noexec/sticky controls,CLI,three generations and7-home cleanup.
Final4Rustnative+3Python pass; verify Runtime2274/46ignored CLI935installer8.
[Pre-merge assets and evidence](evidence/runtime/20261115-oci-shm-size/OUTCOME.md). Full W4/default Host Service
cutover/live continuity/platform gates stay open; Windows ARM64 dependencies
unsupported/no native run and RSS unmeasured. The user requests convergence and
integration now; subsequent merged-source validation is recorded separately.

The [merged-source closeout](evidence/runtime/20261116-integration-closeout/OUTCOME.md)
records freshly rebuilt and isolated installed Box/OCI archives, exact source and
dependency identities, Runtime2274/CLI935/installer8, and5 Rust native tests plus3
Python native scenarios. All9 owned homes and99 socket paths are clear. The first
parallel OCI warm-boot timeout remains a recorded failure; an unchanged isolated
retry passes, without proving its cause fixed. Full W4/default Host Service
cutover and platform promotion remain open; Windows ARM64 dependencies remain
unsupported with no native run. Runtime RSS is unmeasured. Git convergence does
not publish a release or change these qualification limits.

The [current diagnostics checkpoint](evidence/runtime/20261116-native-tool-diagnostics/OUTCOME.md)
records newer rebuilt Box/OCI archives and five installed A–F native scenarios
passing. Full verification currently fails six Runtime disk-service tests and
one SDK test; independent OCI Host startup is blocked by unavailable macOS
DiskManagement. These candidates do not replace a green release gate. Current
owned resources are clear across six homes; prior historical RAM-preservation
failure remains unresolved. OpenSSL lock patches, exact artifacts and Git
convergence limits are linked there. Full W4/default cutover, Windows ARM64
dependency limitations and platform support levels remain unchanged.

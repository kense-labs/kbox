# kbox CLI 端到端测试计划

**版本**：v1.1

**状态**：本机 macOS HVF 非 TEE 矩阵已实施并通过；其他平台与外部发布资格化限制见[执行证据](evidence/box/2026-11-05-cli-e2e/OUTCOME.md)

**范围**：当前 `kbox` 二进制公开命令、`container-prune` 别名、原生后端、测试制品与发布安装 smoke

**非目标**：Box SDK、已删除的 `sdk-bridge`、OCI Runtime 内部协议 SDK、Agent Runtime／Harness／Plugin API、性能达标承诺

## 1. 验收口径

每条命令至少有一个从真实 `kbox` 进程入口开始的场景，验证参数解析、退出码、stdout／stderr、持久状态和实际副作用。只调用 Rust 函数、只检查 `--help`、只检查进程启动，均不算该命令的端到端覆盖。

命令按以下测试道执行：

| 测试道 | 真实边界 | 执行要求 |
| --- | --- | --- |
| E0 本地合同 | CLI 进程 → 本地状态／OCI 制品／测试服务 | 隔离 `K_HOME`，无外部网络，可在 PR CI 运行 |
| E1 原生执行 | CLI 进程 → ExecutionManager → 真实 MicroVM／Sandbox → 客体进程 | 在对应 HVF／KVM／WHPX 主机执行 |
| E2 专项集成 | CLI 进程 → PTY／回环网络／本地 Registry／守护进程 | 使用测试专用资源和合成凭据 |
| E3 TEE 模拟 | CLI 进程 → 模拟 SEV-SNP／RA-TLS → 客体 Secret | 仅证明模拟协议链，不证明硬件信任 |
| E4 TEE 硬件 | CLI 进程 → 真实 SEV-SNP 硬件和证明链 | 独立资格化，未执行时不得声明硬件支持 |

每个场景必须同时回答：

1. 命令声明的结果是否发生，而不只是退出码为 0。
2. stdout／stderr 和退出码是否符合合同，机器输出能否解析。
3. Box、进程、挂载、网络、卷、镜像、快照、socket 和临时目录是否保持正确所有权。
4. 正常结束、失败、取消和清理后是否无旧代次写入、无遗留进程、无晚写、无泄漏。
5. 当前平台不支持时是否在副作用前明确失败，而不是静默降级。

## 2. 当前命令清单与覆盖基线

当前命令面为 59 个一级帮助入口、43 个三级子命令，以及 `container-prune` 别名。`compose`、`snapshot`、`network`、`volume`、`pool` 是一级命令组，其叶子命令在各自矩阵中逐项列出。

现有资产直接复用：

- `command_coverage.rs`：帮助、本地状态、构建、Compose 配置和部分平台拒绝路径。
- `host_smoke.rs`：真实 VM、Compose、暖池、Registry 错误路径。
- `core_smoke.rs`：真实生命周期、PTY、网络、快照、监控和文件系统。
- `tee_integration.rs`：模拟 TEE、seal／unseal、Secret 注入。
- `scale_api_process.rs`：Scale API 进程重启和幂等重放。
- `wait_integration.rs`、`load_multi_platform.rs`、`guest_native_rootfs_integration.rs`：专项回归。
- `tests/support/mod.rs`：真实二进制查找、隔离 Home、进程守护、OCI fixture 和清理助手。

本次实施已闭合原计划中的本机覆盖缺口：

- Clap 动态命令树覆盖全部帮助入口，`network prune` 和 `snapshot prune` 另有真实状态断言。
- `import` 已验证 OCI 制品及导出 rootfs 的导入后实际启动；`audit`、`events`、本地 Registry 成功闭环均已执行。
- `scale-api` 已验证真实副本 `0→2→1→0`、旧 revision 拒绝和资源回收。
- Nginx、Alpine、Compose、暖池和客体原生 rootfs 场景均在 macOS HVF 上实际执行。
- 执行证据明确区分真实通过、ignored、环境跳过和未验证平台。

仍需独立资格化的范围：TEE、Linux KVM／Sandbox／chroot／port-forward、Windows WHPX、外部 Registry 和最终发布归档。

## 3. 测试底座

### 3.1 不再新增独立测试框架

继续使用 `kbox-cli` 的 Rust integration tests 和 `tests/support/mod.rs`。按生命周期、执行 IO、镜像、Compose、快照网络、暖池和专项服务拆分长场景；每个测试能独立运行、独立清理、独立报告。

帮助树通过 Clap `CommandFactory` 递归获取，再与实际二进制的 `--help` 对照，禁止继续维护容易漂移的手工命令列表。别名单独验证与目标命令行为一致。

### 3.2 进程与结果采集

在现有 `CliTest` 上补齐以下能力，保留 `ok`／`fails` 等便捷接口：

- 返回真实退出码、stdout bytes、stderr bytes 和耗时，非 UTF-8 输出不强制转字符串。
- 支持 stdin、PTY、后台进程、超时、SIGINT／SIGTERM 和退出回收。
- 每条命令记录完整 argv、环境白名单、结果和证据路径；Secret 只使用合成值并脱敏。
- 后台 guard 在成功、断言失败和 panic 时都能终止并回收其拥有的进程。
- 原生 VM 测试默认串行；本地 E0 可并行。禁止固定端口、固定 Box 名和共享 Home 造成伪冲突。

### 3.3 隔离与清理

每个测试使用独立 `K_HOME`、唯一项目名和动态回环端口。清理分两步：

1. 先验证命令自身是否按合同释放资源。
2. 再调用测试清理器移除剩余资源，并验证没有 shim／daemon、挂载、socket、listener 和运行时目录残留。

测试清理成功不能掩盖产品清理失败。若命令本应完成清理，必须先断言再补救。

### 3.4 固定测试制品

- 按 amd64／arm64 固定可运行 Linux 镜像 digest，并提供 OCI tar 离线入口；核心测试不依赖 `latest`。
- 准备最小 shell 镜像、HTTP 服务镜像、scratch 构建上下文、rootfs tar、单层和多层 OCI 镜像、多平台 index。
- 在测试进程内实现最小 OCI Distribution Registry，覆盖 pull、push、blob 上传下载、manifest、digest、认证和错误响应；不复用只返回 500 的 `ErrorRegistry` 充当成功路径。
- 凭据均为合成值。外部 Registry 只作为额外资格化，不作为 PR 测试前提。
- 所有归档和镜像检查 digest、文件内容、模式、所有权和 whiteout，不只检查文件存在。

### 3.5 覆盖清单与证据

每次资格化生成机器可读 `command-results.json`：命令、测试道、OS／arch、后端、镜像 digest、退出码、耗时、结果、跳过原因和日志路径。

每个测试道输出 `OUTCOME.md`，记录实际命令、通过／失败／未执行数量、主机、工具链、镜像、日志和 **Explicitly not claimed**。环境缺失在资格化 CI 中是未执行，不得换算成通过；平台明确不支持则用拒绝路径测试验收。

## 4. 一级命令矩阵

下表列出主要成功断言。每条命令还需一个适用的错误场景，例如对象不存在、状态冲突、无效参数或后端不可用；失败时必须保持原状态且无部分资源。

| 命令 | 测试道 | 端到端场景与关键断言 |
| --- | --- | --- |
| `run` | E1／E2 | 前台输出和退出码一致；detached 可观察运行；`--rm` 自动回收；覆盖 VM、Linux Sandbox 和 `run --pool` 路由 |
| `create` | E0＋E1 | 只预约配置，不启动 VM；`inspect` 保留完整命令与资源；随后 `start` 使用同一代次和配置 |
| `start` | E1 | created／stopped Box 实际启动，PID 身份、generation、状态和控制通道更新；重复启动不复制 VM |
| `stop` | E1 | 优雅退出和超时强停均回收客体与宿主进程，保留退出码和最终日志 |
| `restart` | E1 | 同一 Box 新 execution generation，旧控制请求失效，声明的持久数据保留 |
| `rm` | E0＋E1 | stopped 删除；`-f` 删除运行中 Box；记录、挂载和所属临时资源回收，命名卷按合同保留 |
| `kill` | E1 | 指定信号产生预期退出码，进程消失，后续 `wait` 可读取终态 |
| `pause` | E1 | 状态为 paused，客体进程不推进；不支持平台在执行前拒绝 |
| `unpause` | E1 | 原 Box 恢复运行和 exec，持久文件及 generation 关系正确 |
| `ps` | E0＋E1 | running／all／filter／format 与实际状态一致，不显示已回收记录 |
| `stats` | E1 | table／JSON、单次／流式均可解析；CPU、内存、PID、IO 和 pids 来自真实运行样本 |
| `logs` | E1 | stdout／stderr、tail、follow 和时间顺序正确；停止后最终日志不丢失 |
| `exec` | E1＋E2 | 输出、退出码、env、workdir、stdin 和非零结果正确；取消后无晚写 |
| `top` | E1 | 至少显示客体 init 和已知 workload 进程，不包含伪造的宿主进程 |
| `inspect` | E0＋E1 | JSON 可解析，ID、配置、状态、generation、挂载、网络和资源与后端一致 |
| `attach` | E1＋E2 | 能看到实时输出，detach／中断只断开 attach，不误停 Box |
| `attest` | E3／E4 | RA-TLS 验证、模拟标识和测量绑定正确；非 TEE、旧代次和不允许模拟时拒绝 |
| `audit` | E0＋E1 | 产生真实 create／pull／exec 等记录；action、box、outcome、limit、JSON 过滤正确且不含 Secret |
| `seal` | E3／E4 | 密文不含明文，绑定 context、policy 和 TEE 身份，输出 JSON 可解析 |
| `unseal` | E3／E4 | 同身份同 context 恢复原文；错误 context、身份、policy 或 blob 失败 |
| `inject-secret` | E3／E4 | 客体只读到指定 Secret；日志和宿主证据不泄漏；Box 停止后通道失效 |
| `wait` | E0＋E1 | 多 Box 按顺序返回真实退出码；超时不停止 Box；auto-remove 后读取归档终态 |
| `rename` | E0＋E1 | 新旧名称解析、inspect 和后续生命周期正确，ID 不变，冲突时不覆盖 |
| `port` | E0＋E1 | 端口映射与实际 listener 一致，删除或停止后不再显示失效映射 |
| `port-forward` | E2 Linux Sandbox | 回环 TCP 双向传输真实字节；并发连接、停止和断开后无 listener／socket 泄漏 |
| `export` | E1 | rootfs tar 包含新增、修改、删除和权限变化，路径安全且可解包核对 |
| `commit` | E1 | 变更形成可 inspect、save、run 的新镜像，内容、模式、所有权和 whiteout 正确 |
| `diff` | E1 | 新增／修改／删除分类与客体实际变更一致，无运行时内部文件误报 |
| `events` | E0＋E1 | create／start／pause／stop 等真实状态变化按序出现；since／until／filter／JSON 生效 |
| `container-update` | E0＋E1 | stopped 更新影响下一次启动；支持的 live 更新实际生效；不支持项不持久化 |
| `build` | E0＋E1 | scratch／COPY 元数据、RUN 产物、多阶段、cache 隔离和 `--run-pool` 真实构建；失败不发布 tag |
| `images` | E0 | 列表、quiet、digest 和过滤与缓存一致 |
| `pull` | E2 | 从本地 Registry 拉取并逐字节校验 manifest／blob／digest；断线和错误响应不发布半成品 |
| `push` | E2 | 推送后 Registry 可按 digest 读回相同字节；quiet 输出、认证和失败回滚正确 |
| `login` | E0＋E2 | 合成凭据按 registry 保存并用于认证；密码不出现在日志、argv 证据和输出 |
| `logout` | E0＋E2 | 只移除目标 registry 凭据，其他条目保留，之后认证按预期失败 |
| `rmi` | E0 | 多引用、force、被 Box 引用保护和不存在错误正确，不误删共享 blob |
| `image-inspect` | E0 | config、layer、digest、platform、healthcheck 等与 OCI 内容一致 |
| `history` | E0 | 层顺序、created_by、empty layer 和 no-trunc 与镜像历史一致 |
| `image-prune` | E0＋E1 | dangling／all、引用保护和 dry-run／force 语义正确，只删除可删除对象 |
| `tag` | E0 | 新引用指向相同 digest，源引用保留，随后 push／rmi 行为正确 |
| `save` | E0 | OCI tar 的 index、manifest、config、blob 和 digest 完整，可重新加载 |
| `load` | E0＋E1 | save／load 往返保持内容；多平台选择正确；损坏输入不发布 tag；加载镜像可运行 |
| `import` | E0＋E1 | rootfs tar／tar.gz 和 `--change` 形成单层镜像；history／CMD 正确并可实际运行 |
| `cp` | E1 | 宿主→客体和客体→宿主内容、权限、目录与覆盖行为正确；越界路径失败 |
| `df` | E0＋E1 | 各类占用与隔离 Home 中实际制品一致，verbose 可解释总量 |
| `prune` | E0＋E1 | 只删除 stopped Box，运行中 Box 和命名资源按合同保留；别名行为一致 |
| `system-prune` | E0＋E1 | stopped Box 和未使用镜像按选项清理，被引用对象保留，df 反映变化 |
| `version` | E0 | 输出版本与构建包版本一致，退出码为 0 |
| `info` | E0＋E1 | OS／arch、后端、Home、存储和暖池状态与当前环境一致 |
| `monitor` | E1 | 杀死受管 workload 后按 restart policy 拉起新代次；手动 stop 不误拉起；monitor 可干净停止 |
| `shell` | E1＋E2 | PTY 输入命令、获得输出并退出，窗口大小和退出码正确；不支持平台明确拒绝 |
| `scale-api` | E0＋E1 | 真实 HTTP 进程完成扩缩容、健康观察、幂等重放、旧 revision 拒绝和重启恢复 |
| `help` | E0 | 一级及全部嵌套帮助与 Clap 命令树一致，无缺失或陈旧命令 |

`container-prune` 与 `prune` 使用同一隔离 fixture 验证等价行为，不仅检查别名出现在帮助中。

## 5. `compose` 矩阵

使用两个具有依赖关系的服务、一个命名卷、一个项目网络和一个回环 HTTP 端口。先验证依赖顺序和收敛，再逐命令操作；每个场景独立项目，避免长测试相互掩盖。

| 命令 | 测试道 | 关键断言 |
| --- | --- | --- |
| `compose up` | E1 | 依赖顺序、readiness、服务 Box、网络和卷实际创建；重复 up 保持幂等；部分失败整批回收 |
| `compose down` | E1 | 只删除项目拥有的资源；`--volumes` 删除项目卷，前缀相似的非项目资源不受影响 |
| `compose ps` | E0＋E1 | 服务筛选、状态和端口与 Box inspect 一致 |
| `compose config` | E0 | ACL／YAML、环境插值、默认值、依赖、卷和非法输入结果正确，不启动运行时 |
| `compose logs` | E1 | 多服务输出归属、tail 和 follow 正确，停止后最终日志完整 |
| `compose start` | E1 | 指定已停止服务启动，依赖和配置保持一致 |
| `compose stop` | E1 | 服务停止但不删除记录，支持超时，其他服务不受影响 |
| `compose restart` | E1 | 服务新代次运行，卷数据保留，旧控制通道失效 |
| `compose rm` | E1 | 仅删除 stopped 服务；force 行为明确；项目网络和卷按合同处理 |
| `compose kill` | E1 | 信号和退出码正确，wait 能观察终态 |
| `compose pause` | E1 | 指定服务暂停，其他服务不受影响；不支持平台拒绝 |
| `compose unpause` | E1 | 原服务恢复 exec／HTTP，数据保持 |
| `compose wait` | E1 | 多服务退出码按服务输出，心跳和总超时不改变 workload 状态 |
| `compose exec` | E1 | 指定服务执行，env、workdir、TTY、user、stdin 和退出码正确 |
| `compose top` | E1 | 显示对应服务的真实客体进程 |
| `compose port` | E0＋E1 | 服务／私有端口映射与实际回环 listener 一致 |
| `compose cp` | E1 | 与指定服务双向复制，内容一致，不能写入其他服务 |
| `compose images` | E0 | 声明服务与镜像一一对应，可选服务筛选正确 |
| `compose pull` | E2 | 从本地 Registry 拉取全部／指定服务镜像，quiet 和 platform 正确 |
| `compose ls` | E0＋E1 | 项目注册、状态和 quiet 输出正确，down 后不再出现 |
| `compose volumes` | E0 | 只列出声明的命名卷，顺序稳定，不创建运行时资源 |

## 6. `snapshot` 矩阵

| 命令 | 测试道 | 关键断言 |
| --- | --- | --- |
| `snapshot create` | E1 | stopped MicroVM 和受管 live Sandbox 分别创建；元数据、rootfs 和所有权一致；运行中不支持路径拒绝 |
| `snapshot restore` | E1 | 恢复独立 Box，文件回到快照点，后续写入不污染源快照和其他分支 |
| `snapshot ls` | E0＋E1 | 表格／JSON 与实际快照元数据一致 |
| `snapshot rm` | E1 | 被引用快照受保护，force 只解除允许的 COW 引用，不误删源 Box |
| `snapshot inspect` | E0＋E1 | ID、源、大小、时间、描述和制品 digest 正确 |
| `snapshot prune` | E1 | `--keep` 和 `--max-bytes` 按保留策略删除，正在引用和新快照不误删 |

## 7. `network` 矩阵

| 命令 | 测试道 | 关键断言 |
| --- | --- | --- |
| `network create` | E0＋E1 | subnet、driver、isolation、egress、label 持久化；冲突和非法 CIDR 无部分写入 |
| `network ls` | E0 | 表格和 quiet 与网络存储一致 |
| `network rm` | E0＋E1 | 未使用网络删除；占用时拒绝；force 先正确断开端点 |
| `network inspect` | E0＋E1 | subnet、端点、IP／MAC、策略和连接 Box 与后端一致 |
| `network connect` | E1 | stopped Box 获得唯一端点；启动后可按策略通信；重复连接不重复分配 |
| `network disconnect` | E1 | 端点和 listener 回收，Box 回到声明的默认网络；force 行为明确 |
| `network prune` | E0＋E1 | 只删除未使用网络，项目或 Box 使用中的网络保留 |

Linux 验证真实 Bridge／TSI／Sandbox 数据面和 egress 拒绝。Windows 验证不支持的 Bridge 在创建 Box 或网络副作用前失败。

## 8. `volume` 矩阵

| 命令 | 测试道 | 关键断言 |
| --- | --- | --- |
| `volume create` | E0 | driver、label、目录和元数据创建；重名和非法名称不覆盖 |
| `volume ls` | E0 | 表格和 quiet 与实际卷一致 |
| `volume rm` | E0＋E1 | 未使用卷删除；挂载中拒绝；force 的权限边界明确 |
| `volume inspect` | E0 | 路径、driver、label、引用和大小信息正确 |
| `volume prune` | E0＋E1 | 仅未使用卷删除；Box 或 Compose 引用卷保留 |

另设 E1 持久化场景：写入命名卷，stop／restart／删除 Box 后重新挂载，内容、权限和只读约束保持。

## 9. `pool` 矩阵

| 命令 | 测试道 | 关键断言 |
| --- | --- | --- |
| `pool start` | E1 Unix | socket 就绪、指定镜像和 warm 数量达到 idle；配置错误和启动失败可回收；重复启动不冲突 |
| `pool run` | E1 Unix | 热执行 stdout／stderr／退出码正确；并发、多镜像、lazy、deferred、超时和环境隔离正确 |
| `pool stop` | E1 Unix | drain 后 daemon、VM、socket 和 listener 全部退出；重复 stop 有定义行为 |
| `pool status` | E1 Unix | table／JSON 中 idle、active、leased、created、evicted 与真实操作一致 |

Windows 对 `start` 和不支持操作执行拒绝测试。E3／E4 不使用暖池冒充独立 TEE 资格化。

## 10. 平台与失败矩阵

| 平台 | 必跑成功路径 | 必跑拒绝／边界路径 |
| --- | --- | --- |
| Linux KVM | MicroVM 全生命周期、exec、文件、镜像、Compose、快照、网络、暖池、监控 | KVM／guest companion 缺失、旧 generation、取消、SIGKILL、后端故障 |
| Linux OCI Sandbox | Sandbox 生命周期、port-forward、live managed snapshot、资源 stats、共享内核限制 | 不支持 VM-only 能力、seccomp／capability 越权、网络 egress、旧代次 |
| macOS HVF Apple Silicon | MicroVM 全生命周期、rootfs、virtio-fs、TSI、Compose、快照、暖池、模拟 TEE | 非 ARM64／错误镜像、宿主挂载清理、HVF 或同伴缺失 |
| Windows WHPX | 支持的 MicroVM 生命周期、exec、文件、镜像和资源读取 | pause／unpause、Bridge、交互 PTY、pool、健康检查、TEE 等按源码合同拒绝 |

TEE 分为模拟与真实硬件两条证据链。TDX 等未资格化入口只验证明确拒绝，不以模拟结果声明硬件能力。

通用对抗场景至少包括：无效镜像 digest、损坏 tar、路径逃逸、输出上限、无效 CIDR、重复名称、错误状态、旧 generation、daemon SIGKILL、宿主重启后对账、宿主与客体超时，以及取消后的晚写检查。

## 11. 实施顺序

### P0：命令面与基线

- 删除 Box SDK 和 `sdk-bridge`，`stats` 直接调用 ExecutionManager。
- 用 Clap 递归命令树替代手工清单，补齐所有嵌套帮助和别名。
- 固定当前支持矩阵、镜像 digest、命令输出合同和证据格式。

### P1：统一测试底座

- 统一进程、退出码、PTY、后台 guard、隔离 Home、资源清理和结果清单。
- 将 TEE 等旧测试迁入相同隔离与清理约束。
- 实现本地成功／失败 Registry 和离线镜像制品。

### P2：E0 全命令覆盖

- 完成帮助、版本、本地状态、镜像、Compose config、卷和网络元数据、prune、登录态。
- 完成 build／save／load／import 制品链和失败回滚。
- 生成第一张无遗漏命令覆盖表，再进入原生执行扩展。

### P3：核心原生执行

- 拆分真实 VM 长场景，完成生命周期、exec／shell／attach、stats／logs／top、cp、export／commit／diff。
- 完成 Linux Sandbox、port-forward、container-update、events／audit 和 monitor。
- 每个平台独立记录，不用 Linux 结果替代 macOS／Windows。

### P4：组合资源与守护进程

- 完成全部 Compose、snapshot、network、volume、pool 和 scale-api 场景。
- 增加并发、进程重启、崩溃恢复、资源回收和重复操作。

### P5：TEE、发布与稳定性

- 模拟 TEE 全链通过后再执行真实 SEV-SNP 资格化。
- 对最终发布归档做隔离前缀安装，使用安装后的 `kbox` 重跑核心 E0／E1，而非开发目录 binary。
- 反复执行核心生命周期、Compose、pool 和 snapshot，检查资源曲线、晚写与泄漏；只记录性能基线，不预设达标数字。

## 12. CI 与完成门禁

| 触发 | 必跑内容 | 通过条件 |
| --- | --- | --- |
| PR | 格式、Clippy、CLI 库测试、全部 E0、Clap 命令树 | 无失败、无命令遗漏、无未解释跳过 |
| Linux 原生 CI | KVM＋OCI Sandbox E1／E2、Linux 专项 | 两后端分别通过并清理，不共享支持结论 |
| macOS 原生 CI | HVF E1／E2／E3、rootfs 专项 | Apple Silicon 实机通过，模拟 TEE 标识清楚 |
| Windows 原生 CI | WHPX 支持路径和全部拒绝路径 | 无静默降级、无前置副作用 |
| TEE 资格化 | 真实 SEV-SNP E4 | 硬件、测量、证明和 Secret 链全部有证据 |
| Release | 最终归档安装后核心 E0／E1、soak | 制品完整、安装后命令通过、无资源泄漏 |

计划完成必须同时满足：

- [ ] 59 个一级入口、43 个三级子命令和 `container-prune` 别名均有结果记录。
- [ ] 每条命令都有实际结果断言、至少一个失败边界和清理断言。
- [ ] 未把 `--help`、单元测试、mock 成功或进程启动计入端到端成功覆盖。
- [ ] Linux KVM、Linux Sandbox、macOS HVF、Windows WHPX 分别有原生结果。
- [ ] 模拟 TEE 与真实硬件 TEE 结论分开。
- [ ] 无真实凭据、无共享 Home、无固定端口、无跨测试资源依赖。
- [ ] 无未解释的 ignored／skip；未执行项目明确列入 **Explicitly not claimed**。
- [ ] 最终发布归档而非仅开发目录构建通过安装后 smoke。

## 13. 当前未声明

本计划制定时不声明上述端到端矩阵已实施或通过，不声明真实 MicroVM、WHPX、硬件 TEE、外部 Registry 或最终发布归档已在本轮验证。当前仅确认 Box SDK 删除后的构建、CLI 库测试和静态检查结果；真实命令资格化以各测试道后续 OUTCOME 为准。

# Phase 1 设计：Windows WHPX 上的 environment-prewarm

**状态**：设计定稿（2026-11-15）· 实现按 next-workplan WP-5→WP-7 持续推进\
**灵感**：LCOW「UVM 与 task 分离」——**只学状态分层，不接线 HCS/LCOW**\
**基线宿主**：Windows 10/11 x86_64 + WHPX（见 Phase 0 证据）\
**冷启动对照**：`docs/evidence/box/20261115-windows-lcow-opt-phase0-baseline/`（P50 ≈ 12.3 s）

---

## 1. 目标

在**不 reboot MicroVM**的前提下，让兼容的第二次 workload 走 `claim_prewarmed`，墙钟时延显著低于 Phase 0 冷启动 P50。

默认路径仍为 WHPX + libkrun + Linux guest；不引入 Containers/HNS/LCOW v2/Server 2025 硬依赖。

---

## 2. 选用合同：environment-prewarm.v1（非 Unix warm-pool daemon）

| 层 | 采用 | 不采用（本阶段） |
| --- | --- | --- |
| 状态权威 | `LocalExecutionManager` + `ManagedPrewarm`（`local_execution/prewarm.rs`） | 第二套执行状态机 |
| 预热形态 | idle bootstrap：`/bin/sleep` + `infinity`，digest-pinned image，无 secrets | `pool.prewarm` 进程 marker（Unix） |
| 调用面 | example / 窄包装 / 既有 WHPX OCI qualification 入口 | CLI `kbox pool *` 成功路径 |
| 控制面 | named pipe + 既有 WHPX owner（可选 live-session 加固） | Unix domain socket pool 协议 |
| 引擎补给 | 暂缓 Windows `engine::run_prewarm` | 整包 `cfg` 启用 `engine` + `kbox_contract_driver` |

Wire：`scripts/wire-contracts.json` → `environment-prewarm.v1`。

---

## 3. 状态机

```text
ensure_prewarmed_reservation(request, op, scope, owner)
  → store 中至多一个同 scope/owner/compatibility_digest 的未 claim 槽
start_prewarmed(id, generation)   # 持 lifecycle lock，走 ensure_started
  → Running + idle sleep infinity
claim_prewarmed(request, op, scope)
  → backend.inspect 校验 pid / exec_socket / oci_runtime
  → generation = generation+1，替换 managed 元数据，**不**重建 VM
  → 调用方在已 Running 的 VM 上提交真实 workload（exec/后续合同）
release / kill / remove
  → 标准 generation fencing；泄漏清理同既有 WHPX owner 路径
```

失败语义：

- 不兼容 digest / 非 Running / inspect 失败 → claim 返回 `None` 或跳过，调用方 **冷路径** create+start（不得降级隔离）。\
- 冲突 operation → `Conflict`。\
- 禁止：claim 失败后改走 process isolation / HCS。

---

## 4. 落点（真实路径）

| 组件 | 路径 | Phase 1 动作 |
| --- | --- | --- |
| 预热核心 | `crates/box/runtime/src/local_execution/prewarm.rs` | **复用**；必要时仅修 Windows 表现缺陷 |
| 单测 | `.../local_execution/tests/prewarm.rs` | WP-5 在本机跑绿 |
| WHPX 资格 | `examples/windows_whpx_oci_qualification.rs` | 对照；新增 prewarm 专用 example |
| WHPX owner / pipe | `local_execution/oci_whpx_owner.rs` | 长跑预热槽可选加固 |
| live-session | `oci-runtime` `whpx_*session*` / `windows_session_owner.rs` | **可选**；非 P50 阻塞 |
| CLI pool | `crates/box/cli/src/commands/pool.rs` | **保持** Windows fail-closed |
| contract_driver / engine | `lib.rs` 现仅 linux\|macos | **本阶段不整包打开** |

---

## 5. 最小垂直切片（WP-6/7）

1. 隔离 `K_HOME` + 既有 boot/OCI 镜像（digest-pinned，满足 `validate_bootstrap`）。\
2. `ensure_prewarmed_reservation` + `start_prewarmed`，等待 Running。\
3. 计时 `claim_prewarmed`（及随后一次短 exec/观测，若合同需要）。\
4. 对照同环境冷 `create`+`start`（或 Phase 0 `kbox run` 仅作量级参考，**不可**替代 claim 证据）。\
5. Cleanup：无残留 VM / pipe / store 记录。

**通过线（设计目标，非已宣称）**：claim 路径 P50 ≪ Phase 0 冷启动 P50（数量级期望：秒内或明显个位数秒，以实测为准）。

---

## 6. 与 bridge / warm-pool 现状

- `kbox info`：`Warm pool daemon: unsupported on Windows` — **保持**直至另立 CLI 史诗。\
- bridge — 仍硬拒绝；网络属 Phase 3，不在本设计范围。

---

## 7. 兼容性自检

- [x] 默认路径不要求 Server 2025 / LCOW v2 / Containers 功能包\
- [x] 不以 HNS 为预热或网络前提\
- [x] 不共享宿主内核\
- [x] 不静默切换 hypervisor\
- [x] 不因本设计改 `support-matrix` readiness\

---

## 8. 风险与拍板（已取默认）

| 问题 | 默认决定 |
| --- | --- |
| 是否先做 pool CLI？ | 否，先运行时/example |
| 是否打开完整 contract_driver？ | 否；WP-8 才考虑窄包装 |
| claim 后如何跑非 sleep 的用户命令？ | 沿用既有 exec / 后续 apply 合同；切片先证明「不 reboot 的 handoff」 |
| 预热 VM 被 Host 退出杀掉？ | 可选 `K_OCI_WHPX_SESSION_OWNER`；首轮可不启用 |

---

## 9. 相关文档

- `docs/architecture/windows-lcow-inspired-optimization.md`\
- `docs/agent-prompts/lcow-inspired-krun-next-workplan.AGENT.md`\
- `crates/oci-runtime/docs/windows-whpx-live-session.md`\
- `specs/001-krun-engine/research.md`（pool vs environment-prewarm 分工，若存在）\

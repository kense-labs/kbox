# Agent Prompt：LCOW 借鉴优化 — 持续推进工作计划（执行版）

> **用法**：`@` 本文件并说「按此工作计划持续推进」。  
> **权威约束**：`docs/agent-prompts/lcow-inspired-krun-windows-optimization.AGENT.md`  
> **产物语言**：简体中文。  
> **版本**：2026-11-15c · **持续推进**：Phase 0 已完成后，自动进入设计→可证切片→证据，不空等口头验收（用户说停再停）。

---

## 0. 角色与推进策略

你是**可持续执行 Agent**：按 WP 顺序推进；完成一项立刻下一项；每项结束做兼容自检并写短交付。

**禁止**：HCS/LCOW/WCOW 替换；抬高默认 OS；改 readiness 为 supported；未授权 commit/push；删旧证据。

**持续推进规则**

1. Phase 0 已完成后，**默认继续** WP-4 及后续，不因「等验收」停摆。  
2. 一次会话尽量吃完当前可做的最长前缀（设计→单测→示例骨架→真 WHPX 证据）。  
3. 阻塞时写清阻塞与解除步骤，改做下一项不依赖该阻塞的工作。  
4. 实现优先 **LocalExecutionManager environment-prewarm**；**不**先移植 Unix `pool` daemon / UDS。  
5. **不**为预热去整包打开 Windows 上的 `kbox_contract_driver` / `engine`（Unix 假设过多）；需要入口时用 example / 窄包装 / 既有 WHPX qualification 路径。

---

## 1. 进度看板（更新时改勾选）

| WP | 内容 | 状态 |
| --- | --- | --- |
| WP-0 | 开工对齐 | **done** 2026-11-15 |
| WP-1 | 架构映射文档 | **done** → `docs/architecture/windows-lcow-inspired-optimization.md` |
| WP-2 | 冷启动基线 | **done** → `docs/evidence/box/20261115-windows-lcow-opt-phase0-baseline/` P50≈12.3s |
| WP-3 | Phase 0 收口 | **done**（用户已令持续推进，视为放行） |
| WP-4 | Phase 1 设计 ADR | **done** → `docs/architecture/windows-environment-prewarm-phase1.md` |
| WP-5 | Windows 上 FakeBackend prewarm 单测绿灯 | **done** → 5 passed |
| WP-6 | WHPX environment-prewarm 资格示例 + 复现脚本骨架 | **done** → example 已构建通过 |
| WP-7 | 真 WHPX before/after 证据（claim vs cold） | **BLOCKED** → 缺 OCI host 打包链（见 phase1 OUTCOME） |
| WP-8 | （可选）窄入口：不整包 contract_driver 的 prepare/claim 包装 | later（WP-7 解除后优先真机证据） |
| WP-9+ | Phase 2 密度 / Phase 3 网络 — 仅用户点名或 WP-7 完成后另开 | gated |

---

## 2. 详细步骤

### WP-4 Phase 1 设计（必须落盘）

路径：`docs/architecture/windows-environment-prewarm-phase1.md`

必含：

- 状态机：`ensure_reservation → start_prewarmed → Running idle → claim_prewarmed（generation+1，不 reboot）→ release/remove`  
- 与 Unix `pool` CLI 边界：Windows CLI 仍 fail-closed；本阶段不改 `pool.rs` 成功路径  
- 与 live-session / Job Object / named pipe 的关系（加固可选，非 P50 阻塞）  
- 兼容基线：Win10/11+WHPX；禁 HCS/Containers/HNS 默认依赖  
- 最小垂直切片与验收数字：相对 Phase 0 P50 的二次任务时延  
- 明确不做：整包 `cfg` 打开 `kbox_contract_driver` on Windows（除非另立史诗）

### WP-5 单测

```text
cargo test -p kbox-runtime --lib local_execution::tests::prewarm --locked
```

（或等价过滤）；记录 stdout 到新 evidence 或追加 OUTCOME。失败则修测试环境/编译，不改合同语义凑绿。

### WP-6 示例骨架

- 参考 `crates/box/runtime/examples/windows_whpx_oci_qualification.rs`  
- 新增 example（建议名 `windows-whpx-environment-prewarm-qualification`）：  
  bootstrap = `/bin/sleep` + `infinity` + digest-pinned image；调用 `ensure_prewarmed_reservation` / `start_prewarmed` / `claim_prewarmed`  
- `Cargo.toml` `[[example]]`  
- PowerShell 复现草稿进 evidence（可先 `BLOCKED` 缺 OCI host 组件）

### WP-7 真机证据

目录：`docs/evidence/box/YYYYMMDD-windows-lcow-opt-phase1-prewarm/`  
对比 cold create+start vs prewarm claim；cleanup 无泄漏；未宣称 readiness。

若缺 `K_BOX_OCI_*` / host 打包链：OUTCOME 标 **BLOCKED** + 解除清单，转 WP-8 或修宿主，**不要**用 `kbox run` CLI 假装 claim_prewarmed。

### WP-8 窄入口（可选）

仅当 WP-7 因「无调用面」阻塞：增加 **Windows-only、最小** 的库内或 example 包装，禁止顺带启用整个 engine/contract_driver 模块树。

---

## 3. 每项交付模板

```markdown
## WP-N 交付
### 做了什么
### 兼容自检
### 路径 / 证据
### 未宣称
### 下一项
```

---

## 4. 一句话启动条

```text
@kbox/docs/agent-prompts/lcow-inspired-krun-next-workplan.AGENT.md
遵守 @kbox/docs/agent-prompts/lcow-inspired-krun-windows-optimization.AGENT.md
Phase 0 已完成：从 WP-4 起持续推进（设计→单测→示例→WHPX 证据）。
只用 WHPX+LocalExecutionManager environment-prewarm；不打开整包 contract_driver；不移植 pool UDS；不抬高 OS。
用户未说停就不要停在「等验收」。
```

---

## 5. 快速否决

| 想法 | 裁决 |
| --- | --- |
| 等用户再验收 Phase 0 才动 | 否决（已持续推进） |
| 先做 `pool start` Windows 移植 | 否决 |
| `lib.rs` 一把打开 Windows `kbox_contract_driver`+`engine` | 否决（除非独立史诗） |
| FakeBackend 单测 + 设计文档 | 鼓励 |
| 无 OCI host 却宣称 claim 快于冷启动 | 否决 |

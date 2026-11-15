# Phase 1 推进：environment-prewarm 设计 / 单测 / 示例（2026-11-15）

**Overall**：**DESIGN + UNIT TESTS PASS；WHPX LIVE CLAIM TIMING BLOCKED**  
**未宣称**：产品 readiness 晋升；claim 已快于冷启动（真 WHPX 尚未跑通）。

---

## Done

### WP-4 设计

`docs/architecture/windows-environment-prewarm-phase1.md`

### WP-5 FakeBackend 单测（本机 Windows）

```text
cargo test -p kbox-runtime --lib local_execution::tests::prewarm --locked
```

**结果**：`5 passed; 0 failed`（约 0.27s 测试体；全量编译含既有 warnings）

覆盖：generation fencing、并发 claim 唯一赢家、未 claim 环境不可跑 hooks、精确 scope/资源、停止预留后 fence 晚启动。

日志摘要：`logs/unit-prewarm-tests.txt`（若存在）或本 OUTCOME。

### WP-6 示例骨架

- 源码：`crates/box/runtime/examples/windows_whpx_environment_prewarm_qualification.rs`  
- Cargo：`[[example]] name = "windows-whpx-environment-prewarm-qualification"`  
- **本机构建**：`cargo build -p kbox-runtime --example windows-whpx-environment-prewarm-qualification --locked` → **Finished ok**

复现（需完整 OCI WHPX host 栈，与既有 `windows-whpx-oci-qualification` 同级）：

```powershell
$env:K_BOX_WHPX_PREWARM_QUALIFICATION = "1"
$env:K_HOME = "<abs isolated home>"
$env:K_BOX_OCI_HOST_ROOT = "<abs oci host root>"
$env:K_BOX_OCI_WHPX_ENDPOINT = "<endpoint>"
$env:K_BOX_WHPX_OCI_IMAGE = "<name>@sha256:<64 hex>"
$env:K_BOX_WHPX_PREWARM_REPORT = "<abs report.json>"
cargo run -p kbox-runtime --example windows-whpx-environment-prewarm-qualification --locked
```

---

## WP-7 BLOCKED（真 WHPX claim vs cold）

| 缺失 | 说明 |
| --- | --- |
| OCI Host 打包 / `K_BOX_OCI_HOST_ROOT` | Phase 0 CLI `kbox run` 路径 **不等于** `LocalExecutionManager` WHPX OCI qualification；不能拿 CLI 冒充 `claim_prewarmed` |
| digest-pinned 预热镜像 | `validate_bootstrap` 要求 `image@sha256:…` + sleep infinity |
| 可选 box-owned 服务二进制 | `koci` / shim / vm-rootfs / system-image manifest |

**解除后**：在本目录写入 `report.json` + 更新本 OUTCOME 为 PASS，对比 Phase 0 P50（12270 ms）。

---

## Compatibility

- 未依赖 Server 2025 / LCOW / Containers / HNS  
- 未打开 Windows 整包 `kbox_contract_driver` / `engine`  
- 未改 `support-matrix` readiness  
- CLI `pool` 仍 fail-closed  

---

## Next

1. 备齐 WHPX OCI host 工件后跑 WP-7。  
2. 或 WP-8：仅当「无调用面」成为唯一阻塞时，加窄包装（仍禁止整包打开 contract_driver）。  

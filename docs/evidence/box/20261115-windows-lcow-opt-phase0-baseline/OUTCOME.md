# Phase 0 基线：Windows WHPX 冷启动 / RSS（LCOW 借鉴优化）

**日期**：2026-11-15  
**Overall**：**PHASE 0 BASELINE RECORDED** — 仅测量与范围锁定；**未宣称**任何优化收益；**未改**产品 readiness / 运行时代码。  
**计划**：`docs/agent-prompts/lcow-inspired-krun-next-workplan.AGENT.md` WP-2  
**架构映射**：`docs/architecture/windows-lcow-inspired-optimization.md`

---

## Environment

| Item | Value |
| --- | --- |
| OS | Windows 10 Pro（`Professional`）10.0.22631.4890 |
| Arch | x86_64 |
| HypervisorPresent | true |
| WHPX | available（`kbox info`） |
| kbox | 3.4.3；`C:\Users\admin\.kbox\bin\kbox.exe`（47,453,184 bytes） |
| shim / guest-init / DLL | 同目录 `kbox-shim.exe`、`kbox-guest-init`、`krun.dll`、`libkrunfw.dll` |
| Isolated `K_HOME` | `…/20261115-windows-lcow-opt-phase0-baseline/k-home-phase0` |
| Image | `e0/boot:latest` ← `20261109-windows-visualized-e2e/fixtures/boot-rootfs.tar` |
| Containers 功能包 | 未作为依赖；`Get-WindowsOptionalFeature` 需提升未查 |
| HNS / vmms | 宿主上服务可能为 Running（Hyper-V/WHPX 常见）；**本测量未调用 HNS API**；默认路径不依赖 Containers/HNS |

见 `logs/host-notes.json`、`logs/info.txt`。

---

## Cold start

命令（隔离 `K_HOME`）：

```text
kbox run --rm --network none e0/boot:latest -- /bin/sh -c "echo KBOX_PHASE0_BASELINE; uname -a; id"
```

| Metric | Value |
| --- | --- |
| 成功样本 N | **14** |
| 观测失败 | **1**（iter=9，exit=1，无 guest stdout；日志 `logs/run-9.txt`；未计入成功统计） |
| min / max | 11294 / 22455 ms |
| mean | 13986 ms |
| **P50** | **12270 ms** |
| **P95** | **22455 ms** |

成功样例 guest 输出含 `KBOX_PHASE0_BASELINE`、`Linux localhost 6.12.91`、`uid=0(root)`（见 `logs/run-1.txt`）。

原始：`logs/runs-summary.txt`、`logs/cold-start-stats.json`、`logs/run-*.txt`。

---

## RSS（单运行中 MicroVM）

```text
kbox run -d --network none --name phase0-rss e0/boot:latest -- /bin/sleep 180
# sleep 5s 后采样 Get-Process / Win32_Process
kbox rm -f phase0-rss
```

采样时可见两个 `kbox-shim`（详见 `logs/rss-sample.json`）：

| pid | Working Set | Private | 备注 |
| --- | --- | --- | --- |
| 69936 | ~117.3 MB | ~1052 MB | 与本次 running box 对应的主 shim（Private 含映射/固件相关，解释保留为观测值） |
| 9768 | ~11.9 MB | ~2.3 MB | 同名进程残留/旁路；清理后 `kbox ps -a` 为空 |

清理后：`logs/ps-final.txt` 无残留 Box。

---

## Compatibility self-check（权威 §2.4）

- [x] 未引入 Server 2025 / LCOW v2 / Containers 功能包 API  
- [x] 测量在 WHPX + 既有 kbox 二进制上完成；不以 HNS 为网络平面  
- [x] 未把可选能力写成全 Windows 必达  
- [x] 未提高最低 OS 版本；未改 `support-matrix.json`

---

## Explicitly not claimed

- 任何「优化后更快」或产品完成度变化  
- readiness 从 `probe-only` 晋升  
- warm pool / bridge / TEE 在 Windows 上变为 supported  
- HNS 或 LCOW 对照数字（本轮未跑 LCOW）

---

## Reproduce

```powershell
$ev = "D:\code\kense\kbox\docs\evidence\box\20261115-windows-lcow-opt-phase0-baseline"
$env:K_HOME = Join-Path $ev "k-home-phase0"
$env:MSYS_NO_PATHCONV = "1"
$kbox = "$env:USERPROFILE\.kbox\bin\kbox.exe"
# bin 与 e0/boot:latest 已在本目录 K_HOME 中准备；复跑：
& $kbox run --rm --network none e0/boot:latest -- /bin/sh -c "echo KBOX_PHASE0_BASELINE; uname -a; id"
```

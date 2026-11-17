# 项目知识

[任务与产物](Tasks.md) · [演进记录](Chronicle.md) · [决策](Decisions.md)

仅记录本项目真实事实。初始化不继承模板来源项目的历史或完成结论。Wiki ≠ Pass；波次 % ≠ Pass。

## 当前波次

- **HEAD / wave**: `sandbox-isolation`（Spec050）
- **stage_floor**: verifying
- **总体目标（GOALS）**: Docker-shaped 运行时；默认 **sandbox**（融合上游 Apache-2.0 引擎、零品牌残留）；更高隔离可选 **libkrun / MicroVM**。「做成」= 真实用户日常可装可用（默认 sandbox）。门禁绿 / 77 ≠ Pass。
- **本机态势（darwin arm64，2026-11-17）**: Sandbox 默认已落地；包内 stream / captured stdin / Compose Secrets 冒烟绿；可选 MicroVM 回归 `MICROVM_OK`；总体目标仍 **unmet**。

## 已验证（有证据，≠Pass）

| 切片 | 回执 |
| --- | --- |
| Sandbox 默认 + daemon ensure | [T5-DEFAULT-SANDBOX.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/T5-DEFAULT-SANDBOX.md) |
| 包内 stream / stdin / Compose Secrets | [T5-PACKAGE-STREAM.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/T5-PACKAGE-STREAM.md) · [T5-PACKAGE-EXEC-STDIN.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/T5-PACKAGE-EXEC-STDIN.md) · [T5-PACKAGE-COMPOSE-SECRETS.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/T5-PACKAGE-COMPOSE-SECRETS.md) |
| Compose Secrets（原生） | [T5-COMPOSE-SECRETS.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/T5-COMPOSE-SECRETS.md)（`PARK-COMPOSE-SECRETS` 已关闭） |
| 可选 MicroVM 回归 | [T5-MICROVM-REGRESSION.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/T5-MICROVM-REGRESSION.md) |
| 开发者日常安装（quarantine clear · ≠notarize） | [T5-DAILY-INSTALL.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/T5-DAILY-INSTALL.md) |
| 战役总览 | [OUTCOME.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/OUTCOME.md) |

## 停泊 / 残留缺口（总体仍 unmet）

| 项 | 状态 | 回执 |
| --- | --- | --- |
| Notarize / Gatekeeper staple | parked | [PARK-NOTARIZE.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/PARK-NOTARIZE.md) |
| Multi-host 矩阵 | parked | [PARK-MULTIHOST.md](../../docs/evidence/runtime/20261117-sandbox-isolation-backend/PARK-MULTIHOST.md) |
| 融合引擎 create Env 可能残留展开后的 Secret 值 | residual（`boxes.json` 不含明文） | OUTCOME「Remaining overall-goal gap」 |

任务台账：[T2026-11-17-sandbox-isolation-backend](Tasks.md#t2026-11-17-sandbox-isolation-backend)。

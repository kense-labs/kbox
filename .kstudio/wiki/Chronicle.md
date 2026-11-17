# 演进记录

## 2026-11-17 — wave `sandbox-isolation`（Spec050）· verifying · ≠Pass

- GOALS 锁定默认 **sandbox**（R3/R4）；libkrun/MicroVM 更高隔离可选。
- 证据根 `docs/evidence/runtime/20261117-sandbox-isolation-backend/`：Sandbox 默认、包内 stream/stdin/Compose Secrets、MicroVM 回归已记回执；`PARK-COMPOSE-SECRETS` 本机关闭；`PARK-NOTARIZE` / `PARK-MULTIHOST` 停泊；融合引擎 Env Secret 残留仍开。
- stage_floor: verifying · overall unmet · wiki/Tasks 登记 `T2026-11-17-sandbox-isolation-backend`（partial）。

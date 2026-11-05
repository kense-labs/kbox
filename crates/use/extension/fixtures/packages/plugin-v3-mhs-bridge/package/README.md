# MHS Bridge Fixture

## 概述

本目录保存 crates/use/extension/fixtures/packages/plugin-v3-mhs-bridge/package 的说明和使用约束。

## 构建与验证

按所属工程的构建和测试合同使用本目录内容。

This package proves the Krun Use integration boundary for a Model Hardware
Standard (MHS) adapter without defining or emulating the MHS protocol.

The package-owned MCP service is an adapter. It has no direct device authority.
A trusted host must inject a scope-bound MHS gateway session, and the gateway
must enforce device identity, procedure permissions, parameter bounds,
exclusive leases, interlocks, and emergency-stop behavior outside the package.

This fixture is not an MHS implementation and must not be deployed against
physical equipment.

The package deliberately reuses the standard MCP, Flow, Skill, and UI surface
graph. Dynamic device state stays behind MCP rather than entering the Krun Use
capability snapshot. Physical mutations must not be retried after an ambiguous
outcome; the caller must observe and reconcile first.

See `docs/mhs-integration.md` in the Krun Use repository for the complete
ownership, readiness, permission, and safety contract.

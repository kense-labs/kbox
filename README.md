# krun

## 概述

krun 是本地 Agent Runtime 与 OCI 工作负载执行工作区，包含 MicroVM 执行资源、Task／Service 合同、Plugin 服务接入、模型网关和行为干预组件。

工程采用多 crate 结构，并使用独立的 `libkrun`、`libkrunfw` 子模块。

## 工程结构

```text
krun/
├── apps/gateway/          模型 Provider 网关
├── crates/box/            执行资源、MicroVM 与生命周期
├── crates/oci-runtime/    OCI 工作负载运行时
├── crates/runtime/        Task／Service 合同与控制接口
├── crates/sentry/         行为判定与干预
├── libkrun/               libkrun 子模块
└── libkrunfw/             libkrunfw 子模块
```

## 克隆

```bash
git clone --recursive https://github.com/kense-labs/krun
cd krun
```

已有检出使用：

```bash
git submodule update --init --recursive
```

## 构建与测试

```bash
cargo check --workspace --all-targets --locked
cargo test --locked -p krun-runtime -p krun-box-core -p krun-sentry -p krun-libkrun-sys --lib
cargo clippy --locked -p krun-libkrun-sys --all-targets --no-deps
```

工具链版本见 `rust-toolchain.toml`。`publish = false` 的包只通过本地路径解析。

## 文档

- [Agent Runtime 架构](docs/architecture/agent-paas-runtime.md)
- [krun Box 优化与 Agent Runtime 开发计划](docs/krun-box-smolvm-inspired-development-plan.md)
- [smolvm 与 krun Box 对比](smolvm-vs-krun-box-report.md)
- [多 Harness 协作架构](docs/architecture/harness-gateway.md)
- [沙箱、观测与干预架构](docs/architecture/sandbox-control.md)

## 支持边界

Agent Runtime v1.1 是目标架构。文档中的新增能力不代表已经实现。实际支持范围以源码、测试和平台资格化结果为准。

## 许可

见 [LICENSE](LICENSE)。第三方组件保留各自许可和源码说明。

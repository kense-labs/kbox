# krun

Aster Box、OCI Runtime、Agent Runtime、Sentry 与 Model Gateway 的迁移工作区。

## 来源

- 源仓库：`aster`
- 源提交：`0a987c7002afab52114dd89f1407d2d8ebe4af7d`
- 代码来自源提交，架构文档与开发计划来自本次完成版本。
- 未迁移用户状态、运行证据、临时文件和无关工作区修改。

## 工程结构

采用多 crate 工作区，并使用与 smolvm 相同的根目录依赖子模块布局：

- `libkrun`：`kense-labs/libkrun` 的 Git submodule。
- `libkrunfw`：`kense-labs/libkrunfw` 的 Git submodule，固定为 `v5.3.0` 源码。
- `crates/box`：Box 执行资源与 MicroVM 生命周期。
- `crates/oci-runtime`：OCI 工作负载运行时。
- `crates/runtime`：Task／Service 合同和 Runtime 控制接口。
- `crates/sentry`：行为判定与干预。
- `apps/gateway`：模型 Provider 网关。

## 构建

```bash
git clone --recursive https://github.com/kense-labs/krun.git
cd krun
cargo check --workspace --locked
cargo test --locked -p aster-runtime -p aster-box-core -p aster-sentry
```

工具链见 `rust-toolchain.toml`。所有 crate 保持原包名，`publish = false` 的组件仍只在本地解析。

## 文档

- [Agent Runtime 架构](docs/architecture/agent-paas-runtime.md)
- [Aster Box 优化与 Agent Runtime 开发计划](docs/aster-box-smolvm-inspired-development-plan.md)
- [smolvm 与 Aster Box 对比](smolvm-vs-aster-box-report.md)
- [多 Harness 协作架构](docs/architecture/harness-gateway.md)
- [沙箱、观测与干预架构](docs/architecture/sandbox-control.md)

当前迁移不包含 Agent Runtime v1.1 中尚未实现的目标能力。实际支持范围以源码和测试为准。

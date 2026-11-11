# 三套 ACP Harness 验收与架构规划：当前进度

日期：2026-11-10。用户将当前优先事项调整为先规划共享能力底座，最终名称为 KRun。当前架构入口为 [KRun 规划](../../../architecture/krun.md)。本文不是三套 Harness 全部通过的验收报告。

## 实际结果

| 项目 | 结果 | 证据 |
| --- | --- | --- |
| DeepSeek 共用任务矩阵 | 9 PASS，0 FAIL／SKIP／BLOCKED | `deepseek-attempt3/results.json` |
| DeepSeek MCP stdio + 真实模型工具调用 | PASS，1 次工具调用，清理后无运行实例 | `deepseek-mcp-attempt2/results.json` |
| ACP 客户端终端服务 | PASS：实时输出、UTF-8 尾部截断、进程组终止、6 秒迟发写观察 | `client-terminal-services.json` |
| Aster 初次完整任务矩阵 | 6 PASS、3 FAIL；在取消未收齐工具终态后停止，后续两项未完成 | `aster-attempt1/results.json` |
| Aster 取消回归修复 | 修复前 1 FAIL，修复后目标用例 1 PASS；相关测试 16 PASS | `aster-cancel-regression-before.log`、`aster-cancel-regression-after.log`、`aster-cancel-related-tests2.log` |
| Aster clippy | PASS，有现有警告 | `aster-cancel-clippy.log` |
| Aster 工作区格式检查 | FAIL；存在工作区格式差异，未批量重排文件 | `aster-format-check.log` |
| Aster 配套 Linux 构建与包校验 | 初次版本 PASS；CLI／ACP BUILD_ID 一致，glibc ≤ 2.31 | `aster-kbox-build.log`、`aster-package.json` |
| Codex 官方源码 Linux 构建 | 32 GB、2 个 Cargo job 重试后 PASS | `codex-kbox-build32.log` |
| codex-acp adapter 构建 | PASS | `codex-adapter-kbox-build.log` |
| bubblewrap 客体能力探测 | PASS，未启用放宽安全选项 | `aster-bwrap-probe.log` |
| Aster 在 bookworm 候选底座的动态库探测 | PASS，无缺失库 | `shared-base-aster-abi.log` |
| 修复后的 Aster Linux CLI／ACP 配套包 | PASS，CLI 0.3.4、ACP 9.2.2、BUILD_ID 一致 | 独立 `krun/.build/logs/aster-build.log` |
| 三种版本化 ZIP 在共同 kbox 底座启动 | PASS：Aster 0.1.0、Dsh 0.1.0、Codex 0.1.1；含真实 ACP initialize 和 EOF 退出 | [KRun 制品验证记录](../../../../../krun/docs/evidence/2026-11-10-harness-packages/OUTCOME.md) |
| Codex adapter EOF 回归 | 修复前 1 FAIL；修复后 11 PASS，typecheck 与构建通过 | KRun 验证记录中的补丁和红／绿日志 |

成功构建与上述运行均由 kbox 执行。早先调用 Aster 的 OrbStack 外层脚本因客体内缺少 Docker 立即失败，日志保留为 `aster-source-build.log`；之后使用 kbox 直接执行其 Linux 内层构建命令，没有使用 OrbStack 或 Docker 执行成功构建。

## 来源与制品

- kbox 基线：`0304123f562ce7264954a8a104dced9e19b3d483`，加前序 stdout 协议污染修复。
- DeepSeek：`kense-labs/dsh-acp`，`ba01b05988c3cd0c982ca71069efd242f813d6c4`；镜像 `dsh-acp:e2e-ba01b05`，摘要 `sha256:5d8c283ef8cc74d4f6b76f1bbb585cb0758f26eb7a898201bc65502b9e0c6f7b`。
- Aster 初次构建：用户指定本地仓库的独立源码快照，含当时未提交修改；基于 `b1be95c5b7d6de38c1528703f0e8f4e195c579f7`。快照记录在 `aster-source-snapshot.json`。配套 BUILD_ID 为 `aster-cli-pair-v1:73b1d5b5ca5b9ca39f1496d3ac18fbf638bc47e9:aarch64-unknown-linux-gnu`。
- Aster 验收期间原仓库推进到 `4e9f22cb634e43607d601995055edc6c19d421d0`。取消修复在当前工作区中完成，未覆盖或回退其他工作。
- Codex：用户指定的 `org-14957082@github.com:openai/codex.git`，`36e074a2fb365aa788886495c82e6ea612226cd9`。
- ACP adapter：经用户选择采用独立 adapter；`agentclientprotocol/codex-acp`，`ab7a968e982a95d8a765c1e6c6682a520d4f7638`，2.6.0。将通过 `CODEX_PATH` 绑定官方源码产物。

失败尝试保留：DeepSeek 首次新入口缺少 Python `tomllib`，第二次错误覆盖了 Messages API 地址；均未算通过。MCP 首次输入使用相对命令路径被明确拒绝，改为真实绝对路径后通过。Codex 先遇到 Python 3.8 不支持源码工具、Git 目录信任检查和 16 GB 构建中的 rustc SIGKILL，日志分别保留；SIGKILL 原因未获内核证据，不将其宣称为已确认 OOM。

## 复现入口

```text
python3 scripts/e2e-acp-harnesses/run.py --harness <aster|deepseek|codex> --image <tag> --out <new-directory> --image-store <shared-image-store> [--provider-config <KCL>] [--model <id>]
python3 -m unittest discover -s scripts/e2e-acp-harnesses -p 'test_*.py'
python3 -m unittest discover -s scripts/e2e-dsh-acp -p test_harness.py
```

`check_mcp.py` 使用相同 Harness profile；`check_services.py` 对已有测试 VM 执行真实终端服务检查。独立 KRun 仓库已包含实际 ZIP 和共同底座的测试安装入口；正式 CLI、MateHarness 动态装配和能力绑定仍属于规划。早期任务验收脚本的 Harness 镜像不作为最终安装包交付。

## Explicitly not claimed

- 尚未宣称三套 Harness 全部验收通过。
- Aster 取消修复已经重建 Linux 配套 ZIP，但尚未重新完成真实模型矩阵。
- Codex 尚未完成真实 ACP 任务和模型调用验收。
- 共同底座与 ZIP 安装已通过隔离测试入口验证；正式 KRun CLI、MateHarness 动态装配、五类能力绑定和独立控制通道凭据交付尚未实现。
- 当前脚本中 stdin 凭据前置处理仅是测试 fixture，不是最终 ACP 安装包入口；最终架构要求凭据走独立控制通道。
- 未宣称 macOS 宿主只读 virtio-fs 挂载可用，也未宣称 GPU 设备透传、Linux 宿主或 Windows 原生验收通过。
- 工作区修改未提交或推送。

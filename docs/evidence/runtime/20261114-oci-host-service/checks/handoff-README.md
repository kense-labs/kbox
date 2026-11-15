# kbox 智能体工作交接

完整任务见 [TASK.md](TASK.md)。开发工作目录是 `/Users/roylin/code/kense/kbox`。

将以下提示词发送给另一个具备该工作区访问权限的智能体：

```text
请读取 /Users/roylin/code/kense/kbox-agent-handoff/TASK.md，并在 /Users/roylin/code/kense/kbox 中执行其中的任务。

先核对当前工作区、仓库规则与已有证据，保留所有未提交修改和历史产物。任务中的 OCI 文档位于 crates/oci-runtime/ 下，包括 README.md、ROADMAP.md 和 docs/macos-hvf.md。

直接开始实施，按任务文件连续完成代码、测试、原生验收和证据记录，不要只给计划，也不要每完成一步就等待我确认。遇到外部阻塞时记录缺失条件，继续其他可推进项；只有任务完成或全部剩余工作确被外部条件阻塞时才收尾。

不提交、推送或发布。完成后报告实际修改、验证结果、证据位置和剩余阻塞。
```

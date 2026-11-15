# Overall goals (k-goal · S)

Updated: 2026-11-16 · by: S via /k-goal  
Status: **locked**

## Overall goal

KBox 是面向用户的 Docker-shaped MicroVM 运行时：用同一 CLI 拉取/运行 OCI 负载于专用 MicroVM，并管理卷、网络、compose、快照与相关生命周期。执行后端为 **libkrun**（含配套 guest 内核包）。「做成」= 真实用户日常可装可用（默认走 libkrun 能干活）。

## Stretch (within product)

主推平台同一产品门禁可复跑；无对应原生机/无证据不得记产品 Pass；77≠Pass。

## Near-term hard goals

1. 默认路径：主推平台安装后，默认走 libkrun 可日常完成 OCI→MicroVM 主路径。
2. 证据：原生机可复跑门禁与安装/冒烟回执；门禁绿或 exit 77 不得单独当作产品 Pass。

## Non-goals

- 另做新产品或「任意仓库编排」类平台
- 用 accept/门禁绿替代「日常可装可用」作为做成定义
- 本阶段更换默认执行后端或引入第二默认后端

## Real constraints (not self-doubt)

- 执行后端：libkrun（恢复为唯一默认路径）
- 产品面不扩大

## Open turtle-soup

- R1: 总体目标改回同一 KBox + libkrun 为执行后端；本阶段不写入其它默认后端？ → A: **是**
- R2: 「做成」仍为真实用户日常可装可用（默认 libkrun），门禁绿 ≠ Pass？ → A: **是**（沿用上轮；本轮用户要求清掉仓库内全部替代默认后端表述并重锁）
- 用户指令：清理仓库中全部相关替代默认后端内容 → 已执行（此前仅出现在本文件；已重写为 libkrun 锁定稿）
- （已关闭；有新边界再开）

## Sources

- user: `/k-goal 去掉…默认后端回到原来的libkrun` → R1 是
- user: `清理所有kbox中关于…的内容`（清替代默认后端表述并重锁）
- prior: 「做成」= 日常可装可用（上轮 A）
- sitrep: bootstrap；≠Pass

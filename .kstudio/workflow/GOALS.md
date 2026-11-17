# Overall goals (k-goal · S)

Updated: 2026-11-17 · by: S via /k-goal  
Status: **locked**

## Overall goal

KBox 是面向用户的 Docker-shaped 运行时：用同一 CLI 拉取/运行 OCI 负载，并管理卷、网络、compose、快照与相关生命周期。隔离分级：默认 **sandbox**（运行后端由指定上游 Apache-2.0 容器引擎源码融合进本仓；代码与文档零上游产品品牌残留）；更高隔离可选 **libkrun / MicroVM**。「做成」= 真实用户日常可装可用（默认走 sandbox 能干活）。

## Stretch (within product)

主推平台同一产品门禁可复跑；无对应原生机/无证据不得记产品 Pass；77≠Pass。sandbox 默认与可选 MicroVM 路径均须可验证，不得用门禁绿冒充日常可用。

## Near-term hard goals

1. 默认路径：主推平台安装后，默认走 sandbox 隔离后端，可日常完成 OCI 主路径；代码/文档/CLI/包名零上游产品品牌残留。
2. 融合：指定上游引擎能力以 kbox 内 sandbox 后端落地（非并列品牌产品）。
3. 可选升级：同一产品内可选用 libkrun/MicroVM 作为更高隔离级别（非默认）。
4. 证据：原生机可复跑门禁与安装/冒烟回执；门禁绿或 exit 77 不得单独当作产品 Pass。

## Non-goals

- 另做新产品、并列上游品牌产品，或「任意仓库编排」类平台
- 用 accept/门禁绿替代「日常可装可用」作为做成定义
- 删除 MicroVM/libkrun 产品能力（本轮只改默认，不砍更高隔离）
- 在代码或文档中保留上游产品品牌字符串

## Real constraints (not self-doubt)

- 默认执行路径：sandbox（融合自指定上游源码，无品牌）
- 更高隔离：libkrun/MicroVM 可选保留
- 产品面不扩大：仍是同一 KBox
- 品牌：代码与文档零上游产品品牌残留（门禁脚本可复跑）

## Open turtle-soup

- R1: 总体目标改回同一 KBox + libkrun 为执行后端；本阶段不写入其它默认后端？ → A: **是**（上轮；**已被本轮 R3 推翻**）
- R2: 「做成」仍为真实用户日常可装可用（默认 libkrun），门禁绿 ≠ Pass？ → A: **是**（上轮；默认后端口径由 R4 更新为 sandbox）
- **R3**: 默认改为 sandbox（融指定上游引擎、去品牌），libkrun/MicroVM 更高隔离可选保留？ → A: **是**
- **R4**: 「做成」仍为日常可装可用（默认 sandbox），门禁绿 / 77 ≠ Pass？ → A: **是**
- **R5（本轮）**: 代码与文档是否要求零上游产品品牌残留（含规格/证据/NOTICE 文案）？ → A: **是**
- （已关闭；有新边界再开）

## Sources

- user: `/k-goal` 融合指定上游引擎为 sandbox 默认后端并去品牌
- user: `/k-yolo` 追加「代码和文档中不要有任何上游产品品牌残留」
- user: R3 → **是**；R4 → **是**；R5 → **是**
- prior locked: libkrun 唯一默认（2026-11-16；本轮改默认）
- sitrep: sandbox-isolation；≠Pass

# reports/ —— 交付与施工记录库

> **12 册，合订自 121 篇散篇**。这里是 **L5 卷宗层**：当时的交付记录，**不是现状真源**。
> 要问「某项能力现在到哪一级」，查 [`docs/status/claims.yaml`](../docs/status/claims.yaml) 与
> [`docs/03-reference/test-baseline.md`](../docs/03-reference/test-baseline.md)；本库只回答「当时交了什么、怎么验的」。

## 我要什么 → 去哪

| 我想知道 | 去哪 |
|---|---|
| 某个角色当时交了什么 | 册 01 |
| 一次评审/审查的结论 | 册 02 / 册 03 / 册 04 |
| 性能或安全的实测数字 | 册 05 / 册 06 |
| 某篇待评估材料的吸收建议 | 册 07 |
| 架构收敛与契约演进 | 册 08 |
| 某轮派了什么活、拍了什么板 | 册 09 / 册 11 |
| 桌面与界面落地 | 册 10 |
| 零散交付记事 | 册 12 |

## 十二册

| 册 | 合订篇数 | 是什么 |
|---|---|---|
| [`01-role-and-achievement-reports.md`](01-role-and-achievement-reports.md) | 24 | 按角色归档的成就报告（架构 / 后端 / 前端全栈 / QA / 代码审查…） |
| [`02-omnibus-and-batch-handoff.md`](02-omnibus-and-batch-handoff.md) | 7 | R12 附录 N 四路同行评审 + M2 同行审查 + 批次交接 |
| [`03-audit-and-verification.md`](03-audit-and-verification.md) | 14 | 2026-08~09 只读审计卷宗（文档陈旧度 / 代码质量 / 仓库卫生 / 零测试候选） |
| [`04-evaluation-and-guard.md`](04-evaluation-and-guard.md) | 7 | Guard 3.0→3.3 四轮安全守门演进 + 评估完整性 |
| [`05-benchmark-and-performance.md`](05-benchmark-and-performance.md) | 7 | 性能线实测卷宗（基准基线 / 复现线 / 冷启动与沙箱） |
| [`06-security-and-sandbox.md`](06-security-and-sandbox.md) | 11 | 安全线验收与修复闭环（V13 验收 → V14 修复 → rc.1 全量审计） |
| [`07-research-assessments.md`](07-research-assessments.md) | 12 | TP30 待评估清单逐件评估（机制 / 对照 / 吸收建议 / 0 装标注） |
| [`08-capability-and-architecture.md`](08-capability-and-architecture.md) | 10 | 架构线收敛与验收（认知基础设施 / Memory 2.x / 桌面↔后端契约） |
| [`09-round-dispatch-decision-ledger.md`](09-round-dispatch-decision-ledger.md) | 16 | 轮次派发单与决策记录（派了什么活 / 拍了个什么板） |
| [`10-desktop-and-ui.md`](10-desktop-and-ui.md) | 3 | 桌面端与界面侧合成落地（含设置实时生效契约） |
| [`11-milestone-and-decision-records.md`](11-milestone-and-decision-records.md) | 8 | 里程碑级裁决与验收（V25/V26 集成验证 / SDK 砍填 / 默认落地裁定） |
| [`12-misc-delivery-notes.md`](12-misc-delivery-notes.md) | 2 | 零散交付记事（许可声明 / 模型实测对比 / SDK crate 决策 / 目录旧索引） |

**合计 121 篇合订**。每册内部按「目录表 → 逐篇摘录 → 归属链接」编排；
逐篇原件**逐字归档**在 `docs/archive/merged/2026-10-23/reports/`，要读原文点归属。

## 这一层是什么 / 什么不是

- **是**：带日期的施工与交付记录（L5 卷宗）。数字是**当时实测值**，按「历史不追溯改写」保留原样。
- **不是**：现状真源、规格、操作手册。
- **边界**：面向施工者的过程材料在 [`docs/04-internal/`](../docs/04-internal/README.md)；
  面向用户的说明在 [`docs/02-guides/`](../docs/02-guides/README.md)；事实真源在 [`docs/03-reference/`](../docs/03-reference/README.md)。

## 怎么查

1. 先看上面「我要什么 → 去哪」定位到册；
2. 册内先读**目录表**（原标题 / 原日期 / 行数），再看该节摘录；
3. 要原文细节 → 点该节的**归属**，直达归档原件（逐字）。

## 非文档文件

`reports/` 下另有 74 个非 `.md` 文件（UUID 命名的工作日志等），按证据留存策略 **保留原位**，
不参与合订、不删除、不移位。它们是 F-02 大资产出库批的留存面，见册 12 与
`docs/04-internal/evidence-retention-policy.md`。

---

**变更历史**：
- 2026-10-23 首版合订：121 篇散篇 → 12 册，原件逐字归档 + 原路径留桩。
- 2026-10-24 修复：**本文件此前被误当成「121 篇之一」归档进册 12**，导致 `reports/` 失去入口索引；
  现恢复为册目录索引（原 `reports/README.md` 仍逐字留在册 12 与归档区，记录未丢）。

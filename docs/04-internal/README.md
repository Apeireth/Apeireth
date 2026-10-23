# docs/04-internal — 在飞工作面

> **性质**：**L4 在飞**（有明确终点的施工面）+ 少量**L3 常青**（宪章与手册）。
> **不是什么**：不是架构说明（那在 `../01-architecture/`）、不是决策记录（那在 `../adr/`）、不是历史卷宗（那在 `../archive/` 与本目录的 `history/`）。
> **机器可读全量清单**：[`../INDEX.csv`](../INDEX.csv)（含层/题材/日期/被引用数/处置）。

---

## 0. 我要什么 → 去哪

| 我要做的事 | 去哪 |
|---|---|
| **开工前读宪章** | [ENGINEER-MANIFESTO.md](ENGINEER-MANIFESTO.md)（工程总纲：0 装诚实、O-6 三阶审查、提交格式）+ [backend-enhancement/CONSTRUCTION-DISCIPLINE.md](backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（并发施工宪章） |
| **查现在的债 / 待办** | [backend-enhancement/00-backlog.md](backend-enhancement/00-backlog.md) —— **D 编号唯一真源** |
| **查系统级缺口** | [backend-enhancement/system-gap-ledger-2026-10-18.md](backend-enhancement/system-gap-ledger-2026-10-18.md)（SG/NT/NF） |
| **查能力接线现状** | [cognitive-module-wiring.md](cognitive-module-wiring.md)（全仓引用最高的活文档，125 处） |
| **查未接线能力** | [`../status/unwired-ledger.md`](../status/unwired-ledger.md)（W1–W7） |
| **查某批交付了什么** | [handover-ledger.md](handover-ledger.md)（10 篇按批次交接稿的索引） |
| **查我们审过什么** | [audits/](audits/README.md)（4 册） |
| **查历史考古** | [history/](history/)（2 册：阶段施工卷宗 / v1→v2 考古卷宗） |
| **查踩过的坑 / 可复用做法** | [engineering-experience/](engineering-experience/)（`01-collaboration-patterns` = 经验，`02-failure-anatomy` = 教训） |
| **发布前检查** | [release-checklist.md](release-checklist.md) + [`../packaging/release-asset-verification.md`](../packaging/release-asset-verification.md) |

---

## 1. 子目录

| 目录 | 内容 | 层 |
|---|---|---|
| [backend-enhancement/](backend-enhancement/README.md) | 后端增强施工台账：轴 A–P、债栏、缺口账、交付册 | L4 |
| [audits/](audits/README.md) | 审计与核对结论册（架构能力 / 安全治理 / 口径文本） | L5 |
| [history/](history/) | 历史卷宗：阶段施工记录、v1→v2 研究与审计卷宗 | L5 |
| [engineering-experience/](engineering-experience/) | 经验与教训的**提炼产物**（不是过程记录） | L3 |
| [philosophy-study/](philosophy-study/) | 哲学研究（主综述 + 6 个家族研究） | L3 |
| [license-port/](license-port/) | 许可搬运四关（gate1–gate4） | L3 |

---

## 2. 顶层文件分组

### 2.1 宪章与纪律（**只增不改**，改前先看 O-6 三阶审查）

| 文件 | 管什么 |
|---|---|
| [ENGINEER-MANIFESTO.md](ENGINEER-MANIFESTO.md) | 工程总纲：0 装诚实真账、O-6 三阶审查、提交格式、LOCKED 5 项 |
| [backend-enhancement/CONSTRUCTION-DISCIPANCE.md](backend-enhancement/CONSTRUCTION-DISCIPLINE.md) | 多 AI 并发施工宪章：中性表述豁免表、文件归属矩阵 |
| [evidence-retention-policy.md](evidence-retention-policy.md) | 证据留存策略（过程证据不入 git，只入结论） |
| [secret-management-policy.md](secret-management-policy.md) | 密钥与凭据处理 |
| [async-trait-decision-matrix.md](async-trait-decision-matrix.md) | 异步 trait 选型决策矩阵 |

### 2.2 台账与现状（**数字只认这里**）

| 文件 | 是什么 |
|---|---|
| [cognitive-module-wiring.md](cognitive-module-wiring.md) | 认知模块接线现状（**全仓引用最高，125 处**） |
| [claims-evidence-matrix.md](claims-evidence-matrix.md) | 声明 ↔ 证据对照 |
| [live-verification-ledger.md](live-verification-ledger.md) | 实测验收台账 |
| [absorption-ledger.md](absorption-ledger.md) | 吸收账 |
| [design-intent.md](design-intent.md) | 设计意图与拍板历史 |

### 2.3 手册与操作（L3 常青）

[maintenance-guide.md](maintenance-guide.md)｜[kani-verification-playbook.md](kani-verification-playbook.md)｜[ops-network-playbook.md](ops-network-playbook.md)｜[plugin-authoring-guide.md](plugin-authoring-guide.md)｜[github-fetch-guide.md](github-fetch-guide.md)｜[migration-v1-to-v2.md](migration-v1-to-v2.md)｜[release-checklist.md](release-checklist.md)｜[release-plan.md](release-plan.md)

### 2.4 审计与核对

[capability-knob-audit-2026-09-12.md](capability-knob-audit-2026-09-12.md)｜[gitignore-hygiene-audit-2026-08-28.md](gitignore-hygiene-audit-2026-08-28.md)｜[governance-chain-drill-2026-10-20.md](governance-chain-drill-2026-10-20.md)｜[v1-vs-v2-capability-gap-audit-2026-10-06.md](v1-vs-v2-capability-gap-audit-2026-10-06.md)｜[w3-legacy-gap-archaeology-2026-10-10.md](w3-legacy-gap-archaeology-2026-10-10.md)

### 2.5 工程日志（按日/按次记，不改写）

[engineering-log-2026-09-22.md](engineering-log-2026-09-22.md)｜[engineering-log-2026-10-06.md](engineering-log-2026-10-06.md)｜[ci-fix-log-2026-08.md](ci-fix-log-2026-08.md)｜[o6-session-log-2026-08-27.md](o6-session-log-2026-08-27.md)

### 2.6 批次交接（**按批次查，不要逐篇读**）

索引：[handover-ledger.md](handover-ledger.md)。
原件（10 篇，保留原位按批次定位）：[FINAL-HANDOFF-V2.0.0-RC.1.md](FINAL-HANDOFF-V2.0.0-RC.1.md)｜[handoff-log-2026-08-28-mavis.md](handoff-log-2026-08-28-mavis.md)｜[handoff-18crate-ci-fix-2026-09-23.md](handoff-18crate-ci-fix-2026-09-23.md)｜[handoff-k3-frontend-2026-09-23.md](handoff-k3-frontend-2026-09-23.md)｜[handoff-w2-wiring-2026-10-06.md](handoff-w2-wiring-2026-10-06.md)｜[engineering-review-handoff-2026-10-06.md](engineering-review-handoff-2026-10-06.md)｜[handoff-w1-w5-closure-2026-10-10.md](handoff-w1-w5-closure-2026-10-10.md)｜[handoff-pet-avatar-line-2026-10-11.md](handoff-pet-avatar-line-2026-10-11.md)

> 通用接手方法论已归并进 [`../03-reference/team-handover-reference.md`](../03-reference/team-handover-reference.md)《接手一站式手册》——**那才是每次接手都该读的那一篇**；本目录的交接稿只回答"某一批当时做了什么"。

### 2.7 设计与规格

[design-task-lease-2026-08-21.md](design-task-lease-2026-08-21.md)｜[design-three-way-conflict-2026-08-21.md](design-three-way-conflict-2026-08-21.md)｜[scene-d-v2-plan.md](scene-d-v2-plan.md)｜[stored-doc-consumer-migration.md](stored-doc-consumer-migration.md)｜[v2.0.0-release-path.md](v2.0.0-release-path.md)｜[v2.0.0-release-path-integration.md](v2.0.0-release-path-integration.md)｜[v2.0.0-release-path-7-spec-4-block.md](v2.0.0-release-path-7-spec-4-block.md)｜[v2.0.0-rc-roadmap.md](v2.0.0-rc-roadmap.md)

### 2.8 编号考古稿（2026-08 批次，**已被 history/ 册收口**）

`round-*` / `r9-r10-*` / `r11-*` / `r12-*` / `r12-r13-r14-*` / `youyou-list-*` / `stage123-gap-matrix` / `v2-reference-handbook-2026-08-28` / `v2-unabsorbed-features` / `apeireth-1-0-vs-2-0-functional-gap-2026-08-28` 等 → 阅读入口：[history/02-v1-to-v2-research-and-audit-dossier.md](history/02-v1-to-v2-research-and-audit-dossier.md)

### 2.9 研究与调研

[ai-os-landscape-2026-10-18.md](ai-os-landscape-2026-10-18.md)｜[memory-research.md](memory-research.md)｜[absorption-candidates-2026-10.md](absorption-candidates-2026-10.md)｜[upgrade-research-pack.md](upgrade-research-pack.md)｜[b-block-decision-points-research-2026-08-28.md](b-block-decision-points-research-2026-08-28.md)｜[b-block-frontend-readiness-2026-08-28.md](b-block-frontend-readiness-2026-08-28.md)｜[b-block-gateway-sse-research-2026-08-28.md](b-block-gateway-sse-research-2026-08-28.md)｜[frontend-reference-deepseek-harness-2026-09.md](frontend-reference-deepseek-harness-2026-09.md)｜[ref-crawler-research.md](ref-crawler-research.md)｜[ref-gh-accel.md](ref-gh-accel.md)｜[ref-hydra.md](ref-hydra.md)｜[ref-yoyo-evolve.md](ref-yoyo-evolve.md)｜[borrow-from-jimmyxiao2009.md](borrow-from-jimmyxiao2009.md)｜[pm-review-productization-2026-09-25.md](pm-review-productization-2026-09-25.md)｜[A-block-o6-true-account.md](A-block-o6-true-account.md)｜[9-organ-progress-2026-08-28.md](9-organ-progress-2026-08-28.md)｜[team-work-doc.md](team-work-doc.md)｜[v2-rc-1-progress-report.md](v2-rc-1-progress-report.md)

---

## 3. 纪律

1. **债只记一处**：`backend-enhancement/00-backlog.md` 的 **D 编号**是唯一真源；本目录其它文档里的"待办"都是**引用**。
2. **数字只认真源**：测试数字 → [`../03-reference/test-baseline.md`](../03-reference/test-baseline.md)；能力状态 → [`../status/claims.yaml`](../status/claims.yaml)。
3. **历史不追溯改写**：带日期的记录保留原值；更正用「历史口径（日期）」标注，不改原句。
4. **0 装诚实**：说"已接线"必须标四级口径；默认关的能力不得写成系统现状。
5. **归档优先于合并**：L5 内容要移出本目录就**逐字 `git mv` 进 `../archive/` 并留桩**，不要"合并改写"。

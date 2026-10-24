# 04-internal —— 内部工作区

> **这一层是 L4 在飞 + L5 卷宗**：内部过程材料。读者是**施工的人**，不是使用者。
> 面向用户的说明在 [`../02-guides/`](../02-guides/)；面向事实的单一真源在 [`../03-reference/`](../03-reference/)。

## 我要什么 → 去哪

| 我想知道 | 去哪 |
|---|---|
| 规矩是什么、谁定的 | [`00-governance/`](00-governance/README.md) |
| 上一批做到哪、坑在哪 | [`01-handoffs/`](01-handoffs/README.md) |
| 外部怎么做、我们缺什么 | [`02-research/`](02-research/README.md) |
| 哪里不合口径、账对不对 | [`03-audits/`](03-audits/README.md) |
| 当时怎么做的 | [`04-logs/`](04-logs/README.md) |
| 下一步做什么 | [`05-plans/`](05-plans/README.md) |
| 交接现场（合并稿） | [`../03-reference/team-handover-reference.md`](../03-reference/team-handover-reference.md) |
| 批次交接索引 | [`handover-ledger.md`](handover-ledger.md) |

## 台账地图

| 目录 | 是什么 | 篇数 |
|---|---|---|
| [`00-governance/`](00-governance/README.md) | 规矩是什么、谁定的：宪章、政策、手册、决策矩阵、台账 | 6 |
| [`01-handoffs/`](01-handoffs/README.md) | 上一手留给下一手的现场：批次交接稿 | 8 |
| [`02-research/`](02-research/README.md) | 外部输入：调研稿、参考手册、考古与差距研究 | 21 |
| [`03-audits/`](03-audits/README.md) | 对内核对：审计稿、差距矩阵、复核与体检 | 12 |
| [`04-logs/`](04-logs/README.md) | 流水不是结论：工程日志与工作记录 | 2 |
| [`05-plans/`](05-plans/README.md) | 打算做什么：计划、路线图、迁移方案、进度报告 | 18 |

### 顶层留位（20 篇）—— **机器引用，必须留原位**

顶层共 **88 个条目** = **1 篇目录索引（本文件）+ 20 篇留原位的真文件**（下表）+ **67 个 4 行重定向桩**（移位件的原路径，点进去即到新址，见各子目录索引）。

这 20 篇被 **CI workflow / 仓内脚本 / `crates/` 源码路径** 直接指到，
移走会让脚本读到**桩**而不是真文件（静默改行为）。故**保留原路径**，不进子目录。

| 篇 | 为什么留原位 |
|---|---|
| [`ENGINEER-MANIFESTO.md`](ENGINEER-MANIFESTO.md) | 工程总纲（多处引用；AGENTS.md 指向） |
| [`HANDOFF-NOTES.md`](HANDOFF-NOTES.md) | 接手稿（合并进接手手册，但引用点仍在） |
| [`TO-NEW-TEAM.md`](TO-NEW-TEAM.md) | 接手稿（合并进接手手册，但引用点仍在） |
| [`ai-os-landscape-2026-10-18.md`](ai-os-landscape-2026-10-18.md) | 中性词检查脚本引用 |
| [`async-trait-decision-matrix.md`](async-trait-decision-matrix.md) | `crates/` 源码注释引用 |
| [`ci-fix-log-2026-08.md`](ci-fix-log-2026-08.md) | CI workflow 引用 |
| [`cognitive-module-wiring.md`](cognitive-module-wiring.md) | 接手手册引用 |
| [`engineering-review-handoff-2026-10-06.md`](engineering-review-handoff-2026-10-06.md) | **口径门禁脚本直接读它** |
| [`github-fetch-guide.md`](github-fetch-guide.md) | 取材通道指南（AGENTS.md §4.5 指向） |
| [`governance-chain-drill-2026-10-20.md`](governance-chain-drill-2026-10-20.md) | `claims.yaml` 证据引用 |
| [`kani-verification-playbook.md`](kani-verification-playbook.md) | kani CI workflow 引用 |
| [`live-verification-ledger.md`](live-verification-ledger.md) | **单一真源级**：口径基线 + 多处引用 |
| [`maintenance-guide.md`](maintenance-guide.md) | `crates/` 内引用 |
| [`next-team-handbook.md`](next-team-handbook.md) | 接手手册（引用点多） |
| [`o6-session-log-2026-08-27.md`](o6-session-log-2026-08-27.md) | 接手手册引用 |
| [`scene-d-v2-plan.md`](scene-d-v2-plan.md) | `crates/` 源码注释引用 |
| [`secret-management-policy.md`](secret-management-policy.md) | CI workflow + hook 安装脚本引用 |
| [`v1-vs-v2-capability-gap-audit-2026-10-06.md`](v1-vs-v2-capability-gap-audit-2026-10-06.md) | `crates/` 源码注释引用 |
| [`v2-unabsorbed-features.md`](v2-unabsorbed-features.md) | `crates/` 源码注释引用 |
| [`v2.0.0-rc-roadmap.md`](v2.0.0-rc-roadmap.md) | `crates/` 源码注释引用 |

## 纪律

- **只增不改**：带日期的历史记录保留原值，需要时加「历史口径（日期）」标注。
- **受保护资产**：`docs/adr/` 正文、契约与真源（`claims.yaml` / `test-baseline.md` / `wiring.yaml` / `verification-ledger.yaml` / `contract.toml`）**不由本目录流程改写**。
- **移位留桩**：本批移位 67 篇，原路径留 4 行重定向桩 + 登记 [`../_redirects.tsv`](../_redirects.tsv)（238 条），既有链接零破坏。
- **四级口径**：本目录文档声称「已实现 / 已接线」时必须标四级级别（IMPLEMENTED / PRODUCTION WIRED / HARDWARE VALIDATED），口径门禁 `scripts/check_doc_caliber.py` 会拦。

## 门禁

| 门禁 | 命令 |
|---|---|
| 链接完整性 | `python3 scripts/docs_links.py --check` |
| 索引分层 | `python3 scripts/docs_index.py --check` |
| 四级口径 | `python3 scripts/check_doc_caliber.py` |

---

**变更历史**：2026-10-24 文档治理后续批 —— 顶层 88 篇按题材重排为 6 个子目录 + 20 篇留原位（机器引用）+ 67 个 4 行桩；移位 67 篇全部留桩登记重定向，入链修复 52 处。

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
| [`05-plans/`](05-plans/README.md) | 打算做什么：计划、路线图、迁移方案、进度报告 | 17 |

## 顶层构成：88 个条目 = 1 篇索引 + 17 篇真文件 + 70 个桩

**移位不减少文件数** —— 减少的是「要读几篇」和「能不能按题材找」。桩是「旧链接仍可达」的代价。

- **70 个桩**：67 个由本批**移位**生成（每个 4 行 = 3 行正文 + 末尾空行）+ 3 个是更早批次的**合并/归档**遗留桩
  （`HANDOFF-NOTES.md` / `TO-NEW-TEAM.md` / `next-team-handbook.md`，5 行）。点进去即到新址。
- **17 篇真文件**：见下表。

### 为什么这 17 篇留原位

| 篇 | 为什么留原位 |
|---|---|
| [`ai-os-landscape-2026-10-18.md`](ai-os-landscape-2026-10-18.md) | 中性词检查脚本 `scripts/check-neutral-terms.ps1` 引用 |
| [`async-trait-decision-matrix.md`](async-trait-decision-matrix.md) | `crates/foundation/plugin/src/llm_factory.rs` 源码注释引用 |
| [`ci-fix-log-2026-08.md`](ci-fix-log-2026-08.md) | `.github/workflows/`（pii-leak-detection / release-prep）引用 |
| [`cognitive-module-wiring.md`](cognitive-module-wiring.md) | 接手手册引用 |
| [`engineering-review-handoff-2026-10-06.md`](engineering-review-handoff-2026-10-06.md) | 口径门禁 workflow 与 `check_doc_caliber.py` 的**注释/出处**引用（不是脚本读取该文件） |
| [`governance-chain-drill-2026-10-20.md`](governance-chain-drill-2026-10-20.md) | `docs/status/claims.yaml` 证据引用 |
| [`kani-verification-playbook.md`](kani-verification-playbook.md) | `.github/workflows/kani.yml` 引用 |
| [`live-verification-ledger.md`](live-verification-ledger.md) | **真源级**：口径基线 + 多处引用 |
| [`maintenance-guide.md`](maintenance-guide.md) | `crates/foundation/credentials/Cargo.toml` 引用 |
| [`scene-d-v2-plan.md`](scene-d-v2-plan.md) | `crates/foundation/core/src/philosophy.rs` 等源码注释引用 |
| [`secret-management-policy.md`](secret-management-policy.md) | `.github/workflows/rust.yml` + `scripts/install-pre-commit-hook.ps1` 引用 |
| [`v1-vs-v2-capability-gap-audit-2026-10-06.md`](v1-vs-v2-capability-gap-audit-2026-10-06.md) | `crates/` 源码注释引用 |
| [`v2-unabsorbed-features.md`](v2-unabsorbed-features.md) | `crates/foundation/core/src/` 源码注释引用 |
| [`v2.0.0-rc-roadmap.md`](v2.0.0-rc-roadmap.md) | `crates/foundation/plugin/src/perception_backend.rs` 引用 |
| [`ENGINEER-MANIFESTO.md`](ENGINEER-MANIFESTO.md) | **地标**：`AGENTS.md` 与施工纪律指向它（文档引用，非机器引用） |
| [`github-fetch-guide.md`](github-fetch-guide.md) | **地标**：`AGENTS.md` §4.5 取材通道指向它（文档引用） |
| [`o6-session-log-2026-08-27.md`](o6-session-log-2026-08-27.md) | 接手手册引用（文档引用） |

> 前 14 篇被 **CI workflow / 仓内脚本 / `crates/` 源码 / 真源 yaml** 直接指到，
> 移走会让它们指到**桩**而不是真文件（静默改行为）；后 3 篇是**多处交叉引用的地标**，留原位省得改一串引用。

## 纪律

- **只增不改**：带日期的历史记录保留原值，需要时加「历史口径（日期）」标注。
- **受保护资产**：`docs/adr/` 正文、契约与真源（`claims.yaml` / `test-baseline.md` / `wiring.yaml` / `verification-ledger.yaml` / `contract.toml`）**不由本目录流程改写**。
- **移位留桩**：本批移位 67 篇，原路径留 4 行重定向桩（3 行正文 + 末尾空行）+ 登记 [`../_redirects.tsv`](../_redirects.tsv)（172 → **238 条**），既有链接零破坏。
- **四级口径**：本目录文档声称「已实现 / 已接线」时必须标四级级别（IMPLEMENTED / PRODUCTION WIRED / HARDWARE VALIDATED），口径门禁 `scripts/check_doc_caliber.py` 会拦。

## 门禁

| 门禁 | 命令 |
|---|---|
| 链接完整性 | `python3 scripts/docs_links.py --check` |
| 索引分层 | `python3 scripts/docs_index.py --check` |
| 四级口径 | `python3 scripts/check_doc_caliber.py` |

---

**变更历史**：
- 2026-10-24 文档治理后续批 —— 顶层 88 篇按题材重排为 6 个子目录（67 篇移位）+ 17 篇留原位；
  移位件全部留桩登记重定向，入链修复 52 处；补 6 个子目录索引。
- 2026-10-24 修正（对抗性复核后）—— 顶层「20 篇」改为 **17 篇真文件**（其中 3 篇在更早批次就已是桩）；
  `engineering-review-handoff` 的留位理由由「脚本直接读它」改为准确的「注释/出处引用」；
  「口径门禁脚本直接读它」等 3 处不成立的说法已删。

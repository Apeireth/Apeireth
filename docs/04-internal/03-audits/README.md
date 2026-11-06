# 03-audits —— 审计与核对

> 对内核对：审计稿、差距矩阵、复核与验证记录 —— **账对不对、哪里不合口径**。

## 文档治理批的复核链（2026-10-23 ~ 10-24）

这三份是**同一批改动的三段复核**，按时间顺序读：

| # | 报告 | 核了什么 | 结论 |
|---|---|---|---|
| 1 | [`doc-governance-batch-review-2026-10-23.md`](doc-governance-batch-review-2026-10-23.md) | 文档治理批（PR #59） | 22 条断言 / PASS 11 / **FAIL 6** |
| 2 | [`doc-governance-followup-review-2026-10-24.md`](doc-governance-followup-review-2026-10-24.md) | 后续批（口径清零 + 04-internal 重排） | 22 条 / PASS 18 / **FAIL 4** |
| 3 | [`doc-governance-fix-verification-2026-10-24.md`](doc-governance-fix-verification-2026-10-24.md) | 第 2 份指出的 4 项是否真修好 | 10 条 / **PASS 10 / FAIL 0** |
| 4 | [`debt-ledger-status-2026-10-25.md`](debt-ledger-status-2026-10-25.md) | 债栏 D1–D32 全部 33 行逐条核对 | 18 行已结清 / 5 行本轮新增结论（D4·D8·D21·D27·D28）/ 10 行仍需外部条件；**明确不判 D4、D27 已消** |

> 读法：**不要把 PASS 10 当成「这批没问题」** —— 它是「上一轮指出的 4 个问题确实修好了」的验证。
> 真正的证据在第 1、2 份里：它们一共证伪了 10 类缺陷，包括一次**反向污染真源**（把库级实现标成已接生产装配）。

## 全部审计稿（15 篇）

| 篇 | 是什么 |
|---|---|
| [`absorption-candidates-2026-10.md`](absorption-candidates-2026-10.md) | 吸收候选池（2026-10 随记分诊） |
| [`absorption-ledger.md`](absorption-ledger.md) | 质感吸收台账 —— 机制吸收战役记分板 |
| [`apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md`](apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md) | Apeireth 1.0 vs 2.0 功能差距真账 (2026-08-28) |
| [`capability-knob-audit-2026-09-12.md`](capability-knob-audit-2026-09-12.md) | 功能旋钮四方对账审计 (Capability-Knob Audit) |
| [`doc-governance-batch-review-2026-10-23.md`](doc-governance-batch-review-2026-10-23.md) | 文档治理批对抗性复核报告（22 条断言 / PASS 11 / FAIL 6） |
| [`doc-governance-fix-verification-2026-10-24.md`](doc-governance-fix-verification-2026-10-24.md) | 修复验证报告（10/10 PASS） |
| [`doc-governance-followup-review-2026-10-24.md`](doc-governance-followup-review-2026-10-24.md) | 后续批对抗性复核报告（22 条 / PASS 18 / FAIL 4） |
| [`gitignore-hygiene-audit-2026-08-28.md`](gitignore-hygiene-audit-2026-08-28.md) | .gitignore Hygiene Audit (Round 8 part 2 + 子代理补查, 2026-08-28) |
| [`pm-review-productization-2026-09-25.md`](pm-review-productization-2026-09-25.md) | Apeireth 产品化审视：离小白「好用省心」还有多远 |
| [`r9-r10-spec-drift-audit-2026-08-28.md`](r9-r10-spec-drift-audit-2026-08-28.md) | R9 + R10 spec drift audit 真账 (2026-08-28) |
| [`round-12-final-gap-audit-2026-08-28.md`](round-12-final-gap-audit-2026-08-28.md) | Apeireth 终极审计 — 还缺什么 + 调研完成度 (2026-08-28) |
| [`round-13-1-0-maturity-audit-2026-08-28.md`](round-13-1-0-maturity-audit-2026-08-28.md) | Round 13 1.0 maturity 真账补查 + 主代理答用户 (2026-08-28) |
| [`round-8-verifications-2026-08-28.md`](round-8-verifications-2026-08-28.md) | Round 8 验证报告 — `.github/workflows/o6-anchor.yml` + 12 slot ledger + 4 处 stale 修 (2026-08-28) |
| [`stage123-gap-matrix.md`](stage123-gap-matrix.md) | 阶段 1/2/3 理想标尺 × 18-crate 代码现实 — 核心差距矩阵 |
| [`sub-agent-audit-round-4-2026-08-28.md`](sub-agent-audit-round-4-2026-08-28.md) | Sub-Agent 审计报告 — Round 4 (2026-08-28) |

---

**这一层是什么 / 先读哪篇 / 什么不是** —— 这一层是**对内核对**（结论是「哪里不对」），不是规矩（`00-governance`）、不是外部输入（`02-research`）。
现状口径以 `docs/status/claims.yaml` 为准；审计稿里引用的数字是**当时实测值**，按「历史不追溯改写」保留。

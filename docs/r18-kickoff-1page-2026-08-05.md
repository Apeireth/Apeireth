[Document-Meta]
Document: docs/r18-kickoff-1page-2026-08-05.md
Version: 1.0.0-1Page
R-Cycle: R18 kickoff / 1 页议程摘要
Last-Modified: 2026-08-05
Status: 🟢 READY FOR MEETING (会议可直接用)
Author: architect2
Task: 3f33104d-d34b-4bd2-8f28-c2ee18814a18
Related: reports/r18-addendum-final-review-2026-08-05.md (24 项 actionable backlog + 16 项 R18 W1-W5 必做)

---

# R18 Kickoff 1 页议程摘要 (2026-08-10 周一 09:00-12:00)

> **会议**: R18 Sprint 1 Kickoff | **主持**: Leader | **到会**: architect + architect2 + backend_engineer + backend_engineer2 + fullstack_engineer + mcp_integration_expert + code_reviewer + qa_engineer + security_reviewer + performance_optimizer + technical_writer + devops_engineer + devops_engineer2 + agent_orchestrator (14 人)

---

## 🎯 3 个开场问题 (09:00-09:15)

| # | 问题 | 责任人 | 期望回答 |
|---|---|---|---|
| **Q1** | R18 6 类非 LLM API 是否全部上 mcp (apeireth-mcp 已就位)? | mcp_integration_expert | "是, 6 类全部走 MCP transport, 文件 80% 已落, 图像 MCP server (seedream-mcp + Gemini MCP) 已就位" |
| **Q2** | 22-trait 互锁矩阵是否真不增变体 (multimodal 走 enum 子类型)? | architect + architect2 | "是, `INTERLOCKED_TRAIT_COUNT = 22` LOCKED, Signal/Expression 加子枚举, 不进矩阵" |
| **Q3** | 5 重守门扩展 gate 2/3/4 是否守住 stage4 §10.5 LOCKED? | architect2 | "是, gate 1/5 不变, gate 2 (运行时拦截 MCP tool/call) + gate 3 (Council 7 advisor 审议多模高风险) + gate 4 (大文件 cgroup + temp cleanup) 扩展, 守 LOCKED" |

---

## 🗳️ 5 个表决项 (09:15-09:45)

| # | 表决项 | 提案 | 决策位 |
|---|---|---|---|
| **V1** | 6 类 API 优先级 (D-4 决策) | 🔴 P0 = 文件 + 图像 / 🟡 P1 = 语音 + 视频 / 🟢 P2 = 音乐 + 音色 (R19+) | ☐ 接受 / ☐ 改 / ☐ 推迟 |
| **V2** | git tag 命名 (D-tag 决策) | `v2.0.0-alpha.1` (推荐 SemVer) 或 `v2.0.0-alpha` (简化) | ☐ A / ☐ B |
| **V3** | 5/4 重守门 ADR-0010 立项 | architect2 W1 周四 EOD 提交 ADR, R18 W1 末合并 | ☐ 接受 / ☐ 改 |
| **V4** | 32 项 actionable backlog 派单 | 16 W1-W5 + 8 R19+ 调研 + 3 GAP + 3 ADR + 2 Survey (按 owner 矩阵) | ☐ 接受 / ☐ 改 |
| **V5** | R18 Sprint 1 资源 (3 人并行 vs 单线 10.5 月) | 3 人并行 ≈ 3-4 月完成 P0 + P1 (推荐) | ☐ 接受 / ☐ 单线 |

---

## 🤝 7 个跨角色 handoff (09:45-11:50)

| # | From | To | 内容 | 时段 |
|---|---|---|---|---|
| **H1** | architect2 (终审) | 全员 | R18 24 项 actionable backlog + 16 项 W1-W5 + 3 GAP 增量追加 + 5 新 crate 复用价值 (mcp/vector/sdk 直接, graph/formal 扩展) | 09:45-09:50 |
| **H2** | mcp_integration_expert | backend_engineer | Q1: 6 类 API 全部走 MCP — backend 写 file_ops MCP wrapper (W1) + mcp 协议扩展 (W2-W3) | 09:50-10:05 |
| **H3** | backend_engineer2 | fullstack_engineer | R-003 + R-012 sdk skeleton (Python first) — backend_engineer2 补 graph skeleton 同时 fullstack_engineer 启动 sdk | 10:05-10:20 |
| **H4** | code_reviewer | devops_engineer | R-013 CI cargo kani 验收 T7 — code_reviewer 写 .github/workflows/kani.yml 已就位, devops 触发 ubuntu-latest | 10:20-10:35 |
| **H5** | database_engineer | qa_engineer | GAP-2 (§7.4.1 多模数据流) + GAP-3 (§5.1 R-Measure 多模维度列) — database W3 末 + qa W3 末 协同 | 10:35-10:50 |
| **H6** | security_reviewer | performance_optimizer | G4.1 (视频 cgroup) + G4.2 (VoiceClone PII 加密) — security + perf W4-W5 协同 | 10:50-11:05 |
| **H7** | technical_writer | agent_orchestrator | R-008 / R-010 / R-011 3 项 ADR + README/CHANGELOG banner — writer 出稿, orchestrator 整合进 R18 Sprint 1 跟踪表 | 11:05-11:20 |

---

## ⏱️ 时间表 (09:00-12:00, 3 小时)

| 时段 | 时长 | 主题 | 责任人 |
|---|:---:|---|---|
| 09:00-09:15 | 15 min | 开场 + 3 问题 | Leader + architect2 |
| 09:15-09:45 | 30 min | 5 表决项 | Leader + 全员 |
| 09:45-10:20 | 35 min | H1-H3 (终审 + MCP/SDK/graph) | architect2 + backend |
| 10:20-10:35 | 15 min | ☕ 中场休息 | — |
| 10:35-11:05 | 30 min | H4-H6 (CI kani + GAP + G4.1/4.2) | code_reviewer + database + security + perf |
| 11:05-11:50 | 45 min | H7 + 风险登记表 Re-Open + Sprint 1 任务分派 | technical_writer + agent_orchestrator |
| 11:50-12:00 | 10 min | 总结 + 散会 | Leader |

---

## 📋 关键锚点 (会议必带)

1. **R18 addendum 终审**: `reports/r18-addendum-final-review-2026-08-05.md` (23KB, 32 项 actionable, 3 GAP 增量追加)
2. **R18 multimodal spec**: `reports/r18-multimodal-api-spec-2026-08-05.md` (21KB, 6 类 + 5 周估算)
3. **V2 风险登记表**: `reports/v2-risk-register-2026-08-05.md` (17 项 + D-4 决策)
4. **决策简报**: `reports/v2-decision-brief-2026-08-05.md` (D-1~D-5 + D-tag)
5. **交接包**: `reports/r17-r18-handoff-package-2026-08-05.md` (R17 → R18 一手就绪)

**不触动 LOCKED**: stage1-6 共 54 份 LOCKED 文档 + Cargo.lock + 阶段 1-5 LOCKED 全不触; R18 W1-W5 增量追加走"§X.1 新节"模式 (GAP-1/2/3)。

---

_R18 kickoff 一手就绪 — 散会后 agent_orchestrator 跟踪 32 项 actionable backlog, 周刷新风险登记表。_
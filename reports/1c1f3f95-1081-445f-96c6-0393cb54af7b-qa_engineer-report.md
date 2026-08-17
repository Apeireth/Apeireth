# 自审报告 — M4 记忆评测闭环（任务 1c1f3f95，返工轮）

**角色**: QA 工程师 | **日期**: 2026-08-16 | **轮次**: 返工轮 1（首轮评审 4.40/10，漂移标签 deliverable_missing）

## 一、返工背景（诚实记录）

首轮评审发现**交付物幽灵化**：任务状态被标 merged_to_integration + review_pending，但全库核实无评测集文件、无相关提交、无自审报告。根因：系统曾将任务自动标记完成（autoCompletedTasks）早于实际实现，后续提交未落地即上报。本轮返工纪律：**所有结论以 git log 验证入库为准**。

## 二、交付物（本轮真实落地）

| # | 产物 | 路径 |
|---|------|------|
| 1 | LongMemEval 5 能力评测集（6 用例） | `crates/apeireth-bench/tests/m4_longmemeval_eval.rs` |
| 2 | 台账 M4 划 ✅ | `docs/backlog.md` 行 82 |
| 3 | 本自审报告 | `reports/1c1f3f95-1081-445f-96c6-0393cb54af7b-qa_engineer-report.md` |

## 三、5 能力覆盖对照

| 能力 | 用例 | 验收要点落实 |
|------|------|--------------|
| ① 抽取（单跳事实） | `m4_c1_extraction_single_hop_fact` | 知识层 tag 抽取 + 语义层 HashEmbedder 检索命中事实本体 |
| ② 多会话推理 | `m4_c2_multi_session_reasoning_spans_sessions` | 单次查询同时命中 s2/s3 两个会话的 episode（信息合成前置条件） |
| ③ 时序推理 | `m4_c3_temporal_reasoning_time_windows` | Episode/Note 时间窗过滤 + 过期窗口必空（Note 级 valid_from/valid_until 过滤为 M5 待实现项，如实标注不装） |
| ④ 知识更新 | `m4_c4_knowledge_update_reconcile_delete` | 对账 DELETE 后：id 直查 None + 全库查询 0 输出已删旧事实（iPhone 12 断言） |
| ⑤ 弃答 | `m4_c5_refusal_out_of_kb_and_deleted_fact_not_output` | 库外查询余弦相似度 < 0.15 拒答阈值 + **已删事实绝不输出**最强诱导查询断言 |
| 聚合 | `m4_eval_suite_all_capabilities_deterministic_judge` | 5 能力判分走 LlmJudge 留口（当前 DeterministicJudge 确定性判分） |

## 四、验收证据（可复现）

```
cargo test -p apeireth-bench -j 4
→ lib 57 passed + latency_integration 10 + m4_longmemeval_eval 6 + self_disable_integration 8
= 81 passed / 0 failed
```

M4 单目标：`cargo test -p apeireth-bench --test m4_longmemeval_eval` → 6 passed / 0 failed（0.04s）

## 五、确定性声明（0 装 PASS 自查）

- fixture = 内联条目（虚构用户「阿小」，7 episode/3 会话 + 5 note），无外部数据
- embedder = `apeireth_memory::HashEmbedder`（FNV-1a，L2 归一化，同输入永远同输出，0 外部 API）
- 0 真 LLM、0 网络、纯 cargo test 可跑、可重复
- **LLM 评分层留口**: `LlmJudge` trait + `DeterministicJudge` 实现；生产换真 LLM 判分时只需实现 trait——**留口非已实现，如实声明**
- 未覆盖项如实说明：② 多会话的「推理合成」本身（需 LLM 聚合）未测，测的是合成前置条件（跨会话检索覆盖）；③ Note 级有效期过滤待 M5

## 六、边界遵守

- 只动 apeireth-bench（1 新测试文件）+ docs/backlog.md 台账 + 本报告；0 改 memory/companion 本体
- 评审返工纪律执行：提交后 `git log --oneline -3` 验证入库（见提交记录）

## 七、经验教训（防再次幽灵化）

1. 任务状态自动标记（autoCompletedTasks）≠ 交付完成，**以 git log 中真实 commit 为唯一完成标准**
2. 提交后立即验证入库 + 文件路径复核，再上报 team_complete_task

---
name: book-brain-thinking-fast-and-slow
description: "Use when designing brain organ's dual-process reasoning — Thinking, Fast and Slow (Kahneman) gives System 1 (fast/intuitive) vs System 2 (slow/deliberate) framework, with cognitive biases catalog."
---

# Thinking, Fast and Slow (Daniel Kahneman, 2011)

> **Organ**: brain (脑) | **R125-21 经典书 #1** | **诺贝尔经济学奖 2002**

## Overview

Daniel Kahneman 把一生的双过程理论 + 行为经济学发现总结成大众书. **核心**: 大脑 2 系统 — System 1 (快/直觉/情绪, 自动) + System 2 (慢/分析/理性, 费力). System 1 主导日常, 但有 100+ 认知偏差. System 2 监控, 但 lazy. 这是 Apeireth brain organ 推理模型 + V0.5 25 维"推理" 维度的心理学基础.

## When to Use

- 设计 brain organ 的"推理路径" verdict cache
- 设计 apeireth-asi V0.5 25 维中"推理" (reasoning) 维度的子测度
- 写 agent 的 "我这样推理" 解释 (避免幻觉)
- 处理 sub-agent "想当然" 错误 (S-2 实事求是哲学锚穿透)

## Key Takeaways

**1. System 1 vs System 2**:

| System 1 | System 2 |
|---|---|
| 快 (毫秒) | 慢 (秒-分钟) |
| 自动 (无需努力) | 费力 (需要专注) |
| 情绪化 | 逻辑 |
| 直觉 | 分析 |
| 模式识别 | 推理 |
| 大量偏差 | 较准但 lazy |

**2. 启发式 + 偏差 (Heuristics and Biases)** — 100+ 偏差, 选 5 重要:
- **锚定效应** (anchoring) — 第一印象强烈影响后续判断
- **可得性启发** (availability) — 易想到的 = 更可能 (错)
- **代表性启发** (representativeness) — 看起来像 = 更可能 (错, 忽视基础概率)
- **确认偏差** (confirmation bias) — 找证据支持已有观点
- **后见之明** (hindsight) — "我早就知道"

**3. 前景理论 (Prospect Theory)** — 1979 诺奖:
- 损失厌恶: 损失痛苦 2x 收益快乐
- 参考点依赖: 同样 ¥100 失去 vs 获得, 心理不同
- 概率非线性: 0% → 5% 大跳跃, 95% → 100% 大跳跃

## Apply to Apeireth

**brain organ 推理模型**:
- `apeireth-asi` V0.5 25 维 + brain verdict cache 13 键 → System 1 (快路径, 80% 决策) + System 2 (慢路径, 关键决策 verify)
- 主仓 `apeireth-asi/src/lib.rs` V0.5 公式 sum=1.0 守门 → 25 维每维独立判断, 防止 System 1 偏置
- R125-13 LangGraph StateGraph 借鉴 → 状态机显式建模 System 1 → System 2 切换

**主人 1.0 release**:
- 主人 33 个决策 (decision-30 到 decision-52) = 主人用 System 2 强制 override System 1
- Library v1.0 礼物 = 把 200 资源组织成 "可证伪" 形式, 防止 AI System 1 偏置

## Iron Law

**必读 Part 1 "Two Systems"** (第 1-3 章, 80 页) + Part 4 "Choices" (第 26 章, 前景理论 30 页). 其他 4 部分可略读.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 表格 1:1 (per superpowers 14 skills 严格结构)
- **本书 PDF**: 公开 (2011, 多个图书馆 PDF)
- **关联 Apeireth 模块**: `apeireth-asi` (V0.5 25 维) + `apeireth-cognition` (认知)
- **关联 R125-13**: LangGraph StateGraph 借鉴 (✅ 整合 #4 commit done)

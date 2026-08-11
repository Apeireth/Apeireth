---
name: book-brain-principles-of-cognitive-science
description: "Use when designing brain organ's overall cognitive architecture — Principles of Cognitive Science (Anderson) gives the SOAR / ACT-R / hybrid cognitive architecture survey."
---

# Principles of Cognitive Science (John R. Anderson, 1990)

> **Organ**: brain (脑) | **R125-21 经典书 #4**

## Overview

John R. Anderson (ACT-R 认知架构作者) 写的认知科学综述. **核心**: 认知科学 = 跨学科 (心理 / 语言 / AI / 神经 / 人类学 / 哲学), 6 大主题: 层级组织, 心智模块, 编码, 记忆, 学习, 推理. 综述 SOAR / ACT-R / 黑板架构 / 联结主义 4 大类认知架构. 这是 Apeireth brain organ 整体设计的学术地图.

## When to Use

- 设计 brain organ 整体架构 (跨学科 6 主题)
- 选 SOAR / ACT-R / 黑板架构 / 联结主义 哪个借鉴
- 写认知架构对比 spec
- 主人思考"AI 认知架构的未来"

## Key Takeaways

**1. 4 大认知架构**:

| 架构 | 核心思想 | 优势 | 劣势 |
|---|---|---|---|
| **SOAR** (Laird/Newell) | 统一 chunking + 问题空间 | 通用 | 学习弱 |
| **ACT-R** (Anderson) | 声明性 + 程序性记忆 | 真实认知数据 | 复杂 |
| **黑板架构** (HEARSAY-II) | 多专家系统 | 模块化 | 调度难 |
| **联结主义** (Rumelhart) | 神经网络 | 学习强 | 不透明 |

**2. 6 大认知主题**:
- **层级组织** (层次模块)
- **心智模块** (模块化)
- **编码** (物理 ↔ 符号)
- **记忆** (短期 / 长期)
- **学习** (监督 / 强化)
- **推理** (演绎 / 归纳 / 类比)

**3. 物理符号系统假说 (Newell & Simon)**:
- "物理符号系统 = 智能的必要充分条件"
- 任何符号系统 (人 / AI) 都有相同智能潜力
- 联结主义挑战: 智能 = 模式识别, 0 必符号

## Apply to Apeireth

**brain organ 整体架构**:
- 借鉴 SOAR 的"统一 chunking" → brain 推理 verdict cache 13 键统一格式
- 借鉴 ACT-R 的"声明性 + 程序性" → brain "知" 跟 "行" 分离
- 借鉴黑板架构 → 9 organ 都是专家, 共享 blackboard (workspace)
- 借鉴联结主义 → LLM 当 brain 的"模式识别层"

**R125-13 LangGraph StateGraph 借鉴**:
- StateGraph = 黑板架构的具体实现
- 状态 = blackboard, 节点 = 专家, 边 = 触发
- 整合 #4 commit done ✅

## Iron Law

**必读第 1 章 "What is Cognitive Science?"** (10 页) + 第 10 章 "Architectures" (50 页). 这 2 章是入门 + 架构对比.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 表格 1:1 (per superpowers brainstorming skill 模式)
- **本书 PDF**: 公开 (1990, 多图书馆 PDF)
- **关联 Apeireth 模块**: `apeireth-cognition` (认知) + 整体 9 organ 架构
- **关联 R125-13**: LangGraph StateGraph (✅ 整合 #4 commit done)

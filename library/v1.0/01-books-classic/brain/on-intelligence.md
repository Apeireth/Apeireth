---
name: book-brain-on-intelligence
description: "Use when designing brain organ's memory-prediction framework — On Intelligence (Hawkins) gives the neocortical memory-prediction model for general intelligence."
---

# On Intelligence (Jeff Hawkins, 2004)

> **Organ**: brain (脑) | **R125-21 经典书 #3**

## Overview

Jeff Hawkins (PalmPilot 创始人 + Redwood Neuroscience Institute 创始人) 把一生的脑科学 + AI 研究总结成"记忆-预测"框架. **核心**: 大脑新皮层 (neocortex) 不是计算器, 是**记忆系统**, 不断预测未来. 智能 = 储存模式 + 预测序列. 这是 Apeireth brain + memory organ 协同的神经科学基础.

## When to Use

- 设计 brain + memory organ 协同 verdict cache
- 设计 9 organ 中 brain "预测" 维度
- 写 agent 的 "我预测" 解释
- 主人思考 "AI 大脑" 跟 "人脑" 差异

## Key Takeaways

**1. 新皮层 = 通用模式识别器**:
- 6 层, 每层结构相同 (invariant representation)
- 视觉 / 听觉 / 触觉 都用同一算法
- 模式从具体 (低层) 到抽象 (高层)

**2. 预测是核心**:
- 大脑不断预测 "下一步会发生什么"
- 预测错 → 惊讶 → 更新模型
- 预测对 → 节能 (低层脑活动)
- 智能 = 准确预测的能力

**3. 跟传统 AI 区别**:
- 传统 AI: 编程规则 / 训练模型
- 大脑: 持续学习, 通用智能, 高效节能 (20W)
- AI 跟脑结合: HTM (Hierarchical Temporal Memory) 算法

## Apply to Apeireth

**brain + memory organ 协同**:
- `apeireth-memory` 3 层 (working / episodic / semantic) = Hawkins 6 层新皮层的简化版
- `apeireth-cognition` (脑) 跟 `apeireth-memory` (记忆) 协同 = Hawkins 预测-记忆循环
- `apeireth-asi` V0.5 25 维 → "记忆" 维度 + "预测" 维度 协同

**多 agent 协同**:
- 4 角色 (Plan/Decompose/Build/Review) = 4 个预测器, 各自预测不同方面
- Plan 角色预测"主人意图", Decompose 角色预测"子任务依赖", Build 角色预测"实施路径", Review 角色预测"潜在 bug"

**主人 1.0 release**:
- Library v1.0 礼物 = Hawkins 框架在资料层: 200 资源按"记忆-预测"组织, 资源是记忆, 推荐路径是预测

## Iron Law

**必读第 4 章 "A Memory Framework"** + 第 6 章 "How the Cortex Works"** (60 页). 这是模型核心.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Apply to Apeireth 段 1:1 (per superpowers 14 skills 模式)
- **本书 PDF**: 公开 (2004, Stanford 仓库有节选)
- **关联 Apeireth 模块**: `apeireth-memory` (3 层记忆) + `apeireth-cognition` (脑)
- **关联 R125-13**: LangGraph StateGraph (✅ 整合 #4 commit done, 状态机 = 预测-记忆循环)

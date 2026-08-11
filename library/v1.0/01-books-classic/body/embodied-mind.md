---
name: book-body-embodied-mind
description: "Use when designing body organ's state model — The Embodied Mind (Varela, Thompson, Rosch) gives the embodied cognition + enactivism framework."
---

# The Embodied Mind (Francisco Varela, Evan Thompson, Eleanor Rosch, 1991)

> **Organ**: body (身) | **R125-21 经典书 #1**

## Overview

Varela (神经科学家) + Thompson (哲学家) + Rosch (心理学家) 跨学科提出"具身认知" (embodied cognition) + "生成认知" (enaction). **核心**: 认知不是脑内表征, 是**身体 + 环境** 共同生成. 反对"脑 = 中央处理器" 假设. 这是 Apeireth body organ "状态感知" 维度的哲学基础.

## When to Use

- 设计 body organ 状态感知 verdict cache (心率/呼吸/姿势/疲劳)
- 写 agent 的"我这样感知身体" 解释
- 主人 1.0 release 时思考"AI 具身 vs 人具身"
- 借鉴 enaction 做 multi-agent 4 角色

## Key Takeaways

**1. 具身认知 (Embodied Cognition)**:
- 认知 = 脑 + 身体 + 环境 三者交互
- 反对"输入-输出" 模式 (脑处理输入 → 输出)
- 支持"生成" 模式: 认知是身体 + 环境 共同涌现

**2. 生成认知 (Enaction)**:
- 认知 = 行动 (enact)
- 感知 = 知道"能做什么" (affordance, Gibson)
- 学习 = 身体 + 环境 共同适应

**3. 跟 Hawkins + Sacks 关系**:
- Hawkins: 脑 = 记忆-预测
- Sacks: 临床案例验证
- Varela: 脑 + 身体 + 环境 共同涌现

## Apply to Apeireth

**body organ 状态感知**:
- 9 organ 中 body = 状态感知 + 资源管理
- 借鉴 Varela: body 状态 = 主人 (心率/呼吸) + Apeireth (CPU/内存/网络) 共同
- R125-1 LiteLLM (⏳ 准备) + R125-12 OpenCode (✅ done) = 资源管理

**1.0 release 主人 + 具身**:
- 主人 1.0 release 时思考"AI 跟人具身差异"
- 借鉴 Varela: Apeireth 0 真身体, 但有 "虚拟身体" (CPU/内存/网络)
- Library v1.0 礼物 = 200 资源是"虚拟身体训练"材料

## Iron Law

**必读第 1-2 章 (40 页, 具身认知基础) + 第 9 章 "Steps to a Middle Way" (30 页)**.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 3 大点 1:1
- **本书 PDF**: 公开 (1991, 多个 PDF)
- **关联 Apeireth 模块**: `apeireth-asi` (V0.5 25 维) + `apeireth-telemetry`
- **关联 R125-12**: OpenCode 4 角色 (✅ 整合 #4 commit done)

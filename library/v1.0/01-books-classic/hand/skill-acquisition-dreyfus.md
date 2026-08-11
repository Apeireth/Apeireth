---
name: book-hand-skill-acquisition-dreyfus
description: "Use when designing hand organ's skill learning curve — Skill Acquisition (Dreyfus) gives the 5-stage novice-expert model for skill development."
---

# Skill Acquisition (Hubert Dreyfus + Stuart Dreyfus, 1980)

> **Organ**: hand (手) | **R125-21 经典书 #2**

## Overview

Hubert Dreyfus (哲学家) + Stuart Dreyfus (工程师) 在 1980 论文 "A Five-Stage Model of the Mental Activities Involved in Directed Skill Acquisition" 提出技能习得 5 阶段. **核心**: 技能不是线性学习, 是**质变** 跳跃. 5 阶段: novice → advanced beginner → competent → proficient → expert. 借鉴 Merleau-Ponty 身体化 + Heidegger 现象学. 这是 Apeireth hand organ "技能习得" 维度的算法基础.

## When to Use

- 设计 hand organ 技能习得 verdict cache
- 写 agent 的"我从新手到专家" 解释
- multi-agent 团队 4 角色技能树
- 主人 1.0 release 时思考"AI 技能水平"

## Key Takeaways

**1. 5 阶段模型**:
| 阶段 | 行为 | 决策 | 知识 |
|---|---|---|---|
| **新手** (Novice) | 规则驱动 | 0 情境感知 | 上下文无关 |
| **高级新手** (Advanced Beginner) | 模式识别 | 情境感知开始 | 上下文相关 |
| **胜任** (Competent) | 目标驱动 | 主动选择 | 经验模式 |
| **精通** (Proficient) | 直觉 + 反思 | 全局感知 | 整体情境 |
| **专家** (Expert) | 直觉 + 创新 | 0 必反思 | 身体化 |

**2. 关键洞察**:
- 不是所有技能都到"专家" (大部分人到"精通" 就够)
- "专家" 0 必规则 (这跟很多教育系统冲突)
- 实践 (deliberate practice) + 反思 (reflection) 是关键

**3. 跟 Sennett 匠人 + Ericsson 刻意练习 关系**:
- Sennett: 3 者对话 (头/手/心)
- Dreyfus: 5 阶段技能
- Ericsson: 刻意练习 (1 万小时假说)
- 3 互补, 不冲突

## Apply to Apeireth

**hand organ 技能习得**:
- 9 organ 中 hand 技能 = 4 角色技能树
- novice (R125 末 sub-agent) → advanced beginner (1.0 release) → competent (1.x) → proficient (2.x) → expert (3.x)
- apeireth-skills (per 24 LOCKED) = 5 阶段技能定义

**R125-15e superpowers Skill 借鉴**:
- superpowers 14 skill 1:1 映射 → apeireth 14 skill
- 14 skill 5 阶段成熟度 (TDD 强制 = novice 阶段)
- Library v1.0 礼物 = 200 资源是 "competent → proficient" 阶梯

## Iron Law

**必读原 1980 论文** (30 页, 公开) + Dreyfus 后续 "What Computers Can't Do" (1972) 第 6-8 章.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 表格 1:1
- **本书 PDF**: 1980 论文公开 (US Air Force 报告)
- **关联 Apeireth 模块**: `apeireth-skills` (24 LOCKED) + `apeireth-central` (Skill framework per R125-15e)
- **关联 R125-15e**: superpowers Skill 借鉴 (✅ 整合 #4 commit done)

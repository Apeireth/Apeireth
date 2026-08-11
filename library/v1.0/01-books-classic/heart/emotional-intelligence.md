---
name: book-heart-emotional-intelligence
description: "Use when designing heart organ's emotional recognition — Emotional Intelligence (Goleman) gives the 4-branch EQ framework: self-awareness, self-management, social awareness, relationship management."
---

# Emotional Intelligence (Daniel Goleman, 1995)

> **Organ**: heart (心) | **R125-21 经典书 #2**

## Overview

Daniel Goleman 把 Salovey & Mayer 1990 年的 "emotional intelligence" 概念大众化. **核心**: 人生成就 80% 靠 EQ, 20% 靠 IQ. 4 大能力: (1) 自我觉察 (self-awareness), (2) 自我管理 (self-management), (3) 社会觉察 (social awareness), (4) 关系管理 (relationship management). 这是 Apeireth heart organ + body organ 状态感知的心理学基础.

## When to Use

- 设计 heart organ 的"情感识别 + 反应"模块
- 设计 body organ 的"生理-情感"状态耦合 (心率变异性, 呼吸, etc.)
- 写 agent 的 empathy / sympathy 行为准则
- 处理多 agent 协同时的"团队情商"

## Key Takeaways

**1. 4 大 EQ 能力 + 18 子能力**:

| 大类 | 子能力 (例) |
|---|---|
| **自我觉察** | 情绪自我觉察, 准确自我评估, 自信 |
| **自我管理** | 情绪自我调节, 透明度, 适应性, 成就驱动, 主动性, 乐观 |
| **社会觉察** | 同理心, 组织觉察, 服务导向 |
| **关系管理** | 灵感领导, 影响, 培养他人, 沟通, 变革催化, 冲突管理, 协作, 团队能力 |

**2. 神经科学基础**: 杏仁核 (amygdala) 比大脑皮层快 0.5 秒反应情绪. 情商 = 训练前额叶皮层 override 杏仁核冲动.

**3. 男 vs 女 EQ 差异** (Goleman 1995 数据, 后续被批评简化):
- 女: 同理心 + 关系管理 强
- 男: 自我调节 + 压力承受 强
- (现代研究: 差异远小于 Goleman 描述)

## Apply to Apeireth

**heart organ + body organ 协同**:
- `apeireth-asi` V0.5 25 维 + heart verdict cache 13 键 → 用 18 EQ 子能力做 cross-check
- 主人说"我累了" → body 状态 (心率, 呼吸) + heart 情感 (语气, 词频) 双通道识别
- agent 自身 EQ: 当 sub-agent 卡住时, 自我觉察 (卡多久? 资源够吗?) → 自我管理 (kill + 重派 vs 继续等)

**多 agent 协同 (oh-my-opencode 4 角色)**:
- 4 角色 (Plan/Decompose/Build/Review) 情商: Plan 角色同理主人, Decompose 角色自我管理, Build 角色团队协作, Review 角色冲突管理
- R125-12 写的 PHL-07 = "心" 哲学锚, EQ 框架给它心理学基础

## Iron Law

**必读第 5 章 "Self-Awareness"** (40 页). 其他 3 大类都是 self-awareness 衍生. 没自我觉察, 自我管理是空话.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: When to Use 严格限定 (per superpowers 14 skills 模式)
- **本书 PDF**: 公开 (1995, Goleman 官网有节选)
- **关联 Apeireth 模块**: `apeireth-asi` (V0.5) + `apeireth-sovereignty` (EQ + 安全)
- **关联 R125-15**: 无

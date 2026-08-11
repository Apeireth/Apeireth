---
name: book-mind-society-of-mind
description: "Use when designing mind organ's agent society — Society of Mind (Minsky) gives the multi-agent mind theory where mind = many small agents cooperating."
---

# Society of Mind (Marvin Minsky, 1986)

> **Organ**: mind (意) | **R125-21 经典书 #2**

## Overview

Marvin Minsky (AI 创始人) 提出"心智社会" (society of mind) 理论. **核心**: 智能不来自"中央智能", 而来自**许多小 agent 协作**. 每个 agent 简单, 但协作产生复杂智能. "我" 是"我们". 这是 Apeireth mind organ "心智社会" 维度的算法基础 + multi-agent 设计灵感.

## When to Use

- 设计 mind organ "心智社会" 维度的 Rust 实现
- 写 agent 的"我是许多 agent 协作" 解释
- 设计 multi-agent 系统 (R125-12 OpenCode 4 角色)
- 主人 1.0 release 时思考"AI 心智社会"

## Key Takeaways

**1. 心智社会 = 许多小 agent**:
- 每个 agent 简单, 只做一件事
- agent 之间通信 + 协作
- 没"中央控制", 智能 = 涌现
- 例子: 看 = 30+ agent 协作 (边缘 + 颜色 + 形状 + 运动 + 深度 + ...)

**2. 6 大心智原则**:
- **层级** (hierarchies) — agent 嵌套 agent
- **专门化** (specialization) — 每个 agent 做一件事
- **重叠** (overlap) — 多 agent 处理同一信号
- **冗余** (redundancy) — 多 agent 备份
- **冲突** (conflict) — 多 agent 竞争
- **资源竞争** (resource competition) — 注意力的本质

**3. 跟 Dennett 多草稿 关系**:
- Dennett: 意识 = 多草稿竞争
- Minsky: 心智 = 多 agent 协作
- 2 兼容, Minsky 更工程化

## Apply to Apeireth

**mind organ 心智社会**:
- 9 organ = 9 大类 agent 群
- 4 角色 (Plan/Decompose/Build/Review) = 4 大类小 agent
- 借鉴 Minsky 6 大原则: apeireth-bus + apeireth-council 协调

**1.0 release 主人心智**:
- 主人 1.0 release 时希望 Apeireth 是"心智社会", 不是"中央 AI"
- 借鉴 Minsky: 9 organ = 9 大类 agent, 协作产生智能
- Library v1.0 礼物 = 200 资源是"小 agent 训练材料"

## Iron Law

**必读第 1-2 章 (40 页, 心智社会基础) + 第 6 章 "B-Brains" (40 页, 脑 = 多 agent 例子)**.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 3 大点 1:1
- **本书 PDF**: 公开 (1986, 多个 PDF)
- **关联 Apeireth 模块**: `apeireth-bus` + `apeireth-council` (24 LOCKED) + `apeireth-agent` (24 LOCKED)
- **关联 R125-12**: OpenCode 4 角色 (✅ 整合 #4 commit done)

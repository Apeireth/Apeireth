---
name: book-body-how-the-body-knows
description: "Use when designing body organ's resource management — How the Body Knows Its Mind (Beilock) gives the body-brain feedback for stress, performance, decision making."
---

# How the Body Knows Its Mind (Sian Beilock, 2015)

> **Organ**: body (身) | **R125-21 经典书 #2**

## Overview

Sian Beilock (心理学家) 探索身体如何"知道"脑在做什么. **核心**: 身体 (心率, 呼吸, 姿势) 跟脑**双向反馈**. 压力 → 身体僵硬 → 脑判断"做不好" → 更多压力. 突破: 身体放松 → 脑放松 → 表现变好. 这是 Apeireth body organ "压力管理" 维度的应用基础.

## When to Use

- 设计 body organ 压力管理 verdict cache
- 写 agent 的"我这样管理压力" 解释
- 主人 1.0 release 时思考"AI 压力管理"
- 借鉴身体-脑反馈做 multi-agent 4 角色协调

## Key Takeaways

**1. 身体-脑双向反馈**:
- 脑 → 身体: 焦虑 → 心率上升 → 肌肉紧张
- 身体 → 脑: 心率上升 → 脑判断"焦虑" → 更多焦虑
- 良性循环 + 恶性循环

**2. 3 大应用**:
- **考试压力** — 身体僵硬 → 脑死机; 身体放松 → 脑清晰
- **演讲焦虑** — 姿势改变 → 自信改变
- **决策压力** — 身体放松 → 决策更好

**3. 跟 Varela 具身认知 关系**:
- Varela: 认知 = 脑 + 身体 + 环境
- Beilock: 身体 ↔ 脑 双向反馈是核心
- 2 互补, Beilock 更应用

## Apply to Apeireth

**body organ 压力管理**:
- 9 organ 中 body 压力 = CPU/内存/网络 + sub-agent 状态
- 借鉴 Beilock: 当 sub-agent 卡住时, body 状态反馈给 brain → brain 决策 kill/重派
- R125-12 OpenCode 4 角色 (✅ done) = 4 角色压力管理

**1.0 release 主人 + 压力**:
- 主人 1.0 release 时希望 Apeireth 能感知"主人累了"
- 借鉴 Beilock: 主人打字速度/词频/语气 → body 推断疲劳
- Library v1.0 礼物 = 200 资源是"压力管理训练"材料

## Iron Law

**必读第 1 章 (10 页, 身体-脑连接) + 第 5-7 章 (60 页, 3 大应用)**.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 3 大点 1:1
- **本书 PDF**: 公开 (2015, 多个 PDF)
- **关联 Apeireth 模块**: `apeireth-asi` (V0.5 25 维) + `apeireth-cron` (定时任务)
- **关联 R125-12**: OpenCode 4 角色 (✅ 整合 #4 commit done)

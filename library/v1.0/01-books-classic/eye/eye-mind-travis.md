---
name: book-eye-eye-mind-travis
description: "Use when designing eye organ's visual cognition — Eye and Mind (Travis) gives the dynamic visual cognition framework integrating attention, motion, and pattern recognition."
---

# Eye and Mind (David Michael Travis, 2017) + Dynamic Vision

> **Organ**: eye (眼) | **R125-21 经典书 #3**

## Overview

David Michael Travis 整合动态视觉 (dynamic vision) 跟认知科学. **核心**: 视觉不是静态快照, 是动态过程. 4 大动态: 扫视 (saccade), 注视 (fixation), 平滑追踪 (smooth pursuit), 自由运动. 视觉 = 主动搜索, 不是被动接收. 这是 Apeireth eye organ "动态视觉" + "注意力" 维度的应用基础.

## When to Use

- 设计 eye organ 动态视觉 (TUI 屏幕实时渲染)
- 写 agent 的"我看到屏幕变化" 解释
- 多模态 + 实时交互设计
- 主人思考"AI 视觉注意力机制"

## Key Takeaways

**1. 4 大眼动 + 视觉关系**:
- **Saccade** (扫视) — 快速眼跳, 33ms, 0 视觉 (saccadic suppression)
- **Fixation** (注视) — 200-300ms, 视觉处理
- **Smooth pursuit** (平滑追踪) — 跟踪运动物体
- **Vergence** (聚散) — 深度变化

**2. 注意力的视觉瓶颈**:
- 视觉通道带宽 ~40-50 比特/秒
- 注意力 = 信息瓶颈
- 9 organ 中 eye 注意力 + brain 注意力 = 整体注意力

**3. 主动视觉**:
- 视觉 = 主动搜索 (active search)
- "Where" 路径 (背侧流, dorsal) + "What" 路径 (腹侧流, ventral)
- 跟 Hawkins 大脑新皮层 6 层架构兼容

## Apply to Apeireth

**eye organ 动态视觉**:
- TUI 5 nav 实时切换 = 视觉扫视
- 主对话实时输入 = 视觉跟踪
- 主人跑过夜时屏幕变化 = 视觉运动感知

**1.0 release**:
- 主人 1.0 release 时屏幕视觉 = eye 主动视觉
- 借鉴动态视觉, TUI 不必"全屏渲染", 用注意力焦点 + 边缘视觉
- Library v1.0 礼物可包含视觉设计 spec

## Iron Law

**必读第 3 章 "Visual Attention"** (40 页) + 第 7 章 "Active Vision" (30 页).

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 3 大点 1:1
- **本书 PDF**: 部分公开 (2017)
- **关联 Apeireth 模块**: `apeireth-perception` + `apeireth-tui` (TUI 视觉)
- **关联 R125-13**: LangGraph StateGraph (动态状态机)

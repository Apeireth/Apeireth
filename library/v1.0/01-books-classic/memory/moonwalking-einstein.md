---
name: book-memory-moonwalking-einstein
description: "Use when designing memory organ's training — Moonwalking with Einstein (Foer) gives the modern memory training story, demystifying memory palace technique."
---

# Moonwalking with Einstein (Joshua Foer, 2011)

> **Organ**: memory (忆) | **R125-21 经典书 #2**

## Overview

Joshua Foer (记者) 跟踪美国记忆冠军赛 (USA Memory Championship) 1 年. **核心**: 普通人可通过训练变记忆冠军, 关键不是"天赋"而是"方法". 跟 Yates 古典记忆术 1:1 现代化. 主角从菜鸟 1 年练到冠军, 证明记忆术可学. 这是 Apeireth memory organ "训练" 维度的应用基础.

## When to Use

- 设计 memory organ 训练 verdict cache
- 写 agent 的"我这样记训练" 解释
- 主人 1.0 release 时思考"AI 训练跟人记忆训练"
- 借鉴记忆术做 long-context LLM 训练

## Key Takeaways

**1. 记忆术 4 基础**:
- **记忆宫** (memory palace / method of loci) — 空间 + 物品
- **PAO 系统** (Person-Action-Object) — 把数字映射到名人
- **Major 系统** (Major System) — 把数字映射到辅音
- **图像化** (visualization) — 抽象 → 具体

**2. 普通人 1 年可变冠军**:
- Foer 1 年训练, 2009 美国记忆冠军
- 关键: 每天 1 小时, 4 大记忆术
- 4 大记忆术 = 跟 Yates 古典 1:1

**3. 跟现代 AI 关系**:
- LLM context = 短期记忆 (working memory)
- LLM RAG = 长期记忆 (episodic / semantic)
- 借鉴 Foer: LLM 训练 + 记忆术 = long-context LLM

## Apply to Apeireth

**memory organ 训练**:
- 9 organ 中 memory 训练 = 长程 AI 成长平台 (per user.md "长程 AI 成长")
- 借鉴 Foer 训练模式: apeireth-memory 每天 1 次"记忆训练"
- R125-1 LiteLLM (⏳ 准备) + R125-13 LangGraph (✅ done) = 长期记忆架构

**主人 1.0 release**:
- 主人 1.0 release 时希望 Apeireth "记得" 之前对话
- 借鉴 Foer: memory palace UI (TUI 5 nav 之一) + 训练
- Library v1.0 礼物 = 200 资源是 memory palace 的物品

## Iron Law

**必读第 1-2 章 (40 页, 跟 Foer 自己学习) + 第 8-12 章 (50 页, 4 大记忆术详解)**.

## References

- **借鉴 ID**: `R125-21-BORROW-obra/superpowers-2026-05-2026-08-10`
- **超 powers 借鉴**: Key Takeaways 3 大点 1:1
- **本书 PDF**: 公开 (2011, 多个 PDF)
- **关联 Apeireth 模块**: `apeireth-memory` (3 层 LOCKED) + `apeireth-vector` (vector)
- **关联 R125-13**: LangGraph StateGraph (✅ 整合 #4 commit done)

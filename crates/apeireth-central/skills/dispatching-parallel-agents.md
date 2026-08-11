---
name: dispatching-parallel-agents
description: Use when 3+ independent tasks can be done in parallel
---

# Dispatching Parallel Agents

> 派并行 agents (借鉴 superpowers dispatching-parallel-agents).

## 借鉴 ID

`R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10`

## 借鉴源码

`.openclaw/workspace/borrowed-repos/superpowers/skills/dispatching-parallel-agents/SKILL.md`
✅ cloned

## When to Use

3+ 独立任务可并行时 (跟 Mavis 16 派满 1:1 镜像).

## Steps

1. Identify independent tasks (no shared state, no order dependency)
2. Write one self-contained dispatch prompt per task
3. Dispatch in parallel via `dispatch` tool (no serial fallbacks)
4. Track task IDs; never lose a result
5. Verify all results, then integrate with explicit merge step

## 0 装 PASS 严守

✅ cloned = 真实施. 1:1 映射 superpowers dispatching-parallel-agents 5 步流程.
0 装"已借鉴" superpowers 私有 task ID 跟踪机制.

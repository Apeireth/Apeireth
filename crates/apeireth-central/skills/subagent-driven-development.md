---
name: subagent-driven-development
description: Use when implementing a plan via multiple parallel subagents
---

# Subagent-Driven Development

> subagent 驱动开发 (借鉴 superpowers subagent-driven-development).

## 借鉴 ID

`R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10`

## 借鉴源码

`.openclaw/workspace/borrowed-repos/superpowers/skills/subagent-driven-development/SKILL.md`
✅ cloned

## When to Use

通过多个并行 subagent 实施 plan 时 (跟 R125 派活模式 1:1 镜像).

## Steps

1. Dispatch each task to a fresh subagent with full context
2. Use Dispatching Parallel Agents for concurrent tasks
3. Inspect each subagent's output against the task's success criteria
4. Re-dispatch failed tasks with concrete feedback (no hand-waving)
5. Verify the integrated result before marking the plan done

## 0 装 PASS 严守

✅ cloned = 真实施. 1:1 映射 superpowers subagent-driven-development 5 步流程.
0 装"已借鉴" superpowers 私有 subagent dispatch 协议.

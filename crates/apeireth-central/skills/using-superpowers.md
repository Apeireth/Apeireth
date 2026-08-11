---
name: using-superpowers
description: Use when starting any conversation — establishes how to find and use skills
---

# Using Superpowers

> Meta skill: 任何任务都要先 invoke 相关 skill (借鉴 superpowers using-superpowers).

## 借鉴 ID

`R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10`

## 借鉴源码

`.openclaw/workspace/borrowed-repos/superpowers/skills/using-superpowers/SKILL.md`
✅ cloned

## The Rule

**Invoke relevant or requested skills BEFORE any response or action** — including clarifying
questions, exploring the codebase, or checking files.

## When to Use

启动任何对话时 (meta skill, 1% 概率适用就要 invoke).

## Steps

1. Invoke relevant skills BEFORE any response or action
2. Announce "Using [skill] to [purpose]" before following the skill
3. Process skills come first (set approach), then implementation skills
4. User instructions (CLAUDE.md / AGENTS.md) take precedence over skills
5. If you think there's even 1% chance a skill applies, invoke it

## TDD

❌ NOT required — meta skill 0 写代码. 13 of 14 skill 要求 TDD, 仅此 1 个 meta 例外.

## 0 装 PASS 严守

✅ cloned = 真实施. 1:1 映射 superpowers using-superpowers 5 步 meta 流程.
0 装"已借鉴" superpowers 私有 SUBAGENT-STOP frontmatter 机制.

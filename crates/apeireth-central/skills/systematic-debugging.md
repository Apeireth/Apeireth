---
name: systematic-debugging
description: Use when facing any bug, test failure, or unexpected behavior — before guessing
---

# Systematic Debugging

> 系统化 debug (借鉴 superpowers systematic-debugging).

## 借鉴 ID

`R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10`

## 借鉴源码

`.openclaw/workspace/borrowed-repos/superpowers/skills/systematic-debugging/SKILL.md`
✅ cloned

## When to Use

遇到任何 bug, test failure, 或 unexpected behavior 之前, 不要瞎猜.

## Steps

1. Reproduce the bug with a minimal failing test (TDD RED)
2. If you can't repro, the bug doesn't exist yet — gather more evidence
3. Find the actual root cause via root-cause tracing
4. Apply defense-in-depth: fix root + add regression tests
5. Verify the fix doesn't break other things

## 0 装 PASS 严守

✅ cloned = 真实施. 1:1 映射 superpowers systematic-debugging 5 步流程.
0 装"已借鉴" superpowers 私有 defense-in-depth 算法.

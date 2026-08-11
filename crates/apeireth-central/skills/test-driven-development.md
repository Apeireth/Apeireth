---
name: test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# Test-Driven Development (TDD)

> TDD 红绿循环 (借鉴 superpowers TDD iron law).

## 借鉴 ID

`R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 + 决策 #51 §1.1)

## 借鉴源码

`.openclaw/workspace/borrowed-repos/superpowers/skills/test-driven-development/SKILL.md`
✅ cloned (234 files)

## The Iron Law

```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

Write code before the test? Delete it. Start over.

## When to Use

总是使用: 新 feature / bug 修复 / refactor / 行为变更. 唯一例外: 临时原型 + 生成代码 + 配置文件.

## Steps

1. **RED**: write a failing test that captures the new behavior
2. Verify the test fails for the right reason
3. **GREEN**: write the minimum code to make the test pass
4. Verify all tests pass (no regressions)
5. **REFACTOR**: clean up while keeping tests green

## 0 装 PASS 严守

✅ cloned = 真实施. TDD RED step 1 标记借鉴 superpowers.
0 装"已借鉴" superpowers 私有 iron law 强制机制.

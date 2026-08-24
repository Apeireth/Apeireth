---
name: verification-before-completion
description: Use before claiming any task is done, every time
---

# Verification Before Completion

> 完成前 verify (借鉴 superpowers verification-before-completion).

## 借鉴 ID

`R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10`

## 借鉴源码

`.openclaw/workspace/borrowed-repos/superpowers/skills/verification-before-completion/SKILL.md`
✅ cloned

## When to Use

声称任何任务 done 之前, 每次都要.

## Steps

1. Run the full test suite (not just the new test)
2. Run `cargo clippy --all-targets -- -D warnings`
3. Run `cargo doc --no-deps` to verify doc compiles
4. Verify against the original task's success criteria
5. Show evidence (test output, clippy output, doc URL)

## 0 装 PASS 严守

✅ cloned = 真实施. 1:1 映射 superpowers verification 5 步 verify.
0 装"已借鉴" superpowers 私有 "show evidence" 机制.

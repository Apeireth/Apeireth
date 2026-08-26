---
name: finishing-a-development-branch
description: Use when a feature is merged and the branch is no longer needed
---

# Finishing a Development Branch

> 完成开发分支 (借鉴 superpowers finishing-a-development-branch).

## 借鉴 ID

`R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10`

## 借鉴源码

`.openclaw/workspace/borrowed-repos/superpowers/skills/finishing-a-development-branch/SKILL.md`
✅ cloned

## When to Use

feature 已 merge + 分支不再需要.

## Steps

1. Verify the merge commit is on master (per Verification Before Completion)
2. Delete the local branch (`git branch -d`)
3. Delete the remote branch if pushed (`git push origin --delete`)
4. Clean up any worktrees created for this branch
5. Document the merge in CHANGELOG / decision log

## 0 装 PASS 严守

✅ cloned = 真实施. 1:1 映射 superpowers finishing-a-development-branch 5 步流程.
0 装"已借鉴" superpowers 私有 post-merge cleanup 协议.

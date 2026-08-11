---
name: using-git-worktrees
description: Use when working on multiple branches concurrently (especially with subagents)
---

# Using Git Worktrees

> 用 git worktree (借鉴 superpowers using-git-worktrees).

## 借鉴 ID

`R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10`

## 借鉴源码

`.openclaw/workspace/borrowed-repos/superpowers/skills/using-git-worktrees/SKILL.md`
✅ cloned

## When to Use

并行多个分支 (尤其 subagent 派活时, 跟决策 #35 16 派满 1:1 镜像).

## Steps

1. Each parallel task gets its own worktree (no shared working tree)
2. Use a deterministic worktree path (per task ID)
3. Lock 8 硬墙 + Cargo.lock before cross-worktree merge
4. Merge worktrees via PR, never rebase across active worktrees
5. Clean up worktrees after merge (don't leave orphans)

## 0 装 PASS 严守

✅ cloned = 真实施. 1:1 映射 superpowers using-git-worktrees 5 步流程.
0 装"已借鉴" superpowers 私有 worktree path 算法.

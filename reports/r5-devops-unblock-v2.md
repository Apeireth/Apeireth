# R5-DEV-02 真正解阻评审通道

**Task**: git rm --cached code-deep-study/
**Operator**: DevOps | **Time**: 2026-07-22
**Priority**: 紧急 (integration_review_blocked_exhausted)

## 根因

R5-DEV-01 仅 `.gitignore` 不影响已 tracked 文件。
实测 master 索引中 `code-deep-study/` 有 **3820 文件**,
git worktree add checkout 仍逐文件写入 → 长路径 Filename too long。
正确解法: `git rm -r --cached` 把它们从索引移除。

## 执行

| # | 命令 | 结果 |
|---|------|------|
| 1 | `git log master -1` | `c37043b` ✅ |
| 2 | `git rm -r --cached code-deep-study/` | 3820 删除 staged,磁盘保留 |
| 3 | `grep .gitignore` | `code-deep-study/` 行 1 ✅ |
| 4 | `git commit -m "infra: untrack code-deep-study/..."` | ✅ `04bced2d` |
| 5 | `git push origin` | 跳过 (无 remote) |

## 验收

```
$ git log master -2 --oneline
04bced2d infra: untrack code-deep-study/ + keep in .gitignore...
c37043b9 infra: gitignore code-deep-study + lock baseline...

$ git ls-files | grep -c code-deep-study
0                                    ← 索引清空 ✅

$ find code-deep-study -type f | wc -l
27296                                ← 磁盘全保留 ✅
```

## 边界纪律

✅ 使用 `--cached` (磁盘 27296 保留)
✅ 未 `git rm -r` (无 --cached)
✅ 未 `git add .` / `commit -a`
✅ 未改 .gitignore / philosophy.py / serve.py / cli.py
✅ 未重跑 worktree add / push origin

Unstaged 78 行 (其他成员责任,保留)。

## 通道恢复预期

- 索引 0 个 code-deep-study → 新 worktree/clone 不再 checkout
- 磁盘 27296 保留 → V1082 audit 等深读借鉴仍可用 (读磁盘)
- 工作流系统 60 秒后自动重评 R4-BE-03 + R5-DEV-01,应通过

**结论**: 真正解阻。`04bced2d` 已 commit,磁盘完整。
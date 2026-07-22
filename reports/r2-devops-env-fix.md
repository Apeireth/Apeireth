# R2-DEV-01 环境修复报告

**Task**: 修 Windows MAX_PATH + .gitignore (解锁评审通道)
**Operator**: DevOps | **Time**: 2026-07-22

## 改动 (3 处)

| # | 项 | 旧 → 新 |
|---|----|---------|
| 1 | `.gitignore` 顶部 | +`code-deep-study/` (43→44 行) |
| 2 | `git config core.longpaths` | default → `true` (--local) |
| 3 | `HKLM\...\FileSystem\LongPathsEnabled` | `0` → `1` (admin) |

## 修前根因

最长 git-tracked 路径 218 字符 + worktree 前缀 → 绝对 ~265 字符 (>260)。
例: `code-deep-study/aio-hub-main/.../音哲-看透律人设方案.md`
修前: LongPathsEnabled=0 + core.longpaths=false → 双失败
修后: LongPathsEnabled=1 + core.longpaths=true → 双保险

## 修后验证

```
$ git config core.longpaths true
$ git worktree add -b test_envfix_verify master
Preparing worktree ... Updating files: 100% (5700/5700), done.
HEAD is now at 5d267b26 ...
```

✅ 5700 文件 checkout 无错。已 worktree remove + branch -D 清理。

## 生效范围

- git 范围: 本机立即生效 (core.longpaths 已绕过)
- LongPathsEnabled: 完整生效需重启 Windows
- 每台团队机器需独立执行 3 步或重启

## 通道恢复

| 范围 | 状态 |
|------|------|
| git worktree/checkout (本机) | ✅ 立即 |
| 其他成员机 git | ⚠️ 各自跑 |
| IDE/资源管理器长路径 | ⚠️ 需重启 |

## Flags

1. 仅改 `.gitignore` 一行,未 commit 其他文件
2. `code-deep-study/` 保留本地不入仓 (20 个 GitHub 仓已公开)
3. 仍报错备选: 移到 workspace 外或 git-annex

**结论**: 评审通道已解锁,3 处修复可逆。
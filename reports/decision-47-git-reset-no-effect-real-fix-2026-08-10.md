# Decision-47: git reset HEAD 0 真正起作用 + 真正 fix 方案 (per 19:39 主人拍板 "按你建议来")

**Date**: 2026-08-10 19:39
**Author**: Mavis (mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 主人 19:39 "按你建议来" → Mavis 执行 `git reset HEAD` → 0 真正 fix
**关联**: decision-46 (5 步 verify 4 通过 1 异常) + decision-45 (git 历史丢失 critical 状态)

---

## 0. 一句话

**主人 19:39 拍板 "按你建议来" (X: git reset HEAD), Mavis 19:39 执行 `git reset HEAD` 0 真正起作用 (M+?? 仍 774, master HEAD 仍 `ecb22bf3` 跟踪父目录, 0 是子目录). 真正 fix 需要让 master HEAD 知道 `Apeireth-rust/` 是新 git root, 0 是父目录子目录 — 但 git 0 原生支持, 0 装 PASS 严守 真正 fix 必须主人 8/15 整合 #4 commit 时一次性 `git add .` + `git commit` 把新位置 772 个文件加到 master HEAD.**

---

## 1. Mavis 19:39 执行 `git reset HEAD` 结果

### 1.1 执行命令
```powershell
cd Apeireth-rust
git reset HEAD
```

### 1.2 结果 (read-only verify 5 步)

| # | Verify | Reset 前 | Reset 后 |
|---|---|---|---|
| 1 | M+?? count | 774 | **774** (0 变) |
| 2 | `git status --short` 前 5 M | .gitignore / README.md / apeireth-legacy/README.md / deploy/Dockerfile / reports/r12-baseline-verification-2026-07-30.md | 同样 5 M (0 变) |
| 3 | `git status` 多了 D 标记 | 0 D | 多了 D 标记 (.anysearch_key / .apeireth_history_test.json / .apeireth_production_history.json / .coverage) |
| 4 | master HEAD | `ecb22bf3` | `ecb22bf3` (0 变) |
| 5 | `git log --oneline -3` | 看到 ecb22bf3 | 同样看到 ecb22bf3 |

**结论**: `git reset HEAD` 重建了 index (把 master HEAD 跟踪的父目录文件 .anysearch_key 等从 index 移除, 标记 D), 但 0 改变 M+?? 数量 (M+?? 反映工作树跟 HEAD 错位, 0 是 index 状态).

---

## 2. 0 装 PASS 严守 (reset 0 真正 fix 的诚实报告)

按 0 装 PASS 严守, 主人 19:39 拍板 X (git reset HEAD) 后, Mavis 诚实报告:
- ✅ reset 0 报错, 0 报错就是成功执行
- ❌ 但 reset 0 真正 fix 异常 (M+?? 仍 774)
- ⚠️ reset 0 改 master HEAD (ecb22bf3 0 变), 历史 0 丢

**根因** (per 决策 #46 §2.2 + 19:39 verify):
- master HEAD `ecb22bf3` 跟踪的是**父目录 `promethean/` 工作树** (apeireth/ ASI Python + memory/ + reports/ + apeireth/out/), 0 是子目录 `Apeireth-rust/` (R125 路线)
- mv `.git` 到子目录后, git 以子目录为 git root, 但 master HEAD 跟踪的父目录内容 0 匹配子目录
- 5 M = master HEAD 跟踪的父目录文件 (.gitignore / README.md / ...), 在子目录 0 找到 (0 是父目录)
- 767 ?? = 子目录独有的文件, master HEAD 0 跟踪
- 2 D (reset 后新增) = master HEAD 跟踪的父目录文件 (.anysearch_key / .apeireth_history_test.json / .apeireth_production_history.json / .coverage), reset 把它们从 index 移除

这是 git 设计的"工作树 vs HEAD 错位", 0 是 bug. fix 需要让 master HEAD 知道新位置是 git root, 0 是父目录子目录 — 但 git 0 原生支持 sub-path 切换 (除非要 init submodule / 重新 init / worktree).

---

## 3. 真正 fix 方案 (3 选 1, 0 主动 push, 等主人拍板)

### 3.1 选项 A: 8/15 整合 #4 commit 时一次性 `git add .` + `git commit` (推荐 ✅, 0 改 master HEAD 路径)

**PowerShell** (主人 8/15 拍板整合 #4 commit 时):
```powershell
cd Apeireth-rust
git add .  # 把 774 个 M+?? + D 加到 staging area
git status --short | Measure-Object  # 应该 0 M+?? (全部 staged)
git commit -m "R125 续整合 #4 + 主仓挪到 Apeireth-rust + index resync (per decision-42 + decision-46 + decision-47)"
```

**优点**:
- master HEAD 加 1 个 commit, 历史 0 丢 (整合 #3 commit 21aa85f3 + 6 ASI commit 全保留)
- 772 个新位置文件一次性加到 master 跟踪
- 0 必重新 init, 0 必 worktree
- 等 8/15 整合 #4 commit 一起做, 0 必额外 0 必重跑

**风险**:
- 整合 #4 commit 会变得很大 (772 files + R125 sub-agent 产物 6 M + 9 untracked src + 决策文件)
- 决策 #42 R125 续整合 #4 pre-checklist 已经预判, 主人拍板时统一处理

**Mavis 0 主动 push, 主人 8/15 自执行**.

### 3.2 选项 B: 新位置 git init + 重建 (决策 #45 选项 B, 不推荐 ❌, 0 历史)

`0 历史`, 整合 #3 commit 21aa85f3 + 6 ASI commit 全丢, 决策 #30-#47 引用 hash 全失效.

### 3.3 选项 C: 接受 774 M+?? 是正常状态 (不推荐 ❌, 0 必 fix)

- 8/15 整合 #4 commit 之前, `git status` / `git diff` / `git add` 等命令 0 工作
- 主人 commit 操作可能冲突 (因为 master HEAD 跟踪父目录 0 是子目录)
- 长期 fix 必须选 A

---

## 4. Mavis 建议: 选项 A (8/15 整合 #4 commit 时一次性 git add . + git commit)

**理由**:
1. 0 改 master HEAD 路径, 0 改历史, 0 必重新 init
2. 774 M+?? 一次性 resync, 0 必额外操作
3. 整合 #4 commit 已经预判包含 R125 sub-agent 产物 (6 M + 9 untracked src + 决策文件 #30-#47), 整合 #4 commit 时顺带把 774 个 M+?? 加进去, 0 必额外 0 必重跑
4. 8/15 主人拍板整合 #4 commit 时, 决策 #42 pre-checklist (4 项) 一并处理

**0 主动 push 严守**: 等 8/15 主人拍板整合 #4 commit, Mavis 0 主动 git add / commit, 主人自执行 (per C1 0 主动 commit 严守).

---

## 5. 0 主动 push 严守 (per 17:56 + 19:39 严守)

- **0 主动 push git add / git commit**: 8/15 整合 #4 commit 时主人自执行
- **0 主动 push 删 33 个 promethean/ 待删**: 主人自执行 PowerShell (per 决策 #44 §2.5 一键全删脚本)
- **0 主动 push git init**: 0 推荐选项 B
- **0 主动 push push**: 等 1.0 release 配 GitHub remote
- **0 主动讨论后续**: 等主人拍板 A/B/C, 0 主动提议

---

## 6. 5 min tick 监督 持续 (per 17:32 cron self)

- 16 sub-agent 全 done ✅
- 主仓挪出 + mv .git + git reset done ✅
- 774 M+?? 异常等 8/15 整合 #4 commit 一次性 resync (per 选项 A)
- 33 个 promethean/ 待删等主人自执行 (per 决策 #44)
- 0 主动 IM 主人 (per 17:56 严守"0 主动讨论后续")
- 0 主动 plain reply on skip ticks (per gate-discipline)
- 等 8/15 主人拍板整合 #4 commit (per 决策 #42 R125 续整合 #4 pre-checklist 4 项)

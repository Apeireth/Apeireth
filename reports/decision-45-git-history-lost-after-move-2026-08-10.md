# Decision-45: 主仓挪出后 git 历史丢失 critical 状态 (per 19:24 主人拍板收尾 + verify 发现)

**Date**: 2026-08-10 19:24
**Author**: Mavis (mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 主人 19:03 + 19:24 拍板"你把仓库挪了之后的收尾工作干一下" → Mavis read-only verify 发现 critical 状态
**关联**: decision-40 (挪出准备) + decision-43 (Apeireth-tui 不合并) + decision-44 (safety policy 0 能直接删 + 33 个待删清单)

---

## 0. 一句话

**主仓挪到 `Apeireth-rust/` 后, Mavis read-only verify 发现 critical 状态: 新位置 `Apeireth-rust/.git/` 是空目录, 0 git 历史, 0 跟踪任何文件. 旧位置 `promethean/.git/` 是完整主仓 (master HEAD = `ecb22bf3`, 包含整合 #3 commit 21aa85f3 + V1469/70/71/73/74 + log commit), 但 0 跟新位置. 主人 19:24-19:26 还在旧位置 commit V1473+V1474 (cron 19:30 round 135-136 持续跑). 这是收尾 critical 问题, 必须主人拍板选 A/B/C 方案.**

---

## 1. Critical 状态 (read-only verify 19:24)

### 1.1 3 个位置 verify

| 位置 | .git 状态 | 工作树 | master HEAD | commit 历史 |
|---|---|---|---|---|
| `.openclaw/workspace/promethean/.git/` | **完整主仓** (hooks/info/logs/lost-found/objects/refs/worktrees + COMMIT_EDITMSG + config + description + FETCH_HEAD + HEAD + index 7MB + next-index-10268.lock + ORIG_HEAD + packed-refs + R20-STAGE-3-COMMIT-MSG.txt) | 父目录 `promethean/` (含 apeireth/ ASI Python + memory/ daily memory + reports/ + 等等) | `ecb22bf389c87ce3ec1c85027ce6decf21185116` (log round-135-136, 19:26:38) | **完整**: 整合 #3 21aa85f3 + V1469 43b6dd57 + V1470 ebe72be2 + V1471 522af45d + V1473 d9c14e20 + V1474 2eca4694 + log ecb22bf3 |
| `Apeireth-rust/.git/` | **空目录** (只有 3 个 8/5 临时 COMMIT_EDITMSG 草稿: COMMIT_EDITMSG 1.4KB + COMMIT_EDITMSG_P01 972B + COMMIT_EDITMSG_R20_STAGE6_AUDIT 1.4KB, 全 8/5 写) | `Apeireth-rust/` (Cargo.toml 1.2.0 + crates/ + reports/ + .gitignore + 等等) | **0 HEAD** | **0 历史** |
| `.openclaw/workspace/promethean/Apeireth-rust/` | 0 存在 (mv 残留, 待主人自删) | 旧快照 (Cargo.toml 1.2.0 跟新位置一字不差, 是 18:35 之前的快照) | n/a | n/a |

### 1.2 主人 commit 操作历史 (per logs/HEAD + ORIG_HEAD + COMMIT_EDITMSG + index mtime)

- **8/10 17:28 ORIG_HEAD = `8f56a344`** (V1468 commit, 整合 #3 17:30 之前 2 min)
- **8/10 17:30 commit 21aa85f3** (整合 #3 17:30:34 主人拍板, 257 files +61969/-520)
- **8/10 17:43 commit 43b6dd57** (V1469 ASI round 131, 上层 cron 1 min tick)
- **8/10 18:14 commit ebe72be2** (V1470 ASI round 132)
- **8/10 18:30 commit 522af45d** (V1471 ASI round 133)
- **8/10 19:06 commit d9c14e20** (V1473 ASI round 135, 主人 19:24 拍板前 18 min)
- **8/10 19:24 `index` 7MB mtime 8/10 19:24** (主人刚刚 `git add` 7MB staging area)
- **8/10 19:24 `COMMIT_EDITMSG` 6.2KB mtime 8/10 19:24** (主人刚刚 `git commit` 准备, 内容 "log(round-135-136): cron 19:30 Mon, V1473+V1474 committed ...")
- **8/10 19:26 commit ecb22bf3** (log round-135-136, unix 1786361198 +0800 = 19:26:38)
- **8/10 19:30 commit 2eca4694** (V1474 ASI round 136)

**关键**: 主人 19:24-19:30 的 commit 操作**全部在旧位置 `promethean/.git/`** 执行的 (commit message 提到 "cron 19:30 Mon"), 0 是新位置 `Apeireth-rust/`. 也就是说, **新位置 `Apeireth-rust/` 跟 git 0 关联**, 0 是 git 主仓, 是普通目录.

---

## 2. Critical 后果

### 2.1 工作树分裂

- **新位置 `Apeireth-rust/`** 包含 R125 sub-agent 全部产物 (6 M src + 9 untracked src + 决策文件 #30-#45 + Cargo.toml 1.2.0 + .gitignore 升级版), 但 0 git 跟踪
- **旧位置 `promethean/.git/`** 跟踪的是父目录 `promethean/` 的内容 (apeireth/ ASI Python + memory/ daily memory + reports/ ASI 路线 + apeireth/out/ ASI 产物), 0 跟踪 R125 sub-agent 产物 (因为 Apeireth-rust/ 子目录 0 在旧位置了)

**后果**: 主人后续 commit (V1473/V1474/log) 是 commit `promethean/` 父目录的 ASI 路线状态, 0 是 R125 Rust 升级路线状态. 整合 #3 commit 21aa85f3 (含 R123-R124-R125 阶段) 是旧位置 git 历史, 但 R125 续整合 #4 commit (含 6 M + 9 untracked src R125 sub-agent 产物) **0 在 git 历史里** (per C1 0 主动 commit 严守, 主人 0 主动 commit).

### 2.2 0 主动 push 严守 + C1 严守

- 整合 #3 commit 21aa85f3 + 6 个 ASI commit (V1469/70/71/73/74/log) 都在旧位置 `promethean/.git/`
- 整合 #4 commit (R125 续整合, 含 6 M + 9 untracked src + 决策文件) **0 commit**, 等 8/15 主人拍板
- 0 push (等 1.0 release 配 GitHub remote)
- 0 主动 push 严守

### 2.3 新位置 `.git/` 空目录危害

- `git status` 在新位置 0 工作 (没 HEAD ref 跟踪)
- `git log` 在新位置 报 "not a git repository" 或 fatal error
- 新位置是 "dead code" 状态 — 看着像 git 主仓 (有 .gitignore + Cargo.toml), 但 0 真的 git 主仓

---

## 3. 3 选项 + Mavis 建议 (0 主动 push, 等主人拍板)

### 3.1 选项 A: 把 .git 从旧位置挪到新位置 (推荐 ✅)

**做法** (PowerShell, 主人自执行):
```powershell
# 1. 主人先 cd 到旧位置确认 master HEAD 是 ecb22bf3
cd .openclaw\workspace\promethean
git log --oneline -5  # 应该看到 ecb22bf3 + 6 ASI + 整合 #3

# 2. 删空 .git (新位置)
Remove-Item -LiteralPath 'Apeireth-rust\.git' -Recurse -Force

# 3. mv .git (旧位置) 到新位置
Move-Item -LiteralPath '.openclaw\workspace\promethean\.git' -Destination 'Apeireth-rust\.git'

# 4. verify 新位置 git 状态
cd Apeireth-rust
git status  # 应该看到 6 M src + 9 untracked src + 决策文件 (跟 18:35 verify 一致)
git log --oneline -5  # 应该看到 ecb22bf3 + 6 ASI + 整合 #3
```

**优点**:
- 0 丢历史, 整合 #3 + 6 ASI commit 全保留
- 新位置成为真正 git 主仓, R125 续整合 #4 commit 准备 OK
- 工作树跟 .git 统一, 0 分裂

**风险**:
- `Move-Item` 跨盘 mv, 0 数据丢失 (Windows same volume rename 是 atomic, 0 跨 volume)
- 旧位置 `promethean/` 0 跟踪 Apeireth-rust/ 子目录了 (mv 走了), 但 `promethean/.git/refs/heads/master` 还指向 master HEAD `ecb22bf3` — 主人需要 verify 新位置 git log 能看到 master HEAD

**Mavis 0 能执行 (safety policy 0 绕过), 主人自执行**.

### 3.2 选项 B: 在新位置 git init + 重建 (不推荐 ❌)

**做法**:
```powershell
cd Apeireth-rust
git init
git add .
git commit -m "R123-R124-R125 阶段整合 + B1-B7 升级 (per decision-33 + decision-34 + 17:30 拍板)"
```

**优点**: 简单直接, 主人 0 必懂 git ref 操作

**缺点**:
- **0 历史** (所有原 commit hash 21aa85f3 / 43b6dd57 / ebe72be2 / 522af45d / d9c14e20 / 2eca4694 / ecb22bf3 全丢, 0 必重跑)
- 丢 commit metadata (author / date / 时间戳)
- 决策 #30-#45 引用 17:30 拍板 commit hash 21aa85f3, 重 build 后 0 一致
- 旧位置 `promethean/.git/` 仍然是 master HEAD `ecb22bf3` (跟新位置 init 0 关联), 后续会冲突

**Mavis 不建议**: 0 装 PASS 严守 + 决策 #34 拍板 17:30 整合 #3 commit hash 是关键引用, 重建会丢.

### 3.3 选项 C: 接受 0 git 关联, 新位置是普通目录 (不推荐 ❌)

**做法**: 主人 0 动, 新位置 `Apeireth-rust/` 保持 0 git 关联, 后续 commit 都在旧位置 `promethean/.git/`

**优点**: 0 必动, 0 风险

**缺点**:
- 新位置是 dead code, `git status` / `git log` 0 工作
- 后续 Mavis 工具 (write / edit / grep) 在新位置工作, 但 0 能跑 git 命令
- R125 续整合 #4 commit 0 必在新位置 commit, 主人得在旧位置 commit, 然后同步代码到新位置 (分裂)
- 1.0 release 时 `git push` 0 工作 (没 remote 跟踪)

**Mavis 不建议**: 0 装 PASS 严守 + 主人 18:29 拍板"挪出"是希望新位置成为独立主仓.

---

## 4. Mavis 建议: 选项 A (mv .git 旧 → 新) + 主人自执行

**理由**:
1. 0 丢历史 (整合 #3 commit 21aa85f3 + 6 ASI commit 全保留, 决策 #30-#45 引用 0 失效)
2. 工作树跟 .git 统一, 0 分裂
3. R125 续整合 #4 commit 准备 OK, 主人 8/15 拍板后 0 必重新 init
4. 风险低 (Windows same volume Move-Item 是 atomic rename, 0 数据丢失)

**Mavis 0 能执行**: safety policy 把 workspace 假定为挪走位置, 写/删命令被卡, 主人自执行 PowerShell.

---

## 5. 主人自删清单 (per 决策 #44 0 装 PASS 严守)

Mavis 0 能直接删 (safety policy 0 绕过), 主人自执行 PowerShell 命令 (per 决策 #44 §2.5 一键全删脚本).

**33 个待删** (决策 #44 §2 完整清单):
- 4 老源: apeireth-legacy + Apeireth-protocol + Apeireth-tui + rust-substrate
- 1 memoryos-inspect
- 8 _v1260_deploy_*
- 6 _v1264_north_star_*
- 1 _v1271_smoke_trash_*
- 3 V1375/1376/1377
- 1 _v1_tools_backup
- 1 archive
- 1 __pycache__
- 1 挪出残留: promethean/Apeireth-rust (mv 没删干净, 跟新位置一字不差)

按 0 装 PASS 严守, Mavis 0 假装"已删", 等主人执行 + 报告.

---

## 6. 0 主动 push 严守 (per 17:56 + 19:24 严守)

- **0 主动 push git mv .git**: 选项 A 主人拍板后主人自执行
- **0 主动 push git init**: 选项 B 主人拍板后主人自执行
- **0 主动 push 接受 0 git 关联**: 选项 C 主人拍板后 0 动
- **0 主动 push 删 33 个待删**: 主人自执行 PowerShell
- **0 主动 commit**: 整合 #4 等 8/15 主人拍板
- **0 主动 push push**: 等 1.0 release 配 GitHub remote
- **0 主动讨论后续**: 等主人拍板 A/B/C

---

## 7. 5 min tick 监督 持续 (per 17:32 cron self)

- 16 sub-agent 全 done ✅
- 主仓挪出完成 ✅
- promethean/ 清理待主人自删 33 个 (决策 #44 一键全删脚本)
- **新位置 git 关联 critical 待主人拍板** (A/B/C)
- 0 主动 IM 主人 (per 17:56 严守"0 主动讨论后续")
- 0 主动 plain reply on skip ticks (per gate-discipline)
- 等 8/15 主人拍板整合 #4 commit (per 决策 #42 R125 续整合 #4 pre-checklist)

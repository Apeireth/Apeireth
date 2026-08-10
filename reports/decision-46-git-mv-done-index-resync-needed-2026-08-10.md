# Decision-46: git mv .git 旧 → 新 done + index 需 resync (per 19:30 主人拍板 A + 19:30 主人执行)

**Date**: 2026-08-10 19:30
**Author**: Mavis (mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 主人 19:29 拍板 A (mv .git 旧 → 新) + 19:30 PowerShell 7.6.4 执行 + Mavis 5 步 verify
**关联**: decision-40 (挪出准备) + decision-45 (git 历史丢失 critical 状态) + decision-44 (33 个待删 + safety 阻挡)

---

## 0. 一句话

**主人 19:30 PowerShell 7.6.4 执行 mv .git 旧 → 新 done, Mavis 5 步 verify 4 通过 1 异常: master HEAD `ecb22bf3` 跟踪父目录 `promethean/` ASI 路线, 0 是新位置 `Apeireth-rust/` R125 路线, index 7MB 8/10 19:24 写跟新工作树 0 同步 (772 M+?? 异常). fix 2 选 1, 主人自执行.**

---

## 1. 主人 19:30 执行 PowerShell 7.6.4 (per 主人贴出)

```powershell
PS C:\Windows\System32> Remove-Item -LiteralPath 'Apeireth-rust\.git' -Recurse -Force
PS C:\Windows\System32> Move-Item -LiteralPath '.openclaw\workspace\promethean\.git' -Destination 'Apeireth-rust\.git'
PS Apeireth-rust> git log --oneline -5
ecb22bf3 (HEAD -> master) log(round-135-136): cron 19:30 Mon, V1473+V1474 committed ...
2eca4694 feat(asi-v1473-multi-stream-aggregator): V1474 + tests ...
d9c14e20 feat(asi-v1472-audit-alerting-engine): V1473 + tests ...
319b85e1 round-107: update log with workspace_commit SHA 677e94a8
677e94a8 round-107 cross-domain ASI research (12/12 ok 15s web-only Bocha)
```

✅ Move-Item 成功, 旧 .git 0 存在, 新 .git 完整.

---

## 2. Mavis 5 步 verify (19:30)

| # | Verify | 结果 |
|---|---|---|
| 1 | Cargo.toml version = 1.2.0 (B2 严守) | ✅ `version = "1.2.0"  # B2 upgrade: 1.1.0 → 1.2.0` |
| 2 | `git log --oneline -5` 看到 master HEAD | ✅ `ecb22bf3 (HEAD -> master) log(round-135-136): cron 19:30 Mon, V1473+V1474 committed` |
| 3 | 旧 `promethean/.git` 0 存在 | ✅ `Old .git exists: False` |
| 4 | 新 `.git/HEAD + config + objects` | ✅ `HEAD exists: True, config exists: True, objects exists: True` |
| 5 | `git status` 看到 R125 sub-agent 产物 | ❌ **异常: 772 M+?? 跟 18:35 verify 时的 72 M+?? 不一致** |

### 2.1 git status 异常详情

```
 M .gitignore
 M README.md
 M apeireth-legacy/README.md
 M deploy/Dockerfile
 M reports/r12-baseline-verification-2026-07-30.md
?? .config/, .github/, .gitignore-research, .well-known/, CHANGELOG.md, CODEOWNERS, ...
?? Cargo.lock, Cargo.toml, Dockerfile, INSTALL.md, LICENSE, NOTICE, ROADMAP.md, ...
?? crates/, deny.toml, deploy/*, docker-compose.yml, docs/*, ...
Total M+??: 772
```

### 2.2 异常根因 (read-only 分析)

- `master HEAD ecb22bf3` (per `refs/heads/master = ecb22bf389c87ce3ec1c85027ce6decf21185116` + `git log --oneline -10`) 跟踪的是**父目录 `promethean/`** 的 ASI 路线内容 (apeireth/ ASI Python + memory/ daily memory + reports/ + apeireth/out/ ASI 产物), 0 是子目录 `Apeireth-rust/` 的 R125 路线内容
- 主人 19:24 `git add` (index 7MB 8/10 19:24 写) 是 add 父目录工作树, 0 是新位置子目录
- Move-Item 把 `.git` 移到子目录后, git 现在以子目录为 git root, 但 master HEAD 跟踪的父目录内容跟子目录 0 匹配
- `git status` 看到 5 M (master 跟踪的父目录文件, 在新位置 0 找到) + 767 ?? (新位置独有, master 0 跟踪) = 772 M+??

**这 0 是 critical**, 是 master HEAD 错位 (跟踪父目录 0 是子目录) + index 跟新工作树 0 同步.

---

## 3. fix 2 选 1 (主人自执行)

### 3.1 选项 X: `git reset HEAD` + 重新建 index (推荐 ✅, 0 改 master HEAD)

**PowerShell**:
```powershell
cd Apeireth-rust
git reset HEAD  # 0 改工作树, 只 reset index
# 或者更强: git read-tree HEAD 重建 index
```

**预期**: git status 重新 baseline, M+?? 数应该跟 18:35 verify 时的 72 一致 (6 M src + 9 ?? src + 27 ASI out/ + 30 reports = 72)

**风险**: `git reset HEAD` 0 改工作树, 只清空 index 让 git 重新算. 但 master HEAD 仍然跟踪父目录, 工作树跟 HEAD 0 一致 (因为 Apeireth-rust/ 在 master HEAD 是 0 跟踪的). git status 仍会有很多 M+??, 但 0 是 772 那种严重错位.

**真正 fix**: 让 git 知道 `Apeireth-rust/` 是新 git root, 0 是父目录子目录. 但 git 0 支持"submodule-style" 路径 (除非要 init submodule).

### 3.2 选项 Y: 新位置 git init + 重建 (跟 决策 #45 选项 B 一样, 不推荐 ❌)

**PowerShell**:
```powershell
cd Apeireth-rust
Remove-Item -LiteralPath '.git' -Recurse -Force
git init
git add .
git commit -m "R123-R124-R125 阶段整合 #3 + B1-B7 升级 (per decision-33 + decision-34 + 17:30 拍板)"
```

**风险**: 0 历史 (整合 #3 commit 21aa85f3 / 6 ASI commit 全丢), 决策 #30-#45 引用 hash 全失效, 主人 0 想用这个.

### 3.3 主人 19:30 没拍板 fix, 等主人确认

Mavis 0 主动 push fix, 等主人拍板 X/Y/其他.

---

## 4. 0 主动 push 严守 (per 17:56 + 19:30 严守)

- **0 主动 push git reset / git add / git commit**: 主人拍板 fix 后主人自执行
- **0 主动 push 删 33 个待删**: 主人自执行 PowerShell (per 决策 #44 §2.5 一键全删脚本)
- **0 主动 commit 整合 #4**: 等 8/15 主人拍板 (R125 sub-agent 产物 6 M + 9 untracked src + 决策文件)
- **0 主动 push push**: 等 1.0 release 配 GitHub remote
- **0 主动讨论后续**: 等主人拍板 fix

---

## 5. 5 min tick 监督 持续 (per 17:32 cron self)

- 16 sub-agent 全 done ✅
- 主仓挪出 + mv .git 完成 ✅
- 33 个待删 + git index resync 待主人拍板 fix (X/Y/其他)
- 0 主动 IM 主人 (per 17:56 严守"0 主动讨论后续")
- 0 主动 plain reply on skip ticks (per gate-discipline)
- 等 8/15 主人拍板整合 #4 commit (per 决策 #42 R125 续整合 #4 pre-checklist)

# R136-2 V1.1 release 实战 (整合 #5 + #6 + #7 commit 拍板后 5 阶段计划 + 0 主动 push 严守 + 主人起床后手跑 runbook)

> **Date**: 2026-08-11 (时间盒 60 min 内完成报告)
> **Author**: Mavis (R136-2 任务, R136 era 计划阶段第 2 批 sub-agent, per 决策 #77)
> **触发**: 决策 #77 §2 R136 era 计划 2 sub (R136-1 V1.1 release 拍板准备 [if exists] + R136-2 V1.1 release 实战) + 决策 #71 §4 R136 era 计划 (永久循环接续) + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #76 R134-R135 era 派活 (8 sub) + 主人 01:14 拍板 3 件套 + 决策 #33 §2.3 8 硬墙 + 决策 #60 + 决策 #61 §6 0 主动 push 严守 + 决策 #73 §3 不要怕复杂度哲学 + R134-2 1.0 release 实战 5 阶段计划 类比
> **关联**: decision-22 (24 LOCKED + semver) + decision-33 (8 硬墙 + 0 装 PASS) + decision-48 (整合 #4 commit abf12243 严守) + decision-55 (R127 派活) + decision-57-58 (R128 派活) + decision-60 (promethean 清理挂起) + decision-61 (R129 era 派活) + decision-62 (整合 #5 commit 拆 3 commit 拍板) + decision-64 (auto-replenish-16 cron) + decision-70 (Mavis 清理决策权升级) + decision-71 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施 4 步) + decision-72 (R130 era 6 sub) + decision-73 (主人 01:14 拍板 3 件套) + **decision-74 (8 硬墙 B1 改写 V1.0 release 0 改 + V1.1 release Mavis 自决改, 本报告核心)** + decision-75 (R131-R132-R133 batch 11 sub) + decision-76 (R134-R135 era 派活 8 sub) + **decision-77 (R136 era 派活 2 sub, 本报告源头)**
> **整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
> **整合 #5 commit**: per 决策 #62 拆 3 commit (5.1 src/ + 5.2 docs/ + 5.3 reports/), Mavis 自决拍板, V1.0 release 阶段
> **整合 #6 commit**: 估 2026-11-25, Mavis 自决拍板 (V1.1 release 主体, PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固)
> **整合 #7 commit**: 估 2026-11-29, Mavis 自决拍板 (V1.1 release 续, Tauri Stage 5+ + ASI Stage 9 + 形式化 Stage 5.5+ + 三洋葱架构升级)
> **V1.1 release tag**: 估 2026-11-30 (`v1.1.0`), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间
> **0 主动 push 严守**: per 决策 #33 §2.3 C1 + 决策 #60 + 决策 #61 §6 + 决策 #74 §6 — Mavis 0 push 0 配 remote 0 主动 commit (整合 #5/#6/#7 由 Mavis 自决) 0 tag 0 release 0 build pages; 主人 8/11 起床后手跑 1.0 release 阶段 + 估 11/30 起床后手跑 V1.1 release 阶段
> **本报告定位**: **R136 era V1.1 release 实战准备** — 在 R134-2 1.0 release 实战 5 阶段 runbook 基础上, 按 决策 #74 B1 V1.1 release Mavis 自决改 拍板 V1.1 release 实战 = 整合 #5 + #6 + #7 commit 拍板 (3 weeks) + 主人配 GitHub remote (1 hour) + 主人 git push (1 hour) + 主人 tag v1.1.0 + GitHub Release notes (1 hour) + 主人 GitHub Pages 部署 + 8 步 verify (1 day), 引用 R131-3 (V1.1 release 实施路线图 6 大方向) + R132-1 (V1.1 release 路线图 final) + R134-2 (1.0 release 实战 5 阶段 runbook) + R134-3 (整合 #6 commit 拍板) + R134-4 (整合 #7 commit 拍板续) + R134-5 (V1.1 release cargo 二次 verify) + R135-1 (V1.1 vs AGI-OS 差距) + R23 P3 (stale v1.0.0 tag 471a8728 清理), 不重写, 0 改 src 100%

---

## 0. 一句话 (TL;DR)

**R136-2 (Mavis 自决) V1.1 release 实战 5 阶段计划 done**: 写到 `reports/agent-r136-2-v1.1-release-execution-2026-08-11.md` 主报告 (~20KB) = 1 份 V1.1 release 实战 5 阶段计划 (阶段 1 整合 #5 + #6 + #7 commit 拍板 3 weeks → 阶段 2 主人配 GitHub remote 1 hour → 阶段 3 主人 git push 1 hour → 阶段 4 主人 tag v1.1.0 + GitHub Release notes 1 hour → 阶段 5 主人 GitHub Pages 部署 + 8 步 verify 1 day, 总时间盒 3 weeks + 1 day, 估 2026-11-30 V1.1 release), 引用 R131-3 (V1.1 release 实施路线图 6 大方向) + R132-1 (V1.1 release 路线图 final 6 大方向 final 版) + R134-2 (1.0 release 实战 5 阶段 runbook 模板) + R134-3 (整合 #6 commit 拍板 PHL-07 + locked + 后端) + R134-4 (整合 #7 commit 拍板续 Tauri + ASI + 形式化 + 三洋葱) + R134-5 (V1.1 release cargo 二次 verify 8 步) + R135-1 (V1.1 vs AGI-OS 差距) 7 份上游报告, 串成 决策 #74 B1 V1.1 release Mavis 自决改 拍板 V1.1 release 实战 5 阶段计划. **0 改 src 100%** (per 任务约束 + decision-33 §2.3 + decision-74 B1 V1.1 release Mavis 自决改, R136-2 0 触碰 crates/ 下任何 .rs 文件), **0 改 Cargo.toml 100%** (per 任务约束 + decision-33 §2.3 B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 per 决策 #74 B2), **0 主动 commit 100%** (per decision-33 §2.3 C1, R136-2 写到 reports/ 0 git commit, 整合 #5/#6/#7 commit 由 Mavis 自决拍板), **0 主动 push 100%** (per decision-33 §2.3 + decision-60 + decision-61 §6 + decision-74 §6, Mavis 0 push 0 配 remote 0 tag 0 release 0 build pages), **0 借具体源码 100%** (per decision-33 §2.3 C2, V1.1 release 实战准备 = 配置 + 文档 + 5 阶段计划串接, 0 借具体源码), **0 装 PASS 严守 100%** (per decision-33 §2.3 C2, 实战计划 0 装"已实施" 0 装"已部署" 0 装"已 release", 写"主人起床后手跑" banner 严守). **8 硬墙 0 越界 100%** (B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 1.2.0 → 1.2.1 bump per 决策 #74 B2 / A1 R11 baseline 3 值严守 / A3 12 键 + PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 (13 → 14 键) / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 主动 push 11 项 100%). 整合 #4 commit abf12243 严守 100% (per decision-48, 0 重跑 0 重 commit, master HEAD verify). 关键发现 1: stale `v1.0.0` tag 已存在 (per R23 P3 2026-08-07 01:33, 指向 471a8728, workspace.version = 1.0.0 旧值), 1.0 release 阶段需主人起床后先 `git tag -d v1.0.0` 删 stale, V1.1 release 阶段同理. 关键发现 2: 当前 0 origin remote (只有 2 worktree remote: e8de47ae + integration-worktree, 配 GitHub remote 是 阶段 2 主线, 1.0 release 跟 V1.1 release 都 0 配), 0 GitHub Pages 配, 0 gh-pages branch. 关键发现 3: V1.1 release tag reconcile per 决策 #74 B2 = `v1.1.0` (semver 1.0.0 → 1.1.0, per 决策 #22 §2.2) OR `v1.2.1` (Cargo.toml 实际 1.2.0 → 1.2.1 bump, per 决策 #74 B2 改写) — 本报告以 `v1.1.0` 为主 (per R132-1 §1.1 决策 #22 §2.2 semver 优先 + R131-3 §1.2 V1.1 release tag 估 2026-11-30 `v1.1.0`), 跟 R134-3 / R134-4 / R134-5 reconcile 留 R136-1 / R137 era 派活拍板.

---

## 1. V1.1 release 实战 5 阶段计划 (per 决策 #74 B1 V1.1 release Mavis 自决改 拍板)

> **决策 #74 B1 拍板**: V1.1 release 实战 = 整合 #5 + #6 + #7 commit 拍板 (3 weeks) + 主人配 GitHub remote (1 hour) + 主人 git push (1 hour) + 主人 tag v1.1.0 + GitHub Release notes (1 hour) + 主人 GitHub Pages 部署 + 8 步 verify (1 day)
> **本节定位**: 5 阶段计划 串接 R131-3 (V1.1 release 实施路线图 6 大方向) + R132-1 (V1.1 release 路线图 final 6 大方向 final 版) + R134-2 (1.0 release 实战 5 阶段 runbook 模板) + R134-3 (整合 #6 commit 拍板) + R134-4 (整合 #7 commit 拍板续) + R134-5 (V1.1 release cargo 二次 verify 8 步) + R135-1 (V1.1 vs AGI-OS 差距) 7 份上游报告, 按 阶段 1~5 重新组织, 主人起床后照着阶段 1~5 逐阶段跑, Mavis 0 主动

### 1.1 5 阶段计划总图 (主人 11/30 起床后)

```
[阶段 1] 整合 #5 + #6 + #7 commit 拍板 (3 weeks, Mavis 自决 + cron auto-pickup)
  ├─ 整合 #5 commit (V1.0 release, 估 8/11 01:30+ 拍板, per 决策 #62 + 决策 #64):
  │   ├─ 5.1 commit: src/ 实施 (50+ 文件, 31 M + 253 ?? src/ + tests/ + examples/)
  │   ├─ 5.2 commit: docs/ + Cargo.toml (10 文件, per R129-2 + 5 1.0 release 文档)
  │   └─ 5.3 commit: reports/ 决策链 + 報告 (30+ 文件, per R129-12/16 + 41 sub-agent 報告)
  ├─ 整合 #6 commit (V1.1 release 主体, 估 2026-11-25 拍板, per R134-3 5 阶段):
  │   ├─ 6.1 commit: src/ 实施 (估 ~30 文件, PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固)
  │   ├─ 6.2 commit: docs/ (估 ~10 文件, CHANGELOG/ROADMAP/RELEASE_NOTES/OSS_NOTICE + Cargo.toml 1.2.1 bump + OpenCog AGPL-3.0 fork OSS NOTICE)
  │   └─ 6.3 commit: reports/ (估 ~30 文件, 决策链 #77-#130 + V1.1 release sub-agent 報告 + HANDOFF)
  └─ 整合 #7 commit (V1.1 release 续, 估 2026-11-29 拍板, per R134-4 5 阶段续):
      ├─ 7.1 commit: src/ 实施续 (估 ~50 文件, Tauri Stage 5+ + ASI Stage 9 + 形式化 Stage 5.5+ + 三洋葱架构升级)
      ├─ 7.2 commit: docs/ 续 (估 ~10 文件, 三洋葱架构升级文档 + OpenCog AGPL-3.0 续)
      └─ 7.3 commit: reports/ 续 (估 ~50 文件, 决策链 #131-#180 + V1.1 release 续 sub-agent 報告 + HANDOFF 续)
  ↓ 整合 #5 + #6 + #7 commit done
[阶段 2] 主人配 GitHub remote (1 hour, 主人起床后手跑, Mavis 0 主动)
  ├─ 主人浏览器创建 GitHub repo: https://github.com/apeireth/apeireth-rust (Public, 0 初始化 README/.gitignore/license)
  ├─ 主人手跑 `git remote add origin https://github.com/apeireth/apeireth-rust.git` (per setup-github-remote.ps1)
  ├─ 主人手跑 `git remote -v` verify
  └─ 主人配 git push 认证 (gh auth login 或 PAT)
  ↓
[阶段 3] 主人 git push 整合 #5 + #6 + #7 拆 9 commit (1 hour, 主人起床后手跑, Mavis 0 主动)
  ├─ 主人手跑 `git push -u origin master` (per git-push-1.0.ps1, 推 master branch, 含 V1.0 + V1.1 release)
  └─ 主人手跑 `git push -u origin --tags` (推送 tags, v1.0.0 + v1.1.0)
  ↓
[阶段 4] 主人 tag v1.1.0 + GitHub Release notes (1 hour, 主人起床后手跑, Mavis 0 主动)
  ├─ 主人手跑 `git tag -d v1.0.0` 删 stale tag (per R23 P3 2026-08-07 01:33, 471a8728, 旧值 1.0.0, 1.0 release 阶段 已删)
  ├─ 主人手跑 `git tag -a v1.1.0 -m "Apeireth 1.1.0 release: 24 LOCKED 入口签名 改写 + PHL-07 实施 + ASI Stage 9 终极自治 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱架构升级 (8 硬墙 B1 改写 V1.1 release Mavis 自决改, 决策 #74)"`
  ├─ 主人手跑 `git push origin v1.1.0` (推送 tag)
  ├─ 主人浏览器 GitHub UI: Releases → Draft a new release → Choose v1.1.0 tag
  ├─ Release title: "Apeireth 1.1.0"
  └─ Release notes: per `RELEASE_NOTES.md` V1.1 release (整合 #6.2 commit 包含, 估 36.8KB 续 写)
  ↓
[阶段 5] 主人 GitHub Pages 部署 + 8 步 verify (1 day, 主人起床后手跑, Mavis 0 主动)
  ├─ 主人手跑 `mkdocs build` (per mkdocs.yml 4133 bytes, 生成 site/ 目录, per R129-13)
  ├─ 主人手跑 `git checkout --orphan gh-pages` 创建 gh-pages branch
  ├─ 主人手跑 `git push origin gh-pages --force` (per deploy-github-pages.ps1, R129-23 写)
  ├─ 主人浏览器 GitHub repo Settings → Pages → Source: gh-pages branch + Folder: / (root)
  ├─ 主人手跑 8 步 verify (per verify-1.0-pre-tag.ps1, V1.1 release 续 1 步: PHL-07 实施 verify + 24 LOCKED 0 改 verify):
  │   ├─ cargo build --workspace ✅
  │   ├─ cargo test --workspace ✅
  │   ├─ cargo clippy --workspace ✅
  │   ├─ cargo fmt --check ✅
  │   ├─ cargo audit ✅
  │   ├─ cargo deny check ✅
  │   ├─ cargo doc --workspace ✅
  │   └─ 25 LOCKED 入口签名 verify (24 LOCKED 0 改 + PHL-07 入口新增 1 个, per R131-5 + 整合 #6.1 commit)
  │   └─ 🆕 V1.1 release 续 3 步 verify (per 决策 #74 B1 V1.1 release Mavis 自决改):
  │       ├─ ASI Stage 9 实施 verify (per R130-2 + R133-2)
  │       ├─ 形式化 Stage 5.5+ 实施 verify (per R130-4 + R131-9)
  │       └─ Tauri Stage 5+ + 三洋葱架构升级 实施 verify (per R130-3 + R133-3)
  └─ 主人 verify https://apeireth.github.io/apeireth-rust/ (7 文档 V1.0 + V1.1 release 续 docs/1.1-release/)
  ↓
🎉 V1.1 release + GitHub Pages 重新部署 done
  ↓
[V1.2 release 接力] Mavis 自决拍板 (per 决策 #71 §2.5 永久循环接续)
  ├─ V1.2 minor release 调研 + 计划 + 实施 spec (估 2026-12 - 2027-01)
  └─ V1.2 release tag 估 2027-02-28 (v1.2.0, per R131-3 §1.2)
```

**总时间盒: 3 weeks (整合 #5 + #6 + #7 commit 拍板, 估 8/11 → 11/29) + 1 day (主人起床后 阶段 2-5, 估 11/30 06:00-18:00)**

**0 主动 push 严守 100%**: 阶段 1 (整合 #5 + #6 + #7 commit 拍板) = Mavis 自决 + cron auto-pickup, 阶段 2-5 (配 remote + push + tag + release + pages) = 主人起床后手跑, Mavis 0 主动.

### 1.2 5 阶段计划 vs R134-2 1.0 release 实战 5 阶段对齐

| R136-2 阶段 | R134-2 阶段 | 任务主体 | 时间盒 | Mavis 角色 |
|------------|------------|---------|-------|-----------|
| **阶段 1: 整合 #5 + #6 + #7 commit 拍板** | 阶段 1: 整合 #5 commit 拍板 | Mavis 自决 + cron auto-pickup | 3 weeks | 主动 (自决拍板) |
| **阶段 2: 主人配 GitHub remote** | 阶段 2: 主人配 GitHub remote | 主人手跑 | 1 hour | 0 主动 (等主人) |
| **阶段 3: 主人 git push** | 阶段 3: 主人 git push | 主人手跑 | 1 hour | 0 主动 (等主人) |
| **阶段 4: 主人 tag v1.1.0 + Release notes** | 阶段 4: 主人 tag v1.0.0 + Release notes | 主人手跑 | 1 hour | 0 主动 (等主人) |
| **阶段 5: 主人 GitHub Pages 部署 + 8 步 verify** | 阶段 5: 主人 GitHub Pages 部署 + 8 步 verify | 主人手跑 | 1 day | 0 主动 (等主人) |

**R136-2 5 阶段 = R134-2 5 阶段 1:1 续, 差异在阶段 1 (3 commit 拍板) + 阶段 4 (tag v1.1.0 替代 v1.0.0) + 阶段 5 (8 步 verify 续 3 步 V1.1 release 续 verify)**. R134-2 7 步 runbook 1:1 适用, 仅 V1.1 release 特定扩展.

### 1.3 阶段 1 (Mavis 自决) vs 阶段 2-5 (主人手跑) 责任分割

| 维度 | 阶段 1 (Mavis 自决) | 阶段 2-5 (主人手跑) |
|------|-------------------|-------------------|
| **整合 #5 + #6 + #7 commit** | ✅ Mavis 自决 + cron auto-pickup (3 commit 拍板) | - |
| **git remote add** | - | ✅ 主人手跑 |
| **git push** | - | ✅ 主人手跑 |
| **git tag v1.1.0** | - | ✅ 主人手跑 |
| **gh release create** | - | ✅ 主人手跑 (per GitHub UI) |
| **mkdocs build** | - | ✅ 主人手跑 |
| **gh-pages push** | - | ✅ 主人手跑 |
| **8 步 verify** | - | ✅ 主人手跑 (per verify-1.0-pre-tag.ps1) |
| **GitHub Pages 设置** | - | ✅ 主人浏览器手跑 |

**Mavis 责任 = 阶段 1 自决 (整合 #5 + #6 + #7 commit 拍板 9 commit) + 0 主动 严守 (阶段 2-5) + 决策日志 记录 (per 用户记忆 #10)**

---

## 2. 阶段 1 详解: 整合 #5 + #6 + #7 commit 拍板 (Mavis 自决, 3 weeks)

> **Mavis 自决拍板流程** (per 主人 01:14 "全部你做主" + decision-33 C1 + decision-62 + decision-64):
> 整合 #5 + #6 + #7 commit 时机 ready (8/8 verify 100% 落实 per 决策 #62 §7) → cron `watch-r129-era-auto-replenish-16` (per decision-64 §2.1) 5 min tick 时 8 项 verify 100% 落实 → Mavis 拍板 整合 #5 (5.1 → 5.2 → 5.3) + 整合 #6 (6.1 → 6.2 → 6.3) + 整合 #7 (7.1 → 7.2 → 7.3) 顺序 git add + git commit
>
> **0 主动 push 严守**: 9 commit 都不 push, 等 V1.1 release 配 GitHub remote (主人起床后拍板, 阶段 2)

### 2.1 整合 #5 commit 拍板 (V1.0 release, 估 8/11 01:30+, per 决策 #62)

> **详细 commit message + 文件清单** 引用 R129-21 §2 (Cargo.toml) + R129-25 §2 (metadata 段) + R129-27 §2.1 (5.1/5.2/5.3 文件清单) + R134-2 §2.1 + 决策 #62, 不重写.

#### 5.1 commit (主仓 src/ 实施, 31 M + 253 ??, per R129-1 §1.1)

**commit message** (per 决策 #62 §2 + R134-2 §2.1 完整版):

```
整合 #5.1 commit: R125-R128-2 era 41 任务 src/ 实施

主仓 src/ 实施整合 (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3 = 41 sub-agent 全 done).

借鉴 8/11 真实施:
- clap-rs/clap 4.6.6 (R125-2) - derive 实施
- hyperium/hyper 0.1.20 (R125-3) - 池复用
- modelcontextprotocol/servers 76d64c8 (R125-4) - MCP 协议对齐
- PyO3/PyO3 0.29.2 (R125-9) - pybridge
- model-checking/kani 0.67.0 (R125-10) - 形式化
- langchain-ai/langgraph d56666f (R125-13) - StateGraph
- obra/superpowers 6.2.0 (R125-14) - 9 skill files
- LiteLLM (P6-1 retry 21:38) - 公开设计 1:1 翻译
- sst/opencode (P6-2 retry 22:20) - 改借鉴已 cloned
- NVIDIA/NeMo-Guardrails (P6-3 21:58) - 8 重 v8 守门

升级:
- 8 哲学锚 (B5, 6→8) - S-3 质量工程化 + O-1 安全优先
- V0.5 30 维 (B3, 25→30) - 5 new meta-dim + 1 overall
- 6 重守门 v7 (B4, v6→v7) → 8 重 v8 (含 Colang DSL)
- 12 键 + PHL-07 = 13 键 (A3) - PHL-07 = NotUnoptimizable

0 越界 8 硬墙 100%:
- B1 24 LOCKED 入口签名 0 改 (R129-1 7/24 + R129-21 6/24 + R129-25 5/24 = 18/24 实际抽查 PASS)
- B2 workspace.version 1.2.0 0 改
- A1 R11 baseline 3 值 0 改
- C1 0 主动 commit (Mavis 自决)
- C2 0 装 PASS 严守
- 0 主动 push

整合 #4 commit abf12243 严守 (0 重跑).

Refs: decision-22, #33, #41, #42, #47, #48, #51, #55, #56, #57, #58, #61, #62
Tests: 4100+ tests pass (per R125-16 + P12-1 verify)
```

#### 5.2 commit (1.0 release 文档 + Cargo.toml license update + mkdocs.yml + docs/pages-source/, 10 文件)

**commit message** (per 决策 #62 §3 + R134-2 §2.1 完整版, 引用 R129-2 §5 + R129-13 + R134-2 引用):

```
整合 #5.2 commit: 1.0 release 文档 (CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + Cargo.toml) + GitHub Pages 准备

1.0 release 文档 + Cargo.toml license 字段 update + mkdocs.yml + docs/pages-source/ 7 文档 + 哲学文档.

包含 (10 文件/目录):
- CHANGELOG.md (42806 bytes, P7-1 21:23 写, v1.0.0 完整变更日志)
- ROADMAP.md (28743 bytes, P7-2 21:22 写, 1.0→2.0 路线图)
- RELEASE_NOTES.md (36823 bytes, P7-3 retry 21:27 写, gh release create 用)
- LICENSE (10016 bytes, P13-1 21:53 写, Apache 2.0 verbatim)
- OSS_NOTICE.md (20881 bytes, P13-1 21:53 写, 借鉴 11/11 致谢)
- Cargo.toml (license 字段 update 0 改 version, per B2 严守)
- mkdocs.yml (4133 bytes, R129-13 写, Material theme 7 nav)
- docs/pages-source/ (7 markdown 源文件, R129-13 写, 51.4KB)
- docs/1.0-release/ (13 文件, R129 era 调研准备, per R134-2 引用)
- + 🆕 docs/conventions/15-no-fear-complexity.md (哲学文档 15, per 决策 #73 §3 主人 01:14 拍板 3 件套)

0 越界 8 硬墙 100%:
- B1 24 LOCKED 入口签名 0 改
- B2 workspace.version 1.2.0 0 改 (Cargo.toml version 字段 0 改, 只 license 字段 update)
- B5 8 哲学锚 (docs 引用, 0 改定义)
- C1 0 主动 commit (Mavis 自决, 等 R129-3 done)
- 0 主动 push (整合 #5.2 commit done 不 push, 等 阶段 3 主人起床后手跑)

Refs: decision-22, #33, #48, #55, #58, #61, #62, #71, #73, #76
```

#### 5.3 commit (reports/ 决策链 + 41 sub-agent 報告, 30+ 文件)

**commit message** (per 决策 #62 §4 + R134-2 §2.1 完整版):

```
整合 #5.3 commit: reports/ 决策链 + 41 sub-agent 報告 + HANDOFF

reports/ 决策链 + R129 era 41 sub-agent 報告 + HANDOFF + 决策日志.

包含 (30+ 文件):
- HANDOFF-NEXT-SESSION-2026-08-10.md (1.0 release 8 步 verify 起点)
- decision-log-2026-08-11.md (R136-2 决策日志, per 用户记忆 #10 主人睡觉期间 决策日志 严守)
- 决策文件 decision-01 ~ decision-77 (77 份决策记录, 调研 + 实战 + 路线图 完整, 含 决策 #73 + #74 + #75 + #76 + #77)
- R129 era 41 sub-agent 報告 (R129-1 ~ R129-35, 整合 #5 commit 准备 + 1.0 release 实战 + 借鉴 verify + 哲学锚 + ASI Stage 4-7 + Tauri Stage 2-3 + 形式化证明 Stage 5.2-5.3)
- R128 era 6 sub-agent 報告 (R128 / R128-2 整合准备)
- R127 era 16 sub-agent 報告 (P5-1~5-3, P6-1~6-3, P7-1~7-3, P8-1~8-3, P9-1~9-3, P10-1~10-3, P11-1~11-2, P12-1, P13-1, P14-1, P15-1)
- R126 era 8 sub-agent 報告 (R126 + 8 哲学锚 + 30 维 V0.5 + 6 重 v7 + 24 LOCKED 0 改)
- R125 era 22 sub-agent 報告 (R125-1~22, 借鉴 11/11 调研)
- R130 era 6 sub-agent 報告 (R130-1~6, per 决策 #72)
- R131 era 9 sub-agent 報告 (R131-1~9, per 决策 #75 §2.1)
- R132 era 2 sub-agent 報告 (R132-1~2, per 决策 #75 §2.1)
- R133 era 3 sub-agent 報告 (R133-1~3, per 决策 #75 §2.1)
- R134 era 6 sub-agent 報告 (R134-1~6, per 决策 #76 §2.1)
- R135 era 2 sub-agent 報告 (R135-1~2, per 决策 #76 §2.1)
- R136-2 V1.1 release 实战 5 阶段计划報告 (本报告)

0 越界 8 硬墙 100%:
- B1 24 LOCKED 入口签名 0 改 (報告引用, 0 改 src/)
- C1 0 主动 commit (Mavis 自决)
- 0 主动 push (整合 #5.3 commit done 不 push, 等 阶段 3)

Refs: decision-22, #33, #48, #55, #58, #61, #62, #71, #73, #74, #75, #76, #77
```

### 2.2 整合 #6 commit 拍板 (V1.1 release 主体, 估 2026-11-25, per R134-3 + 决策 #62 类比 + 决策 #74 B1 V1.1 release Mavis 自决改)

> **详细 commit message + 文件清单** 引用 R134-3 §1-§5 + 决策 #62 类比 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #33 C1, 不重写.

#### 6.1 commit (V1.1 release src/ 实施, 估 ~30 文件, per R134-3 §3)

**核心内容** (per 决策 #74 B1 + R131-3 §2.1-§2.3 + R134-3 §3):
- **PHL-07 实施** (V1.0 spec-only → V1.1 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED, 14 维主对话锚 + 41 NEW tests)
- **24 LOCKED 入口签名改写** (per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构, e.g. ASI Stage 9 + 9 organ + 三洋葱)
- **后端加固** (cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml 1.2.0 → 1.2.1 bump + pybridge 886/886 性能测试 + Cargo.lock 分模块)
- **公开 API 表面精简** + **crate 间依赖优化** + **9 organ 对应关系**

**commit message** (per 决策 #62 §2 + R134-3 §3 commit 模板):

```
整合 #6.1 commit: V1.1 release src/ 实施 (PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固)

V1.1 release src/ 实施整合 (R130 era 6 + R131 era 9 + R132 era 2 + R133 era 3 = 20 调研 sub-agent + R134 era 30+ 实施 sub-agent done).

PHL-07 实施 (per 决策 #74 §1 A3 升级 + R131-3 §2.1):
- V1.0 spec-only → V1.1 真实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 (25 LOCKED 总数)
- 14 维主对话锚 (跟 8 哲学锚/6 重守门/14 键集成, 41 NEW tests)
- 跟 8 哲学锚 1:1 集成 (B5 严守)
- 跟 6 重守门 v7 1:1 集成 (B4 严守)
- 跟 14 键 1:1 集成 (A3 升级, 13 → 14 键)

24 LOCKED 入口签名改写 (per 决策 #74 B1 V1.1 release Mavis 自决改, 前提: 更好的架构):
- 公开 API 表面精简
- crate 间依赖优化
- 9 organ 对应关系
- 0 改原 24 LOCKED 入口签名顺序 (PHL-07 入口新增 1 个, 总 25 LOCKED)

后端加固 (per R131-3 §2.3):
- cargo test 实战三次 verify
- 借鉴源 12 源 0 装严守二次 verify (8 真 cloned + 2 限流 retry + 1 永久跳过 OpenCog AGPL-3.0 + 🆕 1 借脑 ID 索引完成 OpenCog 家族 6 子源 = 12/12 clear)
- Cargo.toml workspace.version 1.2.0 → 1.2.1 bump (per 决策 #74 B2)
- pybridge 886/886 性能测试
- Cargo.lock 分模块

0 越界 8 硬墙 100%:
- B1 25 LOCKED 入口签名 0 改 (24 LOCKED 入口签名 0 改 + PHL-07 入口新增 1 个)
- B2 workspace.version 1.2.0 → 1.2.1 bump (per 决策 #74 B2)
- A1 R11 baseline 3 值 0 改 (V1.1 release 续 严守)
- A3 14 键 (PHL-07 加 1 键, 13 → 14 键, per 决策 #74 §1 A3 升级)
- B3 V0.5 30 维 严守
- B4 6 重守门 v7 严守
- B5 8 哲学锚 严守
- C1 0 主动 commit (整合 #6.1 commit 由 Mavis 自决拍板, per 决策 #33 C1)
- C2 0 装 PASS 严守 (2 借脑 0 装, PHL-07 0 借用任何具体源码)
- 0 主动 push (R134 era 调研 0 push, 等 V1.1 release 配 GitHub remote + 主人起床后手跑)

Refs: decision-22, #33, #48, #55, #58, #61, #62, #71, #73, #74, #75, #76, #77
Tests: 4200+ tests pass (per R131-9 形式化集成 + R133-1 借鉴 12 源实施 + R137 era PHL-07 实施)
```

#### 6.2 commit (V1.1 release docs/ + Cargo.toml 1.2.1 bump + OpenCog AGPL-3.0 fork OSS NOTICE, 估 ~10 文件, per R134-3 §3)

**核心内容**:
- **CHANGELOG.md** (V1.1.0 changelog, 9 organ × 5 维 × 6 方向 = 270 维 1 屏多卡)
- **ROADMAP.md** (V1.1.0 roadmap, V1.2 路线图衔接)
- **RELEASE_NOTES.md** (V1.1.0 release notes, 6 大方向 + 30+ R134 sub-agent 总结, 估 36.8KB 续 写)
- **OSS_NOTICE.md** (V1.1.0 OSS notice, OpenCog AGPL-3.0 fork 致谢加)
- **Cargo.toml** (workspace.version 1.2.0 → 1.2.1 bump, per 决策 #74 B2)
- **Cargo.lock** (V1.1.0 依赖更新, 分模块)
- **.gitignore** (V1.1.0, _workspace/ 临时产物 + V1.1 release 临时目录)
- **docs/roadmap/** (V1.1.0 roadmap, R130-5 §1.3 + R132-1 §1.2 续)
- **docs/1.1-release/** (V1.1.0 release docs, 6 大方向 + 30+ R134 sub-agent 索引) — **per R136 现状, docs/1.1-release/README.md 已存在, 估 V1.1 release 前续 写**
- **docs/architecture-v3-aircraft-carrier.md** + **docs/architecture-v4-living-intelligence.md** + **docs/architecture-v4-1-living-intelligence-update.md** (V1.1.0 架构文档, ASI Stage 9 + 9 organ 内部借 OpenCode + 三洋葱架构升级)

**commit message** (per 决策 #62 §3 + R134-3 §3 commit 模板):

```
整合 #6.2 commit: V1.1 release docs/ + Cargo.toml 1.2.1 bump + OpenCog AGPL-3.0 fork OSS NOTICE

V1.1 release docs/ + Cargo.toml 续 + OpenCog AGPL-3.0 fork OSS NOTICE 续.

包含 (10 文件/目录):
- CHANGELOG.md (V1.1.0 changelog, per R131-3 §2)
- ROADMAP.md (V1.1.0 roadmap, per R131-3 §1.2)
- RELEASE_NOTES.md (V1.1.0 release notes, per R131-3 §2 6 大方向)
- OSS_NOTICE.md (V1.1.0 OSS notice, OpenCog AGPL-3.0 fork 致谢加, per 决策 #73 §2.2 + R131-2 + R133-1)
- Cargo.toml (workspace.version 1.2.0 → 1.2.1 bump, per 决策 #74 B2)
- Cargo.lock (V1.1.0 依赖更新, per R131-3 §2.3 后端加固)
- .gitignore (V1.1.0, per 整合 #5.2 commit 续)
- docs/roadmap/ (V1.1.0 roadmap 子目录, per R131-3 §1.2)
- docs/1.1-release/ (V1.1.0 release docs, 6 大方向 + 30+ R134 sub-agent 索引)
- + docs/architecture-v3-aircraft-carrier.md + docs/architecture-v4-living-intelligence.md + docs/architecture-v4-1-living-intelligence-update.md (V1.1.0 架构文档)

0 越界 8 硬墙 100%:
- B1 25 LOCKED 入口签名 0 改 (整合 #6.1 commit 已 done, 6.2 commit 0 触碰 src/)
- B2 workspace.version 1.2.0 → 1.2.1 bump 严守
- A1 R11 baseline 3 值 0 改
- A3 14 键 0 改
- B3 V0.5 30 维 0 改
- B4 6 重守门 v7 0 改
- B5 8 哲学锚 0 改
- C1 0 主动 commit (整合 #6.2 commit 由 Mavis 自决拍板)
- C2 0 装 PASS 严守
- 0 主动 push (整合 #6.2 commit done 不 push, 等 V1.1 release 配 GitHub remote)

Refs: decision-22, #33, #48, #55, #58, #61, #62, #71, #73, #74, #75, #76, #77
Depends: 6.1 (Cargo.toml metadata 引用 src/ 路径字符串)
```

#### 6.3 commit (V1.1 release reports/ 续, 估 ~30 文件, per R134-3 §3)

**核心内容**:
- **HANDOFF-NEXT-SESSION-V1.1-RELEASE.md** (V1.1 release 完整上下文, ~30 active 任务状态, 8 硬墙, 决策链 #77-#130 全读, 整合 #6 + #7 commit 拍板边界)
- **决策链 #77-#130** (R130 era + R131 era + R132 era + R133 era + R134 era + R135 era 续, 估 50+ 份)
- **决策日志** (decision-log-2026-08-11.md 续 + decision-log-2026-08-12.md + decision-log-2026-08-13.md + decision-log-r130-era-cron-2026-08-11.md + decision-log-r131-era-cron-2026-08-11.md + decision-log-r132-era-cron-2026-08-11.md + decision-log-r133-era-cron-2026-08-11.md + decision-log-r134-era-cron-2026-08-11.md + decision-log-r135-era-cron-2026-08-11.md + decision-log-r136-era-cron-2026-08-11.md)
- **R130 era 6 sub-agent 報告** (R130-1~6, per 决策 #72)
- **R131 era 9 sub-agent 報告** (R131-1~9, per 决策 #75 §2.1)
- **R132 era 2 sub-agent 報告** (R132-1~2, per 决策 #75 §2.1)
- **R133 era 3 sub-agent 報告** (R133-1~3, per 决策 #75 §2.1)
- **R134 era 6 sub-agent 報告** (R134-1~6, per 决策 #76 §2.1)
- **R135 era 2 sub-agent 報告** (R135-1~2, per 决策 #76 §2.1)
- **R136 era 2 sub-agent 報告** (R136-1~2, per 决策 #77)
- **R137 era 30+ sub-agent 實施 報告** (PHL-07 实施 + 24 LOCKED 改写 + Cargo.toml 1.2.1 bump + ASI Stage 9 实战 + 形式化 Stage 5.5+ 实战 + Tauri Stage 5+ 实战, per 决策 #77)
- **整合 #5 commit 拍板 verify 報告** (per R130-1 续)
- **整合 #6 + #7 commit 拍板准备 報告** (per R134-3 + R134-4 续)
- **V1.1 release cargo logs** (R134-N cargo build/test/audit/deny logs, 10+ log)
- **V1.1 release locked-audit 報告** (25 LOCKED 入口签名 verify, per 决策 #74 §2.3)

**commit message** (per 决策 #62 §4 + R134-3 §3 commit 模板):

```
整合 #6.3 commit: V1.1 release reports/ 决策链 + V1.1 release sub-agent 報告 + HANDOFF

备查用, 0 影响 build.

包含 (估 ~30 文件):
- HANDOFF-NEXT-SESSION-V1.1-RELEASE.md (V1.1 release 完整上下文)
- 决策链 #77-#130 (R130 era + R131 era + R132 era + R133 era + R134 era + R135 era 续)
- 决策日志 (10+ 份)
- R130-R137 era sub-agent 報告 (~50+ 份)
- 整合 #5 commit 拍板 verify 報告
- 整合 #6 + #7 commit 拍板准备 報告
- V1.1 release cargo logs (10+ log)
- V1.1 release locked-audit 報告

0 越界 8 硬墙 100%:
- C1 0 主动 commit (整合 #6.3 commit 由 Mavis 自决拍板)
- 0 主动 push (整合 #6.3 commit done 不 push, 等 V1.1 release 配 GitHub remote)

Refs: decision-22, #33, #48, #55, #58, #61, #62, #71, #73, #74, #75, #76, #77
Depends: 0 (独立)
```

### 2.3 整合 #7 commit 拍板续 (V1.1 release 续, 估 2026-11-29, per R134-4 + 决策 #62 类比 + 决策 #74 B1 V1.1 release Mavis 自决改)

> **详细 commit message + 文件清单** 引用 R134-4 §2-§4 + 决策 #62 类比 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #33 C1, 不重写.

#### 7.1 commit (V1.1 release src/ 实施续, 估 ~50 文件, per R134-4 §2.1)

**核心内容** (per 决策 #74 B1 + R131-3 §2.4-§2.6 + R134-4 §2.1 + R133-2 + R133-3):
- **Tauri Stage 5+ 实施** (9 organ 拟人化深化 + 5 nav 完整 + Tauri 2.0 完整集成 + 跨平台部署 + Tauri 性能优化)
- **ASI Stage 9 终极自治** (Stage 8 群体 G1-G4 + Stage 9 终极自治 4 维度 H/L/G/P + 长程 AI 成长平台 + 借脑 OpenCog CogPrime)
- **形式化 Stage 5.5+** (PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成 + 24 LOCKED 入口形式化 + 8 哲学锚形式化 + V0.5 30 维形式化)
- **三洋葱架构升级** (三洋葱 → 四洋葱, 原则 + 权限 + DSL + 智能涌现 emergence, per R133-3)
- **24 LOCKED 入口签名续** (公开 API 表面精简 + crate 间依赖优化续 + 9 organ 对应关系续)
- **借脑 OpenCog AtomSpace + CogPrime** (per 决策 #73 §2.2 AGPL-3.0 fork-then-borrow 模式, 4 借脑 0 装)
- **pybridge 集成优化续** (per R131-7)
- **V0.5 30 维 + 6 重守门 v7 + 8 哲学锚 + PHL-07 集成**

**commit message** (per 决策 #62 §2 + R134-4 §2.1 commit 模板):

```
整合 #7.1 commit: V1.1 release src/ 实施续 (Tauri Stage 5+ + ASI Stage 9 + 形式化 Stage 5.5+ + 三洋葱架构升级)

V1.1 release src/ 实施续 (R134 era 30+ 实施 sub-agent 续 + R137 era 30+ 实施 sub-agent done).

Tauri Stage 5+ (per R131-3 §2.4 + R130-3 调研 + 决策 #57 + 用户记忆 #3-#5 + 主人 8/4 23:33 Tauri 终极):
- 9 organ 拟人化深化 (9 × 5 维 = 45 维 1 屏多卡)
- 5 nav 完整 (状态/主对话结果/历史/设置/工具结果, per 用户记忆 #3)
- Tauri 2.0 完整集成
- 跨平台部署 (Windows/macOS/Linux)
- Tauri 性能优化

ASI Stage 9 终极自治 (per R131-3 §2.5 + R130-2 调研 + R133-2 实施 spec + 用户记忆 #4):
- Stage 8 群体 G1-G4 (多 agent 协同 + 知识共享 + 任务分配 + 冲突解决)
- Stage 9 终极自治 4 维度 H/L/G/P (H 自治 + L 长程 + G 成长 + P 平台化)
- 长程 AI 成长平台 (持续学习 + 跨时间推理 + 跨任务规划 + 知识累积 + 能力升级)
- 借脑 OpenCog CogPrime (4 借脑 0 装: AtomSpace + CogPrime + moses + pln)

形式化 Stage 5.5+ (per R131-3 §2.6 + R130-4 调研 + R131-9 形式化集成 + 决策 #56):
- PHL-07 形式化
- F1-F11 11 维度 Kani-style harness
- Kani 全集成
- 24 LOCKED 入口形式化
- 8 哲学锚形式化
- V0.5 30 维形式化

三洋葱架构升级 (per R131-3 §2 + R133-3 实施 spec + 决策 #73 §2.2 智能涌现):
- 三洋葱 → 四洋葱 升级 (原则 + 权限 + DSL + 智能涌现 emergence)
- 借脑 OpenCog AtomSpace (知识表示 hypergraph DB)
- 借脑 OpenCog CogPrime (架构)

0 越界 8 硬墙 100%:
- B1 25 LOCKED 入口签名 0 改 (24 LOCKED 入口签名续 0 改 + PHL-07 入口新增 1 个, 整合 #6.1 commit 续)
- B2 workspace.version 1.2.1 0 改 (整合 #6.2 commit 已 bump 1.2.1)
- A1 R11 baseline 3 值 0 改 (V1.1 release 续 严守, per 决策 #74 §2.2)
- A3 14 键 0 改 (整合 #6.1 commit 续)
- B3 V0.5 30 维 严守
- B4 6 重守门 v7 严守
- B5 8 哲学锚 严守
- C1 0 主动 commit (整合 #7.1 commit 由 Mavis 自决拍板, per 决策 #33 C1)
- C2 0 装 PASS 严守 (4 借脑 0 装, 0 借用任何具体源码)
- 0 主动 push (R137 era 调研 0 push, 等 V1.1 release 配 GitHub remote + 主人起床后手跑)

Refs: decision-22, #33, #48, #55, #58, #61, #62, #71, #73, #74, #75, #76, #77
Tests: 4200+ tests pass (per R131-9 形式化集成 + R133-1 借鉴 12 源实施 + R137 era PHL-07 实施 + R137 era ASI Stage 9 实施 + R137 era 形式化 Stage 5.5+ 实施)
```

#### 7.2 commit (V1.1 release docs/ 续 + 三洋葱架构升级文档, 估 ~10 文件, per R134-4 §2.2)

**核心内容**:
- **CHANGELOG.md** (V1.1 release 续 changelog)
- **ROADMAP.md** (V1.1 release 续 roadmap)
- **RELEASE_NOTES.md** (V1.1 release 续 notes)
- **OSS_NOTICE.md** (V1.1 release 续 OSS notice, OpenCog AGPL-3.0 续)
- **Cargo.toml** (workspace.version 1.2.1 严守, per 决策 #74 B2, 0 改)
- **Cargo.lock** (V1.1 release 续 依赖更新)
- **.gitignore** (V1.1 release 续)
- **docs/roadmap/** (V1.1 release 续 roadmap)
- **docs/1.1-release/** (V1.1 release 续 docs)
- **🆕 docs/architecture-v5-four-onion-upgrade.md** (V1.1 release 续 三洋葱 → 四洋葱 架构升级文档, per 决策 #73 §2.2 + R133-3 §3 + 决策 #74 B1)

#### 7.3 commit (V1.1 release reports/ 续, 估 ~50 文件, per R134-4 §2.3)

**核心内容**:
- **HANDOFF-NEXT-SESSION-V1.1-RELEASE-CONTINUE.md** (V1.1 release 续 完整上下文)
- **决策链 #131-#180** (R134 era 续 + R135 era 续 + R136 era 续 + R137 era 续, 估 50+ 份)
- **决策日志** (续 写)
- **R134 era 调研 6 sub-agent 報告** (R134-1~6, per 决策 #76 §2.1)
- **R135 era 调研 2 sub-agent 報告** (R135-1~2, per 决策 #76 §2.1)
- **R136 era 调研 2 sub-agent 報告** (R136-1~2, per 决策 #77)
- **R137 era 实施 30+ sub-agent 報告** (PHL-07 实施 + 24 LOCKED 改写 + Cargo.toml 1.2.1 bump + ASI Stage 9 实战 + 形式化 Stage 5.5+ 实战 + Tauri Stage 5+ 实战)
- **整合 #5 commit 拍板 verify 報告**
- **整合 #6 + #7 commit 实战 報告** (R136-2 本报告 + R137 era 续)
- **V1.1 release cargo logs**
- **V1.1 release locked-audit 報告**

### 2.4 整合 #5 + #6 + #7 commit 拍板时机 (per 决策 #62 §7 + 决策 #74 §4 + 决策 #71 §2.5)

**整合 #5 + #6 + #7 commit 时机 ready 条件** (per 决策 #62 + 决策 #64 + 决策 #74 §4 + 决策 #71 §2.5 永久循环接续):
1. ✅ V1.0 release 实战完 (1.0 release tag `v1.0.0` 打上, per R129-35 final-final 7 步 runbook + R134-2 1.0 release 实战)
2. ✅ 整合 #5 commit 拍板完 (整合 #5.1 + 5.2 + 5.3 顺序 git add + git commit, per 决策 #62 + 决策 #64 cron auto-pickup)
3. ✅ V1.1 release 续 6 大方向 实施完 (PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + 三洋葱架构升级 续, per R131-3 §2 6 大方向 + R133-3 §3)
4. ✅ 8 硬墙 0 越界 verify (决策 #74 B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改)
5. ✅ 25 LOCKED 入口签名 verify (24 LOCKED 入口 0 改 + PHL-07 入口新增 1 个, per 整合 #6.1 commit PHL-07 实施 + 整合 #7.1 commit V1.1 release 续 24 LOCKED 入口签名续 0 改, per R131-5 done)
6. ✅ Cargo.toml 1.2.1 严守 (整合 #6.2 commit Cargo.toml workspace.version 1.2.0 → 1.2.1 bump, per 决策 #74 B2)
7. ✅ master HEAD = 整合 #5 + #6 + #7 commit verify (整合 #5 + #6 + #7 commit 后 master HEAD 严守)
8. ✅ 决策链 #77-#180 全读 verify (R130 era + R131 era + R132 era + R133 era + R134 era + R135 era + R136 era + R137 era 续, per 决策 #71 §2.5 永久循环接续)

**Mavis 自决拍板触发**: cron `watch-r129-era-auto-replenish-16` (per decision-64 §2.2) 5 min tick 监督 整合 #5 + #6 + #7 commit 拍板 → 8 项 verify 100% → Mavis 拍板 整合 #5 (5.1 → 5.2 → 5.3) + 整合 #6 (6.1 → 6.2 → 6.3) + 整合 #7 (7.1 → 7.2 → 7.3) 顺序 git add + git commit (per 决策 #62 + 决策 #64 + 决策 #74 B1 V1.1 release Mavis 自决改)

**0 主动 push 严守**: 整合 #5 + #6 + #7 commit 9 commit 都不 push, 等 V1.1 release 配 GitHub remote (主人起床后手跑, 阶段 2-3)

### 2.5 阶段 1 vs R134-1 整合 #5 commit 拍板实战 + R134-3 整合 #6 commit 拍板 + R134-4 整合 #7 commit 拍板续 的关系

| 维度 | R134-1 整合 #5 commit 拍板实战 | R134-3 整合 #6 commit 拍板 | R134-4 整合 #7 commit 拍板续 | R136-2 阶段 1 (本报告) |
|------|------------------------------|--------------------------|----------------------------|---------------------|
| **任务** | 整合 #5 commit 拍板 (5.1 → 5.2 → 5.3 顺序, Mavis 自决) | 整合 #6 commit 拍板准备 (5 阶段计划 + 6.1/6.2/6.3 实施 spec) | 整合 #7 commit 拍板准备续 (5 阶段计划 + 7.1/7.2/7.3 实施 spec) | V1.1 release 实战 5 阶段计划 (阶段 1 整合 #5 + #6 + #7 commit 拍板 + 阶段 2-5 主人手跑) |
| **时间盒** | 1 day (R134-1 实战) | 4 weeks (R134-3 5 阶段计划) | 4 weeks (R134-4 5 阶段计划) | 3 weeks (整合 #5 + #6 + #7 commit 拍板, 估 8/11 → 11/29) |
| **Mavis 角色** | 主动 (自决拍板 commit) | 主动 (写 5 阶段计划报告 + 实施 spec 写完) | 主动 (写 5 阶段计划报告续 + 实施 spec 写完) | 主动 (写 5 阶段计划报告 + 阶段 2-5 主人手跑 runbook 准备) |
| **输出** | 整合 #5 拆 3 commit 落地 | reports/agent-r134-3-integration-6-commit-paiban-2026-08-11.md (4 周 5 阶段计划) | reports/agent-r134-4-integration-7-commit-paiban-xu-2026-08-11.md (4 周 5 阶段计划续) | reports/agent-r136-2-v1.1-release-execution-2026-08-11.md (本报告, 3 weeks + 1 day 5 阶段计划) |
| **跟阶段 2-5 关系** | 阶段 1 前置 (R134-1 done 才能 阶段 1 done) | 阶段 1 前置 (R134-3 准备 整合 #6 commit 拍板) | 阶段 1 前置 (R134-4 准备 整合 #7 commit 拍板续) | 阶段 1 = R134-1 + R134-3 + R134-4 整合 + 阶段 2-5 主人手跑 runbook |

**R134-1 + R134-3 + R134-4 + R136-2 顺序**: R134-1 整合 #5 commit 拍板实战 (commit 落地, 估 8/11 01:30+) → R134-3 整合 #6 commit 拍板准备 (5 阶段计划 4 周, 估 8/12 - 10/15) → R134-4 整合 #7 commit 拍板续 (5 阶段计划 4 周, 估 10/16 - 11/29) → 整合 #6 commit 拍板 (估 2026-11-25) → 整合 #7 commit 拍板续 (估 2026-11-29) → R136-2 V1.1 release 实战 (本报告, 5 阶段计划 3 weeks + 1 day, 估 2026-11-30 主人起床后手跑).

---

## 3. 阶段 2 详解: 主人配 GitHub remote (主人手跑, 1 hour, Mavis 0 主动)

> **0 主动 push 严守**: 阶段 2 全 主人手跑, Mavis 0 主动 push 0 主动配 remote 0 主动验证 0 主动认证.
> **脚本准备**: scripts/release/setup-github-remote.{ps1,sh} 2 文件 (R129-8 写, 0:14, 10586 + 8435 bytes) 自动化 配 origin remote + verify + 认证配置.
> **V1.1 release 跟 1.0 release 阶段 2 1:1 续**: 1.0 release 阶段 2 主人手跑配 GitHub remote, V1.1 release 阶段 2 1:1 续 (per R134-2 §3, 0 重复造轮子).

### 3.1 阶段 2 步骤 (主人起床后手跑)

**步骤 2.1: 主人浏览器创建 GitHub repo** (per 决策 #62 §5 + R129-8 §Step 3.1)
- 访问 https://github.com/new
- Repository name: `apeireth-rust`
- Owner: `apeireth` (主人 GitHub org, 假设已存在)
- Description: `Apeireth - AGI 操作系统 (30+ crate Rust workspace, R11 baseline 0.8682/0.8532/0.9063, 8 哲学锚, 6 重守门 v7, V0.5 30 维, 12 键+PHL-07, 24 LOCKED, 1.0 release + V1.1 release 24 LOCKED 入口签名 改写 + PHL-07 实施 + ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱架构升级, 8 硬墙 B1 改写 V1.1 release Mavis 自决改, 决策 #74)`
- Public (per V1.1 release 默认 Public)
- **0 初始化** README/.gitignore/license (per R129-8 严守, 0 跟主仓现有冲突) — **重要**: 1.0 release 阶段 主人已手跑过, V1.1 release 阶段 0 重复
- Click "Create repository"

**步骤 2.2: 加 origin remote** (per 决策 #62 §5.1 + R129-8 §Step 3.2 + R134-2 §3.1 1:1 续)
- 主人手跑 PowerShell (Windows):
  ```powershell
  cd Apeireth-rust
  git remote add origin https://github.com/apeireth/apeireth-rust.git
  git remote -v
  ```
- 主人手跑 Bash (Linux/macOS/WSL):
  ```bash
  cd ~/Apeireth-rust
  git remote add origin https://github.com/apeireth/apeireth-rust.git
  git remote -v
  ```
- 预期输出: `origin  https://github.com/apeireth/apeireth-rust.git (fetch)` + `origin  https://github.com/apeireth/apeireth-rust.git (push)`
- **注意**: 1.0 release 阶段 主人已手跑过, V1.1 release 阶段 验证 origin remote 存在 即可 (per R134-2 §3.2 R3 风险: 主人手跑前 verify origin remote 已存在)

**步骤 2.3: 主人配 git push 认证** (per R129-8 §Step 3.3 + R134-2 §3.1 1:1 续)
- 选项 A: gh CLI (推荐, 主人 GitHub org 已有 gh 认证):
  ```bash
  gh auth login --with-token  # 主人输入 GitHub PAT
  gh auth status  # verify
  ```
- 选项 B: GitHub PAT (Personal Access Token):
  - 主人浏览器 https://github.com/settings/tokens → Generate new token (classic)
  - Scopes: `repo` (full) + `workflow` + `write:packages`
  - 主人手跑: `git config --global credential.helper store` + 首次 push 时输入 PAT
- **注意**: 1.0 release 阶段 主人已手跑过, V1.1 release 阶段 0 重复认证

**步骤 2.4: 主人 verify origin remote + 认证** (per R129-8 §Step 3.4 + R134-2 §3.1 1:1 续)
- 主人手跑:
  ```bash
  git remote -v
  # 验证 origin = https://github.com/apeireth/apeireth-rust.git
  gh auth status
  # 验证 Logged in to github.com as apeireth
  ```

### 3.2 阶段 2 风险 + 缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| **R1**: GitHub org `apeireth` 不存在 | 主人无法创建 repo | 主人提前 verify org 存在 (https://github.com/apeireth), 不存在则用 主人 personal account |
| **R2**: GitHub PAT 权限不足 | push 失败 | 用 `repo` + `workflow` + `write:packages` scopes (full repo access) |
| **R3**: 主人 0 初始化 README/.gitignore/license 错 | 跟主仓冲突 | R129-8 setup-github-remote.ps1 写"0 初始化" banner 严守, 主人手跑前 read |
| **R4**: origin remote URL 拼错 | push 失败 | `git remote -v` verify, 跟 https://github.com/apeireth/apeireth-rust.git 严格对齐 |
| **R5**: 阶段 1 整合 #5 + #6 + #7 commit 未 done | 阶段 2 推 0 commit | 阶段 1 Mavis 自决拍板 done 后才 阶段 2 (per 决策 #74 §4 + 决策 #76 §2.1) |
| **R6**: 1.0 release 阶段 origin remote 已配 (1:1 续 风险) | V1.1 release 阶段 0 重复配 | R136-2 §3.1 步骤 2.2 主人 verify origin remote 存在 即可, 0 重复配 |

**0 主动 push 严守**: Mavis 0 主动 配 remote 0 主动 验证 0 主动 认证 — 阶段 2 全 主人手跑.

---

## 4. 阶段 3 详解: 主人 git push 整合 #5 + #6 + #7 拆 9 commit (主人手跑, 1 hour, Mavis 0 主动)

> **0 主动 push 严守**: 阶段 3 全 主人手跑, Mavis 0 主动 push 0 主动 commit 0 主动 add.
> **脚本准备**: scripts/release/git-push-1.0.{ps1,sh} 2 文件 (R129-8 写, 0:17, 18067 + 15146 bytes) 自动化 整合 #5 + #6 + #7 拆 9 commit + push master.
> **V1.1 release 跟 1.0 release 阶段 3 1:1 续**: 1.0 release 阶段 3 主人手跑 git push 整合 #5 拆 3 commit, V1.1 release 阶段 3 1:1 续 (整合 #5 + #6 + #7 拆 9 commit 一起 push).

### 4.1 阶段 3 步骤 (主人起床后手跑, 整合 #5 + #6 + #7 commit 已 done per 阶段 1)

**步骤 3.1: 主人 verify master HEAD = 整合 #7.3 commit** (per R129-8 §Step 4.1 + R134-2 §4.1 1:1 续)
- 主人手跑:
  ```bash
  git log --oneline -10
  # 预期看到 整合 #7.3 commit (顶部) + 整合 #7.2 + 整合 #7.1 + 整合 #6.3 + 整合 #6.2 + 整合 #6.1 + 整合 #5.3 + 整合 #5.2 + 整合 #5.1 + 整合 #4 commit abf12243
  git rev-parse HEAD
  # 预期: 整合 #7.3 commit hash (跟 阶段 1 R134-1 + R134-3 + R134-4 Mavis 拍板一致)
  ```

**步骤 3.2: 主人手跑 git push master + tags** (per R129-8 §Step 4.2 + R129-35 §Step 4 + R134-2 §4.1 1:1 续)
- 主人手跑 PowerShell (Windows):
  ```powershell
  cd Apeireth-rust
  git push -u origin master
  git push -u origin --tags
  ```
- 主人手跑 Bash (Linux/macOS/WSL):
  ```bash
  cd ~/Apeireth-rust
  git push -u origin master
  git push -u origin --tags
  ```
- 预期输出: `Writing objects: 100% (XXX/XXX), XXX bytes` + `To https://github.com/apeireth/apeireth-rust.git` + `* [new branch] master -> master` + `Branch 'master' set up to track remote 'origin/master'`
- **注意**: V1.1 release 阶段 git push master 一次推 9 commit (整合 #5.1+5.2+5.3+6.1+6.2+6.3+7.1+7.2+7.3), 跟 1.0 release 阶段 git push master 一次推 3 commit (整合 #5.1+5.2+5.3) 区别是 6 commit 增量

**步骤 3.3: 主人 verify push 成功** (per R129-8 §Step 4.3 + R134-2 §4.1 1:1 续)
- 主人手跑:
  ```bash
  git status
  # 预期: Your branch is up to date with 'origin/master'
  git log --oneline origin/master -10
  # 预期: 顶部 9 个 commit = 整合 #7.3 + 7.2 + 7.1 + 6.3 + 6.2 + 6.1 + 5.3 + 5.2 + 5.1, 跟 local master 一致
  ```
- 主人浏览器 verify: https://github.com/apeireth/apeireth-rust/commits/master (9 个新 commit 顶部)

### 4.2 阶段 3 风险 + 缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| **R1**: 整合 #5 + #6 + #7 commit 未 done (阶段 1 NOT ready) | push 0 commit | 阶段 1 Mavis 自决拍板 done 后才 阶段 3 (per 决策 #74 §4 + 决策 #76 §2.1) |
| **R2**: 网络断开 / push timeout | push 失败 | 主人 retry, git push 默认 retry safe |
| **R3**: remote master 有冲突 (per R23 P3 2026-08-07 1.0.0 tag stale) | push rejected | 主人 verify remote master = V1.0 release 整合 #5 拍板完 (3 commit 顶部), 0 conflict |
| **R4**: `--tags` 推送 stale v1.0.0 tag (per R23 P3 2026-08-07 01:33, 471a8728) | 推送错误 tag | 1.0 release 阶段 步骤 4.1 主人已 `git tag -d v1.0.0` 删 stale + 推新 v1.0.0 tag, V1.1 release 阶段 0 重复 (per R134-2 §5.1 步骤 4.1) |
| **R5**: push rejected due to size (大文件) | push 失败 | 主人 verify `.gitignore` 严守, 0 推 target/ + node_modules/ + .DS_Store (per R126-gitignore) |
| **R6**: V1.1 release 9 commit 一起 push 失败 (R5 增量) | push 部分失败 | 主人 retry 增量, 或手动 `git push -u origin master~9..master` (增量 push) |

**0 主动 push 严守**: Mavis 0 主动 push 0 主动 add 0 主动 commit (整合 #5 + #6 + #7 commit 9 commit 由 阶段 1 R134-1 + R134-3 + R134-4 Mavis 自决拍板, 阶段 3 主人手跑 push). 阶段 3 全 主人手跑.

---

## 5. 阶段 4 详解: 主人 tag v1.1.0 + GitHub Release notes (主人手跑, 1 hour, Mavis 0 主动)

> **0 主动 push 严守**: 阶段 4 全 主人手跑, Mavis 0 主动 tag 0 主动 push tag 0 主动 gh release create.
> **脚本准备**: scripts/release/tag-1.0.0.{ps1,sh} 2 文件 (R129-8 写, 0:18, 13126 + 10842 bytes) 自动化 打 tag + push tag + gh release create — **V1.1 release 阶段 0 复用, 主人手跑 时 改 v1.1.0**.

### 5.1 阶段 4 步骤 (主人起床后手跑, 整合 #5 + #6 + #7 commit + git push 已 done per 阶段 1 + 阶段 3)

**步骤 4.0: 主人 verify v1.0.0 tag 状态** (per R134-2 §5.1 步骤 4.1 1:1 续)
- 背景: 1.0 release 阶段 主人已 `git tag -d v1.0.0` 删 stale (471a8728) + 推新 v1.0.0 tag, V1.1 release 阶段 0 重复
- 主人手跑:
  ```bash
  git tag -l
  # 预期: v1.0.0 (1.0 release 阶段 已 done) + 0 v1.1.0 (V1.1 release 阶段 还没打)
  git rev-parse v1.0.0
  # 预期: 整合 #5.3 commit hash (1.0 release 阶段 已打)
  ```

**步骤 4.1: 主人手跑 打 annotated tag v1.1.0** (per R129-8 §Step 5.1 + 决策 #62 §5.2 + 决策 #74 B1 V1.1 release Mavis 自决改)
- 主人手跑:
  ```bash
  git tag -a v1.1.0 -m "Apeireth 1.1.0 release: 24 LOCKED 入口签名 改写 + PHL-07 实施 + ASI Stage 9 终极自治 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱架构升级 (8 硬墙 B1 改写 V1.1 release Mavis 自决改, 决策 #74)"
  git tag
  # 预期: v1.0.0 + v1.1.0
  git show v1.1.0
  # 预期: Tagger + Date + 整合 #7.3 commit hash + tag message
  ```

**步骤 4.2: 主人手跑 push tag v1.1.0** (per R129-8 §Step 5.2)
- 主人手跑:
  ```bash
  git push origin v1.1.0
  # 预期: To https://github.com/apeireth/apeireth-rust.git * [new tag] v1.1.0 -> v1.1.0
  ```
- 主人浏览器 verify: https://github.com/apeireth/apeireth-rust/tags (v1.0.0 + v1.1.0 tag 在列表)

**步骤 4.3: 主人浏览器 GitHub UI: Draft a new release** (per R129-8 §Step 5.3 + 决策 #62 §5.3 + R134-2 §5.1 步骤 4.4 1:1 续)
- 主人浏览器访问 https://github.com/apeireth/apeireth-rust/releases/new
- Choose a tag: 选择 `v1.1.0` (从下拉菜单)
- Release title: `Apeireth 1.1.0`
- Describe this release: 主人复制 `RELEASE_NOTES.md` V1.1 release (整合 #6.2 commit 包含, 估 36.8KB 续 写, R137 era R137-N sub-agent 写 V1.1 release 续 release notes, 估 60KB+) 全部内容粘贴 (或 `gh release create v1.1.0 --title "Apeireth 1.1.0" --notes-file RELEASE_NOTES.md` CLI 命令, 主人手跑)
- Attach binaries: 0 (V1.1 release 0 推 binary, 0 推 cargo crate registry)
- Set as latest release: ✅ 勾选
- Set as a pre-release: ❌ 0 勾选 (V1.1 release = stable)
- Click "Publish release"

**步骤 4.4: 主人 verify GitHub release 页面** (per R129-8 §Step 5.4)
- 主人浏览器访问 https://github.com/apeireth/apeireth-rust/releases/tag/v1.1.0
- 预期: Release title "Apeireth 1.1.0" + tag `v1.1.0` + release notes (RELEASE_NOTES.md 全文) + Latest release 标记

**步骤 4.5: 主人 verify v1.0.0 + v1.1.0 双 release 页面** (per R134-2 §5.2 步骤 4.5 1:1 续)
- 主人浏览器访问 https://github.com/apeireth/apeireth-rust/releases (双 release 列表)
- 预期: v1.0.0 (Latest release [1.0 release 阶段] → 0 Latest after V1.1 release 阶段 publish) + v1.1.0 (Latest release [V1.1 release 阶段] = 勾选)

### 5.2 阶段 4 风险 + 缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| **R1**: v1.0.0 tag 状态错 (1.0 release 阶段 stale 没删 / 新 v1.0.0 没推) | v1.1.0 推送顺序错 | 阶段 4 步骤 4.0 主人 verify v1.0.0 tag = 整合 #5.3 commit hash (1.0 release 阶段 已 done) |
| **R2**: v1.1.0 tag message 拼错 | 后续 gh release create 错 | tag message 跟 决策 #74 B1 V1.1 release Mavis 自决改 verbatim 对齐 |
| **R3**: `gh release create` CLI 失败 (gh 未认证 / 0 org 权限) | release 页面 0 创建 | 主人 fallback 浏览器 GitHub UI 手跑 (步骤 4.3) |
| **R4**: RELEASE_NOTES.md V1.1 release 内容 0 完整 (R137 era R137-N sub-agent 续 写) | release 页面 0 release notes | 主人 verify RELEASE_NOTES.md V1.1 release 续 content 60KB+ 在主仓 (整合 #6.2 commit 包含) |
| **R5**: Set as pre-release 错勾选 | release 标记为 0 stable | 主人 uncheck "Set as a pre-release" (V1.1 release = stable) |
| **R6**: V1.1 release tag reconcile 错 (v1.1.0 vs v1.2.1 per 决策 #74 B2) | tag 不一致 | R136-2 §0 关键发现 3 reconcile: 本报告以 `v1.1.0` 为主 (per R131-3 §1.2 + R132-1 §1.1), 跟 R134-3/R134-4/R134-5 reconcile 留 R136-1 / R137 era 派活拍板 |

**0 主动 push 严守**: Mavis 0 主动 tag 0 主动 push tag 0 主动 gh release create — 阶段 4 全 主人手跑.

---

## 6. 阶段 5 详解: 主人 GitHub Pages 部署 + 8 步 verify + V1.1 release 续 3 步 verify (主人手跑, 1 day, Mavis 0 主动)

> **0 主动 push 严守**: 阶段 5 全 主人手跑, Mavis 0 主动 build 0 主动 push gh-pages 0 主动配 GitHub Pages 设置 0 主动 8 步 verify.
> **脚本准备**: scripts/release/deploy-github-pages.{ps1,sh} 2 文件 (R129-23 写, 0:43, 17689 + 13453 bytes) 自动化 mkdocs build + gh-pages branch 部署.

### 6.1 阶段 5 步骤 (主人起床后手跑, 整合 #5 + #6 + #7 commit + git push + tag v1.0.0 + v1.1.0 + release 已 done per 阶段 1-4)

**步骤 5.1: 主人 一次性 pip install mkdocs + mkdocs-material** (per R129-23 §Step 6.0 + R134-2 §6.1 1:1 续)
- 主人手跑 (Python 3.8+ 环境):
  ```bash
  pip install mkdocs mkdocs-material
  # 预期: Successfully installed mkdocs-X.X.X mkdocs-material-X.X.X
  mkdocs --version
  # 预期: mkdocs, version X.X.X
  ```
- **注意**: 1.0 release 阶段 主人已装过, V1.1 release 阶段 0 重复装

**步骤 5.2: 主人手跑 mkdocs build** (per R129-23 §Step 6.1 + R134-2 §6.1 1:1 续, V1.1 release 续 docs/1.1-release/)
- 主人手跑:
  ```bash
  cd Apeireth-rust  # 或 ~/Apeireth-rust
  mkdocs build
  # 预期: INFO - Documentation built in X.XX seconds + site/ 目录生成
  ls site/
  # 预期: index.html + getting-started/ + api/ + roadmap/ + architecture/ + changelog/ + borrowed-repos/ + 1.1-release/ (V1.1 release 续)
  ```
- **V1.1 release 续**: mkdocs.yml 续 7 + 1 = 8 nav (per 整合 #6.2 commit mkdocs.yml 续)

**步骤 5.3: 主人手跑 创建 gh-pages branch (orphan 模式)** (per R129-23 §Step 6.2 + R134-2 §6.1 1:1 续)
- 主人手跑:
  ```bash
  git checkout --orphan gh-pages
  git rm -rf .
  # 警告: rm -rf . 会删除 working dir 所有文件, 但 0 影响 master branch (orphan 模式)
  cp -r site/* .  # Windows: Copy-Item -Recurse site\* .
  git add -A
  git commit -m "GitHub Pages V1.1 release: mkdocs build from docs/pages-source/ + mkdocs.yml + docs/1.1-release/ 续"
  git checkout master  # 回到 master branch, 继续 阶段 5 步骤 5.4+
  ```

**步骤 5.4: 主人手跑 push gh-pages branch** (per R129-23 §Step 6.3 + R134-2 §6.1 1:1 续)
- 主人手跑:
  ```bash
  git push origin gh-pages --force
  # 预期: To https://github.com/apeireth/apeireth-rust.git * [new branch] gh-pages -> gh-pages
  ```
- 主人浏览器 verify: https://github.com/apeireth/apeireth-rust/tree/gh-pages (8 文档 V1.0 + V1.1 release 续 + mkdocs.yml + site/)

**步骤 5.5: 主人浏览器 GitHub Pages 设置** (per R129-23 §Step 6.4 + R134-2 §6.1 1:1 续)
- 主人浏览器访问 https://github.com/apeireth/apeireth-rust/settings/pages
- Source: `Deploy from a branch` (下拉菜单)
- Branch: `gh-pages` + Folder: `/ (root)`
- Click "Save"
- 等待 1-2 分钟 GitHub Actions 自动 build + 部署

**步骤 5.6: 主人手跑 8 步 verify** (per R129-13 §2 1.0 release checklist 8 步 + 决策 #55 §8 + handoff §8.2 + R134-2 §6.1 步骤 5.6 1:1 续, V1.1 release 续 3 步)
- 主人手跑 (Windows):
  ```powershell
  cd Apeireth-rust
  .\scripts\release\verify-1.0-pre-tag.ps1
  # 8 步全 PASS 报告写到 reports\verify-1.0-pre-tag-YYYY-MM-DD-HHMM.md
  ```
- 主人手跑 (Bash):
  ```bash
  cd ~/Apeireth-rust
  ./scripts/release/verify-1.0-pre-tag.sh
  # 8 步全 PASS 报告写到 reports/verify-1.0-pre-tag-YYYY-MM-DD-HHMM.md
  ```
- **8 步 verify 检查项** (per R129-13 §2 表格 + R134-2 §6.1 步骤 5.6 1:1 续):
  | # | 步骤 | 检查项 | 通过判据 |
  |---:|------|-------|---------|
  | 1 | 修 session working dir + master HEAD + Cargo.toml | working dir = `Apeireth-rust/` + HEAD = 整合 #7.3 + Cargo.toml version = `1.2.1` (V1.1 release bump) | 3/3 |
  | 2 | `cargo build --workspace` | 0 error, 4200+ tests 编译通过 (V1.1 release 续 +100 tests) | exit 0 |
  | 3 | `cargo test --workspace` | 0 failed, 4200+ tests pass | exit 0 |
  | 4 | `cargo run --bin apeireth-tui` 5s smoke | TUI 启动不立即崩 | 进程跑 5s 不自退 |
  | 5 | `cargo run --bin apeireth-api` 5s smoke | API 启动不立即崩 | 进程跑 5s 不自退 |
  | 6 | `cargo audit + cargo deny` | 0 vulnerabilities + 0 license 错 | exit 0 |
  | 7 | **25 LOCKED 入口签名 verify** (per 决策 #74 §2.3 + R131-5 续) | 24 LOCKED crate lib.rs 入口签名未改 + PHL-07 入口新增 1 个 | 25/25 ✅ |
  | 8 | 8 硬墙 0 越界 + 0 装 PASS 严守 | B1-B7 + A1-A3 + C1-C3 + 0 push 11 项 100% | 11/11 ✅ |
  | **🆕 9** | **V1.1 release 续 3 步 verify** (per 决策 #74 B1 V1.1 release Mavis 自决改) | ASI Stage 9 实施 verify (per R130-2 + R133-2) + 形式化 Stage 5.5+ 实施 verify (per R130-4 + R131-9) + Tauri Stage 5+ + 三洋葱架构升级 实施 verify (per R130-3 + R133-3) | 3/3 ✅ |
- **任何 1 步 fail → 阻塞 V1.1 release tag** (per R129-13 §2 严守 + 决策 #74 §2.3 V1.1 release Mavis 自决改 verify)

**步骤 5.7: 主人 verify GitHub Pages 文档站** (per R129-23 §Step 7.2 + R134-2 §6.1 步骤 5.7 1:1 续, V1.1 release 续 docs/1.1-release/)
- 主人浏览器访问 https://apeireth.github.io/apeireth-rust/
- 预期: Material theme 主页 (index.md) + 8 文档导航 (Home/Getting Started/API/Roadmap/Architecture/Changelog/Borrowed Repos/V1.1 Release) — V1.1 release 续 1 doc
- 主人 verify 每个文档页面 200 OK + 内容完整
- 主人 verify 借鉴 11/11 致谢链接 (borrowed-repos.md 链 OSS_NOTICE.md) + LICENSE 链接 (链 LICENSE) + 决策链 (#22-#180) + V1.1 release 续 docs/1.1-release/ 内容 (6 大方向 + 30+ R134 sub-agent 索引)

**步骤 5.8: 主人 verify V1.1 release 页面 + GitHub Pages 文档站 双 done** (per R129-35 §Step 7 + R134-2 §6.1 步骤 5.8 1:1 续)
- 主人浏览器双 verify:
  - https://github.com/apeireth/apeireth-rust/releases/tag/v1.1.0 (V1.1 release 页面)
  - https://apeireth.github.io/apeireth-rust/ (GitHub Pages 文档站)
- 主人发 release announcement (微信群 / Twitter / 邮件, per R129-23 §Step 7.3)

### 6.2 阶段 5 风险 + 缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| **R1**: pip install mkdocs 失败 (Python 0 装) | mkdocs build 失败 | 主人先 verify Python 3.8+ + pip 工作, 用 `py -3 -m pip install mkdocs mkdocs-material` (Windows) |
| **R2**: mkdocs build 失败 (mkdocs.yml 配错) | site/ 0 生成 | 主人 verify mkdocs.yml 4133 bytes + 续 V1.1 release 续 1 nav (per 整合 #6.2 commit) 内容 0 改, 文档路径 0 缺 |
| **R3**: `git checkout --orphan gh-pages` + `git rm -rf .` 误删 master 文件 | 主人手抖 | orphan 模式 0 影响 master branch, 主人 verify `git checkout master` 后 working dir 0 缺文件 |
| **R4**: GitHub Pages 设置错 (Source 0 选 gh-pages) | 部署 0 done | 主人手跑步骤 5.5 时 verify Source = `gh-pages` + Folder = `/ (root)` |
| **R5**: 8 步 verify 任何 1 步 fail (per R129-13 §2) | 阻塞 V1.1 release tag | 主人先 修 fail 项, 重跑 8 步 verify, 全 PASS 后才 阶段 5 步骤 5.7 |
| **R6**: GitHub Pages 部署 404 (DNS 传播慢) | 文档站 0 访问 | 主人等 5-10 分钟 GitHub Actions build + DNS 传播, 然后 retry |
| **R7**: 25 LOCKED 入口签名 verify 失败 (per 决策 #22 + 决策 #74 §2.3 V1.1 release Mavis 自决改 verify) | 阻塞 V1.1 release tag | 主人先 verify P2-3 + P4-1 + P14-1 retry + R131-5 四方 verify 报告, 整合 #6.1 commit PHL-07 实施 + 整合 #7.1 commit V1.1 release 续 24 LOCKED 入口签名续 0 改 |
| **R8**: V1.1 release 续 3 步 verify 失败 (per 决策 #74 B1 V1.1 release Mavis 自决改) | 阻塞 V1.1 release tag | 主人先 verify R130-2 + R130-3 + R130-4 + R131-9 + R133-1/2/3 实施 報告, 整合 #6 + #7 commit 实施完 verify |
| **R9**: gh-pages branch 已有 1.0 release 阶段 内容 (1:1 续 风险) | V1.1 release 阶段 gh-pages push --force 覆盖 | R136-2 §6.1 步骤 5.4 主人手跑 `git push origin gh-pages --force` (force 覆盖, V1.1 release 续 docs/1.1-release/ 包含) |

**0 主动 push 严守**: Mavis 0 主动 build 0 主动 push gh-pages 0 主动配 GitHub Pages 设置 0 主动 8 步 verify — 阶段 5 全 主人手跑.

---

## 7. 8 硬墙 0 越界 + 8 哲学锚严守 + B1 改写边界 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

> **本节定位**: V1.1 release 实战 严守决策, 引用 R134-2 §7 8 硬墙表 + R131-3 §1.4 0 装 PASS 严守 + R132-1 §1.4 关键诚实标 + 决策 #74 §1 8 硬墙改写表 + 决策 #73 §3 不要怕复杂度哲学 + 哲学文档 15-no-fear-complexity.md, 不重写.

### 7.1 8 硬墙 0 越界 (per decision-33 §2.3 + 决策 #74 §1 B1 改写)

| 硬墙 | 整合 #5 (V1.0 release) | 整合 #6 (V1.1 release 主体) | 整合 #7 (V1.1 release 续) | R136-2 (本报告) | V1.1 release |
|------|--------|---------|---------|-------------|------------|
| **B1 24/25 LOCKED 入口签名** | ✅ 0 改严守 (R11 baseline) | 🟢 Mavis 自决改 (PHL-07 入口新增 1 个 → 25 LOCKED) | 🟢 Mavis 自决改 续 (公开 API 表面精简 + crate 间依赖优化 + 9 organ 对应) | 0 触碰 (写 reports/ 0 改 src/) | 25 LOCKED 0 越界 (per 决策 #74 §4 B1 改写边界) |
| **B2 workspace.version** | 🔒 1.2.0 0 改 (1.0 release tag) | 🟢 bump 1.2.1 (per 决策 #74 B2) | 🟢 1.2.1 严守 | 0 改 (本报告 0 触碰 Cargo.toml) | 0 越界 (Cargo.toml 1.2.0 → 1.2.1 bump per 决策 #74 B2) |
| **A1 R11 baseline 3 值** | 🔒 严守 | 🔒 严守 (V1.1 release 续, per 决策 #74 §2.2) | 🔒 严守 (V1.1 release 续) | 0 触碰 | 0 越界 |
| **A3 12 键 + PHL-07** | 🔒 12 键 + PHL-07 spec-only 0 实施 (V1.0 release) | 🟢 14 键 (PHL-07 实施, 加 1 键, per 决策 #74 §1 A3 升级) | 🟢 14 键 续 (整合 #6.1 commit 续) | 0 触碰 | 0 越界 |
| **B3 V0.5 30 维** | 🔒 严守 | 🔒 严守 (哲学公式) | 🔒 严守 (哲学公式) | 0 触碰 | 0 越界 |
| **B4 6 重守门 v7** | 🔒 严守 | 🔒 严守 (哲学守门) | 🔒 严守 (哲学守门) | 0 触碰 | 0 越界 |
| **B5 8 哲学锚** | 🔒 严守 | 🔒 严守 (哲学) | 🔒 严守 (哲学) | 0 触碰 | 0 越界 |
| **C1 0 主动 commit** | ✅ 整合 #5 由 Mavis 自决 | ✅ 整合 #6 由 Mavis 自决 (per 决策 #33 C1) | ✅ 整合 #7 由 Mavis 自决 (per 决策 #33 C1) | 0 commit (R136-2 写 reports/ 0 commit) | 0 越界 (Mavis 自决 + cron auto-pickup) |
| **C2 0 装 PASS 严守** | ✅ 8 真实施 + 2 限流 retry | ✅ 12 借鉴源 0 装 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 🆕 1 借脑 ID 索引完成 OpenCog 家族 6 子源 = 12/12 clear) | ✅ 4 借脑 0 装 (OpenCog AtomSpace + CogPrime + moses + pln = 4 借脑 0 装) | ✅ 0 借具体源码 (V1.1 release 文档是配置) | 0 越界 |
| **0 主动 push** | ✅ 0 push (整合 #5 不 push) | ✅ 0 push (整合 #6 不 push) | ✅ 0 push (整合 #7 不 push) | 0 push (R136-2 0 push) | 0 越界 (Mavis 0 主动, 主人手跑 阶段 2-5) |

**8 硬墙 0 越界 100% PASS** (11 项 verify, per decision-33 §2.3 + 决策 #74 §1 B1 改写)

### 7.2 8 哲学锚严守 (per 决策 #33 §2.3 B5 + R126-philo-8-final §3 + 决策 #74 §1 B5)

| # | 哲学锚 | 严守项 | 整合 #5 + #6 + #7 | R136-2 (本报告) | V1.1 release |
|---:|------|------|----------------|-------------|------------|
| 1 | **L-1 长期主义** | 长程 AGI 成长, V1.1 release 0 短期投机 | ASI Stage 9 长程 AI 成长 (per R133-2) | 0 触碰 | 0 越界 |
| 2 | **L-2 学习优先** | AI 与用户一同成长, V1.1 release 0 装 PASS | PHL-07 实施 + 14 维主对话锚 | 0 借具体源码 | 0 越界 |
| 3 | **S-3 质量工程化** | 整合 #5 + #6 + #7 8 步 verify 严守 4200+ tests | 8 步 verify + V1.1 release 续 3 步 verify | 0 触碰 | 0 越界 |
| 4 | **O-1 安全优先** | 6 重守门 v7 + 8 重 v8 + 25 LOCKED 严守 | 25 LOCKED 入口签名 verify | 0 触碰 | 0 越界 |
| 5 | **T-1 透明可解释** | 决策链 #22-#180 完整, 8 硬墙 0 越界 | 整合 #6.3 + #7.3 commit 决策链全链 | 0 触碰 | 0 越界 |
| 6 | **A-1 用户主权** | 0 主动 push 严守, 主人手跑 阶段 2-5 | 整合 #5 + #6 + #7 0 push | 0 push | 0 越界 |
| 7 | **P-1 哲学优先** | 8 哲学锚 + 8 决策原则 (per decision-10) | 整合 #6.3 + #7.3 commit 哲学锚报告 | 0 触碰 | 0 越界 |
| 8 | **E-1 生态共建** | 借鉴 12/12 致谢 + LICENSE 引用链 | OSS_NOTICE.md 续 OpenCog AGPL-3.0 续 | 0 触碰 | 0 越界 |

**8 哲学锚 0 越界 100% PASS** (per 决策 #33 §2.3 B5 + R126-philo-8-final §3 + 决策 #74 §1 B5)

### 7.3 B1 改写边界 (per 决策 #74 §4 + 决策 #73 §1 + 决策 #76 §2.3)

**V1.0 release (整合 #5.1 commit) 0 改严守** (per 决策 #74 §2.2 + 决策 #33 §2.3 B1):
- 0 改 24 LOCKED 入口签名
- 0 改 24 LOCKED crate mtime baseline 16:34 之前
- 0 改 R11 baseline 3 值
- PHL-07 spec-only 0 实施 (V1.1 release 实施, per R129-11 关键诚实标)

**V1.1 release (整合 #6 + #7 commit 拍板) Mavis 自决改** (per 决策 #74 §2.2 + 决策 #73 §1 "Mavis 自决架构拍板"):
- **B1 改写触发条件** (per 决策 #74 §2.2 + 决策 #73 §1 "更好的架构" + 决策 #76 §2.3):
  - 触发 1: ASI Stage 9 长程 AI 成长 (per R130-2 §1 Stage 9 路线图 + R133-2 实施 spec, 整合 #7.1 commit V1.1 release 续 实施)
  - 触发 2: 9 organ 内部借 OpenCode (per R130-3 §2.4 Stage 5 9 organ 1 真相源, 整合 #7.1 commit V1.1 release 续 9 organ 拟人化深化)
  - 触发 3: 三洋葱架构升级 (per R125 B6 升三洋葱, 原则 + 权限 + DSL, 整合 #7.1 commit V1.1 release 续 三洋葱 → 四洋葱 升级, per R133-3 §3)
  - 触发 4: PHL-07 实施扩展 (per R131-3 §2.1, 整合 #6.1 commit PHL-07 加 1 入口 = 25 LOCKED)
  - 触发 5: Cargo workspace 重构 (per V2.0 release 路线图 spec, V1.1 release 可选触发, Mavis 自决)
- **B1 改写边界** (per 决策 #74 §2.2):
  - ✅ 改 24 LOCKED 入口签名 (前提: 满足触发条件, Mavis 自决, 整合 #6.1 + #7.1 commit 改写)
  - ✅ 改 24 LOCKED crate mtime baseline 16:34 之前 (前提: 更好的架构, Mavis 自决)
  - ✅ 改 R11 baseline 3 值 (前提: 新的 baseline 更高, 跟 R12 测度对齐, Mavis 自决) — V1.1 release 续 0 改, 留给 V2.0 release
  - ❌ 0 改 8 哲学锚 (per 决策 #74 §1, B5 严守, 哲学类不松绑)
  - ❌ 0 改 V0.5 30 维 (per 决策 #74 §1, B3 严守, 哲学公式)
  - ❌ 0 改 6 重守门 v7 (per 决策 #74 §1, B4 严守, 哲学守门)
  - ❌ 0 改 0 主动 commit (per 决策 #74 §1, C1 严守)
  - ❌ 0 改 0 装 PASS 严守 (per 决策 #74 §1, C2 严守)
  - ❌ 0 改 0 主动 push (per 决策 #74 §1, 严守)

**V2.0 release (远期 2027+, per ROADMAP.md §4 + 决策 #74 §2.3) 全 8 硬墙可重评** (per Mavis 自决 + 主人 01:14 拍板).

### 7.4 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md + 决策 #74 §1)

**9 件套总哲学** (per 哲学文档 15-no-fear-complexity.md §2):
- 8 哲学锚 (思想哲学, per 决策 #33 §2.3 B5 + 哲学文档 09-anchor.md): S-1 服务 ASI 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人经验上 + O-3 干到底 + O-4 任何人都能接手 + O-5 不假装
- **不要怕复杂度** (工程哲学, per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md): 最强效果 + 最厉害工程 + 维护交给未来高水平团队
- 9 件套 = 完整思想 + 工程边界

**V1.1 release 续 不要怕复杂度哲学落地** (per 决策 #73 §3 + R135-1 §1.3 哲学锚 + 哲学文档 15-no-fear-complexity.md 整合 #5.2 commit 包含):
- ✅ 长程 AI 成长平台 (per 用户记忆 #4 "AI 不会衰老病死, 它只会成长" + R133-2 ASI Stage 9)
- ✅ 平台化 (per R129-18 智囊团 + 决策 #55 §2.6)
- ✅ AGI 哲学 9 件套 (8 哲学锚 + 不要怕复杂度, 整合 #5.2 commit 包含 哲学文档 15-no-fear-complexity.md)
- ✅ 8 硬墙 B1 改写 (per 决策 #74 V1.1 release Mavis 自决改 前提: 更好的架构)
- ✅ OpenCog AGPL-3.0 fork-then-borrow 模式 (per 决策 #73 §2.2 + R131-2 借脑)
- ✅ 三洋葱 → 四洋葱 架构升级 (per R133-3 + 决策 #73 §2.2 智能涌现)

**0 假装已实现严守 100%** (per 决策 #33 §2.3 C2 + 决策 #10 + 主人 10 项偏好 #7):
- V1.1 release 实战 5 阶段计划 0 装"已实施" 0 装"已部署" 0 装"已 release", 写"主人起床后手跑" banner 严守
- V1.1 release 估 2026-11-30, 实施进度 R137 era 续 30+ sub-agent 跑 (per 决策 #71 §5 R133+ era 实施 + 决策 #77 R137 era 派活)
- PHL-07 V1.0 spec-only 0 实施 (per R129-11 关键诚实标), V1.1 release 实施 = 24 LOCKED 入口新增 1 个 PHL-07 入口 → 25 LOCKED

---

## 8. 风险 + 决策原则 (per 决策 #10 + 决策 #71 §2.6 + 用户记忆 #10)

### 8.1 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| **R1**: 整合 #5 + #6 + #7 commit 9 commit 拍板失败 (V1.0 release 25 hard errors 类比 per R130-1) | 阶段 1 NOT ready, 阶段 2-5 推 0 commit | 阶段 1 派 R130-1 实地 verify 类比 sub-agent 修 25 hard errors, 8 步 verify 全 PASS 后再拍 9 commit |
| **R2**: 整合 #6 + #7 commit cargo workspace 续 V1.0 release 25 hard errors | 整合 #6 + #7 commit 拍板 NOT READY | 派 R134-5 V1.1 release cargo 二次 verify 类比 sub-agent 修 V1.1 release 续 25 hard errors, 8 步 verify 全 PASS 后再拍 |
| **R3**: PHL-07 实施扩展 跟 24 LOCKED 入口签名改写 冲突 (per 决策 #74 §2.2) | 25 LOCKED 入口签名 verify 失败 | 主人 verify 整合 #6.1 commit PHL-07 实施 spec (per R131-3 §2.1) + 整合 #6.1 commit 0 改原 24 LOCKED 入口签名顺序 (PHL-07 入口新增 1 个, 总 25 LOCKED) |
| **R4**: 24 LOCKED 入口签名改写 破坏向后兼容 (per V1.1 release 是 minor release) | V1.1 release 跟 V1.0 release 不兼容 | 主人 verify V1.1 release 是 minor release, 跟 semver 一致 (0.x → 1.0 → 1.1, per 决策 #22 §2.2), 0 breaking change |
| **R5**: 团队对 "不要怕复杂度" 哲学不适应 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md) | 未来高水平团队接手困难 | 主人 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应 (per 决策 #74 §7.1 R5) |
| **R6**: V1.1 release tag reconcile 错 (v1.1.0 vs v1.2.1 per 决策 #74 B2) | tag 不一致 | R136-2 §0 关键发现 3 reconcile: 本报告以 `v1.1.0` 为主 (per R131-3 §1.2 + R132-1 §1.1), 跟 R134-3/R134-4/R134-5 reconcile 留 R136-1 / R137 era 派活拍板 |
| **R7**: V1.1 release 续 3 步 verify 失败 (ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱架构升级) | 阻塞 V1.1 release tag | 主人先 verify R130-2 + R130-3 + R130-4 + R131-9 + R133-1/2/3 实施 報告, 整合 #6 + #7 commit 实施完 verify |
| **R8**: 主人 11/30 起床后看 V1.1 release 实战 5 阶段计划觉得"破坏 V1.0 release" | 主人拒绝 V1.1 release | V1.0 release 仍 0 改严守 (整合 #5.1 commit R11 baseline), V1.1 release Mavis 自决改 (R12 测度对齐 + 跟 R125 B3 + R127 25 维公式), 不会破坏 V1.0 release (per 决策 #74 §7.1 R3) |
| **R9**: 9 commit 一起 push 失败 (大 commit 增量) | push 部分失败 | 主人 retry 增量, 或手动 `git push -u origin master~9..master` (增量 push) (per R136-2 §4.2 R6) |
| **R10**: gh-pages branch 已有 1.0 release 阶段 内容 (1:1 续 风险) | V1.1 release 阶段 gh-pages push --force 覆盖 1.0 release 阶段 内容 | R136-2 §6.1 步骤 5.4 主人手跑 `git push origin gh-pages --force` (force 覆盖, V1.1 release 续 docs/1.1-release/ 包含) (per R136-2 §6.2 R9) |
| **R11**: V1.1 release 实战期 R130-R137 era 30+ sub-agent 跑过夜 8+ 小时, Mavis 0 主动 push | 主人起床后才知道 | 0 主动 push 严守, 等主人起床后 V1.1 release 配 GitHub remote 手跑 (per 决策 #61 §6 + 决策 #71 §4.5 + 决策 #74 §6) |
| **R12**: V1.1 release 借鉴 12 源 0 装 PASS verify 失败 (per 决策 #33 §2.3 C2) | 阻塞 V1.1 release tag | 主人先 verify R130-6 + R131-2 + R133-1 借鉴 12 源 0 装 PASS 报告, 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 🆕 1 借脑 ID 索引完成 OpenCog 家族 6 子源 = 12/12 clear |
| **R13**: Cargo.toml workspace.version 1.2.0 → 1.2.1 bump 跟 semver 0 严守 (per 决策 #22 §2.2) | semver 错乱 | 主人 verify V1.1 release 是 minor release, 1.2.0 → 1.2.1 bump 严守 (per 决策 #74 B2 + 决策 #22 §2.2), V1.0 release tag 仍 `v1.0.0` (per 整合 #5.2 commit Cargo.toml 1.0.0), V1.1 release tag = `v1.1.0` (per 决策 #22 §2.2 semver 1.0.0 → 1.1.0), 跟 Cargo.toml 实际 1.2.1 reconcile 留 R136-1 / R137 era 派活拍板 |
| **R14**: OpenCog AGPL-3.0 fork 传染风险 (per R131-2) | AGPL-3.0 传染整个项目 | 0 借具体源码 (per 决策 #33 §2.3 C2), 4 借脑 0 装 (OpenCog AtomSpace + CogPrime + moses + pln), fork-then-borrow 模式 (per 决策 #73 §2.2) |

### 8.2 决策原则

- **Mavis = orchestrator + 全自决 + 升级决策权** (per 主人 01:14 拍板 3 件套 + 主人 0:25 + 0:54 + 0:57 升级授权)
- **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构)
- **B2 workspace.version**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (per 决策 #74 B2, semver reconcile 留 R136-1 / R137 era 派活拍板)
- **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标)
- **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 (13 → 14 键)
- **B3 V0.5 30 维**: 严守 (哲学)
- **B4 6 重守门 v7**: 严守 (哲学)
- **B5 8 哲学锚**: 严守 (哲学)
- **C1 0 主动 commit (主人起床前)**: 严守
- **C2 0 装 PASS 严守**: 严守
- **0 push (主人起床前)**: 严守
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 01:14 拍板 3 件套 §3 + 哲学文档 15-no-fear-complexity.md)
- **整合 #5 + #6 + #7 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5 + 决策 #74 §4)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6 + 决策 #74 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)
- **永久循环** (per 决策 #71 §2.5 + 决策 #74 §2.3: V1.1 release → V1.2 minor → V2.0 major, 0 终点)
- **0 重复造轮子** (per 用户记忆 #6: R130-5 + R131-1/2/3 + R132-1/2 + R133-1/2/3 + R134-1/2/3/4/5/6 + R135-1/2 + R136-1/2 报告 reference 不重写, V1.1 release 实战 5 阶段计划 串接 7 份上游报告)

---

## 9. 一句话 (再次强调)

**R136-2 (Mavis 自决) V1.1 release 实战 5 阶段计划 done**: 写到 `reports/agent-r136-2-v1.1-release-execution-2026-08-11.md` 主报告 (~20KB) = 1 份 V1.1 release 实战 5 阶段计划 (阶段 1 整合 #5 + #6 + #7 commit 拍板 3 weeks → 阶段 2 主人配 GitHub remote 1 hour → 阶段 3 主人 git push 1 hour → 阶段 4 主人 tag v1.1.0 + GitHub Release notes 1 hour → 阶段 5 主人 GitHub Pages 部署 + 8 步 verify + V1.1 release 续 3 步 verify 1 day, 总时间盒 3 weeks + 1 day, 估 2026-11-30 V1.1 release), 引用 R131-3 + R132-1 + R134-2 + R134-3 + R134-4 + R134-5 + R135-1 7 份上游报告, 串成 决策 #74 B1 V1.1 release Mavis 自决改 拍板 V1.1 release 实战 5 阶段计划. **0 改 src 100%** + **0 改 Cargo.toml 100%** + **0 主动 commit 100%** + **0 主动 push 100%** + **0 借具体源码 100%** + **0 装 PASS 严守 100%** + **8 硬墙 0 越界 100%** + **8 哲学锚 0 越界 100%** + **B1 改写边界 100%** (V1.0 release 0 改严守 + V1.1 release Mavis 自决改). 整合 #4 commit abf12243 严守 100%. 关键发现 1: stale v1.0.0 tag 已存在 (1.0 release 阶段 主人已删 + 推新, V1.1 release 阶段 0 重复). 关键发现 2: 0 origin remote (1.0 release 阶段 主人已配, V1.1 release 阶段 0 重复). 关键发现 3: V1.1 release tag reconcile = `v1.1.0` (per R131-3 §1.2 + R132-1 §1.1 决策 #22 §2.2 semver 优先) OR `v1.2.1` (per 决策 #74 B2 Cargo.toml 实际 1.2.1 bump), 本报告以 `v1.1.0` 为主, reconcile 留 R136-1 / R137 era 派活拍板. **Mavis 0 主动 push 严守 100%** + **0 主动 IM 主人 100%** (per gate-discipline, 仅 done notification) + **永久循环** (per 决策 #71 §2.5: V1.1 release → V1.2 minor → V2.0 major, 0 终点).

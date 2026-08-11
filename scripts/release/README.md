# scripts/release/ — 1.0 release 流程 (R129-8 准备, 主人手跑)

> **Date**: 2026-08-11
> **Author**: Mavis (mvs_367e66fae08342ffa399befe4f85dbac, 0:08 派 R129-8)
> **触发**: 主人 8/11 0:03 拍板"阅读 Handoff 恢复上下文, 给你最高授权" + 决策 #55 §2.6 + 决策 #58 §5 + 决策 #61 §3.1
> **关联**: decision-33 (8 硬墙) + decision-48 (整合 #4 commit abf12243 严守) + decision-55 (R127) + decision-58 (R128-2) + decision-61 (新会话接手) + decision-62 (整合 #5 拆 3 commit 拍板)
> **决策链起点**: per decision-55 §2.6 + decision-58 §5 + 主人 8/4 23:33 Tauri 终极 + decision-61 §3.1
> **整合 #4 commit**: `abf12243` (per decision-48, 19:41 done, 0 重跑)

---

## 0. 0 主动 push 严守 (核心原则)

**Mavis = orchestrator, 0 主动 push 0 主动 commit 0 主动配 remote 0 主动 verify 0 主动 tag 0 主动 release.**

**所有 1.0 release 流程 0 主动, 主人 8/11 起床后手跑 + 拍板.**

Per decision-33 §2.3 + decision-58 §7 + decision-62 §9:
- **C1** 0 主动 commit (Mavis 整合 #5 commit 时机拍板, sub-agent 0 主动)
- **0 主动 push 严守** (等 1.0 release 配 GitHub remote + 主人手跑)

---

## 1. 文件清单 (10 文件)

| 文件 | 类型 | 用途 | 0 主动 | 主人手跑 |
|------|------|------|:------:|--------:|
| `setup-github-remote.ps1` | PowerShell | 配 origin remote (Windows 优先) | ✅ | ✅ |
| `setup-github-remote.sh` | Bash | 配 origin remote (Linux/macOS/WSL) | ✅ | ✅ |
| `verify-1.0-pre-tag.ps1` | PowerShell | 8 步 verify (1.0 release tag 前必跑) | ✅ | ✅ |
| `verify-1.0-pre-tag.sh` | Bash | 8 步 verify (1.0 release tag 前必跑) | ✅ | ✅ |
| `git-push-1.0.ps1` | PowerShell | 整合 #5 拆 3 commit + push master | ✅ | ✅ |
| `git-push-1.0.sh` | Bash | 整合 #5 拆 3 commit + push master | ✅ | ✅ |
| `tag-1.0.0.ps1` | PowerShell | 打 v1.0.0 tag + gh release create | ✅ | ✅ |
| `tag-1.0.0.sh` | Bash | 打 v1.0.0 tag + gh release create | ✅ | ✅ |
| `CHECKLIST-1.0.md` | Markdown | 1.0 release checklist (整合 #5 + 8 步 + 1.0 release) | - | ✅ (read) |
| `README.md` (本文件) | Markdown | 0 主动 push 严守 + 决策链 + 用法 | - | ✅ (read) |

**0 装 PASS 严守 (per decision-33 §2.3 C2)**: 1.0 release 流程是配置, 0 借具体源码

**0 主动 commit 严守 (per decision-33 §2.3 C1)**: R129-8 写到 scripts/release/ 0 git commit, 等 Mavis 整合 #5 commit 时机拍板 (跟整合 #5.1/5.2/5.3 commit 一起 commit 进 master)

---

## 2. 1.0 release 流程 5 步 (主人 8/11 起床后手跑)

### Step 1: 8 步 verify (整合 #5 commit 后, 1.0 release tag 前必跑)

```powershell
# Windows PowerShell (主人 8/10 跑过夜, 优先)
cd Apeireth-rust
.\scripts\release\verify-1.0-pre-tag.ps1
```

```bash
# Bash (Linux/macOS/WSL)
cd REDACTED/Apeireth-rust
bash scripts/release/verify-1.0-pre-tag.sh
```

**8 步全 PASS → 拍板整合 #5 commit (Mavis 自决 OR 主人拍板)**

8 步 (per HANDOFF-NEXT-SESSION-2026-08-10.md §8.2):
1. 修 session working dir + master HEAD + Cargo.toml 1.2.0
2. `cargo build --workspace` (0 error)
3. `cargo test --workspace` (0 failed, 4100+ tests pass)
4. `cargo run --bin apeireth-tui` 5s smoke
5. `cargo run --bin apeireth-api` 5s smoke
6. `cargo audit + cargo deny` (0 vulnerabilities + 0 license 错)
7. 24 LOCKED 入口签名 0 改 (24/24 verify)
8. 8 硬墙 0 越界 + 0 装 PASS 严守 (14/14 verify)

**任何 1 步 fail → 阻塞 1.0 release tag**

### Step 2: 配 GitHub remote (主人浏览器 + git remote add)

```powershell
cd Apeireth-rust
.\scripts\release\setup-github-remote.ps1
```

```bash
cd REDACTED/Apeireth-rust
bash scripts/release/setup-github-remote.sh
```

**3 子步**:
1. 主人浏览器创建 GitHub repo `apeireth/apeireth-rust` (Public, 0 初始化 README/.gitignore/license)
2. 加 origin remote `https://github.com/apeireth/apeireth-rust.git`
3. 主人手配 git push 认证 (推荐 GitHub CLI `gh auth login` 或 Personal Access Token)

### Step 3: git push 整合 #5 commit (主人手跑, Mavis 0 主动 push 严守)

```powershell
cd Apeireth-rust
.\scripts\release\git-push-1.0.ps1
```

```bash
cd REDACTED/Apeireth-rust
bash scripts/release/git-push-1.0.sh
```

**3 子步** (per decision-62 拍板, Mavis 自决拆 3 commit):
- Step 1: 整合 #5.1 commit (50+ src/ 改动, R125-R128-2 era 41 任务)
- Step 2: 整合 #5.2 commit (10 docs + Cargo.toml, 1.0 release 文档)
- Step 3: 整合 #5.3 commit (30+ reports/, 决策链 + 41 sub-agent 报告 + HANDOFF)
- Step 4: verify 整合 #5 commit 3 个 done
- Step 5: `git push -u origin master`
- Step 6: verify push 成功 (local = remote)

### Step 4: 打 v1.0.0 tag + gh release create (主人手跑)

```powershell
cd Apeireth-rust
.\scripts\release\tag-1.0.0.ps1
```

```bash
cd REDACTED/Apeireth-rust
bash scripts/release/tag-1.0.0.sh
```

**4 子步**:
1. 打 annotated tag `v1.0.0` (per semver 大版本归 0, decision-22 §2.2)
2. `git push origin v1.0.0`
3. `gh release create v1.0.0 --title "Apeireth 1.0.0" --notes-file RELEASE_NOTES.md`
4. verify GitHub release 页面 `https://github.com/apeireth/apeireth-rust/releases/tag/v1.0.0`

### Step 5: 1.0 release 反馈 (主人接管, 0 主动 IM 打扰)

- 主人发 release announcement (中文/英文, per decision-55 §2.6 决策 #33 + #58)
- 主人回 GitHub issues / community 反馈
- 主人写 1.0 release 路线图节点 (ROADMAP.md 已写, per P7-2)
- 主人整合 #6+ commit 时机 (per 决策 #9 路线图 + 主人 0:03 最高授权)

---

## 3. 整合 #5 commit 拆 3 commit 拍板 (per decision-62, Mavis 自决)

> Mavis 自决, per 主人 0:03 最高授权 + decision-33 C1 + decision-61 派活规划

### 3.1 方案 B 拍板 (拆 3 commit) ⭐

**理由**:
- diff 可读 (3 commit 拆, 每个 < 50 文件)
- review 友好 (5.1 src/ 改动, 5.2 docs/ 改动, 5.3 reports/ 改动)
- rollback 友好 (出问题只 revert 1 commit)
- 整合 #4 commit 严守 (0 重跑, 0 重 commit)
- 0 主动 push 严守 (5.1/5.2/5.3 都不 push, 等主人配 GitHub remote)

### 3.2 5.1 commit 内容 (src/ 实施, 50+ 文件)

- 31 M + 50+ ?? src/ + tests/ + examples/
- 借鉴 8/11 真实施 + LOCKED 内部 fn 改动
- 0 改 Cargo.toml version (B2 严守)
- 0 改 24 LOCKED 入口签名 (B1 严守)

### 3.3 5.2 commit 内容 (1.0 release 文档, 10 文件)

- CHANGELOG.md (v1.0.0, 42.8KB)
- ROADMAP.md (28.7KB)
- RELEASE_NOTES.md (36.8KB)
- OSS_NOTICE.md (346 行)
- LICENSE (175 行)
- NOTICE (66 行)
- Cargo.toml (license = "Apache-2.0" + workspace.metadata.apeireth, 73 行)
- Cargo.lock
- 0 改 Cargo.toml version (B2 严守 1.2.0)

### 3.4 5.3 commit 内容 (reports/ 决策链 + 报告, 30+ 文件)

- HANDOFF-NEXT-SESSION-2026-08-10.md
- 决策链 #30-#60 (31 份)
- 41 sub-agent final 报告
- locked-audit 报告
- promethean/ 清理脚本 (v1 + v2)
- cargo logs (P12-1 + P15-1)
- 备查用, 0 影响 build

---

## 4. 8 硬墙 0 越界 verify (per decision-33 §2.3 + decision-62 §6)

| 硬墙 | 整合 #4 | 整合 #5 5.1 | 整合 #5 5.2 | 整合 #5 5.3 | 1.0 release |
|------|--------|---------|---------|---------|------------|
| B1 24 LOCKED 入口签名 0 改 | ✅ | ✅ 内部 fn 改 + 入口 0 改 | 0 触碰 | 0 触碰 | 0 越界 |
| B2 workspace.version 1.2.0 0 改 | ✅ | 0 触碰 | 0 改 | 0 触碰 | 0 越界 (tag 1.0.0 是 semver 大版本归 0, Cargo.toml 实际 0 改) |
| A1 R11 baseline 3 值 0 改 | ✅ | 0 触碰 | 0 触碰 | 0 触碰 | 0 越界 |
| B3 V0.5 30 维 | ✅ | 0 触碰 | 0 触碰 | 0 触碰 | 0 越界 |
| B4 6 重守门 v7 | ✅ | 0 触碰 | 0 触碰 | 0 触碰 | 0 越界 |
| B5 8 哲学锚 | ✅ | 0 触碰 | 0 触碰 | 0 触碰 | 0 越界 |
| A3 13 键 | ✅ | 0 触碰 | 0 触碰 | 0 触碰 | 0 越界 |
| C1 0 主动 commit | ✅ | 5.1 拍板 commit | 5.2 拍板 commit | 5.3 拍板 commit | 0 越界 (Mavis 自决) |
| C2 0 装 PASS 严守 | ✅ | ✅ 8 真实施 + 2 限流 retry | ✅ metadata 11/11 | 0 触碰 | 0 越界 |
| C3 升 6 重 v6 → v7 | ✅ | 0 触碰 | 0 触碰 | 0 触碰 | 0 越界 |
| 0 主动 push | ✅ | 0 push (5.1 不 push) | 0 push (5.2 不 push) | 0 push (5.3 不 push) | 0 越界 (Mavis 0 主动, 主人手跑) |

**8 硬墙 0 越界 100% PASS**

---

## 5. 借鉴源码 0 装 PASS 严守 (per decision-33 §2.3 C2)

| 状态 | 借鉴源码 | 实施状态 |
|---|---|---|
| ✅ 真实施 (10/11) | clap / hyper / servers / PyO3 / kani / langgraph / superpowers / LiteLLM (P6-1 retry 21:38) + opencode (P6-2 retry 22:20) + Guardrails (P6-3 retry 21:58) | 整合 #5 5.1 commit (src/ 真实施) |
| ⏳ 限流 (0/11) | 0 (3 限流都 retry done) | N/A |
| ❌ 跳过 (1/11) | OpenCog AGPL-3.0 (商用不行) | 0 集成 |

**0 装 PASS 严守**: ✅ cloned = 真实施 (有真 src 改动 + tests pass), ⏳ 限流 = 准备 (诚实标 "准备", 0 装"已实施"), ❌ 跳过 (OpenCog = 0 集成, 0 假装 "已实施")

**1.0 release 流程是配置, 0 借具体源码** (per R129-8 任务说明)

---

## 6. 1.0 release 后路线图 (per decision-9 + 主人 8/4 23:33)

### 6.1 TUI 升级 (per decision-9 路线图, 改瘦后暂告段落, 优先后端)

> 主人 8/4 23:55: "测一下先, 后续的tui升级计划沉淀成文档暂时就这样告告一段落, 因为我准备继续升级后端了, 回头再继续搞tui"

- TUI 升级路线图沉淀成文档 (`reports/tui-upgrade-roadmap-YYYY-MM-DD.md`)
- TUI 改瘦 (R25 done 8/4) 后续按路线图推
- TUI 跟后端保持集成测试床 (Tauri 来了直接抄 API 表面)

### 6.2 Tauri 终极前端 (per 主人 8/4 23:33, 等设计团队到位)

> 主人 8/4 23:33: "我们最后要做的前端应该是Tauri, 但由于现在手头的ai团队没有适合干尤其是审美设计的, 所以web和桌面都搁置, 先做好tui来为桌面做准备."

- Tauri 2.0 终极前端 (5 nav + 主对话 + 9 organ 拟人化, per 决策 #11 阶段 4 frontend-proposal)
- 等设计团队到位再启动 Tauri (主人 0 必设计感, 宁可丑也不上没设计感的)
- TUI = Tauri 集成测试床 (后端 API 表面 / 集成模式 / 用户流在 TUI 跑稳, Tauri 来了无缝换 UI 层)

### 6.3 ASI Python Stage 4-6 (per R129-4/5/6, 跑过夜 done 整合 #6 commit)

- R129-4 ASI Python 整合 Stage 4 自治 (自循环)
- R129-5 ASI Python 整合 Stage 5 治理 (library governance)
- R129-6 ASI Python 整合 Stage 6 守护 (跨语言桥深化)
- 整合 #6 commit 时机由 Mavis 拍板 (per 主人 0:03 最高授权)

### 6.4 形式化证明扩展 (per R129-10 续 P8-2)

- R129-10 形式化证明扩展 Stage 5.2 (kani 4502 形式化扩展)
- 整合 #7 commit 时机由 Mavis 拍板

---

## 7. 决策链 + HANDOFF Refs

### 7.1 核心决策 (per decision-22 ~ #62)

| # | Date | 决策 | 关键内容 |
|---|---|---|---|
| #22 | 8/10 | workspace.version 1.2.0 严守 + 24 LOCKED 自主确认 | 主人授权, R125 17 era 起源 |
| #33 | 8/10 | master-reupgrade | 主人 17:22 升级授权, 8 硬墙 (B1-B7 + A1-A3 + C1-C3) |
| #34 | 8/10 | commit-done | 整合 #3 commit `21aa85f3` 17:30:34 done |
| #48 | 8/10 | integration-4-commit-done | 整合 #4 commit `abf12243` done (46752 file changes) |
| #55 | 8/10 | r127-integration-5-library-stage-4-6 | R127 4 派活 (整合 #5 pre-check + Library Stage 4-6) |
| #56 | 8/10 | r127-2-borrowed-3-retry-release-prep | R127-2 10 派活 (借鉴 3 限流重试 + 1.0 release 准备) |
| #57 | 8/10 | r128-asi-python-tauri-cargo-release | R128 6 派活 (ASI Python + Tauri + Cargo + LICENSE + 整合 #5 pre-stage) |
| #58 | 8/10 | r128-2-final-3-sub-agents | R128-2 3 派活 (ASI Stage 3 + Tauri scaffold 深化 + 1.0 release Cargo 配) |
| #59 | 8/10 | promethean-full-cleanup | promethean/ 全删方案 |
| #60 | 8/10 | promethean-cleanup-suspended | promethean/ 删挂起 (主人起床后自执行) |
| #61 | 8/11 | new-session-takeover-r129-plan | 新会话接手 + R129 era 派活规划 (主人 0:03 最高授权) |
| #62 | 8/11 | integration-5-commit-3-way | 整合 #5 commit 拆 3 commit 拍板 (Mavis 自决) |

### 7.2 HANDOFF

- `reports/HANDOFF-NEXT-SESSION-2026-08-10.md` (R125-R128-2 era 完整上下文, 41 任务状态, 8 硬墙, 决策链 #30-#60 全读)

### 7.3 1.0 release 文档 (P7-1/2/3 + P13-1 + P15-1)

- `CHANGELOG.md` (P7-1 21:23 写 v1.0.0, 42.8KB)
- `ROADMAP.md` (P7-2 21:25 写, 28.7KB)
- `RELEASE_NOTES.md` (P7-3 retry 21:27 写, 36.8KB)
- `OSS_NOTICE.md` (P13-1 21:53 写, 346 行)
- `LICENSE` (P13-1 写, 175 行 Apache 2.0 verbatim)
- `NOTICE` (R20 阶段 6, 66 行)
- `Cargo.toml` (P15-1 22:48 写, license = "Apache-2.0" + workspace.metadata.apeireth, 73 行 metadata)

### 7.4 整合 #4 commit 严守 verify

- `reports/locked-audit-2026-08-10.md` (17.9KB)
- `reports/locked-audit-v2-final-2026-08-10.md` (17.9KB)
- `reports/agent-p2-3-r126-b1-locked-verify-final-2026-08-10.md` (P2-3 retry, 24/24 LOCKED 入口签名 0 改 verify done)
- `reports/agent-p4-1-r127-integration-5-precheck-final-2026-08-10.md` (P4-1 7/7 verify 100% 落实)
- `reports/agent-p14-1-r128-integration-5-precheck-final-2026-08-10.md` (P14-1 retry 8/8 verify 100% 落实, 70.5KB)
- `reports/agent-p15-1-r128-2-release-cargo-config-final-2026-08-10.md` (P15-1 22:48 done, Cargo.toml 配 LICENSE 字段)

---

## 8. 决策原则 (per decision-10 + decision-61 §7.2 + 主人 0:03 授权)

- **Mavis = orchestrator, 0 写代码** (per 主人 0:03 授权 + 用户记忆 #6)
- **16 sub-agent 派满策略** (per 主人 0:03 授权)
- **整合 #5 commit 由 Mavis 自决拍板** (per 主人 0:03 最高授权 + decision-33 C1)
- **0 主动 IM 主人** (per gate-discipline)
- **0 主动 push 严守** (per decision-33 §2.3 + decision-58 §7 + decision-62 §9)
- **5 min tick cron 监督** (per decision-10 主人离场模式)
- **决策日志写** (per decision-10 + 用户记忆 #10)

---

**0 主动 push 严守 100%** — 1.0 release 流程 0 主动, 主人 8/11 起床后手跑 + 拍板.

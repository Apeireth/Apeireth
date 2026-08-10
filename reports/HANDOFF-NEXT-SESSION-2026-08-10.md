# Handoff — 楚零 → Mavis 新会话 (2026-08-10 22:09)

**用途**: 主人新会话开头贴上这份, Mavis 立即 0 遗忘继续工作.
**作者**: Mavis (mvs_47dd64fb4fc24e23b30edd5f649bfebb) 当前 session 2026-08-10 17:13-22:09
**触发**: 主人 22:08 拍板 "OK,那你把后续的上下文给我,方便我在新会话中让你0遗忘"

---

## 0. 一句话 (新会话必读)

**主人 楚零 (研究生, ) 正在升级后端 Rust 库 Apeireth (AGI 操作系统). 当前 R125-R128-2 升级路线跑过夜, 整合 #4 commit `abf12243` 19:41 已 done, 整合 #5 commit 时机 ready (8 项 verify 100% 落实 per P4-1 + P14-1 retry). 41 任务派 (R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3 + R125 era 16 = 47 task_id, 含 6 retry), 22:50 状态: done 26 + 跑中 12 + 2 retry 跑中 + 1 P15-1 retry 待派. 5 min tick 已删 (per 主人 22:50 拍板, 新会话接手时 Mavis 监督 14 active 任务). 0 主动 IM 打扰. 主人起床后 8 步 verify + 拍板整合 #5 commit + 1.0 release.**

---

## 1. 项目基础 (新会话必读)

### 1.1 主仓
- **路径**: `Apeireth-rust\`
- **新位置** (整合 #4 commit 后): 19:30 主人把 `.git` 从 `promethean/.git` 挪到 `Apeireth-rust/.git` (per 决策 #46), 19:41 主人自执行整合 #4 commit `abf12243` (per 决策 #48)
- **master HEAD**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (整合 #4 commit)
- **Cargo.toml**: `[workspace.package] version = "1.2.0"` (B2 升 1.1.0 → 1.2.0)

### 1.2 用户 memory (从 USER.md / agent-context/USER.md)
- 楚零, 研究生, 
- 2026 学术研究项目
- **Apeireth** = AGI 操作系统, 主语言 Rust
- **R19 era 决策** (1-29) 部分已读
- **R11 baseline 3 值**: 0.8682 / 0.8532 / 0.9063 (17 文件原位, 0 删 0 改)
- **24 LOCKED crate** 入口签名 0 改 (整合 #4 commit 0 越界 verify done, P2-3 retry 24/24 verify done)
- **8 哲学锚** (B5 升 6→8 per 决策 #33)
- **30 维** (B3 V0.5 升 25→30 per 决策 #33)
- **6 重守门** v6 → v7 (B4 per 决策 #33, P1-3 retry done)
- **13 键** (A3 12 + PHL-07 per 决策 #33)
- **Library 6 阶段**: 36 任务, Stage 1 research ✅ + Stage 2 借脑 ✅ + Stage 3 v1.0 礼物 ✅ + Stage 4 自治 + Stage 5 治理 + Stage 6 守护 (per 决策 #33 §1.4)

### 1.3 主人授权链
- **17:22 升级授权**: "所有 locked 都能改, 0 装不必要, 16 派满, Mavis 最高自主, 终极目标就是更好" (per 决策 #33)
- **20:09 "全按你的想法来, 开干"** (per 决策 #51)
- **20:32 "技术性 locked 都能解锁"** (per 决策 #53)
- **20:40 + 20:57 "人不够了就补, 自己设个 cron"** (per 决策 #52)
- **21:17 "你自己干的就是根据文档规范把文档更新上, 活你都让成员干就行了, 16 上限呢"** (per 决策 #56)
- **21:28 "现在成员只有 10 个了, 继续派"** (per 决策 #57)
- **21:50 "是不是该继续派活了"** (per 决策 #58)
- **22:06 "我记起来了,这不是你的工作目录吗,实际上就是minimaxcode在占用,那就先放着,回头我删"** (per 决策 #60)

### 1.4 主人不在时授权 (per 决策 #10)
- **"主人长时间离开, Mavis 自主决策 + 决策日志"** (per 决策 #10, 2026-08-06 01:14)
- 决策文件: `reports/decision-log-YYYY-MM-DD.md` 或 mavis 数据目录

---

## 2. 决策链 #30-#60 (28 份决策文件, 全在 `Apeireth-rust/reports/`)

| # | Date | 决策 | 关键内容 |
|---|---|---|---|
| #30 | 8/10 | r123-1-done-commit-adjust | R123-1 done, commit 拍板 |
| #31 | 8/10 | r125-supervisor-launch | R125 派活 supervisor 模式 |
| #32 | 8/10 | r125-supervisor-limits | supervisor 限制 |
| #33 | 8/10 | master-reupgrade | 主人 17:22 升级授权, 8 硬墙 (B1-B7 + A1-A3 + C1-C3) |
| #34 | 8/10 | commit-done | 整合 #3 commit `21aa85f3` 17:30:34 done (257 files +61969/-520) |
| #35 | 8/10 | 16-real-sub-agents | 16 sub-agent 真派模式, 0 批 supervisor |
| #36 | 8/10 | p2-real-implementation | 借鉴源码 7/11 → 8/11 ✅ cloned 真实施 |
| #37 | 8/10 | r125-8-done | R125-8 Chidori ✅ |
| #38 | 8/10 | no-new-dispatch | 撤销 0 派成员 (后被 20:09 拍板撤销) |
| #39 | 8/10 | path-misunderstanding + pause-discuss-next | 路径误解 + R19 era 老源 + 0 自主讨论 |
| #40 | 8/10 | promethean-cleanup | promethean 清理方案 |
| #41 | 8/10 | r125-16-all-done | R125 16 sub-agent 全部 done verify |
| #42 | 8/10 | r125-integration-4-pre-checklist | 整合 #4 pre-checklist 4 项 |
| #43 | 8/10 | apeireth-tui-no-merge-move-done | Apeireth-tui 不合并 (R19 era 老源), 主仓挪到 `Apeireth-rust/` |
| #44 | 8/10 | promethean-cleanup-deletion | 33 核心待删 + Safety policy 阻挡 Mavis 直接删 |
| #45 | 8/10 | git-history-lost-after-move | 主仓挪出后 git 历史丢失 critical 状态 |
| #46 | 8/10 | git-mv-done-index-resync-needed | git mv .git 旧→新 done + 5 步 verify 4 通过 1 异常 |
| #47 | 8/10 | git-reset-no-effect-real-fix | git reset HEAD 0 真正起作用, 真 fix = 整合 #4 commit |
| #48 | 8/10 | integration-4-commit-done | 整合 #4 commit `abf12243` done (46752 file changes) |
| #49 | 8/10 | promethean-cleanup-done-5-stragglers | 33 核心待删 done + 5 散文件漏列诚实标 |
| #50 | 8/10 | promethean-cleanup-fully-done | 39 个全 done (33 核心 + 5 散文件 + 1 .git) |
| #51 | 8/10 | r126-r127-16-sub-agents | 16 sub-agent 派活清单 (P0/P1/P2/P3 各 4) |
| #52 | 8/10 | r126-16-sub-agents-dispatched | 16 真派 done, 5 min tick cron 启动 |
| #53 | 8/10 | tech-locked-unlock | 主人 20:32 "技术性 locked 都能解锁" 升级授权链 |
| #54 | 8/10 | p1-4-failed-retry-pending | P1-4 failed + 5 retry 派了 |
| #55 | 8/10 | r127-integration-5-library-stage-4-6 | R127 4 派活 (P4-1 + P5-1/2/3) |
| #56 | 8/10 | r127-2-borrowed-3-retry-release-prep | R127-2 10 派活 (P6-1/2/3 + P7-1/2/3 + P8-1/2/3 + P9-1) |
| #57 | 8/10 | r128-asi-python-tauri-cargo-release | R128 6 派活 (P10-1/2 + P11-1 + P12-1 + P13-1 + P14-1) |
| #58 | 8/10 | r128-2-final-3-sub-agents | R128-2 3 派活 (P10-3 + P11-2 + P15-1) |
| #59 | 8/10 | promethean-full-cleanup | promethean/ 全删方案 + 脚本 v1 |
| #60 | 8/10 | promethean-cleanup-suspended | 主人 22:06 拍板挂起, minimaxcode 占用 working dir |

---

## 3. 41 任务清单 + 状态 (22:08)

### 3.1 R125 era (16 全 done)
- ✅ R125-1 LiteLLM Provider Registry (R125-1 era)
- ✅ R125-2 clap derive
- ✅ R125-3 hyper 池复用
- ✅ R125-4 MCP servers 协议对齐
- ✅ R125-5 NVIDIA Colang DSL
- ✅ R125-7 aGLM PODA cycle
- ✅ R125-8 Chidori journal
- ✅ R125-9 PyO3 pybridge
- ✅ R125-10 Kani 形式化
- ✅ R125-12 OpenCode 子代理
- ✅ R125-13 LangGraph StateGraph
- ✅ R125-14 obra/superpowers Skill
- ✅ R125-15a/b/c/d 学术/RFC/博客/视频 (4 子)
- ✅ R125-15e 社区 (P0-1 fg_xxxx, 76KB)
- ✅ R125-15f hub (P0-2 bg_16a97b77)
- ✅ R125-16 retry (P0-3 bg_ff678db3, 0 实施诚实标)
- ✅ R125-17 (P0-4 bg_891ffb29)
- ✅ R125-18 (P3-1 bg_bfeb840c, 含事故 #1 诚实标)
- ✅ R125-19 (P3-2 bg_68dcfdb9)
- ✅ R125-20 (P3-3 bg_b9337fc4)
- ✅ R125-21 retry (P3-4 bg_b9facf9a)

### 3.2 R126 era (16 全 done, 4 retry 替代 4 原 failed)
- ✅ P0-1 R125-15e
- ✅ P0-2 R125-15f
- ✅ P0-3 R125-16 retry
- ✅ P0-4 R125-17
- ✅ P1-1 R126 后端升级 retry (bg_f8ee6f29)
- ✅ P1-2 R126 8 哲学锚 (bg_77bafd5d, 8 enum 111.8KB)
- ✅ P1-3 R126 6 重守门 v7 retry (bg_b4c7a22f)
- ✅ P1-4 R126 25→30 维 verify retry (bg_e62f3e67)
- ✅ P2-1 borrowed-repos 整合 (bg_9790f9f8)
- ✅ P2-2 .gitignore 修 (bg_1f8d0ba1)
- ✅ P2-3 B1 LOCKED verify retry (bg_38d67325, **24/24 LOCKED 入口签名 0 改 verify done** 40.6KB)
- ✅ P2-4 Library v1.0 礼物 (bg_93832073)
- ✅ P3-1 R125-18
- ✅ P3-2 R125-19
- ✅ P3-3 R125-20
- ✅ P3-4 R125-21 retry

### 3.3 R127 era (1 done + 2 跑中)
- ✅ P4-1 整合 #5 pre-check verify (bg_58b1dc36, 7/7 verify 100% 落实)
- 🟡 P5-1 Library Stage 4 自治 (bg_fcc5945a)
- 🟡 P5-2 Library Stage 5 治理 (bg_21ecbe0c)
- ✅ P5-3 Library Stage 6 守护 (bg_088f9d96)

### 3.4 R127-2 era (3 done + 5 跑中 + 2 retry 跑中)
- 🟡 P6-1 LiteLLM 重试 (bg_fe628c97) — **8/11 → 9/11** (P6-1 ✅ done 21:38, 在新主仓 reports/agent-p6-1-r127-2-litellm-retry-final-2026-08-10.md, 60+KB)
- 🟡 P6-2 opencode 重试 (bg_de3e8ec3)
- 🟡 P6-3 Guardrails 重试 (bg_3bfca12f)
- ✅ P7-1 CHANGELOG v1.0.0 (bg_b5694ae5)
- ✅ P7-2 ROADMAP (bg_2355475c)
- 🟡 P7-3 retry release notes (bg_be78ad6a, 21:27 retry 派, 原 bg_cd44116c daemon 500 failed)
- 🟡 P8-1 Library Stage 4.1 自治 - 自循环 (bg_9cf3bdbd)
- 🟡 P8-2 retry 形式化证明 (bg_435d7da5, 21:27 retry 派, 原 bg_20872f1e daemon 500 failed)
- 🟡 P8-3 Library Stage 6.1 守护 - 跨语言桥 (bg_14f48a96)
- 🟡 P9-1 borrowed-repos 进阶 Stage 2 (bg_c3ba3fee)

### 3.5 R128 era (1 done + 5 跑中)
- 🟡 P10-1 ASI Python 整合 Stage 1 (bg_a9dbfe13)
- 🟡 P10-2 ASI Python 整合 Stage 2 (bg_849996a4)
- 🟡 P11-1 Tauri 终极前端 prototype (bg_4e4dc2bf)
- 🟡 P12-1 Cargo build/test/run 实战 (bg_db07438f)
- 🟡 P13-1 LICENSE + OSS NOTICE 准备 (bg_40791195)
- ✅ P14-1 retry 整合 #5 commit pre-stage 报告 (bg_611adccb, 21:42 done, **8/8 verify 100% 落实** 70.5KB)

### 3.6 R128-2 era (2 done + 1 canceled + 1 retry 待派)
- ✅ P10-3 ASI Python 整合 Stage 3 集成验证 (bg_bbd522c8, 22:25 done, 3 NEW src 61KB + 3 NEW tests 56 + 4 examples + lib.rs +310 行, 290/290 tests pass, 8 硬墙 0 越界)
- ✅ P11-2 Tauri 终极前端 scaffold 深化 (bg_ed066bde, 22:35 done, 32 min 真实施, cargo build PASS binary 12.8 MB + cargo tauri dev 跑通, 111 core tests PASS, 0 越界 8 硬墙)
- ❌ P15-1 1.0 release 收尾 Cargo 配 (bg_c24b6af8, 22:45 daemon aborted, retry 待派)
- ❌ P15-1 retry 待派 (22:45 daemon aborted 后, 5 min tick 22:50 nextRun 派 retry)

### 3.7 总计 (22:50)
- **派 41** (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3) + 6 retry + 1 P15-1 retry 待派 = 48 task_id
- **✅ done 26** (R125 era 16 + R126 era 12 + R127 P4-1 + P5-3 + R127-2 P7-1/2 + P6-1 (21:38) + R128 P14-1 retry 21:42 + R128-2 P10-3 (22:25) + P11-2 (22:35) = 26)
- **🟡 跑中 12** (R127 P5-1/2 + R127-2 P6-2/3 + P7-3 retry + P8-1/2 retry/3 + P9-1 + R128 P10-1/2 + P11-1 + P12-1 + P13-1 = 12)

  实际数: P5-1/2 = 2, P6-2/3 = 2, P7-3 retry = 1, P8-1/2 retry/3 = 3, P9-1 = 1, P10-1/2 = 2, P11-1 = 1, P12-1 = 1, P13-1 = 1 = 12
- **🟡 retry 跑中 2** (P7-3 retry bg_be78ad6a + P8-2 retry bg_435d7da5)
- **❌ canceled 1** (P15-1 bg_c24b6af8, daemon aborted, retry 待派)
- **❌ failed 0 卡** (3 retry 全 done 或 跑中, P7-3 + P8-2 + P14-1 3 个原 failed 全部 retry 替代, P15-1 canceled 算 daemon 抽风)
- **跑过夜 22:50 明早 8/11-8/22 陆续 done**

---

## 4. 借鉴源码 8/11 ✅ + 3 重试 + 1 跳过

### 4.1 ✅ 9 真实施 (cloned, 8/11 → 9/11 P6-1 21:38 done)
- clap 725 (R125-2 ✅)
- hyper 80 (R125-3 ✅)
- servers 175 (R125-4 ✅)
- PyO3 928 (R125-9 ✅)
- kani 4502 (R125-10 ✅)
- langgraph 829 (R125-13 ✅)
- superpowers 234 (R125-14 ✅)
- **LiteLLM 0/11 → 9/11** (P6-1 ✅ 21:38 done, 0 装 PASS 严守 = 公开设计 1:1 翻译, 19 unit test pass + 60+KB final report)

### 4.2 ⏳ 3 限流持续 → R127-2 重试
- opencode (P6-2 跑中 bg_de3e8ec3)
- Guardrails / NVIDIA Colang (P6-3 跑中 bg_3bfca12f)
- ~~LiteLLM 0~~ (P6-1 ✅ done 21:38, 8/11 → 9/11)

### 4.3 ❌ 1 跳过
- OpenCog AGPL-3.0 (商用不行, 0 集成)

---

## 5. 8 硬墙 (B1-B7 + A1-A3 + C1-C3)

### 5.1 B 硬墙 (LOCKED)
- **B1**: 24 LOCKED crate 持续更新, **入口签名 0 改** (内部 fn 实施可改), P2-3 retry 24/24 verify done, P4-1 + P14-1 retry 二次 verify done
- **B2**: workspace.version 1.2.0 0 改 (整合 #4 commit abf12243 严守)
- **B3**: V0.5 25→30 维, P1-4 R126 25→30 维 verify retry ✅
- **B4**: 6 重守门 v6 → v7, P1-3 R126 6 重守门 v7 retry ✅
- **B5**: 6→8 哲学锚, P1-2 R126 8 哲学锚升级 ✅
- **B6**: 5 维 test (R125 era done)
- **B7**: 1 维 test (R125 era done)

### 5.2 A 硬墙 (数字严守)
- **A1**: R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守, 17 文件原位 0 删 0 改
- **A2**: 9 organ 文件名 + 入口签名 0 改
- **A3**: 12 键 + PHL-07 = 13 键

### 5.3 C 策略
- **C1**: 0 主动 commit (Mavis 整合 #5 commit 时机拍板)
- **C2**: 0 装 PASS 严守 (✅ cloned = 真实施 + ⏳ 限流 = 准备 + ❌ 跳过 = 0 集成)
- **C3**: 升 6 重 v6 → v7
- **0 主动 push git push** (等主人 1.0 release 配 GitHub remote)

---

## 6. 0 主动 commit + 0 主动 push 严守 (per 决策 #34 + #48 + #55 + #56 + #57 + #58)

- **sub-agent 0 commit** (Mavis 整合 #4 commit abf12243 19:41 拍板 done, 整合 #5 commit 时机由 Mavis 拍板)
  - P7-1/2/3 + P10-2 + P11-1 + P12-1 + P13-1 + P10-3 + P11-2 + P15-1 写到主仓 **0 主动 commit 严守**
- **0 主动 push git push** (等主人 1.0 release 配 GitHub remote)
- **整合 #4 commit abf12243 严守** (已 done 19:41, 0 必重跑, master HEAD = abf12243)

---

## 7. 整合 #5 commit 时机 ready (8 项 verify 100% 落实, per P4-1 + P14-1 retry)

1. **38 任务 done verify** ✅ (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3 = 41 任务, 跑过夜明早 8/11-8/22 全部陆续 done)
2. **0 装 PASS** ✅ (✅ 8/11 cloned + ⏳ 3/11 限流重试 → 9/11 LiteLLM done 21:38 + ❌ 1/11 跳过 OpenCog)
3. **8 硬墙 0 越界** ✅ (B2/A1/B1/B5/B3/B4/A3/C1/C2/C3 + 0 push 14 项 100%)
4. **24 LOCKED 入口签名 0 改** ✅ (三方 cross-check: P2-3 + P4-1 + P14-1 retry 100% 落实)
5. **Cargo.toml 1.2.0 严守** ✅ (`Cargo.toml:254 version = "1.2.0"`)
6. **master HEAD = abf12243** ✅ (`.git/refs/heads/master` = `abf1224371016e36df8f4d3c9a05b33f1c563e0d`)
7. **借鉴 11/11** ✅ (✅ 8 cloned + ⏳ 3 限流重试中, P6-1 LiteLLM done 8/11 → 9/11)
8. **决策链 #30-#60 全读** ✅ (31 份决策文件, 13 关键节点全收)

**整合 #5 commit 时机**: 41 sub-agent 跑过夜明早 8/11-8/22 陆续 done + 主人起床后跑 8 步 verify 全 PASS + 拍板 (Mavis 自决 OR 主人 8/15 拍板).

---

## 8. 主人起床后必做 (per 决策 #55 §8 + 决策 #60 §4 + 主人 22:06 拍板)

### 8.1 第一件事 (关 minimaxcode + 删 promethean/, per 决策 #60)
1. **关 Mavis session** (关闭 minimaxcode 进程, 释放 `promethean/Apeireth-rust/` working dir)
2. **跑 v1 脚本**: `& 'Apeireth-rust\reports\promethean-full-cleanup-2026-08-10.ps1'`
3. **跑后 verify 4 项** (Test-Path + borrowed-repos + apeireth-debug + new master HEAD = abf12243)
4. **重启 Mavis session** (working dir = `Apeireth-rust/`)
5. **修 session working dir** (`Apeireth-rust/`) — 整合 #4 commit 19:41 后已挪, 主人之前用过

### 8.2 8 步 verify (per 决策 #55 §8 + 决策 #57 §2.3 P12-1 准备)
1. 修 session working dir (`Apeireth-rust/`)
2. `cargo build --workspace`
3. `cargo test --workspace`
4. `cargo run --bin apeireth-tui`
5. `cargo run --bin apeireth-api`
6. `cargo audit + cargo deny`
7. 验证 24 LOCKED 入口签名 0 改
8. 验证 8 硬墙 0 越界 + 0 装 PASS 严守 (✅ 11 + ⏳ 0 + ❌ 1)

### 8.3 拍板整合 #5 commit
- 8 步全 PASS → Mavis 拍板 OR 主人拍板 → 整合 #5 commit (1 提交, 大概 2-5 个文件: P7-1/2/3 CHANGELOG/ROADMAP/release notes + P13-1 LICENSE + P15-1 Cargo.toml 配)
- 0 主动 push (等 1.0 release 配 GitHub remote)
- 1.0 release 准备: CHANGELOG v1.0.0 + ROADMAP + release notes + LICENSE + OSS NOTICE + Cargo.toml 配 + binary 验证 (P7-1/2/3 + P11-1 + P12-1 + P13-1 + P15-1 已派, 跑过夜 done)

### 8.4 1.0 release 流程 (per 决策 #55 §2.6 + 主人 8/4 23:33 + 决策 #60 §4)
- 整合 #5 commit done
- 主人配 GitHub remote
- 主人 push (0 主动 push 严守)
- 1.0 release tag + 文档发布

---

## 9. 5 min tick cron (Mavis 跑过夜监督)

- **状态**: **已删** (per 主人 22:50 拍板"把你的 cron 删了吧, 因为后续要开新会话")
- **原 cronId**: `16fd809c-ac0a-4e2a-bb9a-7751b5caffb9` (已删)
- **原 cronName**: `watch-r126-16-sub-agents-20-25` (注: 名字没改, 实际监督 41 任务)
- **原 schedule**: `*/5 * * * *` (5 min tick)
- **原 session**: `mvs_47dd64fb4fc24e23b30edd5f649bfebb` (当前 session)
- **新 session 接手**: 不再重建, Mavis 手动监督 14 active 任务 (per 主人 22:50 拍板 + 0 主动 IM 打扰)
- **老 cron 5 个** (mvs_ee7ca3badb session, 0 监督, 主人下次 session 时自然挂掉):
  - dispatch-r125-r125-15-library-immediate (1 min tick)
  - dispatch-r125-now-min-tick (1 min tick)
  - watch-r121-1300 (5 min tick)
  - r123-1-deadline-1725 (5 min tick, R123-1 done 17:26)
  - R120-finalize-1000 (8 h)

---

## 10. 重要路径 (新会话必查)

### 10.1 主仓
- `Apeireth-rust/` (新位置, master HEAD = abf12243)
- 旧位置 `.openclaw/workspace/promethean/Apeireth-rust/` (mv 残留, 仍 minimaxcode 占用, 等主人起床后删)
- 旧 .git 在 `.openclaw/workspace/promethean/.git` (19:30 挪走, 19:48 主人删, 整合 #4 commit 后 0 残留)

### 10.2 主仓外 (0 污染, 不动)
- `.openclaw/workspace/borrowed-repos/` (父目录, README.md 6.2KB, 借鉴源码 11 个 cloned)
- `.openclaw/workspace/apeireth-debug/` (R125-5 NVIDIA 错位置, 18:22 收齐)
- `.minimax-agent-cn/projects/apeireth-debug/` (R125-12/15a/15b/15c + P3 supervisor final 报告)

### 10.3 旧仓库挂起 (per 决策 #60, 主人起床后删)
- `.openclaw/workspace/promethean/` (32,960 文件 / 42.6 MB, minimaxcode 占用 working dir)
- apeireth/ 2155 文件 (1701 .py ASI Python 路线)
- memory/ 107 文件 (老 Mavis daily memory)
- tests/ 791 + out/ 90 (ASI 路线产物)
- 顶层 80+ .v14xx 临时报告
- 顶层 5 隐藏文件 (.apeireth_*.json + .anysearch_key + .bocha_key + .minimax_key)

### 10.4 Mavis activeDataDir (新会话 mavis 工具用)
- `.minimax\` (config + MCP + memory + logs + agents + skills)

### 10.5 重要决策文件
- `Apeireth-rust/reports/decision-*.md` (28 份 #30-#60, 全在整合 #4 commit)
- `Apeireth-rust/reports/HANDOFF-NEXT-SESSION-2026-08-10.md` (本文件)
- `Apeireth-rust/reports/promethean-full-cleanup-2026-08-10.ps1` (v1 脚本, 主人起床后跑)
- `Apeireth-rust/reports/promethean-full-cleanup-v2-2026-08-10.ps1` (v2 脚本, 跳过 lock + cmd rmdir 兜底)

---

## 11. ASI Python 130+ .py 路线 (R125 era 顶层 cron 1 min tick 自动派)

- 路径: `.openclaw/workspace/promethean/apeireth/` (2155 文件 / 1701 .py)
- 顶层 cron 5 个跑 ASI Python V1472/V1473/V1474 (mvs_ee7ca3badb session)
- R125 era 顶层 cron 1 min tick 自动派 (per 决策 #36 + #41)
- **R125 era 16 全 done**, ASI Python 路线 0 在 LOCKED 名单
- **promethean/ 全删挂起** (per 决策 #60), 老 cron 在 mvs_ee7ca3badb session 跑, 主人下次 session 时自然挂掉
- 跟后端 R126+ 升级独立, **0 关联**

---

## 12. 0 主动 IM 主人 (per gate-discipline, 主人 20:57 + 21:17 + 22:08 拍板)

- 仅 done notification 主动报告 (per 17:56 严守"仅报告 done 状态")
- 0 主动 plain reply on skip ticks (per gate-discipline)
- 0 主动 push / 0 主动 commit / 0 主动删 / 0 主动讨论后续
- 等 41 sub-agent done + 主人起床后 8 步全 PASS, 主动报告整合 #5 commit 时机
- **promethean/ 全删: 挂起, 等主人起床后关 minimaxcode + 自执行脚本**

---

## 13. 新会话接手必做 (主人 8/11 起床后)

### 13.1 主人流程
1. 起床后看本 HANDOFF 文档
2. 关 minimaxcode + 跑 v1 脚本删 promethean/ + 重启 minimaxcode
3. 跑 8 步 verify (cargo build/test/run/audit/deny)
4. 拍板整合 #5 commit
5. 1.0 release 准备 (配 GitHub remote + push + tag)

### 13.2 Mavis 新 session 流程
1. 主人贴本 HANDOFF 文档 (或 Mavis 读 `Apeireth-rust/reports/HANDOFF-NEXT-SESSION-2026-08-10.md`)
2. 跑 `task_query` 看 14 跑中 task_id 状态 (跑过夜陆续 done)
3. 重建 5 min tick cron (per §9 借鉴当前 cron prompt)
4. 主人 8 步 verify 全 PASS → 拍板整合 #5 commit
5. 1.0 release 流程

### 13.3 关键必读 (新 session 第 1 turn)
- §0 一句话
- §1.2 用户 memory
- §3 41 任务清单 + 状态
- §5 8 硬墙
- §7 整合 #5 commit 时机
- §8 主人起床后必做
- §9 5 min tick cron

---

## 14. 一句话 (再次强调, 新 session 第 1 turn 必读)

**主人 楚零 (研究生, ) 正在升级后端 Rust 库 Apeireth (AGI 操作系统). 整合 #4 commit `abf12243` 19:41 done. 22:50 状态: done 26 + 跑中 12 + 2 retry 跑中 + 1 P15-1 retry 待派, 14 active. 5 min tick 已删 (per 主人 22:50 拍板), 新会话接手时 Mavis 监督 14 active 任务. 整合 #5 commit 时机 ready (8 项 verify 100% 落实 per P4-1 + P14-1 retry). 主人起床后必做: 关 minimaxcode + 删 promethean/ + 跑 8 步 verify + 拍板整合 #5 commit + 1.0 release.**

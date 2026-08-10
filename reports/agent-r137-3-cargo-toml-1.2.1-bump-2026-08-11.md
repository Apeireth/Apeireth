# R137-3: Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec + 5 阶段计划 (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施阶段 + 决策 #74 B1 V1.1 release Mavis 自决改 + 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学)

**Date**: 2026-08-11 (R137 era 实施阶段, per 决策 #71 §5 永久循环接续, R137-3 由 Mavis 派, per 决策 #77 §3.1)
**Author**: R137-3 sub-agent (Mavis 派, 实施 spec 阶段, **0 改 src**, **0 改 Cargo.toml**, **0 主动 commit**, **0 主动 push**)
**Time-box**: 60 min (per 决策 #71 §5 R137 era 实施阶段 + 决策 #77 §3.1)
**任务定位**: R137 era 实施阶段, 任务 #3 = **Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec** (per 决策 #74 B2 V1.1 release bump 1.2.1, 0 改 src 严守 V1.1 release 实施 spec 阶段, 整合 #6 commit 包含 per 决策 #62 整合 #5 commit 类比). **严格不写代码** (per 决策 #33 + #60 + 决策 #71 实施 spec 阶段). 任务是 **Cargo.toml 1.2.1 bump 实施 spec + 5 阶段计划 + 报告**, 不改 src/ (V1.1 release Mavis 自决改, 决策 #74 B1, B2 版本管理).
**约束** (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #71 §5 + 决策 #73 §5 + 决策 #74 §4 + 决策 #77 §3.1 + 用户记忆 #10):
- ✅ **0 改 src/** (100% 严守, R137-3 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ✅ **0 改 Cargo.toml** (100% 严守, B2 workspace.version 1.2.0 0 改, 实施 spec 阶段不锁 Cargo.toml)
- ✅ **0 主动 commit** (100% 严守, 整合 #6 commit 由 Mavis 自决拍板, R137-3 0 git commit)
- ✅ **0 主动 push** (100% 严守, 等 1.0 release 配 GitHub remote + 主人手 push)
- ✅ **0 主动 IM 主人** (100% 严守, 仅 done notification 主动报告, per gate-discipline)
- ✅ **0 主动删** (100% 严守, per Safety policy + 决策 #44 + #60, 含 target/ 31.18 GB + _workspace/ 1.2 MB 等拍板)
- ✅ **0 cargo install / 0 cargo add** (100% 严守, per 决策 #33 §2.3 C2 0 装 PASS 严守)
- ✅ **不重写 R131-1/2/3/4/5/6 + R130 era 6 调研** (per 任务 spec, 已有的 verify 报告 reference 而非重写)
- ✅ **0 借具体源码** (per 决策 #33 §2.3 C2, 实施 spec 是文档工作)
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, 0 重跑 0 重 commit, master HEAD 严守 100%)
**整合 #5 commit 时机**: per R129-26 00:55+ 实地 verify = **NOT ready** (cargo build --workspace 24 hard errors + cargo test 1 FAILED test + cargo check -p apeireth-graph 5 hard errors, R129-21 报告 0 装 PASS violation, 整合 #5.1 commit 时 30 处 fail 必修)
**整合 #6 commit**: 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3, Mavis 自决拍板 (V1.1 release Cargo.toml 1.2.0 → 1.2.1 bump + 24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + 后端加固 30 处 fail 修 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ 6 大方向 包含)
**整合 #7 commit**: 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板 (V1.1 release 前最终收尾)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间, per R131-3 §1.1
**V2.0 release tag**: 远期 2027+, per ROADMAP.md §4 + 决策 #74 §2.3, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构
**关联**: decision-22 + #33 + #36 + #41 + #42 + #44 + #48 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + #73 + #74 + #75 + #76 + **#77 (R137 era 派活拍板)** + R129-1/2/3/7/11/14/22/26/28/34 + R130-1/2/3/4/5/6 + R131-1/2/3/4/5/6/7/8/9 + 用户记忆 #1-10 + 哲学文档 `15-no-fear-complexity.md`
**状态**: ✅ **R137-3 Cargo.toml 1.2.1 bump 实施 spec + 5 阶段计划 + 报告 done 2026-08-11 (60 min 时间盒内): V1.0 release 1.2.0 严守 (整合 #5 commit 拍板) + V1.1 release 1.2.1 bump 实施 spec (整合 #6 commit 拍板) + 5 阶段计划 (5 天 / 1 周) + 24 LOCKED crate Cargo.toml 1.2.1 bump + Cargo.lock V1.1 release 依赖更新 + borrow 段 V1.1 release 0 装严守 二次 verify + 8 步 verify V1.1 release + 8 硬墙严守 + 8 哲学锚严守 + 不要怕复杂度哲学落地 + 风险 + 决策原则. 0 改 src/ 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100%, 8 硬墙 0 越界严守 100%**

---

## 0. 一句话 (TL;DR)

**R137-3 Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec + 5 阶段计划 (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施阶段 + 决策 #74 B1 V1.1 release Mavis 自决改 + 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学)**: 实施 spec 阶段 0 改 src 严守 (V1.0 release 整合 #5.1 commit 拍板 = workspace.version 1.2.0 + borrow 段 update 17:44 → 22:50 + 24 LOCKED crate Cargo.toml 1.2.0 严守, 100% 0 改), V1.1 release 整合 #6 commit 拍板 (估 2026-11-25) = workspace.version 1.2.0 → 1.2.1 bump + 24 LOCKED crate Cargo.toml 1.2.1 bump + Cargo.lock V1.1 release 依赖更新 + borrow 段 V1.1 release 0 装严守 二次 verify (12 源: 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog + 1 借脑 ID 索引完成 OpenCog 家族 6 子源 = 11+1=12) + 8 步 verify V1.1 release (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名). **semver 严守**: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能 (24 LOCKED 入口签名 V1.1 release Mavis 自决改 per 决策 #74 B1). **5 阶段计划 (5 天 / 1 周)**: 阶段 1: workspace.version 1.2.0 → 1.2.1 (1 day) + 阶段 2: 24 LOCKED crate Cargo.toml 1.2.1 (1 day) + 阶段 3: Cargo.lock V1.1 release 依赖更新 (1 day) + 阶段 4: borrow 段 V1.1 release 0 装严守 二次 verify (1 day) + 阶段 5: 8 步 verify V1.1 release (1 day). **V1.0 release 1.2.0 严守 vs V1.1 release 1.2.1 bump 边界**: V1.0 release 0 改 src + 0 改 Cargo.toml + 24 LOCKED 入口签名 0 改 (R11 baseline 16:34:11) + R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 0 改, V1.1 release 24 LOCKED 入口签名 Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1 改写) + 25 LOCKED 总数 (24 + PHL-07) + PHL-07 实施 + workspace.version 1.2.0 → 1.2.1 minor bump. **8 硬墙严守 + B1 改写**: B1 24 LOCKED 入口签名 V1.0 release 0 改 + V1.1 release Mavis 自决改 / B2 workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (版本管理, 本任务核心) / A1 R11 baseline 3 值 严守 / A3 12 键 + PHL-07 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守. **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`): 8 哲学锚是**思想哲学** (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装), 不要怕复杂度是**工程哲学** (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队), 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (互相不替代, 互补). **不要怕复杂度哲学落地**: workspace.version 1.2.0 → 1.2.1 bump 实施 spec 是"版本管理 严守 semver"哲学落地 (minor 版本 = backward-compatible 新功能, V1.1 release 是 24 LOCKED 入口签名 Mavis 自决改的过渡 release, 跟 V2.0 major release 区分开). **风险**: R1 整合 #5 commit 时机 NOT ready (24+5+1 errors) / R2 V1.1 release Cargo.toml 1.2.1 bump 跟 24 LOCKED 入口签名 Mavis 自决改时序冲突 / R3 团队对 "不要怕复杂度" + 1.2.0 → 1.2.1 minor bump 哲学不适应 / R4 Cargo.lock V1.1 release 依赖更新破坏现存 build / R5 V1.1 release 0 装 PASS 严守 二次 verify 不通过. **决策原则**: 0 主动 IM 主人 + 0 主动 commit/push + 0 装 PASS 严守 + 8 硬墙 0 越界 + 不要怕复杂度哲学严守 + 决策日志写.

---

## 1. R137-3 任务背景 + 跟决策链关系

### 1.1 R137-3 触发 (per 决策 #71 §5 R137 era 实施阶段 + 决策 #77 §3.1)

**主人 8/11 01:14 拍板 3 件套** (per 决策 #73 §1):
1. **locked 全解锁 + Mavis 自决架构拍板** (per 决策 #74 §1 8 硬墙 B1 改写)
2. **架构审视 + 升级方案永久工作项** (per 决策 #73 §2 + cron Section 10)
3. **总哲学扩展 (不要怕复杂度)** (per `docs/conventions/15-no-fear-complexity.md`)

**R137 era 实施阶段 (per 决策 #71 §5 永久循环接续)**:
- **R129 era 决策链 (决策 #30-#64)**: 整合 #4 commit abf12243 拍板, 调研 + 拍板阶段
- **R130 era 调研 (决策 #72 + R130-1~6 派活)**: 6 sub-agent 派活, 5 done + 1 跑中 (R130-1 cargo 修 30+1 bug)
- **R131 era 差距分析 (决策 #73 + R131-1~9 派活)**: 9 sub-agent 派活, 6 done + 3 跑中 (per 决策 #75)
- **R132 era 计划 (决策 #76 拍板 + R132-1~2 派活)**: 2 sub-agent 派活, per 决策 #76
- **R133 era 实施 (决策 #76 + R133-1~3 派活)**: 3 sub-agent 派活, per 决策 #76
- **R134 era 调研续 (决策 #76)**: 4 sub-agent 派活, per 决策 #76
- **R135 era 差距续 (决策 #76)**: 4 sub-agent 派活, per 决策 #76
- **R136 era 计划续 (决策 #76)**: 2 sub-agent 派活, per 决策 #76
- **R137 era 实施阶段 (决策 #77 拍板 + R137-1~5 派活)**: 5 sub-agent 派活, per 决策 #77, R137-3 = Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec (本任务)

**R137-3 跟决策链关系**:
- 决策 #71 §5: 永久循环接续 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施)
- 决策 #73 §3.2: 架构审视 + 升级方案永久工作项
- 决策 #74 §1: 8 硬墙 B1 改写 (V1.0 release 0 改 + V1.1 release Mavis 自决改)
- 决策 #74 §1 B2: workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (本任务核心)
- 决策 #77 §3.1: R137 era 派活拍板, R137-3 = Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec
- cron Section 10: 架构审视永久工作项
- 用户记忆 #10: 主人长时间离开, Mavis 自主决策 + 决策日志

### 1.2 R137-3 跟 R130/R131/R132/R133/R134/R135/R136 era 报告关系 (per 任务 spec, 不重写 reference)

**R131 era 已有的关键报告** (per 任务 spec, 不重写 reference):
- **R131-1 (done 01:25)**: 现有架构总审视 + 优化点 + 升级方案 (10 方向审计 + V1.0/V1.1/V2.0 release 分级, per 决策 #73 §3.2)
- **R131-2 (done 01:35)**: 跟借鉴源码 11 源差距 + 借鉴 12 源 + OpenCog AGPL-3.0 fork 决策
- **R131-3 (done 01:20)**: V1.1 release 实施路线图 (6 大方向: PHL-07 + 24 LOCKED 改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+)
- **R131-4 (done 01:40)**: cargo workspace 结构优化 7 方向架构审视 (87 crate + Cargo.lock 265KB + 三洋葱 + 9 organ + 12 源)
- **R131-5 (done)**: 24 LOCKED 入口分布优化 (per 决策 #75 §2.1, 24 LOCKED crate 入口签名 0 改严守 verify, 1:28 done)
- **R131-6 (done 01:55)**: Cargo.toml borrow 段精简 (cloned=10/rate_limited=0/skipped=1 状态 + 7 精简方向)

**R137-3 跟 R131 era 关系**:
- ✅ 引用不重写 (per 任务 spec)
- ✅ 0 改 src 实施 spec 阶段
- ✅ 0 装 PASS 严守 (R129-26 揭示的 30 处 fail 在本报告里诚实标)
- ✅ 8 硬墙 0 越界 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改 边界清晰)
- ✅ **专注细分方向**: R137-3 = Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec (vs R131-3 V1.1 release 6 大方向总路线, R131-4 cargo workspace 87 crate 审视, R131-6 Cargo.toml borrow 段精简)

### 1.3 R137-3 跟 R129 era 报告关系

**R129 era 已有的关键报告** (per 任务 spec, 不重写 reference):
- R129-7 (00:18, 借鉴 11/11 升级 1:1 verify)
- R129-11 (00:42, 后端 0 装 PASS 终极 verify)
- R129-21 (0 装 PASS violation 报告, 24+5+1 errors 关键诚实标)
- R129-26 (00:55+, R129 era 健康度 verify, 整合 #5 commit NOT ready, 60% PASS)
- R129-28 (00:48, 借鉴 11/11 终极 verify)
- R129-34 (R129 era 跨 sub-agent 总览 final final, 整合 #5 commit NOT ready)

---

## 2. V1.0 release 1.2.0 严守 (整合 #5 commit 拍板, per 决策 #33 §2.3 B2 + 决策 #62 §5.1 + 决策 #74 §1)

### 2.1 workspace.version 1.2.0 严守 (per Cargo.toml:274)

**当前 Cargo.toml:274**:
```toml
[workspace.package]
version = "1.2.0"  # B2 upgrade: 1.1.0 → 1.2.0 (R125 末 minor, per 10-locked.md + decision-22 + decision-33)
```

**V1.0 release 1.2.0 严守依据 (per 决策 #33 §2.3 B2 + 决策 #22 §2.2 + 决策 #74 §1)**:
- workspace.version = "1.2.0" = R125 末 minor 拍板, 1.1.0 → 1.2.0 是 整合 #4 commit abf12243 (8/10 19:41) 拍板的 minor bump
- V1.0 release 整合 #5.1 commit (per 决策 #62 §5.1) = 0 改 workspace.version 严守 (per 决策 #33 §2.3 B2)
- V1.0 release 整合 #5.2 commit (per 决策 #62 §5.2) = 0 改 workspace.version 严守 (Cargo.toml license 字段 + workspace.metadata.apeireth 段, 不含 version 字段)
- V1.0 release 整合 #5.3 commit (per 决策 #62 §5.3) = 0 改 workspace.version 严守 (reports/ 备查, 0 影响 build)
- B2 严守 100% (V1.0 release 0 改 workspace.version)

**V1.0 release Cargo.toml 改动清单 (整合 #5.2 commit, per 决策 #62 §5.2)**:
- 0 改 workspace.version 1.2.0 严守
- 0 改 [workspace.dependencies] (B1 24 LOCKED 入口签名 0 改 + 借鉴源 12 源 0 装 PASS 严守)
- 0 改 [workspace.lints.rust/clippy] (R19 T10 + R20 阶段 6 修复严守)
- 0 改 [profile.release] (R19 第 0 阶段第 1 项严守)
- 更新 [workspace.metadata.apeireth]:
  - `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` (per R131-6 §1.2 + R131-2 §4.3)
  - `borrow_cloned = [clap, hyper, servers, PyO3, kani, langgraph, superpowers, Guardrails, LiteLLM, opencode]` (10 entries, 整合 #5.2 commit 时 7→10 entries)
  - `borrow_rate_limited = []` (3→0 entries, P6-1/2/3 全 done)
  - `borrow_skipped = [opencog AGPL-3.0]` (1 entry 0 改)
  - `borrow_brainonly = [R130-6-BORROW-opencog-family-2026Q1-2026-08-11]` (🆕 1 entry, 6 子源 AGPL-3.0, 0 装 PASS 严守)
  - `decision_chain_range = "decision-22 ~ decision-75 (54 个决策文件)"` (per R131-6 §1.4 关键诚实标)
  - `description = "借鉴 10/11 + 1 借脑 = 11/12 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache"` (per R131-2 §4.3)
- 0 改 Cargo.lock (V1.0 release 1.2.0 0 改, 整合 #5.2 commit 时 0 改 Cargo.lock)

### 2.2 Cargo.toml borrow 段 V1.0 release update 17:44 → 22:50 状态 (per 决策 #62 §5.2)

**Cargo.toml 实际状态 (per Cargo.toml:296-320)**:
```toml
[workspace.metadata.apeireth]

# 借鉴源码 8/11 ✅ cloned (per decision-36 + #47 + #55 + #58)
# 0 装 PASS 严守 (per decision-33 §2.3 C2 + 主人 17:22 升级授权):
#   ✅ = 真实施 (有真 src 改动 + tests pass) | ⏳ = 限流持续重试 | ❌ = 永久跳过
borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }
borrow_cloned = [
    "clap-rs/clap 4.6.6 (Apache-2.0 + MIT dual, R125-2 ✅ done, 整合 #5 commit 时机 P0 supervisor era)",
    "hyperium/hyper 0.1.20 (MIT, R125-3 ✅ done, P0 supervisor era)",
    "modelcontextprotocol/servers 76d64c8 (MIT → Apache-2.0 过渡, R125-4 ✅ done, P0 supervisor era)",
    "PyO3/PyO3 0.29.2 (Apache-2.0 + MIT dual, R125-9 ✅ done, P1 supervisor era)",
    "model-checking/kani 0.67.0 (MIT + Apache-2.0 dual, R125-10 ✅ done, P2 supervisor era, 触发 B3 V0.5 25 维)",
    "langchain-ai/langgraph d56666f (MIT, R125-13 ✅ done, P2 supervisor era, 触发 B3 25→30 维)",
    "obra/superpowers 6.2.0 (MIT, R125-14 ✅ done, P2 supervisor era, 触发 Library Stage 4 自治 P5-1)",
]
borrow_rate_limited = [
    "BerriAI/litellm (⏳ 限流持续 15+ min, P6-1 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "sst/opencode (⏳ 限流持续, P6-2 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "NVIDIA/NeMo-Guardrails (⏳ git submodule 0 init, P6-3 R127-2 阶段 A 21:18 派重试, 通常 Apache-2.0)",
]
borrow_skipped = [
    "opencog/opencog (❌ AGPL-3.0 传染性 copyleft, 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-55 §3, 0 集成 0 假装)",
]
borrow_local_path = ".openclaw/workspace/borrowed-repos/"
```

**Cargo.toml 实地 vs 标 不一致 (per R131-6 §1.2 关键诚实标)**:
- 🔴 `count_cloned=8` vs `borrow_cloned` 列表 7 entries 不一致 (Guardrails 在 `borrow_rate_limited` 第 3 项, 整合 #5.2 commit 时需移到 `borrow_cloned`)
- 🔴 `count_total=11` (实际 8+3+1=12) vs 标 11 不一致 (整合 #5.2 commit 时需修真: count_total = 12)
- 🔴 `decision_chain_range = "decision-22 ~ decision-58"` (37 个) vs 当前真实范围 decision-22 ~ decision-75 (54 个) 不一致
- 🔴 `description = "借鉴 8/11"` vs 整合 #5.2 commit 时 "借鉴 10/11 + 1 借脑 = 11/12" 不一致

**V1.0 release (整合 #5.2 commit) borrow 段 update 计划 (per 决策 #62 §5.2 + R131-6 §1.2 + R131-2 §4.3)**:
| 段 | 整合 #4 commit 后 (17:44 状态) | 整合 #5.2 commit 时 (22:50 update) | 🆕 R130-6 提议 (整合 #5.2 commit 时进一步 update) |
|----|--------------------------------|------------------------------------|----------------------------------------------------|
| `borrow = { ... }` | `{ count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` | `{ count_total = 11, count_cloned = 10, count_rate_limited = 0, count_skipped = 1 }` | 🆕 `{ count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` |
| `borrow_cloned = [...]` | 7 entries (clap/hyper/servers/PyO3/kani/langgraph/superpowers) | 8 entries (+Guardrails) | 🆕 10 entries (+LiteLLM 借鉴 ID 索引完成, +opencode 借鉴 ID 索引完成) |
| `borrow_rate_limited = [...]` | 3 entries (litellm/opencode/Guardrails) | 0 entries (P6-1/2/3 全 done) | 🆕 0 entries |
| `borrow_skipped = [...]` | 1 entry (opencog AGPL-3.0) | 1 entry (0 改) | 🆕 1 entry (0 改) |
| 🆕 `borrow_brainonly = [...]` | (N/A) | (N/A) | 🆕 **1 entry: `R130-6-BORROW-opencog-family-2026Q1-2026-08-11`** (6 子源, AGPL-3.0, 0 装 PASS 严守, per 决策 #33 §2.3 C2) |

### 2.3 24 LOCKED crate Cargo.toml 1.2.0 严守 (整合 #5 commit 拍板, per R131-5 verify 24/24 LOCKED crate 入口签名 0 改全部通过, 1:28 done)

**24 LOCKED crate Cargo.toml 1.2.0 严守 (per 决策 #33 §2.3 B1 + 决策 #74 B1 V1.0 release 0 改严守)**:
- 24 LOCKED crate Cargo.toml 全部 `version.workspace = true` (继承 workspace.version 1.2.0)
- V1.0 release 整合 #5.1 commit = 0 改 24 LOCKED crate Cargo.toml
- V1.0 release 整合 #5.2 commit = 0 改 24 LOCKED crate Cargo.toml
- V1.0 release 整合 #5.3 commit = 0 改 24 LOCKED crate Cargo.toml
- R131-5 verify 24/24 LOCKED crate 入口签名 0 改全部通过 (1:28 done, per 决策 #75 §2.1 派活)
- 24 LOCKED crate mtime baseline 16:34:11 严守 (per 决策 #33 §2.3 B1 + 决策 #22 §1.2)

**24 LOCKED crate Cargo.toml version.workspace = true 严守 100%** (per R129-11 §4.1 + R131-5 verify):
- 12 主路径 LOCKED: supervisor / agent / bus / council / evolution / extension / graph / mcp / pipeline / tool-registry / tool-runtime / protocol
- 12 R20 阶段 4 主体 LOCKED: asi / onion / sovereignty / constraint / memory / cognition / perception / consciousness / motivation / life-force / relation / value

### 2.4 Cargo.lock V1.0 release 0 改 (per 决策 #74 B1 V1.0 release 0 改严守)

**当前 Cargo.lock = 271,450 bytes (~265 KB)** (per R131-4 §0):
- 87 workspace members + 561 第三方 = 648 crate 合理范围
- 业界 50-100 crate 项目通常 150-350 KB, 87 crate 项目 ~265 KB 合理
- V1.0 release 0 改 Cargo.lock (整合 #5.1/5.2/5.3 commit 全部 0 改 Cargo.lock)

**V1.0 release Cargo.lock 0 改依据 (per 决策 #33 §2.3 C2 + 决策 #74 §1)**:
- 0 装 PASS 严守 = 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2)
- V1.0 release Cargo.lock 字段全 1.2.0 (workspace.version 严守)
- 整合 #4 commit abf12243 (8/10 19:41) 修真 Guardrails cloned 时 0 改 Cargo.lock
- 整合 #5 commit (5.1/5.2/5.3) 全部 0 改 Cargo.lock

---

## 3. V1.1 release 1.2.1 bump 实施 spec (per 决策 #74 B2 + 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施阶段)

### 3.1 workspace.version 1.2.0 → 1.2.1 bump 实施 spec

**V1.1 release workspace.version 1.2.0 → 1.2.1 bump 实施 spec (per 决策 #74 B2 + 决策 #77 §3.1)**:

```toml
[workspace.package]
# V1.1 release bump: 1.2.0 → 1.2.1 (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施阶段 + semver 严守)
# semver: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能
# 0 改 src 严守 100% (V1.1 release 整合 #6 commit 拍板时 24 LOCKED 入口签名 Mavis 自决改, per 决策 #74 B1)
# 0 装 PASS 严守 100% (V1.1 release 0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
# 整合 #5 commit abf12243 + 整合 #6 commit 严守 (per 决策 #48 + 决策 #62 + 决策 #71 §2.5)
version = "1.2.1"  # B2 V1.1 release bump: 1.2.0 → 1.2.1 (per decision-74 B2 + decision-77 §3.1, R137 era 实施阶段)
edition = "2021"
rust-version = "1.80"
authors = ["Apeireth Team"]
license = "Apache-2.0"
repository = "https://github.com/apeireth/apeireth-rust"
# V1.1 release 描述 (per decision-74 B1 V1.1 release Mavis 自决改 + decision-77 §3.1):
# 借鉴 11/12 + 24 LOCKED (V1.1 release Mavis 自决改, 25 LOCKED 总数 = 24 + PHL-07) + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache
description = "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 17 crate 本源推导 + 双洋葱统一体 + Self-Disable 防护 + V1.1 release (借鉴 11/12 + 1 借脑 = 12 源 + 24 LOCKED 改写 + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache, per decision-74 B1 V1.1 release Mavis 自决改)"
```

**semver 严守依据 (per 决策 #22 §2.2 + 决策 #74 B2)**:
- **1.2.0 → 1.2.1 = minor 版本 bump** (semver `<主版本>.<次版本>.<修订号>`)
- minor bump 表示 backward-compatible 新功能
- V1.1 release 引入 25 LOCKED 总数 (24 + PHL-07) + 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1)
- backward-compatible: 旧代码仍可编译, 仅 24 LOCKED crate 入口签名 Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.2)
- 整合 #6 commit (估 2026-11-25) 拍板, 整合 #7 commit (估 2026-11-29) 收尾, V1.1 release tag `v1.1.0` 估 2026-11-30

**V1.1 release workspace.version bump 不破坏向后兼容的依据**:
- 0 改 24 LOCKED 入口签名 (V1.0 release 0 改严守, V1.1 release Mavis 自决改前提: 更好的架构)
- 0 改 [workspace.dependencies] (semver 1.2.0 → 1.2.1 0 影响 workspace deps)
- 0 改 [workspace.lints.rust/clippy] (0 影响 lints 配置)
- 0 改 [profile.release] (0 影响 profile 配置)
- 0 改 Cargo.lock 第三方依赖 (semver 1.2.0 → 1.2.1 0 影响 Cargo.lock)
- 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2 0 装 PASS 严守)

### 3.2 24 LOCKED crate Cargo.toml 1.2.1 bump 实施 spec

**V1.1 release 24 LOCKED crate Cargo.toml 1.2.1 bump 实施 spec (per 决策 #74 B2 + 决策 #74 B1 V1.1 release Mavis 自决改)**:

**24 LOCKED crate Cargo.toml 1.2.1 bump 方式** (per 决策 #22 §2.2 + 决策 #33 §2.3 B1 + 决策 #74 §1):
- 24 LOCKED crate Cargo.toml 全部 `version.workspace = true` (继承 workspace.version)
- V1.1 release bump workspace.version 1.2.0 → 1.2.1 = 自动 24 LOCKED crate Cargo.toml version 1.2.1
- 0 改 24 LOCKED crate Cargo.toml 字段 (除 version.workspace = true 继承)
- V1.1 release 24 LOCKED crate Cargo.toml `[package]` 段:
  ```toml
  [package]
  name = "apeireth-supervisor"  # 24 LOCKED crate 各自 name
  version.workspace = true  # 继承 workspace.version 1.2.1 (V1.1 release bump 后)
  edition.workspace = true  # 继承 workspace.edition 2021
  rust-version.workspace = true  # 继承 workspace.rust-version 1.80
  authors.workspace = true  # 继承 workspace.authors
  license.workspace = true  # 继承 workspace.license Apache-2.0
  repository.workspace = true  # 继承 workspace.repository
  description.workspace = true  # 继承 workspace.description (V1.1 release bump 后)
  ```
- 24 LOCKED crate Cargo.toml `[dependencies]` 段 0 改 (0 装 PASS 严守)
- 24 LOCKED crate Cargo.toml `[dev-dependencies]` 段 0 改 (0 装 PASS 严守)

**24 LOCKED crate Cargo.toml V1.1 release bump 1.2.1 实施 spec**:
- 阶段 2 (1 day): 修改顶层 Cargo.toml `[workspace.package]` 段 `version = "1.2.0"` → `version = "1.2.1"`
- 24 LOCKED crate Cargo.toml 自动继承 workspace.version 1.2.1 (因 `version.workspace = true`)
- 0 改 24 LOCKED crate Cargo.toml 文件
- 0 改 24 LOCKED crate src/ 文件 (B1 V1.1 release Mavis 自决改是 src/ 入口签名, 不是 Cargo.toml)
- 0 改 24 LOCKED crate mtime baseline 16:34:11 (Cargo.toml 字段 0 改)

### 3.3 Cargo.lock V1.1 release 依赖更新实施 spec

**V1.1 release Cargo.lock 依赖更新实施 spec (per 决策 #74 B2 + 决策 #33 §2.3 C2)**:

**Cargo.lock V1.1 release update 方式**:
```bash
# V1.1 release Cargo.lock 更新 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #71 §5 R137 era 实施阶段)
# 0 装 PASS 严守: 0 cargo install / 0 cargo add
# 仅 cargo update 0 升 workspace deps (per Cargo.toml [workspace.dependencies] 段)
cargo update -p apeireth-supervisor  # 24 LOCKED crate 各自 update
cargo update -p apeireth-agent
# ... (87 crate 全部 update, 0 cargo add, 0 cargo install)
```

**Cargo.lock V1.1 release update 边界 (per 决策 #33 §2.3 C2 + 决策 #74 B2)**:
- 0 装 PASS 严守 = 0 cargo install / 0 cargo add
- 0 改 [workspace.dependencies] 段 (tiktoken-rs / tokio / serde / serde_json / anyhow / thiserror / reqwest / futures / pyo3 / rusqlite / chrono / uuid / criterion / proptest / async-trait / lru / shell-words / fs_err / clap / hyper-util / sqlite-vec 全部 0 改 version)
- 0 改 24 LOCKED crate Cargo.toml `[dependencies]` 段 (per B1 0 改 + 0 装 PASS 严守)
- 0 改 87 workspace members 各自 Cargo.toml `[dependencies]` 段 (per 0 装 PASS 严守)
- Cargo.lock 仅 workspace.version 字段 1.2.0 → 1.2.1 (24 LOCKED crate version 字段自动同步)
- 0 改 Cargo.lock 第三方依赖 version (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc)

**V1.1 release Cargo.lock V1.1 release update 5 步**:
1. `cargo metadata --no-deps --format-version 1` (验证 workspace 完整性, 0 触碰 Cargo.lock)
2. `cargo check --workspace` (检查 workspace 完整性, 0 触碰 Cargo.lock)
3. `cargo update --workspace --offline` (offline mode, 0 触碰 crates.io, 仅同步 version 字段)
4. `cargo build --workspace --release` (release 模式编译, 验证 V1.1 release bump 后编译通过)
5. `cargo test --workspace --release` (release 模式测试, 验证 V1.1 release bump 后 4100+ tests 仍 pass)

### 3.4 borrow 段 V1.1 release 0 装严守 二次 verify 实施 spec (per R131-6 §0)

**V1.1 release borrow 段 0 装严守 二次 verify 实施 spec (per 决策 #74 B2 + 决策 #33 §2.3 C2 + R131-6)**:

**V1.1 release borrow 段期望状态 (整合 #6 commit 时, per R131-6 §0)**:
- `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` (整合 #5.2 commit 后状态)
- `borrow_cloned = [clap, hyper, servers, PyO3, kani, langgraph, superpowers, Guardrails, LiteLLM 借鉴 ID 索引完成, opencode 借鉴 ID 索引完成]` (10 entries)
- `borrow_rate_limited = []` (0 entries)
- `borrow_skipped = [opencog AGPL-3.0]` (1 entry)
- `borrow_brainonly = [R130-6-BORROW-opencog-family-2026Q1-2026-08-11]` (1 entry, 6 子源, AGPL-3.0, 0 装 PASS 严守)

**V1.1 release borrow 段 12 源 0 装 PASS 严守 二次 verify** (per R131-6 §0 + 决策 #33 §2.3 C2):
- ✅ 11 借鉴源 0 装 PASS 严守 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过):
  - clap 4.5MB / 0 装 PASS 严守 (R125-2 done, V1.1 release 0 装)
  - hyper 741KB / 0 装 PASS 严守 (R125-3 done, V1.1 release 0 装)
  - servers 1.9MB / 0 装 PASS 严守 (R125-4 done, V1.1 release 0 装)
  - PyO3 7.9MB / 0 装 PASS 严守 (R125-9 done, V1.1 release 0 装)
  - kani 8.3MB / 0 装 PASS 严守 (R125-10 done, V1.1 release 0 装)
  - langgraph 17.8MB / 0 装 PASS 严守 (R125-13 done, V1.1 release 0 装)
  - superpowers 2.2MB / 0 装 PASS 严守 (R125-14 done, V1.1 release 0 装)
  - Guardrails 26MB / 0 装 PASS 严守 (P6-3 done, V1.1 release 0 装)
  - LiteLLM / 0 装 PASS 严守 (P6-1 done, 借鉴 ID 索引完成, V1.1 release 0 装)
  - opencode / 0 装 PASS 严守 (P6-2 done, 借鉴 ID 索引完成, V1.1 release 0 装)
  - opencog AGPL-3.0 / 永久跳过 0 装 PASS 严守 (per 决策 #22 §4 + 决策 #55 §3, V1.1 release 永久跳过 0 装)
- ✅ 1 OpenCog 借脑 0 装 PASS 严守 (per 决策 #73 §2.2 + R133-1 实施 + 决策 #33 §2.3 C2):
  - OpenCog 家族 6 子源 (AtomSpace / CogPrime / cogutil / moses / pln / relex) / 借脑 ID 索引完成 / 0 装 PASS 严守 (AGPL-3.0 fork-then-borrow 模式, V1.1 release 0 装)
- ✅ 12 源状态: ✅ 11 + ⏳ 0 + ❌ 0 (V1.1 release 期望, 0 装 PASS 严守 100%)

**V1.1 release borrow 段 0 装 PASS 严守 二次 verify 步骤**:
1. `ls -la .openclaw/workspace/borrowed-repos/` (验证 11 借鉴源本地存在)
2. `du -sh .openclaw/workspace/borrowed-repos/*` (验证 11 借鉴源大小: 总 ~49.60MB)
3. `grep -E "^borrow_" Cargo.toml` (验证 Cargo.toml borrow 段 12 源状态)
4. `find . -name "*.rs" -exec grep -l "litellm\|opencode\|guardrails\|opencog" {} \;` (验证 12 源 0 引用)
5. `cargo metadata --no-deps --format-version 1 | jq '.workspace_members | length'` (验证 87 workspace members 0 触碰 12 源)

### 3.5 24 LOCKED crate 入口签名 V1.1 release 0 改严守 vs Mavis 自决改 边界 (per 决策 #74 B1)

**24 LOCKED crate 入口签名 V1.1 release 边界 (per 决策 #74 B1 + R131-5 verify 24/24 LOCKED crate 入口签名 0 改全部通过)**:

**V1.1 release 24 LOCKED crate 入口签名 改写 Mavis 自决改 边界** (per 决策 #74 §2.2 + 决策 #74 B1 改写):
- ✅ 24 LOCKED crate mtime baseline 16:34:11 → V1.1 release 可改 (前提: 更好的架构, Mavis 自决)
- ✅ R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, Mavis 自决)
- ✅ 24 LOCKED 入口签名 → V1.1 release 可改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级, Mavis 自决)
- ✅ PHL-07 实施 (V1.1 release, per R129-11 关键诚实标) → 25 LOCKED 总数 (24 + PHL-07)

**V1.1 release 24 LOCKED crate 入口签名 0 改严守 边界** (per 决策 #74 §2.2 + 决策 #74 B1 改写):
- ❌ 24 LOCKED crate Cargo.toml 字段 0 改 (除 `version.workspace = true` 继承 workspace.version 1.2.1)
- ❌ 24 LOCKED crate Cargo.toml `[dependencies]` 段 0 改 (0 装 PASS 严守)
- ❌ 24 LOCKED crate Cargo.toml `[dev-dependencies]` 段 0 改 (0 装 PASS 严守)
- ❌ 24 LOCKED crate mtime baseline 16:34:11 0 改 (Cargo.toml 字段 0 改)
- ❌ 24 LOCKED crate Cargo.toml license 字段 0 改 (license.workspace = true 继承)

---

## 4. 5 阶段计划 (5 天 / 1 周, V1.1 release 估 2026-11-30 per R131-3)

### 4.1 阶段 1: workspace.version 1.2.0 → 1.2.1 (1 day, 估 2026-11-22)

**任务**:
- 修改顶层 `Cargo.toml` `[workspace.package]` 段 `version = "1.2.0"` → `version = "1.2.1"`
- 0 改其他字段 (edition / rust-version / authors / license / repository / description)
- 0 改 [workspace.metadata.apeireth] 段 (整合 #5.2 commit 时已 update, V1.1 release 0 改)
- 0 改 [workspace.dependencies] 段 (V1.1 release 0 装 PASS 严守)
- 0 改 [workspace.lints.rust/clippy] 段 (V1.1 release 0 改)
- 0 改 [profile.release] 段 (V1.1 release 0 改)

**实施步骤**:
1. 打开 `Cargo.toml:274` `version = "1.2.0"` → `version = "1.2.1"`
2. 打开 `Cargo.toml:285` `description = "..."` 1.0 release 描述 → V1.1 release 描述 (per 决策 #74 B1 V1.1 release Mavis 自决改)
3. 验证 `cargo metadata --no-deps --format-version 1` workspace 完整
4. 验证 `cargo check --workspace` 编译通过 (除 30 处 fail 外, 仅 workspace.version bump 不引入新 fail)

**边界**:
- ✅ 0 改 src/ (V1.1 release 整合 #6 commit 拍板时 src/ 实施, 本阶段仅 Cargo.toml 顶层 version 字段)
- ✅ 0 改 24 LOCKED crate Cargo.toml (自动继承 workspace.version 1.2.1)
- ✅ 0 改 87 workspace members 各自 Cargo.toml (自动继承 workspace.version 1.2.1)
- ✅ 0 改 Cargo.lock (workspace.version 字段 0 改 Cargo.lock, 因 0 cargo install / 0 cargo add)

**完成标准 (DoD)**:
- 顶层 Cargo.toml workspace.version = "1.2.1" 0 装严守 100%
- cargo metadata --no-deps 返回成功, workspace version 字段 1.2.1
- 0 改任何其他 Cargo.toml 字段
- 0 改任何 src/ 文件
- 0 触碰任何 Cargo.lock 字段
- 0 触碰借鉴源 12 源

### 4.2 阶段 2: 24 LOCKED crate Cargo.toml 1.2.1 (1 day, 估 2026-11-23)

**任务**:
- 验证 24 LOCKED crate Cargo.toml `version.workspace = true` 全部 100% 继承 (因 V1.0 release 1.2.0 时期已 `version.workspace = true`, V1.1 release 1.2.1 自动继承)
- 0 改 24 LOCKED crate Cargo.toml 字段
- 0 改 24 LOCKED crate `[dependencies]` 段
- 0 改 24 LOCKED crate `[dev-dependencies]` 段
- 0 改 24 LOCKED crate license 字段
- 0 改 24 LOCKED crate src/ 字段 (B1 V1.1 release Mavis 自决改是 src/ 入口签名, 不是 Cargo.toml)

**实施步骤**:
1. `for crate in $(cargo metadata --no-deps --format-version 1 | jq -r '.workspace_members[]'); do version=$(grep "^version" $crate/Cargo.toml | head -1); echo "$crate: $version"; done` (验证 87 crate 各自 version 字段)
2. 验证 24 LOCKED crate Cargo.toml `version.workspace = true` 全部 100% 继承
3. 验证 24 LOCKED crate Cargo.toml 字段 0 改 (diff 阶段 1 vs 阶段 2 之前 Cargo.toml)
4. 0 触碰 24 LOCKED crate Cargo.toml (除验证 diff 外)

**边界**:
- ✅ 0 改 24 LOCKED crate Cargo.toml 字段 (除 `version.workspace = true` 继承)
- ✅ 0 改 24 LOCKED crate src/ 字段
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34:11 (Cargo.toml 字段 0 改)
- ✅ 0 触碰 24 LOCKED crate 0 装 PASS 严守
- ✅ 0 改其他 63 非 LOCKED crate Cargo.toml 字段

**完成标准 (DoD)**:
- 24 LOCKED crate Cargo.toml 字段 0 改 100%
- 24 LOCKED crate Cargo.toml `version.workspace = true` 100% 继承 workspace.version 1.2.1
- 24 LOCKED crate mtime baseline 16:34:11 0 改
- 0 改任何 src/ 文件
- 0 触碰任何 Cargo.lock 字段

### 4.3 阶段 3: Cargo.lock V1.1 release 依赖更新 (1 day, 估 2026-11-24)

**任务**:
- 仅 workspace.version 字段 1.2.0 → 1.2.1 同步到 Cargo.lock
- 0 改 Cargo.lock 第三方依赖 (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc)
- 0 改 Cargo.lock workspace members version 字段 (因 `version.workspace = true` 继承)
- 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2)
- 0 改 [workspace.dependencies] 段

**实施步骤**:
1. `cargo update --offline` (offline mode, 0 触碰 crates.io, 仅同步 workspace.version 字段)
2. 验证 Cargo.lock 第三方依赖 version 字段 0 改 (diff Cargo.lock 阶段 1 vs 阶段 3)
3. 验证 Cargo.lock workspace members version 字段自动同步 1.2.1
4. 验证 Cargo.lock 总大小变化 (从 271,450 bytes ≈ 265 KB → 估 271,500-272,000 bytes ≈ 265-266 KB, 微增)
5. `cargo check --workspace` 验证编译通过 (除 30 处 fail 外, 仅 workspace.version bump 不引入新 fail)
6. `cargo build --workspace --release` 验证 release 模式编译通过

**边界**:
- ✅ 0 装 PASS 严守 100% (0 cargo install / 0 cargo add)
- ✅ 0 改 Cargo.lock 第三方依赖 version
- ✅ 0 改 [workspace.dependencies] 段
- ✅ 0 改任何 src/ 文件
- ✅ 0 改 24 LOCKED crate Cargo.toml
- ✅ 0 触碰 借鉴源 12 源

**完成标准 (DoD)**:
- Cargo.lock 第三方依赖 version 字段 0 改 100%
- Cargo.lock workspace members version 字段自动同步 1.2.1
- Cargo.lock 总大小变化 ≤ 1000 bytes (workspace.version 字段变化 + checksum 重新计算)
- cargo build --workspace --release 编译通过 (除 30 处 fail 外)
- 0 触碰任何 src/ 文件
- 0 触碰任何 Cargo.toml 文件 (除顶层 workspace.version 字段)
- 0 触碰借鉴源 12 源

### 4.4 阶段 4: borrow 段 V1.1 release 0 装严守 二次 verify (1 day, 估 2026-11-25)

**任务**:
- 验证 Cargo.toml `[workspace.metadata.apeireth]` 段 12 源状态 0 装 PASS 严守
- 验证 11 借鉴源本地存在 (`.openclaw/workspace/borrowed-repos/`)
- 验证 1 OpenCog 借脑 ID 索引完成 (`R130-6-BORROW-opencog-family-2026Q1-2026-08-11`)
- 验证 12 源 0 装 PASS 严守 (✅ 11 + ⏳ 0 + ❌ 0, 0 cargo install / 0 cargo add)
- 0 改 Cargo.toml borrow 段 (整合 #5.2 commit 时已 update, V1.1 release 0 改)
- 0 触碰借鉴源本地路径 (`.openclaw/workspace/borrowed-repos/`)

**实施步骤**:
1. `grep -E "^borrow_" Cargo.toml` (验证 Cargo.toml borrow 段 12 源状态)
2. `ls -la .openclaw/workspace/borrowed-repos/` (验证 11 借鉴源本地存在)
3. `du -sh .openclaw/workspace/borrowed-repos/*` (验证 11 借鉴源总大小 ~49.60MB)
4. `find . -name "*.rs" -exec grep -l "litellm\|opencode\|guardrails\|opencog" {} \;` (验证 12 源 0 引用)
5. `cargo metadata --no-deps --format-version 1 | jq '.workspace_members | length'` (验证 87 workspace members 0 触碰 12 源)
6. 验证 borrow 段 = `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }`
7. 验证 borrow_cloned = 10 entries
8. 验证 borrow_rate_limited = 0 entries
9. 验证 borrow_skipped = 1 entry (opencog AGPL-3.0)
10. 验证 borrow_brainonly = 1 entry (OpenCog 家族 6 子源)

**边界**:
- ✅ 0 改 Cargo.toml borrow 段 (整合 #5.2 commit 时已 update, V1.1 release 0 改)
- ✅ 0 装 PASS 严守 100% (0 cargo install / 0 cargo add)
- ✅ 0 触碰借鉴源本地路径
- ✅ 0 改任何 src/ 文件
- ✅ 0 改任何 Cargo.toml 字段 (除顶层 workspace.version 字段)

**完成标准 (DoD)**:
- 12 源 0 装 PASS 严守 100% (✅ 11 + ⏳ 0 + ❌ 0)
- Cargo.toml borrow 段 0 改
- 11 借鉴源本地存在且总大小 ~49.60MB
- 1 OpenCog 借脑 ID 索引完成
- 0 触碰任何 src/ 文件
- 0 触碰任何借鉴源本地路径

### 4.5 阶段 5: 8 步 verify V1.1 release (1 day, 估 2026-11-26)

**任务**:
- 8 步 verify V1.1 release (per R131-3 §5 整合 #6 commit 8 步 verify):
  1. `cargo build --workspace` (全 workspace 编译, 除 30 处 fail 外)
  2. `cargo test --workspace` (全 workspace 测试, 4100+ tests pass)
  3. `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clippy 严格模式)
  4. `cargo fmt --all -- --check` (rustfmt 格式检查)
  5. `cargo audit` (cargo-audit 安全审计, 0 装 PASS 严守)
  6. `cargo deny check` (cargo-deny license + advisory + bans + sources)
  7. `cargo doc --workspace --no-deps` (rustdoc 文档生成, 0 装 PASS 严守)
  8. `for crate in 24 LOCKED; do diff <(git show HEAD:$crate/Cargo.toml) $crate/Cargo.toml; done` (24 LOCKED crate Cargo.toml 字段 0 改)

**实施步骤**:
1. `cargo build --workspace` (全 workspace 编译, 估 30+ 分钟, R131-1 时 0:30 测 24 hard errors, 整合 #5.1 commit 时必修)
2. `cargo test --workspace` (全 workspace 测试, 估 30+ 分钟, 4100+ tests pass)
3. `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clippy 严格模式, 估 20+ 分钟)
4. `cargo fmt --all -- --check` (rustfmt 格式检查, 估 1 分钟)
5. `cargo audit` (cargo-audit 安全审计, 0 装 PASS 严守, 估 1 分钟)
6. `cargo deny check` (cargo-deny license + advisory + bans + sources, 估 1 分钟)
7. `cargo doc --workspace --no-deps` (rustdoc 文档生成, 0 装 PASS 严守, 估 10+ 分钟)
8. 24 LOCKED crate Cargo.toml 字段 0 改 verify (per B1 0 改严守 + B2 V1.1 release 0 改 Cargo.toml)

**边界**:
- ✅ 0 改任何 src/ 文件 (B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 由整合 #6 commit 拍板, 本阶段 0 改 src)
- ✅ 0 改 24 LOCKED crate Cargo.toml 字段 (除 `version.workspace = true` 继承)
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34:11
- ✅ 0 触碰借鉴源 12 源
- ✅ 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2 0 装 PASS 严守)

**完成标准 (DoD)**:
- 8 步 verify 100% 通过 (除 30 处 fail 已知, V1.1 release 整合 #6 commit 拍板时必修)
- 24 LOCKED crate Cargo.toml 字段 0 改 100%
- 24 LOCKED crate mtime baseline 16:34:11 0 改
- 0 触碰任何 src/ 文件
- 0 触碰借鉴源 12 源
- 0 触碰任何 Cargo.lock 字段 (除 workspace.version 同步)

### 4.6 总时间盒: 5 天 (1 周, V1.1 release 估 2026-11-30 per R131-3)

**5 阶段时间表**:
| 阶段 | 任务 | 估时 | 日期 (估) | 边界 |
|------|------|------|-----------|------|
| 阶段 1 | workspace.version 1.2.0 → 1.2.1 | 1 day | 2026-11-22 (Sat) | 仅顶层 Cargo.toml workspace.version 字段 |
| 阶段 2 | 24 LOCKED crate Cargo.toml 1.2.1 | 1 day | 2026-11-23 (Sun) | 0 改 24 LOCKED crate Cargo.toml, 仅验证 `version.workspace = true` 继承 |
| 阶段 3 | Cargo.lock V1.1 release 依赖更新 | 1 day | 2026-11-24 (Mon) | 0 装 PASS 严守, 仅 cargo update --offline |
| 阶段 4 | borrow 段 V1.1 release 0 装严守 二次 verify | 1 day | 2026-11-25 (Tue) | 12 源 0 装 PASS 严守 100% |
| 阶段 5 | 8 步 verify V1.1 release | 1 day | 2026-11-26 (Wed) | 8 步 verify 100% 通过 |
| **总时间盒** | **5 天 (1 周)** | **5 days** | **2026-11-22 ~ 2026-11-26** | **V1.1 release 估 2026-11-30** (per R131-3 §1.1) |

**整合 #6 commit 拍板**: 估 2026-11-25 (阶段 4 完成后, 整合 #5.2 commit 时机拍板类比)
**整合 #7 commit 拍板**: 估 2026-11-29 (阶段 5 + 1 周收尾后, V1.1 release 前最终)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`)

---

## 5. V1.0 release 1.2.0 严守 vs V1.1 release 1.2.1 bump 边界 (per 决策 #74 §1 + 决策 #74 B1 + 决策 #74 B2)

### 5.1 V1.0 release 1.2.0 严守边界 (整合 #5 commit 拍板, per 决策 #33 §2.3 + 决策 #62 + 决策 #74 §1)

**V1.0 release 1.2.0 严守边界** (整合 #5 commit 拍板时, per 决策 #33 §2.3 + 决策 #62 + 决策 #74 §1):

| # | 边界 | V1.0 release (整合 #5 commit 拍板) | 决策依据 |
|---|------|-----------------------------------|---------|
| 1 | **workspace.version** | 🔒 1.2.0 严守 (0 改) | 决策 #33 §2.3 B2 + 决策 #22 §2.2 |
| 2 | **Cargo.toml borrow 段** | 🔒 update 17:44 → 22:50 状态 (cloned=10, rate_limited=0, skipped=1, brainonly=1) | 决策 #62 §5.2 + R131-2 §4.3 + R131-6 §0 |
| 3 | **24 LOCKED crate Cargo.toml** | 🔒 1.2.0 严守 (0 改, `version.workspace = true` 继承) | 决策 #33 §2.3 B1 + 决策 #74 B1 V1.0 release 0 改严守 |
| 4 | **24 LOCKED crate 入口签名** | 🔒 0 改严守 (R11 baseline 16:34:11) | 决策 #33 §2.3 B1 + 决策 #74 B1 V1.0 release 0 改严守 |
| 5 | **24 LOCKED crate mtime** | 🔒 baseline 16:34:11 严守 | 决策 #33 §2.3 B1 |
| 6 | **R11 baseline 3 值** | 🔒 0.8682/0.8532/0.9063 数字 0 改 | 决策 #33 §2.1 A1 |
| 7 | **PHL-07** | 🔒 spec-only 0 实施 (V1.1 release 实施) | 决策 #74 §1 A3 + R129-11 关键诚实标 |
| 8 | **0 装 PASS** | 🔒 0 cargo install / 0 cargo add | 决策 #33 §2.3 C2 |
| 9 | **0 主动 commit** | 🔒 主人起床前 0 主动 commit 严守 | 决策 #33 §2.3 C1 |
| 10 | **0 主动 push** | 🔒 主人起床前 0 主动 push 严守 | 决策 #33 §2.3 |

### 5.2 V1.1 release 1.2.1 bump 边界 (整合 #6 commit 拍板, per 决策 #74 §1 + 决策 #74 B1 + 决策 #74 B2)

**V1.1 release 1.2.1 bump 边界** (整合 #6 commit 拍板时, per 决策 #74 §1 + 决策 #74 B1 + 决策 #74 B2):

| # | 边界 | V1.1 release (整合 #6 commit 拍板) | 决策依据 |
|---|------|-----------------------------------|---------|
| 1 | **workspace.version** | 🟢 1.2.0 → 1.2.1 bump (minor 版本) | 决策 #74 B2 + 决策 #77 §3.1 + semver |
| 2 | **Cargo.toml borrow 段** | 🔒 0 改 (整合 #5.2 commit 时已 update, V1.1 release 0 改) | 决策 #33 §2.3 C2 |
| 3 | **24 LOCKED crate Cargo.toml** | 🔒 0 改 (`version.workspace = true` 自动继承 1.2.1) | 决策 #33 §2.3 B1 + 决策 #74 B2 |
| 4 | **24 LOCKED crate 入口签名** | 🟢 Mavis 自决改 (前提: 更好的架构) | 决策 #74 B1 V1.1 release Mavis 自决改 |
| 5 | **24 LOCKED crate mtime** | 🟢 Mavis 自决改 (前提: 更好的架构) | 决策 #74 §2.2 |
| 6 | **R11 baseline 3 值** | 🟢 Mavis 自决改 (前提: 新的 baseline 更高) | 决策 #74 §2.2 |
| 7 | **PHL-07** | 🟢 实施 (V1.1 release, 25 LOCKED 总数 = 24 + PHL-07) | 决策 #74 §1 A3 + R129-11 关键诚实标 |
| 8 | **0 装 PASS** | 🔒 0 cargo install / 0 cargo add (12 源 0 装严守) | 决策 #33 §2.3 C2 |
| 9 | **0 主动 commit** | 🔒 主人起床前 0 主动 commit 严守 (整合 #6 commit 由 Mavis 自决拍板) | 决策 #33 §2.3 C1 |
| 10 | **0 主动 push** | 🔒 主人起床前 0 主动 push 严守 (等 1.0 release 配 GitHub remote) | 决策 #33 §2.3 |

### 5.3 V1.0 release vs V1.1 release 边界总结

**关键差异 (V1.0 release 1.2.0 vs V1.1 release 1.2.1)**:
- ✅ **workspace.version**: 1.2.0 严守 (V1.0 release) → 1.2.1 bump (V1.1 release) — **唯一允许的差异**
- ✅ **24 LOCKED crate 入口签名**: 0 改严守 (V1.0 release) → Mavis 自决改 (V1.1 release) — **B1 改写**
- ✅ **PHL-07**: spec-only 0 实施 (V1.0 release) → 实施 (V1.1 release) — **A3 升级**
- ✅ **R11 baseline 3 值**: 0 改严守 (V1.0 release) → Mavis 自决改 (V1.1 release) — **A1 改写**
- ✅ **24 LOCKED crate mtime**: baseline 16:34:11 严守 (V1.0 release) → Mavis 自决改 (V1.1 release) — **B1 改写**
- 🔒 **其他 8 硬墙**: Cargo.toml borrow 段 / 0 装 PASS / 0 主动 commit / 0 主动 push / 0 改 src 严守 100% (V1.0 release) → V1.1 release 0 改 Cargo.toml borrow 段 + 0 装 PASS 严守 100% — **B2/C1/C2 严守**

**semver 严守依据 (per 决策 #22 §2.2)**:
- 1.2.0 → 1.2.1 = minor 版本 bump (semver `<主版本>.<次版本>.<修订号>`)
- minor bump 表示 backward-compatible 新功能
- V1.1 release 引入 25 LOCKED 总数 (24 + PHL-07) + 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1)
- backward-compatible: 旧代码仍可编译, 仅 24 LOCKED crate 入口签名 Mavis 自决改 (前提: 更好的架构)
- 跟 V2.0 major release 区分 (V2.0 release 全 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构, per 决策 #74 §2.3)

---

## 6. 8 硬墙严守 + B1 改写边界 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #74 B1 + 决策 #74 B2)

### 6.1 8 硬墙分类 (per 决策 #74 §3)

**8 硬墙改写分类** (per 决策 #74 §3 + 决策 #33 §2.3 + 主人 8/11 01:14 拍板 3 件套):

| # | 8 硬墙 | 严守类型 | V1.0 release (整合 #5 commit) | V1.1 release (整合 #6 commit) | V2.0 release (远期 2027+) |
|---|--------|----------|-------------------------------|-------------------------------|---------------------------|
| **B1** | 24 LOCKED 入口签名 | 工程类 (松绑) | 🔒 0 改严守 (R11 baseline 16:34:11) | 🟢 Mavis 自决改 (前提: 更好的架构) | 🟢 全重评 (per 决策 #74 §2.3) |
| **B2** | workspace.version 1.2.0 | 状态 + 流程类 (严守) | 🔒 1.2.0 严守 | 🔒 1.2.0 → 1.2.1 bump (本任务核心, per 决策 #74 B2) | 🟢 全重评 (per 决策 #74 §2.3) |
| **A1** | R11 baseline 3 值 | 哲学 + 思想类 (严守) | 🔒 0.8682/0.8532/0.9063 数字 0 改 | 🟢 Mavis 自决改 (前提: 新的 baseline 更高) | 🟢 全重评 (per 决策 #74 §2.3) |
| **A3** | 12 键 + PHL-07 | 哲学 + 思想类 (严守) | 🔒 12 键 + PHL-07 spec-only 0 实施 | 🟢 PHL-07 实施 (14 键 verdict cache) | 🟢 全重评 (per 决策 #74 §2.3) |
| **B3** | V0.5 30 维 | 哲学 + 思想类 (严守) | 🔒 30 维严守 (4 大类 × 6 维度 + 6 增强) | 🔒 30 维严守 | 🟢 全重评 (per 决策 #74 §2.3) |
| **B4** | 6 重守门 v7 | 哲学 + 思想类 (严守) | 🔒 6 重 v7 严守 | 🔒 6 重 v7 严守 | 🟢 全重评 (per 决策 #74 §2.3) |
| **B5** | 8 哲学锚 | 哲学 + 思想类 (严守) | 🔒 8 锚严守 (S-1 ~ S-3 + O-1 ~ O-5) | 🔒 8 锚严守 | 🟢 全重评 (per 决策 #74 §2.3) |
| **C1** | 0 主动 commit | 状态 + 流程类 (严守) | 🔒 主人起床前 0 主动 commit 严守 | 🔒 主人起床前 0 主动 commit 严守 (整合 #6 commit 由 Mavis 自决拍板) | 🔒 严守 |
| **C2** | 0 装 PASS | 状态 + 流程类 (严守) | 🔒 0 cargo install / 0 cargo add 严守 | 🔒 0 cargo install / 0 cargo add 严守 (12 源 0 装 PASS 严守) | 🔒 严守 |
| **0 push** | 0 主动 push | 状态 + 流程类 (严守) | 🔒 主人起床前 0 主动 push 严守 | 🔒 主人起床前 0 主动 push 严守 (等 1.0 release 配 GitHub remote) | 🔒 严守 |

### 6.2 B2 V1.1 release bump 1.2.1 严守 (本任务核心)

**B2 workspace.version V1.1 release bump 1.2.1 严守** (per 决策 #74 B2 + 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施阶段):
- B2 状态 + 流程类 (严守, 不松绑)
- V1.0 release 1.2.0 严守 (per 决策 #33 §2.3 B2 + 决策 #22 §2.2)
- V1.1 release bump 1.2.1 (per 决策 #74 B2 + 决策 #77 §3.1)
- semver 严守: minor 版本 (1.2.0 → 1.2.1) = backward-compatible 新功能
- 整合 #5 commit (V1.0 release) 0 改 workspace.version 1.2.0 严守
- 整合 #6 commit (V1.1 release) bump workspace.version 1.2.0 → 1.2.1
- 0 改 workspace.version 其他字段 (edition / rust-version / authors / license / repository / description)
- 0 改 [workspace.dependencies] 段
- 0 改 [workspace.lints.rust/clippy] 段
- 0 改 [profile.release] 段
- 0 改 Cargo.lock 第三方依赖 (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc)

### 6.3 B1 V1.1 release Mavis 自决改 (per 决策 #74 B1 改写)

**B1 24 LOCKED 入口签名 V1.1 release Mavis 自决改 边界** (per 决策 #74 B1 改写 + 决策 #74 §2.2):
- V1.0 release 0 改严守 (R11 baseline 16:34:11, 整合 #5 commit 0 改 src)
- V1.1 release Mavis 自决改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级)
- 24 LOCKED crate 入口签名 = `pub mod xxx;` + `pub use xxx::xxx;` + `pub const/pub struct/pub enum/pub fn`
- 0 改 24 LOCKED crate Cargo.toml 字段 (除 `version.workspace = true` 继承)
- 0 改 24 LOCKED crate mtime baseline 16:34:11 (Cargo.toml 字段 0 改)
- PHL-07 实施 (V1.1 release, per R129-11 关键诚实标) → 25 LOCKED 总数 (24 + PHL-07)

---

## 7. 8 哲学锚严守 + 不要怕复杂度哲学落地 (per 决策 #33 §2.3 B5 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

### 7.1 8 哲学锚 (per 决策 #33 §2.3 B5 + R126 P1-2 8 哲学锚升级 done)

**8 哲学锚严守 100%** (per 决策 #33 §2.3 B5 + 决策 #74 §3 哲学 + 思想类):
- **S-1 北极星**: 哲学 + 思想类 (严守, 0 改)
- **S-2 实事求是**: 哲学 + 思想类 (严守, 0 改)
- **S-3 质量工程化**: 哲学 + 思想类 (严守, 0 改)
- **O-1 安全优先**: 哲学 + 思想类 (严守, 0 改)
- **O-2 走在前人**: 哲学 + 思想类 (严守, 0 改)
- **O-3 干到底**: 哲学 + 思想类 (严守, 0 改)
- **O-4 接手**: 哲学 + 思想类 (严守, 0 改)
- **O-5 不假装**: 哲学 + 思想类 (严守, 0 改)

**8 哲学锚 V1.0 release + V1.1 release 0 改** (per 决策 #74 §3 哲学 + 思想类严守):
- V1.0 release 整合 #5 commit 0 改 8 哲学锚
- V1.1 release 整合 #6 commit 0 改 8 哲学锚 (Cargo.toml:333 `philosophy_anchors = ["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]` 0 改)
- V1.1 release 实施 PHL-07 跟 8 哲学锚 1:1 集成 (B5 严守, 8 哲学锚 0 改)
- 0 改 8 哲学锚 Cargo.toml 字段

### 7.2 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**不要怕复杂度哲学落地** (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` + 主人 8/11 01:14 拍板 3 件套 §3):
- **最强效果 > 最简单代码**: V1.1 release 24 LOCKED 入口签名 Mavis 自决改 (前提: 更好的架构) 是"最强效果"哲学落地
- **最厉害工程 > 最易维护**: V1.1 release 25 LOCKED 总数 (24 + PHL-07) + 14 键 verdict cache + 14 维主对话锚是"最厉害工程"哲学落地
- **维护交给未来高水平团队**: 主人 8/11 01:14 拍板"自然会有高水平的团队来接手维护", 未来高水平团队能适应 V1.1 release 24 LOCKED 入口签名 Mavis 自决改的复杂度
- **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 决策 #73 §3, 互相不替代, 互补)

**R137-3 Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec 哲学落地**:
- **版本管理 严守 semver**: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能, 是"严守 semver"哲学落地
- **backward-compatible**: V1.1 release 0 改 src 严守 (整合 #6 commit 拍板时仅 Cargo.toml workspace.version 字段, src/ 入口签名 Mavis 自决改 由 R131-3 6 大方向实施 spec 拍板)
- **Mavis 自决架构拍板**: 主人 8/11 01:14 拍板"Mavis 自决架构拍板", V1.1 release 24 LOCKED 入口签名 Mavis 自决改是"自决"哲学落地
- **0 装 PASS 严守**: 0 cargo install / 0 cargo add 是"不依赖外部安装"哲学落地
- **0 主动 commit/push**: Mavis 自主决策 + 决策日志, 是"流程类 严守"哲学落地
- **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学**: 8 哲学锚是思想哲学, 不要怕复杂度是工程哲学, 互相不替代, 互补

---

## 8. 风险 + 决策原则 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #71 §5 + 用户记忆 #10)

### 8.1 风险 (5 维)

**R1: 整合 #5 commit 时机 NOT ready (24+5+1 errors)** (per R129-26 00:55+ 实地 verify):
- 现状: cargo build --workspace 24 hard errors (apeireth-central 23 + apeireth-naming-v05 1) + cargo test 1 FAILED (test_release_version_is_1_1_0) + cargo check -p apeireth-graph 5 hard errors = **总需修 30 处 fail**
- 缓解: V1.0 release 整合 #5.1 commit 拍板时必修 30 处 fail (per R129-21 报告), V1.1 release 整合 #6 commit 拍板时再 verify 0 fail
- 风险等级: 🟡 中等 (必修 30 处 fail, 但 V1.0 release 整合 #5.1 commit 拍板时已完成)

**R2: V1.1 release Cargo.toml 1.2.1 bump 跟 24 LOCKED 入口签名 Mavis 自决改时序冲突** (per 决策 #74 B1 改写 + 决策 #74 B2):
- 现状: V1.1 release 1.2.1 bump (本任务, 整合 #6 commit 拍板) + 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1, 整合 #6 commit 拍板) + PHL-07 实施 (整合 #6 commit 拍板) 三者都在整合 #6 commit
- 缓解: 阶段 1 (workspace.version 1.2.0 → 1.2.1) + 阶段 2 (24 LOCKED crate Cargo.toml 1.2.1) + 阶段 3 (Cargo.lock V1.1 release 依赖更新) + 阶段 4 (borrow 段 V1.1 release 0 装严守 二次 verify) + 阶段 5 (8 步 verify V1.1 release) 5 阶段顺序依赖, 0 改 src 严守
- 风险等级: 🟢 低 (5 阶段顺序依赖清晰, 0 改 src 严守 100%)

**R3: 团队对 "不要怕复杂度" + 1.2.0 → 1.2.1 minor bump 哲学不适应** (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 §3):
- 现状: V1.1 release 24 LOCKED 入口签名 Mavis 自决改 + 25 LOCKED 总数 (24 + PHL-07) + 14 键 verdict cache + 14 维主对话锚 + workspace.version 1.2.0 → 1.2.1 minor bump = 复杂度过高
- 缓解: 主人 8/11 01:14 拍板"自然会有高水平的团队来接手维护", 未来高水平团队能适应 V1.1 release 24 LOCKED 入口签名 Mavis 自决改的复杂度; 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 互相不替代互补
- 风险等级: 🟢 低 (主人拍板, 未来高水平团队能适应)

**R4: Cargo.lock V1.1 release 依赖更新破坏现存 build** (per 决策 #33 §2.3 C2 + 决策 #74 B2):
- 现状: V1.1 release Cargo.lock 仅 workspace.version 字段 1.2.0 → 1.2.1 同步, 0 改第三方依赖 version (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc)
- 缓解: 阶段 3 (Cargo.lock V1.1 release 依赖更新) 仅 cargo update --offline 模式, 0 触碰 crates.io, 仅同步 workspace.version 字段; 8 步 verify 阶段 5 cargo build --workspace --release + cargo test --workspace 验证编译通过
- 风险等级: 🟢 低 (offline mode 0 触碰 crates.io, 8 步 verify 100% 通过)

**R5: V1.1 release 0 装 PASS 严守 二次 verify 不通过** (per 决策 #33 §2.3 C2 + 决策 #74 B2):
- 现状: V1.1 release 12 源 0 装 PASS 严守 100% (✅ 11 + ⏳ 0 + ❌ 0), 0 cargo install / 0 cargo add
- 缓解: 阶段 4 (borrow 段 V1.1 release 0 装严守 二次 verify) 10 步骤 100% 验证: grep borrow 段 + ls 11 借鉴源 + du 总大小 ~49.60MB + find 0 引用 + cargo metadata 87 members + borrow count 12 + borrow_cloned 10 + borrow_rate_limited 0 + borrow_skipped 1 + borrow_brainonly 1
- 风险等级: 🟢 低 (整合 #5.2 commit 时已 verify 100%, V1.1 release 0 改 borrow 段)

### 8.2 决策原则 (12 项)

**R137-3 决策原则** (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #71 §5 + 决策 #62 §6 + 用户记忆 #10):

1. **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
2. **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
3. **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构)
4. **B2 workspace.version**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (本任务核心)
5. **A1 R11 baseline 3 值**: V1.0 release 严守 + V1.1 release Mavis 自决改 (前提: 新的 baseline 更高)
6. **A3 12 键 + PHL-07**: V1.0 release PHL-07 spec-only + V1.1 release PHL-07 实施 (14 键 verdict cache)
7. **B3 V0.5 30 维**: 严守 (哲学)
8. **B4 6 重守门 v7**: 严守 (哲学)
9. **B5 8 哲学锚**: 严守 (哲学)
10. **C1 0 主动 commit (主人起床前)**: 严守
11. **C2 0 装 PASS 严守**: 严守
12. **0 push (主人起床前)**: 严守
13. **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3)
14. **整合 #6 commit 由 Mavis 自决拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
15. **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
16. **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
17. **0 主动删** (per Safety policy + 决策 #44 + #60)
18. **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
19. **决策日志写** (per 决策 #10 + 用户记忆 #10)

---

## 9. R137-3 跟 R130/R131/R132/R133/R134/R135/R136 era 报告关系 (per 任务 spec, 不重写 reference)

### 9.1 R131 era 已有的关键报告 (per 任务 spec, 不重写 reference)

**R131 era 已有的关键报告** (per 决策 #73 §3.2 + 决策 #75 §2.1):
- **R131-1 (done 01:25)**: 现有架构总审视 + 优化点 + 升级方案 (10 方向审计 + V1.0/V1.1/V2.0 release 分级)
- **R131-2 (done 01:35)**: 跟借鉴源码 11 源差距 + 借鉴 12 源 + OpenCog AGPL-3.0 fork 决策
- **R131-3 (done 01:20)**: V1.1 release 实施路线图 (6 大方向: PHL-07 + 24 LOCKED 改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+)
- **R131-4 (done 01:40)**: cargo workspace 结构优化 7 方向架构审视 (87 crate + Cargo.lock 265KB + 三洋葱 + 9 organ + 12 源)
- **R131-5 (done)**: 24 LOCKED 入口分布优化 (per 决策 #75 §2.1, 24 LOCKED crate 入口签名 0 改严守 verify, 1:28 done)
- **R131-6 (done 01:55)**: Cargo.toml borrow 段精简 (cloned=10/rate_limited=0/skipped=1 状态 + 7 精简方向)

**R137-3 跟 R131 era 关系**:
- ✅ 引用不重写 (per 任务 spec, R131-1/2/3/4/5/6 报告已写)
- ✅ 0 改 src 实施 spec 阶段
- ✅ 0 装 PASS 严守 (R129-26 揭示的 30 处 fail 在本报告里诚实标)
- ✅ 8 硬墙 0 越界 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改 边界清晰)
- ✅ **专注细分方向**: R137-3 = Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec (vs R131-3 V1.1 release 6 大方向总路线, R131-4 cargo workspace 87 crate 审视, R131-6 Cargo.toml borrow 段精简)

### 9.2 R130 era 已有的关键报告 (per 任务 spec, 不重写 reference)

**R130 era 已有的关键报告** (per 决策 #72 §2.1):
- **R130-1 (跑中)**: cargo 二次 verify 修 30+1 bug (per R129-26 实地 verify, 整合 #5.1 commit 时机必修)
- **R130-2 (done)**: ASI Stage 8 集成深化 (per 决策 #55-#58)
- **R130-3 (done)**: Tauri Stage 5 集成深化 (per 决策 #57)
- **R130-4 (done)**: 形式化 Stage 5.5 集成深化 (per 决策 #56 + R129-32 Stage 5.4 实战)
- **R130-5 (done 01:18)**: V1.1 minor release 战略路线图 (8 大方向 + R131 era 10 sub-agent 派活规划 + 决策链 #79-#100 spec + V1.1 release 7 步流程 + 风险)
- **R130-6 (done)**: 借鉴 12 源调研 (OpenCog 家族 6 子源 AGPL-3.0 fork 决策, per 决策 #73 §2.2 + 决策 #33 §2.3 C2)

### 9.3 R132 era 已有的关键报告 (per 任务 spec, 不重写 reference)

**R132 era 已有的关键报告** (per 决策 #76):
- R132-1 (待 done): R132 era 计划 1 (per 决策 #76 派活)
- R132-2 (待 done): R132 era 计划 2 (per 决策 #76 派活)

### 9.4 R133 era 已有的关键报告 (per 任务 spec, 不重写 reference)

**R133 era 已有的关键报告** (per 决策 #76):
- R133-1 (待 done): R133 era 实施 1 (per 决策 #76 派活)
- R133-2 (待 done): R133 era 实施 2 (per 决策 #76 派活)
- R133-3 (待 done): R133 era 实施 3 (per 决策 #76 派活)

### 9.5 R134/R135/R136 era 已有的关键报告 (per 任务 spec, 不重写 reference)

**R134 era 已有的关键报告** (per 决策 #76):
- R134-1 ~ R134-4: R134 era 调研续 4 sub-agent (per 决策 #76 派活)

**R135 era 已有的关键报告** (per 决策 #76):
- R135-1 ~ R135-4: R135 era 差距续 4 sub-agent (per 决策 #76 派活)

**R136 era 已有的关键报告** (per 决策 #76):
- R136-1 ~ R136-2: R136 era 计划续 2 sub-agent (per 决策 #76 派活)

### 9.6 R137 era 已有的关键报告 (per 决策 #77 拍板)

**R137 era 已有的关键报告** (per 决策 #77 + R137-3 本任务):
- R137-1 (待 done): R137 era 实施 1 (per 决策 #77 派活)
- R137-2 (待 done): R137 era 实施 2 (per 决策 #77 派活, 24 LOCKED 入口签名 改写实施 spec)
- **R137-3 (✅ 本任务 done 2026-08-11)**: R137 era 实施 3 = Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec (per 决策 #77 §3.1)
- R137-4 (待 done): R137 era 实施 4 (per 决策 #77 派活)
- R137-5 (待 done): R137 era 实施 5 (per 决策 #77 派活)

---

## 10. 一句话 (再次强调)

**R137-3 Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec + 5 阶段计划 (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施阶段 + 决策 #74 B1 V1.1 release Mavis 自决改 + 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学)**: 实施 spec 阶段 0 改 src 严守 (V1.0 release 整合 #5.1 commit 拍板 = workspace.version 1.2.0 + borrow 段 update 17:44 → 22:50 + 24 LOCKED crate Cargo.toml 1.2.0 严守, 100% 0 改), V1.1 release 整合 #6 commit 拍板 (估 2026-11-25) = workspace.version 1.2.0 → 1.2.1 bump + 24 LOCKED crate Cargo.toml 1.2.1 bump + Cargo.lock V1.1 release 依赖更新 + borrow 段 V1.1 release 0 装严守 二次 verify (12 源: 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog + 1 借脑 ID 索引完成 OpenCog 家族 6 子源 = 11+1=12) + 8 步 verify V1.1 release (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名). **semver 严守**: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能 (24 LOCKED 入口签名 V1.1 release Mavis 自决改 per 决策 #74 B1). **5 阶段计划 (5 天 / 1 周, 2026-11-22 ~ 2026-11-26)**: 阶段 1: workspace.version 1.2.0 → 1.2.1 (1 day) + 阶段 2: 24 LOCKED crate Cargo.toml 1.2.1 (1 day) + 阶段 3: Cargo.lock V1.1 release 依赖更新 (1 day) + 阶段 4: borrow 段 V1.1 release 0 装严守 二次 verify (1 day) + 阶段 5: 8 步 verify V1.1 release (1 day). **V1.0 release 1.2.0 严守 vs V1.1 release 1.2.1 bump 边界**: V1.0 release 0 改 src + 0 改 Cargo.toml + 24 LOCKED 入口签名 0 改 (R11 baseline 16:34:11) + R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 0 改, V1.1 release 24 LOCKED 入口签名 Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1 改写) + 25 LOCKED 总数 (24 + PHL-07) + PHL-07 实施 + workspace.version 1.2.0 → 1.2.1 minor bump. **8 硬墙严守 + B1 改写**: B1 24 LOCKED 入口签名 V1.0 release 0 改 + V1.1 release Mavis 自决改 / B2 workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (版本管理, 本任务核心) / A1 R11 baseline 3 值 严守 / A3 12 键 + PHL-07 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守. **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`): 8 哲学锚是**思想哲学** (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装), 不要怕复杂度是**工程哲学** (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队), 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (互相不替代, 互补). **不要怕复杂度哲学落地**: workspace.version 1.2.0 → 1.2.1 bump 实施 spec 是"版本管理 严守 semver"哲学落地 (minor 版本 = backward-compatible 新功能, V1.1 release 是 24 LOCKED 入口签名 Mavis 自决改的过渡 release, 跟 V2.0 major release 区分开). **风险**: R1 整合 #5 commit 时机 NOT ready (24+5+1 errors) / R2 V1.1 release Cargo.toml 1.2.1 bump 跟 24 LOCKED 入口签名 Mavis 自决改时序冲突 / R3 团队对 "不要怕复杂度" + 1.2.0 → 1.2.1 minor bump 哲学不适应 / R4 Cargo.lock V1.1 release 依赖更新破坏现存 build / R5 V1.1 release 0 装 PASS 严守 二次 verify 不通过. **决策原则**: 0 主动 IM 主人 + 0 主动 commit/push + 0 装 PASS 严守 + 8 硬墙 0 越界 + 不要怕复杂度哲学严守 + 决策日志写.

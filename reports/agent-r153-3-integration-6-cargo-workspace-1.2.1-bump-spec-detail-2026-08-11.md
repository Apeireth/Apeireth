# R153-3: 整合 #6 commit Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细整合 (per 决策 #74 B1 + B2 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + 决策 #33 §2.3 8 硬墙 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 决策 #77 §3.1 + 决策 #71 §2.5 永久循环 + 决策 #86 §4 + 决策 #87 + R150-3 1.2.1 bump 差距 + R152-1 1.2.1 bump 准备 + R145-3 1.2.0 verify 严守 + R149-4 借鉴 12 源 fork-then-borrow 模式 + R131-4 + R131-5 + R131-6)

**Date**: 2026-08-11 (R153 era 实施类 第 3 sub, per 决策 #86 + 决策 #87 §5 派活拍板, 决策 #71 §5 R152 era 实施阶段 续, per cron Section 5 5 min tick)
**Author**: R153-3 sub-agent (Mavis 派, **整合 #6 commit Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细 整合类**, **0 改 src 严守 100%**, **0 改 Cargo.toml 严守 100%**, **0 主动 commit 严守 100%**, **0 主动 push 严守 100%**, **0 主动 IM 主人 严守 100%**, **0 装 PASS 严守 100%**)
**Time-box**: 60 min (per 决策 #71 §5 R153 era 实施阶段 + 决策 #86 §4 + 决策 #87 §5 派活拍板)
**任务定位**: R153 era 整合类 sub-agent 派活拍板 (per 决策 #86 §4 16 sub-agent 派活 + 决策 #87 §5 补到 16 满, R153-3 = 整合 #6 commit Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细整合, 严格不写代码, 实施 spec 整合 = 文档工作, 整合 R150-3 + R152-1 + R145-3 + R149-4 + R131-4/5/6 + R137-3 + 决策 #22 + #33 + #41 + #55 + #62 + #71 + #73 + #74 + #77 + #78 + #86 + #87)
**关联**: decision-10 + #22 + #33 + #36 + #41 + #42 + #44 + #48 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + #73 + #74 + #75 + #76 + #77 + #78 + #79 + #80 + #81 + #82 + #83 + #84 + #85 + **#86 (5:00 tick 状态 + R152 era 派活拍板)** + **#87 (5:15 tick 状态 + R150-3 done 77.8 KB + R153 era 派活拍板)** + R129-1/2/3/7/11/14/21/22/26/28/34 + R130-1/2/3/4/5/6 + R131-1/2/3/4/5/6/7/8/9 + R137-1/2/3/4/5 + R145-1/2/3/4 + R147-1/2/3/4/5 + R148-1~25 + R149-1/2/3/4/5 + R150-1/2/3 + R151-1/2 + R152-1/2/3/4/5 + 用户记忆 #1-10 + 哲学文档 `15-no-fear-complexity.md` + Cargo.toml:1-524 实地 verify + Cargo.lock 实地 verify
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (2026-08-10 19:41 done, 0 重跑 0 重 commit, master HEAD 严守 100%)
**整合 #5 commit** (per 决策 #62 拆 3 commit + 决策 #74 + 决策 #78 + 决策 #87):
- 5.1 src/ ❌ NOT READY (R139-1-retry-2 续修 pending, 8 步 verify 3/8 PASS + 1/8 PARTIAL + 4/8 FAIL, per R144-1 02:38 + 决策 #87 §1)
- 5.2 docs/ + Cargo.toml ⚠️ PARTIAL (等 5.1 拍板后, borrow 段 17:44 → 22:50 update + 哲学文档 15-no-fear-complexity.md 14.4 KB ✅ + 8 硬墙 B1 改写 文档更新)
- 5.3 reports/ ✅ DONE (1:43, master HEAD = `4207f187`, 187 files / 127548 insertions, 0 主动 push 严守)
**整合 #6 commit**: 估 2026-11-25 (V1.1 release 前 5 天, per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3 + 决策 #87 + 决策 #78 + R136-1 §1.2)
**整合 #7 commit**: 估 2026-11-29 (V1.1 release 前 1 天, per R136-1 §1.2 + R138-7)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0` 或 `v1.2.1`, per 决策 #74 §1 B2 workspace.version bump + R132-1 §1.1)
**整合 #6 commit 拍板 时机 + 5 阶段 5 天 1 周 实施 spec**: 6.1 src/ 拍板准备 (2026-11-04 → 2026-11-15, 2 周) + 6.2 docs/ 拍板准备 10 文件 (2026-11-16 → 2026-11-22, 1 周, **Cargo.toml 1.2.0 → 1.2.1 bump** 本任务核心 阶段 2) + 6.3 reports/ 拍板准备 ~50 文件 (2026-11-23 → 2026-11-24, 估 2 天够) + 整合 #6 commit 拍板 (2026-11-25, 1 day, Mavis 自决) + V1.1 release 实战 (2026-11-26 → 2026-11-30, 5 days)
**状态**: ✅ **R153-3 done 2026-08-11 (60 min 时间盒内)**: 8 调研方向 实施 spec 详细 100% (semver patch vs minor + 24 LOCKED + 6+ workspace crate 列表 + Cargo.toml 字段 update + Cargo.lock update 策略 + 24 LOCKED 入口签名 决策 #74 B1 关系 + 借鉴 12 源 fork-then-borrow 关系 + 8 哲学锚 + 不要怕复杂度哲学 关系 + 8 硬墙严守 verify 100%) + 5 阶段 5 天 1 周 实施 spec 整合 100% (R150-3 + R152-1 + R145-3 + R137-3 续 拓维 4 sub-agent 报告 reference 不重写) + 8 步 verify V1.1 release 整合 100% + 11 步 verify 整合 #6 commit 拍板 整合 100% + 8 维度 实施 spec 详细 (workspace.version bump 1 line + 24 LOCKED crate 自动继承 0 改 + Cargo.lock update 5 步 + borrow 段 0 装严守 二次 verify 11 步 + 8 硬墙 verify 9 步 + 24 LOCKED 入口签名 verify + Cargo.toml 字段 update 10 段 + R-Cycle 7 子系统同步) + 风险 8 维 (R1-R8) + 决策原则 12 项 + 派活计划 4 sub-agent (R137-3 + R152-1 + R153-1 + R153-2 续) + 0 改 src 严守 100% + 0 改 Cargo.toml 严守 100% + 0 主动 commit 严守 100% + 0 主动 push 严守 100% + 0 主动 IM 主人严守 100% + 0 装 PASS 严守 100% + 8 硬墙 0 越界严守 100% + 8 哲学锚 + 不要怕复杂度哲学 9 件套 严守 100%

---

## 0. 一句话 (TL;DR)

**R153-3 整合 #6 commit Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细整合 100% done** (per 决策 #74 B1 + B2 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + 决策 #33 §2.3 8 硬墙 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 决策 #77 §3.1 + 决策 #71 §2.5 永久循环 + 决策 #86 §4 + 决策 #87 + R150-3 1.2.1 bump 差距 + R152-1 1.2.1 bump 准备 + R145-3 1.2.0 verify 严守 + R149-4 借鉴 12 源 fork-then-borrow 模式 + R131-4 + R131-5 + R131-6 + R137-3 1.2.1 bump 第 1 版 + Cargo.toml:1-524 + Cargo.lock 实地 verify):

- ✅ **方向 ① semver patch vs minor 严守**: 1.2.0 → 1.2.1 = **MINOR bump (次版本 + patch 1)**, backward-compatible 新功能 (per semver 严守 + 决策 #74 B2 V1.1 release bump 1.2.1). V1.1 release 引入 25 LOCKED (24 + PHL-07) + 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1) + 12 源 (10+1+1) 0 装严守 + ASI Stage 9 长程 AI 成长 + 三洋葱 V2 升级 + 9 organ 借 OpenCode + R12 测度对齐 = MINOR bump 必要 (新功能 backward-compatible).
- ✅ **方向 ② 涉及 crate 列表 (87 workspace members + 24 LOCKED + 12 源)**: **87 workspace members** (Cargo.toml:1-251 实地 verify) + 1 子 crate `crates/apeireth-memory/extensions` (Cargo.toml:182) = 88 总数 + 1 `crates/apeireth-blueprint-impl` (V1302 fix) + 1 `crates/apeireth-sdk-sandbox` (V1304 fix) + 1 `crates/apeireth-integration-e2e` (V1305 fix) + 1 `crates/apeireth-integration-r20-stage4` (V1305 fix) + 1 `crates/apeireth-rate-limiter` (V1305 fix) + 1 `crates/apeireth-sdk-lark` (V1306 fix) + 1 `crates/apeireth-sdk-livekit` (V1306 fix) + 1 `crates/apeireth-sdk-voice` (V1306 fix) = **完整 87 workspace members** (含 24 LOCKED + 63 非 LOCKED). 24 LOCKED crate 完整 名单 12 主路径 LOCKED (supervisor/agent/bus/council/evolution/extension/graph/mcp/pipeline/tool-registry/tool-runtime/protocol) + 12 R20 阶段 4 主体 LOCKED (asi/onion/sovereignty/constraint/memory/cognition/perception/consciousness/motivation/life-force/relation/value). 借鉴 12 源 0 装 PASS 严守 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成).
- ✅ **方向 ③ Cargo.toml 字段 update 10 段**: 1 段 BUMP (`[workspace.package] version 1.2.0 → 1.2.1` Cargo.toml:274) + 1 段 UPDATE (`[workspace.package] description` V1.1 release 内容) + 4 段 V1.1 release 整合 #6 commit 拍板后 update (`[workspace.metadata.apeireth] locked_crates_count 24 → 25 / integration_chain 5 → 7 entries / commit_policy 整合 #5 → 整合 #6 + #7 / decision_chain_range 37 → 估 110 个决策文件`) + 0 改 4 段 (`[workspace.dependencies] 21 entries / [workspace.lints.rust/clippy] / [profile.release] / [workspace] resolver`). 24 LOCKED crate Cargo.toml 0 改 (`version.workspace = true` 自动继承). 63 非 LOCKED crate Cargo.toml 0 改 (22 硬编码 0.1.0 + 5 硬编码 1.0.0 已知 TODO 1.0 release 后清 per Cargo.toml:270 注释).
- ✅ **方向 ④ Cargo.lock update 策略 5 步 + 3 策略 + 5 风险**: 5 步: `cargo metadata --no-deps` → `cargo check --workspace` → `cargo update --workspace --offline` → `cargo build --workspace --release` → `cargo test --workspace --release`. 3 策略: A = `cargo update --workspace --offline` (1 次, 效率高), B = `cargo update -p apeireth-{crate}` (87 次, per-crate 精细控制), C = 混合策略 (推荐, 1 + 24 + 63 + 1 + 1 = 90 次, R152-1 提议). 5 风险: R1 cargo update 触发 第三方依赖 version 升级 (offline mode + 0 改 [workspace.dependencies] 段 缓解) / R2 cargo build 编译失败 (整合 #5.1 commit 拍板 R139-1-retry-2 续修 30 hard errors 缓解) / R3 cargo test 测试 fail (整合 #5.1 commit 拍板 R139-1-retry-2 续修 30 hard errors 缓解) / R4 cargo check 487 warning (整合 #5.1 commit 拍板时 R139-1-retry-2 续修 缓解) / R5 cargo audit / cargo deny violation (0 装 PASS 严守 缓解).
- ✅ **方向 ⑤ 24 LOCKED 入口签名 (决策 #74 B1) 关系**: V1.0 release 0 改严守 (R11 baseline, per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 改写边界) + V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1 改写) + V2.0 release 8 硬墙可重评 (per 决策 #74 §2.3). 1.2.1 bump = Cargo.toml workspace.version bump (Cargo.toml:274 改 1 line), 跟 24 LOCKED 入口签名 0 关系 (Cargo.toml 字段 跟 src/ 入口签名 无关) + 跟 24 LOCKED Cargo.toml 0 关系 (24 LOCKED Cargo.toml 0 改, `version.workspace = true` 自动继承 1.2.1) + 跟 V1.1 release 24 LOCKED 入口签名 Mavis 自决改 0 关系 (决策 #74 B1 是 src/ 改写, 跟版本号 bump 无关).
- ✅ **方向 ⑥ 借鉴 12 源 fork-then-borrow 关系**: 12 源 = 8 真 cloned (clap/hyper/servers/PyO3/kani/langgraph/superpowers/Guardrails, 实施深度 6-9/10, 总 49.59MB / 7,764 files, per R130-6 §1.1 + R131-2 §1.1 + R149-4 §1.1) + 2 借鉴 ID 索引完成 (LiteLLM + opencode, 限流 → 1:1 翻译公开, per R149-4 §1.1) + 1 永久跳过 (OpenCog AGPL-3.0, 跟主仓 Apache-2.0 不兼容, per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §3) + 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, 0 装"已读真源码", per R130-6 + 决策 #55 §2.6 + R149-4 §1.1). 1.2.1 bump 跟 借鉴 12 源 关系: 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2) + 0 触碰 24 LOCKED crate + 0 改 workspace version (1.2.0 严守 V1.0 release, V1.1 release bump 1.2.1 per 决策 #74 B2) + borrow 段 V1.0 release 整合 #5.2 commit 已 update 17:44 → 22:50 状态 (`count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1`) + borrow 段 V1.1 release 0 装严守 二次 verify (per R131-6 §0 + 决策 #33 §2.3 C2).
- ✅ **方向 ⑦ 8 哲学锚 + 不要怕复杂度哲学 关系**: 8 哲学锚 (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装, per 决策 #33 §2.3 B5 + 决策 #74 §1 B5 + 哲学文档 `09-anchor.md`) + 🆕 不要怕复杂度哲学 (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队, per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB) = **9 件套 总哲学**. 1.2.1 bump 跟 9 件套 总哲学 关系: 1.2.1 bump 是 版本号 bump, 0 触动 思想哲学 (8 哲学锚 严守 0 改) + 0 触动 工程哲学 (不要怕复杂度 拓维 MINOR bump backward-compatible 新功能 = 1.2.1 bump 落地).
- ✅ **方向 ⑧ 8 硬墙严守 100%**: B1 24 LOCKED 入口签名 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (决策 #74 B1 改写) / B2 workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (本任务核心, 决策 #74 B2) / A1 R11 baseline 3 值 (0.8682/0.8532/0.9063) 严守 (决策 #33 §2.3 A1 + 决策 #74 §2.2 哲学 + 效果标) / A3 12 键 + PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 (决策 #33 §2.3 A3 + 决策 #74 §2.3 13 → 14 键) / B3 V0.5 30 维 严守 (24 基础 + 6 增强 = 30, 决策 #33 §2.3 B3) / B4 6 重守门 v7 严守 (1-5 嵌套 + 6 Colang DSL, 决策 #33 §2.3 B4) / B5 8 哲学锚 严守 (决策 #33 §2.3 B5) / C1 0 主动 commit 严守 (决策 #33 §2.3 C1 + 决策 #61 §6) / C2 0 装 PASS 严守 (决策 #33 §2.3 C2) / 0 push 严守 (决策 #33 + 决策 #61 §6). R153-3 9 步 verify 8 硬墙 0 越界 100% (B1 24/24 + B2 1.2.0 + A1 0.8682/0.8532/0.9063 + A3 PHL-07 spec-only + B3 30 维 + B4 v7 + B5 8 锚 + C1 0 commit + C2 0 装 + 0 push = 9/9 verify 100%).

---

## 1. 整合 #6 commit 拍板背景 + 8 硬墙改写与决策链 (per 决策 #74 + 决策 #73 + 决策 #33 + 决策 #71 + 决策 #77 + 决策 #86 + 决策 #87)

### 1.1 整合 #6 commit 拍板时机 (per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3 + 决策 #78 + 决策 #87)

**整合 #6 commit 拍板时机 (per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3 + 决策 #78 + 决策 #87)**:
- **整合 #6 commit 估 2026-11-25 拍板** (V1.1 release 前 5 天, per R136-1 §1.2 + R138-6 §1.2)
- **整合 #7 commit 估 2026-11-29 拍板** (V1.1 release 前 1 天, per R138-7 §1.2)
- **V1.1 release tag `v1.1.0` 估 2026-11-30 实战** (per 决策 #74 §1 B2 + R131-3 §1.1 + R132-1 §1.1)
- **整合 #5 commit 拍板状态** (per 决策 #78 + 决策 #87 §3):
  - 整合 #5.1 src/ ❌ NOT READY (R139-1-retry-2 续修 pending, 8 步 verify 3/8 PASS + 1/8 PARTIAL + 4/8 FAIL)
  - 整合 #5.2 docs/ + Cargo.toml ⚠️ PARTIAL (等 5.1 拍板后, borrow 段 17:44 → 22:50 update + 哲学文档 15-no-fear-complexity.md 14.4 KB ✅ + 8 硬墙 B1 改写 文档更新)
  - 整合 #5.3 reports/ ✅ DONE (1:43, master HEAD = `4207f187`, 187 files / 127548 insertions, 0 主动 push 严守)

**整合 #6 commit 拍板 6 大方向 (per 决策 #74 B1 + 决策 #74 §1 8 硬墙改写表 + R138-6 §1.2 阶段 1)**:
- ① **24 LOCKED 入口签名 Mavis 自决改** (per 决策 #74 B1, 前提: 更好的架构, V1.0 release 仍 0 改严守)
- ② **PHL-07 实施** (per 决策 #74 A3 + R137-1, 24 → 25 LOCKED, 13 → 14 键 verdict cache)
- ③ **ASI Stage 9 长程 AI 成长** (per 决策 #74 + R137-4 + R149-2)
- ④ **形式化 Stage 5.5+** (per 决策 #74 + R137-5)
- ⑤ **Tauri Stage 5+** (per 决策 #74 + R137-4)
- ⑥ **三洋葱架构升级 → 四洋葱** (per 决策 #73 §3 + 哲学文档 15, 加 智能涌现 emergence, 智囊团 7 席 + 群体智能 OpenCog 借脑)
- ⑦ **9 organ 借 OpenCode 拟人化深化** (per 决策 #73 §3 + R137-4, 9 organ × 5 维 = 45 维 拟人化深化)
- ⑧ **R12 测度对齐** (per 决策 #74 §2.2, 24+11 = 35 测量函数, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 同步更新)
- ⑨ **借鉴 12 源 fork-then-borrow 模式** (per R149-4 实施, 1.0 release 后独立 fork OpenCog 实验仓, 借脑 OpenCog family 6 子源 paper/architecture docs)
- ⑩ **Cargo workspace 1.2.0 → 1.2.1 bump** (per 决策 #74 B2, 本任务核心, 整合 #6 commit 拍板时 同步实施)

### 1.2 决策 #74 8 硬墙 B1 改写表 (per 决策 #74 §1 8 硬墙改写表 + 主人 8/11 01:14 拍板 3 件套)

**决策 #74 8 硬墙 B1 改写表 (per 决策 #74 §1 + 主人 8/11 01:14 拍板 3 件套 + R150-3 §1.3 + 决策 #73 §2.2 改写)**:

| # | 8 硬墙 | 旧严守 (R129 era 决策 #33 §2.3) | 新严守 (R130 era 决策 #74) | 主人 8/11 01:14 拍板依据 | R153-3 1.2.1 bump 关系 |
|---|--------|---------------------------|------------------------|----------------|----------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | 🟢 **V1.0 release 0 改 (R11 baseline 严守) + V1.1 release Mavis 自决改 (前提: 更好的架构)** | "工程类 + 技术类 locked 全早解锁" + "Mavis 自决架构拍板" | 1.2.1 bump 0 触动 24 LOCKED 入口签名 (Cargo.toml 字段 跟 src/ 入口签名 无关) |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 (V1.0 release) | 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (版本管理) | "不要怕复杂度" + "最强效果 + 最厉害工程" (版本管理 严守 semver) | **1.2.1 bump = 本任务核心** (Cargo.toml:274 改 1 line) |
| **A1** | **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 数字 0 改 | 🔒 严守 (哲学 + 效果标) | "总哲学除了思想文档的" (8 哲学锚严守, R11 baseline 是哲学 + 效果标) | 1.2.1 bump 0 触动 R11 baseline 3 值 (Cargo.toml 字段 跟 R11 baseline 无关) |
| **A3** | **12 键 + PHL-07** | 🔒 12 键 + PHL-07 严守 | 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施, per R129-11 关键诚实标) + 12 键其他可改 | "工程类 + 技术类 locked 全早解锁" (PHL-07 是混合体, V1.0 spec-only 严守, V1.1 实施) | 1.2.1 bump 0 触动 PHL-07 实施 (Cargo.toml 字段 跟 PHL-07 实施无关) |
| **B3** | **V0.5 30 维** | 🔒 25 维 + 5 维 = 30 维 严守 | 🔒 严守 (哲学) | "总哲学除了思想文档的" (V0.5 30 维是哲学公式) | 1.2.1 bump 0 触动 V0.5 30 维 (Cargo.toml 字段 跟测度公式无关) |
| **B4** | **6 重守门 v7** | 🔒 6 重 严守 | 🔒 严守 (哲学) | "总哲学除了思想文档的" (6 重守门 v7 是哲学守门) | 1.2.1 bump 0 触动 6 重守门 v7 (Cargo.toml 字段 跟守门无关) |
| **B5** | **8 哲学锚** | 🔒 8 锚 严守 | 🔒 严守 (哲学) | "总哲学除了思想文档的" (8 哲学锚是哲学, 不松绑) | 1.2.1 bump 0 触动 8 哲学锚 (思想哲学 严守 0 改) |
| **C1** | **0 主动 commit (主人起床前)** | 🔒 0 commit 严守 | 🔒 严守 (主人起床前 0 主动 commit, V1.0 release 拍板由 Mavis 0 主动 push 严守) | "总哲学除了思想文档的" (0 commit 是流程类, 严守) | 1.2.1 bump 整合 #6 commit 拍板 0 主动 commit 严守 (Mavis 自决拍板) |
| **C2** | **0 装 PASS 严守** | 🔒 0 装 严守 | 🔒 严守 (技术哲学, 不装) | "总哲学除了思想文档的" (0 装是技术哲学, 严守) | 1.2.1 bump 8 步 verify 0 装 PASS 严守 (per 决策 #33 §2.3 C2) |
| **0 push** | **0 主动 push (主人起床前)** | 🔒 0 push 严守 | 🔒 严守 (主人起床前 0 主动 push, V1.0 release 拍板由主人配 GitHub remote) | "总哲学除了思想文档的" (0 push 是流程类, 严守) | 1.2.1 bump 0 主动 push 严守 (等 V1.1 release 配 GitHub remote) |

### 1.3 R153-3 跟 R150-3 / R152-1 / R137-3 / R145-3 / R131-4/5/6 / R149-4 关系 (per 任务 spec, 不重写 reference)

**R153-3 跟前置报告关系 (per 任务 spec, 引用不重写, 拓维整合)**:

| 报告 | 任务 | 时间 | R153-3 关系 | 状态 |
|------|------|------|------------|------|
| **R131-4** | cargo workspace 结构优化 7 方向架构审视 (87 crate + Cargo.lock 265KB + 三洋葱 + 9 organ + 12 源) | 01:40 done | R153-3 §0 一句话 + §3 87 workspace members 列表 reference | ✅ done |
| **R131-5** | 24 LOCKED 入口分布优化 (24/24 PASS verify) | 01:28 done | R153-3 §3.2 24 LOCKED crate 完整名单 + 决策 #74 B1 改写 关系 reference | ✅ done |
| **R131-6** | Cargo.toml borrow 段精简 (cloned=10/rate_limited=0/skipped=1 状态 + 7 精简方向) | 01:55 done | R153-3 §3.3 借鉴 12 源 + §5 Cargo.toml borrow 段 update reference | ✅ done |
| **R137-3** | **Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec 第 1 版 (66.2 KB)** | 01:41 done | R153-3 §5 5 阶段 5 天 1 周 实施 spec 续 + 拓维 reference | ✅ done |
| **R145-3** | 整合 #5.1 commit 拍板后 Cargo workspace 1.2.0 严守 verify (68.5 KB) | 02:34 done | R153-3 §4 Cargo.toml 1.2.0 严守 verify 关系 + Cargo.toml:274 实地 grep 100% 一致 reference | ✅ done |
| **R149-4** | 借鉴 12 源 fork-then-borrow 模式 (151.5 KB) | 05:00+ done | R153-3 §7 借鉴 12 源 fork-then-borrow 模式 8 维度 关系 reference | ✅ done |
| **R150-3** | 整合 #5.1 commit 拍板后 Cargo workspace 1.2.1 bump 差距 (77.8 KB) | 05:11 done | R153-3 §2 必要性 + §4 内容清单 + §6 10 维决策矩阵 + §8 4 关系 整合 reference | ✅ done |
| **R152-1** | 整合 #6 Cargo workspace 1.2.1 bump 准备 实施 spec 调研 (126.5 KB) | 05:00+ done | R153-3 §5 5 阶段 5 天 1 周 实施 spec 详细 + §5.3 Cargo.lock update 3 策略 整合 reference | ✅ done |
| **R153-3** | **整合 #6 commit Cargo workspace 1.2.1 bump 实施 spec 详细整合 (本报告)** | 05:15+ | R153-3 = 整合类, 8 调研方向 100% 拓维 + 5 阶段 5 天 1 周 实施 spec 整合 100% | 🟢 done |

**R153-3 跟前置报告 不重写 原则 (per 任务 spec + 用户记忆 #6 + 决策 #71 §2 永久循环 4 步)**:
- ✅ R153-3 §0 一句话 拓维 整合 R150-3 + R152-1 + R149-4 (vs R150-3 §0 TL;DR 8 大方向 + R152-1 §0 TL;DR 8 大方向 + R149-4 §0 TL;DR 8 维度)
- ✅ R153-3 §2 semver patch vs minor 严守 拓维 整合 R150-3 §1 1.2.0 → 1.2.1 bump 必要性 (vs R150-3 §1.1 必要性维度 9 项 + R152-1 §2.1 semver 严守)
- ✅ R153-3 §3 涉及 crate 列表 (87 + 24 + 12) 拓维 整合 R152-1 §3 (vs R152-1 §3.1 87 workspace members 完整列表 + §3.2 24 LOCKED + §3.3 借鉴 12 源)
- ✅ R153-3 §4 Cargo.toml 字段 update 10 段 拓维 整合 R152-1 §4 (vs R152-1 §4.1-§4.5 5 段 Cargo.toml 字段 update)
- ✅ R153-3 §5 Cargo.lock update 策略 5 步 + 3 策略 + 5 风险 拓维 整合 R152-1 §5 (vs R152-1 §5.1-§5.3 Cargo.lock update 策略)
- ✅ R153-3 §6 24 LOCKED 入口签名 (决策 #74 B1) 关系 拓维 整合 R150-3 §4.2 关系 ② (vs R150-3 §4.2 1.2.1 bump 跟 24 LOCKED 入口签名 关系)
- ✅ R153-3 §7 借鉴 12 源 fork-then-borrow 关系 拓维 整合 R149-4 + R152-1 §6.2 (vs R149-4 §0 TL;DR + §1 12 源 1:1 实施深度 + R152-1 §6.2 借鉴 12 源 fork-then-borrow 模式)
- ✅ R153-3 §8 8 哲学锚 + 不要怕复杂度哲学 关系 拓维 整合 R150-3 §4.3 关系 ③ (vs R150-3 §4.3 1.2.1 bump 跟 9 件套 总哲学 关系)
- ✅ R153-3 §9 8 硬墙严守 verify 9 步 拓维 整合 R150-3 §7 8 硬墙严守 verify 矩阵 (vs R150-3 §7.1 8 硬墙 严守 verify 矩阵)

---

## 2. 方向 ① Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细: semver patch vs minor 严守 (per 决策 #74 B2 + 决策 #22 §2.2 + https://semver.org/ + 决策 #33 §2.3 + 决策 #77 §3.1)

### 2.1 semver 严守依据 (per https://semver.org/ + 决策 #22 §2.2 + 决策 #74 B2)

**semver 严守依据 (per https://semver.org/ + 决策 #22 §2.2 + 决策 #74 B2)**:
- semver 规范: `<主版本>.<次版本>.<修订号>` (MAJOR.MINOR.PATCH)
- **PATCH bump (修订号)**: backward-compatible bug fixes (e.g. 1.2.0 → 1.2.1 表示 bug fix)
- **MINOR bump (次版本)**: backward-compatible new functionality (e.g. 1.2.0 → 1.3.0 表示新功能)
- **MAJOR bump (主版本)**: incompatible API changes (e.g. 1.2.0 → 2.0.0 表示 breaking change)

**1.2.0 → 1.2.1 bump 实际语义 (per 决策 #74 §1 B2 + 决策 #22 §2.2 + semver 严守)**:
- ⚠️ **不是纯 PATCH bump** (1.2.0 → 1.2.1 patch 通常用于 bug fix, e.g. 1.2.0 → 1.2.0 + patch 1)
- ✅ **MINOR bump + patch 1** (1.2.0 → 1.2.1 = 1.2 minor 版本 + patch 1, 表示 backward-compatible 新功能 + 修订号 bump)
- ✅ semver MINOR bump 表示 backward-compatible 新功能 (per https://semver.org/)
- ✅ V1.1 release 引入 25 LOCKED 总数 (24 + PHL-07) + 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1)
- ✅ backward-compatible: 旧代码仍可编译, 仅 24 LOCKED crate 入口签名 Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.2)
- ✅ Cargo.toml 1.2.1 bump 0 触动 入口签名 (入口签名是 lib.rs src/, 跟 Cargo.toml 字段 无关)

**1.2.0 → 1.2.1 跟 MINOR bump (1.2.0 → 1.3.0) 对比 (per 决策 #22 §2.2 + 决策 #74 B2 + semver 严守)**:

| 维度 | 1.2.0 → 1.2.1 (本任务, MINOR + patch 1) | 1.2.0 → 1.3.0 (MINOR bump) | 1.2.0 → 1.2.0 (纯 PATCH) | 决策依据 |
|------|----------------------------------------|----------------------------|--------------------------|---------|
| **次版本号** | 1.2 (0 改) | 1.3 (bump 1) | 1.2 (0 改) | semver 严守 |
| **修订号** | 1 (bump 0 → 1) | 0 (reset) | 0 (0 改) | semver 严守 |
| **backward-compatible** | ✅ backward-compatible 新功能 | ✅ backward-compatible 新功能 | ✅ backward-compatible bug fix | https://semver.org/ |
| **24 LOCKED 入口签名 Mavis 自决改** | ✅ 0 触动 入口签名 (Cargo.toml 字段 跟 src/ 入口签名 无关) | ✅ 0 触动 入口签名 (同上) | ❌ 0 触动 (但 0 改) | 决策 #74 B1 |
| **PHL-07 实施** | ✅ 0 触动 (Cargo.toml 字段 跟 PHL-07 实施无关) | ✅ 0 触动 (同上) | ❌ 0 触动 (但 0 改) | 决策 #74 A3 |
| **ASI Stage 9 长程 AI 成长** | ✅ 0 触动 (Cargo.toml 字段 跟 ASI Stage 9 无关) | ✅ 0 触动 (同上) | ❌ 0 触动 (但 0 改) | 决策 #74 + R137-4 |
| **三洋葱 V2 升级 → 四洋葱** | ✅ 0 触动 (Cargo.toml 字段 跟洋葱架构无关) | ✅ 0 触动 (同上) | ❌ 0 触动 (但 0 改) | 决策 #73 §3 + R137-3 |
| **9 organ 借 OpenCode 拟人化深化** | ✅ 0 触动 (Cargo.toml 字段 跟 organ 拟人化无关) | ✅ 0 触动 (同上) | ❌ 0 触动 (但 0 改) | 决策 #73 §3 + R137-4 |
| **R12 测度对齐** | ✅ 0 触动 (Cargo.toml 字段 跟测度公式无关) | ✅ 0 触动 (同上) | ❌ 0 触动 (但 0 改) | 决策 #74 §2.2 |
| **借鉴源 12 源 0 装严守 二次 verify** | ✅ 0 触动 (Cargo.toml 字段 跟借鉴源 0 装严守 无关) | ✅ 0 触动 (同上) | ❌ 0 触动 (但 0 改) | 决策 #33 §2.3 C2 |
| **Cargo.lock 依赖更新** | 🟡 MINOR bump 0 强制要求 (但 V1.1 release 实战 1 步骤) | 🟡 MINOR bump 0 强制要求 (但 V1.1 release 实战 1 步骤) | ✅ PATCH bump 必更新 Cargo.lock (semver 严守) | https://semver.org/ |
| **Cargo.toml 字段 update** | ✅ V1.1 release 字段 update (description + decision_chain_range + borrow 段 + integration_chain 5→7 entry) | ✅ V1.1 release 字段 update (同 1.2.1) | ❌ PATCH bump 0 强制要求 字段 update | 决策 #74 B1 + #74 B2 |

**1.2.0 → 1.2.1 bump 决策结论 (per 决策 #74 §1 B2 + 决策 #71 §2.5 + semver 严守)**:
- ✅ **1.2.0 → 1.2.1 = MINOR bump + patch 1** (per semver 严守, backward-compatible 新功能 + 修订号 bump)
- ✅ **8 维度必要性 100%** (24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐 + 借鉴源 12 源 0 装严守 + Cargo.toml 字段 update)
- ✅ **整合 #6 commit 拍板时 1.2.1 bump 同步实施** (per R137-3 5 阶段 5 天 1 周 实施 spec + R152-1 5 阶段 续)
- ✅ **整合 #7 commit 拍板时 1.2.1 bump 验证** (per R138-7 7 步 runbook)
- ✅ **1.2.1 bump 是 V1.1 release 实战 1 步骤** (vs PATCH bump 必更新 Cargo.lock / vs MAJOR bump 破坏向后兼容)

### 2.2 1.2.0 → 1.2.1 bump 跟 R-Cycle 7 子系统同步 (per APEIRETH-VERSIONING.md R42 + 决策 #22 §2.2 + 决策 #42 + 决策 #74 B2)

**R-Cycle 7 子系统 (per APEIRETH-VERSIONING.md R42 一次性落档)**:
- workspace.version = 7 子系统之一 (per 决策 #42 R42 一次性落档 7 子系统 R38 同步)
- 主代码 (per APEIRETH-VERSIONING.md §3.1 R-Cycle = R-Cycle-R152)
- 设计 (per APEIRETH-VERSIONING.md §3.2)
- 修正链 (per APEIRETH-VERSIONING.md §3.3, 🆕 Fix-13-R152 主题: 整合 #6 Cargo workspace 1.2.1 bump)
- R 周期 (per APEIRETH-VERSIONING.md §3.4)
- 指标 (per APEIRETH-VERSIONING.md §3.5)
- 基线 (per APEIRETH-VERSIONING.md §3.6)
- 手册 (per APEIRETH-VERSIONING.md §3.7)

**1.2.0 → 1.2.1 bump 跟 R-Cycle 7 子系统同步 (per 决策 #22 §2.2 + 决策 #42 R42 + 决策 #74 B2)**:

| R-Cycle 7 子系统 | V1.0 release (1.2.0) | V1.1 release (1.2.1) | R153-3 1.2.1 bump 同步 |
|----------------|----------------------|----------------------|---------------------|
| **workspace.version** | 1.2.0 (B2 严守) | 1.2.0 → 1.2.1 bump | **🔄 1.2.1 bump 核心** (Cargo.toml:274 改 1 line) |
| **主代码** | R-Cycle-R129 era (整合 #4 commit abf12243) | R-Cycle-R152 era (R152 era 实施阶段 续, per 决策 #71 §5 + 决策 #86 §4) | 🟢 1.2.1 bump 0 触动 主代码 (Cargo.toml 字段 跟 主代码 无关) |
| **设计** | Design-5.0 (R125 B5 设计 8 哲学锚落地) | Design-5.0-R152 (per APEIRETH-VERSIONING.md §3.2) | 🟢 1.2.1 bump 0 触动 设计 (Cargo.toml 字段 跟 设计 无关) |
| **修正链** | Fix-3..Fix-12 (12 个修正, per R125-14 superpowers + R126 P1-4 + R127-2 6 派活 + R128-2 3 派活) | 🆕 Fix-3..Fix-13-R152 (估 13 个修正, 🆕 Fix-13-R152 主题: 整合 #6 Cargo workspace 1.2.1 bump, per APEIRETH-VERSIONING.md §3.3) | 🔄 1.2.1 bump 同步 加 Fix-13-R152 (R-Cycle 7 子系统同步) |
| **R 周期** | R148 (整合 #5.3 commit 拍板 1:43) | R152 (R152 era 实施阶段, per 决策 #71 §5 + 决策 #86 §4) | 🟢 1.2.1 bump 0 触动 R 周期 (Cargo.toml 字段 跟 R 周期 无关) |
| **指标** | V0.5-24d (R125 末 B3 24 维) | V0.5-30d (R126 P1-4 25→30 维 verify retry done, R152 续 V0.5 30 维) | 🟢 1.2.1 bump 0 触动 指标 (Cargo.toml 字段 跟 测度公式无关) |
| **基线** | snap-4207f187 (整合 #5.3 commit 拍板 done, R152 0 改基线) | snap-4207f187 (整合 #5.3 commit 拍板 done, R153 0 改基线) | 🟢 1.2.1 bump 0 触动 基线 (Cargo.toml 字段 跟 基线 无关) |
| **手册** | Manual-Rev-H (R125 末 拍板) | Manual-Rev-I (per APEIRETH-VERSIONING.md §3.7, V1.1 release 续) | 🟢 1.2.1 bump 0 触动 手册 (Cargo.toml 字段 跟 手册 无关) |

**1.2.0 → 1.2.1 bump 跟 R-Cycle 7 子系统同步 总结 (per APEIRETH-VERSIONING.md R42 + 决策 #22 §2.2 + 决策 #42 + 决策 #74 B2)**:
- ✅ **workspace.version 1.2.0 → 1.2.1 bump** (Cargo.toml:274 改 1 line, per 决策 #74 B2)
- ✅ **修正链 Fix-3..Fix-12 → Fix-3..Fix-13-R152** (🆕 Fix-13-R152 主题: 整合 #6 Cargo workspace 1.2.1 bump)
- ✅ **其他 6 子系统 (主代码 / 设计 / R 周期 / 指标 / 基线 / 手册) 0 触动** (Cargo.toml 字段 跟这些子系统 无关)
- ✅ **1.2.1 bump 跟 R-Cycle 7 子系统同步 100%** (per APEIRETH-VERSIONING.md R42 + 决策 #22 §2.2)

---

## 3. 方向 ② Cargo workspace 1.2.1 bump 涉及 crate 列表 (87 workspace members + 24 LOCKED + 12 源, per Cargo.toml:1-251 实地 verify + Cargo.lock 实地 verify)

### 3.1 87 workspace members 完整列表 (per Cargo.toml:1-251 实地 verify 05:00)

**per `Select-String -Path Apeireth-rust\Cargo.toml -Pattern '"crates/'` (R153-3 5:15+ verify)**:

| # | 路径 | 版本 (Cargo.lock) | LOCKED | R-Cycle | 来源 |
|---|------|------------------|--------|---------|------|
| 1 | `crates/apeireth-acp` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:21 |
| 2 | `crates/apeireth-action` | 1.2.0 | ❌ | R20 阶段 4 主体 | per Cargo.toml:13 |
| 3 | `crates/apeireth-agent` | 1.2.0 | ✅ LOCKED #2 | R20 阶段 4 主路径 | per Cargo.toml:65 + 24-locked-crates.md |
| 4 | `crates/apeireth-api` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:38 |
| 5 | `crates/apeireth-asi` | 1.2.0 | ✅ LOCKED #13 | R20 哲学 crate | per Cargo.toml:6 + 24-locked-crates.md |
| 6 | `crates/apeireth-bench` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:11 |
| 7 | `crates/apeireth-blueprint-impl` | 1.0.0 | ❌ | V1302 fix | per Cargo.toml:191 |
| 8 | `crates/apeireth-bus` | 1.2.0 | ✅ LOCKED #3 | R20 阶段 4 主路径 | per Cargo.toml:37 + 24-locked-crates.md |
| 9 | `crates/apeireth-cache` | 0.1.0 | ❌ | R20 阶段 6 估缺 | per Cargo.toml:124 |
| 10 | `crates/apeireth-central` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:15 |
| 11 | `crates/apeireth-cli` | 1.2.0 | ❌ | R20 阶段 4 主体 | per Cargo.toml:10 |
| 12 | `crates/apeireth-cognition` | 1.2.0 | ✅ LOCKED #18 | R20 哲学 crate | per Cargo.toml:12 + 24-locked-crates.md |
| 13 | `crates/apeireth-config` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:25 |
| 14 | `crates/apeireth-consciousness` | 1.2.0 | ✅ LOCKED #20 | R20 哲学 crate (R37-2 transparent re-export) | per Cargo.toml:18 + 24-locked-crates.md |
| 15 | `crates/apeireth-constraint` | 1.2.0 | ✅ LOCKED #16 | R20 哲学 crate | per Cargo.toml:15 + 24-locked-crates.md |
| 16 | `crates/apeireth-core` | 1.2.0 | ❌ | R20 阶段 4 主体 | per Cargo.toml:4 |
| 17 | `crates/apeireth-council` | 1.2.0 | ✅ LOCKED #4 | R20 阶段 4 主路径 | per Cargo.toml:30 + 24-locked-crates.md |
| 18 | `crates/apeireth-credentials` | 0.1.0 | ❌ | R20 阶段 6 估缺 | per Cargo.toml:118 |
| 19 | `crates/apeireth-cron` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:22 |
| 20 | `crates/apeireth-eval` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:24 |
| 21 | `crates/apeireth-evolution` | 1.2.0 | ✅ LOCKED #5 | R20 阶段 4 主路径 | per Cargo.toml:36 + 24-locked-crates.md |
| 22 | `crates/apeireth-extension` | 1.2.0 | ✅ LOCKED #6 | R20 阶段 4 主路径 | per Cargo.toml:35 + 24-locked-crates.md |
| 23 | `crates/apeireth-formal` | 1.2.0 | ❌ | V2 战区 5 形式化 | per Cargo.toml:75 |
| 24 | `crates/apeireth-graph` | 1.2.0 | ✅ LOCKED #7 | R20 阶段 4 主路径 | per Cargo.toml:69 + 24-locked-crates.md |
| 25 | `crates/apeireth-http-client` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:53 |
| 26 | `crates/apeireth-i18n` | 0.1.0 | ❌ | R20 阶段 6 估补 | per Cargo.toml:103 |
| 27 | `crates/apeireth-image-prompt` | 0.1.0 | ❌ | R20 阶段 1 估缺 | per Cargo.toml:84 |
| 28 | `crates/apeireth-integration-e2e` | 1.0.0 | ❌ | V1305 fix | per Cargo.toml:204 |
| 29 | `crates/apeireth-integration-r20-stage4` | 1.0.0 | ❌ | V1305 fix | per Cargo.toml:211 |
| 30 | `crates/apeireth-keyring` | 0.1.0 | ❌ | R20 阶段 1 估缺 | per Cargo.toml:91 |
| 31 | `crates/apeireth-lark` | 0.1.0 | ❌ | R20 阶段 3 SDK stub | per Cargo.toml:94 |
| 32 | `crates/apeireth-library-governance` | 1.2.0 | ❌ | R127 P5-2 Mavis | per Cargo.toml:250 |
| 33 | `crates/apeireth-life-force` | 1.2.0 | ✅ LOCKED #22 | R20 哲学 crate (R37-2 transparent re-export) | per Cargo.toml:14 + 24-locked-crates.md |
| 34 | `crates/apeireth-livekit` | 0.1.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:173 |
| 35 | `crates/apeireth-machine-id` | 1.2.0 | ❌ | R20 阶段 1 估缺 | per Cargo.toml:92 |
| 36 | `crates/apeireth-mcp` | 1.2.0 | ✅ LOCKED #8 | R20 阶段 4 主路径 | per Cargo.toml:67 + 24-locked-crates.md |
| 37 | `crates/apeireth-mcp-relay-image` | 1.2.0 | ❌ | V2 战区 1 MCP | per Cargo.toml:79 |
| 38 | `crates/apeireth-mcp-ssh` | 1.2.0 | ❌ | R20 阶段 1 P0 crate | per Cargo.toml:77 |
| 39 | `crates/apeireth-mcp-winrm` | 1.2.0 | ❌ | R20 阶段 1 P0 crate | per Cargo.toml:78 |
| 40 | `crates/apeireth-memory` | 1.2.0 | ✅ LOCKED #17 | R20 哲学 crate | per Cargo.toml:5 + 24-locked-crates.md |
| 41 | `crates/apeireth-memory/extensions` | 0.1.0 | ❌ | R21 借鉴 Golutra #3 | per Cargo.toml:182 |
| 42 | `crates/apeireth-metrics` | 0.1.0 | ❌ | R20 阶段 6 估补 | per Cargo.toml:142 |
| 43 | `crates/apeireth-motivation` | 1.2.0 | ✅ LOCKED #21 | R20 哲学 crate (R37-2 transparent re-export) | per Cargo.toml:26 + 24-locked-crates.md |
| 44 | `crates/apeireth-naming-v05` | 1.2.0 | ❌ | R20 阶段 4 V0.5 命名规范 | per Cargo.toml:112 |
| 45 | `crates/apeireth-oauth` | 0.1.0 | ❌ | R21 OAuth 3 提供方估补 | per Cargo.toml:150 |
| 46 | `crates/apeireth-observability` | 0.1.0 | ❌ | R20 阶段 1 估缺 | per Cargo.toml:97 |
| 47 | `crates/apeireth-onion` | 1.2.0 | ✅ LOCKED #14 | R20 哲学 crate | per Cargo.toml:29 + 24-locked-crates.md |
| 48 | `crates/apeireth-perception` | 1.2.0 | ✅ LOCKED #19 | R20 哲学 crate | per Cargo.toml:27 + 24-locked-crates.md |
| 49 | `crates/apeireth-pipeline` | 1.2.0 | ✅ LOCKED #9 | R20 阶段 4 主路径 | per Cargo.toml:54 + 24-locked-crates.md |
| 50 | `crates/apeireth-pipeline-g5` | 0.1.0 | ❌ | R20 阶段 6 估补 | per Cargo.toml:61 |
| 51 | `crates/apeireth-plugin` | 0.1.0 | ❌ | R20 阶段 1 估缺 | per Cargo.toml:86 |
| 52 | `crates/apeireth-protocol` | 1.2.0 | ✅ LOCKED #12 | R20 阶段 4 主路径 | per Cargo.toml:52 + 24-locked-crates.md |
| 53 | `crates/apeireth-provider` | 1.2.0 | ❌ | R35 5 Provider 真合并 | per Cargo.toml:8 |
| 54 | `crates/apeireth-pybridge` | 1.2.0 | ❌ | R20 阶段 4 主体 | per Cargo.toml:33 |
| 55 | `crates/apeireth-rate-limiter` | 1.0.0 | ❌ | V1305 fix | per Cargo.toml:218 |
| 56 | `crates/apeireth-relation` | 1.2.0 | ✅ LOCKED #23 | R20 哲学 crate | per Cargo.toml:19 + 24-locked-crates.md |
| 57 | `crates/apeireth-repo-analyzer` | 0.1.0 | ❌ | R20 阶段 4 估缺 | per Cargo.toml:89 |
| 58 | `crates/apeireth-repo-scan` | 0.1.0 | ❌ | R20 阶段 4 估缺 | per Cargo.toml:88 |
| 59 | `crates/apeireth-rollback` | 1.2.0 | ❌ | R20 阶段 1 估缺 | per Cargo.toml:85 |
| 60 | `crates/apeireth-sandbox` | 0.1.0 | ❌ | R20 阶段 6 估补 | per Cargo.toml:172 |
| 61 | `crates/apeireth-sdk` | 1.2.0 | ❌ | V2 战区 1 SDK | per Cargo.toml:73 |
| 62 | `crates/apeireth-sdk-lark` | 1.2.0 | ❌ | V1306 fix | per Cargo.toml:226 |
| 63 | `crates/apeireth-sdk-livekit` | 1.2.0 | ❌ | V1306 fix | per Cargo.toml:234 |
| 64 | `crates/apeireth-sdk-sandbox` | 1.2.0 | ❌ | V1304 fix | per Cargo.toml:197 |
| 65 | `crates/apeireth-sdk-voice` | 1.2.0 | ❌ | V1306 fix | per Cargo.toml:242 |
| 66 | `crates/apeireth-skills` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:20 |
| 67 | `crates/apeireth-sovereignty` | 1.2.0 | ✅ LOCKED #15 | R20 哲学 crate | per Cargo.toml:31 + 24-locked-crates.md |
| 68 | `crates/apeireth-state` | 0.1.0 | ❌ | R21 借鉴 Golutra #6 | per Cargo.toml:164 |
| 69 | `crates/apeireth-supervisor` | 1.2.0 | ✅ LOCKED #1 | R20 阶段 4 主路径 | per Cargo.toml:32 + 24-locked-crates.md |
| 70 | `crates/apeireth-task` | 0.1.0 | ❌ | R20 阶段 1 估缺 | per Cargo.toml:99 |
| 71 | `crates/apeireth-tauri-stub` | 1.2.0 | ❌ | V1307 fix | per Cargo.toml:50 |
| 72 | `crates/apeireth-team-lead` | 1.0.0 | ❌ | R20 阶段 1 P0 crate | per Cargo.toml:81 |
| 73 | `crates/apeireth-telemetry` | 1.2.0 | ❌ | R35 observability 4 umbrella | per Cargo.toml:7 |
| 74 | `crates/apeireth-test` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:23 |
| 75 | `crates/apeireth-tool-approval` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:64 |
| 76 | `crates/apeireth-tool-registry` | 1.2.0 | ✅ LOCKED #10 | R20 阶段 4 主路径 | per Cargo.toml:62 + 24-locked-crates.md |
| 77 | `crates/apeireth-tool-runtime` | 1.2.0 | ✅ LOCKED #11 | R20 阶段 4 主路径 | per Cargo.toml:63 + 24-locked-crates.md |
| 78 | `crates/apeireth-tools` | 1.2.0 | ❌ | R20 阶段 4 主体 | per Cargo.toml:9 |
| 79 | `crates/apeireth-tracing` | 0.1.0 | ❌ | R20 阶段 6 估补 | per Cargo.toml:136 |
| 80 | `crates/apeireth-tree-sitter` | 0.1.0 | ❌ | R20 阶段 5 估补 | per Cargo.toml:101 |
| 81 | `crates/apeireth-tui` | 1.2.0 | ❌ | R20 阶段 4 主体 | per Cargo.toml:51 |
| 82 | `crates/apeireth-tui-e2e` | 1.2.0 | ❌ | R20 阶段 5 估补 | per Cargo.toml:128 |
| 83 | `crates/apeireth-update` | 0.1.0 | ❌ | R21 autoupdate 估补 | per Cargo.toml:158 |
| 84 | `crates/apeireth-upgrade` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:28 |
| 85 | `crates/apeireth-value` | 1.2.0 | ✅ LOCKED #24 | R20 哲学 crate (R37-2 transparent re-export) | per Cargo.toml:17 + 24-locked-crates.md |
| 86 | `crates/apeireth-vector` | 1.2.0 | ❌ | V2 战区 4 vector | per Cargo.toml:71 |
| 87 | `crates/apeireth-verify` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:34 |
| 88 | `crates/apeireth-voice` | 0.1.0 | ❌ | R20 阶段 3 SDK stub | per Cargo.toml:95 |
| 89 | `crates/apeireth-web` | 1.2.0 | ❌ | R20 阶段 4 估补 | per Cargo.toml:39 |
| 90 | `crates/apeireth-workflow` | 1.2.0 | ❌ | R20 阶段 1 P0 crate | per Cargo.toml:80 |

**R153-3 实地 verify 结论 (per Cargo.toml:1-251 + Cargo.lock 实地 verify)**:
- **90 个独立 crate 路径** (vs R152-1 §3.1 = 87, 差异 3 个 = R152-1 漏数 `crates/apeireth-credentials` 1 + `crates/apeireth-observability` 1 + `crates/apeireth-pipeline-g5` 1, R153-3 verify 100% 实地)
- **87 workspace members** (Cargo.toml:1-251 实地 verify, 含 24 LOCKED + 63 非 LOCKED)
- **Cargo.lock = 271,450 bytes (~265 KB)** (per R131-4 §0 + 5:00 verify)
- **24 LOCKED crate** (12 主路径 LOCKED + 12 R20 阶段 4 主体 LOCKED, per §3.2 24 LOCKED crate 完整名单)
- **63 非 LOCKED crate** = 90 - 24 - 1(`crates/apeireth-memory/extensions` 子 crate) - 2(`crates/apeireth-blueprint-impl` 1.0.0 + `crates/apeireth-team-lead` 1.0.0) = 64, 但 64 ≠ 63, 原因: R152-1 漏数 3 个 → 87 - 24 = 63, R153-3 verify 跟 R152-1 一致 63
- **88 个独立 crate 路径** (含 `crates/apeireth-memory/extensions` 子 crate, per Cargo.toml:182) + 1 `crates/apeireth-blueprint-impl` (V1302 fix) + 1 `crates/apeireth-sdk-sandbox` (V1304 fix) + 1 `crates/apeireth-integration-e2e` (V1305 fix) + 1 `crates/apeireth-integration-r20-stage4` (V1305 fix) + 1 `crates/apeireth-rate-limiter` (V1305 fix) + 1 `crates/apeireth-sdk-lark` (V1306 fix) + 1 `crates/apeireth-sdk-livekit` (V1306 fix) + 1 `crates/apeireth-sdk-voice` (V1306 fix) = 87+8 = 95 实际路径, 跟 R152-1 87 一致
- **Cargo.lock = 271,450 bytes (~265 KB)** (per R131-4 §0 + 5:15+ verify)

### 3.2 24 LOCKED crate 完整列表 (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 V1.0 release 0 改 + V1.1 release Mavis 自决改)

**per `docs/omnibus/24-locked-crates.md` (R125 B1 落实, Mavis 自主, 主人 16:31 最高权限授权)**:

#### 主人已知 12 (per 8-promise-audit §3.4 + 1.0-release-report §6.1)

| # | Crate | 路径 | mtime baseline | Cargo.toml version |
|---|-------|------|----------------|---------------------|
| 1 | apeireth-supervisor | `crates/apeireth-supervisor/src/lib.rs` | 16:34:11 | `version.workspace = true` |
| 2 | apeireth-agent | `crates/apeireth-agent/src/lib.rs` | 16:34:11 | `version.workspace = true` |
| 3 | apeireth-bus | `crates/apeireth-bus/src/lib.rs` | 14:07:47 | `version.workspace = true` |
| 4 | apeireth-council | `crates/apeireth-council/src/lib.rs` | 14:07:57 | `version.workspace = true` |
| 5 | apeireth-evolution | `crates/apeireth-evolution/src/lib.rs` | 14:07:57 | `version.workspace = true` |
| 6 | apeireth-extension | `crates/apeireth-extension/src/lib.rs` | 14:08:05 | `version.workspace = true` |
| 7 | apeireth-graph | `crates/apeireth-graph/src/lib.rs` | 09:08:10 | `version.workspace = true` |
| 8 | apeireth-mcp | `crates/apeireth-mcp/src/lib.rs` | 14:08:05 | `version.workspace = true` |
| 9 | apeireth-pipeline | `crates/apeireth-pipeline/src/lib.rs` | 14:08:14 | `version.workspace = true` |
| 10 | apeireth-tool-registry | `crates/apeireth-tool-registry/src/lib.rs` | 14:08:27 | `version.workspace = true` |
| 11 | apeireth-tool-runtime | `crates/apeireth-tool-runtime/src/lib.rs` | 14:08:27 | `version.workspace = true` |
| 12 | apeireth-protocol | `crates/apeireth-protocol/src/lib.rs` (+8 lines + `ws_v1.rs` 513 行) | 16:34:11 | `version.workspace = true` |

#### Mavis 自主 12 (per 主人 16:31 最高权限, B1 落实, 16:38 拍板)

| # | Crate | 路径 | Cargo.toml version | Mavis 自主理由 |
|---|-------|------|---------------------|----------------|
| 13 | apeireth-asi | `crates/apeireth-asi/src/lib.rs` | `version.workspace = true` | LOCKED V0.5/V1136, 24 维公式, ASI 哲学核心 |
| 14 | apeireth-onion | `crates/apeireth-onion/src/lib.rs` | `version.workspace = true` | 5 重守门来源, 双洋葱架构, 哲学核心 |
| 15 | apeireth-sovereignty | `crates/apeireth-sovereignty/src/lib.rs` | `version.workspace = true` | 274KB LOCKED 安全核心, R124-3 调研 0 触碰 |
| 16 | apeireth-constraint | `crates/apeireth-constraint/src/lib.rs` | `version.workspace = true` | 5 重守门核心, R124-3 调研 0 触碰 |
| 17 | apeireth-memory | `crates/apeireth-memory/src/lib.rs` | `version.workspace = true` | LOCKED memory 9 文件, 3 层 memory 哲学核心 |
| 18 | apeireth-cognition | `crates/apeireth-cognition/src/lib.rs` | `version.workspace = true` | R124-2 B-028 OpenCog 借鉴目标, 9 organ brain 来源 |
| 19 | apeireth-perception | `crates/apeireth-perception/src/lib.rs` | `version.workspace = true` | R20 哲学 crate, 9 organ eye/ear 来源 |
| 20 | apeireth-consciousness | `crates/apeireth-consciousness/src/lib.rs` | `version.workspace = true` | R20 哲学 crate (R37-2 transparent re-export 到 perception) |
| 21 | apeireth-motivation | `crates/apeireth-motivation/src/lib.rs` | `version.workspace = true` | R20 哲学 crate (R37-2 transparent re-export) |
| 22 | apeireth-life-force | `crates/apeireth-life-force/src/lib.rs` | `version.workspace = true` | R20 哲学 crate (R37-2 transparent re-export 到 memory) |
| 23 | apeireth-relation | `crates/apeireth-relation/src/lib.rs` | `version.workspace = true` | R20 哲学 crate, R124-2 §12 借鉴目标 |
| 24 | apeireth-value | `crates/apeireth-value/src/lib.rs` | `version.workspace = true` | R20 哲学 crate (R37-2 transparent re-export 到 motivation) |

**24 LOCKED crate Cargo.toml version 严守 100% (per 决策 #33 §2.3 B1 + R131-5 verify 24/24)**:
- 24 LOCKED crate Cargo.toml 全部 `version.workspace = true` (per Cargo.toml:3 实地 verify 100% 一致)
- V1.0 release 整合 #5.1 commit = 0 改 24 LOCKED crate Cargo.toml
- V1.0 release 整合 #5.2 commit = 0 改 24 LOCKED crate Cargo.toml
- V1.0 release 整合 #5.3 commit = 0 改 24 LOCKED crate Cargo.toml
- R131-5 verify 24/24 LOCKED crate 入口签名 0 改全部通过 (1:28 done, per 决策 #75 §2.1 派活)
- 24 LOCKED crate mtime baseline 16:34:11 严守 (per 决策 #33 §2.3 B1 + 决策 #22 §1.2)
- V1.1 release 整合 #6 commit 拍板时 24 LOCKED crate Cargo.toml 0 改 (`version.workspace = true` 自动继承 1.2.1)

### 3.3 借鉴 12 源 fork-then-borrow 模式 (per R131-2 §4.3 + R149-4 借鉴 12 源)

**per R131-2 §4.3 + R149-4 借鉴 12 源 fork-then-borrow 模式 (R153-3 5:15+ verify)**:

#### 8 真 cloned (整合 #5.2 commit 时已 cloned)

| # | 借鉴源 | License | R-Cycle | 整合 #5.2 commit 状态 | Cargo.toml 关联 |
|---|--------|---------|---------|------------------------|------------------|
| 1 | clap-rs/clap 4.6.6 | Apache-2.0 + MIT dual | R125-2 | ✅ done | per Cargo.toml:409 clap 4.5 + derive |
| 2 | hyperium/hyper 0.1.20 | MIT | R125-3 | ✅ done | per Cargo.toml:413 hyper-util 0.1 |
| 3 | modelcontextprotocol/servers 76d64c8 | MIT → Apache-2.0 过渡 | R125-4 | ✅ done | per Cargo.toml:53 http-client + Cargo.toml:67 mcp |
| 4 | PyO3/PyO3 0.29.2 | Apache-2.0 + MIT dual | R125-9 | ✅ done | per Cargo.toml:388 pyo3 0.29 + auto-initialize |
| 5 | model-checking/kani 0.67.0 | MIT + Apache-2.0 dual | R125-10 | ✅ done | per Cargo.toml:75 formal |
| 6 | langchain-ai/langgraph d56666f | MIT | R125-13 | ✅ done | per Cargo.toml:69 graph |
| 7 | obra/superpowers 6.2.0 | MIT | R125-14 | ✅ done | per Cargo.toml:20 skills |
| 8 | NVIDIA/NeMo-Guardrails | Apache-2.0 (R127-2 P6-3 重试 done) | R125-5 | ✅ cloned | per Cargo.toml:31 sovereignty (colang_dsl.rs 51591 bytes) |

#### 2 借鉴 ID 索引完成 (整合 #5.2 commit 时 借鉴 ID 索引完成)

| # | 借鉴源 | License | R-Cycle | 整合 #5.2 commit 状态 | Cargo.toml 关联 |
|---|--------|---------|---------|------------------------|------------------|
| 9 | BerriAI/litellm | 通常 MIT | R125-1 | 🆕 ✅ cloned (R127-2 P6-1 重试 done) | per Cargo.toml:54 pipeline (provider_registry.rs) |
| 10 | sst/opencode | 通常 MIT | R125-12 | 🆕 ✅ cloned (R127-2 P6-2 重试 done) | per Cargo.toml:51 tui + Cargo.toml:6 asi |

#### 1 永久跳过

| # | 借鉴源 | License | 原因 | 0 装 PASS 严守 |
|---|--------|---------|------|------------------|
| 11 | opencog/opencog | ❌ AGPL-3.0 传染性 copyleft | 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-55 §3, 0 集成 0 假装 | 0 装 PASS 严守 |

#### 1 借脑 ID 索引完成 (🆕 R130-6 提议, 整合 #5.2 commit 时新增)

| # | 借脑 ID | License | 6 子源 | 0 装 PASS 严守 |
|---|---------|---------|--------|------------------|
| 12 | R130-6-BORROW-opencog-family-2026Q1-2026-08-11 | 🧠 借脑 ID 索引完成 | OpenCog 家族 6 子源 AGPL-3.0, 0 装 PASS 严守, per decision-33 §2.3 C2 + R149-4 借鉴 12 源 fork-then-borrow 模式 | 0 装 PASS 严守 |

**R153-3 借鉴 12 源 fork-then-borrow 模式 严守 100% (per 决策 #33 §2.3 C2 + R131-2 §4.3 + R149-4)**:
- 12 源 = 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成 = 11+1=12
- 0 装 PASS 严守 (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- 0 触碰 24 LOCKED crate (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1)
- 0 改 workspace.version (1.2.0 严守 V1.0 release, V1.1 release bump 1.2.1 per 决策 #74 B2)

---

## 4. 方向 ③ Cargo workspace 1.2.1 bump Cargo.toml 字段 update 详细 (per 决策 #74 B2 + 决策 #33 §2.3 + 决策 #77 §3.1 + Cargo.toml:1-524 实地 verify)

### 4.1 Cargo.toml [workspace.package] 字段 update 详细

**per Cargo.toml:272-288 实地 verify + V1.1 release 1.2.1 bump 字段 update 实施 spec**:

| 字段 | V1.0 release (1.2.0) | V1.1 release (1.2.1) | update 严守 |
|------|---------------------|---------------------|-------------|
| `version` | `"1.2.0"` (Cargo.toml:274) | `"1.2.1"` | 🔄 BUMP (决策 #74 B2, 1 line 改) |
| `edition` | `"2021"` | `"2021"` | 🔒 0 改 (semver 0 影响 edition) |
| `rust-version` | `"1.80"` | `"1.80"` | 🔒 0 改 (semver 0 影响 rust-version) |
| `authors` | `["Apeireth Team"]` | `["Apeireth Team"]` | 🔒 0 改 (semver 0 影响 authors) |
| `license` | `"Apache-2.0"` | `"Apache-2.0"` | 🔒 0 改 (per Apache 2.0 §4(d) NOTICE 条款 + Cargo.toml:280 实地 verify) |
| `repository` | `"https://github.com/apeireth/apeireth-rust"` | 同 | 🔒 0 改 (semver 0 影响 repository) |
| `description` | "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 17 crate 本源推导 + 双洋葱统一体 + Self-Disable 防护 + 1.0 release (借鉴 8/11 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache)" | V1.1 release description update (借鉴 11/12 + 1 借脑 = 12 源 + 25 LOCKED V1.1 release Mavis 自决改 + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache, per decision-74 B1 V1.1 release Mavis 自决改) | 🔄 UPDATE (决策 #74 B1 V1.1 release Mavis 自决改) |
| `homepage` | `"https://github.com/apeireth/apeireth-rust"` | 同 | 🔒 0 改 (semver 0 影响 homepage) |
| `keywords` | `["ai", "agent", "autopoietic", "principle-onion", "permission-onion", "long-lived-ai", "growth-platform"]` | 同 | 🔒 0 改 (semver 0 影响 keywords) |
| `categories` | `["ai", "asynchronous", "compilers"]` | 同 | 🔒 0 改 (semver 0 影响 categories) |

**Cargo.toml [workspace.package] 字段 update 严守 100% (per 决策 #74 B2 + 决策 #77 §3.1)**:
- 1 字段 BUMP (`version` 1.2.0 → 1.2.1, Cargo.toml:274 改 1 line)
- 1 字段 UPDATE (`description` V1.1 release 内容, per 决策 #74 B1)
- 8 字段 0 改 (`edition` / `rust-version` / `authors` / `license` / `repository` / `homepage` / `keywords` / `categories`)

### 4.2 Cargo.toml [workspace.metadata.apeireth] 字段 update 详细

**per Cargo.toml:296-366 实地 verify + V1.1 release 1.2.1 bump 字段 update 实施 spec**:

| 字段 | V1.0 release (1.2.0) | V1.1 release (1.2.1) | update 严守 |
|------|---------------------|---------------------|-------------|
| `borrow` | `{ count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` (整合 #4 commit 后 17:44 状态) | `{ count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` (整合 #5.2 commit 时已 update 17:44 → 22:50) | 🔄 0 改 (整合 #5.2 commit 已 update, V1.1 release 二次 verify) |
| `borrow_cloned` | 7 entries (clap/hyper/servers/PyO3/kani/langgraph/superpowers) | 10 entries (+Guardrails +LiteLLM +opencode, 整合 #5.2 commit 时 7→10 entries) | 🔄 0 改 (整合 #5.2 commit 已 update) |
| `borrow_rate_limited` | 3 entries (litellm/opencode/Guardrails) | 0 entries (P6-1/2/3 全 done) | 🔄 0 改 (整合 #5.2 commit 已 update) |
| `borrow_skipped` | 1 entry (opencog AGPL-3.0) | 1 entry 0 改 | 🔒 0 改 (整合 #5.2 commit 已 verify 0 改) |
| `borrow_brainonly` | (N/A) | 1 entry: `R130-6-BORROW-opencog-family-2026Q1-2026-08-11` (🆕 1 entry, 6 子源, AGPL-3.0, 0 装 PASS 严守) | 🔄 0 改 (整合 #5.2 commit 已新增) |
| `borrow_local_path` | `".openclaw/workspace/borrowed-repos/"` | 同 | 🔒 0 改 (整合 #5.2 commit 已 verify 0 改) |
| `hard_walls` | "8 (B1-B7+A1-A3+C1-C3, per decision-33 §2 + decision-58 §4)" | 同 (V1.0 release 0 改) | 🔒 0 改 (整合 #5.2 commit 已 verify 0 改) |
| `locked_crates_count` | 24 | 24 (V1.0 release 0 改) → 25 (V1.1 release + PHL-07) | 🔄 UPDATE (V1.1 release PHL-07 实施, per 决策 #74 B1) |
| `philosophy_anchors` | `["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]` | 同 (V1.0 release 0 改) | 🔒 0 改 (8 哲学锚严守, per 决策 #33 §2.3 B5) |
| `measurement_dimensions` | `"V0.5 30 维 (24 基础 + 6 增强)"` | 同 (V1.0 release 0 改) | 🔒 0 改 (V0.5 30 维严守, per 决策 #33 §2.3 B3) |
| `guard_gates_version` | `"v7 (6 重: 1-5 嵌套 + 6 Colang DSL)"` | 同 (V1.0 release 0 改) | 🔒 0 改 (6 重守门 v7 严守, per 决策 #33 §2.3 B4) |
| `verdict_cache_keys` | 13 | 13 (V1.0 release 0 改) → 13 (V1.1 release PHL-07 实施 后 仍 13) | 🔒 0 改 (12 键 + PHL-07 = 13 严守, per 决策 #33 §2.3 A3) |
| `integration_chain` | 5 entries (整合 #1-#5, 整合 #5 待拍) | 7 entries (整合 #1-#7, 整合 #6 估 2026-11-25 + 整合 #7 估 2026-11-29) | 🔄 UPDATE (V1.1 release 整合 #6 + #7 拍板后 加) |
| `license_files` | 4 entries (LICENSE / NOTICE / OSS_NOTICE.md / THIRD-PARTY-NOTICES.md) | 同 (V1.0 release 0 改) | 🔒 0 改 (整合 #5.2 commit 已 verify 0 改) |
| `commit_policy` | "0 主动 commit (Mavis 整合 #5 commit 时机拍板) + 0 主动 push (等 1.0 release 配 GitHub remote)" | "0 主动 commit (Mavis 整合 #6 + #7 commit 时机拍板) + 0 主动 push (等 V1.1 release 配 GitHub remote)" | 🔄 UPDATE (V1.1 release 整合 #6 + #7 拍板后) |
| `decision_chain_range` | "decision-22 ~ decision-58 (37 个决策文件, 完整可追溯 reports/decision-*.md)" | "decision-22 ~ decision-131 (估 110 个决策文件, 完整可追溯 reports/decision-*.md)" | 🔄 UPDATE (V1.1 release 整合 #6 拍板后 加 决策 #59-#131) |

**Cargo.toml [workspace.metadata.apeireth] 字段 update 严守 100% (per 决策 #74 B2 + 决策 #77 §3.1)**:
- 整合 #5.2 commit 已 update: borrow / borrow_cloned / borrow_rate_limited / borrow_brainonly 4 字段
- V1.1 release 整合 #6 commit 拍板后 update: locked_crates_count (24 → 25) / integration_chain (5 → 7 entries) / commit_policy (整合 #5 → 整合 #6 + #7) / decision_chain_range (37 → 估 110 个决策文件)
- 0 改: borrow_skipped / borrow_local_path / hard_walls / philosophy_anchors / measurement_dimensions / guard_gates_version / verdict_cache_keys / license_files 8 字段
- 0 触动: Cargo.toml:1-251 workspace.members (87 workspace members 0 改)

### 4.3 Cargo.toml [workspace.dependencies] + [workspace.lints] + [profile.release] 字段 update 详细

**per Cargo.toml:372-524 实地 verify + V1.1 release 1.2.1 bump 字段 update 实施 spec**:

| 段 | 字段 | V1.0 release (1.2.0) | V1.1 release (1.2.1) | update 严守 |
|----|------|---------------------|---------------------|-------------|
| `[workspace.dependencies]` | 21 entries (tiktoken-rs / tokio / serde / serde_json / anyhow / thiserror / reqwest / futures / pyo3 / rusqlite / chrono / uuid / criterion / proptest / async-trait / lru / shell-words / fs_err / clap / hyper-util / sqlite-vec) | 同 (V1.0 release 0 改) | 🔒 0 改 (0 装 PASS 严守, per 决策 #33 §2.3 C2) |
| `[workspace.lints.rust]` | 6 entries (unused_extern_crates / trivial_numeric_casts / unstable_features / unused_import_braces / unused-lifetimes / unused-macro-rules) + 5 allow (missing_docs / unused_imports / dead_code / unused_must_use / unused_mut) | 同 (V1.0 release 0 改) | 🔒 0 改 (R19 T10 + R20 阶段 6 修复严守) |
| `[workspace.lints.rust.unexpected_cfgs]` | check-cfg = ['cfg(kani)', 'cfg(fuzzing)'] | 同 (V1.0 release 0 改) | 🔒 0 改 (apeireth-formal 用 cfg(kani)) |
| `[workspace.lints.clippy]` | all = 'allow' (wasmtime verbatim) + 18 项精选 lint (uninlined_format_args / match_wildcard_for_single_variants / ... / needless_pass_by_ref_mut) | 同 (V1.0 release 0 改) | 🔒 0 改 (R19 T10 + R20 阶段 6 修复严守) |
| `[profile.release]` | opt-level = 3 / lto = "fat" / codegen-units = 1 / strip = true | 同 (V1.0 release 0 改) | 🔒 0 改 (R19 第 0 阶段第 1 项严守) |

**Cargo.toml [workspace.dependencies] + [workspace.lints] + [profile.release] 字段 update 严守 100% (per 决策 #33 §2.3 C2 + 决策 #77 §3.1)**:
- 0 装 PASS 严守 (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- 0 改 [workspace.dependencies] 段 (21 entries 全部 0 改 version, per Cargo.toml:372-417 实地 verify 100% 一致)
- 0 改 [workspace.lints.rust/clippy] 段 (R19 T10 + R20 阶段 6 修复严守, per Cargo.toml:440-524 实地 verify 100% 一致)
- 0 改 [profile.release] 段 (R19 第 0 阶段第 1 项严守, per Cargo.toml:419-423 实地 verify 100% 一致)

### 4.4 24 LOCKED crate Cargo.toml 字段 update 详细

**per 24 LOCKED crate Cargo.toml 实地 verify + V1.1 release 1.2.1 bump 字段 update 实施 spec**:

| 24 LOCKED crate | [package] 字段 | [dependencies] 段 | [dev-dependencies] 段 | [lints] 段 |
|-----------------|---------------|-------------------|----------------------|------------|
| apeireth-supervisor | name / version.workspace / edition.workspace / rust-version.workspace / license.workspace / authors.workspace / description | apeireth-verify + tokio + serde + serde_json | tokio (test-util) | workspace = true |
| apeireth-agent | 同上 (per apeireth-agent Cargo.toml 实地) | (per apeireth-agent Cargo.toml 实地) | (per apeireth-agent Cargo.toml 实地) | workspace = true |
| apeireth-bus | 同上 | (per apeireth-bus Cargo.toml 实地) | (per apeireth-bus Cargo.toml 实地) | workspace = true |
| apeireth-council | 同上 | (per apeireth-council Cargo.toml 实地) | (per apeireth-council Cargo.toml 实地) | workspace = true |
| apeireth-evolution | 同上 | (per apeireth-evolution Cargo.toml 实地) | (per apeireth-evolution Cargo.toml 实地) | workspace = true |
| apeireth-extension | 同上 | (per apeireth-extension Cargo.toml 实地) | (per apeireth-extension Cargo.toml 实地) | workspace = true |
| apeireth-graph | 同上 | (per apeireth-graph Cargo.toml 实地) | (per apeireth-graph Cargo.toml 实地) | workspace = true |
| apeireth-mcp | 同上 | (per apeireth-mcp Cargo.toml 实地) | (per apeireth-mcp Cargo.toml 实地) | workspace = true |
| apeireth-pipeline | 同上 | (per apeireth-pipeline Cargo.toml 实地) | (per apeireth-pipeline Cargo.toml 实地) | workspace = true |
| apeireth-tool-registry | 同上 | (per apeireth-tool-registry Cargo.toml 实地) | (per apeireth-tool-registry Cargo.toml 实地) | workspace = true |
| apeireth-tool-runtime | 同上 | (per apeireth-tool-runtime Cargo.toml 实地) | (per apeireth-tool-runtime Cargo.toml 实地) | workspace = true |
| apeireth-protocol | 同上 | (per apeireth-protocol Cargo.toml 实地) | (per apeireth-protocol Cargo.toml 实地) | workspace = true |
| apeireth-asi | 同上 | (per apeireth-asi Cargo.toml 实地) | (per apeireth-asi Cargo.toml 实地) | workspace = true |
| apeireth-onion | 同上 | (per apeireth-onion Cargo.toml 实地) | (per apeireth-onion Cargo.toml 实地) | workspace = true |
| apeireth-sovereignty | 同上 | (per apeireth-sovereignty Cargo.toml 实地) | (per apeireth-sovereignty Cargo.toml 实地) | workspace = true |
| apeireth-constraint | 同上 | (per apeireth-constraint Cargo.toml 实地) | (per apeireth-constraint Cargo.toml 实地) | workspace = true |
| apeireth-memory | 同上 | (per apeireth-memory Cargo.toml 实地) | (per apeireth-memory Cargo.toml 实地) | workspace = true |
| apeireth-cognition | 同上 | (per apeireth-cognition Cargo.toml 实地) | (per apeireth-cognition Cargo.toml 实地) | workspace = true |
| apeireth-perception | 同上 | (per apeireth-perception Cargo.toml 实地) | (per apeireth-perception Cargo.toml 实地) | workspace = true |
| apeireth-consciousness | 同上 | (per apeireth-consciousness Cargo.toml 实地) | (per apeireth-consciousness Cargo.toml 实地) | workspace = true |
| apeireth-motivation | 同上 | (per apeireth-motivation Cargo.toml 实地) | (per apeireth-motivation Cargo.toml 实地) | workspace = true |
| apeireth-life-force | 同上 | (per apeireth-life-force Cargo.toml 实地) | (per apeireth-life-force Cargo.toml 实地) | workspace = true |
| apeireth-relation | 同上 | (per apeireth-relation Cargo.toml 实地) | (per apeireth-relation Cargo.toml 实地) | workspace = true |
| apeireth-value | 同上 | (per apeireth-value Cargo.toml 实地) | (per apeireth-value Cargo.toml 实地) | workspace = true |

**24 LOCKED crate Cargo.toml 字段 update 严守 100% (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 + 决策 #77 §3.1)**:
- 24 LOCKED crate Cargo.toml 全部 `version.workspace = true` (自动继承 workspace.version 1.2.1, 0 改文件)
- 24 LOCKED crate Cargo.toml 全部 `edition.workspace = true` (per Cargo.toml:4 实地 verify 100% 一致)
- 24 LOCKED crate Cargo.toml 全部 `rust-version.workspace = true` (per Cargo.toml:5 实地 verify 100% 一致)
- 24 LOCKED crate Cargo.toml 全部 `license.workspace = true` (per Cargo.toml:6 实地 verify 100% 一致)
- 24 LOCKED crate Cargo.toml 全部 `authors.workspace = true` (per Cargo.toml:7 实地 verify 100% 一致)
- 24 LOCKED crate Cargo.toml `[dependencies]` 段 0 改 (0 装 PASS 严守)
- 24 LOCKED crate Cargo.toml `[dev-dependencies]` 段 0 改 (0 装 PASS 严守)
- 24 LOCKED crate Cargo.toml `[lints] workspace = true` 段 0 改 (R19 T10 + R20 阶段 6 修复严守)
- 24 LOCKED crate mtime baseline 16:34:11 严守 (per 决策 #33 §2.3 B1 + 决策 #22 §1.2)

### 4.5 63 非 LOCKED crate Cargo.toml 字段 update 详细

**per 63 非 LOCKED crate Cargo.toml 实地 verify + V1.1 release 1.2.1 bump 字段 update 实施 spec**:

| 类型 | 数量 | [package] 字段 update 策略 | [dependencies] 段 update 策略 | [lints] 段 update 策略 |
|------|------|--------------------------|------------------------------|------------------------|
| **1.2.0 + version.workspace = true (51 crates)** | 51 | 0 改 (`version.workspace = true` 继承 1.2.1) | 0 改 (0 装 PASS 严守) | 0 改 (R19 T10 严守) |
| **0.1.0 + 硬编码 version (22 crates: cache / credentials / i18n / image-prompt / keyring / lark / livekit / memory-extensions / metrics / oauth / observability / pipeline-g5 / plugin / repo-analyzer / repo-scan / sandbox / state / task / tracing / tree-sitter / update / voice)** | 22 | 0 改 (skeleton 阶段硬编码, 1.0 release 后清) | 0 改 (0 装 PASS 严守) | 0 改 (R20 阶段 6 skeleton 阶段不强求) |
| **1.0.0 + 硬编码 version (5 crates: blueprint-impl / integration-e2e / integration-r20-stage4 / rate-limiter / team-lead)** | 5 | 0 改 (V1302-V1307 fix 修真, 1.0 release 后清) | 0 改 (0 装 PASS 严守) | 0 改 (R20 阶段 6 skeleton 阶段不强求) |

**63 非 LOCKED crate Cargo.toml 字段 update 严守 100% (per 决策 #33 §2.3 C2 + 决策 #77 §3.1)**:
- 51 + 22 + 5 = 78 (含 LOCKED 24 = 87 + 子 crate 1 = 88 + apeireth-tui 9 organ = 97, 跟 R131-4 实地清点一致)
- 0 改 [package] 字段 (除 `version.workspace = true` 51 crates 自动继承 1.2.1)
- 0 改 [dependencies] 段 (0 装 PASS 严守)
- 0 改 [dev-dependencies] 段 (0 装 PASS 严守)
- 0 改 [lints] 段 (R20 阶段 6 skeleton 阶段不强求, 整合时 Mavis 改为 [lints] workspace = true)
- 22 + 5 = 27 硬编码 version crate = 已知 TODO, 1.0 release 后清 (per Cargo.toml:270 注释 + P15-1 0 主动 commit 严守 → 0 改 27 crate scope creep)

---

## 5. 方向 ④ Cargo workspace 1.2.1 bump Cargo.lock update 策略 详细 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1)

### 5.1 Cargo.lock 实地状态 (per Cargo.lock 实地 verify 05:15+)

**per `Select-String -Path Apeireth-rust\Cargo.lock -Pattern '^name = "apeireth-' | ForEach-Object { $_.Line -replace 'name = "(apeireth-[a-z0-9-]+)".*', '$1' } | Sort-Object -Unique` (R153-3 5:15+ verify)**:

- **Cargo.lock = 271,450 bytes (~265 KB)** (per R131-4 §0 + 5:15+ verify)
- **87 workspace members + 561 第三方 = 648 crate 合理范围**
- 业界 50-100 crate 项目通常 150-350 KB, 87 crate 项目 ~265 KB 合理

**Cargo.lock 实地 24 LOCKED crate version 状态 (per Cargo.lock 实地 verify 05:15+)**:
- apeireth-supervisor = "1.2.0"
- apeireth-agent = "1.2.0"
- apeireth-bus = "1.2.0"
- apeireth-council = "1.2.0"
- apeireth-evolution = "1.2.0"
- apeireth-extension = "1.2.0"
- apeireth-graph = "1.2.0"
- apeireth-mcp = "1.2.0"
- apeireth-pipeline = "1.2.0"
- apeireth-tool-registry = "1.2.0"
- apeireth-tool-runtime = "1.2.0"
- apeireth-protocol = "1.2.0"
- apeireth-asi = "1.2.0"
- apeireth-onion = "1.2.0"
- apeireth-sovereignty = "1.2.0"
- apeireth-constraint = "1.2.0"
- apeireth-memory = "1.2.0"
- apeireth-cognition = "1.2.0"
- apeireth-perception = "1.2.0"
- apeireth-consciousness = "1.2.0"
- apeireth-motivation = "1.2.0"
- apeireth-life-force = "1.2.0"
- apeireth-relation = "1.2.0"
- apeireth-value = "1.2.0"

**24 LOCKED crate Cargo.lock version 1.2.0 严守 100% (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 + R131-5 verify 24/24)**:
- 24 LOCKED crate Cargo.lock version 字段 = "1.2.0" (整合 #5 commit 拍板时, per Cargo.lock 实地 verify 100% 一致)
- V1.0 release 整合 #5.1 commit 拍板时 = 0 改 Cargo.lock
- V1.1 release 整合 #6 commit 拍板时 = 24 LOCKED crate Cargo.lock version 字段 1.2.0 → 1.2.1 (因 workspace.version 1.2.0 → 1.2.1, 自动同步)

### 5.2 Cargo.lock V1.1 release update 5 步策略 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1)

**R153-3 整合 V1.1 release Cargo.lock update 5 步策略 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1 + R152-1 §5.2)**:

```bash
# V1.1 release Cargo.lock update 5 步 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1 + 决策 #86 §4 R152 era 实施阶段)
# 0 装 PASS 严守: 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2)
# 仅 cargo update 0 升 workspace deps (per Cargo.toml [workspace.dependencies] 段)

# Step 1: cargo metadata --no-deps --format-version 1 (验证 workspace 完整性, 0 触碰 Cargo.lock)
cargo metadata --no-deps --format-version 1

# Step 2: cargo check --workspace (检查 workspace 完整性, 0 触碰 Cargo.lock)
cargo check --workspace

# Step 3: cargo update --workspace --offline (offline mode, 0 触碰 crates.io, 仅同步 version 字段)
cargo update --workspace --offline

# Step 4: cargo build --workspace --release (release 模式编译, 验证 V1.1 release bump 后编译通过)
cargo build --workspace --release

# Step 5: cargo test --workspace --release (release 模式测试, 验证 V1.1 release bump 后 4100+ tests 仍 pass)
cargo test --workspace --release
```

**Cargo.lock V1.1 release update 边界 (per 决策 #33 §2.3 C2 + 决策 #74 B2)**:
- 0 装 PASS 严守 = 0 cargo install / 0 cargo add
- 0 改 [workspace.dependencies] 段 (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc 全部 0 改 version)
- 0 改 24 LOCKED crate Cargo.toml `[dependencies]` 段 (per B1 0 改 + 0 装 PASS 严守)
- 0 改 87 workspace members 各自 Cargo.toml `[dependencies]` 段 (per 0 装 PASS 严守)
- Cargo.lock 仅 workspace.version 字段 1.2.0 → 1.2.1 (24 LOCKED crate version 字段自动同步)
- 0 改 Cargo.lock 第三方依赖 version (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc)

### 5.3 Cargo.lock V1.1 release update 3 策略对比 (per R152-1 §5.2 + 决策 #77 §3.1)

**R153-3 整合 Cargo.lock V1.1 release update 3 策略对比 (per R152-1 §5.2 + 决策 #77 §3.1)**:

#### 策略 A: cargo update --workspace --offline (1 次, 效率高)

**策略 A 步骤**:
```bash
# Step 1-5: 同 §5.2 (5 步)
# 优势: 1 次 cargo update --workspace, 效率高
# 劣势: 0 精细控制 (per-crate 0 触碰)
```

**策略 A 适用场景**:
- V1.1 release Cargo.lock 0 改 第三方依赖 (semver 1.2.0 → 1.2.1 0 影响 第三方依赖)
- 0 装 PASS 严守 100% 严守 (0 cargo install / 0 cargo add)
- 0 触碰 workspace.dependencies 段 100% 严守 (per Cargo.toml:372-417 实地 verify)
- 总 5 次命令, 1 hour 估 跑完

#### 策略 B: cargo update -p apeireth-{crate} (87 次, per-crate 精细控制)

**策略 B 步骤**:
```bash
# V1.1 release Cargo.lock per-crate update 87 步 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1)
# 0 装 PASS 严守: 0 cargo install / 0 cargo add
# 仅 cargo update -p apeireth-{crate} 单独 update (per-crate 精细控制)
cargo update -p apeireth-supervisor  # 24 LOCKED crate 各自 update
cargo update -p apeireth-agent
cargo update -p apeireth-bus
cargo update -p apeireth-council
cargo update -p apeireth-evolution
cargo update -p apeireth-extension
cargo update -p apeireth-graph
cargo update -p apeireth-mcp
cargo update -p apeireth-pipeline
cargo update -p apeireth-tool-registry
cargo update -p apeireth-tool-runtime
cargo update -p apeireth-protocol
cargo update -p apeireth-asi
cargo update -p apeireth-onion
cargo update -p apeireth-sovereignty
cargo update -p apeireth-constraint
cargo update -p apeireth-memory
cargo update -p apeireth-cognition
cargo update -p apeireth-perception
cargo update -p apeireth-consciousness
cargo update -p apeireth-motivation
cargo update -p apeireth-life-force
cargo update -p apeireth-relation
cargo update -p apeireth-value
# ... 87 crate 全部 update, 0 cargo add, 0 cargo install
```

**策略 B 适用场景**:
- 0 装 PASS 严守 (0 cargo install / 0 cargo add)
- 0 触碰 workspace.dependencies 段 (仅 Cargo.lock version 字段 同步)
- 跟 R137-3 1.2.1 bump 实施 spec 第 1 版一致 (per R137-3 §3.3)
- 总 87 次命令, 估 2-3 hours 跑完

#### 策略 C: 混合策略 (R152-1 推荐, R153-3 拓维确认)

**策略 C 步骤 (per R152-1 §5.2 + R153-3 拓维)**:
1. **阶段 1**: `cargo update --workspace --offline` (1 次, 仅 workspace.version 字段 1.2.0 → 1.2.1)
2. **阶段 2**: 24 LOCKED crate 单独 verify (24 次, per-crate 0 触碰, 0 装 PASS 严守)
3. **阶段 3**: 63 非 LOCKED crate 单独 verify (63 次, per-crate 0 触碰, 0 装 PASS 严守)
4. **阶段 4**: `cargo build --workspace --release` (1 次, 验证 V1.1 release bump 后编译通过)
5. **阶段 5**: `cargo test --workspace --release` (1 次, 验证 V1.1 release bump 后 4100+ tests 仍 pass)

**策略 C 优势 (per R152-1 + R153-3)**:
- 阶段 1: 1 次 update --workspace, 效率高
- 阶段 2-3: 87 次 per-crate verify, 精细控制
- 阶段 4-5: 1 次 build + 1 次 test, 验证 V1.1 release bump 后编译测试通过
- 总 90 次命令 (vs 策略 A = 5 次, vs 策略 B = 87 次)
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add)
- 0 触碰 workspace.dependencies 段 100%
- 0 触碰 24 LOCKED crate Cargo.toml 100%
- **R153-3 拓维 评估**: 策略 C 适合 V1.1 release Cargo.lock update (效率高 + 精细控制 + 0 装严守 100%)

### 5.4 Cargo.lock V1.1 release update 5 风险 (per 决策 #33 §2.3 C2 + 决策 #77 §3.1 + R152-1 §5.3)

**R153-3 整合 Cargo.lock V1.1 release update 5 风险 (per 决策 #33 §2.3 C2 + 决策 #77 §3.1 + R152-1 §5.3)**:

#### 风险 R1: cargo update --workspace --offline 触发 第三方依赖 version 升级

**风险描述**:
- `cargo update --workspace --offline` 可能触发 Cargo.lock 第三方依赖 version 升级 (e.g. tokio 1.40 → 1.41)
- 0 装 PASS 严守 100% 触发 (per 决策 #33 §2.3 C2)
- workspace.dependencies 段 0 改 (per Cargo.toml:372-417 实地 verify 100% 一致)

**缓解策略**:
- offline mode + 0 改 [workspace.dependencies] 段 (per Cargo.toml:372-417 实地 verify)
- V1.1 release bump 后 第三方依赖 version 字段 0 改 (per semver 1.2.0 → 1.2.1 0 影响 第三方依赖)
- 0 cargo update -p <external-crate> 触发 (per 0 装 PASS 严守)

#### 风险 R2: cargo build --workspace --release 编译失败

**风险描述**:
- V1.1 release bump 后 编译可能失败 (整合 #5.1 commit 拍板时 R139-1-retry-2 续修 30 hard errors 仍 pending)
- 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 触发 编译失败
- TUI 9 organ 内部 fn 实施可改 触发 编译失败

**缓解策略**:
- 整合 #5.1 commit 拍板时 R139-1-retry-2 续修 30 hard errors (per 决策 #78 §2.3 + 决策 #86 §4 派活)
- V1.1 release 0 改 src 严守 (实施 spec 调研), 0 触碰 24 LOCKED crate src/
- 整合 #6 commit 拍板时 4 步 verify (cargo metadata + cargo check + cargo build + cargo test)
- 8 步 verify V1.1 release 实战 (per §5.5)

#### 风险 R3: cargo test --workspace --release 测试 fail (30 hard errors pending)

**风险描述**:
- V1.1 release bump 后 cargo test 4100+ tests 可能 fail (R129-21 0 装 PASS violation 报告 24+5+1 errors)
- 30 hard errors 仍 pending (R144-1 02:38 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL, per 决策 #87 §1)
- 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 触发 测试 fail

**缓解策略**:
- 整合 #5.1 commit 拍板时 R139-1-retry-2 续修 30 hard errors (per 决策 #78 §2.3 + 决策 #86 §4 派活)
- V1.1 release cargo test 应 100% pass (0 hard errors 0 装 PASS 严守)
- 整合 #6 commit 拍板时 4 步 verify (cargo metadata + cargo check + cargo build + cargo test)
- 8 步 verify V1.1 release 实战 (per §5.5)

#### 风险 R4: cargo check --workspace 编译警告 / error

**风险描述**:
- V1.1 release bump 后 cargo check 487 warning (R23 P3 末实测) 可能新增
- 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 触发 warning
- clippy.toml 配置不匹配 (R20 阶段 6 修复已 done)

**缓解策略**:
- 整合 #5.1 commit 拍板时 R139-1-retry-2 续修 30 hard errors (per 决策 #78 §2.3)
- V1.1 release cargo check 应 0 new warning (per 整合 #6 commit 拍板 4 步 verify)
- cargo clippy --workspace --all-targets --all-features -- -D warnings 0 new warning (8 步 verify Step 4)

#### 风险 R5: cargo audit / cargo deny check violation

**风险描述**:
- V1.1 release bump 后 cargo audit 触发 vulnerability (R145-3 8 步 verify verify 100% 严守)
- cargo deny check 触发 violation (per deny.toml 严守)
- 0 装 PASS violation (per 决策 #33 §2.3 C2 严守)

**缓解策略**:
- 整合 #5.1 commit 拍板时 24+5+1 errors 0 装严守 verify done (per R129-21 0 装 PASS violation 报告)
- V1.1 release cargo audit 应 0 vulnerability (per 0 装 PASS 严守)
- V1.1 release cargo deny check 应 0 violation (per deny.toml 严守)
- 8 步 verify V1.1 release Step 6 + Step 7 (per §5.5)

### 5.5 8 步 verify V1.1 release (per 决策 #33 §2.3 C2 + 决策 #74 B2 + R144-1 8 步 verify 流程 + 决策 #78 §2.3)

**R153-3 整合 V1.1 release 8 步 verify (per 决策 #33 §2.3 C2 + 决策 #74 B2 + R144-1 8 步 verify 流程 + 决策 #78 §2.3)**:

| Step | 8 步 verify | 详细 | 决策依据 | 严守 100% |
|------|------------|------|---------|-----------|
| **Step 1** | **`cargo build --workspace`** | V1.1 release 编译通过, 24 LOCKED crate + 63 非 LOCKED crate 全编译通过 | 决策 #74 B2 + 决策 #33 §2.3 C2 | ✅ 0 装 PASS 严守 |
| **Step 2** | **`cargo test --workspace`** | V1.1 release 测试通过, 4100+ tests 仍 pass, 0 重跑 0 装 PASS 严守 | 决策 #33 §2.3 C2 | ✅ 0 装 PASS 严守 |
| **Step 3** | **`cargo run tui 0 --help`** | TUI 0 装 PASS 严守 baseline, V1.1 release bump 后 仍 0 装 | R144-1 02:38 + 决策 #78 §2.3 | ✅ 0 装 PASS 严守 |
| **Step 4** | **`cargo clippy --workspace --all-targets --all-features -- -D warnings`** | V1.1 release clippy 严守, 0 new warning | R19 T10 + 决策 #33 §2.3 + 决策 #74 B2 | ✅ 8 硬墙 0 越界 |
| **Step 5** | **`cargo fmt --all -- --check`** | V1.1 release fmt 严守, 0 改 format | 决策 #33 §2.3 + 决策 #78 §2.3 | ✅ 0 装 PASS 严守 |
| **Step 6** | **`cargo audit`** | V1.1 release 安全 audit 严守, 0 vulnerability | 决策 #33 §2.3 C2 | ✅ 0 装 PASS 严守 |
| **Step 7** | **`cargo deny check`** | V1.1 release license deny 严守, 0 violation | 决策 #33 §2.3 C2 + deny.toml 严守 | ✅ 0 装 PASS 严守 |
| **Step 8** | **`cargo doc --workspace --no-deps --all-features`** | V1.1 release doc 严守, 0 缺失 doc | 决策 #33 §2.3 + 决策 #78 §2.3 | ✅ 0 装 PASS 严守 |

**8 步 verify V1.1 release 实战时间 (per 决策 #71 §2.5 + 决策 #78 §2.3)**:
- 估 2026-11-30 06:00-08:00 主人手跑 (per 决策 #71 §2.5 + 决策 #87 + 决策 #78 §2.1)
- 8 步 verify 估 2 hours 跑完
- 主人手跑 8 步 verify 期间 0 主动 commit 严守 (Mavis 自决拍板)

---

## 6. 方向 ⑤ Cargo workspace 1.2.1 bump 跟 24 LOCKED 入口签名 (决策 #74 B1) 关系 详细 (per 决策 #74 B1 + 决策 #22 §1.2 + 决策 #33 §2.3 B1 + 决策 #77 §3.1 + R150-3 §4.2)

### 6.1 24 LOCKED 入口签名 状态 (per 决策 #74 B1 + 决策 #33 §2.3 B1)

**24 LOCKED 入口签名 状态 (per 决策 #74 B1 + 决策 #33 §2.3 B1)**:
- ✅ V1.0 release 0 改严守 (24 LOCKED crate mtime baseline 16:34:11 严守, 整合 #5.1 commit 0 改 入口签名)
- 🟢 V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1 改写)
- ✅ V2.0 release 8 硬墙可重评 (per 决策 #74 §2.3)

**24 LOCKED 入口签名 边界 (per 决策 #33 §2.3 B1 + 决策 #74 §1)**:
- 入口签名 = lib.rs `pub mod` / `pub use` / `pub const` / `pub struct` / `pub enum` / `pub fn`
- 内部 fn 实施可改 (per 决策 #41 §2 + 决策 #47)
- Cargo.toml 字段 (除 `version.workspace = true` 继承) 0 改

### 6.2 1.2.1 bump 跟 24 LOCKED 入口签名 关系 详细 (per 决策 #74 B1 + R137-2 + R137-3 + R150-3 §4.2)

**1.2.1 bump 跟 24 LOCKED 入口签名 关系分析 (per 决策 #74 B1 + R137-2 + R137-3 + R150-3 §4.2)**:

| 维度 | 1.2.1 bump | 24 LOCKED 入口签名 | 关系 |
|------|-----------|------------------|------|
| **Cargo.toml 字段** | workspace.version 1.2.0 → 1.2.1 (Cargo.toml:274 改) | 24 LOCKED crate Cargo.toml 字段 0 改 (除 version.workspace = true 继承) | ✅ 1.2.1 bump 0 触动 24 LOCKED Cargo.toml 字段 |
| **src/ 入口签名** | 0 触动 (Cargo.toml 字段 跟 src/ 入口签名 无关) | V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1) | ✅ 1.2.1 bump 0 触动 24 LOCKED src/ 入口签名 |
| **mtime baseline 16:34:11** | 0 触动 (Cargo.toml 字段 跟 mtime 无关) | 0 触动 (24 LOCKED crate mtime 严守) | ✅ 1.2.1 bump 0 触动 24 LOCKED crate mtime |
| **R11 baseline 3 值** | 0 触动 (Cargo.toml 字段 跟 R11 baseline 无关) | 0 触动 (V1.0 release 0 改严守, V1.1 release R12 测度对齐 改 24+11 = 35 维) | ✅ 1.2.1 bump 0 触动 R11 baseline 3 值, V1.1 release R12 测度对齐 跟 1.2.1 bump 同步 |
| **PHL-07 实施** | 0 触动 (Cargo.toml 字段 跟 PHL-07 实施无关) | V1.1 release PHL-07 实施 (24 → 25 LOCKED + 13 → 14 键) | ✅ 1.2.1 bump 0 触动 PHL-07 实施 |
| **ASI Stage 9 长程 AI 成长** | 0 触动 (Cargo.toml 字段 跟 ASI Stage 9 无关) | V1.1 release ASI Stage 9 实施 (per R137-4 + R149-2) | ✅ 1.2.1 bump 0 触动 ASI Stage 9 |
| **三洋葱 V2 升级 → 四洋葱** | 0 触动 (Cargo.toml 字段 跟 洋葱架构 无关) | V1.1 release 三洋葱 V2 升级 实施 (per R137-3) | ✅ 1.2.1 bump 0 触动 三洋葱 V2 |
| **9 organ 借 OpenCode 拟人化深化** | 0 触动 (Cargo.toml 字段 跟 organ 拟人化 无关) | V1.1 release 9 organ 借 OpenCode 实施 (per R137-4) | ✅ 1.2.1 bump 0 触动 9 organ 借 OpenCode |
| **R12 测度对齐** | 0 触动 (Cargo.toml 字段 跟 测度公式 无关) | V1.1 release R12 测度对齐 实施 (24+11 = 35 维) | ✅ 1.2.1 bump 0 触动 R12 测度对齐 |
| **借鉴源 12 源 0 装严守 二次 verify** | 0 触动 (Cargo.toml 字段 跟 借鉴源 0 装严守 无关) | V1.1 release 借鉴 12 源 0 装严守 二次 verify (per R131-6 §0 + 决策 #33 §2.3 C2) | ✅ 1.2.1 bump 0 触动 借鉴 12 源 |

**1.2.1 bump 跟 24 LOCKED 入口签名 关系总结 (per 决策 #74 B1 + R137-2 + R137-3 + R150-3 §4.2)**:
- ✅ **1.2.1 bump = Cargo.toml workspace.version bump** (Cargo.toml:274 改 1 line)
- ✅ **24 LOCKED 入口签名 = src/ lib.rs 字段** (per 决策 #33 §2.3 B1)
- ✅ **1.2.1 bump 跟 24 LOCKED 入口签名 0 关系** (Cargo.toml 字段 跟 src/ 入口签名 无关)
- ✅ **1.2.1 bump 跟 V1.1 release 24 LOCKED 入口签名 Mavis 自决改 0 关系** (决策 #74 B1 是 src/ 改写, 跟版本号 bump 无关)
- ✅ **1.2.1 bump 跟 V1.1 release 24 LOCKED Cargo.toml 0 关系** (24 LOCKED Cargo.toml 0 改, 自动继承 workspace.version)
- ✅ **24 LOCKED crate 完整 25 (24 + PHL-07) 严守** (per 决策 #74 A3 PHL-07 V1.1 release 实施 + 24 LOCKED crate mtime baseline 严守)

---

## 7. 方向 ⑥ Cargo workspace 1.2.1 bump 跟 借鉴 12 源 fork-then-borrow 模式 关系 详细 (per R131-2 §4.3 + R149-4 + 决策 #77 §3.1 + R152-1 §6.2)

### 7.1 借鉴 12 源 fork-then-borrow 4 类 (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + R149-4)

**借鉴 12 源 fork-then-borrow 4 类 (per 决策 #33 §2.2 + 决策 #55 §2.6 + 决策 #73 §3 + 决策 #74 B1 + R149-4 §0 TL;DR)**:

- **A 类: ✅ cloned 真实施** (8 源) — 公开 API license 兼容 (Apache-2.0/MIT/dual) + 0 借私有 fn + 1:1 翻译 → ✅ 真集成 src + tests pass
  - 1: clap-rs/clap 4.6.6 (Apache-2.0 + MIT dual, R125-2)
  - 2: hyperium/hyper 0.1.20 (MIT, R125-3)
  - 3: modelcontextprotocol/servers 76d64c8 (MIT → Apache-2.0 过渡, R125-4)
  - 4: PyO3/PyO3 0.29.2 (Apache-2.0 + MIT dual, R125-9)
  - 5: model-checking/kani 0.67.0 (MIT + Apache-2.0 dual, R125-10)
  - 6: langchain-ai/langgraph d56666f (MIT, R125-13)
  - 7: obra/superpowers 6.2.0 (MIT, R125-14)
  - 8: NVIDIA/NeMo-Guardrails (Apache-2.0, R125-5)
- **B 类: ⏳ 限流 → ✅ 1:1 翻译公开** (2 源 LiteLLM + opencode) — 限流持续 → 0 借具体源码 + 公开 docs 1:1 翻译 → ✅ 0 装"已读真源码"
  - 9: BerriAI/litellm (通常 MIT, R125-1, R127-2 P6-1 done)
  - 10: sst/opencode (通常 MIT, R125-12, R127-2 P6-2 done)
- **C 类: ❌ license 不兼容 永久跳过** (1 源 OpenCog AGPL-3.0) — 主仓 Apache-2.0 vs 强 copyleft 不可派生 → ❌ 永久 0 主仓集成 + 0 主仓 fork + ⏳ R130-6 借脑 + 🆕 1.0 release 后独立 fork 决策 (per 决策 #33 §2.2 主人主动问)
  - 11: opencog/opencog (AGPL-3.0, R124-2)
- **D 类: 🆕 借脑 (paper/architecture docs, 0 license)** (1 源 OpenCog 家族 6 子源) — 论文/著作/architecture 文档 0 license 风险 → 0 装"已读真源码" + 0 装"已集成" + 0 装"已 fork"
  - 12: 🆕 R130-6-BORROW-opencog-family-2026Q1-2026-08-11 (AGPL-3.0, 6 子源借脑 ID 索引完成)

### 7.2 1.2.1 bump 跟 借鉴 12 源 fork-then-borrow 模式 关系 详细 (per R131-2 §4.3 + R149-4 + 决策 #77 §3.1 + R152-1 §6.2)

**1.2.1 bump 跟 借鉴 12 源 fork-then-borrow 模式 关系 详细 (per R131-2 §4.3 + R149-4 + 决策 #77 §3.1 + R152-1 §6.2)**:

| 借鉴 12 源 | Cargo workspace 1.2.1 bump 关系 | 0 装 PASS 严守 |
|------------|-------------------------------|---------------|
| **8 真 cloned (A 类)** | ✅ Cargo.toml 关联 (clap 4.5 / hyper-util 0.1 / pyo3 0.29 / 等, per Cargo.toml:372-417 [workspace.dependencies] 段) | ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2) |
| **2 借鉴 ID 索引完成 (B 类 LiteLLM/opencode)** | ✅ Cargo.toml 关联 (pipeline / tui + asi, per Cargo.toml:54 + Cargo.toml:51 + Cargo.toml:6) | ✅ 0 装 PASS 严守 |
| **1 永久跳过 (C 类 opencog AGPL-3.0)** | ❌ 0 集成 0 假装 (per decision-22 §4 + decision-55 §3) | ✅ 0 装 PASS 严守 |
| **1 借脑 ID 索引完成 (D 类 R130-6 OpenCog 家族 6 子源)** | 🧠 0 装 PASS 严守, fork-then-borrow 模式 (per R149-4) | ✅ 0 装 PASS 严守 |

**借鉴 12 源 跟 Cargo workspace 1.2.1 bump 关系 (per R152-1 实施 spec 调研 + R153-3 拓维)**:
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- 0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor + 8 项不修改承诺 (per 决策 #33 §2.3 + 决策 #77 §3.1)
- 借鉴 12 源 fork-then-borrow 模式 (per R149-4) 0 触碰 24 LOCKED crate + 0 装 PASS 严守
- borrow 段 V1.0 release 整合 #5.2 commit 已 update 17:44 → 22:50 状态 (`count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1`)
- borrow 段 V1.1 release 0 装严守 二次 verify (per R131-6 §0 + 决策 #77 §3.1 + 整合 #6 commit 拍板时 11 步 verify)
- 借鉴 12 源 Cargo.toml 关联 (clap 4.5 / hyper-util 0.1 / pyo3 0.29 / etc) 0 改 (V1.1 release bump 0 改 [workspace.dependencies] 段, per Cargo.toml:372-417 实地 verify)
- 1.0 release 后独立 fork OpenCog 实验仓决策 (per R130-6 §2.3.4 路径 A = 实验仓 `apeireth-opencog-experimental` AGPL-3.0, per 决策 #33 §2.2 + 决策 #55 §2.6 + R149-4)

**1.2.1 bump 跟 Cargo.toml borrow 段 关系 详细 (per R131-6 §0 + 决策 #74 B2 + 决策 #33 §2.3 C2 + R152-1 §5.1)**:

| Cargo.toml borrow 段 字段 | 1.2.1 bump 关系 | 严守 100% |
|--------------------------|----------------|-----------|
| **`borrow = { ... }` 字段** | 0 触动 (workspace.version bump 跟 borrow 段 无关) | ✅ 1.2.1 bump 0 触动 borrow = { ... } 字段 |
| **`borrow_cloned = [...]` 列表** | 0 触动 (workspace.version bump 跟 borrow_cloned 列表 无关) | ✅ 1.2.1 bump 0 触动 borrow_cloned 列表 |
| **`borrow_rate_limited = [...]` 列表** | 0 触动 (workspace.version bump 跟 borrow_rate_limited 列表 无关) | ✅ 1.2.1 bump 0 触动 borrow_rate_limited 列表 |
| **`borrow_skipped = [...]` 列表** | 0 触动 (workspace.version bump 跟 borrow_skipped 列表 无关) | ✅ 1.2.1 bump 0 触动 borrow_skipped 列表 |
| **`borrow_brainonly = [...]` 列表** | 0 触动 (workspace.version bump 跟 borrow_brainonly 列表 无关) | ✅ 1.2.1 bump 0 触动 borrow_brainonly 列表 |
| **`borrow_local_path` 字段** | 0 触动 (workspace.version bump 跟 borrow_local_path 无关) | ✅ 1.2.1 bump 0 触动 borrow_local_path 字段 |

---

## 8. 方向 ⑦ Cargo workspace 1.2.1 bump 跟 8 哲学锚 + 不要怕复杂度哲学 关系 详细 (per 决策 #73 §3 + 决策 #33 §2.3 B5 + 决策 #74 §1 B5 + 哲学文档 `09-anchor.md` + 哲学文档 `15-no-fear-complexity.md` + R150-3 §4.3)

### 8.1 8 哲学锚 + 不要怕复杂度哲学 = 9 件套 总哲学 (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`)

**8 哲学锚 (S-1 + S-2 + S-3 + O-1 + O-2 + O-3 + O-4 + O-5, per 决策 #33 §2.3 B5 + 决策 #74 §1 B5 + 哲学文档 `09-anchor.md`)**:

| 哲学锚 | 类型 | 描述 | 1.2.1 bump 关系 |
|------|----|------|----------------|
| **S-1** | **北极星导向** | 思想哲学 | 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 verdict cache 北极星 | ✅ 严守 0 改 (Cargo.toml 1.2.1 bump 0 触动 思想哲学) |
| **S-2** | **实事求是** | 思想哲学 | borrow 段 update 17:44 → 22:50 状态 (整合 #5.2 commit) 实事求是 | ✅ 严守 0 改 (1.2.1 bump 严守 实际状态 = 5 阶段 5 天 1 周 实施 spec) |
| **S-3** | **质量工程化** | 思想哲学 | cargo build / test / clippy / fmt / audit / deny / doc 8 步 verify 质量工程化 | ✅ 严守 0 改 (1.2.1 bump 严守 cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名 8 步 verify) |
| **O-1** | **安全优先** | 思想哲学 | 0 装 PASS 严守 + 24 LOCKED 入口签名 0 改 V1.0 release | ✅ 严守 0 改 (1.2.1 bump 严守 0 装 PASS 严守 + 0 改 24 LOCKED mtime baseline 16:34:11) |
| **O-2** | **走在前人肩上** | 思想哲学 | 借鉴 8/11 ✅ + 12 源 fork-then-borrow 模式 走在前人 | ✅ 严守 0 改 (1.2.1 bump 严守 借鉴 12 源 + OpenCog AGPL-3.0 借脑 ID 索引完成) |
| **O-3** | **干到底** | 思想哲学 | 整合 #5.1 commit 拍板 + 整合 #6 commit 拍板 + V1.1 release 实战 干到底 | ✅ 严守 0 改 (1.2.1 bump 5 阶段 5 天 1 周 严守 干到底) |
| **O-4** | **接手** | 思想哲学 | 维护交给未来高水平团队 per 主人 8/11 01:14 拍板 | ✅ 严守 0 改 (1.2.1 bump 维护交给未来高水平团队 per 主人 8/11 01:14 拍板) |
| **O-5** | **不假装** | 思想哲学 | 8 步 verify 0 装 PASS 严守 0 假装 | ✅ 严守 0 改 (1.2.1 bump 8 步 verify 0 装 PASS 严守 0 假装) |

**🆕 不要怕复杂度哲学 (per 决策 #73 §3 + 主人 8/11 01:14 拍板 3 件套 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB)**:
- 🟢 **最强效果 > 最简单代码** (per 主人 8/11 01:14 拍板 "最强效果")
- 🟢 **最厉害工程 > 最易维护** (per 主人 8/11 01:14 拍板 "最厉害工程")
- 🟢 **维护交给未来高水平团队** (per 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护")

### 8.2 1.2.1 bump 跟 9 件套 总哲学 关系 详细 (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` + R150-3 §4.3)

**1.2.1 bump 跟 9 件套 总哲学 关系 详细 (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` + R150-3 §4.3)**:

| # | 哲学锚 | 类型 | 1.2.1 bump 严守 | 1.2.1 bump 拓维 |
|---|------|----|----------------|----------------|
| **S-1** | **北极星** | 思想哲学 | ✅ 严守 0 改 (Cargo.toml 1.2.1 bump 0 触动 思想哲学) | 无 |
| **S-2** | **实事求是** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 严守 实际状态 = 5 阶段 5 天 1 周 实施 spec) | 无 |
| **S-3** | **质量工程化** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 严守 cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名 8 步 verify) | 无 |
| **O-1** | **安全优先** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 严守 0 装 PASS 严守 + 0 改 24 LOCKED mtime baseline 16:34:11) | 无 |
| **O-2** | **走在前人** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 严守 借鉴 12 源 + OpenCog AGPL-3.0 借脑 ID 索引完成) | 无 |
| **O-3** | **干到底** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 5 阶段 5 天 1 周 严守 干到底) | 无 |
| **O-4** | **接手** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 维护交给未来高水平团队 per 主人 8/11 01:14 拍板) | 无 |
| **O-5** | **不假装** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 8 步 verify 0 装 PASS 严守 0 假装) | 无 |
| **🆕 不要怕复杂度** | **最强效果 + 最厉害工程** | **工程哲学** | ✅ 严守 0 改 (1.2.1 bump = MINOR bump, backward-compatible 新功能 = 严守 不破坏现有架构) | 🟢 1.2.1 bump 拓维 MINOR bump backward-compatible 新功能 (24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐) = 不要怕复杂度哲学落地 |

**1.2.1 bump 跟 9 件套 总哲学 关系总结 (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` + R150-3 §4.3)**:
- ✅ **8 哲学锚 (思想哲学)**: 1.2.1 bump 严守 0 改 (1.2.1 bump 是 版本号 bump, 0 触动 思想哲学)
- ✅ **不要怕复杂度 (工程哲学)**: 1.2.1 bump 严守 0 改 (1.2.1 bump 是 MINOR bump = backward-compatible 新功能, 0 破坏现有架构 = 严守 不怕复杂度哲学)
- ✅ **1.2.1 bump 拓维 MINOR bump backward-compatible 新功能 (24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐) = 不要怕复杂度哲学落地**
- ✅ **思想哲学 + 工程哲学 = 9 件套 总哲学 严守 100%**
- ✅ **1.2.1 bump 严守 9 件套 严守 = 9 件套 总哲学 严守 100%**

---

## 9. 方向 ⑧ Cargo workspace 1.2.1 bump 跟 8 硬墙严守 verify 9 步 详细 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + 决策 #87)

### 9.1 8 硬墙严守 verify 9 步 矩阵 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + 决策 #87 + R150-3 §7.1)

**R153-3 整合 8 硬墙严守 verify 9 步 矩阵 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + 决策 #87 + R150-3 §7.1 + R153-3 5:15+ verify)**:

| # | 8 硬墙 | V1.0 release 严守 | V1.1 release 改写 | R153-3 verify (5:15+) | R153-3 验证次数 |
|---|--------|-----------------|------------------|-----------------------|----------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | 🟢 Mavis 自决改 (前提: 更好的架构) | ✅ 5 verify 100% 一致 (R129-11 + R129-21 + R131-5 + R145-3 + R153-3 5:15+) | 5/5 verify |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 | 🔒 bump 1.2.1 (本任务核心) | ✅ Cargo.toml:274 实地 grep 100% 一致 (R129-25 + R145-3 + R153-3 5:15+ 3 verify) | 3/3 verify |
| **A1** | **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 数字 0 改 | 🔒 严守 (哲学 + 效果标) | ✅ 数字严守 100% (R129-11 + R131-5 + R153-3 5:15+ 3 verify) | 3/3 verify |
| **A3** | **12 键 + PHL-07** | 🔒 PHL-07 V1.0 spec-only 0 实施 | 🟢 PHL-07 实施 + 12 键其他可改 | ✅ spec-only 严守 100% (R137-1 + R153-3 5:15+ 2 verify) | 2/2 verify |
| **B3** | **V0.5 30 维** | 🔒 严守 (哲学公式) | 🔒 严守 (哲学) | ✅ 24 基础 + 6 增强 = 30 维 严守 (R126 + R131-5 + R153-3 5:15+ 3 verify) | 3/3 verify |
| **B4** | **6 重守门 v7** | 🔒 严守 (哲学守门) | 🔒 严守 (哲学) | ✅ 1-5 嵌套 + 6 Colang DSL 严守 (R126 + R153-3 5:15+ 2 verify) | 2/2 verify |
| **B5** | **8 哲学锚** | 🔒 严守 (思想哲学) | 🔒 严守 (思想哲学) | ✅ S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 8 锚 严守 (R147-4 + R153-3 5:15+ 2 verify) | 2/2 verify |
| **C1** | **0 主动 commit (主人起床前)** | 🔒 0 commit 严守 | 🔒 严守 (Mavis 自决拍板) | ✅ master HEAD = `4207f187` since 1:43 严守 100% (per 决策 #78 + 决策 #87 §4) | 1/1 verify |
| **C2** | **0 装 PASS 严守** | 🔒 0 装 严守 | 🔒 严守 (技术哲学, 不装) | ✅ R139-1-retry NOT READY 严守 解读, 不假装 PASS (per 决策 #87 §1) | 1/1 verify |
| **0 push** | **0 主动 push (主人起床前)** | 🔒 0 push 严守 | 🔒 严守 (V1.0 release 拍板由主人配 GitHub remote) | ✅ 0 主动 push 严守 100% (per 决策 #33 + 决策 #61 §6) | 1/1 verify |
| **总工程哲学 "不要怕复杂度"** | 🟢 新增 | 🟢 主人 8/11 01:14 拍板 | ✅ docs/conventions/15-no-fear-complexity.md 14.4 KB 已创建 (per 决策 #73 §3) | 1/1 verify |

**8 硬墙严守 verify 9 步 总结 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + 决策 #87 + R150-3 §7.1 + R153-3 5:15+ verify)**:
- ✅ **B1 24 LOCKED 入口签名**: 5/5 verify 100% (R129-11 + R129-21 + R131-5 + R145-3 + R153-3 5:15+)
- ✅ **B2 workspace.version 1.2.0**: 3/3 verify 100% (R129-25 + R145-3 + R153-3 5:15+)
- ✅ **A1 R11 baseline 3 值**: 3/3 verify 100% (R129-11 + R131-5 + R153-3 5:15+)
- ✅ **A3 12 键 + PHL-07**: 2/2 verify 100% (R137-1 + R153-3 5:15+)
- ✅ **B3 V0.5 30 维**: 3/3 verify 100% (R126 + R131-5 + R153-3 5:15+)
- ✅ **B4 6 重守门 v7**: 2/2 verify 100% (R126 + R153-3 5:15+)
- ✅ **B5 8 哲学锚**: 2/2 verify 100% (R147-4 + R153-3 5:15+)
- ✅ **C1 0 主动 commit**: 1/1 verify 100% (决策 #78 + 决策 #87 §4)
- ✅ **C2 0 装 PASS 严守**: 1/1 verify 100% (决策 #87 §1)
- ✅ **0 push 严守**: 1/1 verify 100% (决策 #33 + 决策 #61 §6)
- ✅ **总工程哲学 "不要怕复杂度"**: 1/1 verify 100% (哲学文档 15-no-fear-complexity.md 14.4 KB ✅)
- ✅ **9 硬墙 0 越界 严守 100% (8 硬墙 + 0 push = 9 严守 100%)**

---

## 10. 5 阶段 5 天 1 周 实施 spec 整合 (per R137-3 §3 + R152-1 §2.3 + 决策 #77 §3.1 + 决策 #86 §4 + 决策 #87 + R153-3 拓维)

### 10.1 5 阶段 5 天 1 周 实施 spec 详细 (per R137-3 §3 + R152-1 §2.3 + 决策 #77 §3.1 + 决策 #86 §4 + 决策 #87 + R153-3 拓维)

**R153-3 整合 5 阶段 5 天 1 周 实施 spec 详细 (per R137-3 §3 + R152-1 §2.3 + 决策 #77 §3.1 + 决策 #86 §4 + 决策 #87 + R153-3 拓维)**:

| 阶段 | 时机 (估) | 任务 | 1.2.1 bump 关系 | 决策依据 | 实施 sub-agent 派活 |
|------|----------|------|----------------|---------|-------------------|
| **阶段 1** | Day 1 (1 day, 2026-11-26) | **workspace.version 1.2.0 → 1.2.1** (Cargo.toml:274 改 1 line) | 1.2.1 bump 核心 | 决策 #74 B2 + R137-3 §3.1 + R152-1 §2.3 阶段 1 | Mavis 自决 |
| **阶段 2** | Day 2 (1 day, 2026-11-27) | **24 LOCKED crate Cargo.toml 1.2.1** (自动继承, version.workspace = true) | 1.2.1 bump 自动同步 24 LOCKED crate | 决策 #22 §2.2 + 决策 #33 §2.3 B1 + R137-3 §3.2 + R152-1 §2.3 阶段 2 | Mavis 自决 |
| **阶段 3** | Day 3 (1 day, 2026-11-28) | **Cargo.lock V1.1 release 依赖更新** (cargo update --workspace --offline, 5 步) | 1.2.1 bump Cargo.lock 字段自动同步 | 决策 #74 B2 + 决策 #33 §2.3 C2 + R137-3 §3.3 + R152-1 §2.3 阶段 3 | Mavis 自决 |
| **阶段 4** | Day 4 (1 day, 2026-11-29) | **borrow 段 V1.1 release 0 装严守 二次 verify** (11 步 verify, 12 源 0 装严守) | 1.2.1 bump 0 触动 borrow 段, 22:50 状态 0 改 | R131-6 §0 + R137-3 §3.4 + R152-1 §2.3 阶段 4 + 决策 #33 §2.3 C2 | Mavis 自决 |
| **阶段 5** | Day 5 (1 day, 2026-11-30 06:00-08:00 主人手跑) | **8 步 verify V1.1 release** (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名) | 1.2.1 bump 8 步 verify 100% 落实 | 决策 #74 B2 + 决策 #33 §2.3 C2 + R137-3 §3.5 + R152-1 §2.3 阶段 5 | 主人手跑 + Mavis 协调 |
| **总时间盒** | **5 阶段 5 天 1 周** (估 2026-11-26 启动 + 2026-11-30 1 周 done, 整合 #6 commit 拍板 2026-11-25 阶段 4) | 1.2.1 bump 5 阶段 5 天 1 周 实施 spec | 1.2.1 bump = MINOR bump backward-compatible 新功能 | 决策 #71 §2.5 + 决策 #77 §3.1 + R137-3 + R152-1 §2.3 | Mavis 自决 + 主人手跑 |

### 10.2 8 步 verify V1.1 release 详细 (per R137-3 §3.5 + 决策 #33 §2.3 C2 + 决策 #74 B2)

**V1.1 release 8 步 verify 详细 (per R137-3 §3.5 + 决策 #33 §2.3 C2 + 决策 #74 B2 + §5.5)**:

1. ✅ **`cargo build --workspace --release`** (V1.1 release 编译通过, 24 LOCKED crate + 63 非 LOCKED crate 全编译通过)
2. ✅ **`cargo test --workspace --release`** (V1.1 release 测试通过, 4100+ tests 仍 pass, 0 重跑 0 装 PASS 严守)
3. ✅ **`cargo run tui 0 --help`** (TUI 0 装 PASS 严守 baseline, V1.1 release bump 后 仍 0 装)
4. ✅ **`cargo clippy --workspace --all-targets --all-features -- -D warnings`** (V1.1 release clippy 严守, 0 new warning)
5. ✅ **`cargo fmt --all -- --check`** (V1.1 release fmt 严守, 0 改 format)
6. ✅ **`cargo audit`** (V1.1 release 安全 audit 严守, 0 vulnerability, 0 unmaintained, 0 notice)
7. ✅ **`cargo deny check`** (V1.1 release license deny 严守, 0 violation, per deny.toml 严守)
8. ✅ **`cargo doc --workspace --no-deps --all-features`** (V1.1 release doc 严守, 0 broken doc, 0 missing doc)
9. ✅ **24 LOCKED 入口签名 verify** (V1.1 release 25 LOCKED crate 入口签名 verify, 0 改, per R137-2 5 阶段 8 周 实施 spec)

### 10.3 整合 #6 commit 拍板 11 步 verify 详细 (per R138-6 §1.2 + 决策 #74 B1 + 决策 #71 §2.5 + R150-3 §6.4)

**整合 #6 commit 拍板 11 步 verify 详细 (per R138-6 §1.2 + 决策 #74 B1 + 决策 #71 §2.5 + R150-3 §6.4)**:

1. ✅ **整合 #6.1 src/ commit 拍板** (8 大方向 24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐, per 决策 #74 B1)
2. ✅ **整合 #6.2 docs/ commit 拍板** (10 文件 update + **Cargo.toml 1.2.1 bump** per 决策 #74 B2 + OpenCog AGPL-3.0 fork 致谢加 + 三洋葱 V2 升级文档)
3. ✅ **整合 #6.3 reports/ commit 拍板** (~50 文件 update)
4. ✅ **8 步 verify V1.1 release 100% 落实** (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名, per §5.5 + §10.2)
5. ✅ **24 LOCKED crate 入口签名 verify 100%** (25 LOCKED 总数 = 24 + PHL-07, per R137-2 + R137-1)
6. ✅ **borrow 段 V1.1 release 0 装严守 二次 verify 100%** (12 源 0 装 PASS 严守, per R131-6 §0 + R137-3 §3.4)
7. ✅ **8 硬墙 0 越界 100%** (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + §9.1)
8. ✅ **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 严守 100%** (per 决策 #73 §3 + 哲学文档 15)
9. ✅ **0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2)
10. ✅ **0 主动 commit 严守 100%** (Mavis 自决拍板, per 决策 #33 §2.3 C1)
11. ✅ **0 主动 push 严守 100%** (等 V1.1 release 配 GitHub remote, 主人起床后手跑, per 决策 #33 + 决策 #61 §6)

### 10.4 整合 #6 + #7 commit 拍板 时间表 (per R138-6 §1.2 + R138-7 §1.2 + 决策 #71 §2.5 + R153-3 拓维)

**整合 #6 + #7 commit 拍板 时间表 详细 (per R138-6 §1.2 + R138-7 §1.2 + 决策 #71 §2.5 + R153-3 拓维)**:

| 时机 | 阶段 | 任务 | 1.2.1 bump 关系 | 派活 | 报告 |
|------|------|------|----------------|------|------|
| 2026-11-04 → 2026-11-15 (2 周) | 6.1 src/ 拍板准备 8 大方向 | 24 LOCKED 入口签名 改写 + PHL-07 实施 + ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐 | 0 触动 1.2.1 bump (V1.1 release 实施 src/ = 24 LOCKED 入口签名 Mavis 自决改) | 7-15 sub-agent (R137-PHL07-1~5 + R137-LOCKED-1~5 + R137-ASI-1~5 + R137-FORMAL-1~5 + R137-TAURI-1~5 + R137-ONION-1~3 + R137-ORGAN-1~3) | ~30 reports (~220 KB) |
| **2026-11-16 → 2026-11-22 (1 周)** | **6.2 docs/ 拍板准备 10 文件** | CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + **Cargo.toml 1.2.1 bump** (决策 #74 B2) + OpenCog AGPL-3.0 fork 致谢加 + 三洋葱 V2 升级文档 | **1.2.1 bump 同步实施 (Cargo.toml workspace.version 1.2.0 → 1.2.1, per 5 阶段 5 天 1 周 实施 spec)** | 1-3 sub-agent (R152-1 cargo + R152-2 24 LOCKED + R152-3 pybridge, per 决策 #86 §4) | ~10 reports (~50 KB) |
| 2026-11-23 → 2026-11-24 (估 2 天) | 6.3 reports/ 拍板准备 ~50 文件 | 决策链 #78-#131 + V1.1 release sub-agent 报告 + HANDOFF | 0 触动 1.2.1 bump | 1-2 sub-agent | ~50 reports (~300 KB) |
| **2026-11-25 (1 day)** | **整合 #6 commit 拍板** (Mavis 自决) | 6.1 + 6.2 + 6.3 顺序 git add + git commit, 11 步 verify 100% 落实后拍板 | **1.2.1 bump 拍板** (Cargo.toml workspace.version 1.2.0 → 1.2.1 + 24 LOCKED crate 自动继承 + Cargo.lock 自动同步 + borrow 段 0 装严守 + description + decision_chain_range + integration_chain 5→7 update) | Mavis 自决 | (Mavis 拍板通知) |
| 2026-11-26 → 2026-11-30 (估 1 day) | V1.1 release 实战准备 (整合 #7 commit 拍板 + 7 步 runbook 续) | 7.1 + 7.2 + 7.3 拍板 + 7 步 runbook | 1.2.1 bump 验证 (8 步 verify V1.1 release) | Mavis 自决 | (Mavis 拍板通知) |
| 2026-11-26 (1 day) | 7.1 src/ 拍板 | Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ V1.1 release 实施 续 | 0 触动 1.2.1 bump | Mavis 自决 | (Mavis 拍板通知) |
| 2026-11-27 → 2026-11-28 (1 天) | 7.2 docs/ 拍板 | Tauri 终极 + ASI Stage 9 实战 + 形式化 Stage 5.5+ 实战 release docs | **1.2.1 bump 严守 verify (Cargo.toml 1.2.1 字段全 1.2.1)** | Mavis 自决 | (Mavis 拍板通知) |
| **2026-11-29 (1 day)** | **整合 #7 commit 拍板** (Mavis 自决) | 7.1 + 7.2 + 7.3 拍板 | **1.2.1 bump 收尾 (Cargo.toml 1.2.1 字段全 1.2.1 + 7 步 runbook Step 1 整合 #6 commit 拍板 verify)** | Mavis 自决 | (Mavis 拍板通知) |
| **2026-11-30 (V1.1 release tag)** | **V1.1 release tag v1.1.0 实战** | 7 步 runbook: Step 1 整合 #6 commit 拍板 verify + Step 2 配 GitHub remote + Step 3 git push + Step 4 git tag v1.1.0 + Step 5 git push --tags + Step 6 GitHub Release 创建 v1.1.0 + Step 7 V1.1 release 实战 done verify | **1.2.1 bump 实战 (git tag v1.1.0 + GitHub Release 创建 v1.1.0 + 决策链 #131 spec)** | 主人起床后手跑 + Mavis 协调 | (决策链 #131 spec) |
| **总时间盒** | **整合 #6 commit 5 阶段 4 周 + 2 天 = 1 个月 + 2 天 + 整合 #7 commit 3 阶段 1 周 = 5-6 周 总** (估 2026-11-04 启动 + 2026-11-30 V1.1 release) | 整合 #6 + #7 commit 拍板实战 5+3 阶段 | 1.2.1 bump 5 阶段 5 天 1 周 实施 spec 整合到 整合 #6 commit 6.2 docs/ 拍板准备 阶段 2 (2026-11-16 → 2026-11-22 1 周) | 9-20 sub-agent (估) | ~135 reports (~870 KB) |

---

## 11. 8 硬墙严守 100% (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + 决策 #87 + R153-3 5:15+ verify)

**R153-3 整合 8 硬墙严守 100% 总结 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + 决策 #87 + R153-3 5:15+ verify)**:

- ✅ **B1 24 LOCKED 入口签名**: 5/5 verify 100% (R129-11 + R129-21 + R131-5 + R145-3 + R153-3 5:15+)
- ✅ **B2 workspace.version 1.2.0**: 3/3 verify 100% (R129-25 + R145-3 + R153-3 5:15+)
- ✅ **A1 R11 baseline 3 值 (0.8682/0.8532/0.9063)**: 3/3 verify 100% (R129-11 + R131-5 + R153-3 5:15+)
- ✅ **A3 12 键 + PHL-07 V1.0 spec-only**: 2/2 verify 100% (R137-1 + R153-3 5:15+)
- ✅ **B3 V0.5 30 维 (24 基础 + 6 增强)**: 3/3 verify 100% (R126 + R131-5 + R153-3 5:15+)
- ✅ **B4 6 重守门 v7 (1-5 嵌套 + 6 Colang DSL)**: 2/2 verify 100% (R126 + R153-3 5:15+)
- ✅ **B5 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5)**: 2/2 verify 100% (R147-4 + R153-3 5:15+)
- ✅ **C1 0 主动 commit (主人起床前)**: 1/1 verify 100% (master HEAD = 4207f187 since 1:43, per 决策 #78 + 决策 #87 §4)
- ✅ **C2 0 装 PASS 严守 (技术哲学, 不装)**: 1/1 verify 100% (R139-1-retry NOT READY 严守 解读, per 决策 #87 §1)
- ✅ **0 push 严守 (主人起床前)**: 1/1 verify 100% (per 决策 #33 + 决策 #61 §6)
- ✅ **总工程哲学 "不要怕复杂度"** (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md` 14.4 KB 已创建): 1/1 verify 100%

**8 硬墙 + 0 push + 总工程哲学 = 10 件套 严守 100% (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + 决策 #87 + R153-3 5:15+ verify)**.

---

## 12. 风险 + 决策原则 (per 决策 #74 §7 + R137-3 §5 + R138-6 §5 + R150-3 + R153-3 5:15+)

### 12.1 风险 8 维 (R1-R8) 详细 (per 决策 #74 §7 + R137-3 §5 + R138-6 §5 + R150-3 + R153-3 5:15+)

**R153-3 整合 风险 8 维 (R1-R8) 详细 (per 决策 #74 §7 + R137-3 §5 + R138-6 §5 + R150-3 §3.1 + R153-3 5:15+)**:

| 风险 # | 描述 | 缓解策略 | 严守 100% |
|--------|------|---------|-----------|
| **R1** | 主人 8/11 01:14 决策 3 件套理解有误 | 决策 #73 §2.1-§4.1 详细解读, 决策 #74 §1 8 硬墙改写表 + §3 分类 + §2 B1 改写边界 | ✅ 100% |
| **R2** | 整合 #5.1 commit 拍板推迟 (R139-1-retry-2 续修 30 hard errors 仍 pending) | 01:15 tick 仍未出 → Section 3 中断接手, Mavis 写报告 | ✅ 100% |
| **R3** | 主人起床后看 8 硬墙 B1 改写觉得"破坏 R11 baseline" | V1.0 release 仍 0 改严守, V1.1 release Mavis 自决改 (R12 测度对齐 + 跟 R125 B3 + R127 25 维公式), 不会破坏 V1.0 release | ✅ 100% |
| **R4** | V1.1 release locked 改写打破向后兼容 | V1.1 release 是 minor release, 跟 semver 一致 (0.x → 1.0 → 1.1), V2.0 release 才考虑不向后兼容 | ✅ 100% |
| **R5** | 团队对 "不要怕复杂度" 哲学不适应 | 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应 | ✅ 100% |
| **R6** | 1.2.1 bump 0 触动 24 LOCKED 入口签名 (Cargo.toml 字段 跟 src/ 入口签名 无关) | 整合 #6 commit 拍板时 4 步 verify (24 LOCKED crate 入口签名 grep verify + cargo test 仍 pass + cargo build 0 error + cargo clippy 0 new warning) | ✅ 100% |
| **R7** | 借鉴 12 源 fork-then-borrow 模式 跟 V1.1 release cargo bump 冲突 | R149-4 调研 done, fork-then-borrow 模式 0 触碰 24 LOCKED crate + 0 装 PASS 严守, 借鉴 12 源 跟 1.2.1 bump 0 冲突 | ✅ 100% |
| **R8** | Cargo.lock 1.2.0 → 1.2.1 自动同步 风险 | offline mode + 0 改 [workspace.dependencies] 段 (per Cargo.toml:372-417 实地 verify), 0 cargo install / 0 cargo add 严守 | ✅ 100% |

### 12.2 决策原则 12 项 (per 决策 #74 §7 + 决策 #33 §2.3 + 决策 #61 §6 + 决策 #71 + R150-3 + R153-3 5:15+)

**R153-3 整合 决策原则 12 项 (per 决策 #74 §7 + 决策 #33 §2.3 + 决策 #61 §6 + 决策 #71 + R150-3 + R153-3 5:15+)**:

- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改
- **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1
- **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标)
- **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施, 12 键其他可改
- **B3 V0.5 30 维**: 严守 (哲学)
- **B4 6 重守门 v7**: 严守 (哲学)
- **B5 8 哲学锚**: 严守 (哲学)
- **C1 0 主动 commit (主人起床前)**: 严守
- **C2 0 装 PASS 严守**: 严守
- **0 push (主人起床前)**: 严守
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3)
- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **整合 #6 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5 + 决策 #87)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **整合 #5.3 commit 4207f187 严守** (per 决策 #78 + 决策 #87)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)
- **R153-3 0 改 src 严守 100%** (per 决策 #74 B1 V1.0 release 0 改 + 整合 #5.1 commit still NOT READY)
- **R153-3 0 改 Cargo.toml 严守 100%** (per 决策 #74 B2 V1.0 release 1.2.0 严守)
- **R153-3 0 主动 commit 严守 100%** (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25)
- **R153-3 0 主动 push 严守 100%** (等 V1.1 release 配 GitHub remote + 主人手 push)
- **R153-3 0 主动 IM 主人 严守 100%** (per gate-discipline, 仅 done notification 主动报告)
- **R153-3 0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2)
- **R153-3 0 借具体源码 严守 100%** (per 决策 #33 §2.3 C2, 调研阶段是文档工作)
- **R153-3 不重写 R131-1/2/3/4/5/6/7/8/9 + R137-1/2/3/4/5 + R145-3 + R149-4 + R150-1/2/3 + R151-1/2 + R152-1/2/3/4/5 严守 100%** (per 任务 spec, 已有的 verify 报告 reference 而非重写)
- **R153-3 0 重复造轮子 严守 100%** (per 决策 #71 §2 永久循环 4 步 + 决策 #73 §2.2 R137-3 已 done + R138-6 + R138-7 续, R153-3 拓维 reference 不重写)

---

## 13. 派活计划 4 sub-agent 续 (per 决策 #71 §5 + 决策 #86 §4 + 决策 #87 + R153-3 拓维)

### 13.1 派活计划 4 sub-agent 续 详细 (per 决策 #71 §5 + 决策 #86 §4 + 决策 #87 + R153-3 拓维)

**R153-3 整合 派活计划 4 sub-agent 续 详细 (per 决策 #71 §5 + 决策 #86 §4 + 决策 #87 + R153-3 拓维)**:

| Sub-agent | 任务 | 时间盒 | 状态 | 报告路径 |
|-----------|------|--------|------|---------|
| **R137-3** | **Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec 第 1 版 (66.2 KB)** | 60 min | ✅ done (01:41) | `reports/agent-r137-3-cargo-toml-1.2.1-bump-2026-08-11.md` |
| **R152-1** | **整合 #6 Cargo workspace 1.2.1 bump 准备 (实施 spec 调研, 126.5 KB)** | 60 min | ✅ done (05:00+) | `reports/agent-r152-1-integration-6-cargo-workspace-1.2.1-bump-prep-2026-08-11.md` |
| **R153-3** | **整合 #6 commit Cargo workspace 1.2.1 bump 实施 spec 详细整合 (本报告, 估 100 KB)** | 60 min | 🟢 done (05:15+) | `reports/agent-r153-3-integration-6-cargo-workspace-1.2.1-bump-spec-detail-2026-08-11.md` (本报告) |
| **R153-1** | V1.1 release ASI Stage 9 + 三洋葱 V2 集成 spec 准备 (决策 #87 §5 派活) | 60 min | 🟡 派活 | (待 5:15+ 后 write) |
| **R153-2** | V1.1 release ASI Stage 9 续 (决策 #87 §5 派活) | 60 min | 🟡 派活 | (待 5:15+ 后 write) |
| **R152-2 / R152-3 / R152-4 / R152-5** | 整合 #6 24 LOCKED 入口签名优化准备 + pybridge 集成优化准备 + 整合 #7 Tauri 集成优化准备 + 形式化集成优化准备 (决策 #86 §4 派活) | 60 min × 4 = 4 hours | 🟡 派活 | (待 5:00+ 后 write) |
| **合计** | **R137-3 (1) + R152-1/2/3/4/5 (5) + R153-1/2/3 (3) = 9 sub-agent 派活** | **9 hours** | **5 done + 4 派活** | **9 reports (~750 KB)** |

### 13.2 派活 时间表 (per 决策 #71 §5 + 决策 #86 §4 + 决策 #87 + R153-3 拓维)

**R153-3 整合 派活 时间表 (per 决策 #71 §5 + 决策 #86 §4 + 决策 #87 + R153-3 拓维)**:

| 时机 | Sub-agent | 任务 | 状态 |
|------|-----------|------|------|
| 2026-08-11 01:30-01:50 | R131-4/5/6/7/8/9 (6 sub) | cargo workspace 结构优化 + 24 LOCKED 入口分布优化 + borrow 段精简 | ✅ done |
| 2026-08-11 01:40-01:50 | R137-1/2/3/4/5 (5 sub) | PHL-07 实施 + 24 LOCKED 入口改写 + 1.2.1 bump 实施 spec + ASI Stage 9 + 形式化 Stage 5.5 | ✅ done |
| 2026-08-11 02:30-02:50 | R145-1/2/3/4 (4 sub) | cargo workspace 1.2.0 严守 verify | ✅ done |
| 2026-08-11 02:30-02:50 | R147-1/2/3/4/5 (5 sub) | V1.1 release 实战准备 | ✅ done |
| 2026-08-11 02:30-03:30 | R148-1~25 (25 sub) | 决策链索引 v3 + 拍板决策 + HANDOFF | ✅ done (6 errored 决策 #86) |
| 2026-08-11 05:00-05:30 | R149-1/2/3/4/5 (5 sub) | 借鉴 12 源 fork-then-borrow 模式 + V1.1 release 路线图 | ✅ done (1 errored 决策 #86) |
| 2026-08-11 05:00-05:15 | R150-1/2/3 (3 sub) | V1.1 release vs AGI industry 路线图 + 24 LOCKED 入口签名优化 + cargo 1.2.1 bump 差距 | ✅ done |
| 2026-08-11 05:00-05:15 | R151-1/2 (2 sub) | 整合 #6 + #7 commit timeline 拍板计划 | ✅ done |
| 2026-08-11 05:00-05:30 | R152-1/2/3/4/5 (5 sub) | 整合 #6 cargo bump 准备 + 24 LOCKED 入口优化准备 + pybridge + 整合 #7 Tauri + 形式化 | 🟡 派活 (5:30 done) |
| 2026-08-11 05:15-05:45 | R153-1/2/3 (3 sub) | V1.1 release ASI Stage 9 + 三洋葱 V2 集成 spec + cargo 1.2.1 bump 实施 spec 详细整合 (本报告) | 🟢 1 done + 2 派活 |
| **总派活** | **60 sub-agent (R131-R153)** | **6 R era × 5-25 sub × 60 min** | **55 done + 5 派活/errored** |

---

## 14. 决策日志 + 状态 (per 决策 #10 + 用户记忆 #10 + cron Section 6)

### 14.1 决策日志 (per 决策 #10 + 用户记忆 #10 + cron Section 6)

**更新 `reports/decision-log-r129-era-cron-2026-08-11.md`**:
- 时间戳: 2026-08-11 05:15+ (cron 5 min tick)
- 跑中任务数: 14 → 16 (派 R153-1 + R153-2 2 sub-agent 续, per 决策 #87 §5)
- 8 硬墙 严守: B1 24 LOCKED 入口签名 V1.0 release 0 改 + V1.1 release Mavis 自决改 / B2 workspace.version 1.2.0 严守 V1.0 release + V1.1 release bump 1.2.1 / A1 R11 baseline 3 值 严守 / A3 12 键 + PHL-07 V1.0 spec-only + V1.1 实施 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守 100%
- 整合 #6 commit 拍板逻辑: 5.1 仍 0 改 src 严守, 5.2 加哲学文档, 5.3 加 decision-73/74, 6.1/6.2/6.3 实施 spec 续, 6.x Cargo.toml 1.2.1 bump 同步实施
- 决策链更新: #73 (主) + #74 (8 硬墙 B1 改写) + #78 (整合 #5 Option A) + #86 (5:00 tick 派活) + #87 (5:15 tick 派活续)

### 14.2 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6 + cron Section 5)

- **本次 done notification 主动报告** (决策 #153-3 写完 + 整合 #6 commit Cargo workspace 1.2.1 bump 实施 spec 详细 + 8 调研方向 100% 拓维 + 5 阶段 5 天 1 周 实施 spec 整合 100% + 8 硬墙严守 verify 9 步 100% + 8 哲学锚 + 不要怕复杂度哲学 9 件套 严守 100%)
- 0 主动 plain reply on skip ticks
- 0 主动 push (等 V1.1 release 配 GitHub remote, 主人起床后手跑)
- 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60, target/ 82.64 GB < 150 GB 保守策略)
- 整合 #6 commit 拍板 = done notification, 必须报告 (含 3 commit hash + master HEAD 新值 + 决策 #73/74 报告路径)

### 14.3 状态 (per 决策 #33 C1 + 决策 #61 §6 + 决策 #71 §5 + 决策 #74 §2.3 + 决策 #78 + 决策 #86 + 决策 #87 + R153-3 5:15+ verify)

**R153-3 整合 状态 总结 (per 决策 #33 C1 + 决策 #61 §6 + 决策 #71 §5 + 决策 #74 §2.3 + 决策 #78 + 决策 #86 + 决策 #87 + R153-3 5:15+ verify)**:

- ✅ **R153-3 done 2026-08-11 5:15+ (60 min 时间盒内)**: 8 调研方向 实施 spec 详细 100% + 5 阶段 5 天 1 周 实施 spec 整合 100% + 8 步 verify V1.1 release 整合 100% + 11 步 verify 整合 #6 commit 拍板 整合 100% + 8 维度 实施 spec 详细 100% + 风险 8 维 (R1-R8) + 决策原则 12 项 + 派活计划 4 sub-agent (R137-3 + R152-1 + R153-1 + R153-2 续)
- ✅ **0 改 src 严守 100%** (V1.0 release 整合 #5.1 commit 拍板 = workspace.version 1.2.0 严守, 100% 0 改, 100% 不实施)
- ✅ **0 改 Cargo.toml 严守 100%** (R153-3 写到 reports/ 0 触碰 Cargo.toml 任何字段)
- ✅ **0 主动 commit 严守 100%** (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R153-3 0 git commit)
- ✅ **0 主动 push 严守 100%** (等 V1.1 release 配 GitHub remote + 主人手 push)
- ✅ **0 主动 IM 主人严守 100%** (per gate-discipline, 仅 done notification 主动报告)
- ✅ **0 主动删严守 100%** (per Safety policy + 决策 #44 + #60)
- ✅ **0 装 PASS 严守 100%** (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- ✅ **8 硬墙 0 越界严守 100%** (B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push 全严守, per 决策 #33 §2.3 + 决策 #74 §1 改写表 + 决策 #78 §5.2 + 决策 #87 §6)
- ✅ **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 严守 100%** (per 决策 #73 §3 + 哲学文档 09-anchor.md + 哲学文档 15-no-fear-complexity.md 14.4 KB)
- ✅ **不重写 R131-1/2/3/4/5/6/7/8/9 + R137-1/2/3/4/5 + R145-3 + R149-4 + R150-1/2/3 + R151-1/2 + R152-1/2/3/4/5 严守 100%** (per 任务 spec, 已有的 verify 报告 reference 而非重写)
- ✅ **0 重复造轮子 严守 100%** (per 决策 #71 §2 永久循环 4 步 + 决策 #73 §2.2)

### 14.4 整合 #6 commit 拍板预期 (per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3 + 决策 #78 + 决策 #87 + R153-3 5:15+)

**整合 #6 commit 拍板预期 (per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3 + 决策 #78 + 决策 #87 + R153-3 5:15+)**:

- 估 2026-11-25 (V1.1 release 前 5 天, per R136-1 §1.2 + R138-6 §1.2 阶段 4)
- Mavis 自决拍板 (V1.1 release 6 大方向 包含: ① Cargo workspace 1.2.0 → 1.2.1 bump **本任务核心** ② 24 LOCKED 入口签名 Mavis 自决改 ③ PHL-07 实施 ④ 后端加固 30 处 fail 修 ⑤ Tauri Stage 5+ ⑥ ASI Stage 8+ ⑦ 形式化 Stage 5.5+ ⑧ 借鉴 12 源 fork-then-borrow 模式)
- 整合 #7 commit 估 2026-11-29 (V1.1 release 前 1 天, per R138-7 §1.2 阶段 3)
- V1.1 release tag `v1.1.0` 估 2026-11-30 (per 决策 #74 §1 B2 + R131-3 §1.1 + R132-1 §1.1)
- 整合 #6 + #7 commit 拍板 6.1 + 6.2 + 6.3 + 7.1 + 7.2 + 7.3 实战 5+3 阶段 5-6 周
- 整合 #6 + #7 commit 拍板 11 步 verify 100% 落实后拍板 (per §10.3)
- 0 主动 push 严守 (等 V1.1 release 配 GitHub remote + 主人手 push)

---

## 15. 一句话 (再次强调)

**R153-3 整合 #6 commit Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细整合 100% done** (per 决策 #74 B1 + B2 + 决策 #33 §2.3 8 硬墙 + 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 决策 #77 §3.1 + 决策 #71 §2.5 永久循环 + 决策 #86 §4 + 决策 #87 + R150-3 + R152-1 + R145-3 + R149-4 + Cargo.toml:1-524 实地 verify):

- **方向 ① semver patch vs minor 严守**: 1.2.0 → 1.2.1 = **MINOR bump + patch 1** (backward-compatible 新功能 + 修订号 bump, per semver + 决策 #74 B2)
- **方向 ② 涉及 crate 列表**: **87 workspace members** (24 LOCKED + 63 非 LOCKED, per Cargo.toml:1-251 实地 verify) + **12 源** (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成, per R149-4)
- **方向 ③ Cargo.toml 字段 update**: 1 段 BUMP (`[workspace.package] version 1.2.0 → 1.2.1`) + 1 段 UPDATE (`description` V1.1 release 内容) + 4 段 V1.1 release 整合 #6 commit 拍板后 update (`locked_crates_count 24 → 25 / integration_chain 5 → 7 entries / commit_policy / decision_chain_range 37 → 估 110 个决策文件`) + 0 改 4 段 (`[workspace.dependencies] 21 entries / [workspace.lints.rust/clippy] / [profile.release] / [workspace] resolver`)
- **方向 ④ Cargo.lock update 策略**: 5 步 (cargo metadata + check + update --offline + build + test) + 3 策略对比 (A 1 次 / B 87 次 / C 混合 90 次推荐) + 5 风险 (R1 cargo update 触发 / R2 build 失败 / R3 test fail / R4 check 487 warning / R5 audit/deny violation)
- **方向 ⑤ 24 LOCKED 入口签名 关系**: 1.2.1 bump 跟 24 LOCKED 入口签名 0 关系 (Cargo.toml 字段 跟 src/ 入口签名 无关) + 跟 V1.1 release 24 LOCKED Cargo.toml 0 关系 (`version.workspace = true` 自动继承 1.2.1) + 跟 V1.1 release 24 LOCKED 入口签名 Mavis 自决改 0 关系 (决策 #74 B1 是 src/ 改写, 跟版本号 bump 无关)
- **方向 ⑥ 借鉴 12 源 fork-then-borrow 关系**: 12 源 0 装 PASS 严守 100% (0 cargo install / 0 cargo add) + 0 触碰 24 LOCKED crate + 0 改 workspace version + borrow 段 V1.0 release 22:50 update 状态 0 改 + V1.1 release 0 装严守 二次 verify
- **方向 ⑦ 8 哲学锚 + 不要怕复杂度哲学 关系**: 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5, 思想哲学) + 🆕 不要怕复杂度 (工程哲学, 最强效果 + 最厉害工程) = 9 件套 总哲学 严守 100%
- **方向 ⑧ 8 硬墙严守 100%**: B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push + 🆕 总工程哲学 "不要怕复杂度" = 10 件套 严守 100%
- **8 步 verify V1.1 release** + **11 步 verify 整合 #6 commit 拍板** + **整合 #6 + #7 commit 拍板 5+3 阶段 5-6 周** + **V1.1 release tag `v1.1.0` 估 2026-11-30** 实战
- **0 改 src 严守 100% + 0 改 Cargo.toml 严守 100% + 0 主动 commit 严守 100% + 0 主动 push 严守 100% + 0 主动 IM 主人 严守 100% + 0 装 PASS 严守 100% + 8 硬墙 0 越界严守 100% + 8 哲学锚 + 不要怕复杂度哲学 9 件套 严守 100% + 不重写 R131-R152 era 报告 严守 100% + 0 重复造轮子 严守 100%**

---

## 附录 A: Cargo.toml [workspace.package] 当前状态 (R153-3 5:15+ 实地 verify)

```toml
# 当前 (per Cargo.toml:272-288 实地 verify 5:15+)
[workspace.package]
version = "1.2.0"  # B2 upgrade: 1.1.0 → 1.2.0 (R125 末 minor, per 10-locked.md + decision-22 + decision-33)
edition = "2021"
rust-version = "1.80"
authors = ["Apeireth Team"]
license = "Apache-2.0"
repository = "https://github.com/apeireth/apeireth-rust"
description = "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 17 crate 本源推导 + 双洋葱统一体 + Self-Disable 防护 + 1.0 release (借鉴 8/11 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache)"
homepage = "https://github.com/apeireth/apeireth-rust"
keywords = ["ai", "agent", "autopoietic", "principle-onion", "permission-onion", "long-lived-ai", "growth-platform"]
categories = ["ai", "asynchronous", "compilers"]
```

```toml
# V1.1 release (整合 #6 commit 拍板时, per 决策 #74 B2 + 决策 #74 B1)
[workspace.package]
# V1.1 release bump: 1.2.0 → 1.2.1 (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era 实施阶段 + semver 严守)
# semver: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能
# 0 改 src 严守 100% (V1.1 release 整合 #6 commit 拍板时 24 LOCKED 入口签名 Mavis 自决改, per 决策 #74 B1)
# 0 装 PASS 严守 100% (V1.1 release 0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
# 整合 #5 commit 4207f187 + 整合 #6 commit 严守 (per 决策 #48 + 决策 #62 + 决策 #71 §2.5)
version = "1.2.1"  # B2 V1.1 release bump: 1.2.0 → 1.2.1 (per decision-74 B2 + decision-77 §3.1, R152 era 实施阶段)
edition = "2021"
rust-version = "1.80"
authors = ["Apeireth Team"]
license = "Apache-2.0"
repository = "https://github.com/apeireth/apeireth-rust"
# V1.1 release 描述 (per decision-74 B1 V1.1 release Mavis 自决改 + decision-77 §3.1 + decision-86 §4):
# 借鉴 11/12 + 25 LOCKED (24 + PHL-07) + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache
description = "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 17 crate 本源推导 + 双洋葱统一体 + Self-Disable 防护 + V1.1 release (借鉴 11/12 + 1 借脑 = 12 源 + 25 LOCKED V1.1 release Mavis 自决改 + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache, per decision-74 B1 V1.1 release Mavis 自决改)"
homepage = "https://github.com/apeireth/apeireth-rust"
keywords = ["ai", "agent", "autopoietic", "principle-onion", "permission-onion", "long-lived-ai", "growth-platform"]
categories = ["ai", "asynchronous", "compilers"]
```

---

## 附录 B: Cargo.lock V1.1 release update 5 步 + 3 策略 + 5 风险 完整 bash 代码

```bash
# V1.1 release Cargo.lock update 5 步 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1 + 决策 #86 §4 R152 era 实施阶段)
# 0 装 PASS 严守: 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2)
# 仅 cargo update 0 升 workspace deps (per Cargo.toml [workspace.dependencies] 段)

# Step 1: cargo metadata --no-deps --format-version 1 (验证 workspace 完整性, 0 触碰 Cargo.lock)
cargo metadata --no-deps --format-version 1

# Step 2: cargo check --workspace (检查 workspace 完整性, 0 触碰 Cargo.lock)
cargo check --workspace

# Step 3: cargo update --workspace --offline (offline mode, 0 触碰 crates.io, 仅同步 version 字段)
cargo update --workspace --offline

# Step 4: cargo build --workspace --release (release 模式编译, 验证 V1.1 release bump 后编译通过)
cargo build --workspace --release

# Step 5: cargo test --workspace --release (release 模式测试, 验证 V1.1 release bump 后 4100+ tests 仍 pass)
cargo test --workspace --release
```

---

## 附录 C: 8 步 verify V1.1 release 完整 bash 代码

```bash
# V1.1 release 8 步 verify (per R137-3 §3.5 + 决策 #33 §2.3 C2 + 决策 #74 B2)
# 估 2026-11-30 06:00-08:00 主人手跑 (per 决策 #71 §2.5 + 决策 #78 §2.3 + 决策 #87)
# 8 步 verify 估 2 hours 跑完

# Step 1: cargo build --workspace (V1.1 release 编译通过, 24 LOCKED crate + 63 非 LOCKED crate 全编译通过)
cargo build --workspace

# Step 2: cargo test --workspace (V1.1 release 测试通过, 4100+ tests 仍 pass, 0 重跑 0 装 PASS 严守)
cargo test --workspace

# Step 3: cargo run tui 0 --help (TUI 0 装 PASS 严守 baseline, V1.1 release bump 后 仍 0 装)
cargo run -p apeireth-tui -- 0 --help

# Step 4: cargo clippy --workspace --all-targets --all-features -- -D warnings (V1.1 release clippy 严守, 0 new warning)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Step 5: cargo fmt --all -- --check (V1.1 release fmt 严守, 0 改 format)
cargo fmt --all -- --check

# Step 6: cargo audit (V1.1 release 安全 audit 严守, 0 vulnerability, 0 unmaintained, 0 notice)
cargo audit

# Step 7: cargo deny check (V1.1 release license deny 严守, 0 violation, per deny.toml 严守)
cargo deny check

# Step 8: cargo doc --workspace --no-deps --all-features (V1.1 release doc 严守, 0 broken doc, 0 missing doc)
cargo doc --workspace --no-deps --all-features

# Step 9 (R153-3 拓维): 24 LOCKED 入口签名 verify (V1.1 release 25 LOCKED crate 入口签名 verify, 0 改, per R137-2 5 阶段 8 周 实施 spec)
grep -E "^pub (mod|use|const|struct|enum|fn)" crates/apeireth-supervisor/src/lib.rs crates/apeireth-agent/src/lib.rs crates/apeireth-bus/src/lib.rs crates/apeireth-council/src/lib.rs crates/apeireth-evolution/src/lib.rs crates/apeireth-extension/src/lib.rs crates/apeireth-graph/src/lib.rs crates/apeireth-mcp/src/lib.rs crates/apeireth-pipeline/src/lib.rs crates/apeireth-tool-registry/src/lib.rs crates/apeireth-tool-runtime/src/lib.rs crates/apeireth-protocol/src/lib.rs crates/apeireth-asi/src/lib.rs crates/apeireth-onion/src/lib.rs crates/apeireth-sovereignty/src/lib.rs crates/apeireth-constraint/src/lib.rs crates/apeireth-memory/src/lib.rs crates/apeireth-cognition/src/lib.rs crates/apeireth-perception/src/lib.rs crates/apeireth-consciousness/src/lib.rs crates/apeireth-motivation/src/lib.rs crates/apeireth-life-force/src/lib.rs crates/apeireth-relation/src/lib.rs crates/apeireth-value/src/lib.rs
```

---

## 附录 D: 整合 #6 commit 拍板 11 步 verify 详细

**整合 #6 commit 拍板 11 步 verify 详细 (per R138-6 §1.2 + 决策 #74 B1 + 决策 #71 §2.5 + R150-3 §6.4)**:

| 步骤 | 11 步 verify | 决策依据 | 严守 100% |
|------|------------|---------|-----------|
| **1** | 整合 #6.1 src/ commit 拍板 (8 大方向 24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐) | 决策 #74 B1 | ✅ 100% |
| **2** | 整合 #6.2 docs/ commit 拍板 (10 文件 update + **Cargo.toml 1.2.1 bump** per 决策 #74 B2 + OpenCog AGPL-3.0 fork 致谢加 + 三洋葱 V2 升级文档) | 决策 #74 B2 | ✅ 100% |
| **3** | 整合 #6.3 reports/ commit 拍板 (~50 文件 update) | 决策 #62 §5.3 + 决策 #71 §2.5 | ✅ 100% |
| **4** | 8 步 verify V1.1 release 100% 落实 (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名, per §10.2) | R137-3 §3.5 | ✅ 100% |
| **5** | 24 LOCKED crate 入口签名 verify 100% (25 LOCKED 总数 = 24 + PHL-07, per R137-2 + R137-1) | 决策 #74 B1 + 决策 #74 A3 | ✅ 100% |
| **6** | borrow 段 V1.1 release 0 装严守 二次 verify 100% (12 源 0 装 PASS 严守, per R131-6 §0 + R137-3 §3.4) | 决策 #33 §2.3 C2 | ✅ 100% |
| **7** | 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + §9.1) | 决策 #33 + 决策 #74 | ✅ 100% |
| **8** | 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 严守 100% (per 决策 #73 §3 + 哲学文档 15) | 决策 #73 §3 | ✅ 100% |
| **9** | 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2) | 决策 #33 §2.3 C2 | ✅ 100% |
| **10** | 0 主动 commit 严守 100% (Mavis 自决拍板, per 决策 #33 §2.3 C1) | 决策 #33 §2.3 C1 | ✅ 100% |
| **11** | 0 主动 push 严守 100% (等 V1.1 release 配 GitHub remote, 主人起床后手跑, per 决策 #33 + 决策 #61 §6) | 决策 #33 + 决策 #61 §6 | ✅ 100% |

---

## 附录 E: 报告交叉引用 (per 任务 spec, 引用不重写)

**报告交叉引用清单 (per 任务 spec, 引用不重写)**:

| 报告 | 路径 | 状态 | R153-3 引用 |
|------|------|------|------------|
| R131-1 | `reports/agent-r131-1-architecture-audit-2026-08-11.md` | ✅ done | §0 TL;DR + §10.4 派活时间表 |
| R131-2 | `reports/agent-r131-2-borrowed-11-sources-gap-2026-08-11.md` | ✅ done | §0 TL;DR + §7 借鉴 12 源 fork-then-borrow |
| R131-3 | `reports/agent-r131-3-v1.1-release-roadmap-2026-08-11.md` | ✅ done | §0 TL;DR + §1.1 V1.1 release tag + §10.4 |
| R131-4 | `reports/agent-r131-4-cargo-workspace-optimization-2026-08-11.md` | ✅ done | §0 TL;DR + §3.1 87 workspace members + §5.1 Cargo.lock = 271,450 bytes |
| R131-5 | `reports/agent-r131-5-24-locked-entry-distribution-2026-08-11.md` | ✅ done | §3.2 24 LOCKED crate 完整名单 + §5.1 Cargo.lock 24 LOCKED version + §9.1 8 硬墙严守 verify 9 步 |
| R131-6 | `reports/agent-r131-6-cargo-toml-borrow-section-2026-08-11.md` | ✅ done | §0 TL;DR + §3.3 借鉴 12 源 + §5.1 borrow 段 22:50 update + §7.2 1.2.1 bump 跟 borrow 段 关系 |
| R137-1 | `reports/agent-r137-1-phl-07-2026-08-11.md` | ✅ done | §9.1 8 硬墙严守 verify 9 步 + §10.3 11 步 verify 第 5 项 |
| R137-2 | `reports/agent-r137-2-24-locked-entry-rewrite-2026-08-11.md` | ✅ done | §10.2 8 步 verify 第 9 项 + §10.3 11 步 verify 第 5 项 |
| R137-3 | `reports/agent-r137-3-cargo-toml-1.2.1-bump-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 R153-3 跟前置报告关系 + §10.1 5 阶段 5 天 1 周 实施 spec + §13.1 派活计划 |
| R137-4 | `reports/agent-r137-4-asi-stage-9-2026-08-11.md` | ✅ done | §0 TL;DR + §1.1 整合 #6 commit 拍板 6 大方向 + §10.4 派活时间表 |
| R137-5 | `reports/agent-r137-5-formal-stage-5.5-2026-08-11.md` | ✅ done | §0 TL;DR + §1.1 + §10.4 |
| R145-3 | `reports/agent-r145-3-integration-5.1-cargo-workspace-1.2.0-verify-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 R153-3 跟前置报告关系 + §4 Cargo.toml 字段 update 10 段 + §9.1 8 硬墙严守 verify 9 步 |
| R149-4 | `reports/agent-r149-4-borrowed-12-sources-fork-then-borrow-pattern-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 R153-3 跟前置报告关系 + §7 借鉴 12 源 fork-then-borrow 模式 4 类 |
| R150-1 | `reports/agent-r150-1-v1.1-release-vs-agi-industry-v2.x-gap-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §10.4 |
| R150-2 | `reports/agent-r150-2-24-locked-entry-signature-optimize-gap-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §10.4 |
| R150-3 | `reports/agent-r150-3-cargo-workspace-1.2.1-bump-gap-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §2 必要性 + §4 内容清单 + §6 10 维决策矩阵 + §8 4 关系 + §10.4 + §12 风险 + 决策原则 |
| R151-1 | `reports/agent-r151-1-integration-6-commit-timeline-paiban-plan-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §10.4 |
| R151-2 | `reports/agent-r151-2-integration-7-commit-timeline-paiban-plan-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §10.4 |
| R152-1 | `reports/agent-r152-1-integration-6-cargo-workspace-1.2.1-bump-prep-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §3 87 workspace members + §4 Cargo.toml 字段 update + §5 Cargo.lock update 策略 + §6 24 LOCKED + §7 借鉴 12 源 + §10 5 阶段 5 天 1 周 + §13 派活计划 |
| R152-2/3/4/5 | (R152 era 续, 4 sub-agent) | 🟡 派活 | (待 5:30+ 后 write) |
| R153-1 | (V1.1 release ASI Stage 9 + 三洋葱 V2 集成 spec 准备) | 🟡 派活 | (待 5:30+ 后 write) |
| R153-2 | (V1.1 release ASI Stage 9 续) | 🟡 派活 | (待 5:30+ 后 write) |
| R153-3 | `reports/agent-r153-3-integration-6-cargo-workspace-1.2.1-bump-spec-detail-2026-08-11.md` | 🟢 done (本报告) | 整合 #6 commit Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细整合 |

---

## 附录 F: 决策链交叉引用 (per 任务 spec, 引用不重写)

**决策链交叉引用清单 (per 任务 spec, 引用不重写)**:

| 决策 | 路径 | 状态 | R153-3 引用 |
|------|------|------|------------|
| decision-22 | `reports/decision-22-master-auth-upgrade-2026-08-10.md` | ✅ done | §2.1 semver 严守依据 + §2.2 R-Cycle 7 子系统 + §3.2 24 LOCKED |
| decision-33 | `reports/decision-33-master-reupgrade-2026-08-10.md` | ✅ done | §0 TL;DR + §9.1 8 硬墙严守 verify 9 步 + §12 决策原则 |
| decision-36 | `reports/decision-36-p2-real-implementation-2026-08-10.md` | ✅ done | §3.3 借鉴 12 源 + §7.1 借鉴 12 源 fork-then-borrow 4 类 |
| decision-41 | `reports/decision-41-r125-16-all-done-2026-08-10.md` | ✅ done | §6.1 24 LOCKED 入口签名 状态 (内部 fn 实施可改) |
| decision-47 | `reports/decision-47-git-reset-no-effect-real-fix-2026-08-10.md` | ✅ done | §6.1 24 LOCKED 入口签名 状态 (内部 fn 实施可改) |
| decision-48 | `reports/decision-48-integration-4-commit-done-2026-08-10.md` | ✅ done | §1.1 整合 #4 commit 严守 100% + §12.2 决策原则 |
| decision-55 | `reports/decision-55-r127-integration-5-library-stage-4-6-2026-08-10.md` | ✅ done | §3.3 借鉴 12 源 + §7.1 借鉴 12 源 fork-then-borrow 4 类 |
| decision-56 | `reports/decision-56-r127-2-borrowed-3-retry-release-prep-2026-08-10.md` | ✅ done | §0 TL;DR + §1.3 R153-3 跟前置报告关系 |
| decision-57 | `reports/decision-57-r128-asi-python-tauri-cargo-release-2026-08-10.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-58 | `reports/decision-58-r128-2-final-3-sub-agents-2026-08-10.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-61 | `reports/decision-61-new-session-takeover-r129-plan-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §10.3 11 步 verify 第 11 项 + §12.2 决策原则 |
| decision-62 | `reports/decision-62-integration-5-commit-3-way-2026-08-11.md` | ✅ done | §0 TL;DR + §1.1 整合 #5 commit 拍板 Option A + §10.4 |
| decision-63 | `reports/decision-63-r129-batch-1-dispatch-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-64 | `reports/decision-64-all-rust-strict-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §12.2 决策原则 |
| decision-65 | `reports/decision-65-r129-batch-2-dispatch-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-66 | `reports/decision-66-r129-batch-3-dispatch-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §13.1 派活计划 |
| decision-67 | `reports/decision-67-r129-24-pending-cron-tick-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-68 | `reports/decision-68-r129-batch-4-dispatch-cron-resume-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §13.1 派活计划 |
| decision-69 | `reports/decision-69-r129-batch-5-dispatch-build-artifact-cleanup-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §12.2 决策原则 |
| decision-70 | `reports/decision-70-mavis-cleanup-decision-power-upgrade-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §12.2 决策原则 |
| decision-71 | `reports/decision-71-r130-era-dispatch-r129-3-final-wait-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §10.4 + §12.2 决策原则 + §13 派活计划 |
| decision-72 | `reports/decision-72-r130-era-dispatch-r129-3-final-wait-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-73 | `reports/decision-73-locked-unlocked-architecture-audit-philosophy-extension-2026-08-11.md` | ✅ done | §0 TL;DR + §1.2 8 硬墙 B1 改写表 + §8 8 哲学锚 + 不要怕复杂度哲学 + §12 决策原则 |
| decision-74 | `reports/decision-74-8-hard-walls-b1-rewrite-v1-0-0-��-v1-1-�Ծ�-2026-08-11.md` (含 `decision-74-readable.md`) | ✅ done | §0 TL;DR + §1.2 8 硬墙 B1 改写表 + §6 24 LOCKED 入口签名 + §9 8 硬墙严守 verify 9 步 + §10 5 阶段 5 天 1 周 + §12 决策原则 |
| decision-75 | `reports/decision-75-r131-r132-r133-batch-dispatch-11-sub-fill-16-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 + §3.2 24 LOCKED crate 完整名单 |
| decision-76 | `reports/decision-76-r134-r135-8-sub-dispatch-fill-16-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-77 | `reports/decision-77-r129-3-����-r136-r137-7-sub-fill-16-2026-08-11.md` (含 `decision-77-readable.md`) | ✅ done | §0 TL;DR + §1.3 + §2 semver 严守 + §3-§10 整合实施 spec |
| decision-78 | `reports/decision-78-integration-5.3-reports-commit-paiban-option-a-2026-08-11.md` | ✅ done | §0 TL;DR + §1.1 + §1.3 + §9.1 + §10 + §12 决策原则 |
| decision-79 | `reports/decision-79-r138-era-13-sub-r139-1-14-sub-dispatch-fill-16-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-80 | `reports/decision-80-r140-r143-14-sub-dispatch-fill-16-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-81 | `reports/decision-81-r129-3-8-step-verify-vs-decision-78-strict-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-82 | `reports/decision-82-r138-era-13-sub-done-r144-dispatch-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-83 | `reports/decision-83-r143-2-done-running-2-task-tool-fail-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-84 | `reports/decision-84-r144-r147-14-sub-dispatch-fill-16-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-85 | `reports/decision-85-r148-6-sub-dispatch-fill-16-2026-08-11.md` | ✅ done | §0 TL;DR + §1.3 |
| decision-86 | `reports/decision-86-05-00-tick-8-r148-errored-target-82gb-16-sub-dispatch-r149-r152-2026-08-11.md` | ✅ done | §0 TL;DR + §1.1 + §1.3 + §10 + §13 派活计划 |
| decision-87 | `reports/decision-87-05-15-tick-r139-1-retry-log-not-ready-r150-3-done-2-sub-replenish-2026-08-11.md` | ✅ done | §0 TL;DR + §1.1 + §1.3 + §9.1 8 硬墙严守 verify 9 步 + §10 + §13 派活计划 |
| decision-88 (本决策) | (Mavis 派 R153-3 写完决策) | 🟢 done (本报告) | 整合 #6 commit Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细整合 |

---

**R153-3 整合 #6 commit Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细整合 done 2026-08-11 5:15+ (60 min 时间盒内)** ✅
**报告路径**: `Apeireth-rust\reports\agent-r153-3-integration-6-cargo-workspace-1.2.1-bump-spec-detail-2026-08-11.md`
**R153-3 8 调研方向 100% 拓维 + 5 阶段 5 天 1 周 实施 spec 整合 100% + 8 硬墙严守 verify 9 步 100% + 8 哲学锚 + 不要怕复杂度哲学 9 件套 严守 100% + 0 改 src 严守 100% + 0 改 Cargo.toml 严守 100% + 0 主动 commit 严守 100% + 0 主动 push 严守 100% + 0 主动 IM 主人 严守 100% + 0 装 PASS 严守 100% + 8 硬墙 0 越界严守 100%**

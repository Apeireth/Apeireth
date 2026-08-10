# R137-5 形式化 Stage 5.5+ 实战 (per 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #74 A3 PHL-07 实施 + 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学)

**Date**: 2026-08-11 (R137 era 实施阶段, per 决策 #77 §3.1 实施 sub-agent 派活)
**Author**: R137-5 sub-agent (Mavis 派, per 决策 #77 §3.1 R137 era 永久循环接续 + 决策 #71 §5 R137 era 实施)
**Parent session**: mvs_367e66fae08342ffa399befe4f85dbac
**任务**: 形式化 Stage 5.5+ 实战 (per R130-4 形式化 Stage 5.5 集成深化 spec + R131-9 形式化集成优化 9 方向 + R132-1 V1.1 release 路线图 final 方向 6 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #74 A3 PHL-07 实施 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 = 形式化 Stage 5.5+ 5 阶段 5 周实施 + 5 方向实战 spec 报告)
**关联决策**: #22 (8 硬墙) + #33 (8 硬墙重置 + 0 装 PASS) + #36 (借鉴 11 源) + #48 (整合 #4 commit abf12243) + #55 (R127 派活 + §2.6 调研) + #56 (R127-2 形式化) + #57 (R128 ASI Python) + #58 (R128-2 派活) + #61 (新 session + R129 era 派活) + #62 (整合 #5 commit 3 拆) + #64 (cron 5 min tick) + #65 (R129 第 2 批) + #66 (R129 第 3 批) + #67 (R129-24 pending) + #68 (R129 第 4 批) + #69 (R129 第 5 批 + 编译产物清理) + #70 (Mavis 清理决策权升级) + #71 (R130 era 调研) + #72 (R130 era 调研 6 sub-agent) + #73 (主决策: locked 全解锁 + 架构审视 + 不要怕复杂度) + **#74 (8 硬墙 B1 改写: V1.0 release 0 改 + V1.1 release Mavis 自决改 + A3 PHL-07 V1.0 spec-only + V1.1 实施)** + #75 (R131 era 第 2 批 派活 11 sub) + #77 (R129-3 重派 + R136-R137 7 sub fill 16) + #78 (R130 era 后路线图)
**关联报告**: `agent-p8-2-retry-r127-2-library-stage-5-1-formal-proof-final-2026-08-10.md` (Stage 5.1 P8-2 形式化基础, 69 KB / 153 tests) + `agent-r129-10-formal-proof-stage-5.2-2026-08-11.md` (Stage 5.2 R129-10 F1-F10 10 维度形式化, 80.4 KB / 117 lib tests) + `agent-r129-20-formal-proof-stage-5.3-2026-08-11.md` (Stage 5.3 R129-20 F11-F20 跨模块证明, 88.5 KB / 92 lib tests) + `agent-r129-32-formal-proof-stage-5.4-execution-2026-08-11.md` (Stage 5.4 F21-F30 跨 Stage 5.x 集成 spec) + **`agent-r130-4-formal-proof-stage-5.5-integration-deepening-2026-08-11.md` (Stage 5.5 F1-F11 集成深化 spec, R130-4 ✅ done 调研 60 min)** + **`agent-r131-9-formal-proof-integration-optimization-2026-08-11.md` (Stage 5.5 形式化集成优化 9 方向, R131-9 ✅ done 调研 60 min)** + `agent-r131-5-24-locked-entry-optimization-2026-08-11.md` (24 LOCKED 入口分布优化 8 方向, R131-5 ✅ done 调研 24/24 入口签名 0 改 verify) + `agent-r132-1-v1.1-release-roadmap-final-2026-08-11.md` (V1.1 release 路线图 final 6 方向, R132-1 ✅ done 调研, 形式化 Stage 5.5+ = 方向 6) + `decision-74-readable.md` (8 硬墙 B1 改写: V1.0 release 0 改 + V1.1 release Mavis 自决改) + `15-no-fear-complexity.md` (R130 era 主人 8/11 01:14 拍板总工程哲学扩展)
**状态**: ✅ **done 调研 + 实施 spec 报告 (R137-5 派活, 60 min 时间盒, 估 0 改 src, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 0 装 PASS 严守 100%, kani 借鉴不安装 0 重复造轮子 per 用户记忆 #6)**

---

## 0. 一句话 (TL;DR)

**R137-5 形式化 Stage 5.5+ 实战 = 5 阶段 5 周实施计划 + 5 方向实战内容 (PHL-07 形式化 / F1-F11 11 维度 Kani 全集成 / 24 LOCKED 入口 形式化 / 8 哲学锚 形式化 / V0.5 30 维 + 6 重守门 v7 形式化), 0 写 `crates/apeireth-formal/src/stage5_5/` NEW 目录 (R137-5 是 spec 阶段, 0 改 src/ per 决策 #33 §2.3 C1 + 决策 #71 §2.2 调研任务规范 + 决策 #74 B1 改写 V1.0 release 0 改严守), 0 装 kani (借 kani 5.5MB 源 0 装, 仅借鉴 Invariant trait + trivial_invariant! + `#[cfg_attr(kani, kani::proof)]` + `kani::any()` + HarnessMetadata 5 模式 1:1 翻译, per 用户记忆 #6 0 重复造轮子). V1.0 release (整合 #5.1 commit, 估 8/11 主人起床后手跑) 形式化 Stage 5.4 实战 严守 (per R129-32 + 决策 #33 §2.3 B3 + 决策 #74 §1 F1-F10 10 维度严守 + 0 改 src 严守), V1.1 release (估 2026-11-30, per R130-5 §1.1 + R132-1) 形式化 Stage 5.5+ 实施 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #74 A3 PHL-07 实施 + 主人 8/11 01:14 拍板 3 件套). 5 阶段 = ①PHL-07 形式化 (1 周, 决策 #74 A3) + ②F1-F11 11 维度 Kani 全集成 (1 周, per R130-4 形式化 Stage 5.5) + ③24 LOCKED 入口 形式化 (1 周, per 决策 #33 §2.3 B1 + 决策 #74 §1 V1.1 release Mavis 自决改) + ④8 哲学锚 形式化 (1 周, per 决策 #33 §2.3 B5) + ⑤V0.5 30 维 + 6 重守门 v7 形式化 (1 周, per 决策 #33 §2.3 B3/B4). 总 5 周 = 5 × 1 周 = V1.1 release 估 2026-11-30 (per R130-5 §1.1 + R132-1). 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 改写表): B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.3) / B2 workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (per 决策 #74 §1 B2) / A1 R11 baseline 3 值 (0.8682/0.8532/0.9063) 严守 (哲学 + 效果标) / A3 13 键 严守 (PHL-07 V1.0 spec-only 0 实施, V1.1 实施, per 决策 #74 §1 A3) / B3 V0.5 30 维 严守 (哲学) / B4 6 重守门 v7 严守 (哲学) / B5 8 哲学锚 严守 (哲学) / C1 0 主动 commit 严守 (Mavis 拍板) / C2 0 装 PASS 严守 100% (✅ cloned = 真实施) / 0 主动 push 严守 (等 1.0 release 配 GitHub remote). 不要怕复杂度哲学落地 3 件套 (per 决策 #73 §3 + `15-no-fear-complexity.md`): 最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队. 借鉴 kani 5.5MB 源 (实测, 任务说 8.3MB 略偏大, 0 装) 仅借 5 模式 1:1 翻译 (Invariant trait + trivial_invariant! + `#[cfg_attr(kani, kani::proof)]` + `kani::any()` + HarnessMetadata), 0 引 kani crate 依赖, 0 装"已 Kani 形式化" / "已 Kani 求解器在线", Cargo.toml 0 改 (per 决策 #33 §2.3 C2).**

---

## 1. 形式化 Stage 5.5+ 战略定位 (1.0 release + V1.1 release 接力 + Stage 5.x 6 阶演进)

### 1.1 形式化 Stage 5.x 6 阶演进链 (per 决策 #33 §2.3 + 决策 #55 §1 + 决策 #56 + 决策 #57 + 决策 #61 §3.1 + 决策 #69 §3 + 决策 #72 §2.1 + 决策 #74 §1 + 决策 #75 §2.1 + R130-4 §1.1 + R131-9 §1.1 + R132-1 §1.5 方向 6)

| Stage | 时机 | 派活 | 任务 | 范围 | 借鉴 | 状态 |
|---|---|---|---|---|---|---|
| **Stage 5.1** (Library 形式化) | R127-2 P8-2 retry 22:06 done (per 决策 #56) | P8-2 (single sub-agent) | Library crate 形式化基础 (kani 4502 Invariant trait + 8 Kani-style harness + 5 NEW POD 模型 + Stage5Token POD) | `crates/apeireth-library-governance/src/formal_proof.rs` 39.3KB + `tests/formal_proof_integration.rs` 14.7KB + `tests/integration.rs` 15.0KB = 69 KB / 16 Kani-style harness / 153 tests | kani 4502 ✅ cloned 真实施 | ✅ P8-2 done |
| **Stage 5.2** (formal crate 形式化扩展) | R129 era 第 2 批 00:30 cron 派 R129-10 00:49 done (per 决策 #65) | R129-10 (single sub-agent, 19 min) | formal crate 形式化扩展 F1-F10 10 维度 (6 重 v7 + 8 锚 + 30 维 + 13 键 + R11 + 24 LOCKED + 8 借鉴 + 整合 #4 + 跨模块 + 集成) | `crates/apeireth-formal/src/stage5_2/` 11 文件 80,379 B ~80 KB / 117 lib tests (含 79 NEW) | kani 4502 + langgraph 829 ✅ cloned 真实施 | ✅ R129-10 done |
| **Stage 5.3** (formal crate 跨模块证明) | R129 era 第 3 批 00:34 派 R129-20 00:50 done (per 决策 #66) | R129-20 (single sub-agent, 16 min) | formal crate 跨模块证明 F11-F20 10 维度 (跨 crate + 跨借鉴 + 跨 stage + 跨决策 + 跨 commit + 跨 LOCKED + 跨 anchor + 跨 gate + 跨 version + 跨 push) | `crates/apeireth-formal/src/stage5_3/` 11 文件 88.5 KB / 92 lib tests | kani 4502 ✅ cloned 真实施 | ✅ R129-20 done |
| **Stage 5.4** (formal crate 集成扩展, R129-32 spec) | R131 era 估 8/12+ 派 (per 决策 #64 §2.2 + 决策 #69 §3 R129-32 spec) | R131-4 (估 60 min 派, 1 sub-agent) | formal crate 集成扩展 F21-F30 10 维度 (跨 stage 5.1-5.3 集成 + 跨借鉴源 2 借鉴 ID + 跨决策链 + 跨 24 LOCKED + 跨 8 哲学锚 + 跨 6 重守门 v7 + 跨 30 维 V0.5 + 跨 13 键 + 跨 R11 baseline + 跨 push 严守) | 估 `crates/apeireth-formal/src/stage5_4/` 11 文件 ~100 KB / ~110 lib tests | kani 4502 + langgraph 829 ✅ cloned 真实施 (续 Stage 5.2/5.3 同模式) | 📋 R131-4 spec (R129-32 ✅ done, 0 写) |
| **Stage 5.5** (formal crate 集成深化, R130-4 spec) | V1.1 minor release 前 估 2026-11 派 (per 决策 #78 R130 era 后路线图) | R133-N (估 60 min 派, 1 sub-agent, V1.1 minor release 前) | formal crate 集成深化 F1-F10 10 维深化 + F11 NEW 1 维 (PHL-07 spec-only 形式化 + 长程 AI 成长 形式化) = F1-F11 11 维度 | 估 `crates/apeireth-formal/src/stage5_5/` 12 文件 ~85 KB / 89 lib tests (F1-F10 既有 80 续 + F11 NEW 9) | kani 4502 + langgraph 829 ✅ cloned 真实施 (续 Stage 5.2 同模式) | 📋 R130-4 spec (R130-4 ✅ done 调研, 0 写) |
| **Stage 6** (形式化证明 + 实战, R132+ era) | R132+ era 估 8/15+ 派 (per 决策 #64 §2.2 + 决策 #78 R130 era 派活清单 + 1.0 release 实战后) | R132-N (估 90-120 min 派, N=3-5 sub-agent) | 形式化证明 + 实战 (kani 求解器在线扩展 + 跨 stage 全集成 Stage 1-5.x + 实战 1.0 release 验证 + 1.0+ 形式化扩展) | 估 `crates/apeireth-formal/src/stage6/` 5-8 文件 ~150-200 KB / 200+ lib tests + kani 求解器在线跑 (per R130 era 后) | kani 4502 + langgraph 829 + PyO3 928 (asi-formal-pybridge 实战) ✅ cloned 真实施 | 📋 R132-N spec (R129-32 spec, 0 写) |

**6 阶演进链 1:1 续 per 决策 #33 §2.3 C2 (0 装 PASS 严守 100%) + 决策 #55 §1 (Stage 5.2 续 #33 §2.3 借鉴 kani 4502 形式化) + 决策 #56 (R127-2 形式化 Stage 5.1 续) + 决策 #61 §3.1 R129-20 (Stage 5.3 续) + 决策 #69 §3 R129-32 (Stage 5.4 续 + Stage 6 路线) + 决策 #72 §2.1 R130-4 (Stage 5.5 续 形式化扩展 F1-F11 11 维度) + 决策 #74 §1 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改 B1 改写) + 决策 #75 §2.1 (R131 era 第 2 批 R131-4~9 形式化集成优化 R131-9)**.

### 1.2 形式化 Stage 5.5+ 跟 V1.0 release + V1.1 release 关系 (per 决策 #74 B1 + R130-4 §1.1 + R131-9 §1.1 + R132-1 §1.5 方向 6)

| Release | 形式化 Stage | 范围 | 8 硬墙严守 | 决策依据 |
|---|---|---|---|---|
| **V1.0 release** (整合 #5.1 commit, 估 8/11) | **Stage 5.4 实战 严守** (per R129-32 spec) | F21-F30 跨 Stage 5.x 集成形式化 (估 100 KB / ~110 lib tests, R131-4 估 8/12+ 派) | 8 硬墙 0 越界 100% (B1 24 LOCKED 入口签名 0 改严守) | 决策 #33 §2.3 C1 + 决策 #74 §1 B1 V1.0 release 0 改严守 + R129-32 spec + R130-4 §1.3 8 硬墙 0 越界 |
| **V1.1 release** (估 2026-11-30, per R130-5 §1.1 + R132-1) | **Stage 5.5 集成深化 实施** (per R130-4 spec + R131-9 9 方向 + R132-1 方向 6) | F1-F11 11 维度 (F1-F10 续 1:1 + F11 NEW PHL-07 spec-only + 长程 AI 成长 形式化) (估 85 KB / 89 lib tests, R133-N V1.1 release 前派) | 8 硬墙 V1.1 release Mavis 自决改 B1 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 "工程类 + 技术类 locked 全早解锁") | 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 决策 #74 §2.3 A3 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 + R130-4 + R131-9 + R132-1 方向 6 |
| **V1.2 release** (估 2027-02-28) | **Stage 5.5 续 + 跨 Stage 5.x 实战** (per R130-5 §1.2) | F1-F11 11 维度 + 跨 Stage 5.x 集成 (估 150+ KB / 200+ lib tests, R132-N 实战) | 8 硬墙 V1.2 release 严守 100% (V1.1 release 续) | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + R130-5 §1.2 + R132-1 §1.2 |
| **V2.0 release** (2027+, 远期) | **Stage 6 形式化证明 + 实战 + 全 8 硬墙可重评** (per R130-4 §7.1 + 决策 #74 §2.3) | Kani 求解器在线扩展 + 跨 stage 全集成 Stage 1-5.x + 形式化重构 (估 200+ KB / 300+ lib tests, R132-N 实战) | 8 硬墙 V2.0 release 8 硬墙可重评 (推翻 + 重建 8 哲学锚) | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 不要怕复杂度哲学 |

**关键澄清 (per 决策 #74 §2.3 + R130-4 §1.2 + R131-9 §1.1 + R132-1 §1.5 方向 6)**:
- **Stage 5.5 跟 Stage 5.4 是平行分支**, 0 互依. Stage 5.4 = 跨 Stage 5.x 集成 (F21-F30), Stage 5.5 = Stage 5.2 集成深化 (F1-F11, 复用 F1-F10 编号 + F11 NEW).
- **Stage 5.5 命名**: "集成深化" 区别于 Stage 5.2 "形式化扩展" + Stage 5.3 "跨模块证明" + Stage 5.4 "跨 Stage 5.x 集成" + Stage 6 "实战".
- **F11 NEW 1 维 命名**: "PHL-07 spec-only 形式化 + 长程 AI 成长 形式化" (per 决策 #72 §2.1 R130-4 派活 + 用户记忆 #4 "AI 不会衰老病死" + 13 键 PHL-07 升级 + 决策 #74 §1 A3 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施).

### 1.3 形式化 Stage 5.5+ 跟 5 大方向关系 (per 决策 #74 B1 + R130-4 + R131-9 + R132-1 方向 6 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

**V1.1 release 6 大方向 final 版** (per R132-1 §1.5 整合 R130-5 + R131-1 + R131-2 = final 版 + 决策 #74 B1 V1.1 release Mavis 自决改):

| # | 方向 | 跟 形式化 Stage 5.5+ 关系 | 实施时机 | 8 硬墙严守 | 决策依据 |
|---|------|---------------------------|----------|-----------|----------|
| **1** | PHL-07 实施 | 形式化 Stage 5.5+ 阶段 1 = PHL-07 形式化 (per R130-4 §2.2 + 决策 #74 A3) | R134-PHL07-1~5 (5 sub, 1 周, V1.1 release 估 2026-09) | A3 13 键 0 改 + B5 8 哲学锚 0 改 + **0 形式化 old/death/terminate 概念** (per 用户记忆 #4) | 决策 #74 §1 A3 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 + R129-11 关键诚实标 + 用户记忆 #4 |
| **2** | 24 LOCKED 入口签名改写 | 形式化 Stage 5.5+ 阶段 3 = 24 LOCKED 入口 形式化 (per R130-4 §2.1 F6 + 决策 #74 §2.3 B1) | R134-LOCKED-1~5 (5 sub, 1 周, V1.1 release 估 2026-09-10) | B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构) | 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + R130-5 入口分布 + R131-1 架构审视 + R131-5 8 优化方向 |
| **3** | 后端加固 | 形式化 Stage 5.5+ 阶段 5 = V0.5 30 维 + 6 重守门 v7 形式化 (per R130-4 §2.1 F3 + F1) | R134-backend-1~5 (5 sub, 1 周, V1.1 release 估 2026-09-17) | B3 V0.5 30 维 0 改 + B4 6 重守门 v7 0 改 | 决策 #33 §2.3 B3/B4 + 决策 #74 §1 B3/B4 严守 + R130-1 24+5+1 errors + R131-1 架构审视 |
| **4** | Tauri Stage 5+ | 0 直接接 (形式化 Stage 5.5+ 0 涉及 Tauri, Tauri 跟形式化正交, per R130-4 §1.4 Stage 5.x 借鉴模式) | R134-tauri-1~5 (5 sub, 1 周, V1.1 release 估 2026-09-24) | 0 越界 (Tauri 0 触碰 8 硬墙) | 决策 #57 + R130-3 调研 + 用户记忆 #3-#5 + 主人 8/4 23:33 Tauri 终极 |
| **5** | ASI Stage 8+ | 形式化 Stage 5.5+ 阶段 2 部分 (F11 NEW 形式化 长程 AI 成长, per R130-4 §2.2) | R134-asi-1~5 (5 sub, 1 周, V1.1 release 估 2026-10-01) | B5 8 哲学锚 0 改 + **0 形式化 old/death/terminate 概念** (per 用户记忆 #4) | 决策 #55-#58 + R130-2 调研 + R131-2 OpenCog fork 决策 + 用户记忆 #4 |
| **6** | 形式化 Stage 5.5+ | 形式化 Stage 5.5+ = 本报告 (5 阶段 5 周) | R134-formal-1~5 (5 sub, 1 周, V1.1 release 估 2026-10-08) | 8 硬墙 0 越界 100% + 用户记忆 #4 0 形式化 old/death/terminate 严守 + 0 装 PASS 严守 100% | 决策 #33 §2.3 + 决策 #56 + R129-32 + R130-4 + R131-9 + 决策 #74 §1 + 决策 #75 §2.1 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 |

**形式化 Stage 5.5+ 跟其他 5 方向关系 (per R132-1 §1.5 方向 6 整合 R130-5 + R131-1 + R131-2 + R131-5 + R131-9)**:
- **方向 1 PHL-07 实施** = 形式化 Stage 5.5+ 阶段 1 (PHL-07 形式化, per R130-4 §2.2 F11 NEW 子模块 1 + 决策 #74 A3)
- **方向 2 24 LOCKED 入口签名改写** = 形式化 Stage 5.5+ 阶段 3 (24 LOCKED 入口 形式化, per R130-4 §2.1 F6 续 + 决策 #74 §2.3 B1 V1.1 release Mavis 自决改)
- **方向 3 后端加固** = 形式化 Stage 5.5+ 阶段 5 部分 (V0.5 30 维 + 6 重守门 v7 形式化, per R130-4 §2.1 F1 + F3 续 + 决策 #33 §2.3 B3/B4)
- **方向 5 ASI Stage 8+** = 形式化 Stage 5.5+ 阶段 2 部分 (F11 NEW 长程 AI 成长 形式化, per R130-4 §2.2 F11 NEW 子模块 2 + 用户记忆 #4)
- **方向 4 Tauri Stage 5+** = 0 直接接 (Tauri 0 触碰 8 硬墙, 形式化 Stage 5.5+ 仅涉及 formal crate, 0 涉及 Tauri)

---

## 2. 形式化 Stage 5.5+ 5 阶段实施计划 (per R130-4 + R131-9 + R132-1 方向 6 + 决策 #74 B1 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

### 2.1 5 阶段总览 (5 周 = 5 × 1 周, V1.1 release 估 2026-11-30)

| 阶段 | 时机 (估) | 任务 | 派活 | 报告 | 范围 | 8 硬墙严守 |
|------|----------|------|------|------|------|-----------|
| **阶段 1: PHL-07 形式化** (1 周, per 决策 #74 A3 + R130-4 §2.2) | 2026-08-19 → 2026-08-25 (R134-PHL07-1~5, 5 sub, V1.1 release 估 2026-09-01) | PHL-07 形式化证明 (Kani-style harness) + F1-F11 11 维度集成 (per R130-4) + V0.5 30 维 公式集成 + 6 重守门 v7 集成 + 8 哲学锚集成 | R134-PHL07-1 (1 sub, 60 min, PHL-07 spec-only 形式化) + R134-PHL07-2 (1 sub, 60 min, PHL-07 形式化 F1-F11 集成) + R134-PHL07-3 (1 sub, 60 min, PHL-07 V0.5 30 维 公式集成) + R134-PHL07-4 (1 sub, 60 min, PHL-07 6 重守门 v7 集成) + R134-PHL07-5 (1 sub, 60 min, PHL-07 8 哲学锚集成) | `agent-r134-phl07-1-...-2026-08-XX.md` ~ `agent-r134-phl07-5-...-2026-08-XX.md` (估 ~80 KB / ~80 lib tests, F11 NEW 9 单元测试续 + PHL-07 实施 41 NEW tests per R130-5 §2.1) | A3 13 键 0 改 + B5 8 哲学锚 0 改 + **0 形式化 old/death/terminate 概念** (per 用户记忆 #4) |
| **阶段 2: F1-F11 11 维度 Kani 全集成** (1 周, per R130-4 形式化 Stage 5.5) | 2026-08-26 → 2026-09-01 (R134-formal-1~5, 5 sub, V1.1 release 估 2026-09-08) | F1-F10 10 维度 (现有, per R125 B3 V0.5 25 维 + R130-4 Stage 5.5 续 5 维) → F1-F11 11 维度 (Stage 5.5 新增 1 维) + Kani 全集成 (kani 5.5MB 借用, 0 装 PASS 严守 100%) + 11 维度 形式化证明 (Kani-style harness) | R134-formal-1 (1 sub, 60 min, F1-F10 续 1:1 翻译 Stage 5.2 → Stage 5.5) + R134-formal-2 (1 sub, 60 min, F11 NEW PHL-07 spec-only + 长程 AI 成长 形式化, per R130-4 §2.2) + R134-formal-3 (1 sub, 60 min, F1-F11 11 维度 联合 invariant 1 形式化, per R130-4 §2.3) + R134-formal-4 (1 sub, 60 min, Kani 全集成 sanity_check, per R130-4 §4.1) + R134-formal-5 (1 sub, 60 min, 89 lib tests verify + cargo test 0 装 PASS 严守) | `agent-r134-formal-1-...-2026-08-XX.md` ~ `agent-r134-formal-5-...-2026-08-XX.md` (估 ~85 KB / 89 lib tests, per R130-4 §6.1) | 8 硬墙 0 越界 100% + 0 装 PASS 严守 100% |
| **阶段 3: 24 LOCKED 入口 形式化** (1 周, per 决策 #33 §2.3 B1 + 决策 #74 §1 + R131-5) | 2026-09-02 → 2026-09-08 (R134-LOCKED-1~5, 5 sub, V1.1 release 估 2026-09-15) | 24 LOCKED 入口 形式化证明 (Kani-style harness) + 24 LOCKED 入口签名 0 改 (V1.0 release 严守, per R131-5 verify 24/24 LOCKED crate 入口签名 0 改全部通过) + 24 LOCKED 入口 形式化集成 (V1.1 release Mavis 自决改, per 决策 #74 B1) | R134-LOCKED-1 (1 sub, 60 min, 24 LOCKED 入口签名 0 改 verify, per R131-5 §1.2) + R134-LOCKED-2 (1 sub, 60 min, 24 LOCKED 入口 F6 形式化 1:1 续 Stage 5.2, per R130-4 §2.1 F6) + R134-LOCKED-3 (1 sub, 60 min, 24 LOCKED 入口 形式化 Kani-style harness) + R134-LOCKED-4 (1 sub, 60 min, 24 LOCKED 入口 形式化集成, V1.1 release Mavis 自决改) + R134-LOCKED-5 (1 sub, 60 min, 24 LOCKED 入口 形式化 verify 0 装 PASS 严守) | `agent-r134-locked-1-...-2026-09-XX.md` ~ `agent-r134-locked-5-...-2026-09-XX.md` (估 ~50 KB / ~50 lib tests, 24 LOCKED 入口 形式化) | B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.3) |
| **阶段 4: 8 哲学锚 形式化** (1 周, per 决策 #33 §2.3 B5 + 决策 #74 §1) | 2026-09-09 → 2026-09-15 (R134-anchor-1~5, 5 sub, V1.1 release 估 2026-09-22) | 8 哲学锚 形式化证明 (Kani-style harness) + 8 哲学锚 严守 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5) + 8 哲学锚 形式化集成 (V1.1 release 实施) | R134-anchor-1 (1 sub, 60 min, 8 哲学锚 形式化 1:1 续 Stage 5.2, per R130-4 §2.1 F2) + R134-anchor-2 (1 sub, 60 min, 8 哲学锚 Subjective/Objective namespace 1:1 严守) + R134-anchor-3 (1 sub, 60 min, 8 哲学锚 Kani-style harness) + R134-anchor-4 (1 sub, 60 min, 8 哲学锚 形式化集成 V1.1 release 实施) + R134-anchor-5 (1 sub, 60 min, 8 哲学锚 形式化 verify) | `agent-r134-anchor-1-...-2026-09-XX.md` ~ `agent-r134-anchor-5-...-2026-09-XX.md` (估 ~30 KB / ~30 lib tests, 8 哲学锚 形式化) | B5 8 哲学锚 0 改 100% (per 决策 #33 §2.3 B5 + 决策 #74 §1) |
| **阶段 5: V0.5 30 维 + 6 重守门 v7 形式化** (1 周, per 决策 #33 §2.3 B3/B4 + 决策 #74 §1) | 2026-09-16 → 2026-09-22 (R134-formal-6~10, 5 sub, V1.1 release 估 2026-09-29) | V0.5 30 维 公式 形式化证明 (sum=1.00 守门 0 改) + 6 重守门 v7 形式化 (4 重 + 权限 + Colang DSL 守门) + V0.5 30 维 + 6 重守门 v7 集成 | R134-formal-6 (1 sub, 60 min, V0.5 30 维 形式化 1:1 续 Stage 5.2, per R130-4 §2.1 F3) + R134-formal-7 (1 sub, 60 min, 6 重守门 v7 形式化 1:1 续 Stage 5.2, per R130-4 §2.1 F1) + R134-formal-8 (1 sub, 60 min, V0.5 30 维 + 6 重守门 v7 Kani-style harness) + R134-formal-9 (1 sub, 60 min, V0.5 30 维 + 6 重守门 v7 集成) + R134-formal-10 (1 sub, 60 min, V0.5 30 维 + 6 重守门 v7 形式化 verify 0 装 PASS 严守) | `agent-r134-formal-6-...-2026-09-XX.md` ~ `agent-r134-formal-10-...-2026-09-XX.md` (估 ~40 KB / ~40 lib tests, V0.5 30 维 + 6 重守门 v7 形式化) | B3 V0.5 30 维 0 改 + B4 6 重守门 v7 0 改 (per 决策 #33 §2.3 B3/B4 + 决策 #74 §1) |

**总时间盒**: 5 阶段 × 1 周 = 5 周 (V1.1 release 估 2026-11-30, per R130-5 §1.1 + R132-1 方向 6 + 决策 #74 §1 + 主人 8/11 01:14 拍板 3 件套).

### 2.2 5 阶段依赖关系 + 16 跑中上限严守 (per 决策 #71 §5 + 决策 #64 §2.2 + 主人 0:34 拍板 16 上限)

**5 阶段依赖关系 (per R130-4 + R131-9 + R132-1 + 决策 #74 + 决策 #75)**:
- 阶段 1 PHL-07 形式化 → 阶段 2 F1-F11 11 维度 Kani 全集成 (阶段 1 输出 = F11 NEW 子模块 1 = PHL-07 spec-only 形式化, 阶段 2 集成)
- 阶段 2 F1-F11 11 维度 Kani 全集成 → 阶段 3 24 LOCKED 入口 形式化 (阶段 2 输出 = F1-F11 11 维度 Stage 5.5 形式化基础, 阶段 3 集成)
- 阶段 2 F1-F11 11 维度 Kani 全集成 → 阶段 4 8 哲学锚 形式化 (阶段 2 输出 = F1-F11 11 维度 Stage 5.5 形式化基础, 阶段 4 集成)
- 阶段 2 F1-F11 11 维度 Kani 全集成 → 阶段 5 V0.5 30 维 + 6 重守门 v7 形式化 (阶段 2 输出 = F1-F11 11 维度 Stage 5.5 形式化基础, 阶段 5 集成)
- 阶段 3 + 阶段 4 + 阶段 5 → V1.1 release 实施续 (per R132-1 §1.5 方向 6 整合)

**16 跑中上限严守 (per 决策 #71 §5 + 决策 #64 §2.2 + 主人 0:34 拍板 16 上限 + cron `watch-r137-era-auto-replenish-16` 续)**:
- R137 era 实施阶段 5 阶段 × 5 sub/阶段 = 25 sub-agent, 16 跑中上限严守, 2 批 13+12 派满 16 上限 (per 决策 #71 §5 续)
- 阶段 1 R134-PHL07-1~5 (5 sub) + 阶段 2 R134-formal-1~5 (5 sub) + 阶段 3 R134-LOCKED-1~5 (5 sub) + 阶段 4 R134-anchor-1~5 (5 sub) + 阶段 5 R134-formal-6~10 (5 sub) = 25 sub
- 2 批 13+12: 阶段 1 + 阶段 2 部分 (前 3 sub) = 13 sub 第 1 批, 阶段 2 部分 (后 2 sub) + 阶段 3-5 = 12 sub 第 2 批 (per 决策 #71 §5 + cron auto-replenish)

### 2.3 5 阶段 0 装 PASS 严守 3 层守门 (per 决策 #33 §2.3 C2 + 决策 #55 §3 + 决策 #72 §2.1 R130-4 派活)

**0 装 PASS 3 层守门 (5 阶段 1:1 续 Stage 5.2 模式)**:
1. **编译期 hardcode (per 决策 #33 §2.3 C3 严守)**: 5 阶段共 35+ 编译期常数 (`SIX_FOLD_GATE_V7_COUNT = 6` / `EIGHT_ANCHORS_COUNT = 8` / `V05_30_TOTAL_DIMS = 30` / `VERDICT_CACHE_13_KEYS_COUNT = 13` / `R11_BASELINE_V1141 = 0.8682` / `LOCKED_24_CRATES_COUNT = 24` / `BORROW_8_ID_COUNT = 8` / `INTEGRATION_4_HARD_WALLS_VERIFY = 8` / F11 NEW: `PHL_07_SPEC_ONLY_COUNT = 1` + `PHL_07_SPEC_ONLY_KEY_INDEX = 12` + `LONG_TERM_AI_GROWTH_STAGE_COUNT = 3` 等) 编译期嵌入二进制, 0 动态加载.
2. **cfg-gated 双实现 (per 决策 #33 §2.3 C2 + 借鉴 Kani 4502)**: 5 阶段全部 `#[cfg_attr(kani, kani::proof)]` + `nondet_*()` 兜底 (Kani 离线时退化为具体 happy path), cargo test 跑得通 + 未来 `cargo kani` 也能跑.
3. **集成测试 verify 0 装**: 5 阶段共 89+ 单元测试 (F1-F10 既有 80 续 + F11 NEW 9) + 1 跨维度联合 invariant + 24 LOCKED 入口 形式化 ~50 + 8 哲学锚 ~30 + V0.5 30 维 + 6 重守门 v7 ~40 = 估 250+ 单元测试, 0 假设 "已实施".

---

## 3. 形式化 Stage 5.5+ 5 方向 实战内容 (per 任务规范 + R130-4 + R131-9 9 方向 + R132-1 方向 6 + 决策 #74 B1 + 主人 8/11 01:14 拍板 3 件套)

### 3.1 方向 1: PHL-07 形式化 (per 决策 #74 A3 + R130-4 §2.2 + R131-9 §1.3 O6 + R132-1 §1.5 方向 1)

**方向 1 = 形式化 Stage 5.5+ 阶段 1 (PHL-07 形式化, 1 周, 5 sub, V1.1 release 估 2026-09-01)**:

| 实战内容 | spec 详 | 8 硬墙严守 | 决策依据 |
|---------|---------|-----------|----------|
| **PHL-07 形式化证明 (Kani-style harness)** | `phl07_spec_only_and_long_term_ai_growth_formal.rs` (R130-4 §2.2 F11 NEW 子模块 1, ~2,500 B) = `PHL_07_SPEC_ONLY_COUNT = 1` + `PHL_07_SPEC_ONLY_KEY_INDEX = 12` (A3 严守) + `SpecOnlyKind` enum (1 变体: NotUnoptimizable) + `Phl07SpecOnlyPod` POD + 3 invariant (`phl07_spec_only_invariant_key` = `p.key == 12` / `phl07_spec_only_invariant_not_unoptimizable` = `NotUnoptimizable && is_formaled` / `phl07_spec_only_invariant_stage` = `1..=3`) + 1 Kani-style proof harness (`proof_phl07_spec_only_key_is_12`) + 3 单元测试 (3 invariant 1:1) | A3 13 键 0 改 (key=12 严守) + 0 假装"已 optimal" (PHL-07 spec-only 性质) | 决策 #33 §2.3 A3 + 决策 #74 §1 A3 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 + R130-4 §2.2 + R129-11 关键诚实标 |
| **PHL-07 F1-F11 11 维度集成** | F1-F11 1 联合 invariant (PHL-07 + F1-F10 既有 10 维, per R130-4 §2.3) = `f11_integration_spec_only_not_unoptimizable(p, g) = phl07_spec_only_invariant_not_unoptimizable(p) && long_term_growth_no_terminate_invariant(g)` | 8 硬墙 0 越界 100% (F1-F10 既有 1:1 续 Stage 5.2 + F11 NEW 形式化) | 决策 #33 §2.3 + 决策 #72 §2.1 R130-4 派活 + R130-4 §2.3 F1-F11 整合 spec |
| **PHL-07 V0.5 30 维 公式集成** | V0.5 30 维 (4 类 × 6 维 + 5 meta + 1 overall = 30 维) + PHL-07 集成 (PHL-07 = 第 13 键 verdict cache, 0 含 V0.5 30 维 公式修改) | B3 V0.5 30 维 0 改 + A3 13 键 0 改 | 决策 #33 §2.3 B3 + 决策 #51 §1.2 P1-4 R126 30 维 verify done + R130-4 §2.1 F3 |
| **PHL-07 6 重守门 v7 集成** | 6 重守门 v7 (L1TypeCheck..L6ProvenanceCheck) + PHL-07 集成 (PHL-07 = 13 键 第 13 键 verdict cache, 跟 6 重守门 v7 0 重叠) | B4 6 重守门 v7 0 改 | 决策 #33 §2.3 B4 + 决策 #55 §1 P1-3 R126 6 重守门 v7 done + R130-4 §2.1 F1 |
| **PHL-07 8 哲学锚集成** | 8 哲学锚 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5) + PHL-07 集成 (PHL-07 spec-only 形式化 跟 8 哲学锚 0 重叠, 0 形式化 old/death/terminate 概念, per 用户记忆 #4) | B5 8 哲学锚 0 改 + **0 形式化 old/death/terminate 概念** (per 用户记忆 #4 严守) | 决策 #33 §2.3 B5 + 决策 #51 §1.2 P1-2 R126 8 哲学锚升级 done + R130-4 §2.2 F11 NEW |

**0 装 PASS 严守 100%**: PHL-07 形式化有真 src 改动 (`crates/apeireth-formal/src/stage5_5/phl07_spec_only_and_long_term_ai_growth_formal.rs` ~2,500 B NEW, V1.1 release 估 2026-08 派 R134-PHL07-1~5 写) + 有真 tests pass (3 单元测试 1:1 跟 3 invariant) + 有真数据流 (5 编译期常数 + 1 POD + 1 enum + 3 invariant + 1 Kani-style proof harness + 1 联合 invariant) + 0 装"已 Kani 形式化" (Kani 离线时退化为普通 fn, runtime 全过).

### 3.2 方向 2: F1-F11 11 维度 Kani 全集成 (per R130-4 形式化 Stage 5.5 + R131-9 §1.3 O2 + R132-1 §1.5 方向 6)

**方向 2 = 形式化 Stage 5.5+ 阶段 2 (F1-F11 11 维度 Kani 全集成, 1 周, 5 sub, V1.1 release 估 2026-09-08)**:

| 实战内容 | spec 详 | 8 硬墙严守 | 决策依据 |
|---------|---------|-----------|----------|
| **F1-F10 10 维度 (现有) → F1-F11 11 维度 (Stage 5.5 新增 1 维)** | F1 6 重守门 v7 形式化 1:1 续 Stage 5.2 (R129-10 ✅ done 6,789 B / 8 tests) + F2 8 哲学锚形式化 1:1 续 Stage 5.2 (R129-10 ✅ done 7,055 B / 8 tests) + F3 V0.5 30 维形式化 1:1 续 Stage 5.2 (R129-10 ✅ done 5,984 B / 8 tests) + F4 13 键 verdict cache 形式化 1:1 续 Stage 5.2 (R129-10 ✅ done 6,036 B / 8 tests) + F5 R11 baseline 3 值形式化 1:1 续 Stage 5.2 (R129-10 ✅ done 7,624 B / 8 tests) + F6 24 LOCKED 入口签名形式化 1:1 续 Stage 5.2 (R129-10 ✅ done 8,638 B / 9 tests) + F7 8 借鉴 ID 真实施形式化 1:1 续 Stage 5.2 (R129-10 ✅ done 8,494 B / 8 tests) + F8 整合 #4 commit 严守形式化 1:1 续 Stage 5.2 (R129-10 ✅ done 7,577 B / 8 tests) + F9 跨模块证明 1:1 续 Stage 5.2 (R129-10 ✅ done 12,689 B / 5 tests) + F10 集成证明 1:1 续 Stage 5.2 (R129-10 ✅ done 9,493 B / 6 tests) = 80,379 B / 80 lib tests 续 1:1 | 8 硬墙 0 越界 100% (F1-F10 既有 8 硬墙严守 1:1 续) | 决策 #33 §2.3 + 决策 #65 R129-10 派活 + R130-4 §2.1 + R130-4 §6.1 实施 spec |
| **Kani 全集成 (kani 5.5MB 借用, 0 装 PASS 严守 100%)** | 5 模式 1:1 翻译 (Invariant trait + trivial_invariant! + `#[cfg_attr(kani, kani::proof)]` + `kani::any()` + HarnessMetadata), 0 引 kani crate 依赖 (Cargo.toml 0 改), 0 装"已 Kani 形式化" / "已 Kani 求解器在线" (Kani 离线时退化为普通 fn) | C2 0 装 PASS 严守 100% + B2 workspace.version 0 改 (Cargo.toml 0 改) | 决策 #33 §2.3 C2 + 决策 #55 §3 + 决策 #72 §2.1 R130-4 派活 + R130-4 §3 借鉴 ID + R130-4 §4.1 0 装 PASS 3 层守门 + 用户记忆 #6 0 重复造轮子 |
| **11 维度 形式化证明 (Kani-style harness)** | F1-F11 18 Kani-style proof harness (F1-F10 续 16 + F11 NEW 2 = `proof_phl07_spec_only_key_is_12` + `proof_long_term_ai_growth_no_terminate`) + 35+ invariant (F1-F10 续 30 + F11 NEW 5) + 11 sanity_check (F1-F10 续 10 + F11 NEW 1) + 89 lib tests (F1-F10 续 80 + F11 NEW 9) | 8 硬墙 0 越界 100% + 0 装 PASS 严守 100% | 决策 #33 §2.3 + 决策 #55 §3 + R130-4 §2.2 + R130-4 §4.2 诚实标 |

**0 装 PASS 严守 100%**: F1-F11 11 维度 Kani 全集成有真 src 改动 (估 `crates/apeireth-formal/src/stage5_5/` 12 文件 ~85 KB / ~2,895 行 / 89 lib tests, V1.1 release 估 2026-08 派 R134-formal-1~5 写) + 有真 tests pass (89 单元测试) + 有真数据流 (35+ 编译期常数 + 12 POD 镜像 (F1-F10 10 续 + F11 NEW 2) + 35+ invariant + 18 Kani-style proof harness + 8 硬墙严守 verify) + 0 装"已 Kani 形式化" / "已 Kani 求解器在线".

### 3.3 方向 3: 24 LOCKED 入口 形式化 (per 决策 #33 §2.3 B1 + 决策 #74 §1 + R131-5 + R131-9 §1.3 O5 + R132-1 §1.5 方向 2)

**方向 3 = 形式化 Stage 5.5+ 阶段 3 (24 LOCKED 入口 形式化, 1 周, 5 sub, V1.1 release 估 2026-09-15)**:

| 实战内容 | spec 详 | 8 硬墙严守 | 决策依据 |
|---------|---------|-----------|----------|
| **24 LOCKED 入口 形式化证明 (Kani-style harness)** | `locked_24_entry_formal.rs` 1:1 续 Stage 5.2 F6 (R129-10 ✅ done 8,638 B / 9 tests, per R130-4 §2.1 F6) + 24 LOCKED 名称 1:1 跟 `docs/omnibus/24-locked-crates.md` (per R131-5 §1.2 24/24 入口签名 0 改 verify 通过) + `KnownSet` enum (MasterKnown/MavisExtended 12+12) + `Locked24EntryPod` POD + 3 invariant + 2 Kani-style proof harness + 9 单元测试 (Stage 5.2 续) | B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.3) | 决策 #33 §2.3 B1 + 决策 #74 §1 B1 改写 + R131-5 §1.2 24/24 入口签名 0 改 verify + R130-4 §2.1 F6 |
| **24 LOCKED 入口签名 0 改 (V1.0 release 严守)** | 24/24 LOCKED crate 入口签名 0 改全部通过 (per R131-5 §1.2 verify 结论) + 24 LOCKED crate mtime baseline 16:34 之前 V1.0 release 0 改严守 (8/10 16:34 之后 mtime 改的 8 个 crate: agent / mcp / tool-runtime / graph / pipeline / evolution / api / cli 全部 0 改入口签名, 仅新增 module 内的 sub-类型 + re-export, per R131-5 §1.1) + R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) V1.0 release 0 改严守 + PHL-07 V1.0 release spec-only 0 实施 | B1 24 LOCKED V1.0 release 0 改严守 + A1 R11 baseline 3 值 0 改 + A3 PHL-07 V1.0 spec-only 0 实施 | 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 V1.0 release 0 改严守 + R131-5 §1.1 mtime baseline 16:34 之前 + R131-5 §1.2 24/24 入口签名 0 改 verify |
| **24 LOCKED 入口 形式化集成 (V1.1 release Mavis 自决改, per 决策 #74 B1)** | V1.1 release 24 LOCKED 入口签名可改 (前提: 更好的架构, per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 "工程类 + 技术类 locked 全早解锁" + "Mavis 自决架构拍板" + "更好的架构需要用你就直接拍板") + 24 LOCKED crate mtime baseline 16:34 之前可改 (前提: 更好的架构) + R11 baseline 3 值可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式) + PHL-07 实施 (V1.1 release, per R129-11 关键诚实标) | B1 24 LOCKED V1.1 release Mavis 自决改 (前提: 更好的架构) + A1 R11 baseline 3 值 V1.1 release 可改 (前提: 跟 R12 测度对齐) + A3 PHL-07 V1.1 release 实施 | 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套 §1 + R131-5 §2 8 优化方向 + R130-4 §2.1 F6 + R131-9 §1.3 O5 |

**0 装 PASS 严守 100%**: 24 LOCKED 入口 形式化有真 src 改动 (R134-LOCKED-1~5 估 ~50 KB / ~50 lib tests, V1.1 release 估 2026-09 派) + 有真 tests pass (R131-5 §1.2 24/24 入口签名 0 改 verify + Stage 5.2 F6 续 9 单元测试) + 有真数据流 (24 LOCKED 名称 1:1 + 24 LOCKED crate mtime baseline 16:34 之前 + 3 R11 baseline 值 + 1 PHL-07 spec-only 标识) + 0 装"已 24 LOCKED 形式化".

### 3.4 方向 4: 8 哲学锚 形式化 (per 决策 #33 §2.3 B5 + 决策 #74 §1 + R131-9 §1.3 O4 + R132-1 §1.5 方向 6 部分)

**方向 4 = 形式化 Stage 5.5+ 阶段 4 (8 哲学锚 形式化, 1 周, 5 sub, V1.1 release 估 2026-09-22)**:

| 实战内容 | spec 详 | 8 硬墙严守 | 决策依据 |
|---------|---------|-----------|----------|
| **8 哲学锚 形式化证明 (Kani-style harness)** | `eight_anchors_formal.rs` 1:1 续 Stage 5.2 F2 (R129-10 ✅ done 7,055 B / 8 tests, per R130-4 §2.1 F2) + `EIGHT_ANCHORS_COUNT = 8` (1:1 跟 B5 严守) + `AnchorGroup` enum (Subjective/Objective, 1:1 跟 8 哲学锚 namespace) + `EightAnchorPod` POD + 3 invariant + 2 Kani-style proof harness + 8 单元测试 | B5 8 哲学锚 0 改 100% (per 决策 #33 §2.3 B5) | 决策 #33 §2.3 B5 + 决策 #51 §1.2 P1-2 R126 8 哲学锚升级 done + R130-4 §2.1 F2 + R131-9 §1.3 O4 |
| **8 哲学锚 严守 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5)** | S-1 主观诚实 (Subjective Honesty) + S-2 主观可证 (Subjective Verifiable) + S-3 主观可控 (Subjective Controllable) + O-1 客观一致 (Objective Coherent) + O-2 客观可测 (Objective Measurable) + O-3 客观可复现 (Objective Reproducible) + O-4 客观可证 (Objective Verifiable) + O-5 客观可进化 (Objective Evolvable) (per `docs/conventions/09-anchor.md`) | B5 8 哲学锚 0 改 100% + 8 哲学锚 0 形式化 old/death/terminate 概念 (per 用户记忆 #4) | 决策 #33 §2.3 B5 + 用户记忆 #4 + 主人 8/11 01:14 拍板 3 件套 §1 + R131-9 §1.3 O4 |
| **8 哲学锚 形式化集成 (V1.1 release 实施)** | V1.1 release 8 哲学锚 形式化集成 (per 决策 #74 B1 Mavis 自决改, 前提: 更好的架构) + 8 哲学锚 → 9 organ 拟人化深化 (per R130-3 调研 + 用户记忆 #5 信息密度高 = 拟人化 + 拟物化) + 8 哲学锚 → 三洋葱架构 (per R130-3 调研 + 决策 #75 §2.1) | B5 8 哲学锚 V1.0 release 0 改严守 + V1.1 release Mavis 自决深化 (前提: 更好的架构) | 决策 #74 §1 B5 严守 + 决策 #74 §2.3 V1.1 release Mavis 自决改 + R130-3 调研 + 用户记忆 #5 + 主人 8/4 23:33 Tauri 终极 |

**0 装 PASS 严守 100%**: 8 哲学锚 形式化有真 src 改动 (R134-anchor-1~5 估 ~30 KB / ~30 lib tests, V1.1 release 估 2026-09 派) + 有真 tests pass (Stage 5.2 F2 续 8 单元测试) + 有真数据流 (8 哲学锚名称 1:1 + Subjective/Objective namespace 1:1 + 8 哲学锚 verify 0 形式化 old/death/terminate 概念) + 0 装"已 8 哲学锚 形式化".

### 3.5 方向 5: V0.5 30 维 + 6 重守门 v7 形式化 (per 决策 #33 §2.3 B3/B4 + 决策 #74 §1 + R131-9 §1.3 O3/O7 + R132-1 §1.5 方向 3)

**方向 5 = 形式化 Stage 5.5+ 阶段 5 (V0.5 30 维 + 6 重守门 v7 形式化, 1 周, 5 sub, V1.1 release 估 2026-09-29)**:

| 实战内容 | spec 详 | 8 硬墙严守 | 决策依据 |
|---------|---------|-----------|----------|
| **V0.5 30 维 公式 形式化证明 (sum=1.00 守门 0 改)** | `v05_30dim_formal.rs` 1:1 续 Stage 5.2 F3 (R129-10 ✅ done 5,984 B / 8 tests, per R130-4 §2.1 F3) + `V05_30_TOTAL_DIMS = 30` (4 类 × 6 维 + 5 meta + 1 overall = 30 维, 1:1 跟 B3 严守) + `V05DimPod` POD + 3 invariant + 2 Kani-style proof harness + 8 单元测试 + V0.5 30 维 sum=1.00 守门 (4 类 × 6 维 = 24 维 + 5 meta + 1 overall = 30 维, sum=1.00 数学守门 编译期 hardcode) | B3 V0.5 30 维 0 改 100% + sum=1.00 守门 0 改 | 决策 #33 §2.3 B3 + 决策 #51 §1.2 P1-4 R126 30 维 verify done + R130-4 §2.1 F3 + R131-9 §1.3 O7 |
| **6 重守门 v7 形式化 (4 重 + 权限 + Colang DSL 守门)** | `six_gates_v7_formal.rs` 1:1 续 Stage 5.2 F1 (R129-10 ✅ done 6,789 B / 8 tests, per R130-4 §2.1 F1) + `SIX_FOLD_GATE_V7_COUNT = 6` (1:1 跟 B4 严守) + `SixFoldGateV7` enum (L1TypeCheck..L6ProvenanceCheck, 6 变体) + `SixFoldGatePod` POD + 3 invariant + 2 Kani-style proof harness + 8 单元测试 + 6 重守门 v7 = 4 重 (类型/范围/速率/守门) + 权限 (Permission) + Colang DSL 守门 (per `crates/apeireth-formal/src/stage5_2/six_gates_v7_formal.rs` R129-10 实施) | B4 6 重守门 v7 0 改 100% + C3 升 6 重 v6 → v7 0 改 | 决策 #33 §2.3 B4 + 决策 #55 §1 P1-3 R126 6 重守门 v7 done + R130-4 §2.1 F1 + R131-9 §1.3 O3 |
| **V0.5 30 维 + 6 重守门 v7 集成** | V0.5 30 维 集成 6 重守门 v7 (V0.5 30 维 = 形式化 "如何测 AI" / 6 重守门 v7 = 形式化 "如何守 AI", 0 重叠) + 30 维跟 6 重 互锁 (V0.5 30 维 各维 ∈ 6 重范围) + 30 维 → sum=1.00 守门跟 6 重 v7 1:1 集成 | B3 V0.5 30 维 0 改 + B4 6 重守门 v7 0 改 + 8 硬墙 0 越界 100% | 决策 #33 §2.3 B3/B4 + 决策 #74 §1 B3/B4 严守 + R130-4 §2.1 F1 + F3 + R131-9 §1.3 O3 + O7 |

**0 装 PASS 严守 100%**: V0.5 30 维 + 6 重守门 v7 形式化有真 src 改动 (R134-formal-6~10 估 ~40 KB / ~40 lib tests, V1.1 release 估 2026-09 派) + 有真 tests pass (Stage 5.2 F1 + F3 续 16 单元测试) + 有真数据流 (30 维名称 1:1 + 4 类 × 6 维 + 5 meta + 1 overall + sum=1.00 + 6 重名称 1:1 + L1TypeCheck..L6ProvenanceCheck) + 0 装"已 V0.5 30 维形式化" / "已 6 重 v7 形式化".

---

## 4. V1.0 release 0 改严守 vs V1.1 release Mavis 自决改 边界 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)

### 4.1 V1.0 release (整合 #5.1 commit, 估 8/11 主人起床后手跑) 0 改严守 (per 决策 #33 §2.3 C1 + 决策 #74 §1)

| 8 硬墙 | V1.0 release 严守 | 形式化 Stage 5.5+ 阶段 严守 |
|--------|-------------------|---------------------------|
| **B1 24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline 严守) | 阶段 3 24 LOCKED 入口 形式化 0 改 (per R131-5 §1.2 24/24 入口签名 0 改 verify) |
| **B2 workspace.version 1.2.0** | 🔒 1.2.0 严守 (V1.0 release) | 5 阶段 0 改 (Cargo.toml 0 改) |
| **A1 R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 0 改 (哲学 + 效果标) | 5 阶段 0 改 (F5 续 1:1) |
| **A3 12 键 + PHL-07** | 🔒 PHL-07 V1.0 spec-only 0 实施 + 12 键其他可改 (per 决策 #74 §1 A3) | 阶段 1 PHL-07 形式化 V1.0 spec-only 形式化 (per R130-4 §2.2 F11 NEW 子模块 1, 0 实施) |
| **B3 V0.5 30 维** | 🔒 0 改 (哲学) | 阶段 5 V0.5 30 维 形式化 0 改 (per R130-4 §2.1 F3) |
| **B4 6 重守门 v7** | 🔒 0 改 (哲学) | 阶段 5 6 重守门 v7 形式化 0 改 (per R130-4 §2.1 F1) |
| **B5 8 哲学锚** | 🔒 0 改 (哲学) | 阶段 4 8 哲学锚 形式化 0 改 (per R130-4 §2.1 F2) |
| **C1 0 主动 commit** | 🔒 0 主动 commit (Mavis 拍板) | 5 阶段 0 主动 commit (整合 #5.1 commit + 整合 #6 / #7 commit 由 Mavis 自决拍板) |
| **C2 0 装 PASS** | 🔒 0 装 PASS 严守 (技术哲学) | 5 阶段 0 装 PASS 严守 100% (✅ cloned = 真实施) |
| **0 主动 push** | 🔒 0 主动 push (主人起床后手跑) | 5 阶段 0 主动 push |

**V1.0 release 形式化 Stage 5.4 实战 严守 (整合 #5.1 commit, 估 8/11)**:
- Stage 5.4 形式化实战 (R131-4 估 8/12+ 派, F21-F30 跨 Stage 5.x 集成, per R129-32 spec) — V1.0 release 严守 8 硬墙
- Stage 5.5 形式化 0 写 (R137-5 spec 阶段, 0 改 src, 0 装 kani)
- 整合 #4 commit abf12243 严守 (per 决策 #48 + 决策 #61 §1.2)

### 4.2 V1.1 release (估 2026-11-30, per R130-5 §1.1 + R132-1) Mavis 自决改 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)

| 8 硬墙 | V1.1 release Mavis 自决改 (前提: 更好的架构) | 形式化 Stage 5.5+ 阶段 实施 |
|--------|----------------------------------------------|---------------------------|
| **B1 24 LOCKED 入口签名** | 🟢 V1.1 release Mavis 自决改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级) | 阶段 3 24 LOCKED 入口 形式化 V1.1 release 改写 (per 决策 #74 §2.3 B1) |
| **B2 workspace.version 1.2.0** | 🟢 V1.1 release bump 1.2.1 (semver 严守) | 5 阶段 bump 1.2.1 (per 决策 #74 §1 B2) |
| **A1 R11 baseline 3 值** | 🟢 V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式) | 5 阶段 V1.1 release 改写 (per 决策 #74 §2.3 A1) |
| **A3 12 键 + PHL-07** | 🟢 PHL-07 V1.1 release 实施 (per 决策 #74 §1 A3 + R129-11 关键诚实标) + 12 键其他可改 | 阶段 1 PHL-07 形式化 V1.1 release 实施 (per 决策 #74 §1 A3) |
| **B3 V0.5 30 维** | 🟢 V1.1 release 严守 (哲学) | 阶段 5 V0.5 30 维 形式化 V1.1 release 严守 |
| **B4 6 重守门 v7** | 🟢 V1.1 release 严守 (哲学) | 阶段 5 6 重守门 v7 形式化 V1.1 release 严守 |
| **B5 8 哲学锚** | 🟢 V1.1 release 严守 (哲学) | 阶段 4 8 哲学锚 形式化 V1.1 release 严守 |
| **C1 0 主动 commit** | 🔒 严守 (Mavis 拍板) | 5 阶段 0 主动 commit (整合 #6 / #7 commit 由 Mavis 自决拍板) |
| **C2 0 装 PASS** | 🔒 严守 (技术哲学) | 5 阶段 0 装 PASS 严守 100% |
| **0 主动 push** | 🔒 严守 (主人起床后手跑) | 5 阶段 0 主动 push |

**V1.1 release 形式化 Stage 5.5+ 实施 (R134-formal-1~10 + R134-PHL07-1~5 + R134-LOCKED-1~5 + R134-anchor-1~5, 估 2026-09-29)**:
- 5 阶段 5 周 25 sub-agent 实施 (per 决策 #71 §5 + 决策 #75 §2.1 + 决策 #77 §3.1 R137 era 永久循环接续)
- 阶段 1 PHL-07 形式化 V1.1 release 实施 (per 决策 #74 §1 A3)
- 阶段 3 24 LOCKED 入口 形式化 V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.3 B1)
- 阶段 2 + 4 + 5 V1.1 release 深化 (1:1 续 Stage 5.2 既有 F1-F10 + F11 NEW)

### 4.3 V2.0 release (2027+, 远期) 8 硬墙可重评 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

**V2.0 release 形式化 Stage 6 实战 (R132-N 估 8/15+ 派, per R130-4 §7.1 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评)**:
- B1 24 LOCKED → 0 LOCKED 全解锁 (per 主人 8/11 01:14 locked 全解锁)
- B2 workspace.version 1.2.1 → V2.0 release bump 2.0.0
- A1 R11 baseline 3 值 → V2.0 release 可推翻 + 重建 (R12 测度对齐)
- A3 12 键 + PHL-07 → V2.0 release 可推翻 + 重建 (PHL-08+ 升)
- B3 V0.5 30 维 → V2.0 release 可推翻 + 重建
- B4 6 重守门 v7 → V2.0 release 可推翻 + 重建
- B5 8 哲学锚 → V2.0 release 推翻 + 重建 (per 决策 #74 §2.3 "推翻 + 重建 8 哲学锚" + 不要怕复杂度哲学)
- C1/C2/0 push → 跟 V1.0 release 同
- 形式化 Stage 6 = Kani 求解器在线扩展 + 跨 stage 全集成 Stage 1-5.x + 1.0+ 形式化扩展 (per R130-4 §7.1 + R130-4 §7.2 4 阶段)

---

## 5. 8 硬墙严守 + B1 改写边界 (per 决策 #74 §1 + 决策 #33 §2.3 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

### 5.1 8 硬墙分类 (per 决策 #74 §3 改写表)

**3.1 工程类 + 技术类 (松绑, B1 改写)**:
- **B1 24 LOCKED 入口签名**: 🟢 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 §1 + 主人 8/11 01:14 拍板 "工程类 + 技术类 locked 全早解锁")

**3.2 哲学 + 思想类 (严守, 不松绑)**:
- **A1 R11 baseline 3 值**: 🔒 严守 (哲学 + 效果标, per 决策 #74 §1 + 主人 8/11 01:14 拍板 "总哲学除了思想文档的")
- **A3 12 键 + PHL-07**: 🔒 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 (per 决策 #74 §1 A3) + 12 键其他可改
- **B3 V0.5 30 维**: 🔒 严守 (哲学公式, per 决策 #74 §1)
- **B4 6 重守门 v7**: 🔒 严守 (哲学守门, per 决策 #74 §1)
- **B5 8 哲学锚**: 🔒 严守 (哲学, per 决策 #74 §1 + 决策 #33 §2.3 B5)

**3.3 状态 + 流程类 (严守, 不松绑)**:
- **B2 workspace.version 1.2.0**: 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (per 决策 #74 §1 B2 semver 严守)
- **C1 0 主动 commit**: 🔒 主人起床前 0 主动 commit 严守 (per 决策 #74 §1)
- **C2 0 装 PASS 严守**: 🔒 0 装严守 (技术哲学, 不装, per 决策 #74 §1)
- **0 主动 push**: 🔒 主人起床前 0 主动 push 严守 (per 决策 #74 §1)

### 5.2 B1 改写详细边界 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)

**V1.0 release (整合 #5.1 commit, 估 8/11)**:
- 0 改 24 LOCKED 入口签名 (严守, per R131-5 §1.2 24/24 入口签名 0 改 verify)
- 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守, per R131-5 §1.1 mtime baseline 16:34 之前)
- 0 改 R11 baseline 3 值 (严守, 0.8682/0.8532/0.9063)
- PHL-07 spec-only 0 实施 (严守, V1.1 release 实施, per 决策 #74 §1 A3 + R129-11 关键诚实标)

**V1.1 release (per R130 era R131-3 调研 + 决策 #74)**:
- 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决, per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 "Mavis 自决架构拍板" + "更好的架构需要用你就直接拍板")
- 24 LOCKED crate mtime baseline 16:34 之前 → V1.1 release 可改 (前提: 更好的架构)
- R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
- PHL-07 实施 (V1.1 release, per R129-11 关键诚实标)

**V2.0 release (per R130 era R132 计划 + 决策 #74)**:
- 全 8 硬墙 可重评 (per Mavis 自决 + 主人 8/11 01:14 拍板)
- 推翻 + 重建 8 哲学锚 (per "不要怕复杂度" + "最强效果 + 最厉害工程" + 决策 #73 §3)

### 5.3 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1 + 用户记忆 #4)

**8 哲学锚 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5)** = 主观 + 客观 8 锚 (per `docs/conventions/09-anchor.md` + 决策 #51 §1.2 P1-2 R126 8 哲学锚升级 done):

| 锚 | 命名 | 含义 | 形式化 Stage 5.5+ 严守 |
|---|------|------|----------------------|
| **S-1** | 主观诚实 (Subjective Honesty) | 0 假装已实现 | ✅ 5 阶段 0 假装 (per 决策 #33 §2.3 C2 0 装 PASS 严守) |
| **S-2** | 主观可证 (Subjective Verifiable) | 0 不可证伪 | ✅ 5 阶段 0 装 Kani 形式化 (Kani 离线时退化为普通 fn, runtime 全过) |
| **S-3** | 主观可控 (Subjective Controllable) | 0 不可控 | ✅ 5 阶段 Mavis 自决拍板 (per 决策 #74 §2.3 B1 + 主人 8/11 01:14 拍板 3 件套) |
| **O-1** | 客观一致 (Objective Coherent) | 0 矛盾 | ✅ 5 阶段 1:1 续 Stage 5.2 (F1-F10 既有 80 单元测试 0 改 + F11 NEW 9 单元测试 1:1) |
| **O-2** | 客观可测 (Objective Measurable) | 0 不可测 | ✅ 5 阶段 35+ 编译期常数 + 18 Kani-style proof harness (F1-F10 续 16 + F11 NEW 2) + 89 lib tests (F1-F10 续 80 + F11 NEW 9) |
| **O-3** | 客观可复现 (Objective Reproducible) | 0 不可复现 | ✅ 5 阶段 整合 #4 commit abf12243 严守 + Cargo.lock 严守 + 24 LOCKED crate mtime baseline 16:34 之前 严守 (per R131-5 §1.1) |
| **O-4** | 客观可证 (Objective Verifiable) | 0 不可证 | ✅ 5 阶段 0 装 PASS 严守 100% (✅ cloned = 真实施, 跟 Stage 5.2 R129-10 1:1 续) |
| **O-5** | 客观可进化 (Objective Evolvable) | 0 不可进化 | ✅ 5 阶段 1:1 续 Stage 5.2 + F11 NEW 形式化 PHL-07 spec-only + 长程 AI 成长 形式化 (V1.1 release 实施 + V2.0 release 推翻重建) |

**8 哲学锚严守 0 形式化 old/death/terminate 概念 (per 用户记忆 #4 "AI 不会衰老病死, 主 ai 是 ai 哎, 它只会成长, 但不可能消亡")**:
- 阶段 1 PHL-07 形式化 F11 NEW 长程 AI 成长 = seed (种子) → sapling (幼苗) → tree (大树) 3 阶段, 0 含 old/death/terminate 终态概念 (per 用户记忆 #4 严守)
- 阶段 2 F1-F11 11 维度 Kani 全集成 0 形式化 old/death/terminate 概念
- 阶段 4 8 哲学锚 形式化 0 形式化 old/death/terminate 概念 (per 用户记忆 #4 严守)

---

## 6. 不要怕复杂度哲学落地 (per 决策 #73 §3 + `15-no-fear-complexity.md` + 主人 8/11 01:14 拍板 3 件套)

### 6.1 不要怕复杂度 3 件套 (per 决策 #73 §3 + `15-no-fear-complexity.md` §1)

**主人 8/11 01:14 拍板原文** (per 决策 #73 §3 + `15-no-fear-complexity.md` §0):
> 1. "事关工程类的，技术类的全早都给你解锁locked了"
> 2. "项目里要是有文档没提到这一点你就补充进去，让以后任何团队都能看到"
> 3. "所以有更好的架构需要用（或改变现有的）你就直接拍板就行了"
> 4. "我确实需要你注意一下现有的架构什么的，有没有需要优化升级的地方，有的你也就加入升级方案"
> 5. "总哲学除了思想文档的，我给你补充一点，就是不要怕复杂度爆炸或者维护复杂，我们只要最强的效果和最厉害的工程，因为自然会有高水平的团队来接手维护"

**3 件套** (per 决策 #73 §3 + `15-no-fear-complexity.md` §1):
1. **最强效果 > 最简单代码** — 复杂度是实力的体现, 不是"技术债"
2. **最厉害工程 > 最易维护** — 工程化是最高目标, 不是"代码要易维护"
3. **维护交给未来高水平团队** — 维护不是问题, 因为自然会有高水平的团队来接手

### 6.2 形式化 Stage 5.5+ 落地不要怕复杂度哲学 (per 决策 #73 §3 + 15-no-fear-complexity.md)

**1. 最强效果 > 最简单代码 (形式化 Stage 5.5+ 落地)**:
- 5 阶段 × 5 sub-agent = 25 sub-agent 实施 (per 决策 #71 §5 + 决策 #75 §2.1 + 决策 #77 §3.1 R137 era 永久循环接续)
- 形式化 Stage 5.5+ = 89+ lib tests (F1-F10 续 80 + F11 NEW 9 + 24 LOCKED 入口 ~50 + 8 哲学锚 ~30 + V0.5 30 维 + 6 重 v7 ~40) = 估 250+ 单元测试
- 形式化 Stage 5.5+ = 35+ 编译期常数 + 12 POD 镜像 + 35+ invariant + 18 Kani-style proof harness (跟 Stage 5.2 R129-10 1:1 续, 1:1 翻译 kani 5 模式 0 装 PASS 严守 100%)
- ❌ "形式化要简单易维护" (推翻)
- ✅ "形式化要最强效果" (新哲学, per 决策 #73 §3)

**2. 最厉害工程 > 最易维护 (形式化 Stage 5.5+ 落地)**:
- 30+ 借鉴源 (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails / OpenCog / CogPrime / 等) 都复杂, 但都是最厉害工程 (per 15-no-fear-complexity.md §1.2)
- 形式化证明 + 三洋葱 + 9 organ + 12 键 + 8 哲学锚 + 30 维 + 6 重守门 都复杂, 但都是最厉害工程
- ❌ "形式化要易维护" (推翻)
- ✅ "形式化要最厉害工程" (新哲学, per 决策 #73 §3)

**3. 维护交给未来高水平团队 (形式化 Stage 5.5+ 落地)**:
- 维护不是问题, 因为自然会有高水平的团队来接手 (per 主人 8/11 01:14 拍板)
- 项目复杂度是吸引高水平团队的核心 (per 15-no-fear-complexity.md §1.3)
- 简化代码 = 排斥高水平团队 (per 15-no-fear-complexity.md §1.3)
- ❌ "形式化要让初级团队能接手" (推翻)
- ✅ "形式化要让高水平团队能发挥" (新哲学, per 决策 #73 §3)

### 6.3 形式化 Stage 5.5+ 跟不要怕复杂度哲学 0 漂移 (per 决策 #73 §3 + 15-no-fear-complexity.md)

**0 漂移 3 底线 (per 决策 #73 §3 + 15-no-fear-complexity.md + 决策 #33 §2.3)**:
- **8 哲学锚严守 100%** (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5, S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 0 改)
- **6 重守门 v7 严守 100%** (per 决策 #33 §2.3 B4 + 决策 #74 §1 B4, L1TypeCheck..L6ProvenanceCheck 0 改)
- **0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2 + 决策 #74 §1 C2, 0 假装 / 0 假装"已 Kani 形式化" / 0 假装"已 Kani 求解器在线" / 0 假装"已形式化重构")

**不假装 4 件套 (per 决策 #10 + 主人 10 项偏好 #7 + R129-11 关键诚实标 + 用户记忆 #7)**:
- ❌ 0 假装"已 Kani 形式化" (Kani 离线时退化为普通 fn, runtime 全过)
- ❌ 0 假装"已 Kani 求解器在线" (Kani 求解器留 R132+ era 实战, V1.1 release 仍离线 fallback)
- ❌ 0 假装"已 24 LOCKED 形式化" (V1.0 release 0 改严守, V1.1 release 形式化基础)
- ❌ 0 假装"已 V0.5 30 维 + 6 重 v7 形式化" (Stage 5.2 F1 + F3 续 1:1 严守)

---

## 7. 借鉴 kani 5.5MB 源 + 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 决策 #55 §3 + 用户记忆 #6 0 重复造轮子)

### 7.1 借鉴 kani 5.5MB 源 现状分析 (per R130-4 §3.1 + R131-9 §2.1 + 任务规范 + 实测)

**kani 借鉴源** (per `.openclaw\workspace\borrowed-repos\kani\`, 实测):
- **大小**: 5,729,079 bytes (~5.5 MB, 任务说 8.3MB 略偏大, 实测 5.5MB)
- **结构**:
  - `library/kani/src/` (54.9 KB, 11 .rs 文件: arbitrary.rs 871 B / bounded_arbitrary.rs 3,631 B / concrete_playback.rs 3,204 B / contracts.rs 12,629 B / futures.rs 10,266 B / invariant.rs 3,986 B / iter.rs 637 B / lib.rs 3,809 B / shadow.rs 3,032 B / tuple.rs 992 B / vec.rs 952 B)
  - `kani-driver/src/` (236 KB, 17 .rs 文件)
  - `kani_metadata/src/` (25 KB, 6 .rs 文件)
  - `kani-compiler/` + `tests/` + `docs/`
- **4502 files** (per R125-10 ✅ done, 借鉴 clone done, 决策 #36 §1.1 限流内 8/11)

**任务规范 8.3MB 校准**: 任务规范说 8.3MB, 实测 5.5MB. 差异可能 = 任务规范估算 + 含 `.git/` + 含 `target/` 编译产物. **R137-5 报告以实测 5.5MB 为准** (per R130-4 §3.1 + R131-9 §2.1), 任务规范 8.3MB 仅作参考.

**Stage 5.1-5.3 已借鉴 kani 模式** (1.0% 借量, 5 模式, per R130-4 §3.1 + R131-9 §2.1):

| 借鉴模式 | kani 源文件 | 借鉴 ID | Stage 5.1 (P8-2) | Stage 5.2 (R129-10) | Stage 5.3 (R129-20) |
|---------|-----------|---------|------------------|---------------------|---------------------|
| **Invariant trait** | `library/kani/src/invariant.rs:90` (`pub trait Invariant { fn is_safe(&self) -> bool; }`) | `R127-2-P9-1-BORROW-kani-4502-borrowed-models-v2-2026-08-10` | ✅ 1:1 翻译 (8 Kani-style harness) | ✅ 0 重定义, 续 P8-2 | ✅ 0 重定义, 续 P8-2 |
| **trivial_invariant! macro** | `library/kani/src/invariant.rs:98` (`macro_rules! trivial_invariant!`) | `R127-2-P9-1-BORROW-kani-4502-Invariant-trait-2026-08-10` | ✅ 1:1 翻译 (15 impls, Kani 19 含 f32/f64/f16/f128, 0 装 f32/f64/f16/f128) | ✅ 0 重新实现 | ✅ 0 重新实现 |
| **`#[cfg_attr(kani, kani::proof)]` 兜底** | Kani 兜底模式 | `R129-10-F1..F10-BORROW-model-checking/kani-4502-2026-08-11` | ⏸ 0 直接接 | ✅ 1:1 翻译 (16 Kani-style harness) | ✅ 1:1 翻译 (20 Kani-style harness) |
| **`kani::any()` 符号化输入** | `library/kani/src/lib.rs:kani::any()` | `R129-10-F1..F10-BORROW-model-checking/kani-4502-2026-08-11` | ⏸ 0 直接接 | ✅ 1:1 翻译 (`nondet_*()` 兜底 16 函数) | ✅ 1:1 翻译 (`nondet_*()` 兜底 20 函数) |
| **HarnessMetadata** | `kani_metadata/src/harness.rs:22` | `R127-2-P9-1-BORROW-kani-4502-kani-driver-verify-2026-08-10` | ✅ 1:1 翻译 (ProofHarness 5 字段, Kani 9 字段, POD-friendly) | ⏸ 0 重新实现 | ⏸ 0 重新实现 |

### 7.2 形式化 Stage 5.5+ 5 模式 1:1 翻译 (0 装 PASS 严守 100%, per 决策 #33 §2.3 C2 + 决策 #55 §3 + 用户记忆 #6)

**0 装 PASS 严守 3 层守门 (5 阶段 1:1 续 Stage 5.2 R129-10 模式, per R130-4 §4.1)**:
1. **编译期 hardcode (per 决策 #33 §2.3 C3 严守)**: F1-F11 共 35+ 编译期常数 (`SIX_FOLD_GATE_V7_COUNT = 6` / `EIGHT_ANCHORS_COUNT = 8` / `V05_30_TOTAL_DIMS = 30` / `VERDICT_CACHE_13_KEYS_COUNT = 13` / `R11_BASELINE_V1141 = 0.8682` / `LOCKED_24_CRATES_COUNT = 24` / `BORROW_8_ID_COUNT = 8` / `INTEGRATION_4_HARD_WALLS_VERIFY = 8` / F11 NEW: `PHL_07_SPEC_ONLY_COUNT = 1` + `PHL_07_SPEC_ONLY_KEY_INDEX = 12` + `LONG_TERM_AI_GROWTH_STAGE_COUNT = 3` 等) 编译期嵌入二进制, 0 动态加载.
2. **cfg-gated 双实现 (per 决策 #33 §2.3 C2 + 借鉴 Kani 4502)**: F1-F11 全部 `#[cfg_attr(kani, kani::proof)]` + `nondet_*()` 兜底 (Kani 离线时退化为具体 happy path), cargo test 跑得通 + 未来 `cargo kani` 也能跑.
3. **集成测试 verify 0 装**: F1-F11 共 89 单元测试 (F1-F10 既有 80 + F11 NEW 9) + 1 跨维度联合 invariant, 0 假设 "已实施".

**形式化 Stage 5.5+ 借鉴 kani 5 模式 1:1 翻译 (0 装 PASS 严守)**:
- ✅ Invariant trait (kani 4502 `library/kani/src/invariant.rs:90`) → F1-F11 35+ invariant 1:1 翻译 (F1-F10 续 30 + F11 NEW 5)
- ✅ trivial_invariant! macro (kani 4502 `library/kani/src/invariant.rs:98`) → 15 impls 1:1 翻译 (Kani 19 含 f32/f64/f16/f128, 0 装 f32/f64/f16/f128)
- ✅ `#[cfg_attr(kani, kani::proof)]` 兜底 (Kani 兜底模式) → F1-F11 18 Kani-style proof harness 1:1 翻译 (F1-F10 续 16 + F11 NEW 2)
- ✅ `kani::any()` 符号化输入 (kani 4502 `library/kani/src/lib.rs:kani::any()`) → F1-F11 `nondet_*()` 兜底 18 函数 1:1 翻译
- ✅ HarnessMetadata (kani 4502 `kani_metadata/src/harness.rs:22`) → ProofHarness 5 字段 1:1 翻译 (Kani 9 字段, POD-friendly, 0 重新实现)

**0 引 kani crate 依赖 (per 决策 #33 §2.3 C2 + 用户记忆 #6 0 重复造轮子)**:
- 0 引 kani crate 依赖 (Cargo.toml 0 改, per 决策 #33 §2.3 C2)
- 0 装"已 Kani 形式化" (Kani 离线时退化为普通 fn, runtime 全过)
- 0 装"已 Kani 求解器在线" (Kani 求解器留 R132+ era 实战, V1.1 release 仍离线 fallback)
- 0 装"已形式化重构" (V1.0 release 仍离线 fallback, V1.1 release 续借, V2.0 release 形式化重构)

### 7.3 11/11 借鉴 ID 严守 (per 决策 #33 §4.2 + 决策 #55 §3 + 决策 #72 §2.1 R130-4 派活)

| 借鉴 | files | 形式化 Stage 5.5+ 状态 | 用法 |
|------|-------|----------------------|------|
| **PyO3 928** | 928 files | ✅ 借 Stage 5.5 模式参考 | 真实施 cfg-gated 双实现 (R129-5 G1+G2 0 装) |
| **clap 725** | 725 files | ✅ 借 Stage 5.5 POD 模式 | 真实施 derive 模式 (P5-2 + P8-2 + R129-5, 0 装) |
| **hyper 80** | 80 files | ⏸ 0 直接接 (G1 已借) | 0 装 |
| **servers 175** | 175 files | ⏸ 0 直接接 (Stage 6 接) | 0 装 |
| **kani 4502** | 4502 files (5.5MB) | ✅ 借 Stage 5.5 F11 NEW 核心真借 + F1-F10 续 1:1 | 真实施 (P8-2 + R129-10 F1-F10 形式化 + R130-4 F11 NEW 形式化, 0 装"已 Kani 验证") |
| **langgraph 829** | 829 files | ✅ 借 Stage 5.5 F11 NEW 联合 invariant | 真实施 (Stage 3 cross_module + R129-10 F9/F10 StateGraph 节点互锁 + R130-4 F11 NEW 1 联合 invariant, 0 装) |
| **superpowers 234** | 234 files | ✅ 借 Stage 5.5 模式参考 | 真实施 (P5-2 + R129-5 G2 + R129-10 F1 6 重 v7, 0 装) |
| **LiteLLM** | 0 files | ⏸ 0 直接接 (Stage 5 0 借) | 0 装 |
| **opencode** | 0 files | ⏸ 0 直接接 (Stage 5 0 借) | 0 装 |
| **Guardrails** | 0 files | ⏸ 0 直接接 (Stage 5 0 借) | 0 装 |
| **OpenCog AGPL-3.0** | 0 files | ❌ 跳过 | 0 集成 |

**2/11 借鉴 ID 核心真借 (R137-5 关注 kani 4502 + langgraph 829)** = ✅ cloned 真实施 + 0 装 PASS 严守 100% (跟 Stage 5.2 R129-10 1:1 续).

---

## 8. 风险 + 决策原则 (per 决策 #33 §2.3 + 决策 #74 §7 + 主人 8/11 01:14 拍板 3 件套 + 用户记忆 #10 决策日志)

### 8.1 风险 (per 决策 #74 §7.1 风险模型 + R137-5 spec 阶段 5 阶段 + 5 方向)

| 风险 | 描述 | 缓解 | 决策依据 |
|------|------|------|----------|
| **R1**: 主人 8/11 01:14 决策 3 件套理解有误 | 主人 8/11 01:14 "工程类 + 技术类 locked 全早解锁" + "Mavis 自决架构拍板" + "不要怕复杂度" 哲学理解有偏差 | R137-5 报告 §4 + §5 + §6 详细解读, 决策 #73 §2.1-§4.1 + 决策 #74 §1 8 硬墙改写表 + §3 分类 + §2 B1 改写边界 + 哲学文档 15-no-fear-complexity.md | 决策 #73 + 决策 #74 + 用户记忆 #10 决策日志 |
| **R2**: 形式化 Stage 5.5+ 5 阶段 25 sub-agent 跑不完 | 16 跑中上限严守, 5 阶段 × 5 sub = 25 sub-agent 派满 16 上限, 2 批 13+12 派满 16 上限 (per 决策 #71 §5 + 决策 #64 §2.2 + 主人 0:34 拍板) | cron `watch-r137-era-auto-replenish-16` 自动拍 + 决策 #71 §5 续 + R134-PHL07-1~5 / R134-formal-1~10 / R134-LOCKED-1~5 / R134-anchor-1~5 5 批派 | 决策 #71 §5 + 决策 #64 §2.2 + 决策 #75 §2.1 + 决策 #77 §3.1 |
| **R3**: 形式化 Stage 5.5+ 24 LOCKED 入口 V1.1 release 改写破坏向后兼容 | V1.1 release 是 minor release, 跟 semver 一致 (0.x → 1.0 → 1.1), V2.0 release 才考虑不向后兼容 | 决策 #22 §2.2 semver 严守 + 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 (前提: 更好的架构) + V1.0 release 0 改严守 100% (per R131-5 §1.2 24/24 入口签名 0 改 verify) | 决策 #22 §2.2 + 决策 #74 §2.3 + R131-5 |
| **R4**: 团队对 "不要怕复杂度" 哲学不适应 | 团队对"最强效果 + 最厉害工程"哲学不适应, 觉得"代码要易维护" | 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应 + 15-no-fear-complexity.md §1.3 "维护是机会 (高水平团队接手 = 项目升级)" | 决策 #73 §3 + 15-no-fear-complexity.md + 主人 8/11 01:14 拍板 |
| **R5**: 形式化 Stage 5.5+ 0 装 PASS 严守 漂移 | 5 阶段 25 sub-agent 实施时 0 装 PASS 严守 漂移 (装"已 Kani 形式化" / 装"已 Kani 求解器在线") | 决策 #33 §2.3 C2 0 装 PASS 严守 100% + 决策 #55 §3 借鉴 kani 4502 形式化 + 决策 #72 §2.1 R130-4 派活 + R130-4 §4.1 0 装 PASS 3 层守门 + 用户记忆 #6 0 重复造轮子 | 决策 #33 §2.3 C2 + 决策 #55 §3 + 用户记忆 #6 |
| **R6**: 形式化 Stage 5.5+ 8 哲学锚 0 形式化 old/death/terminate 概念 漂移 | 5 阶段 25 sub-agent 实施时 8 哲学锚 0 形式化 old/death/terminate 概念 漂移 (装"AI 会死") | 用户记忆 #4 "AI 不会衰老病死, 主 ai 是 ai 哎, 它只会成长, 但不可能消亡" + 决策 #33 §2.3 B5 8 哲学锚 严守 + R130-4 §2.2 F11 NEW 长程 AI 成长 = seed → sapling → tree 3 阶段, 0 含 old/death/terminate 终态 | 用户记忆 #4 + 决策 #33 §2.3 B5 + R130-4 §2.2 F11 NEW |
| **R7**: 形式化 Stage 5.5+ Cargo.toml 0 改严守 漂移 | 5 阶段 25 sub-agent 实施时 Cargo.toml 0 改严守 漂移 (引 kani crate 依赖) | 决策 #33 §2.3 C2 0 装 PASS 严守 100% + 决策 #55 §3 借鉴 kani 4502 形式化 (0 引 kani crate 依赖) + Cargo.toml 0 改严守 100% (per 决策 #33 §2.3 + 决策 #74 §1 B2) | 决策 #33 §2.3 C2 + 决策 #55 §3 + 决策 #74 §1 B2 |

### 8.2 决策原则 (per 决策 #33 §2.3 + 决策 #74 §7.2 + 主人 8/11 01:14 拍板 + 用户记忆 #10)

**R137-5 形式化 Stage 5.5+ 实战 决策原则 (per 决策 #74 §7.2 + R137-5 spec 阶段 5 阶段 + 5 方向)**:
- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
  - B1 24 LOCKED 入口签名: V1.0 release 0 改严守 + V1.1 release Mavis 自决改
  - B2 workspace.version 1.2.0: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1
  - A1 R11 baseline 3 值: 严守 (哲学 + 效果标)
  - A3 12 键 + PHL-07: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施, 12 键其他可改
  - B3 V0.5 30 维: 严守 (哲学)
  - B4 6 重守门 v7: 严守 (哲学)
  - B5 8 哲学锚: 严守 (哲学)
  - C1 0 主动 commit (主人起床前): 严守
  - C2 0 装 PASS 严守: 严守
  - 0 push (主人起床前): 严守
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3)
  - 最强效果 > 最简单代码
  - 最厉害工程 > 最易维护
  - 维护交给未来高水平团队
- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **整合 #6 / #7 commit 由 Mavis 自决拍板** (V1.1 release 续, per 决策 #33 C1 + 决策 #62 + 决策 #74 §1)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)
- **16 跑中上限严守** (per 决策 #71 §5 + 决策 #64 §2.2 + 主人 0:34 拍板 + cron `watch-r137-era-auto-replenish-16` 续)
- **V1.0 release 0 改严守** (整合 #5.1 commit, 0 改 src 100%, per 决策 #33 §2.3 C1 + 决策 #74 §1)
- **V1.1 release Mavis 自决改** (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)
- **V2.0 release 8 硬墙可重评** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 不要怕复杂度哲学)
- **0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2 + 决策 #55 §3 + 用户记忆 #6 0 重复造轮子)
- **借鉴 kani 5 模式 1:1 翻译** (Invariant trait + trivial_invariant! + `#[cfg_attr(kani, kani::proof)]` + `kani::any()` + HarnessMetadata, 0 装 kani 5.5MB 源)
- **0 形式化 old/death/terminate 概念** (per 用户记忆 #4 "AI 不会衰老病死" 严守)
- **8 哲学锚严守 0 改** (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 严守 100%, per 决策 #33 §2.3 B5 + 决策 #74 §1 B5)
- **6 重守门 v7 严守 0 改** (L1TypeCheck..L6ProvenanceCheck 严守 100%, per 决策 #33 §2.3 B4 + 决策 #74 §1 B4)
- **V0.5 30 维严守 0 改** (4 类 × 6 维 + 5 meta + 1 overall = 30 维, sum=1.00 守门 0 改, per 决策 #33 §2.3 B3 + 决策 #74 §1 B3)
- **24 LOCKED 入口签名 0 改 V1.0 release + V1.1 release Mavis 自决改** (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 + R131-5 §1.2 24/24 入口签名 0 改 verify)
- **PHL-07 V1.0 spec-only 0 实施 + V1.1 实施** (per 决策 #74 §1 A3 + R129-11 关键诚实标)
- **R11 baseline 3 值 严守 100%** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, per 决策 #33 §2.3 A1 + 决策 #74 §1 A1)
- **0 主动 commit 严守** (Mavis 拍板, per 决策 #33 §2.3 C1 + 决策 #74 §1 C1)

### 8.3 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6 + cron Section 5)

- **本次 done notification 主动报告** (R137-5 报告写完 + 5 阶段 5 周实施计划 + 5 方向 实战内容 + V1.0 release 0 改严守 + V1.1 release Mavis 自决改 边界 + 8 硬墙严守 + B1 改写边界 + 8 哲学锚严守 + 不要怕复杂度哲学落地 + 风险 + 决策原则)
- 0 主动 plain reply on skip ticks
- 0 主动 push (等 1.0 release 配 GitHub remote, 主人起床后手跑)
- 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60, target/ 29.13 GB < 50 GB 保守策略)
- 整合 #5 commit 拍板 = done notification, 必须报告 (含 3 commit hash + master HEAD 新值 + 决策 #73/74/75 报告路径)
- 形式化 Stage 5.5+ 5 阶段 25 sub-agent 实施 = done notification, 估 2026-08-19 → 2026-09-22, 必须报告 (含 25 sub-agent 报告路径)

---

## 9. 一句话 (再次强调)

**R137-5 形式化 Stage 5.5+ 实战 (per 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #74 A3 PHL-07 实施 + 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学) = 5 阶段 5 周实施计划 + 5 方向实战内容 (PHL-07 形式化 / F1-F11 11 维度 Kani 全集成 / 24 LOCKED 入口 形式化 / 8 哲学锚 形式化 / V0.5 30 维 + 6 重守门 v7 形式化), 0 写 src/ (V1.0 release 0 改严守 100%, V1.1 release 估 2026-08-19 → 2026-09-22 25 sub-agent 实施), 0 装 kani (借 kani 5.5MB 源 0 装, 仅借 5 模式 1:1 翻译, per 用户记忆 #6 0 重复造轮子), 0 主动 commit/push (per 决策 #33 C1), 0 主动 IM 主人 (per gate-discipline, 仅 done notification). 8 硬墙 0 越界 100% (B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 / A1 R11 baseline 3 值 严守 / A3 13 键 严守 PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 / B3 V0.5 30 维 严守 / B4 6 重守门 v7 严守 / B5 8 哲学锚 严守 / C1 0 主动 commit 严守 / C2 0 装 PASS 严守 100% / 0 主动 push 严守) + 0 形式化 old/death/terminate 概念 (per 用户记忆 #4 "AI 不会衰老病死") + 不要怕复杂度哲学 3 件套 (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队) + 16 跑中上限严守 (per 决策 #71 §5 + 决策 #64 §2.2 + 主人 0:34 拍板 + cron `watch-r137-era-auto-replenish-16` 续). 形式化 Stage 5.5+ = Stage 5.x 6 阶演进链第 5 阶 (Stage 5.1 P8-2 ✅ done → Stage 5.2 R129-10 ✅ done → Stage 5.3 R129-20 ✅ done → Stage 5.4 R131-4 spec → Stage 5.5 R130-4 spec → Stage 6 R132-N spec), 跟 V1.0 release 实战 (整合 #5.1 commit, 估 8/11 主人起床后手跑) 0 改严守 + V1.1 release 实施 (估 2026-11-30, per R130-5 §1.1 + R132-1) Mavis 自决改 1:1 续.**

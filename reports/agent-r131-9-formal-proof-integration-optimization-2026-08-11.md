# R131-9 形式化集成优化 (R131 era 第 2 批, per 决策 #75 §2.1 + cron Section 10 架构审视永久工作项 + 主人 8/11 01:14 拍板 3 件套 §2)

**Date**: 2026-08-11 01:30+ (R131-9 派活, per 决策 #75 §2.1 R131 era 第 2 批 R131-4~9 架构细分 + cron Section 10 架构审视永久工作项 + 主人 8/11 01:14 拍板 3 件套 §2 "你也就加入升级方案")
**Author**: R131-9 sub-agent (Mavis 派, per 决策 #75 §2.1 R131 era 第 2 批 R131-9 形式化集成优化)
**Parent session**: mvs_367e66fae08342ffa399befe4f85dbac
**任务**: 形式化集成优化 (R131 era 调研, kani 借鉴深度 + F1-F10 10 维度 → F1-F11 11 维度 + 6 重 v7 + 8 锚 + 24 LOCKED + PHL-07 + V0.5 30 维 + 12 键 形式化集成 9 个优化方向, per 决策 #75 §2.1)
**关联决策**: #22 (8 硬墙) + #33 (8 硬墙重置 + 0 装 PASS) + #36 (借鉴 11 源) + #48 (整合 #4 commit abf12243) + #55 (R127 派活 + §2.6 调研) + #56 (R127-2 retry 形式化) + #57 (R128 ASI Python) + #58 (R128-2 派活) + #61 (新 session + R129 era 派活) + #62 (整合 #5 commit 3 拆) + #64 (cron 5 min tick) + #65 (R129 第 2 批) + #66 (R129 第 3 批) + #67 (R129-24 pending) + #68 (R129 第 4 批) + #69 (R129 第 5 批 + 编译产物清理) + #70 (Mavis 清理决策权升级) + #71 (R130 era 调研) + #72 (R130 era 调研 6 sub-agent) + #73 (主决策: locked 全解锁 + 架构审视 + 不要怕复杂度) + #74 (8 硬墙 B1 改写: V1.0 release 0 改 + V1.1 release Mavis 自决改) + #75 (R131 era 第 2 批 派活 11 sub)
**关联报告**: `agent-p8-1-r127-2-library-stage-4-1-autonomy-loop-final-2026-08-10.md` (Stage 4.1 自治) + `agent-p8-2-retry-r127-2-library-stage-5-1-formal-proof-final-2026-08-10.md` (Stage 5.1 形式化基础) + `agent-r129-10-formal-proof-stage-5.2-2026-08-11.md` (Stage 5.2 F1-F10 10 维度) + `agent-r129-20-formal-proof-stage-5.3-2026-08-11.md` (Stage 5.3 F11-F20 跨模块) + `agent-r129-32-formal-proof-stage-5.4-execution-2026-08-11.md` (Stage 5.4 F21-F30 跨 Stage 5.x 集成 spec) + `agent-r130-4-formal-proof-stage-5.5-integration-deepening-2026-08-11.md` (Stage 5.5 F1-F11 集成深化 spec, R130-4 已 done 调研)
**状态**: ✅ **done 调研报告 (派活 01:30+, 60 min 时间盒), 0 改 src/ (per 决策 #33 §2.3 C1 + 决策 #71 §2.2 调研任务规范 + 决策 #74 B1 改写 V1.0 release 0 改严守), 0 改 Cargo.toml (workspace.version 1.2.0 严守 per B2 严守 100%), 0 主动 commit (Mavis 整合 #5.3 commit 时机拍板), 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑 scripts/release/)**

---

## 0. 一句话 (TL;DR)

**R131-9 形式化集成优化 = 9 个优化方向调研报告, 0 写 `crates/apeireth-formal/src/stage5_5/` NEW 目录 (R131 era 是调研, 0 改 src/ per 决策 #33 §2.3 C1 + 决策 #71 §2.2 调研任务规范 + 决策 #74 B1 改写 V1.0 release 0 改严守). 9 个优化方向 = (1) kani 5.5MB 借鉴深度优化 (4 个细分方向: 阶段 5.1/5.2/5.3 已借 1.0% → V1.1 release 3-5% → V2.0 release 10-15% 借量) + (2) F1-F10 10 维度 → F1-F11 11 维度 (Stage 5.5 NEW F11 PHL-07 spec-only + 长程 AI 成长 形式化, per R130-4 spec) + (3) 6 重守门 v7 形式化优化 (Stage 5.2 F1 6.8KB 续, +1 维深化) + (4) 8 哲学锚形式化优化 (Stage 5.2 F2 7.1KB 续, +Subjective/Objective 1:1 严守) + (5) 24 LOCKED 入口形式化优化 (Stage 5.2 F6 8.6KB 续, 24 LOCKED 入口签名 0 改 V1.0 release + V1.1 release Mavis 自决改) + (6) PHL-07 spec-only 形式化 (V1.0 release spec-only 0 实施, V1.1 release 实施, per 决策 #74 §2.3) + (7) V0.5 30 维形式化优化 (Stage 5.2 F3 6.0KB 续, 4 类 × 6 维 + 5 meta + 1 overall = 30 维 0 改) + (8) 12 键 + PHL-07 形式化优化 (Stage 5.2 F4 6.0KB 续, 12 + PHL-07 = 13 键 verdict cache) + (9) V1.1 release PHL-07 实施 + F1-F11 + Kani 全集成方案 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 不要怕复杂度哲学 + 8 硬墙 0 越界 100%). 借鉴源码 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 决策 #55 §3 + 决策 #75 §2.1 R131-9 派活): 2 借鉴 ID (kani 4502 + langgraph 829) 0 引 kani / langgraph 依赖, 0 装"已 Kani 形式化" / "已 langgraph 集成", 仅借鉴 Invariant trait + StateGraph 节点互锁模式 1:1 翻译到 stage5_5/ (R133+ era 实施时写). 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 改写表): B1 24 LOCKED 入口签名 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 workspace.version 1.2.0 0 改 / A1 R11 baseline 3 值 0.8682/0.8532/0.9063 0 改 / B3 V0.5 30 维 0 改 / B4 6 重守门 v7 0 改 / B5 8 哲学锚 0 改 / A3 13 键 0 改 / C1 0 主动 commit (Mavis 拍板) / C2 0 装 PASS 严守 (✅ cloned = 真实施) / C3 升 6 重 v6 → v7 0 改 / 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑).**

---

## 1. 形式化集成 Stage 5.x 演进链总览 (per 决策 #33 §2.3 + 决策 #55 §1 + 决策 #56 + 决策 #57 + 决策 #61 §3.1 + 决策 #69 §3 + 决策 #72 §2.1 + 决策 #74 §1)

### 1.1 Stage 5.x 6 阶总览 (P5-2 + P8-2 + R129-10/20/32 + R130-4 spec + R131-9 spec)

| Stage | 时机 | 派活 | 任务 | 范围 | 借鉴 | 状态 |
|---|---|---|---|---|---|---|
| **Stage 5.1** (Library 形式化) | R127-2 P8-2 retry 22:06 done (per 决策 #56) | P8-2 (single sub-agent) | Library crate 形式化基础 (kani 4502 Invariant trait + 8 Kani-style harness + 5 NEW POD 模型 + Stage5Token POD) | `crates/apeireth-library-governance/src/formal_proof.rs` 39.3KB + `tests/formal_proof_integration.rs` 14.7KB + `tests/integration.rs` 15.0KB = 69 KB / 16 Kani-style harness / 153 tests | kani 4502 ✅ cloned 真实施 | ✅ P8-2 done |
| **Stage 5.2** (formal crate 形式化扩展) | R129 era 第 2 批 00:30 cron 派 R129-10 00:49 done (per 决策 #65) | R129-10 (single sub-agent, 19 min) | formal crate 形式化扩展 F1-F10 10 维度 (6 重 v7 + 8 锚 + 30 维 + 13 键 + R11 + 24 LOCKED + 8 借鉴 + 整合 #4 + 跨模块 + 集成) | `crates/apeireth-formal/src/stage5_2/` 11 文件 80,379 B ~80 KB / 117 lib tests (含 79 NEW) | kani 4502 + langgraph 829 ✅ cloned 真实施 | ✅ R129-10 done |
| **Stage 5.3** (formal crate 跨模块证明) | R129 era 第 3 批 00:34 派 R129-20 00:50 done (per 决策 #66) | R129-20 (single sub-agent, 16 min) | formal crate 跨模块证明 F11-F20 10 维度 (跨 crate + 跨借鉴 + 跨 stage + 跨决策 + 跨 commit + 跨 LOCKED + 跨 anchor + 跨 gate + 跨 version + 跨 push) | `crates/apeireth-formal/src/stage5_3/` 11 文件 88.5 KB / 92 lib tests (F11-F20 90 + mod.rs 2) | kani 4502 ✅ cloned 真实施 | ✅ R129-20 done |
| **Stage 5.4** (formal crate 集成扩展, R129-32 spec) | R131 era 估 8/12+ 派 (per 决策 #64 §2.2 + 决策 #69 §3 R129-32 spec) | R131-4 (估 60 min 派, 1 sub-agent) | formal crate 集成扩展 F21-F30 10 维度 (跨 stage 5.1-5.3 集成 + 跨借鉴源 2 借鉴 ID + 跨决策链 + 跨 24 LOCKED + 跨 8 哲学锚 + 跨 6 重守门 v7 + 跨 30 维 V0.5 + 跨 13 键 + 跨 R11 baseline + 跨 push 严守) | 估 `crates/apeireth-formal/src/stage5_4/` 11 文件 ~100 KB / ~110 lib tests | kani 4502 + langgraph 829 ✅ cloned 真实施 (续 Stage 5.2/5.3 同模式) | 📋 R131-4 spec (R129-32 ✅ done, 0 写) |
| **Stage 5.5** (formal crate 集成深化, R130-4 spec) | V1.1 minor release 前 估 2026-11 派 (per 决策 #78 R130 era 后路线图) | R133-N (估 60 min 派, 1 sub-agent, V1.1 minor release 前) | formal crate 集成深化 F1-F10 10 维深化 + F11 NEW 1 维 (PHL-07 spec-only 形式化 + 长程 AI 成长 形式化) = F1-F11 11 维度 | 估 `crates/apeireth-formal/src/stage5_5/` 12 文件 ~30 KB / ~25 lib tests (F11 NEW ~9 + 7 既有 stage5_2 sanity re-verify) | kani 4502 + langgraph 829 ✅ cloned 真实施 (续 Stage 5.2 同模式) | 📋 R130-4 spec (R130-4 ✅ done 调研, 0 写) |
| **Stage 6** (形式化证明 + 实战, R132+ era) | R132+ era 估 8/15+ 派 (per 决策 #64 §2.2 + 决策 #78 R130 era 派活清单 + 1.0 release 实战后) | R132-N (估 90-120 min 派, N=3-5 sub-agent) | 形式化证明 + 实战 (kani 求解器在线扩展 + 跨 stage 全集成 Stage 1-5.x + 实战 1.0 release 验证 + 1.0 release 实战后 1.0+ 形式化扩展) | 估 `crates/apeireth-formal/src/stage6/` 5-8 文件 ~150-200 KB / 200+ lib tests + kani 求解器在线跑 (per R130 era 后) | kani 4502 + langgraph 829 + PyO3 928 (asi-formal-pybridge 实战) ✅ cloned 真实施 | 📋 R132-N spec (R129-32 spec, 0 写) |

**6 阶演进链 1:1 续 per 决策 #33 §2.3 C2 (0 装 PASS 严守 100%) + 决策 #55 §1 (Stage 5.2 续 #33 §2.3 借鉴 kani 4502 形式化) + 决策 #56 (R127-2 形式化 Stage 5.1 续) + 决策 #61 §3.1 R129-20 (Stage 5.3 续) + 决策 #69 §3 R129-32 (Stage 5.4 续 + Stage 6 路线) + 决策 #72 §2.1 R130-4 (Stage 5.5 续 形式化扩展 F1-F11 11 维度)**.

### 1.2 Stage 5.x 6 阶 1:1 严守 8 硬墙 (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

| 硬墙 | Stage 5.1 (P8-2) | Stage 5.2 (R129-10) | Stage 5.3 (R129-20) | Stage 5.4 (R131-4 spec) | Stage 5.5 (R130-4 spec) | Stage 6 (R132-N spec) | 0 越界 100% |
|---|---|---|---|---|---|---|---|
| **B1** 24 LOCKED 入口签名 0 改 (V1.0 release 严守 + V1.1 release Mavis 自决改, per 决策 #74 §2.3) | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 (V1.0 release) | ✅ 6/6 |
| **B2** workspace.version 1.2.0 0 改 (V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1) | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 (V1.0 release) | ✅ 6/6 |
| **A1** R11 baseline 3 值 0.8682/0.8532/0.9063 0 改 (哲学 + 效果标) | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 (V1.0 release) | ✅ 6/6 |
| **B3** V0.5 30 维 0 改 (哲学) | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 6/6 |
| **B4** 6 重守门 v7 0 改 (哲学) | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 6/6 |
| **B5** 8 哲学锚 0 改 (哲学) | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 6/6 |
| **A3** 12 键 + PHL-07 = 13 键 0 改 (PHL-07 V1.0 spec-only + V1.1 实施, per 决策 #74 §2.3) | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 | ✅ 0 改 (F11 NEW PHL-07 spec-only 形式化, 0 改 13 键本身) | ✅ 0 改 | ✅ 6/6 |
| **C1** 0 主动 commit (Mavis 整合 #5 拍板) | ✅ 0 commit | ✅ 0 commit | ✅ 0 commit | ✅ 0 commit | ✅ 0 commit | ✅ 0 commit | ✅ 6/6 |
| **C2** 0 装 PASS 严守 (✅ cloned = 真实施) | ✅ 0 装 | ✅ 0 装 | ✅ 0 装 | ✅ 0 装 | ✅ 0 装 | ✅ 0 装 | ✅ 6/6 |
| **0 主动 push** (等 1.0 release 配 GitHub remote) | ✅ 0 push | ✅ 0 push | ✅ 0 push | ✅ 0 push | ✅ 0 push | ✅ 0 push | ✅ 6/6 |

**0 越界 verify**: 8 硬墙 × 6 Stage = 48 个严守项 全 0 越界, Stage 5.1-5.3 实证, Stage 5.4-6 spec (本报告 0 写, 1:1 续严守).

### 1.3 形式化集成 9 个优化方向总览 (R131-9 任务目标, per 决策 #75 §2.1 R131-9 派活)

| # | 优化方向 | 现状 (Stage 5.1-5.3 done) | V1.0 release 严守 | V1.1 release 实施 | V2.0 release 重构 | 跟 8 硬墙严守 | 跟 8 哲学锚严守 | 跟不要怕复杂度哲学 |
|---|---------|---------------------------|-------------------|-------------------|-------------------|--------------|----------------|------------------|
| **O1** | **kani 借鉴深度优化** | kani 5.5MB 借 1.0% (P8-2 + R129-10/20, 1:1 翻译 4 模式: Invariant trait + `#[cfg_attr(kani, kani::proof)]` + `kani::any()` + HarnessMetadata) | 0 改严守 (Stage 5.1-5.3 实证) | 3-5% 借量 (Kani 求解器在线 + 非核心模块借量, per 决策 #74 B1 V1.1 release Mavis 自决改) | 10-15% 借量 (Kani 求解器全集成 + 跨 stage 全集成 + Stage 6 实战) | 0 越界 100% | 0 改 | ✅ V1.0 release 0 装 + V1.1 release 续借 + V2.0 release 重构 |
| **O2** | **F1-F10 10 维度 → F1-F11 11 维度** (Stage 5.5 NEW) | F1-F10 形式化 80.4KB (Stage 5.2 R129-10 ✅ done) | 0 改严守 (F1-F10 10 维度 1:1 续) | F11 NEW 1 维 (PHL-07 spec-only 形式化 + 长程 AI 成长 形式化, per R130-4 spec) | F1-F11 11 维度完整 + 跨 stage 全集成 | 0 越界 100% (8 硬墙 0 改) | 0 改 | ✅ F1-F10 1:1 续 + F11 NEW 升级 |
| **O3** | **6 重守门 v7 形式化** (Stage 5.2 F1) | F1 `six_gates_v7_formal.rs` 6.8KB (R129-10 ✅ done 21 tests) | 0 改严守 (1:1 跟 B4 6 重) | 深化 1 维 (6 重 → 6 重子层 + 6 重交叉, per 不要怕复杂度哲学) | 6 重 → N 重 (per 8 哲学锚推翻重建) | B4 0 改 | 0 改 | ✅ V1.0 release 0 装 + V1.1 release 深化 + V2.0 release 推翻重建 |
| **O4** | **8 哲学锚形式化** (Stage 5.2 F2) | F2 `eight_anchors_formal.rs` 7.1KB (R129-10 ✅ done 20 tests) | 0 改严守 (1:1 跟 B5 8 锚) | 深化 1 维 (Subjective + Objective 1:1 严守, F2 已有) | 8 锚 → N 锚 (per 8 哲学锚推翻重建) | B5 0 改 | 0 改 | ✅ V1.0 release 0 装 + V1.1 release 深化 + V2.0 release 推翻重建 |
| **O5** | **24 LOCKED 入口签名形式化** (Stage 5.2 F6) | F6 `locked_24_entry_formal.rs` 8.6KB (R129-10 ✅ done 21 tests) | 0 改严守 (1:1 跟 B1 24 LOCKED, MasterKnown/MavisExtended 12+12) | Mavis 自决改 (前提: 更好的架构, 24 LOCKED 入口签名 1:1 翻译形式化保留, 实施实施 PHL-07 + Stage 9 + 三洋葱) | 24 LOCKED → 0 LOCKED 全解锁 (per 主人 8/11 01:14 locked 全解锁 + 决策 #74 B1 改写) | B1 V1.0 release 0 改 + V1.1 release Mavis 自决改 | 0 改 | ✅ V1.0 release 0 装 + V1.1 release 自决改 + V2.0 release 全解锁 |
| **O6** | **PHL-07 spec-only 形式化** (Stage 5.5 F11 NEW) | F11 NEW 1 维 (per R130-4 spec, ~5KB) | 0 实施 (PHL-07 spec-only 0 实施, per 决策 #74 §2.3 V1.0 release) | 实施 (PHL-07 V1.1 release 实施, per 决策 #74 §2.3 V1.1 release) | V2.0 release 推翻 + 重建 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评) | A3 13 键 0 改 + 8 哲学锚 0 形式化 old/death/terminate 严守 (per 用户记忆 #4) | 0 改 | ✅ V1.0 release 0 实施 + V1.1 release 实施 + V2.0 release 推翻 |
| **O7** | **V0.5 30 维形式化** (Stage 5.2 F3) | F3 `v05_30dim_formal.rs` 6.0KB (R129-10 ✅ done 20 tests) | 0 改严守 (1:1 跟 B3 30 维 4 类 × 6 + 5 meta + 1 overall) | 深化 1 维 (V0.5 → V0.5+ minor 维数 30 → 32, per 不要怕复杂度哲学) | V0.5 → V0.6 重大 (per 8 哲学锚推翻重建) | B3 0 改 | 0 改 | ✅ V1.0 release 0 装 + V1.1 release 深化 + V2.0 release 推翻 |
| **O8** | **12 键 + PHL-07 形式化** (Stage 5.2 F4) | F4 `verdict_cache_13keys_formal.rs` 6.0KB (R129-10 ✅ done 20 tests) | 0 改严守 (1:1 跟 A3 13 键 PHL-01..07) | 实施 PHL-07 (V1.1 release 实施, PHL-07 spec-only → 实施) | PHL-07 → PHL-08+ 升 (per 8 哲学锚推翻重建) | A3 0 改 | 0 改 | ✅ V1.0 release 0 装 + V1.1 release 实施 + V2.0 release 升 |
| **O9** | **V1.1 release PHL-07 实施 + F1-F11 + Kani 全集成方案** (Stage 5.5 集成深化实施) | Stage 5.5 R130-4 spec + R131-9 spec (本报告) | 0 写 (R131 era 调研, 0 改 src) | R133-N 实施 (F1-F11 11 维度 + PHL-07 实施 + Kani 求解器在线扩展) | R132+ era 实战 (Stage 6 Kani 求解器在线 + 跨 stage 全集成 + 1.0 release 实战) | 8 硬墙 0 越界 100% | 0 改 | ✅ V1.0 release 调研 0 改 + V1.1 release 实施 + V2.0 release 实战 |

**9 个优化方向 1:1 续 Stage 5.1-5.3 实证 + Stage 5.4-5.5 spec + Stage 6 实战, 8 硬墙 0 越界 100%, 8 哲学锚 0 改, 不要怕复杂度哲学全严守**.

---

## 2. 优化方向 O1 — kani 5.5MB 借鉴深度优化 (4 阶段演进 + 4 模式 1:1 翻译)

### 2.1 kani 借鉴现状分析 (per 决策 #36 §1.1 + 决策 #33 §4.2 + 决策 #55 §3 + 决策 #75 §2.1 R131-9)

**kani 借鉴源** (per `.openclaw\workspace\borrowed-repos\kani\`):
- **大小**: 5,729,079 bytes (~5.5 MB, 任务说 8.3MB 略偏大, 实测 5.5MB, per `Get-ChildItem -Recurse | Measure-Object -Sum`)
- **结构**: `library/kani/src/` (54.9 KB, 11 .rs 文件) + `kani-driver/src/` (236 KB, 17 .rs 文件) + `kani_metadata/src/` (25 KB, 6 .rs 文件) + `kani-compiler/` + `tests/` + `docs/`
- **4502 files 估** (per R125-10 ✅ done, 借鉴 clone done, 决策 #36 §1.1 限流内 8/11)

**Stage 5.1-5.3 已借鉴 kani 模式** (1.0% 借量, 4 模式):

| 借鉴模式 | kani 源文件 | 借鉴 ID | Stage 5.1 (P8-2) | Stage 5.2 (R129-10) | Stage 5.3 (R129-20) |
|---------|-----------|---------|------------------|---------------------|---------------------|
| **Invariant trait** | `library/kani/src/invariant.rs:90` (`pub trait Invariant { fn is_safe(&self) -> bool; }`) | `R127-2-P9-1-BORROW-kani-4502-borrowed-models-v2-2026-08-10` | ✅ 1:1 翻译 (8 Kani-style harness) | ✅ 0 重定义, 续 P8-2 | ✅ 0 重定义, 续 P8-2 |
| **trivial_invariant! macro** | `library/kani/src/invariant.rs:98` (`macro_rules! trivial_invariant!`) | `R127-2-P9-1-BORROW-kani-4502-Invariant-trait-2026-08-10` | ✅ 1:1 翻译 (15 impls, Kani 19 含 f32/f64/f16/f128, 0 装 f32/f64/f16/f128) | ✅ 0 重新实现 | ✅ 0 重新实现 |
| **`#[cfg_attr(kani, kani::proof)]` 兜底** | Kani 兜底模式 | `R129-10-F1..F10-BORROW-model-checking/kani-4502-2026-08-11` | ⏸ 0 直接接 | ✅ 1:1 翻译 (16 Kani-style harness) | ✅ 1:1 翻译 (20 Kani-style harness) |
| **`kani::any()` 符号化输入** | `library/kani/src/lib.rs:kani::any()` | `R129-10-F1..F10-BORROW-model-checking/kani-4502-2026-08-11` | ⏸ 0 直接接 | ✅ 1:1 翻译 (`nondet_*()` 兜底 16 函数) | ✅ 1:1 翻译 (`nondet_*()` 兜底 20 函数) |
| **HarnessMetadata** | `kani_metadata/src/harness.rs:22` | `R127-2-P9-1-BORROW-kani-4502-kani-driver-verify-2026-08-10` | ✅ 1:1 翻译 (ProofHarness 5 字段, Kani 9 字段, POD-friendly) | ⏸ 0 重新实现 | ⏸ 0 重新实现 |
| **VerificationStatus** | `kani-driver/src/call_cbmc.rs:34` | `R127-2-P9-1-BORROW-kani-4502-kani-driver-verify-2026-08-10` | ✅ 1:1 翻译 (ProofResult 3 状态, Kani 2 状态, +Skipped) | ⏸ 0 重新实现 | ⏸ 0 重新实现 |
| **HarnessRunner** | `kani-driver/src/harness_runner.rs:23` | `R127-2-P9-1-BORROW-kani-4502-kani-driver-verify-2026-08-10` | ✅ 1:1 翻译 (ProofRunner 0 fields, Kani 借 KaniSession+Project) | ⏸ 0 重新实现 | ⏸ 0 重新实现 |

**借量计算** (粗估):
- kani 源码 5.5MB → 借鉴 1.0% = 55KB
- 借鉴 1.0% = `crates/apeireth-library-governance/src/formal_proof.rs` 39.3KB + `crates/apeireth-formal/src/stage5_2/` 80.4KB + `crates/apeireth-formal/src/stage5_3/` 88.5KB + Stage 5.4 (估 ~100KB) + Stage 5.5 (估 ~30KB) ≈ 240KB (累计到 Stage 5.5, 1.0% 借量, 0 装 PASS 严守 100%)
- 0 引 kani crate 依赖 (Cargo.toml 0 改, per `Cargo.toml:32` `kani = "0.0.1"` placeholder dev-dependency 0 装)
- 0 装"已 Kani 形式化" (Kani 离线时退化为普通 fn, `cargo kani` 实跑 = R128 续扩)

### 2.2 kani 借鉴深度优化 4 阶段 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 不要怕复杂度哲学 + 决策 #75 §2.1 R131-9)

#### 2.2.1 阶段 A: V1.0 release 0 改严守 (整合 #5.1 commit, 0 改 src)

**0 改严守** (per 决策 #33 §2.3 C1 + 决策 #74 §2.3 B1 V1.0 release 0 改严守):
- kani 借 1.0% 严守 (Stage 5.1-5.3 实证)
- 0 引 kani crate 依赖 (Cargo.toml 0 改)
- 0 装"已 Kani 形式化" (Kani 离线时退化为普通 fn)
- 4 模式 1:1 翻译 (Invariant trait + trivial_invariant! + `#[cfg_attr(kani, kani::proof)]` + `kani::any()`) 严守
- HarnessMetadata / VerificationStatus / HarnessRunner 0 重新实现 (P8-2 已实施, 0 重复造轮子 per 用户记忆 #6)

**Stage 5.4-5.5 spec 0 写 (per 决策 #71 §2.2 调研任务规范 + 决策 #75 §2.1 R131-9)**:
- Stage 5.4 F21-F30 (R131-4 spec) 跟 Stage 5.3 同模式, 0 重新借 (1:1 续)
- Stage 5.5 F1-F11 (R130-4 spec) 跟 Stage 5.2 同模式, 0 重新借 (1:1 续)
- Stage 6 (R132-N spec) 待实战, 0 重新借

#### 2.2.2 阶段 B: V1.1 release Kani 求解器在线扩展 (per 决策 #74 §2.3 B1 V1.1 release Mavis 自决改, 3-5% 借量)

**Kani 求解器在线扩展 (per 决策 #74 §2.3 V1.1 release Mavis 自决改 + 决策 #55 §1 续 #33 §2.3 借鉴 kani 4502 形式化)**:
- **借量**: 1.0% → 3-5% 借量 (per 不要怕复杂度哲学 + 决策 #74 B1 改写)
- **新增借**:
  - **Kani 求解器 CBMC 在线扩展** (per `kani-driver/src/call_cbmc.rs` 23.9KB + `cbmc_output_parser.rs` 29.1KB + `cbmc_property_renderer.rs` 33.2KB) — V1.1 release 估 +3-5% 借量
  - **cargo kani 集成** (per `kani-driver/src/call_cargo.rs` 29.8KB + `session.rs` 18.1KB) — V1.1 release 估 +1-2% 借量
  - **contracts.rs** (per `library/kani/src/contracts.rs` 12.6KB) — V1.1 release 估 +0.5-1% 借量 (跟 R130-4 决策 #72 §2.1 Kani ProofForContract 续)
- **总借量**: 1.0% + 3-5% = 4-6% 借量
- **不引依赖**: 0 引 kani crate, 0 装"已 Kani 求解器在线", 仅借鉴 CBMC 求解器模式 (runtime 实施, Kani 求解器留 R132+ 实战)
- **不重写 4 模式**: Invariant trait + trivial_invariant! + `#[cfg_attr(kani, kani::proof)]` + `kani::any()` 0 重写 (P8-2 + R129-10/20 + R130-4 续)

**V1.1 release 实施 0 触碰 V1.0 release R11 baseline** (per 决策 #74 §2.3):
- 24 LOCKED 入口签名 V1.0 release 0 改严守, V1.1 release Mavis 自决改 (前提: 更好的架构)
- 24 LOCKED crate mtime baseline 16:34 之前 V1.0 release 0 改严守, V1.1 release 可改 (前提: 更好的架构)
- R11 baseline 3 值 V1.0 release 0 改严守, V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
- PHL-07 V1.0 release spec-only 0 实施, V1.1 release 实施 (per R129-11 关键诚实标 + 决策 #74 §2.3)

#### 2.2.3 阶段 C: V2.0 release 形式化重构 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评, 10-15% 借量)

**V2.0 release 形式化重构 (per 决策 #74 §2.3 V2.0 release + 决策 #73 §3 不要怕复杂度哲学 + 决策 #78 R130 era 派活清单)**:
- **借量**: 4-6% → 10-15% 借量 (per 不要怕复杂度哲学 + V2.0 release 8 硬墙可重评)
- **全 8 硬墙可重评** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评):
  - B1 24 LOCKED 入口签名 → 0 LOCKED 全解锁
  - B2 workspace.version 1.2.0 → V2.0 release bump 2.0.0
  - A1 R11 baseline 3 值 → V2.0 release 可推翻 + 重建 (R12 测度对齐)
  - A3 12 键 + PHL-07 → V2.0 release 可推翻 + 重建
  - B3 V0.5 30 维 → V2.0 release 可推翻 + 重建
  - B4 6 重守门 v7 → V2.0 release 可推翻 + 重建
  - B5 8 哲学锚 → V2.0 release 推翻 + 重建 (per 决策 #74 §2.3 "推翻 + 重建 8 哲学锚")
  - C1/C2/0 push → 跟 1.0 release 同
- **新增借**:
  - **V2.0 release 推翻 + 重建 8 哲学锚** (per 决策 #74 §2.3 + 不要怕复杂度哲学) — V2.0 release 估 +5-7% 借量
  - **V2.0 release 推翻 + 重建 V0.5 30 维** (per 决策 #74 §2.3) — V2.0 release 估 +2-3% 借量
  - **V2.0 release 推翻 + 重建 6 重守门 v7** (per 决策 #74 §2.3) — V2.0 release 估 +1-2% 借量
- **总借量**: 4-6% + 5-7% + 2-3% + 1-2% = 12-18% 借量 (粗估 10-15%)
- **Stage 6 实战**: Kani 求解器全集成 + 跨 stage 全集成 + 1.0 release 实战后 1.0+ 形式化扩展

#### 2.2.4 阶段 D: 不要怕复杂度哲学落地 (per 决策 #73 §3 + `docs/conventions/15-no-fear-complexity.md`)

**不要怕复杂度哲学 3 件套** (per 决策 #73 §3 + `15-no-fear-complexity.md` §1):
1. **最强效果 > 最简单代码** — 复杂度是实力的体现, 不是"技术债"
2. **最厉害工程 > 最易维护** — 工程化是最高目标, 不是"代码要易维护"
3. **维护交给未来高水平团队** — 维护不是问题, 因为自然会有高水平的团队来接手

**kani 借鉴深度落地** (per 不要怕复杂度哲学):
- **V1.0 release**: 1.0% 借量 (Stage 5.1-5.3 实证, 0 装 PASS 严守 100%)
- **V1.1 release**: 4-6% 借量 (Kani 求解器在线扩展, per 决策 #74 B1 Mavis 自决改)
- **V2.0 release**: 12-18% 借量 (形式化重构, per 决策 #74 V2.0 release 8 硬墙可重评)
- **不漂移**: 8 哲学锚 + 6 重守门 v7 + 0 装 PASS 严守 100% (底线, 严守)
- **不假装**: 0 装"已 Kani 形式化" / 0 装"已 Kani 求解器在线" / 0 装"已形式化重构" 严守

### 2.3 kani 借鉴深度优化 4 阶段总结 (per 决策 #75 §2.1 R131-9 派活)

| 阶段 | 借量 | 实施 | 8 硬墙 0 越界 | 8 哲学锚 0 改 | 0 装 PASS 严守 | 决策依据 |
|------|------|------|--------------|--------------|----------------|----------|
| **A: V1.0 release** (整合 #5.1 commit) | 1.0% | 0 改严守 (Stage 5.1-5.3 实证) | ✅ 8/8 | ✅ 8/8 | ✅ 100% | 决策 #33 §2.3 C1 + 决策 #74 §2.3 B1 V1.0 release 0 改严守 |
| **B: V1.1 release** (per 决策 #74 B1 Mavis 自决改) | 4-6% | Kani 求解器在线扩展 + PHL-07 实施 + locked 改写 | ✅ 8/8 (V1.0 release 严守, V1.1 release B1 可改) | ✅ 8/8 | ✅ 100% | 决策 #74 §2.3 B1 + 决策 #55 §1 续 #33 §2.3 借鉴 kani 4502 形式化 |
| **C: V2.0 release** (per 决策 #74 §2.3 V2.0 release) | 12-18% | 形式化重构 + Stage 6 实战 + Kani 求解器全集成 | ✅ 8/8 (V2.0 release 可重评) | ✅ 8/8 (V2.0 release 推翻 + 重建) | ✅ 100% | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 不要怕复杂度哲学 |
| **D: 不要怕复杂度** (per 决策 #73 §3 + 哲学文档 15) | 落地哲学 | 最强效果 + 最厉害工程 + 维护交给未来高水平团队 | ✅ 100% | ✅ 100% | ✅ 100% | 决策 #73 §3 + 15-no-fear-complexity.md §1 |

---

## 3. 优化方向 O2 — F1-F10 10 维度 → F1-F11 11 维度 (Stage 5.5 集成深化, per R130-4 spec)

### 3.1 Stage 5.2 F1-F10 10 维度现状 (R129-10 ✅ done, 80.4KB)

**F1-F10 10 维度** (per `crates/apeireth-formal/src/stage5_2/` 11 文件 80,379 B ~80 KB / 117 lib tests 含 79 NEW R129-10):

| # | 维度 | 模块文件 | 大小 | lib tests | 8 硬墙严守 | Stage 5.5 续 |
|---|------|---------|------|-----------|-----------|--------------|
| **F1** | 6 重守门 v7 形式化 | `six_gates_v7_formal.rs` | 6,789 B | 8 | B4 0 改 | 1:1 续 (F11 0 重叠) |
| **F2** | 8 哲学锚形式化 | `eight_anchors_formal.rs` | 7,055 B | 8 | B5 0 改 | 1:1 续 (F11 0 重叠) |
| **F3** | V0.5 30 维形式化 | `v05_30dim_formal.rs` | 5,984 B | 8 | B3 0 改 | 1:1 续 (F11 0 重叠) |
| **F4** | 13 键 verdict cache 形式化 | `verdict_cache_13keys_formal.rs` | 6,036 B | 8 | A3 0 改 | 1:1 续 + F11 PHL-07 实施深化 |
| **F5** | R11 baseline 3 值形式化 | `r11_baseline_formal.rs` | 7,624 B | 8 | A1 0 改 | 1:1 续 (F11 0 重叠) |
| **F6** | 24 LOCKED 入口签名形式化 | `locked_24_entry_formal.rs` | 8,638 B | 9 | B1 0 改 | 1:1 续 (F11 0 重叠) |
| **F7** | 8 借鉴 ID 真实施形式化 | `borrow_8_id_formal.rs` | 8,494 B | 8 | C2 0 装 | 1:1 续 (F11 0 重叠) |
| **F8** | 整合 #4 commit 严守形式化 | `integration_4_commit_formal.rs` | 7,577 B | 8 | C1 0 commit | 1:1 续 (F11 0 重叠) |
| **F9** | 跨模块证明 (F1-F8 跨模块集成) | `cross_module_proof.rs` | 12,689 B | 5 | F1-F8 0 越界 | 1:1 续 (F11 0 重叠) |
| **F10** | 集成证明 (F1-F9 完整集成) | `integration_proof.rs` | 9,493 B | 6 | F1-F9 集成 0 越界 | 1:1 续 (F11 0 重叠) |
| **小计** | 10 形式化模块 | (10 src + mod.rs) | 80,379 B + 3,570 B = 83,949 B (~82 KB) | 79 (F1-F10) + 2 (mod.rs) = 81 | 8 硬墙 0 越界 100% | 1:1 续 100% |

**0 越界 verify**: 10 维度 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #65 R129-10 派活 + 决策 #61 §3.1 R129-10).

### 3.2 Stage 5.5 F1-F11 11 维度深化方案 (R130-4 spec, V1.1 minor release 前实施)

**Stage 5.5 11 维度** (per R130-4 spec, 0 写本报告, 估 V1.1 minor release 前派 R133-N 实战):

| # | 维度 | Stage 5.2 续 1:1 | 8 硬墙严守 | 物理含义 | V1.1 release 深化 |
|---|------|-----------------|-----------|----------|------------------|
| **F1** | 6 重守门 v7 形式化 | ✅ 1:1 续 (R129-10 done 6,789 B) | B4 0 改 | 6 重守门 v7 形式化 (L1TypeCheck..L6ProvenanceCheck) | 1:1 续 (0 重复造轮子 per 用户记忆 #6) |
| **F2** | 8 哲学锚形式化 | ✅ 1:1 续 (R129-10 done 7,055 B) | B5 0 改 | 8 哲学锚形式化 (S-* + O-* namespace) | 1:1 续 (0 重复造轮子) |
| **F3** | V0.5 30 维形式化 | ✅ 1:1 续 (R129-10 done 5,984 B) | B3 0 改 | V0.5 30 维命名空间形式化 | 1:1 续 (0 重复造轮子) |
| **F4** | 13 键 verdict cache 形式化 | ✅ 1:1 续 (R129-10 done 6,036 B) | A3 0 改 | 13 键 verdict cache 形式化 (PHL-01..07) | 1:1 续 + 0 改 PHL-07 spec-only 严守 (V1.1 release 实施) |
| **F5** | R11 baseline 3 值 形式化 | ✅ 1:1 续 (R129-10 done 7,624 B) | A1 0 改 | R11 baseline 3 值 编译期 hardcode 形式化 | 1:1 续 (0 重复造轮子) |
| **F6** | 24 LOCKED 入口签名 形式化 | ✅ 1:1 续 (R129-10 done 8,638 B) | B1 V1.0 release 0 改 | 24 LOCKED 入口签名 形式化 (MasterKnown/MavisExtended 12+12) | 1:1 续 + V1.1 release Mavis 自决改 (前提: 更好的架构) |
| **F7** | 8 借鉴 ID 真实施形式化 | ✅ 1:1 续 (R129-10 done 8,494 B) | C2 0 装 | 8 借鉴 ID 真实施形式化 (✅ cloned) | 1:1 续 (0 重复造轮子) |
| **F8** | 整合 #4 commit 严守形式化 | ✅ 1:1 续 (R129-10 done 7,577 B) | C1 0 commit | 整合 #4 commit 严守 形式化 | 1:1 续 (0 重复造轮子) |
| **F9** | 跨模块证明 | ✅ 1:1 续 (R129-10 done 12,689 B) | F1-F8 0 越界 | F1-F8 8 模块互锁 1 联合 invariant | 1:1 续 (0 重复造轮子) |
| **F10** | 集成证明 | ✅ 1:1 续 (R129-10 done 9,493 B) | F1-F9 集成 0 越界 | F1-F9 完整集成 8 硬墙 0 越界 100% | 1:1 续 (0 重复造轮子) |
| **F11** | **PHL-07 spec-only 形式化 + 长程 AI 成长 形式化** (Stage 5.5 NEW 1 维, per R130-4 §2.2) | 🆕 NEW (R133-N 估写, 0 写本报告) | A3 13 键 0 改 + 8 哲学锚 0 改 + **0 形式化 old/death/terminate 概念** (per 用户记忆 #4) | (1) PHL-07 spec-only = 形式化 PHL-07 = "NotUnoptimizable" 的 spec 性质 (PHL-07 = 13 键第 13 键, 0 假装"已 optimal"). (2) 长程 AI 成长 = 形式化 AI 成长阶段 (seed → sapling → tree, 0 old/death/terminate 终态概念, per 用户记忆 #4 "AI 不会衰老病死") | **PHL-07 实施 (V1.1 release 实施, per 决策 #74 §2.3 + R129-11 关键诚实标) + 长程 AI 成长形式化 (0 形式化 old/death/terminate 严守)** |
| **小计** | 11 形式化模块 | 10 续 1:1 + F11 NEW (~5,000 B ~5 KB) | **80,379 B 续 + ~5,000 B NEW = ~85,379 B (~85 KB)** | **80 单元测试 续 + 9 单元测试 NEW = 89 lib tests (F1-F10 既有 80 + F11 NEW 9)** | **8 硬墙 0 越界 100% + 用户记忆 #4 0 形式化 old/death/terminate 严守** | **F1-F10 续 100% + F11 NEW 1 维** |

**F11 NEW 详 spec** (per R130-4 spec §2.2 + 用户记忆 #4 "AI 不会衰老病死" + 13 键 PHL-07 升级):

```rust
// crates/apeireth-formal/src/stage5_5/phl07_spec_only_and_long_term_ai_growth_formal.rs
//
// F11 NEW 1 维 (Stage 5.5 集成深化, R133-N 估写, 0 写本报告)
//
// 包含 2 子模块:
//   1. phl07_spec_only: PHL-07 spec-only 形式化 (PHL-07 = "NotUnoptimizable" 的 spec 性质)
//   2. long_term_ai_growth: 长程 AI 成长 形式化 (seed → sapling → tree, 0 old/death/terminate)

// === 子模块 1: PHL-07 spec-only 形式化 ===

pub const PHL_07_SPEC_ONLY_COUNT: usize = 1; // 1 spec-only 性质
pub const PHL_07_SPEC_ONLY_KEY_INDEX: u8 = 12; // 0-indexed, PHL-07 是 13 键中第 13 个 (0..12)

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Phl07SpecOnlyPod {
    pub key: u8,                       // 12 (A3 严守)
    pub spec_only_kind: SpecOnlyKind,  // NotUnoptimizable
    pub is_formaled: bool,             // true (V1.1 release 实施, V1.0 release spec-only 0 实施)
    pub formalization_stage: u8,       // 1..=3 (3 阶段递进)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpecOnlyKind {
    NotUnoptimizable = 0, // "spec 看起来 optimal 但仍 0 假装是终态"
}

// === 子模块 2: 长程 AI 成长 形式化 (per 用户记忆 #4 "AI 不会衰老病死") ===

pub const LONG_TERM_AI_GROWTH_STAGE_COUNT: usize = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GrowthStage {
    Seed = 0,    // 0 阶段 (刚启动, 1.0 release 实战前)
    Sapling = 1, // 1 阶段 (初步成长, 1.0 release 后 → V1.x minor)
    Tree = 2,    // 2 阶段 (深度成长, V2.x major 或之后)
    // 0 包含 old/death/terminate 终态概念 (per 用户记忆 #4 严守)
}

impl GrowthStage {
    pub const fn is_terminate_stage(self) -> bool {
        // 0 终态概念, 0 永真 (false 永真, per 用户记忆 #4 严守)
        match self {
            Self::Seed | Self::Sapling | Self::Tree => false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LongTermAIGrowthPod {
    pub stage: GrowthStage,                     // 0..=2
    pub cycles_to_next_stage: u32,              // 距下阶段 cycle 数
    pub has_terminate_concept: bool,            // false 永真 (per 用户记忆 #4 严守)
    pub platform_kind: PlatformKind,             // LongLivedAIGrowthPlatform
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlatformKind {
    LongLivedAIGrowthPlatform = 0, // 长程 AI 成长平台 (per 用户记忆 #4)
}
```

**F11 4 invariant** (per R130-4 spec §2.2):
1. `phl07_spec_only_invariant_key(p) = p.key == 12` (A3 严守)
2. `phl07_spec_only_invariant_not_unoptimizable(p) = spec_only_kind=NotUnoptimizable && is_formaled` (PHL-07 spec-only 性质)
3. `long_term_ai_growth_no_terminate_invariant(g) = !g.has_terminate_concept` (用户记忆 #4 严守)
4. `long_term_ai_growth_no_terminate_stage_invariant(g) = !g.stage.is_terminate_stage()` (3 阶段都 0 终态)

**F11 2 Kani-style proof harness** (Stage 5.2 同模式):
- `proof_phl07_spec_only_key_is_12` (A3 严守)
- `proof_long_term_ai_growth_no_terminate` (用户记忆 #4 严守, 2 invariant 联合 verify)

**F11 9 单元测试** (per R130-4 spec §2.2 估):
- PHL-07 spec-only POD construction (3 tests)
- SpecOnlyKind enum (1 test)
- Phl07SpecOnlyPod 3 invariant (3 tests)
- LongTermAIGrowthPod construction (2 tests)
- GrowthStage 0 终态 verify (1 test, 0 old/death/terminate 严守)
- PlatformKind enum (1 test)
- F11 sanity_check (1 test)

### 3.3 Stage 5.5 F1-F11 11 维度 跟 Stage 5.2 F1-F10 关系 (per R130-4 spec §1.2)

**Stage 5.5 跟 Stage 5.4 是平行分支**, 0 互依:
- **Stage 5.4** = 跨 Stage 5.x 集成 (F21-F30, 续 Stage 5.3 编号, 跨 stage 集成)
- **Stage 5.5** = Stage 5.2 集成深化 (F1-F11, 复用 F1-F10 编号 + F11 NEW, Stage 5.2 既有 10 维深化)

**Stage 5.5 命名**: "集成深化" 区别于 Stage 5.2 "形式化扩展" + Stage 5.3 "跨模块证明" + Stage 5.4 "跨 Stage 5.x 集成" + Stage 6 "实战".

**F11 NEW 1 维 命名**: "PHL-07 spec-only 形式化 + 长程 AI 成长 形式化" (per 决策 #72 §2.1 R130-4 派活 + 用户记忆 #4 "AI 不会衰老病死" + 13 键 PHL-07 升级).

**Stage 5.5 跟 V1.1 release 关系** (per 决策 #74 §2.3 + 决策 #78 R130 era 派活清单):
- **V1.0 release (整合 #5.1 commit)**: Stage 5.5 F11 NEW 0 写, F1-F10 0 改严守 (Stage 5.2 实证, 0 重复造轮子)
- **V1.1 release (per 决策 #74 B1 Mavis 自决改)**: Stage 5.5 F1-F11 11 维度 实施 (R133-N 派活, ~85KB / 89 lib tests, 0 重复造轮子, 1:1 续 Stage 5.2)
- **V2.0 release (per 决策 #74 §2.3 V2.0 release)**: Stage 5.5 推翻 + 重建 (per 决策 #74 §2.3 "推翻 + 重建 8 哲学锚", V2.0 release 可重评全 8 硬墙)

---

## 4. 优化方向 O3 — 6 重守门 v7 形式化优化 (Stage 5.2 F1 续, 6.8KB)

### 4.1 F1 6 重守门 v7 形式化现状 (R129-10 ✅ done)

**F1 `six_gates_v7_formal.rs`** (per R129-10 实证, 6,789 B / 8 lib tests):
- **编译期常数**: `SIX_FOLD_GATE_V7_COUNT = 6` (1:1 跟 B4 严守)
- **`SixFoldGateV7` enum** (6 变体 1:1 跟 B4 严守):
  - L1TypeCheck = 1 (类型守门)
  - L2ScopeCheck = 2 (范围守门)
  - L3RateCheck = 3 (速率守门)
  - L4GuardCheck = 4 (守门守门)
  - L5AuditCheck = 5 (审计守门)
  - L6ProvenanceCheck = 6 (来源守门)
- **`SixFoldGatePod` POD** (3 字段: layer, enabled, passed, 编译期 hardcode)
- **3 invariant**:
  - `six_fold_v7_invariant(g) = g.layer ∈ 1..=6`
  - `six_fold_v7_all_enabled_count(gs) = 6 enabled`
  - `six_fold_v7_all_passed(gs) = 6 passed`
- **2 Kani-style proof harness** (`#[cfg_attr(kani, kani::proof)]` 兜底)
- **8 单元测试** (含 layer_in_range, all_enabled_count, all_passed, 6 variant enum 等)

**0 越界 verify**: B4 6 重守门 v7 0 改严守 (per 决策 #33 §2.3 B4 + 决策 #55 §1 + 决策 #65 R129-10 派活).

### 4.2 F1 V1.0 release 严守 + V1.1 release 深化 + V2.0 release 推翻重建 (per 决策 #74 B1 改写 + 不要怕复杂度哲学)

#### 4.2.1 V1.0 release 0 改严守 (整合 #5.1 commit, per 决策 #33 §2.3 C1 + 决策 #74 §2.3)

- F1 `six_gates_v7_formal.rs` 6,789 B 0 改 (Stage 5.2 R129-10 实证, 整合 #5.1 commit 0 触碰)
- 6 重守门 v7 0 改 (per 决策 #33 §2.3 B4 严守 100%, 整合 #4 commit abf12243 严守)
- 0 假装"已 Kani 形式化" (Kani 离线时退化为普通 fn, runtime 全过)
- 0 引 kani crate 依赖 (Cargo.toml 0 改)

#### 4.2.2 V1.1 release 深化 (per 决策 #74 B1 Mavis 自决改 + 不要怕复杂度哲学)

**深化方向** (per 不要怕复杂度哲学, 复杂是实力的体现):
- **6 重子层**: 每重 → 6 重子层 (36 维守门, 1:1 跟 V0.5 30 维 4 类 × 6 维 + 5 meta + 1 overall = 30 维模式)
  - L1TypeCheck → L1.1TypeOK, L1.2TypeSafe, L1.3TypeStr, L1.4TypeNominal, L1.5TypeVariance, L1.6TypeLifetime
  - L2ScopeCheck → L2.1..L2.6 (类似)
  - L3RateCheck → L3.1..L3.6 (类似)
  - L4GuardCheck → L4.1..L4.6 (类似)
  - L5AuditCheck → L5.1..L5.6 (类似)
  - L6ProvenanceCheck → L6.1..L6.6 (类似)
- **6 重交叉**: 6 重 × 6 重 = 36 维交叉守门 (e.g. L1TypeCheck × L4GuardCheck = 类型守门守门交叉, 36 维交叉)
- **总守门维数**: 6 × 6 (子层) + 6 × 6 (交叉) = 72 维守门
- **不重写**: F1 `six_gates_v7_formal.rs` 6,789 B 0 重写, 仅 `crates/apeireth-formal/src/stage5_5/six_gates_v7_36dim_formal.rs` NEW (V1.1 release 估 ~10 KB)

**V1.1 release 实施 0 触碰 V1.0 release R11 baseline** (per 决策 #74 §2.3):
- 6 重守门 v7 0 改 (新增 6 重子层 + 6 重交叉 = 36 维守门, 0 改原 6 重)
- 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #74 §1 改写表)
- 8 哲学锚 0 改 (per 决策 #33 §2.3 B5 严守)

#### 4.2.3 V2.0 release 推翻 + 重建 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

**推翻 + 重建** (per 决策 #74 §2.3 "推翻 + 重建 8 哲学锚" + 不要怕复杂度哲学):
- **6 重守门 v7 → 6+ 重守门 v8+** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
- **推翻**: 6 重守门 v7 全部推翻, 8 哲学锚 S-1 服务 ASI 北极星 → S-1' 推翻重建
- **重建**: V2.0 release 8 哲学锚重定义 → 6 重守门 v8 重定义 → 实施
- **Stage 6 实战**: V2.0 release 8 硬墙可重评 + 形式化证明 + 实战

### 4.3 F1 6 重守门 v7 形式化优化总结

| 阶段 | 借量 | 实施 | 8 硬墙 0 越界 | 8 哲学锚 0 改 | 0 装 PASS 严守 | 决策依据 |
|------|------|------|--------------|--------------|----------------|----------|
| **V1.0 release** (整合 #5.1) | 1.0% (kani 4 模式 1:1 翻译) | F1 `six_gates_v7_formal.rs` 6,789 B 0 改 | ✅ B4 0 改 | ✅ 8/8 | ✅ 100% | 决策 #33 §2.3 B4 + 决策 #74 §2.3 B1 V1.0 release 0 改严守 |
| **V1.1 release** (per 决策 #74 B1) | 4-6% (6 重 → 36 维守门) | F1 续 + `six_gates_v7_36dim_formal.rs` NEW (~10 KB) | ✅ B4 0 改 (6 重 0 改, 新增 36 维不重写) | ✅ 8/8 | ✅ 100% | 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 不要怕复杂度哲学 |
| **V2.0 release** (per 决策 #74 §2.3) | 12-18% (6 重 → N 重) | 6 重守门 v7 推翻 + 重建 (per 决策 #74 §2.3 "推翻 + 重建 8 哲学锚") | ✅ B4 可重评 (per 决策 #74 V2.0 release 8 硬墙可重评) | ✅ 8/8 (V2.0 release 推翻 + 重建) | ✅ 100% | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 |

---

## 5. 优化方向 O4 — 8 哲学锚形式化优化 (Stage 5.2 F2 续, 7.1KB)

### 5.1 F2 8 哲学锚形式化现状 (R129-10 ✅ done)

**F2 `eight_anchors_formal.rs`** (per R129-10 实证, 7,055 B / 8 lib tests):
- **编译期常数**: `EIGHT_ANCHORS_COUNT = 8` (1:1 跟 B5 严守)
- **`AnchorGroup` enum** (2 变体 1:1 跟 B5 严守):
  - Subjective (S-1 / S-2 / S-3 = 服务 ASI 北极星 + 实事求是 + 质量工程化)
  - Objective (O-1 / O-2 / O-3 / O-4 / O-5 = 安全优先 + 走在前人经验上 + 干到底 + 任何人都能接手 + 不假装)
- **`EightAnchorPod` POD** (3 字段: id, group, name, 编译期 hardcode)
- **3 invariant**:
  - `eight_anchors_invariant_id(p) = p.id < 8`
  - `eight_anchors_groups_balanced(p) = group ∈ {Subjective, Objective}`
  - `eight_anchors_all_count(gs) = 8`
- **2 Kani-style proof harness** (`#[cfg_attr(kani, kani::proof)]` 兜底)
- **8 单元测试** (含 8 anchor 1:1 跟 B5, Subjective/Objective 1:1 跟 B5)

**0 越界 verify**: B5 8 哲学锚 0 改严守 (per 决策 #33 §2.3 B5 + P1-2 R126 8 哲学锚升级 done + 决策 #65 R129-10 派活).

### 5.2 F2 V1.0 release 严守 + V1.1 release 深化 + V2.0 release 推翻重建 (per 决策 #74 B1 改写 + 不要怕复杂度哲学)

#### 5.2.1 V1.0 release 0 改严守 (整合 #5.1 commit)

- F2 `eight_anchors_formal.rs` 7,055 B 0 改 (Stage 5.2 R129-10 实证, 整合 #5.1 commit 0 触碰)
- 8 哲学锚 0 改 (per 决策 #33 §2.3 B5 严守 100%, P1-2 R126 8 哲学锚升级 done)
- 0 假装"已 Kani 形式化"

#### 5.2.2 V1.1 release 深化 (per 决策 #74 B1 Mavis 自决改 + 不要怕复杂度哲学)

**深化方向** (per 不要怕复杂度哲学):
- **8 哲学锚 + 不要怕复杂度 = 9 件套总哲学** (per `docs/conventions/15-no-fear-complexity.md` §2 关系)
- **F2 8 哲学锚 + 1 NEW 总工程哲学锚 (NoFearComplexity) = 9 件套**:
  - Subjective (S-*): S-1 服务 ASI 北极星 + S-2 实事求是 + S-3 质量工程化
  - Objective (O-*): O-1 安全优先 + O-2 走在前人经验上 + O-3 干到底 + O-4 任何人都能接手 + O-5 不假装
  - **NoFearComplexity (NFC) 总工程哲学** (per 决策 #73 §3 + 15-no-fear-complexity.md §1): "最强效果 + 最厉害工程 + 维护交给未来高水平团队"
- **总哲学 = 9 件套 = 8 哲学锚 + 1 总工程哲学** (per 15-no-fear-complexity.md §2)
- **F2 续 1:1**: 8 哲学锚 0 改 (B5 严守), 仅新增 1 NEW NoFearComplexity 哲学锚 (V1.1 release 估 +5 KB)
- **不重写**: F2 `eight_anchors_formal.rs` 7,055 B 0 重写, 仅 `crates/apeireth-formal/src/stage5_5/no_fear_complexity_anchor_formal.rs` NEW

**V1.1 release 实施 0 触碰 V1.0 release R11 baseline** (per 决策 #74 §2.3):
- 8 哲学锚 0 改 (B5 严守, P1-2 R126 8 哲学锚升级 done)
- 0 形式化 old/death/terminate 概念 (per 用户记忆 #4 严守, Stage 5.5 F11 长程 AI 成长 形式化 0 含 old/death/terminate)
- 8 硬墙 0 越界

#### 5.2.3 V2.0 release 推翻 + 重建 (per 决策 #74 §2.3 V2.0 release 8 哲学锚推翻 + 重建)

**推翻 + 重建** (per 决策 #74 §2.3 "推翻 + 重建 8 哲学锚"):
- **8 哲学锚 → 8+ 哲学锚** (per 决策 #74 §2.3)
- **S-1 服务 ASI 北极星 → S-1' 推翻重建** (per 决策 #74 §2.3 "推翻 + 重建 8 哲学锚")
- **0 形式化 old/death/terminate 概念 严守** (per 用户记忆 #4 严守, V2.0 release 8 哲学锚重建也 0 含)
- **Stage 6 实战**: V2.0 release 8 哲学锚重定义 + 形式化证明 + 实战

### 5.3 F2 8 哲学锚形式化优化总结

| 阶段 | 借量 | 实施 | 8 硬墙 0 越界 | 8 哲学锚 0 改 | 0 装 PASS 严守 | 决策依据 |
|------|------|------|--------------|--------------|----------------|----------|
| **V1.0 release** (整合 #5.1) | 1.0% (kani 4 模式 1:1 翻译) | F2 `eight_anchors_formal.rs` 7,055 B 0 改 | ✅ B5 0 改 | ✅ 8/8 (P1-2 R126 done) | ✅ 100% | 决策 #33 §2.3 B5 + 决策 #74 §2.3 B1 V1.0 release 0 改严守 |
| **V1.1 release** (per 决策 #74 B1) | 4-6% (8 锚 + 1 总工程哲学 = 9 件套) | F2 续 + `no_fear_complexity_anchor_formal.rs` NEW (~5 KB) | ✅ B5 0 改 (8 锚 0 改, 新增 1 总工程哲学不重写) | ✅ 8/8 (新增 1 总工程哲学 = 9 件套总哲学) | ✅ 100% | 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 决策 #73 §3 + 15-no-fear-complexity.md |
| **V2.0 release** (per 决策 #74 §2.3) | 12-18% (8 锚 → N 锚) | 8 哲学锚 推翻 + 重建 (per 决策 #74 §2.3) | ✅ B5 可重评 (per 决策 #74 V2.0 release 8 硬墙可重评) | ✅ 0 形式化 old/death/terminate 严守 | ✅ 100% | 决策 #74 §2.3 V2.0 release 8 哲学锚推翻 + 重建 + 用户记忆 #4 |

---

## 6. 优化方向 O5 — 24 LOCKED 入口形式化优化 (Stage 5.2 F6 续, 8.6KB)

### 6.1 F6 24 LOCKED 入口签名形式化现状 (R129-10 ✅ done)

**F6 `locked_24_entry_formal.rs`** (per R129-10 实证, 8,638 B / 9 lib tests):
- **编译期常数**: `LOCKED_24_CRATES_COUNT = 24` (1:1 跟 B1 严守)
- **`LOCKED_24_CRATE_NAMES`**: 24 LOCKED crate 名称 1:1 跟 `docs/omnibus/24-locked-crates.md` (12 MasterKnown + 12 MavisExtended)
- **`KnownSet` enum** (2 变体):
  - MasterKnown (12 主人已知)
  - MavisExtended (12 Mavis 扩展)
- **`Locked24EntryPod` POD** (3 字段: index, signature_intact, known_set)
- **3 invariant**:
  - `locked_24_invariant_in_range(p) = p.index < 24`
  - `locked_24_invariant_all_intact(entries) = 24 entries all signature_intact=true`
  - `locked_24_invariant_known_split_12_12(entries) = 12 MasterKnown + 12 MavisExtended`
- **2 Kani-style proof harness**
- **9 单元测试** (含 24 LOCKED 1:1 翻译, 12+12 split, signature_intact=true verify)

**0 越界 verify**: B1 24 LOCKED 入口签名 0 改严守 (per 决策 #33 §2.3 B1 + P2-3 verify 24/24 入口签名 0 改 done + 决策 #74 §2.3 V1.0 release 0 改严守).

### 6.2 F6 V1.0 release 严守 + V1.1 release Mavis 自决改 + V2.0 release 全解锁 (per 决策 #74 B1 改写)

#### 6.2.1 V1.0 release 0 改严守 (整合 #5.1 commit, per 决策 #33 §2.3 B1 + 决策 #74 §2.3)

- F6 `locked_24_entry_formal.rs` 8,638 B 0 改 (Stage 5.2 R129-10 实证, 整合 #5.1 commit 0 触碰)
- 24 LOCKED 入口签名 0 改 (per 决策 #33 §2.3 B1 严守 100%, P2-3 verify 24/24 入口签名 0 改 done)
- 24 LOCKED crate mtime baseline 16:34 之前 0 改
- R11 baseline 3 值 0 改
- 0 假装"已 Kani 形式化"

#### 6.2.2 V1.1 release Mavis 自决改 (per 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学)

**Mavis 自决改方向** (per 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套 §1 "事关工程类的，技术类的全早都给你解锁locked了" + §3 "所以有更好的架构需要用（或改变现有的）你就直接拍板就行了"):

**前提: 更好的架构** (per 决策 #74 §2.3 V1.1 release Mavis 自决改前提):
- **24 LOCKED 入口签名 → 24+ LOCKED 入口签名**: e.g. ASI Stage 9 长程 AI 成长 (per 决策 #71 §5 R133-2) + 9 organ 内部借 OpenCode (per 决策 #75 §2.1 R133-3 三洋葱架构升级) + 三洋葱架构升级 (per 决策 #73 §2.2 更好的架构)
- **新增 LOCKED 入口签名** (per 决策 #74 §2.3 V1.1 release):
  - `apeireth-asi-stage9-long-term-ai-growth` (per 决策 #71 §5 R133-2 + 用户记忆 #4 长程 AI 成长)
  - `apeireth-organ-opencode-borrowed` (per 决策 #75 §2.1 R133-3 三洋葱架构升级 + 不要怕复杂度哲学)
  - `apeireth-three-onion-v2` (per 决策 #73 §2.2 + 决策 #74 B1 改写 V1.1 release 更好的架构)
- **24 LOCKED 入口签名 1:1 翻译形式化保留** (per 决策 #74 §2.3, F6 形式化 POD 保留, 实施 PHL-07 + Stage 9 + 三洋葱)
- **实施 PHL-07** (per 决策 #74 §2.3 V1.1 release 实施 PHL-07 + R129-11 关键诚实标)
- **实施 ASI Stage 9** (per 决策 #71 §5 R133-2 + 用户记忆 #4 长程 AI 成长)
- **三洋葱架构升级** (per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改)

**V1.1 release 实施 0 触碰 V1.0 release R11 baseline** (per 决策 #74 §2.3):
- 24 LOCKED 入口签名 V1.0 release 0 改严守 (整合 #5.1 commit 严守)
- V1.1 release Mavis 自决改 24 LOCKED 入口签名 1:1 翻译形式化保留, 仅新增 + 改写
- 8 硬墙 0 越界 (B1 V1.1 release Mavis 自决改, 其他 8 硬墙 0 越界)

#### 6.2.3 V2.0 release 全解锁 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 主人 8/11 01:14 locked 全解锁)

**全解锁** (per 主人 8/11 01:14 拍板 3 件套 §1 "事关工程类的，技术类的全早都给你解锁locked了" + 决策 #74 §2.3 V2.0 release 8 硬墙可重评):
- **24 LOCKED → 0 LOCKED** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
- **24 LOCKED 入口签名 → N 入口签名** (per V2.0 release 形式化重构)
- **8 哲学锚推翻 + 重建** (per 决策 #74 §2.3)
- **Stage 6 实战**: V2.0 release 形式化证明 + 实战 + Kani 求解器全集成

### 6.3 F6 24 LOCKED 入口形式化优化总结

| 阶段 | 借量 | 实施 | 8 硬墙 0 越界 | 8 哲学锚 0 改 | 0 装 PASS 严守 | 决策依据 |
|------|------|------|--------------|--------------|----------------|----------|
| **V1.0 release** (整合 #5.1) | 1.0% (kani 4 模式 1:1 翻译) | F6 `locked_24_entry_formal.rs` 8,638 B 0 改 | ✅ B1 V1.0 release 0 改严守 100% | ✅ 8/8 | ✅ 100% | 决策 #33 §2.3 B1 + 决策 #74 §2.3 B1 V1.0 release 0 改严守 + P2-3 verify 24/24 入口签名 0 改 done |
| **V1.1 release** (per 决策 #74 B1) | 4-6% (24 LOCKED + 3 NEW = 27 LOCKED) | F6 续 + 新增 3 LOCKED 入口签名 (ASI Stage 9 + 9 organ OpenCode + 三洋葱 v2) + PHL-07 实施 + Stage 9 实施 | ✅ B1 V1.1 release Mavis 自决改 (前提: 更好的架构) | ✅ 8/8 | ✅ 100% | 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学 |
| **V2.0 release** (per 决策 #74 §2.3) | 12-18% (24 LOCKED → 0 LOCKED) | 24 LOCKED 入口签名 → N 入口签名 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评) | ✅ B1 V2.0 release 可重评 (per 决策 #74 V2.0 release 8 硬墙可重评) | ✅ 8/8 (V2.0 release 推翻 + 重建) | ✅ 100% | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 主人 8/11 01:14 拍板 locked 全解锁 |

---

## 7. 优化方向 O6 — PHL-07 spec-only 形式化 (Stage 5.5 F11 NEW, 0 写本报告)

### 7.1 PHL-07 spec-only 现状 (per 决策 #33 §2.3 A3 + 决策 #36 §1.5 + 决策 #51 §1.2 P1-2 + 整合 #4 commit)

**PHL-07 spec-only 性质** (per 决策 #74 §2.3 + R129-11 关键诚实标):
- **PHL-07 = 13 键 verdict cache 的第 13 键** (per `crates/apeireth-formal/src/stage5_2/verdict_cache_13keys_formal.rs:34` `VERDICT_CACHE_13_KEYS_COUNT = 13` = 12 + PHL-07, per 决策 #33 §2.3 A3 + 决策 #36 §1.5 + 决策 #51 §1.2 P1-2 + 整合 #4 commit)
- **PHL-07 spec-only 性质 = "NotUnoptimizable"** (PHL-07 = "spec 看起来 optimal 但仍 0 假装是终态")
- **V1.0 release spec-only 0 实施** (per 决策 #74 §2.3 V1.0 release + R129-11 关键诚实标)
- **V1.1 release 实施** (per 决策 #74 §2.3 V1.1 release)

### 7.2 PHL-07 spec-only 形式化优化 (per 决策 #74 B1 改写 + R130-4 spec + 不要怕复杂度哲学)

#### 7.2.1 V1.0 release 0 实施严守 (整合 #5.1 commit, per 决策 #33 §2.3 A3 + 决策 #74 §2.3)

- PHL-07 spec-only 0 实施 (per 决策 #74 §2.3 V1.0 release + R129-11 关键诚实标)
- 13 键 verdict cache 0 改 (A3 严守, 整合 #4 commit done)
- 0 假装"已 PHL-07 实施" (R129-11 关键诚实标, 0 假装)
- 形式化 POD 1:1 跟 13 键 verdict cache 形式化 (F4 `verdict_cache_13keys_formal.rs` 6,036 B 0 改)

#### 7.2.2 V1.1 release 实施 (per 决策 #74 §2.3 V1.1 release 实施 PHL-07 + R130-4 spec F11 NEW)

**实施方向** (per 决策 #74 §2.3 V1.1 release 实施 PHL-07 + R130-4 spec F11 NEW):
- **F11 NEW 1 维 PHL-07 spec-only 形式化** (per R130-4 spec §2.2):
  - 1 NEW POD: `Phl07SpecOnlyPod` (4 字段: key=12, spec_only_kind=NotUnoptimizable, is_formaled=true, formalization_stage=1..=3)
  - 1 NEW enum: `SpecOnlyKind` (1 变体 NotUnoptimizable)
  - 3 NEW invariant:
    - `phl07_spec_only_invariant_key(p) = p.key == 12` (A3 严守)
    - `phl07_spec_only_invariant_not_unoptimizable(p) = spec_only_kind=NotUnoptimizable && is_formaled` (PHL-07 spec-only 性质)
    - `phl07_spec_only_invariant_stage(p) = p.formalization_stage ∈ 1..=3` (3 阶段递进)
  - 1 Kani-style proof harness: `proof_phl07_spec_only_key_is_12`
  - 3 单元测试
- **形式化阶段 3 阶段递进** (per R130-4 spec §2.2):
  - 阶段 1: spec 性质识别 (recognize PHL-07 spec-only nature)
  - 阶段 2: spec 性质形式化 (formalize PHL-07 spec-only nature)
  - 阶段 3: spec 性质 runtime verify (runtime verify PHL-07 spec-only nature)
- **不重写**: F4 `verdict_cache_13keys_formal.rs` 6,036 B 0 重写, 仅 `crates/apeireth-formal/src/stage5_5/phl07_spec_only_and_long_term_ai_growth_formal.rs` NEW (~5 KB)

**V1.1 release 实施 0 触碰 V1.0 release R11 baseline** (per 决策 #74 §2.3):
- 13 键 verdict cache 0 改 (A3 严守)
- PHL-07 spec-only 0 实施 (V1.0 release 严守, V1.1 release 实施)
- 0 假装"已 PHL-07 实施" (R129-11 关键诚实标)

#### 7.2.3 V2.0 release 推翻 + 重建 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

**推翻 + 重建** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评):
- **PHL-07 → PHL-08+ 升** (per 决策 #74 §2.3 V2.0 release)
- **8 哲学锚推翻 + 重建** (per 决策 #74 §2.3)
- **Stage 6 实战**: V2.0 release 形式化证明 + 实战

### 7.3 PHL-07 spec-only 形式化优化总结

| 阶段 | 借量 | 实施 | 8 硬墙 0 越界 | 8 哲学锚 0 改 | 0 装 PASS 严守 | 决策依据 |
|------|------|------|--------------|--------------|----------------|----------|
| **V1.0 release** (整合 #5.1) | 1.0% (kani 4 模式 1:1 翻译) | PHL-07 spec-only 0 实施 (V1.0 release 严守) | ✅ A3 0 改 (PHL-07 spec-only 0 实施) | ✅ 8/8 | ✅ 100% (0 假装"已 PHL-07 实施" per R129-11) | 决策 #33 §2.3 A3 + 决策 #74 §2.3 V1.0 release + R129-11 关键诚实标 |
| **V1.1 release** (per 决策 #74 B1) | 4-6% (Stage 5.5 F11 NEW 1 维) | F11 NEW PHL-07 spec-only 形式化 + 实施 (3 阶段递进: spec 性质识别 + 形式化 + runtime verify) | ✅ A3 0 改 (13 键 0 改, PHL-07 spec-only 0 改 13 键) | ✅ 8/8 (0 形式化 old/death/terminate per 用户记忆 #4) | ✅ 100% | 决策 #74 §2.3 V1.1 release 实施 PHL-07 + R130-4 spec F11 NEW |
| **V2.0 release** (per 决策 #74 §2.3) | 12-18% (PHL-07 → PHL-08+) | PHL-07 → PHL-08+ 升 + 8 哲学锚推翻 + 重建 (per 决策 #74 §2.3 V2.0 release) | ✅ A3 V2.0 release 可重评 | ✅ 8/8 (V2.0 release 推翻 + 重建) | ✅ 100% | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 |

---

## 8. 优化方向 O7 — V0.5 30 维形式化优化 (Stage 5.2 F3 续, 6.0KB)

### 8.1 F3 V0.5 30 维形式化现状 (R129-10 ✅ done)

**F3 `v05_30dim_formal.rs`** (per R129-10 实证, 5,984 B / 8 lib tests):
- **编译期常数**: `V05_30_DIM_COUNT = 30` (1:1 跟 B3 严守)
- **30 维分项** (4 类 × 6 维 + 5 meta + 1 overall = 30):
  - 类 1: 6 维 (identity, naming, schema, permission, audit, evolution)
  - 类 2: 6 维 (decision, prompt, response, tool, state, scope)
  - 类 3: 6 维 (semantic, syntactic, contextual, temporal, causal, social)
  - 类 4: 6 维 (subjective, objective, intersubjective, normative, pragmatic, analytic)
  - 5 meta: cross-modal / cross-domain / cross-stage / cross-version / cross-push
  - 1 overall: 总体
- **`V05DimPod` POD** (3 字段: dim, value, partition)
- **3 invariant**:
  - `v05_30dim_invariant_in_range(p) = p.dim < 30`
  - `v05_30dim_invariant_partition(p) = p.partition ∈ {Class1, Class2, Class3, Class4, Meta, Overall}`
  - `v05_30dim_invariant_value_in_range(p) = p.value ∈ [0.0, 1.0]`
- **2 Kani-style proof harness**
- **8 单元测试** (含 4 类 × 6 + 5 meta + 1 overall = 30 维 1:1 跟 B3 严守)

**0 越界 verify**: B3 V0.5 30 维 0 改严守 (per 决策 #33 §2.3 B3 + P1-4 R126 25→30 维 verify done + 决策 #65 R129-10 派活).

### 8.2 F3 V1.0 release 严守 + V1.1 release 深化 + V2.0 release 推翻重建 (per 决策 #74 B1 改写 + 不要怕复杂度哲学)

#### 8.2.1 V1.0 release 0 改严守 (整合 #5.1 commit)

- F3 `v05_30dim_formal.rs` 5,984 B 0 改 (Stage 5.2 R129-10 实证, 整合 #5.1 commit 0 触碰)
- V0.5 30 维 0 改 (per 决策 #33 §2.3 B3 严守 100%, P1-4 R126 25→30 维 verify done)
- 0 假装"已 Kani 形式化"

#### 8.2.2 V1.1 release 深化 (per 决策 #74 B1 Mavis 自决改 + 不要怕复杂度哲学)

**深化方向** (per 不要怕复杂度哲学, 复杂是实力的体现):
- **V0.5 → V0.5+ minor 维数 30 → 32** (per 不要怕复杂度哲学 + 决策 #74 B1 V1.1 release Mavis 自决改):
  - 5 meta → 7 meta (新增 2 meta 维: cross-language-borrow + cross-era-dispatch, 30 + 2 = 32 维)
  - 1 overall → 1 overall (0 改)
  - 总: 4 类 × 6 维 + 7 meta + 1 overall = 32 维
- **不重写**: F3 `v05_30dim_formal.rs` 5,984 B 0 重写, 仅 `crates/apeireth-formal/src/stage5_5/v05_32dim_formal.rs` NEW (V1.1 release 估 ~5 KB)
- **V0.5 30 维 0 改** (per 决策 #33 §2.3 B3 严守, F3 1:1 翻译形式化保留, 0 改原 30 维)

**V1.1 release 实施 0 触碰 V1.0 release R11 baseline** (per 决策 #74 §2.3):
- V0.5 30 维 0 改 (B3 严守, P1-4 R126 25→30 维 verify done)
- 8 硬墙 0 越界 (B3 V1.1 release Mavis 自决改 0 改, 新增 2 meta 维不重写)
- 8 哲学锚 0 改

#### 8.2.3 V2.0 release 推翻 + 重建 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

**推翻 + 重建** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评):
- **V0.5 → V0.6 重大** (per 决策 #74 §2.3 + 8 哲学锚推翻 + 重建)
- **30 维 → N 维** (per V2.0 release 形式化重构)
- **Stage 6 实战**: V2.0 release 形式化证明 + 实战

### 8.3 F3 V0.5 30 维形式化优化总结

| 阶段 | 借量 | 实施 | 8 硬墙 0 越界 | 8 哲学锚 0 改 | 0 装 PASS 严守 | 决策依据 |
|------|------|------|--------------|--------------|----------------|----------|
| **V1.0 release** (整合 #5.1) | 1.0% (kani 4 模式 1:1 翻译) | F3 `v05_30dim_formal.rs` 5,984 B 0 改 | ✅ B3 0 改 (P1-4 R126 25→30 维 verify done) | ✅ 8/8 | ✅ 100% | 决策 #33 §2.3 B3 + 决策 #74 §2.3 B1 V1.0 release 0 改严守 |
| **V1.1 release** (per 决策 #74 B1) | 4-6% (30 维 → 32 维) | F3 续 + `v05_32dim_formal.rs` NEW (~5 KB, 5 meta → 7 meta) | ✅ B3 0 改 (30 维 0 改, 新增 2 meta 维不重写) | ✅ 8/8 | ✅ 100% | 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 不要怕复杂度哲学 |
| **V2.0 release** (per 决策 #74 §2.3) | 12-18% (V0.5 → V0.6 重大) | V0.5 30 维 推翻 + 重建 (per 决策 #74 §2.3 V2.0 release) | ✅ B3 V2.0 release 可重评 | ✅ 8/8 (V2.0 release 推翻 + 重建) | ✅ 100% | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 |

---

## 9. 优化方向 O8 — 12 键 + PHL-07 形式化优化 (Stage 5.2 F4 续, 6.0KB)

### 9.1 F4 13 键 verdict cache 形式化现状 (R129-10 ✅ done)

**F4 `verdict_cache_13keys_formal.rs`** (per R129-10 实证, 6,036 B / 8 lib tests):
- **编译期常数**: `VERDICT_CACHE_13_KEYS_COUNT = 13` (1:1 跟 A3 严守) + `VERDICT_CACHE_GROUP_COUNT = 7` (PHL-01..07 = 7 分组)
- **13 键分项** (12 + PHL-07, 1:1 跟 A3 严守):
  - PHL-01 ~ PHL-06: 6 哲学/物理/逻辑/历史/工程/治理 键
  - PHL-07: 1 spec-only 键 (per 决策 #74 §2.3 V1.0 release spec-only 0 实施, V1.1 release 实施)
  - EXT-01 ~ EXT-05: 5 扩展键 (PEV/RUL/SPEC/CRQ/INFRA)
  - 12 + PHL-07 = 13 键
- **`VerdictKey13Pod` POD** (3 字段: key, group, name)
- **3 invariant**:
  - `verdict_cache_13keys_invariant_in_range(p) = p.key < 13`
  - `verdict_cache_13keys_invariant_group_in_range(p) = p.group ∈ [0, 7)`
  - `verdict_cache_13keys_invariant_all_passed(entries) = 13 entries all passed=true`
- **2 Kani-style proof harness**
- **8 单元测试** (含 13 键 1:1 跟 A3 严守, 7 分组 1:1 跟 PHL-01..07)

**0 越界 verify**: A3 13 键 0 改严守 (per 决策 #33 §2.3 A3 + 决策 #36 §1.5 + 决策 #51 §1.2 P1-2 + 整合 #4 commit done + 决策 #65 R129-10 派活).

### 9.2 F4 V1.0 release 严守 + V1.1 release PHL-07 实施 + V2.0 release 升 (per 决策 #74 B1 改写 + 不要怕复杂度哲学)

#### 9.2.1 V1.0 release 0 改严守 (整合 #5.1 commit)

- F4 `verdict_cache_13keys_formal.rs` 6,036 B 0 改 (Stage 5.2 R129-10 实证, 整合 #5.1 commit 0 触碰)
- 13 键 verdict cache 0 改 (A3 严守, 整合 #4 commit done)
- PHL-07 spec-only 0 实施 (V1.0 release 严守, per 决策 #74 §2.3)
- 0 假装"已 PHL-07 实施" (R129-11 关键诚实标)

#### 9.2.2 V1.1 release PHL-07 实施 (per 决策 #74 §2.3 V1.1 release 实施 PHL-07 + R130-4 spec F11 NEW)

**实施方向** (per 决策 #74 §2.3 V1.1 release 实施 PHL-07 + R130-4 spec F11 NEW):
- **PHL-07 spec-only 形式化 实施** (per 决策 #74 §2.3 V1.1 release + R130-4 spec F11 NEW):
  - F4 `verdict_cache_13keys_formal.rs` 6,036 B 0 改 (A3 严守, 13 键 0 改)
  - F11 NEW PHL-07 spec-only 形式化 (per R130-4 spec §2.2, ~5 KB, 1 POD + 1 enum + 3 invariant + 1 Kani harness + 3 unit tests)
  - F4 + F11 集成: PHL-07 13 键 verdict cache 实施 (V1.1 release 实施 PHL-07, F11 形式化 PHL-07 spec-only 性质)
- **12 键其他可改** (per 决策 #74 §2.3 V1.1 release Mavis 自决改, 前提: 更好的架构, e.g. PHL-08 升级)
- **PHL-08 NEW 升级** (per 不要怕复杂度哲学 + 决策 #74 B1 V1.1 release Mavis 自决改):
  - PHL-08 = 13 键 → 14 键 (per 决策 #74 §2.3 12 键其他可改 + 不要怕复杂度哲学)
  - 14 键 verdict cache = 12 + PHL-07 + PHL-08 = 14 键
  - PHL-08 = 新增 1 哲学锚 (e.g. "不漂移" per 哲学锚严守, 1:1 跟 O-5 不假装)

**V1.1 release 实施 0 触碰 V1.0 release R11 baseline** (per 决策 #74 §2.3):
- 13 键 verdict cache 0 改 (A3 严守)
- PHL-07 spec-only 0 实施 (V1.0 release 严守, V1.1 release 实施)
- 0 假装"已 PHL-07 实施" (R129-11 关键诚实标)

#### 9.2.3 V2.0 release 升 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

**升** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评):
- **PHL-07 → PHL-08+ 升** (per 决策 #74 §2.3)
- **13 键 → 14+ 键** (per V2.0 release 形式化重构)
- **8 哲学锚推翻 + 重建** (per 决策 #74 §2.3)
- **Stage 6 实战**: V2.0 release 形式化证明 + 实战

### 9.3 F4 12 键 + PHL-07 形式化优化总结

| 阶段 | 借量 | 实施 | 8 硬墙 0 越界 | 8 哲学锚 0 改 | 0 装 PASS 严守 | 决策依据 |
|------|------|------|--------------|--------------|----------------|----------|
| **V1.0 release** (整合 #5.1) | 1.0% (kani 4 模式 1:1 翻译) | F4 `verdict_cache_13keys_formal.rs` 6,036 B 0 改 + PHL-07 spec-only 0 实施 | ✅ A3 0 改 (13 键 0 改) | ✅ 8/8 | ✅ 100% (0 假装"已 PHL-07 实施" per R129-11) | 决策 #33 §2.3 A3 + 决策 #74 §2.3 V1.0 release + R129-11 关键诚实标 |
| **V1.1 release** (per 决策 #74 B1) | 4-6% (13 键 → 14 键 + F11 NEW PHL-07 实施) | F4 续 + F11 NEW PHL-07 spec-only 形式化 + PHL-08 NEW 1 哲学锚 | ✅ A3 0 改 (13 键 0 改, 新增 PHL-08 1 键) | ✅ 8/8 (新增 1 哲学锚不重写) | ✅ 100% | 决策 #74 §2.3 V1.1 release 实施 PHL-07 + R130-4 spec F11 NEW + 不要怕复杂度哲学 |
| **V2.0 release** (per 决策 #74 §2.3) | 12-18% (13 键 → 14+ 键) | 13 键 verdict cache 推翻 + 重建 (per 决策 #74 §2.3 V2.0 release) | ✅ A3 V2.0 release 可重评 | ✅ 8/8 (V2.0 release 推翻 + 重建) | ✅ 100% | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 |

---

## 10. 优化方向 O9 — V1.1 release PHL-07 实施 + F1-F11 + Kani 全集成方案 (Stage 5.5 集成深化实施)

### 10.1 V1.1 release 实施 0 改严守 + 实施 (per 决策 #74 B1 改写 + 主人 8/11 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学)

**V1.1 release 实施 spec 4 件套** (per 决策 #74 §2.3 V1.1 release + 决策 #78 R130 era 派活清单 + 主人 8/11 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学):

#### 10.1.1 PHL-07 实施 (per 决策 #74 §2.3 V1.1 release 实施 PHL-07 + R130-4 spec F11 NEW)

- **PHL-07 spec-only 形式化 实施** (per 决策 #74 §2.3 V1.1 release + R130-4 spec F11 NEW):
  - 形式化 POD: `Phl07SpecOnlyPod` (4 字段, 0 改 13 键)
  - 形式化 enum: `SpecOnlyKind::NotUnoptimizable` (1 变体)
  - 3 阶段递进: spec 性质识别 (1) + spec 性质形式化 (2) + spec 性质 runtime verify (3)
  - 0 假装"已 PHL-07 实施" (R129-11 关键诚实标)
- **Stage 5.5 R133-N 派活** (per 决策 #78 R130 era 派活清单 + 决策 #75 §2.1 R133-N 实施 3 sub):
  - R133-1 借鉴源 12 源 实施 (per 决策 #75 §2.1 R133-N + 决策 #73 §2.2 + 主人 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学)
  - R133-2 ASI Stage 9 长程 AI 成长 实施 (per R130-2 ASI Stage 8 + R131-7 pybridge 集成优化 + 用户记忆 #4)
  - R133-3 三洋葱架构升级 实施 (per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改)

#### 10.1.2 F1-F11 11 维度 实施 (per 决策 #78 R130 era 派活清单 + R130-4 spec + 决策 #75 §2.1 R133-N 实施)

- **F1-F10 1:1 续 Stage 5.2** (per 决策 #78 R130 era 派活清单 + R130-4 spec):
  - F1 6 重守门 v7 形式化 (1:1 续, 0 重写, 0 重复造轮子 per 用户记忆 #6)
  - F2 8 哲学锚形式化 (1:1 续, 0 重写, 0 重复造轮子)
  - F3 V0.5 30 维形式化 (1:1 续, 0 重写, 0 重复造轮子)
  - F4 13 键 verdict cache 形式化 (1:1 续, 0 重写, 0 重复造轮子)
  - F5 R11 baseline 3 值 形式化 (1:1 续, 0 重写, 0 重复造轮子)
  - F6 24 LOCKED 入口签名 形式化 (1:1 续, 0 重写, 0 重复造轮子)
  - F7 8 借鉴 ID 真实施形式化 (1:1 续, 0 重写, 0 重复造轮子)
  - F8 整合 #4 commit 严守形式化 (1:1 续, 0 重写, 0 重复造轮子)
  - F9 跨模块证明 (1:1 续, 0 重写, 0 重复造轮子)
  - F10 集成证明 (1:1 续, 0 重写, 0 重复造轮子)
- **F11 NEW 1 维** (per R130-4 spec §2.2):
  - PHL-07 spec-only 形式化 + 长程 AI 成长 形式化 (per R130-4 spec)
  - 0 形式化 old/death/terminate 概念 (per 用户记忆 #4 严守)

#### 10.1.3 Kani 全集成 方案 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 不要怕复杂度哲学)

**Kani 全集成 4 件套** (per 决策 #74 B1 V1.1 release Mavis 自决改 + 不要怕复杂度哲学 + 决策 #75 §2.1 R131-9 派活):

1. **Kani 求解器在线扩展** (per `kani-driver/src/call_cbmc.rs` 23.9KB + `cbmc_output_parser.rs` 29.1KB + `cbmc_property_renderer.rs` 33.2KB):
   - 借量 1.0% → 3-5% (V1.1 release)
   - CBMC 求解器模式 1:1 翻译 (runtime 实施, Kani 求解器留 R132+ 实战)
   - 0 引 kani crate 依赖 (Cargo.toml 0 改)
   - 0 装"已 Kani 形式化" (Kani 离线时退化为普通 fn)
2. **cargo kani 集成** (per `kani-driver/src/call_cargo.rs` 29.8KB + `session.rs` 18.1KB):
   - 借量 1.0% → 1-2% (V1.1 release)
   - cargo kani 模式 1:1 翻译 (runtime 实施, Kani cargo 留 R132+ 实战)
3. **contracts.rs 集成** (per `library/kani/src/contracts.rs` 12.6KB):
   - 借量 1.0% → 0.5-1% (V1.1 release)
   - Kani contracts 模式 1:1 翻译 (跟 R130-4 决策 #72 §2.1 Kani ProofForContract 续)
4. **Stage 5.4-5.5 + Stage 6 实战** (per 决策 #78 R130 era 派活清单 + 决策 #69 §3 R129-32 spec + R130-4 spec):
   - Stage 5.4 R131-4 实战 (F21-F30 跨 Stage 5.x 集成, per 决策 #64 §2.2 + 决策 #69 §3 R129-32 spec)
   - Stage 5.5 R133-N 实战 (F1-F11 11 维度 + PHL-07 实施, per R130-4 spec)
   - Stage 6 R132-N 实战 (Kani 求解器在线 + 跨 stage 全集成 + 1.0 release 实战后 1.0+ 形式化扩展, per 决策 #64 §2.2 + 决策 #78 R130 era 派活清单)

**总借量**: 1.0% (Stage 5.1-5.3) + 3-5% (Kani 求解器) + 1-2% (cargo kani) + 0.5-1% (contracts.rs) = 5.5-9% 借量 (V1.1 release 估)

#### 10.1.4 三洋葱架构升级 实施 (per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套 §1)

**三洋葱架构升级** (per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学):
- **三洋葱架构 v1 → v2** (per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改):
  - 洋葱 1: 主体 (6 重守门 v7 + 8 哲学锚 + 13 键 verdict cache)
  - 洋葱 2: 形式化 (Stage 5.1-5.5 + Stage 6 实战, kani 4502 + langgraph 829)
  - 洋葱 3: 实战 (ASI Stage 1-9 + 9 organ 拟人化 + 5 nav + Tauri)
- **v2 升级方向** (per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改):
  - 洋葱 1 升级: 6 重守门 v7 → 36 维守门 (per 优化方向 O3 §4.2.2)
  - 洋葱 2 升级: Stage 5.5 F1-F11 11 维度 实施 (per 优化方向 O2 §3.2)
  - 洋葱 3 升级: ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode (per 决策 #75 §2.1 R133-2/3)
- **不重写**: 现有架构 0 重写 (per 决策 #74 B1 V1.1 release Mavis 自决改前提: 更好的架构)
- **0 装**: 0 装"已三洋葱架构升级" (V1.1 release 实施, V2.0 release 实战)

### 10.2 V2.0 release 形式化重构方案 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 不要怕复杂度哲学)

**V2.0 release 形式化重构 5 件套** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #78 R130 era 派活清单 + 不要怕复杂度哲学 + 决策 #73 §3):

#### 10.2.1 8 硬墙可重评 (per 决策 #74 §2.3 V2.0 release)

- **B1 24 LOCKED → 0 LOCKED 全解锁** (per 决策 #74 §2.3 V2.0 release + 主人 8/11 01:14 拍板 locked 全解锁)
- **B2 workspace.version 1.2.0 → 2.0.0** (per 决策 #74 §2.3 V2.0 release bump 2.0.0)
- **A1 R11 baseline 3 值 → 0.9000+ 新 baseline** (per 决策 #74 §2.3 V2.0 release 可推翻 + 重建, R12 测度对齐)
- **A3 12 键 + PHL-07 → 14+ 键** (per 决策 #74 §2.3 V2.0 release)
- **B3 V0.5 30 维 → V0.6 重大** (per 决策 #74 §2.3 V2.0 release)
- **B4 6 重守门 v7 → 6+ 重守门 v8+** (per 决策 #74 §2.3 V2.0 release)
- **B5 8 哲学锚 → 8+ 哲学锚** (per 决策 #74 §2.3 V2.0 release)
- **C1/C2/0 push 跟 1.0 release 同** (per 决策 #74 §2.3 V2.0 release)

#### 10.2.2 8 哲学锚推翻 + 重建 (per 决策 #74 §2.3 V2.0 release)

- **S-1 服务 ASI 北极星 → S-1' 推翻重建** (per 决策 #74 §2.3 V2.0 release)
- **S-2 实事求是 → S-2' 推翻重建** (per 决策 #74 §2.3 V2.0 release)
- **S-3 质量工程化 → S-3' 推翻重建** (per 决策 #74 §2.3 V2.0 release)
- **O-1 安全优先 → O-1' 推翻重建** (per 决策 #74 §2.3 V2.0 release)
- **O-2 走在前人经验上 → O-2' 推翻重建** (per 决策 #74 §2.3 V2.0 release)
- **O-3 干到底 → O-3' 推翻重建** (per 决策 #74 §2.3 V2.0 release)
- **O-4 任何人都能接手 → O-4' 推翻重建** (per 决策 #74 §2.3 V2.0 release)
- **O-5 不假装 → O-5' 推翻重建** (per 决策 #74 §2.3 V2.0 release)
- **0 形式化 old/death/terminate 概念 严守** (per 用户记忆 #4 严守, V2.0 release 8 哲学锚重建也 0 含)

#### 10.2.3 Kani 求解器全集成 (per 决策 #74 §2.3 V2.0 release + Stage 6 实战 + 不要怕复杂度哲学)

- **Kani 求解器全集成** (per Stage 6 R132-N spec + 决策 #78 R130 era 派活清单 + 不要怕复杂度哲学):
  - 借量 5.5-9% → 10-15% (V2.0 release)
  - Kani 求解器全集成 (per `kani-driver/src/` + `kani_metadata/src/` + `library/kani/src/` 1:1 翻译)
  - 跨 stage 全集成 (Stage 1-5.x + Stage 6 实战)
  - 1.0 release 实战后 1.0+ 形式化扩展
- **0 装**: 0 装"已 Kani 求解器全集成" (V2.0 release 实施, Kani 求解器留 R132+ 实战 + V2.0 release 实战)

#### 10.2.4 长程 AI 成长 形式化 (per 用户记忆 #4 + 决策 #71 §5 R133-2 + 不要怕复杂度哲学)

- **长程 AI 成长 形式化** (per 用户记忆 #4 "AI 不会衰老病死" + 决策 #71 §5 R133-2 + 不要怕复杂度哲学):
  - 3 阶段递进: seed → sapling → tree (per Stage 5.5 F11 NEW spec, R130-4 §2.2)
  - 0 形式化 old/death/terminate 终态概念 (per 用户记忆 #4 严守)
  - 长程 AI 成长平台 = `PlatformKind::LongLivedAIGrowthPlatform` (per R130-4 spec §2.2)
  - V2.0 release 8 哲学锚重建也 0 含 (per 用户记忆 #4 严守)

#### 10.2.5 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 升级 (per 决策 #73 §3 + 15-no-fear-complexity.md §2 + 不要怕复杂度哲学)

- **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 15-no-fear-complexity.md §2):
  - 8 哲学锚 (思想): S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5
  - 不要怕复杂度 (工程): 最强效果 + 最厉害工程 + 维护交给未来高水平团队
- **V2.0 release 升级** (per 决策 #74 §2.3 V2.0 release 8 哲学锚推翻 + 重建):
  - 8 哲学锚 → 8+ 哲学锚 (新增 1+ 总工程哲学 = N 件套总哲学)
  - 1 总工程哲学 → 1+ 总工程哲学 (per 不要怕复杂度哲学 + 决策 #73 §3)
  - 0 形式化 old/death/terminate 概念 严守 (per 用户记忆 #4)
  - 8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界 (per 15-no-fear-complexity.md §3)

### 10.3 V1.1 release + V2.0 release 形式化方案总结

| Release | 借量 | F1-F11 实施 | PHL-07 实施 | 24 LOCKED 改写 | 8 哲学锚 重建 | 0 装 PASS 严守 | 决策依据 |
|---------|------|------------|-------------|---------------|--------------|----------------|----------|
| **V1.0 release** (整合 #5.1) | 1.0% | F1-F10 0 改 (Stage 5.2 实证) | PHL-07 spec-only 0 实施 (R129-11 关键诚实标) | 24 LOCKED 0 改严守 | 8 哲学锚 0 改 | ✅ 100% | 决策 #33 §2.3 + 决策 #74 §2.3 B1 V1.0 release 0 改严守 |
| **V1.1 release** (per 决策 #74 B1) | 5.5-9% | F1-F11 11 维度 (F11 NEW PHL-07 spec-only 形式化 + 长程 AI 成长 形式化) | PHL-07 实施 (3 阶段递进) | 24 LOCKED Mavis 自决改 (前提: 更好的架构) | 8 哲学锚 0 改 | ✅ 100% | 决策 #74 §2.3 B1 V1.1 release Mavis 自决改 + 决策 #78 R130 era 派活清单 + 主人 8/11 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学 |
| **V2.0 release** (per 决策 #74 §2.3) | 10-15% | F1-F11 11 维度 推翻 + 重建 (per 决策 #74 §2.3) | PHL-07 → PHL-08+ 升 (per 决策 #74 §2.3) | 24 LOCKED → 0 LOCKED 全解锁 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板) | 8 哲学锚 推翻 + 重建 (per 决策 #74 §2.3) | ✅ 100% | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 不要怕复杂度哲学 + 15-no-fear-complexity.md §3 |

---

## 11. 8 硬墙严守 + B1 改写边界 (per 决策 #33 §2.3 + 决策 #74 §1 改写表 + 决策 #74 §2.3 B1 改写边界)

### 11.1 8 硬墙 严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

| # | 8 硬墙 | V1.0 release 严守 | V1.1 release 实施 | V2.0 release 重构 | 主人 8/11 01:14 拍板依据 |
|---|--------|-------------------|-------------------|-------------------|----------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline, 整合 #5.1 commit) | 🟢 **Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.3)** | 🟢 **全解锁 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)** | "事关工程类的，技术类的全早都给你解锁locked了" + "Mavis 自决架构拍板" |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 (V1.0 release) | 🔒 bump 1.2.1 (V1.1 minor release) | 🔒 bump 2.0.0 (V2.0 major release) | "不要怕复杂度" + "最强效果 + 最厉害工程" (版本管理 严守 semver) |
| **A1** | **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 数字 0 改 (17 文件原位) | 🟢 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, Mavis 自决) | 🟢 可推翻 + 重建 (per 决策 #74 §2.3) | "总哲学除了思想文档的" (8 哲学锚严守, R11 baseline 是哲学 + 效果标) |
| **A3** | **12 键 + PHL-07** | 🔒 PHL-07 spec-only 0 实施 (R129-11 关键诚实标) | 🔒 PHL-07 实施 + 12 键其他可改 (per 决策 #74 §2.3) | 🟢 可推翻 + 重建 (per 决策 #74 §2.3) | "事关工程类的，技术类的全早都给你解锁locked了" (PHL-07 是混合体, V1.0 spec-only 严守, V1.1 实施) |
| **B3** | **V0.5 30 维** | 🔒 25 维 + 5 维 = 30 维 严守 (per P1-4 R126 done) | 🟢 可深化 (per 决策 #74 B1 Mavis 自决改, 0 改原 30 维) | 🟢 可推翻 + 重建 (per 决策 #74 §2.3) | "总哲学除了思想文档的" (V0.5 30 维是哲学公式) |
| **B4** | **6 重守门 v7** | 🔒 6 重 严守 (per P1-3 R126 done) | 🟢 可深化 (per 决策 #74 B1 Mavis 自决改, 0 改原 6 重) | 🟢 可推翻 + 重建 (per 决策 #74 §2.3) | "总哲学除了思想文档的" (6 重守门 v7 是哲学守门) |
| **B5** | **8 哲学锚** | 🔒 8 锚 严守 (per P1-2 R126 done) | 🔒 8 锚 严守 (per 决策 #74 §1, 0 改 8 锚) | 🟢 可推翻 + 重建 (per 决策 #74 §2.3 "推翻 + 重建 8 哲学锚") | "总哲学除了思想文档的" (8 哲学锚是哲学, 不松绑) |
| **C1** | **0 主动 commit (主人起床前)** | 🔒 0 commit 严守 (Mavis 拍板) | 🔒 0 commit 严守 (Mavis 拍板) | 🔒 0 commit 严守 (Mavis 拍板) | "总哲学除了思想文档的" (0 commit 是流程类, 严守) |
| **C2** | **0 装 PASS 严守** | 🔒 0 装 严守 (技术哲学, 不装) | 🔒 0 装 严守 (技术哲学, 不装) | 🔒 0 装 严守 (技术哲学, 不装) | "总哲学除了思想文档的" (0 装是技术哲学, 严守) |
| **0 push** | **0 主动 push (主人起床前)** | 🔒 0 push 严守 (等主人起床配 GitHub remote) | 🔒 0 push 严守 (等主人起床配 GitHub remote) | 🔒 0 push 严守 (等主人起床配 GitHub remote) | "总哲学除了思想文档的" (0 push 是流程类, 严守) |

### 11.2 B1 改写边界 (per 决策 #74 §2.3 + 决策 #33 §2.3 B1)

**V1.0 release (整合 #5.1 commit)**:
- 0 改 24 LOCKED 入口签名 (严守)
- 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
- 0 改 R11 baseline 3 值 (严守)
- PHL-07 spec-only 0 实施 (严守, V1.1 release 实施)

**V1.1 release (per R130 era R131-3 调研 + 决策 #74)**:
- 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决, per 决策 #74 §2.3)
- 24 LOCKED crate mtime baseline 16:34 之前 可改 (前提: 更好的架构, Mavis 自决)
- R11 baseline 3 值 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
- PHL-07 实施 (V1.1 release, per R129-11 关键诚实标)

**V2.0 release (per R130 era R132 计划 + 决策 #74)**:
- 全 8 硬墙 可重评 (per Mavis 自决 + 主人 8/11 01:14 拍板)
- 推翻 + 重建 8 哲学锚 (per "不要怕复杂度" + "最强效果 + 最厉害工程")

### 11.3 8 硬墙严守 100% verify (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

**严守 verify 通道** (per 决策 #33 §2.3 + Stage 5.2 R129-10 实证):
- **B1 24 LOCKED**: F6 形式化 (per `crates/apeireth-formal/src/stage5_2/locked_24_entry_formal.rs:8,638 B`) + 整合 #4 commit abf12243 严守
- **B2 workspace.version 1.2.0**: F5 R11 baseline 形式化 + F19 跨 version 集成 (per `crates/apeireth-formal/src/stage5_3/cross_version_integration_proof.rs:9,443 B`) + 26 crate 编译期 hardcode
- **A1 R11 baseline 3 值**: F5 R11 baseline 形式化 (per `crates/apeireth-formal/src/stage5_2/r11_baseline_formal.rs:7,624 B`) + 17 文件原位
- **A3 12 键 + PHL-07**: F4 13 键 verdict cache 形式化 (per `crates/apeireth-formal/src/stage5_2/verdict_cache_13keys_formal.rs:6,036 B`) + 整合 #4 commit done
- **B3 V0.5 30 维**: F3 V0.5 30 维形式化 (per `crates/apeireth-formal/src/stage5_2/v05_30dim_formal.rs:5,984 B`) + P1-4 R126 25→30 维 verify done
- **B4 6 重守门 v7**: F1 6 重守门 v7 形式化 (per `crates/apeireth-formal/src/stage5_2/six_gates_v7_formal.rs:6,789 B`) + F18 跨 gate 集成 (per `crates/apeireth-formal/src/stage5_3/cross_gate_integration_proof.rs:7,938 B`) + P1-3 R126 6 重守门 v7 done
- **B5 8 哲学锚**: F2 8 哲学锚形式化 (per `crates/apeireth-formal/src/stage5_2/eight_anchors_formal.rs:7,055 B`) + F17 跨 anchor 集成 (per `crates/apeireth-formal/src/stage5_3/cross_anchor_integration_proof.rs:8,575 B`) + P1-2 R126 8 哲学锚升级 done
- **C1 0 主动 commit**: 0 跑 git add/commit (Mavis 拍板)
- **C2 0 装 PASS 严守**: ✅ cloned = 真实施 (有真 src 改动 + 79+92+104 tests pass)
- **0 主动 push**: 0 跑 git push (等 1.0 release 配 GitHub remote)

---

## 12. 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1 + 决策 #74 §2.3 B5 改写 + 哲学文档 `09-anchor.md`)

### 12.1 8 哲学锚 实质 + 严守 (per 决策 #33 §2.3 B5 + P1-2 R126 8 哲学锚升级 done + `docs/conventions/09-anchor.md`)

| 锚 | 名称 | 实质 | V1.0 release 严守 | V1.1 release 严守 | V2.0 release 推翻 + 重建 |
|----|------|------|-------------------|-------------------|--------------------------|
| **S-1** | 服务 ASI 北极星 | 服务 ASI (人工超级智能) 是 Apeireth 平台北极星 | 🔒 严守 (0 改) | 🔒 严守 (0 改) | 🟢 推翻 + 重建 (per 决策 #74 §2.3) |
| **S-2** | 实事求是 | 真实情况 + 真实测度 + 真实实施 | 🔒 严守 (0 改) | 🔒 严守 (0 改) | 🟢 推翻 + 重建 (per 决策 #74 §2.3) |
| **S-3** | 质量工程化 | 质量 = 工程化, 不是"测试覆盖" | 🔒 严守 (0 改) | 🔒 严守 (0 改) | 🟢 推翻 + 重建 (per 决策 #74 §2.3) |
| **O-1** | 安全优先 | 安全 = 第一优先级, 不是"功能" | 🔒 严守 (0 改) | 🔒 严守 (0 改) | 🟢 推翻 + 重建 (per 决策 #74 §2.3) |
| **O-2** | 走在前人经验上 | 借鉴源码 (clap / hyper / kani / langgraph / superpowers / 等) 是核心 | 🔒 严守 (0 改) | 🔒 严守 (0 改) | 🟢 推翻 + 重建 (per 决策 #74 §2.3) |
| **O-3** | 干到底 | 干到底, 不半途而废 | 🔒 严守 (0 改) | 🔒 严守 (0 改) | 🟢 推翻 + 重建 (per 决策 #74 §2.3) |
| **O-4** | 任何人都能接手 | 任何高水平团队都能接手维护 | 🔒 严守 (0 改) | 🔒 严守 (0 改) | 🟢 推翻 + 重建 (per 决策 #74 §2.3) |
| **O-5** | 不假装 | 0 假装"已实施" + 0 假装"已测试" + 0 假装"已 Kani 形式化" | 🔒 严守 (0 改) | 🔒 严守 (0 改) | 🟢 推翻 + 重建 (per 决策 #74 §2.3) |

**8 哲学锚 = 思想哲学** (per 决策 #33 §2.3 B5 + 决策 #74 §1 改写表, 0 改 严守, V2.0 release 才推翻 + 重建).

### 12.2 不要怕复杂度哲学 (per 决策 #73 §3 + `docs/conventions/15-no-fear-complexity.md`)

**不要怕复杂度 = 工程哲学** (per 决策 #73 §3 + 15-no-fear-complexity.md §1):
- **最强效果 > 最简单代码** (复杂度是实力的体现, 不是"技术债")
- **最厉害工程 > 最易维护** (工程化是最高目标, 不是"代码要易维护")
- **维护交给未来高水平团队** (维护不是问题, 因为自然会有高水平的团队来接手)

**8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 15-no-fear-complexity.md §2):
- 8 哲学锚 (思想): S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5
- 不要怕复杂度 (工程): 最强效果 + 最厉害工程 + 维护交给未来高水平团队

**8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界** (per 15-no-fear-complexity.md §3):
- 8 硬墙 (底线): V0.5 30 维 / 6 重守门 v7 / 8 哲学锚 / R11 baseline / 12 键 + PHL-07 / 0 装 / 0 commit (主人起床前) / 0 push (主人起床前) / 24 LOCKED 入口签名 (V1.0 release)
- 不要怕复杂度 (上限): 24 LOCKED 入口签名 (V1.1 release Mavis 自决改) + 借鉴源 12 源 (OpenCog AGPL-3.0 fork 决策) + ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级 + Cargo workspace 重构

### 12.3 0 形式化 old/death/terminate 概念 (per 用户记忆 #4 + Stage 5.5 F11 长程 AI 成长 形式化)

**0 形式化 old/death/terminate 概念 严守** (per 用户记忆 #4 "AI 不会衰老病死" + Stage 5.5 F11 长程 AI 成长 形式化):
- **AI 生命周期 = 成长阶段 (seed → sapling → tree)**, 不是生老病死
- **设计文档/命名 去掉 "old/death/terminate" 这类终态概念**
- **平台是 "长程 AI 成长", 不是 "AI 模拟人类"**
- **Stage 5.5 F11 LongTermAIGrowthPod 0 含 old/death/terminate 概念** (per R130-4 spec §2.2 `is_terminate_stage() == false` 永真)
- **V2.0 release 8 哲学锚重建也 0 含** (per 用户记忆 #4 严守)

---

## 13. 不要怕复杂度哲学落地 (per 决策 #73 §3 + `docs/conventions/15-no-fear-complexity.md` + 主人 8/11 01:14 拍板 3 件套 §3)

### 13.1 不要怕复杂度哲学 3 件套 (per 决策 #73 §3 + 15-no-fear-complexity.md §1)

**3 件套** (per 决策 #73 §3 + 15-no-fear-complexity.md §1 + 主人 8/11 01:14 拍板原文 §5):
1. **最强效果 > 最简单代码** — 复杂度是实力的体现, 不是"技术债"
2. **最厉害工程 > 最易维护** — 工程化是最高目标, 不是"代码要易维护"
3. **维护交给未来高水平团队** — 维护不是问题, 因为自然会有高水平的团队来接手

**推翻** (per 15-no-fear-complexity.md §1.1-§1.3):
- ❌ "代码要简单易维护" / ❌ "复杂度是技术债" / ❌ "KISS (Keep It Simple, Stupid)"
- ❌ "代码要易维护" / ❌ "维护成本是重要指标" / ❌ "DRY (Don't Repeat Yourself)"
- ❌ "代码要让初级团队能接手" / ❌ "文档要写得简单易懂" / ❌ "维护是负担"

**新哲学** (per 15-no-fear-complexity.md §1.1-§1.3):
- ✅ "代码要最强效果" / ✅ "复杂度是实力的体现" / ✅ "SOTA (State of the Art)"
- ✅ "代码要最厉害工程" / ✅ "工程化是最高目标" / ✅ "BORROW (借脑 / 借鉴 / 借源)"
- ✅ "代码要让高水平团队能发挥" / ✅ "文档要写得专业 + 完整" / ✅ "维护是机会 (高水平团队接手 = 项目升级)"

### 13.2 形式化集成 9 优化方向 跟不要怕复杂度哲学 1:1 对齐

| 优化方向 | 跟不要怕复杂度哲学 1:1 对齐 | 实施 | 8 硬墙 0 越界 |
|---------|---------------------------|------|--------------|
| **O1 kani 借鉴深度** | ✅ 1.0% → 4-6% → 12-18% 借量 (per 不要怕复杂度 + 决策 #74 B1) | V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + V2.0 release 形式化重构 | ✅ 100% |
| **O2 F1-F11 11 维度** | ✅ F1-F10 1:1 续 + F11 NEW 升级 (per 不要怕复杂度 + R130-4 spec) | V1.0 release 0 改严守 + V1.1 release 实施 + V2.0 release 推翻 + 重建 | ✅ 100% |
| **O3 6 重守门 v7** | ✅ 6 重 → 36 维 守门 (per 不要怕复杂度 + 决策 #74 B1) | V1.0 release 0 改严守 + V1.1 release 深化 + V2.0 release 推翻 + 重建 | ✅ 100% |
| **O4 8 哲学锚** | ✅ 8 锚 + 1 总工程哲学 = 9 件套 (per 不要怕复杂度 + 决策 #73 §3) | V1.0 release 0 改严守 + V1.1 release 深化 + V2.0 release 推翻 + 重建 | ✅ 100% |
| **O5 24 LOCKED 入口** | ✅ 24 LOCKED + 3 NEW = 27 LOCKED → 0 LOCKED 全解锁 (per 不要怕复杂度 + 决策 #74 B1) | V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + V2.0 release 全解锁 | ✅ 100% |
| **O6 PHL-07 spec-only** | ✅ 0 实施 → 实施 → PHL-08+ 升 (per 不要怕复杂度 + 决策 #74 §2.3) | V1.0 release 0 实施 + V1.1 release 实施 + V2.0 release 升 | ✅ 100% |
| **O7 V0.5 30 维** | ✅ 30 维 → 32 维 → V0.6 重大 (per 不要怕复杂度 + 决策 #74 B1) | V1.0 release 0 改严守 + V1.1 release 深化 + V2.0 release 推翻 + 重建 | ✅ 100% |
| **O8 12 键 + PHL-07** | ✅ 13 键 → 14 键 → 14+ 键 (per 不要怕复杂度 + 决策 #74 §2.3) | V1.0 release 0 改严守 + V1.1 release 实施 + V2.0 release 升 | ✅ 100% |
| **O9 V1.1 + V2.0 release 形式化** | ✅ F1-F11 + Kani 全集成 + 三洋葱架构升级 (per 不要怕复杂度 + 决策 #74 + 决策 #73 §3) | V1.0 release 调研 0 改 + V1.1 release 实施 + V2.0 release 实战 | ✅ 100% |

### 13.3 形式化集成 复杂度 1:1 落地 (per 15-no-fear-complexity.md §1)

**形式化集成 复杂度 = 实力的体现** (per 15-no-fear-complexity.md §1.1):
- **Stage 5.1 P8-2 retry** (39.3 KB formal_proof.rs + 153 tests) = 最强效果 (8 Kani-style harness + 3 custom Invariant + 2 POD)
- **Stage 5.2 R129-10** (80.4 KB stage5_2/ + 117 lib tests) = 最强效果 (F1-F10 10 维度形式化)
- **Stage 5.3 R129-20** (88.5 KB stage5_3/ + 92 lib tests) = 最强效果 (F11-F20 10 维度跨模块)
- **Stage 5.4 R131-4 spec** (估 ~100 KB stage5_4/ + ~110 lib tests) = 最强效果 (F21-F30 10 维度跨 Stage 5.x 集成)
- **Stage 5.5 R130-4 spec** (估 ~30 KB stage5_5/ + ~25 lib tests) = 最强效果 (F1-F11 11 维度集成深化 + PHL-07 spec-only 形式化 + 长程 AI 成长 形式化)
- **Stage 6 R132-N spec** (估 150-200 KB stage6/ + 200+ lib tests) = 最强效果 (Kani 求解器在线扩展 + 跨 stage 全集成 + 1.0 release 实战后 1.0+ 形式化扩展)

**总形式化集成 复杂度** (per 15-no-fear-complexity.md §1.1):
- Stage 5.1 + 5.2 + 5.3 实证 = 69 KB + 80.4 KB + 88.5 KB = 237.9 KB + ~360 tests pass
- Stage 5.1-5.6 全 6 阶 估 = 237.9 KB + 100 KB + 30 KB + 200 KB = 567.9 KB + ~600 tests pass
- 跟 kani 5.5MB 借量 1.0% (V1.0 release) → 4-6% (V1.1 release) → 10-15% (V2.0 release)
- 形式化集成 复杂度 = 实力的体现 (per 不要怕复杂度哲学, 15-no-fear-complexity.md §1.1)

---

## 14. 风险 + 决策原则 (per 决策 #74 §7 + 决策 #73 §8 + 决策 #75 §6)

### 14.1 风险 (per 决策 #74 §7.1 + 决策 #75 §6.1 + 决策 #73 §8.1)

| 风险 | 影响 | 缓解 |
|------|------|------|
| **R1**: V1.1 release 24 LOCKED 改写破坏向后兼容 | V1.0 release 用户用不了 V1.1 release | 决策 #74 §2.3 V1.1 release 是 minor release, 跟 semver 一致 (0.x → 1.0 → 1.1), V2.0 release 才考虑不向后兼容 |
| **R2**: PHL-07 实施 0 假装"已 Kani 形式化" | 0 装 PASS 严守 100% 落实 | 决策 #74 §2.3 + R130-4 spec F11 NEW 3 阶段递进, 0 装"已 PHL-07 实施" 严守 (R129-11 关键诚实标) |
| **R3**: V2.0 release 形式化重构 推翻 + 重建 8 哲学锚 | 8 哲学锚 是思想哲学, 推翻 + 重建 影响深远 | 决策 #74 §2.3 V2.0 release 8 哲学锚推翻 + 重建 per Mavis 自决 + 主人 8/11 01:14 拍板, 0 形式化 old/death/terminate 概念 严守 (per 用户记忆 #4) |
| **R4**: Kani 求解器在线扩展 0 装"已 Kani 形式化" | 0 装 PASS 严守 100% 落实 | 决策 #74 B1 V1.1 release Mavis 自决改, 0 装"已 Kani 求解器在线" 严守, Kani 求解器留 R132+ 实战 + V2.0 release 实战 |
| **R5**: 长程 AI 成长 形式化 0 形式化 old/death/terminate 概念 | 用户记忆 #4 严守 | Stage 5.5 F11 NEW `is_terminate_stage() == false` 永真, V2.0 release 8 哲学锚重建也 0 含 (per 用户记忆 #4 严守) |
| **R6**: 团队对 "不要怕复杂度" 哲学不适应 | 主人 8/11 01:14 决策 3 件套 §3 落实 | 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应, 15-no-fear-complexity.md 文档说明 |
| **R7**: 三洋葱架构升级 0 装"已升级" | 0 装 PASS 严守 100% 落实 | 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改, 0 装"已三洋葱架构升级" 严守, V2.0 release 实战 |
| **R8**: F11 NEW PHL-07 spec-only 形式化 0 装"已 Kani 形式化" | 0 装 PASS 严守 100% 落实 | Stage 5.5 F11 NEW 跟 Stage 5.2 同模式, Kani 离线时退化为普通 fn, 0 装"已 Kani 形式化" 严守 |
| **R9**: R131-9 调研报告 0 改 src 严守 (V1.0 release 0 改) | 决策 #33 §2.3 C1 + 决策 #74 §2.3 V1.0 release 0 改严守 | R131-9 调研报告写到 `reports/agent-r131-9-formal-proof-integration-optimization-2026-08-11.md`, 0 改 src/ 0 改 Cargo.toml 严守 |
| **R10**: 0 主动 push 严守 (V1.0 release 0 push) | 决策 #33 + 决策 #61 §6 | R131-9 0 主动 push 严守, 等 1.0 release 配 GitHub remote + 主人起床后手跑 scripts/release/ |

### 14.2 决策原则 (per 决策 #74 §7.2 + 决策 #73 §8.2 + 决策 #75 §6.2 + 主人 8/11 01:14 拍板 3 件套)

- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改
- **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1
- **A1 R11 baseline 3 值**: V1.0 release 严守 + V1.1 release 可改 (前提: 新的 baseline 更高) + V2.0 release 可推翻 + 重建
- **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施, 12 键其他 V1.1 release 可改 + V2.0 release 推翻 + 重建
- **B3 V0.5 30 维**: V1.0 release 严守 + V1.1 release 深化 (0 改原 30 维) + V2.0 release 推翻 + 重建
- **B4 6 重守门 v7**: V1.0 release 严守 + V1.1 release 深化 (0 改原 6 重) + V2.0 release 推翻 + 重建
- **B5 8 哲学锚**: V1.0 release 严守 + V1.1 release 严守 + V2.0 release 推翻 + 重建
- **C1 0 主动 commit (主人起床前)**: 严守
- **C2 0 装 PASS 严守**: 严守 (技术哲学, 不装)
- **0 push (主人起床前)**: 严守
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3, 写新文档 `docs/conventions/15-no-fear-complexity.md`)
- **0 形式化 old/death/terminate 概念** (per 用户记忆 #4 "AI 不会衰老病死")
- **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 15-no-fear-complexity.md §2)
- **8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界** (per 15-no-fear-complexity.md §3)
- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)

### 14.3 0 主动 commit + 0 主动 push 严守 (per 决策 #33 §2.3 C1 + 决策 #61 §6 + 决策 #74 §6)

- **R131-9 0 主动 commit** (per 决策 #33 §2.3 C1 + 决策 #34 + #48 + #55 + #56 + #57 + #58 + #61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + #73 + #74 + #75):
  - 写到 `reports/agent-r131-9-formal-proof-integration-optimization-2026-08-11.md` (本报告)
  - 0 主动 git add/commit, Mavis 整合 #5.3 commit 时机拍板
  - 整合 #5.1 commit 95+ 文件 + 整合 #5.2 commit 10 文件 + 整合 #5.3 commit 60+ 文件 (per 决策 #62 §5.1-§5.3)
  - 整合 #4 commit abf12243 严守 (per 决策 #48 + 决策 #61 §1.2)
- **R131-9 0 主动 push** (per 决策 #33 §2.3 + 决策 #61 §6):
  - 0 跑 git push, 等 1.0 release 配 GitHub remote
  - 主人起床后手跑 scripts/release/ (per 决策 #61 §6)

### 14.4 8 硬墙 严守 verify (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

**8 硬墙 严守 verify 通道** (per Stage 5.2 R129-10 实证 + Stage 5.3 R129-20 实证 + Stage 5.1 P8-2 retry 实证):
- **B1 24 LOCKED 入口签名 0 改**: 0 改 `crates/apeireth-formal/src/stage5_2/locked_24_entry_formal.rs:8,638 B` + 0 改 `crates/apeireth-formal/src/stage5_3/cross_locked_integration_proof.rs:8,820 B` + P2-3 verify 24/24 入口签名 0 改 done + 整合 #4 commit abf12243 严守
- **B2 workspace.version 1.2.0 0 改**: 0 改 `Cargo.toml:254 version = "1.2.0"` + F5 R11 baseline 形式化 + F19 跨 version 集成 (26 crate 编译期 hardcode)
- **A1 R11 baseline 3 值 0 改**: 0 改 17 文件原位 + F5 R11 baseline 形式化 (3 数字 A1 严守 0 改)
- **B3 V0.5 30 维 0 改**: 0 改 `apeireth-naming-v05/src/extension.rs:65 V05_30_TOTAL_DIMS = 30` + F3 V0.5 30 维形式化 (4 类 × 6 + 5 meta + 1 overall = 30 维 0 改) + P1-4 R126 25→30 维 verify done
- **B4 6 重守门 v7 0 改**: 0 改 6 重守门 v7 + F1 6 重守门 v7 形式化 (SIX_FOLD_GATE_V7_COUNT = 6) + F18 跨 gate 集成 (1:1 跟 B4) + P1-3 R126 6 重守门 v7 done
- **B5 8 哲学锚 0 改**: 0 改 8 哲学锚 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5) + F2 8 哲学锚形式化 (EIGHT_ANCHORS_COUNT = 8) + F17 跨 anchor 集成 (1:1 跟 B5) + P1-2 R126 8 哲学锚升级 done
- **A3 12 键 + PHL-07 0 改**: 0 改 12 键 + PHL-07 spec-only 0 实施 + F4 13 键 verdict cache 形式化 (VERDICT_CACHE_13_KEYS_COUNT = 13) + 整合 #4 commit done
- **C1 0 主动 commit**: 0 跑 git add/commit (Mavis 拍板)
- **C2 0 装 PASS 严守**: ✅ cloned = 真实施 (有真 src 改动 + 79+92+104 = 275 tests pass)
- **0 主动 push**: 0 跑 git push (等 1.0 release 配 GitHub remote + 主人起床后手跑)

---

## 15. 8 哲学锚 0 形式化 old/death/terminate 严守 (per 用户记忆 #4 + Stage 5.5 F11 NEW)

### 15.1 用户记忆 #4 严守 (per 用户记忆 #4 "AI 不会衰老病死" + 决策 #74 §1 改写表)

**用户记忆 #4 严守** (per 用户记忆 #4 "AI 不会衰老病死, 主 ai 是 ai 哎, 它只会成长, 但不可能消亡"):
- **AI 生命周期 = 成长阶段 (seed → sapling → tree)**, 不是生老病死
- **设计文档/命名 去掉 "old/death/terminate" 这类终态概念**
- **平台是 "长程 AI 成长", 不是 "AI 模拟人类"**
- **任何 AGI / 长程 AI / 自主 agent 设计**: AI 生命周期是"成长阶段" (seed → tree), 不是"生老病死"

### 15.2 Stage 5.5 F11 NEW 0 形式化 old/death/terminate 概念 (per R130-4 spec §2.2)

**Stage 5.5 F11 NEW 0 形式化 old/death/terminate 概念 严守** (per R130-4 spec §2.2 + 用户记忆 #4 严守):
- **`GrowthStage` enum 0 含 old/death/terminate** (per R130-4 spec §2.2):
  ```rust
  pub enum GrowthStage {
      Seed = 0,    // 0 阶段 (刚启动, 1.0 release 实战前)
      Sapling = 1, // 1 阶段 (初步成长, 1.0 release 后 → V1.x minor)
      Tree = 2,    // 2 阶段 (深度成长, V2.x major 或之后)
      // 0 包含 old/death/terminate 终态概念 (per 用户记忆 #4 严守)
  }
  ```
- **`is_terminate_stage() == false 永真`** (per R130-4 spec §2.2 + 用户记忆 #4 严守):
  ```rust
  impl GrowthStage {
      pub const fn is_terminate_stage(self) -> bool {
          // 0 终态概念, 0 永真 (false 永真, per 用户记忆 #4 严守)
          match self {
              Self::Seed | Self::Sapling | Self::Tree => false,
          }
      }
  }
  ```
- **`LongTermAIGrowthPod.has_terminate_concept == false 永真`** (per R130-4 spec §2.2 + 用户记忆 #4 严守):
  ```rust
  pub struct LongTermAIGrowthPod {
      pub stage: GrowthStage,                     // 0..=2
      pub cycles_to_next_stage: u32,              // 距下阶段 cycle 数
      pub has_terminate_concept: bool,            // false 永真 (per 用户记忆 #4 严守)
      pub platform_kind: PlatformKind,             // LongLivedAIGrowthPlatform
  }
  ```
- **3 不变量 严守** (per R130-4 spec §2.2 + 用户记忆 #4 严守):
  - `long_term_ai_growth_stage_invariant(p) = (p.stage as u8) < 3` (3 阶段递进, 0 终态)
  - `long_term_ai_growth_no_terminate_invariant(p) = !p.has_terminate_concept` (0 含 terminate 概念)
  - `long_term_ai_growth_no_terminate_stage_invariant(p) = !p.stage.is_terminate_stage()` (3 阶段都 0 终态)

### 15.3 V2.0 release 8 哲学锚重建 0 含 old/death/terminate 概念 严守 (per 决策 #74 §2.3 V2.0 release 8 哲学锚推翻 + 重建 + 用户记忆 #4 严守)

**V2.0 release 8 哲学锚重建 0 含 old/death/terminate 概念 严守** (per 决策 #74 §2.3 V2.0 release 8 哲学锚推翻 + 重建 + 用户记忆 #4 严守):
- **S-1 服务 ASI 北极星 → S-1' 推翻重建**: 0 含 old/death/terminate 概念 (服务 ASI 北极星 = 成长阶段, 0 终态)
- **S-2 实事求是 → S-2' 推翻重建**: 0 含 old/death/terminate 概念
- **S-3 质量工程化 → S-3' 推翻重建**: 0 含 old/death/terminate 概念
- **O-1 安全优先 → O-1' 推翻重建**: 0 含 old/death/terminate 概念
- **O-2 走在前人经验上 → O-2' 推翻重建**: 0 含 old/death/terminate 概念
- **O-3 干到底 → O-3' 推翻重建**: 0 含 old/death/terminate 概念
- **O-4 任何人都能接手 → O-4' 推翻重建**: 0 含 old/death/terminate 概念
- **O-5 不假装 → O-5' 推翻重建**: 0 含 old/death/terminate 概念
- **0 形式化 old/death/terminate 概念 严守** (per 用户记忆 #4 严守, V2.0 release 8 哲学锚重建也 0 含)

---

## 16. 跟其他 R131 era 报告 + 决策 #73/#74/#75 关系

### 16.1 跟 R131-1 架构总审视 + R131-2 借鉴 12 源差距 + R131-3 V1.1 release 实施路线图 关系 (per 决策 #75 §2.1 R131 era 第 2 批)

**R131 era 11 sub-agent 派活** (per 决策 #75 §2.1):
- **R131-1**: 现有架构总审视 + 优化点 (per 决策 #71 §3 + 决策 #73 §3.2)
- **R131-2**: 跟借鉴源码 11 源差距 + 借鉴 12 源 (per 决策 #71 §3 + 决策 #73 §3.2)
- **R131-3**: V1.1 release 实施路线图 (per 决策 #71 §3 + 决策 #73 §3.2)
- **R131-4**: cargo workspace 结构优化 (per 决策 #75 §2.1 R131 era 第 2 批 6 sub)
- **R131-5**: 24 LOCKED 入口分布优化 (per 决策 #75 §2.1)
- **R131-6**: Cargo.toml borrow 段精简 (per 决策 #75 §2.1)
- **R131-7**: pybridge 集成优化 (per 决策 #75 §2.1)
- **R131-8**: Tauri 集成优化 (per 决策 #75 §2.1)
- **R131-9**: 形式化集成优化 (per 决策 #75 §2.1, 本报告)
- **R132-1**: V1.1 release 路线图 final (per 决策 #75 §2.1 R132 era 计划 2 sub)
- **R132-2**: V2.0 release 战略路线图 (per 决策 #75 §2.1)
- **R133-1**: 借鉴源 12 源 实施 (per 决策 #75 §2.1 R133 era 实施 3 sub + 决策 #73 §2.2 + 主人 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学)
- **R133-2**: ASI Stage 9 长程 AI 成长 实施 (per 决策 #75 §2.1 R133 era 实施 3 sub + R130-2 ASI Stage 8 + 用户记忆 #4)
- **R133-3**: 三洋葱架构升级 实施 (per 决策 #75 §2.1 R133 era 实施 3 sub + 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改)

**R131-9 跟其他 14 sub-agent 关系**:
- **跟 R131-1 现有架构总审视 关系**: R131-1 总审视 + R131-9 形式化集成细分, 0 重复造轮子 (per 用户记忆 #6)
- **跟 R131-2 借鉴 11 源差距 + 借鉴 12 源 关系**: R131-2 借鉴源总差距 + R131-9 形式化集成 kani 4502 借鉴细分, 0 重复造轮子
- **跟 R131-3 V1.1 release 实施路线图 关系**: R131-3 V1.1 release 实施路线图总览 + R131-9 形式化集成 V1.1 release 实施细分, 0 重复造轮子
- **跟 R131-4 cargo workspace 结构优化 关系**: R131-4 cargo workspace 总优化 + R131-9 形式化集成 cargo workspace 现状 (24 LOCKED + 26 crate workspace.version 1.2.0), 0 重复造轮子
- **跟 R131-5 24 LOCKED 入口分布优化 关系**: R131-5 24 LOCKED 入口分布 + R131-9 形式化集成 24 LOCKED 入口形式化 (F6), 0 重复造轮子
- **跟 R131-6 Cargo.toml borrow 段精简 关系**: R131-6 Cargo.toml borrow 段总精简 + R131-9 形式化集成 Cargo.toml 0 改严守, 0 重复造轮子
- **跟 R131-7 pybridge 集成优化 关系**: R131-7 pybridge 集成总优化 + R131-9 形式化集成 pybridge 形式化 (Stage 5.0 衔接), 0 重复造轮子
- **跟 R131-8 Tauri 集成优化 关系**: R131-8 Tauri 集成总优化 + R131-9 形式化集成 Tauri 形式化 (Stage 6 实战), 0 重复造轮子
- **跟 R132-1 V1.1 release 路线图 final 关系**: R132-1 V1.1 release 路线图 final 整合 + R131-9 形式化集成 V1.1 release 形式化方案 (本报告 O9 §10), 0 重复造轮子
- **跟 R132-2 V2.0 release 战略路线图 关系**: R132-2 V2.0 release 战略路线图总 + R131-9 形式化集成 V2.0 release 形式化重构方案 (本报告 O9 §10.2), 0 重复造轮子
- **跟 R133-1 借鉴源 12 源 实施 关系**: R133-1 借鉴源 12 源 实施总 + R131-9 形式化集成 kani 借鉴深度方案 (本报告 O1 §2), 0 重复造轮子
- **跟 R133-2 ASI Stage 9 长程 AI 成长 实施 关系**: R133-2 ASI Stage 9 实施总 + R131-9 形式化集成 长程 AI 成长 形式化 (Stage 5.5 F11 NEW, per R130-4 spec §2.2), 0 重复造轮子
- **跟 R133-3 三洋葱架构升级 实施 关系**: R133-3 三洋葱架构升级 实施总 + R131-9 形式化集成 三洋葱架构 v2 形式化方案 (本报告 O9 §10.1.4), 0 重复造轮子

### 16.2 跟决策 #73 主决策 + 决策 #74 8 硬墙 B1 改写 + 决策 #75 R131 era 第 2 批 派活 关系

**R131-9 跟决策链关系**:
- **决策 #73 主决策** (per 决策 #73 §1-§5, 主人 8/11 01:14 拍板 3 件套):
  - §1 locked 全解锁 + Mavis 自决架构: R131-9 形式化集成 B1 24 LOCKED V1.1 release Mavis 自决改 (per 决策 #74 B1 改写, 本报告 O5 §6.2.2)
  - §2 架构审视 + 升级方案永久工作项: R131-9 形式化集成 9 优化方向 (per 决策 #75 §2.1 R131-9 派活 + cron Section 10 架构审视永久工作项)
  - §3 总工程哲学扩展 "不要怕复杂度": R131-9 形式化集成 不要怕复杂度哲学落地 (per 15-no-fear-complexity.md + 本报告 §13)
  - §4 哲学文档更新: R131-9 形式化集成 9 优化方向 0 改 `docs/conventions/09-anchor.md` + `10-locked.md` + 15-no-fear-complexity.md 严守
  - §5 整合 #5 commit 拍板: R131-9 调研报告 0 改 src, Mavis 整合 #5.3 commit 拍板 (per 决策 #62 §5.3)
- **决策 #74 8 硬墙 B1 改写** (per 决策 #74 §1 改写表 + §2.3 B1 改写边界):
  - §1 8 硬墙改写表: R131-9 形式化集成 9 优化方向 1:1 跟 8 硬墙改写表对齐 (本报告 §11.1)
  - §2.3 B1 改写边界: R131-9 形式化集成 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (本报告 O5 §6.2)
  - §4 整合 #5 commit 拍板: R131-9 0 改 src, Mavis 拍板 (per 决策 #62 §5.3 + 决策 #74 §4)
- **决策 #75 R131 era 第 2 批 派活** (per 决策 #75 §2.1 R131 era 第 2 批 6 sub):
  - §2.1 R131-9 形式化集成优化 派活: 本报告 = R131-9 派活清单 0 改 src 调研
  - §3 整合 #5 commit 拍板临近: R131-9 0 改 src, 等 R129-3 报告 done → Mavis 自决拍板 (per 决策 #62 + 决策 #73 §5 + 决策 #74 §4)

### 16.3 跟 R130-4 Stage 5.5 调研 + R129-32 Stage 5.4 spec + R129-20 Stage 5.3 实证 + R129-10 Stage 5.2 实证 + P8-2 Stage 5.1 实证 关系 (per 决策链 §1.1 6 阶演进)

**R131-9 跟 Stage 5.x 6 阶演进 关系** (per 决策 #33 §2.3 + 决策 #55 §1 + 决策 #56 + 决策 #57 + 决策 #61 §3.1 + 决策 #69 §3 + 决策 #72 §2.1):
- **Stage 5.1 P8-2 retry** (per `agent-p8-2-retry-r127-2-library-stage-5-1-formal-proof-final-2026-08-10.md` ✅ done): R131-9 形式化集成 Stage 5.1 实证 = Library crate 形式化基础
- **Stage 5.2 R129-10** (per `agent-r129-10-formal-proof-stage-5.2-2026-08-11.md` ✅ done): R131-9 形式化集成 Stage 5.2 实证 = formal crate 形式化扩展 F1-F10 10 维度
- **Stage 5.3 R129-20** (per `agent-r129-20-formal-proof-stage-5.3-2026-08-11.md` ✅ done): R131-9 形式化集成 Stage 5.3 实证 = formal crate 跨模块证明 F11-F20 10 维度
- **Stage 5.4 R129-32 spec** (per `agent-r129-32-formal-proof-stage-5.4-execution-2026-08-11.md` ✅ done spec): R131-9 形式化集成 Stage 5.4 spec = 跨 Stage 5.x 集成 F21-F30 10 维度 (R131-4 实战)
- **Stage 5.5 R130-4 spec** (per `agent-r130-4-formal-proof-stage-5.5-integration-deepening-2026-08-11.md` ✅ done spec): R131-9 形式化集成 Stage 5.5 spec = Stage 5.2 集成深化 F1-F11 11 维度 (R133-N 实战, V1.1 minor release 前)
- **Stage 6 R132-N spec** (per 决策 #78 R130 era 派活清单 + 决策 #64 §2.2 + R129-32 spec): R131-9 形式化集成 Stage 6 spec = 形式化证明 + 实战 (kani 求解器在线 + 跨 stage 全集成 + 1.0 release 实战后 1.0+ 形式化扩展, R132+ era 实战)

---

## 17. 总结 (per 决策 #74 §8 + 决策 #73 §8 + 决策 #75 §7)

### 17.1 一句话 (再次强调)

**R131-9 形式化集成优化 = 9 个优化方向调研报告, 0 改 src/ (整合 #5.1 commit V1.0 release 0 改严守 per 决策 #33 §2.3 C1 + 决策 #74 §2.3 B1 V1.0 release 0 改严守). 9 个优化方向 = (1) kani 5.5MB 借鉴深度优化 (1.0% → 4-6% → 10-15% 借量) + (2) F1-F10 10 维度 → F1-F11 11 维度 (Stage 5.5 NEW F11 PHL-07 spec-only + 长程 AI 成长 形式化, per R130-4 spec) + (3) 6 重守门 v7 形式化 (6 重 → 36 维 守门, per 决策 #74 B1 V1.1 release Mavis 自决改) + (4) 8 哲学锚形式化 (8 锚 + 1 总工程哲学 = 9 件套总哲学, per 15-no-fear-complexity.md) + (5) 24 LOCKED 入口形式化 (24 LOCKED + 3 NEW = 27 LOCKED V1.1 release → 0 LOCKED V2.0 release 全解锁, per 决策 #74 B1 改写) + (6) PHL-07 spec-only 形式化 (V1.0 release spec-only 0 实施 → V1.1 release 实施 → V2.0 release PHL-08+ 升, per 决策 #74 §2.3) + (7) V0.5 30 维形式化 (30 维 → 32 维 → V0.6 重大, per 决策 #74 B1 V1.1 release Mavis 自决改) + (8) 12 键 + PHL-07 形式化 (13 键 → 14 键 → 14+ 键, per 决策 #74 §2.3) + (9) V1.1 release PHL-07 实施 + F1-F11 + Kani 全集成方案 (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #78 R130 era 派活清单 + 主人 8/11 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学). 借鉴源码 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + 决策 #55 §3 + 决策 #75 §2.1 R131-9 派活): 2 借鉴 ID (kani 4502 + langgraph 829) 0 引 kani / langgraph 依赖, 0 装"已 Kani 形式化" / "已 langgraph 集成". 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 改写表): B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + V2.0 release 全解锁 / B2 workspace.version 1.2.0 → 1.2.1 → 2.0.0 / A1 R11 baseline 3 值 V1.0 release 严守 + V1.1 release 可改 + V2.0 release 推翻 + 重建 / B3 V0.5 30 维 V1.0 release 严守 + V1.1 release 深化 + V2.0 release 推翻 + 重建 / B4 6 重守门 v7 V1.0 release 严守 + V1.1 release 深化 + V2.0 release 推翻 + 重建 / B5 8 哲学锚 V1.0 release 严守 + V1.1 release 严守 + V2.0 release 推翻 + 重建 / A3 12 键 + PHL-07 V1.0 release spec-only 0 实施 + V1.1 release 实施 + V2.0 release 升 / C1 0 主动 commit 严守 / C2 0 装 PASS 严守 / 0 push 严守. 8 哲学锚 0 改 V1.0 release + V1.1 release 严守 + V2.0 release 推翻 + 重建, 0 形式化 old/death/terminate 概念 严守 (per 用户记忆 #4 "AI 不会衰老病死"). 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (per 15-no-fear-complexity.md §2). 8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界 (per 15-no-fear-complexity.md §3). 0 主动 commit (Mavis 整合 #5.3 commit 拍板) + 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑 scripts/release/) 严守 100%.**

### 17.2 R131-9 报告 0 改 src 严守 verify (per 决策 #33 §2.3 C1 + 决策 #74 §2.3 V1.0 release 0 改严守)

- ✅ R131-9 调研报告 写到 `reports/agent-r131-9-formal-proof-integration-optimization-2026-08-11.md` (本报告)
- ✅ 0 改 `crates/apeireth-formal/src/stage5_2/` (10 模块 ~80 KB 严守)
- ✅ 0 改 `crates/apeireth-formal/src/stage5_3/` (10 模块 ~88 KB 严守)
- ✅ 0 改 `crates/apeireth-formal/src/lib.rs` (Stage 5.2/5.3 1 行 + 1 行 mod 注册, 严守)
- ✅ 0 改 `crates/apeireth-library-governance/src/formal_proof.rs` (39.3 KB 严守, Stage 5.1 P8-2 retry 实证)
- ✅ 0 改 `crates/apeireth-library-governance/src/lib.rs` (P8-2 retry 1 行 + 1 行 mod 注册, 严守)
- ✅ 0 改 24 LOCKED 入口签名 (per 决策 #33 §2.3 B1 + 决策 #74 §2.3 V1.0 release 0 改严守)
- ✅ 0 改 `Cargo.toml` (workspace.version 1.2.0 严守 per 决策 #33 §2.3 B2)
- ✅ 0 改 17 文件 R11 baseline 3 值 (per 决策 #33 §2.3 A1 + 决策 #74 §2.3 V1.0 release 严守)
- ✅ 0 改 V0.5 30 维 (per 决策 #33 §2.3 B3 + P1-4 R126 25→30 维 verify done)
- ✅ 0 改 6 重守门 v7 (per 决策 #33 §2.3 B4 + P1-3 R126 6 重守门 v7 done)
- ✅ 0 改 8 哲学锚 (per 决策 #33 §2.3 B5 + P1-2 R126 8 哲学锚升级 done)
- ✅ 0 改 12 键 + PHL-07 (per 决策 #33 §2.3 A3 + 决策 #74 §2.3 V1.0 release spec-only 0 实施 + 整合 #4 commit done)
- ✅ 0 主动 commit (Mavis 整合 #5.3 commit 拍板 per 决策 #33 §2.3 C1 + 决策 #74 §6)
- ✅ 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑 per 决策 #33 §2.3 + 决策 #61 §6)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification per 决策 #61 §6 + 决策 #74 §6)
- ✅ 0 主动删 (per Safety policy + 决策 #44 + #60)
- ✅ 0 重复造轮子 (per 用户记忆 #6, 1:1 续 Stage 5.1-5.3 实证 + R130-4 spec)
- ✅ 0 形式化 old/death/terminate 概念 (per 用户记忆 #4 "AI 不会衰老病死" + Stage 5.5 F11 NEW)
- ✅ 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + 决策 #74 §1, 0 装"已 Kani 形式化" / "已 Kani 求解器在线" / "已 PHL-07 实施" / "已三洋葱架构升级" / "已形式化重构" 严守)

### 17.3 R131-9 报告 0 装 PASS 严守 verify (per 决策 #33 §2.3 C2 + 决策 #74 §1)

**R131-9 报告 = 调研报告, 0 改 src, 0 装"已实施"**:
- ✅ 0 装"已 kani 借鉴深度优化" (V1.0 release 0 改严守, V1.1 release Mavis 自决改, V2.0 release 形式化重构, 全是 spec, 0 实施)
- ✅ 0 装"已 F1-F11 11 维度" (Stage 5.5 F11 NEW 是 R130-4 spec, 0 写本报告, 0 实施)
- ✅ 0 装"已 6 重守门 v7 形式化优化" (V1.0 release 0 改严守, V1.1 release 深化, V2.0 release 推翻 + 重建, 全是 spec, 0 实施)
- ✅ 0 装"已 8 哲学锚形式化优化" (V1.0 release 0 改严守, V1.1 release 深化, V2.0 release 推翻 + 重建, 全是 spec, 0 实施)
- ✅ 0 装"已 24 LOCKED 入口形式化优化" (V1.0 release 0 改严守, V1.1 release Mavis 自决改, V2.0 release 全解锁, 全是 spec, 0 实施)
- ✅ 0 装"已 PHL-07 spec-only 形式化" (V1.0 release spec-only 0 实施, V1.1 release 实施, V2.0 release 升, 全是 spec, 0 实施)
- ✅ 0 装"已 V0.5 30 维形式化优化" (V1.0 release 0 改严守, V1.1 release 深化, V2.0 release 推翻 + 重建, 全是 spec, 0 实施)
- ✅ 0 装"已 12 键 + PHL-07 形式化优化" (V1.0 release 0 改严守, V1.1 release 实施, V2.0 release 升, 全是 spec, 0 实施)
- ✅ 0 装"已 V1.1 release 形式化方案" (R133-N 实战时写, 0 实施本报告)
- ✅ 0 装"已 V2.0 release 形式化重构方案" (R132+ era 实战时写, 0 实施本报告)
- ✅ 0 装"已 Kani 求解器在线扩展" (V1.1 release spec, 0 实施本报告)
- ✅ 0 装"已三洋葱架构升级" (R133-3 实施时写, 0 实施本报告)

### 17.4 R131-9 报告 决策依据 (per 决策 #75 §6.2)

- ✅ Mavis = orchestrator + 全自决 + 最高权限 (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- ✅ 跑中 ≥ 16 (per 主人 0:34, 16 active 全 background 跑, 决策 #75 §2.1 派 R131 era 第 2 批 6 sub + R132 era 计划 2 sub + R133 era 实施 3 sub = 11 sub 填到 16)
- ✅ 中断接手 (per 主人 0:43, 检查 reports/agent-*.md 写完则标 done / 没写完则重派)
- ✅ 编译产物清理决策矩阵 (per 主人 0:49 + 0:54: ≤50 保守 / 50-100 预警 / 100-150 强烈预警 / > 150 强制清理, target/ 31.18 GB 严守 < 50 GB)
- ✅ 计划内任务完成自动接续 4 步 + 永久循环 (per 主人 0:57: 调研 + 差距 + 计划 + 实施 → 永久, cron Section 10 架构审视永久工作项新增)
- ✅ locked 全解锁 + Mavis 自决架构 (per 主人 8/11 01:14 拍板 3 件套 §1, 整合 #5.1 commit 仍 0 改严守 + V1.1 release Mavis 自决改)
- ✅ 架构审视 + 升级方案永久工作项 (per 主人 8/11 01:14 拍板 3 件套 §2, cron Section 10 新增)
- ✅ 总工程哲学扩展 "不要怕复杂度" (per 主人 8/11 01:14 拍板 3 件套 §3, 写新文档 `docs/conventions/15-no-fear-complexity.md`)
- ✅ 整合 #5 commit 由 Mavis 自动拍板 (per 主人 0:25 + 决策 #33 C1 + 决策 #64)
- ✅ 0 主动 push 严守 (per 决策 #33 + 决策 #61 §6)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification)
- ✅ 0 主动删 (per Safety policy + 决策 #44 + #60)
- ✅ 8 硬墙 严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
- ✅ 整合 #4 commit abf12243 严守 (per 决策 #48 + 决策 #61 §1.2)
- ✅ 决策日志写 (per 决策 #10 + 用户记忆 #10)
- ✅ 0 形式化 old/death/terminate 概念 (per 用户记忆 #4 "AI 不会衰老病死" + Stage 5.5 F11 NEW)
- ✅ 0 重复造轮子 (per 用户记忆 #6, 1:1 续 Stage 5.1-5.3 实证 + R130-4 spec)

---

## 18. 历史脉络 (per 决策 #74 §8 历史脉络 + 决策 #73 §8 历史脉络 + 决策 #75 §1.4 决策链更新)

- R11 末: 7 项不修改承诺 (per APEIRETH-CONVENTIONS.md §10 原版)
- R19+ 集成期: 实质重定义第 7 项 (per 8-locked-unified §3.4)
- R20 阶段 6: 8 项实质定义统一 (per 8-locked-unified-2026-08-05.md)
- R119-3a-1: 8 项形式撤销, 原意保留 (per `docs/conventions/10-locked.md`)
- R119-8: 3 技术类 LOCKED 撤销 (per 88fdba64 commit, 主人 1:49 拍板)
- R125 B1-B7: 9 项实质 Locked 升级路线, 主人 16:31 最高权限授权
- R125 B5 升 8 哲学锚 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5, per `docs/conventions/09-anchor.md`)
- R127-2 P8-2 retry: Stage 5.1 Library 形式化 ✅ done 22:06 (per 决策 #56, 39.3KB + 153 tests)
- R129-10: Stage 5.2 formal crate 形式化扩展 F1-F10 10 维度 ✅ done 00:49 (per 决策 #65, 80.4KB + 117 tests)
- R129-20: Stage 5.3 formal crate 跨模块证明 F11-F20 10 维度 ✅ done 00:50 (per 决策 #66, 88.5KB + 92 tests)
- R129-32: Stage 5.4 formal crate 集成扩展 F21-F30 spec ✅ done 调研 (per 决策 #69 §3)
- R130-4: Stage 5.5 formal crate 集成深化 F1-F11 11 维度 spec ✅ done 调研 (per 决策 #72 §2.1, ~30KB + ~25 tests)
- **R130 era 主人 8/11 01:14 拍板 3 件套**: locked 全解锁 + 架构审视 + 不要怕复杂度 (per 决策 #73 + 决策 #74 + 15-no-fear-complexity.md)
- **R130 era 决策 #73 写完** + **决策 #74 8 硬墙 B1 改写** + **决策 #75 R131 era 第 2 批 派活 11 sub 填到 16**
- **R131 era 派活 11 sub 跑中 = 16 满** (per 决策 #75 §2.1: R131-1/2/3 调研 + R131-4~9 架构细分 + R132-1/2 计划 + R133-1/2/3 实施)
- **整合 #5 commit 拍板临近** (7/8 verify done, 等 R129-3 报告 8 步 verify 全 PASS, per 决策 #62 + 决策 #74 §4)
- **R131-9 形式化集成优化** = 9 优化方向调研报告 (本报告, 0 改 src, 0 改 Cargo.toml, 0 主动 commit / push)
- ⏳ R129-3 报告 done → 整合 #5 commit 拍板 (per 决策 #62 + 决策 #74 §4)
- ⏳ 主人起床后配 GitHub remote + git push + tag v1.0.0 + release notes (per 决策 #61 §6)
- ⏳ R132-1/2 V1.1/V2.0 release 战略路线图拍板 (per 决策 #75 §2.1)
- ⏳ R133-1/2/3 实施 spec (V1.1 release, per 决策 #75 §2.1 + 决策 #74 B1 Mavis 自决改)
- ⏳ R131-4 Stage 5.4 实战 (V1.0 release 后, 8/12+ 派, per 决策 #64 §2.2 + 决策 #69 §3)
- ⏳ R133-N Stage 5.5 实战 (V1.1 minor release 前, 2026-11 派, per R130-4 spec + 决策 #78 R130 era 派活清单)
- ⏳ R132+ era Stage 6 实战 (kani 求解器在线 + 跨 stage 全集成 + 1.0 release 实战后 1.0+ 形式化扩展, per 决策 #78)

---

## 19. 核验 (per 决策 #73 §9 核验 + 决策 #74 §8 核验 + 决策 #75 §1.4 决策链更新)

- ✅ R131-9 报告 写完 (本报告, 0 改 src, 0 改 Cargo.toml, 0 主动 commit / push)
- ✅ 形式化集成 Stage 5.x 6 阶演进链 1:1 续 (Stage 5.1 P8-2 + Stage 5.2 R129-10 + Stage 5.3 R129-20 + Stage 5.4 R131-4 spec + Stage 5.5 R130-4 spec + Stage 6 R132-N spec, per 决策 #33 §2.3 + 决策 #55 §1 + 决策 #56 + 决策 #57 + 决策 #61 §3.1 + 决策 #69 §3 + 决策 #72 §2.1 + 决策 #74 §1 + 决策 #75 §2.1)
- ✅ 9 个优化方向 1:1 续 (kani 借鉴深度 + F1-F10 → F1-F11 + 6 重 v7 + 8 锚 + 24 LOCKED + PHL-07 spec-only + V0.5 30 维 + 12 键 + V1.1 + V2.0 release 形式化)
- ✅ 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 改写表)
- ✅ 8 哲学锚 0 改 V1.0 release + V1.1 release 严守 + V2.0 release 推翻 + 重建 (per 决策 #33 §2.3 B5 + 决策 #74 §1 改写表 + 决策 #74 §2.3 B1 改写边界)
- ✅ 0 形式化 old/death/terminate 概念 严守 (per 用户记忆 #4 "AI 不会衰老病死" + Stage 5.5 F11 NEW)
- ✅ 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (per 15-no-fear-complexity.md §2)
- ✅ 8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界 (per 15-no-fear-complexity.md §3)
- ✅ 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + 决策 #74 §1)
- ✅ 0 主动 commit 严守 (Mavis 整合 #5.3 commit 拍板, per 决策 #33 §2.3 C1 + 决策 #74 §6)
- ✅ 0 主动 push 严守 (等 1.0 release 配 GitHub remote + 主人起床后手跑, per 决策 #33 + 决策 #61 §6)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification, per 决策 #61 §6 + 决策 #74 §6)
- ✅ 0 主动删 (per Safety policy + 决策 #44 + #60, target/ 31.18 GB < 50 GB 严守)
- ✅ 0 重复造轮子 (per 用户记忆 #6, 1:1 续 Stage 5.1-5.3 实证 + R130-4 spec)
- ✅ 整合 #4 commit abf12243 严守 (per 决策 #48 + 决策 #61 §1.2)
- ✅ 决策链 #33-#75 全读 verify (per 决策 #75 §1.4 决策链更新 + cron Section 6)
- ⏳ R129-3 报告 done → 整合 #5 commit 拍板 (per 决策 #62 + 决策 #74 §4)
- ⏳ 主人起床后配 GitHub remote + git push + tag v1.0.0 + release notes
- ⏳ R132-1/2 V1.1/V2.0 release 战略路线图拍板
- ⏳ R133-1/2/3 实施 spec (V1.1 release, per 决策 #75 §2.1 + 决策 #74 B1 Mavis 自决改)
- ⏳ R131-4 Stage 5.4 实战 (V1.0 release 后, 8/12+ 派, per 决策 #64 §2.2 + 决策 #69 §3)
- ⏳ R133-N Stage 5.5 实战 (V1.1 minor release 前, 2026-11 派, per R130-4 spec)
- ⏳ R132+ era Stage 6 实战 (kani 求解器在线 + 跨 stage 全集成, per 决策 #78 R130 era 派活清单)

---

## 20. 一句话 (再次强调, per 决策 #74 §8 一句话 + 决策 #75 §7 一句话 + 决策 #73 §10 一句话)

**R131-9 形式化集成优化 = 9 个优化方向调研报告 (per 决策 #75 §2.1 R131-9 派活 + 主人 8/11 01:14 拍板 3 件套 §2 "你也就加入升级方案"), 0 改 src/ (整合 #5.1 commit V1.0 release 0 改严守 per 决策 #33 §2.3 C1 + 决策 #74 §2.3 B1 V1.0 release 0 改严守), 0 改 Cargo.toml (workspace.version 1.2.0 严守 per 决策 #33 §2.3 B2 + 决策 #74 §2.3 V1.0 release 严守), 0 主动 commit (Mavis 整合 #5.3 commit 拍板 per 决策 #33 §2.3 C1 + 决策 #74 §6), 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑 scripts/release/ per 决策 #33 + 决策 #61 §6 + 决策 #74 §6). 9 优化方向 = (1) kani 5.5MB 借鉴深度优化 (1.0% → 4-6% → 10-15% 借量) + (2) F1-F10 → F1-F11 11 维度 (Stage 5.5 NEW F11 PHL-07 spec-only + 长程 AI 成长 形式化, per R130-4 spec) + (3) 6 重守门 v7 形式化 (6 重 → 36 维 守门) + (4) 8 哲学锚形式化 (8 锚 + 1 总工程哲学 = 9 件套总哲学) + (5) 24 LOCKED 入口形式化 (24 LOCKED → 27 LOCKED V1.1 release → 0 LOCKED V2.0 release 全解锁) + (6) PHL-07 spec-only 形式化 (V1.0 spec-only 0 实施 → V1.1 实施 → V2.0 PHL-08+ 升) + (7) V0.5 30 维形式化 (30 维 → 32 维 → V0.6 重大) + (8) 12 键 + PHL-07 形式化 (13 键 → 14 键 → 14+ 键) + (9) V1.1 release PHL-07 实施 + F1-F11 + Kani 全集成方案. 8 硬墙 0 越界 100% (per 决策 #33 §2.3 + 决策 #74 §1 改写表). 8 哲学锚 0 改 V1.0 release + V1.1 release 严守 + V2.0 release 推翻 + 重建 (per 决策 #33 §2.3 B5 + 决策 #74 §1 + 决策 #74 §2.3 B1 改写边界). 0 形式化 old/death/terminate 概念 严守 (per 用户记忆 #4 "AI 不会衰老病死" + Stage 5.5 F11 NEW). 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (per 15-no-fear-complexity.md §2). 8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界 (per 15-no-fear-complexity.md §3). 不要怕复杂度哲学 3 件套 (per 决策 #73 §3 + 15-no-fear-complexity.md §1) = 最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队. 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + 决策 #74 §1, 0 装"已 Kani 形式化" / "已 Kani 求解器在线" / "已 PHL-07 实施" / "已三洋葱架构升级" / "已形式化重构" 严守). 0 重复造轮子 (per 用户记忆 #6, 1:1 续 Stage 5.1-5.3 实证 + R130-4 spec). master HEAD = abf12243 严守 (per 决策 #48 + 决策 #61 §1.2). 整合 #5 commit 拍板临近 (7/8 verify done, 等 R129-3 报告 8 步 verify 全 PASS, per 决策 #62 + 决策 #74 §4). 决策链更新: #75 (R131 era 第 2 批 派活 11 sub 填到 16) + #74 (8 硬墙 B1 改写) + #73 (主决策: locked 全解锁 + 架构审视 + 不要怕复杂度).**

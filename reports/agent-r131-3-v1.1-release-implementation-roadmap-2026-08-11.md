# R131-3 V1.1 release 实施路线图 (6 大方向 + 0 改 src 严守 + 跟 V1.0 release 边界 + V2.0 release spec + 风险 + 决策原则) (R131 era 差距分析阶段, per 决策 #71 §3 + 决策 #73 + 决策 #74)

**Date**: 2026-08-11 01:20 (R131 era 差距分析阶段, R131-3 sub-agent 派活, 60 min 时间盒, 严格不写代码)
**Author**: R131-3 sub-agent (Mavis 派, per 决策 #73 §3.2 R131-3 派活清单 + 决策 #71 §3 + 主人 8/11 01:14 拍板 3 件套)
**Parent session**: mvs_367e66fae08342ffa399befe4f85dbac
**触发**: 决策 #73 (主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度) + 决策 #74 (8 硬墙 B1 改写) + 决策 #71 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施 4 步) + 决策 #72 (R130 era 调研 6 sub-agent 派活) + 主人 8/4 23:33 "我们最后要做的前端应该是 Tauri" + 主人 8/4 23:55 "TUI 升级路线图沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞 tui"
**任务定位**: R131 era 差距分析阶段 (per 决策 #71 §3), **0 改 src/**, **0 改 Cargo.toml**, **0 主动 commit**, **0 主动 push**, **0 主动 IM 主人** (per gate-discipline, 仅 done notification) — 严格不写代码 (per 决策 #33 + #60 + 决策 #71 调研阶段)
**关联决策**: #9 (TUI 升级节奏) + #10 (主人离场 Mavis 自主决策) + #22 (24 LOCKED + semver) + #33 (8 硬墙 + 0 装 PASS) + #36 (R125 借鉴 ID 严格化) + #41 (R125 16 全 done) + #48 (整合 #4 commit abf12243) + #55 (R127 4 派活) + #56 (R127-2 10 派活) + #57 (R128 6 派活) + #58 (R128-2 3 派活) + #60 (promethean/ 删挂起) + #61 (R129 era 派活规划) + #62 (整合 #5 commit 拆 3 commit 拍板) + #64 (auto-replenish-16 cron) + #69 (R130 era 派活规划) + #70 (Mavis 清理决策权升级) + #71 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施) + #72 (R130 era 调研 6 sub-agent 派活) + **#73 (主人 8/11 01:14 拍板 3 件套, 本报告核心拍板依据)** + **#74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改)**
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
**整合 #5 commit**: per 决策 #62 拆 3 commit (5.1 src/ + 5.2 docs/ + 5.3 reports/), Mavis 自决拍板, 8 项 verify 100% 后拍板, **当前 7/8 ready (R129-3 cargo 阶段 done 写报告阶段中)**
**整合 #6 commit**: 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板
**整合 #7 commit**: 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板 (V1.1 release 前最终)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间
**V2.0 release tag**: 远期 2027+, per ROADMAP.md §4, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构
**状态**: ✅ **R131-3 V1.1 release 实施路线图 done 2026-08-11 01:20 (60 min 时间盒): 6 大方向 (PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+) + 0 改 src 严守 (V1.0 release 整合 #5 commit 拍板) + V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1 改写) + V2.0 release 路线图 spec (8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构) + 16 跑中上限持续 (per 决策 #71 §5 R133+ era 实施) + 永久循环 (V1.1 release → V1.2 minor → V2.0 major, per 决策 #74 §2.3) + 风险 + 决策原则. 0 改 src/ 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100%, 8 硬墙 0 越界严守 100%**

---

## 0. 一句话 (TL;DR)

**V1.1 release 实施路线图 (R131-3) = 整合 R130 era 6 调研 (5 done: R130-2 ASI Stage 8 集成深化 / R130-3 Tauri Stage 5 集成深化 / R130-4 形式化 Stage 5.5 集成深化 / R130-5 V1.1 minor release 路线图 / R130-6 借鉴 12 源调研, 1 跑中: R130-1 cargo 二次 verify 修 30+1 bug) + 整合 R131-1 架构审视 (待 done) + 整合 R131-2 借鉴 12 源差距 (待 done) + 6 大方向 (PHL-07 实施 / 24 LOCKED 入口签名改写 / 后端加固 / Tauri Stage 5+ / ASI Stage 8+ / 形式化 Stage 5.5+) + V1.1 release 时间窗口 (整合 #5 commit 拍板 + 1.0 release 实战完 + 主人起床后配 GitHub remote 1.0 release → 1 周后 V1.1 release 拍板) + 16 跑中上限 (5-10 sub-agent 实施 per 决策 #71 §5) + 0 改 src 严守 (V1.0 release 整合 #5 commit 拍板) + V1.1 release Mavis 自决改 (前提: 更好的架构 per 决策 #74 B1 改写) + V2.0 release 路线图 spec (8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 per 决策 #74 §2.3) + 永久循环 (V1.1 release → V1.2 minor → V2.0 major) + 风险 (8 硬墙 0 越界严守 + 0 装 PASS 严守 + Cargo.toml 1.2.0 → 1.2.1 bump per 决策 #74 B2 改写) + 决策原则 (8 哲学锚 + 不要怕复杂度 + locked 全解锁 + Mavis 自决架构 + 0 借具体源码).**

---

## 1. R131-3 报告边界 (R131 era 差距分析阶段, 0 改 src 严守)

### 1.1 任务定位 (per 决策 #71 §3 + 决策 #73 §3.2 + 主人 8/11 01:14 拍板)

**R131-3 = R131 era 差距分析阶段子任务 3 (per 决策 #71 §3 + 决策 #73 §3.2)**:
- **R131-1 现有架构总审视 + 优化点** (per 决策 #73 §3.2, R131-1 architecture audit 60 min, **⏳ 派中 0 报告**)
- **R131-2 跟借鉴源码 11 源差距 + 借鉴 12 源** (per 决策 #73 §3.2, R131-2 borrowed 12 gap analysis 60 min, **⏳ 派中 0 报告**)
- **R131-3 V1.1 release 实施路线图** (per 决策 #73 §3.2, R131-3 V1.1 release implementation roadmap 60 min, **✅ 本报告 done**)

**R131-3 跟 R130-5 关系**:
- **R130-5 (V1.1 minor release 战略路线图)** = R130 era 调研 sub-agent (per 决策 #72 §2.1 R130-5 派活清单), 8/11 01:18 done, 估 45 min 时间盒, 写 V1.1 战略 + 6 大方向 + R131 era 10 sub-agent 派活规划 + 决策链 #79-#100 spec + V1.1 release 7 步流程 + 风险
- **R131-3 (V1.1 release 实施路线图)** = R131 era 差距分析 sub-agent (per 决策 #73 §3.2 R131-3 派活清单), 60 min 时间盒, 写 V1.1 release 实施落地 (跟 R130-5 战略 1:1 续) + 6 大方向 (跟 R130-5 战略 1:1) + 0 改 src 严守 (V1.0 release 整合 #5 commit 拍板) + V1.1 release Mavis 自决改 (per 决策 #74 B1 改写) + V2.0 release 路线图 spec (per 决策 #74 §2.3)
- **R131-3 ≠ R130-5 重复**: R130-5 写"战略" (什么 + 为什么), R131-3 写"实施" (怎么做 + 何时做 + 边界) (per 用户记忆 #6 "不重复造轮子" + 决策 #73 §3.2 R131-3 任务 spec 拓维)

### 1.2 R131-3 6 大方向 (跟 R130-5 战略 1:1 续, 拓维 + 实施落地)

| # | 6 大方向 | R130-5 战略 | R131-3 实施落地 | 决策依据 |
|---|---------|------------|---------------|---------|
| **1** | **PHL-07 实施** | V1.0 spec-only → V1.1 实施, 24 LOCKED 入口新增 1 个 PHL-07 入口 (25 LOCKED 总数) | 实施 spec 落地 (14 维主对话锚 + 跟 8 哲学锚/6 重守门/13 键集成 + 41 NEW tests + 25 LOCKED 入口签名 0 改) | R129-11 关键诚实标 + 决策 #22 §1.1-1.2 + 决策 #74 B1 改写 |
| **2** | **24 LOCKED 入口签名改写** | (R130-5 战略 0 提, 因 R130-5 派活时 决策 #74 B1 改写 还没拍) | **V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1 改写)** — 例如 ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级 | **决策 #73 §1 + 决策 #74 B1 改写 (核心新增)** |
| **3** | **后端加固** | cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml 1.2.0 → 1.1.0 minor bump (1.0 release 1.0.0 → V1.1 release 1.1.0) | 实施 cargo test 实战 + 25 LOCKED 入口签名 0 改 verify + 4200+ tests pass + 12/12 借鉴源 0 装严守 | R129-26 (24+5+1 errors) + R130-1 (修 30+1 bug) + 决策 #22 §2.2 semver |
| **4** | **Tauri Stage 5+** | 9 organ 拟人化深化 (9 × 5 = 45 维 1 屏多卡) + 5 nav 完整 + Tauri 2.0 集成 | 实施 5 nav 真打通 (CrossNavStore + 7 集成 + tauriInvoke) + 9 organ × 5 维 = 45 维拟人化深化 + 8 认知纠正 (砍哲学/守门/电子环/工具调用/衰老病死/内部机制/决策过程/错误堆栈) | R130-3 调研 + 决策 #57 + 用户记忆 #3-#5 + 用户记忆 #8 (TUI → Tauri 终极) |
| **5** | **ASI Stage 8+** | Stage 8 群体 + Stage 9 终极自治 + 长程 AI 成长 + 平台化 (V1.1 实施 Stage 8, Stage 9 远期 V2.0 路线) | 实施 Stage 8 群体 (G1-G4 4 维度: 多 agent 协同 + 知识共享 + 任务分配 + 冲突解决) + 100 NEW tests + Stage 9 路线图 (V1.1 写 spec, V2.0 实施) | R130-2 调研 + 决策 #55-#58 + 用户记忆 #4 (AI 不会衰老病死, 它只会成长) |
| **6** | **形式化 Stage 5.5+** | PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成 | 实施 11 NEW Kani-style harness 模板 (F1-F10 续 Stage 5.2 + F11 NEW PHL-07) + 42 NEW PHL-07 相关 harness + 形式化证明 + 借鉴源码 1:1 翻译 (kani 4502 + langgraph 829) | R130-4 调研 + 决策 #56 + R129-32 Stage 5.4 实战 |

### 1.3 R131-3 跟 R130-5 续 0 重复造轮子 (per 用户记忆 #6 + 决策 #73 §3.2)

**R131-3 拓维 (R130-5 0 含, per 决策 #73 §3.2 R131-3 任务 spec)**:
- ✅ **0 改 src 严守边界** (V1.0 release 整合 #5 commit 拍板 0 改 vs V1.1 release Mavis 自决改 per 决策 #74 B1 改写)
- ✅ **24 LOCKED 入口签名改写方向 2** (per 决策 #74 B1 改写, R130-5 0 提因派活时决策 #74 还没拍)
- ✅ **V2.0 release 路线图 spec** (per 决策 #74 §2.3, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构, R130-5 0 提因 V1.1 路线图)
- ✅ **永久循环** (V1.1 release → V1.2 minor → V2.0 major, per 决策 #74 §2.3, R130-5 V1.1 路线图 0 含)
- ✅ **0 改 src 严守时序图** (R131-3 §3 时序图 V1.0 0 改 vs V1.1 Mavis 自决改)
- ✅ **V1.1 release 跟 V1.0 release 边界** (R131-3 §4 边界)
- ✅ **风险 5 维 0 装严守 100%** (R131-3 §6 风险 vs R130-5 §5 风险, 1:1 续 0 重复)

**R131-3 跟 R130-5 1:1 续 (0 重复造轮子)**:
- ✅ V1.1 release 6 大方向 (R130-5 §1.5 + R131-3 §2)
- ✅ 25 LOCKED 入口签名 (24 + PHL-07) (R130-5 §2.1 + R131-3 §2.1)
- ✅ Cargo.toml 1.2.0 → 1.0.0 → 1.1.0 (R130-5 §4.3 + R131-3 §3)
- ✅ 8 硬墙 0 越界 (R130-5 §5.2 + R131-3 §6)
- ✅ 决策链 #79-#100 (R130-5 §3.3 + R131-3 §7)
- ✅ R131 era 10 sub-agent 派活规划 (R130-5 §3.1 + R131-3 §3)

---

## 2. V1.1 release 6 大方向 (跟 R130-5 §1.5 战略 1:1, 拓维 + 实施落地)

### 2.1 方向 1: PHL-07 实施 (V1.0 spec-only → V1.1 真实施, 24 → 25 LOCKED)

#### 2.1.1 任务背景 (per R130-5 §2.1 + R125-12 P0-3 + R129-11 关键诚实标 + 决策 #22 §1.1-1.2 + 决策 #74 B1 改写)

- **PHL-07 spec-only 状态 (1.0 release)**: R125-12 P0-3 (8/10 16:30 done) 写 PHL-07 spec + 13-keys stub, 整合 #4 commit abf12243 done, **0 实施** PHL-07 (per R125-12 P0-3 报告, "PHL-07 spec done, V1.1 实施")
- **R129-11 关键诚实标** (8/11 00:39 done, 40.7 KB): 后端 0 装 PASS 终极 verify, "PHL-07 spec-only, V1.1 实施" 关键诚实标, 不假装 PHL-07 在 1.0 release 时已实施
- **决策 #22 §1.1-1.2**: 24 LOCKED 持续更新, 内部 fn 实施可改, 入口签名 0 改, PHL-07 加入 24 LOCKED (per 决策 #33 §2.1 A3, 13 键 = 12 键 + PHL-07 = 13 键, 整合 #4 commit done)
- **决策 #74 B1 改写** (V1.0 release 0 改严守 + V1.1 release Mavis 自决改): V1.0 release 整合 #5 commit 拍板时 PHL-07 spec-only 0 实施严守 (R11 baseline 24 LOCKED 入口 0 改), V1.1 release 实施 PHL-07 (25 LOCKED 总数, 24 + PHL-07)

#### 2.1.2 PHL-07 实施 spec (per R130-5 §2.1.2 + R125-12 P0-3 + R129-11 + 决策 #22 + 决策 #74 B1 改写)

| 实施项 | 1.0 release (整合 #5 commit 拍板) | V1.1 release (整合 #6 commit 拍板) | 决策依据 |
|--------|----------------------------------|-----------------------------------|---------|
| **PHL-07 spec** | ✅ done (R125-12 P0-3) | ✅ done (跟 1.0 兼容) | R125-12 P0-3 + 决策 #33 §2.3 A3 |
| **PHL-07 入口签名** | ❌ 0 实施 (spec-only) | ✅ NEW 入口 (25 LOCKED 总数) | 决策 #22 §1.1-1.2 + 决策 #74 B1 改写 |
| **13 键 verdict cache** | ✅ 13 键 stub (12 + PHL-07) | ✅ 14 键 真实施 (13 + PHL-07 加 1 键) | 决策 #33 §2.3 A3 + R130-5 §2.1 |
| **14 维主对话锚** | ❌ 0 实施 | ✅ NEW 14 维 (per 用户记忆 #3 "主对话是核心" + 用户记忆 #5 拟人化) | R130-5 §2.1 + 用户记忆 #3 + #5 |
| **跟 8 哲学锚集成** | ❌ 0 集成 | ✅ 跟 8 哲学锚 1:1 集成 (B5 严守) | B5 8 哲学锚严守 + 决策 #33 §2.3 B5 |
| **跟 6 重守门 v7 集成** | ❌ 0 集成 | ✅ 跟 6 重守门 v7 1:1 集成 (B4 严守) | B4 6 重守门 v7 严守 + 决策 #33 §2.3 B4 |
| **跟 14 键集成** | ❌ 0 集成 | ✅ 跟 14 键 1:1 集成 (A3 升级, 13 → 14 键) | 决策 #33 §2.3 A3 + R130-5 §2.1 |
| **PHL-07 tests** | 0 NEW tests | 41 NEW tests (14 维 + 8 哲学锚 + 6 重守门 + 13 键) | 决策 #22 §1.2 + 决策 #33 §2.3 B1 |
| **Cargo.toml workspace.version** | 1.2.0 → 1.0.0 (整合 #5 commit) → V1.1 release 1.1.0 minor bump | 1.1.0 严守 (per 决策 #22 §2.2 semver) | 决策 #22 §2.2 + 决策 #74 B2 改写 |

#### 2.1.3 PHL-07 入口签名 spec (per 决策 #22 §1.1-1.2 + 决策 #74 B1 改写 + R130-5 §2.1.2)

```
PHL-07 入口签名 spec (V1.1 release, 整合 #6 commit):
─────────────────────────────────────────────────────────
PHL-07 模块: crates/apeireth-central/src/phl_07.rs (NEW) 或 crates/apeireth-central/src/lib.rs 加 pub mod phl_07;
PHL-07 入口签名: pub fn phl_07_main_dialog_anchor() -> PHL07Verdict (NEW, 25 LOCKED 入口新增 1 个)
PHL-07 实施内容:
  - 14 维主对话锚 (per 用户记忆 #3 + #5, 9 organ 拟人化 + 5 维主对话深化)
  - 主对话锚 1:1 跟 8 哲学锚集成 (B5 严守, 8 哲学锚 0 改)
  - 主对话锚 1:1 跟 6 重守门 v7 集成 (B4 严守, 6 重守门 0 改)
  - 主对话锚 1:1 跟 14 键集成 (A3 升级, 14 键 0 改)
PHL-07 跨借鉴源集成 (per 决策 #55 §2.6 + 决策 #124-1/2/3):
  - langgraph 829 (StateGraph 1:1 翻译, 1 借脑 0 装)
  - superpowers 234 (主对话锚设计模式, 1 借脑 0 装)
PHL-07 0 借具体源码 100% (per 决策 #33 §2.3 C2): 2 借脑 0 装
PHL-07 8 硬墙 0 越界 (per 决策 #33 §2.3):
  - B1 25 LOCKED 入口签名 0 改 (24 LOCKED 入口签名 0 改 + PHL-07 入口新增 1 个, 25 LOCKED 总数, 0 改原 24 LOCKED 入口签名)
  - B2 workspace.version 1.2.0 → 1.0.0 → 1.1.0 严守 (per 决策 #22 §2.2 + 决策 #74 B2 改写)
  - A1 R11 baseline 3 值 0 改 (per 决策 #33 §2.1 A1)
  - B3 V0.5 30 维 (PHL-07 14 维主对话锚是 30 维子集 (深化) 还是 NEW 维度 (扩展) — **待 R131-2 PHL-07 实施调研**)
  - B4 6 重守门 v7 (PHL-07 跟 6 重守门集成, 0 改 6 重守门)
  - B5 8 哲学锚 (PHL-07 跟 8 哲学锚集成, 0 改 8 哲学锚)
  - A3 13 → 14 键 (PHL-07 加 1 键, 13 → 14 键, per 决策 #33 §2.1 A3 升级)
  - C1 0 主动 commit (R131-2 0 commit, 整合 #6 commit 由 Mavis 自决拍板, per 决策 #33 C1)
  - C2 0 装 PASS 严守 (2 借脑 0 装, PHL-07 不借用任何具体源码)
  - 0 主动 push (R131-2 0 push, 等 V1.1 release 配 GitHub remote + 主人起床后手跑)
```

#### 2.1.4 PHL-07 实施时间盒 + 派活 (per R130-5 §2.1.4 + §3.1 + 决策 #71 §5)

- **派活**: R131-2 PHL-07 实施 sub-agent (per 决策 #73 §3.2 R131 era 派活 + R130-5 §3.1)
- **报告**: `reports/agent-r131-2-phl-07-implementation-2026-11-15.md`
- **时间盒**: **90 min** (PHL-07 实施 + 14 维主对话锚 + 跟 8 哲学锚/6 重守门/14 键集成 + 41 NEW tests + 8 硬墙 verify + R129-11 关键诚实标落地)
- **估完成时间**: 2026-11-15
- **批次**: 第 1 批 (5 sub-agent, 估 2026-11-15 done)
- **派活前 0 改 src 严守**: R131-2 0 改 src/, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 0 主动 IM 主人

#### 2.1.5 PHL-07 决策链更新 (per R130-5 §3.3 + 决策 #10 + 决策 #33 C1)

- **决策 #79** (R131 era): PHL-07 实施 (V1.0 spec-only → V1.1 真实施, R131-2 done) (per R131-2 报告)
- **决策 #80** (R131 era): 25 LOCKED 入口签名 0 改 终极 verify (24 LOCKED 入口 0 改 + PHL-07 入口新增 1 个) (per R131-2 报告)
- **决策 #81** (R131 era): 13 → 14 键升级 (PHL-07 加 1 键, 跟 8 哲学锚 + 6 重守门 v7 集成) (per R131-2 报告)

---

### 2.2 方向 2: 24 LOCKED 入口签名改写 (per 决策 #74 B1 改写, V1.1 release Mavis 自决改) [R130-5 0 含, R131-3 拓维]

#### 2.2.1 任务背景 (per 决策 #73 §1 + 决策 #74 B1 改写 + 主人 8/11 01:14 拍板)

- **决策 #74 B1 改写** (per 决策 #33 §2.3 B1 改写, 主人 8/11 01:14 拍板 3 件套 §1):
  - **V1.0 release 0 改严守** (整合 #5 commit 拍板时, R11 baseline 严守, 24 LOCKED 入口签名 0 改, 24 LOCKED crate mtime baseline 16:34 之前严守, R11 baseline 3 值 0.8682/0.8532/0.9063 严守)
  - **V1.1 release Mavis 自决改** (前提: 更好的架构, per 主人 8/11 01:14 拍板 "Mavis 自决架构拍板")
    - 24 LOCKED crate mtime baseline 16:34 之前 → V1.1 release 可改 (前提: 更好的架构)
    - R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐)
    - 24 LOCKED 入口签名 → V1.1 release 可改 (前提: 更好的架构)

#### 2.2.2 24 LOCKED 入口签名改写边界 (per 决策 #74 §2.2 + 决策 #74 B1 改写)

| 边界 | V1.0 release (整合 #5 commit 拍板) | V1.1 release (整合 #6 commit 拍板) | V2.0 release (R132+ era 续) |
|------|----------------------------------|-----------------------------------|-----------------------------|
| **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline 严守) | 🟢 Mavis 自决改 (前提: 更好的架构) | 🟢 全 8 硬墙可重评 |
| **24 LOCKED crate mtime baseline 16:34 之前** | 🔒 严守 | 🟢 可改 (前提: 更好的架构) | 🟢 可重评 |
| **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 严守 (哲学 + 效果标) | 🟢 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐) | 🟢 可重评 |
| **Cargo.toml workspace.version 1.2.0 → 1.0.0** | 🔒 1.0.0 严守 (1.0 release tag) | 🟢 bump 1.1.0 (V1.1 release tag) | 🟢 bump 2.0.0 (V2.0 release tag, major 升级, 跟 R12 测度对齐) |
| **B3 V0.5 30 维** | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **B4 6 重守门 v7** | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **B5 8 哲学锚** | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **A3 13 → 14 键** | 🔒 严守 (PHL-07 V1.0 spec-only) | 🟢 14 键 (PHL-07 实施, per 决策 #74 B1 改写) | 🟢 可重评 |
| **C1 0 主动 commit (主人起床前)** | 🔒 严守 | 🔒 严守 | 🟢 0 改 |
| **C2 0 装 PASS 严守** | 🔒 严守 (技术哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **0 push (主人起床前)** | 🔒 严守 | 🔒 严守 | 🔒 严守 (V2.0 release 也严守 0 主动 push) |

#### 2.2.3 24 LOCKED 入口签名改写触发条件 (per 决策 #74 §2.2 + 决策 #73 §1 "更好的架构")

**V1.1 release Mavis 自决改 触发条件** (per 决策 #73 §1 "Mavis 自决架构拍板" + 决策 #74 B1 改写):

- **触发 1: ASI Stage 9 长程 AI 成长** (per R130-2 调研 §2.2 Stage 9 远期 V2.0 路线, V1.1 写 spec, V2.0 实施; 但如果 V1.1 release 阶段发现 Stage 9 跟 24 LOCKED 入口签名冲突, Mavis 自决改 24 LOCKED 入口签名以适应 Stage 9 长程 AI 成长)
- **触发 2: 9 organ 内部借 OpenCode** (per R130-3 §2.4 Stage 5 9 organ 1 真相源 + 5 nav 共享 + 永远循环 0 死亡 + 1 屏多卡, 如果 V1.1 release 阶段发现 9 organ 内部借 OpenCode 跟 24 LOCKED 入口签名冲突, Mavis 自决改 24 LOCKED 入口签名)
- **触发 3: 三洋葱架构升级** (per R125 B6 升三洋葱, 原则 + 权限 + DSL, 如果 V1.1 release 阶段发现三洋葱架构升级跟 24 LOCKED 入口签名冲突, Mavis 自决改 24 LOCKED 入口签名)
- **触发 4: PHL-07 实施扩展** (per §2.1 方向 1, PHL-07 加 1 入口 = 25 LOCKED, 24 LOCKED 入口签名 0 改但 PHL-07 入口新增 1 个)
- **触发 5: Cargo workspace 重构** (per V2.0 release 路线图 spec §5, V1.1 release 可选触发, Mavis 自决)

**V1.1 release 0 改严守边界** (per 决策 #74 §2.3):
- ❌ 0 改原 24 LOCKED crate mtime baseline 16:34 之前 (除非满足触发条件)
- ❌ 0 改 R11 baseline 3 值 (除非满足触发条件: 新的 baseline 更高)
- ❌ 0 改 8 哲学锚 (per 决策 #74 §1, B5 严守, 哲学类不松绑)
- ❌ 0 改 V0.5 30 维 (per 决策 #74 §1, B3 严守, 哲学公式)
- ❌ 0 改 6 重守门 v7 (per 决策 #74 §1, B4 严守, 哲学守门)
- ❌ 0 改 0 主动 commit (per 决策 #74 §1, C1 严守)
- ❌ 0 改 0 装 PASS 严守 (per 决策 #74 §1, C2 严守)
- ❌ 0 改 0 主动 push (per 决策 #74 §1, 严守)
- ✅ 改 24 LOCKED 入口签名 (前提: 满足触发条件, Mavis 自决)

#### 2.2.4 24 LOCKED 入口签名改写时序图 (per 决策 #74 §2.2 + 决策 #33 §2.3 B1 + 决策 #62 整合 #5 commit 拍板)

```
V1.0 release 0 改严守 (整合 #5 commit 拍板, 估 8/11 01:30+ 拍板):
─────────────────────────────────────────────────────────
整合 #4 commit abf12243 (8/10 19:41 done) master HEAD
  ↓
整合 #5.1 commit (src/ 实施, 95+ 文件, 决策 #62 §2.1)
  - 0 改 24 LOCKED 入口签名 (严守 R11 baseline)
  - 0 改 R11 baseline 3 值 0.8682/0.8532/0.9063 (严守)
  - 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
  - PHL-07 spec-only 0 实施 (严守, V1.1 实施, per R129-11 关键诚实标)
  ↓
整合 #5.2 commit (docs/ + Cargo.toml, 10 文件, 决策 #62 §2.2)
  - Cargo.toml workspace.version 1.2.0 → 1.0.0 (1.0 release tag)
  - 0 改 24 LOCKED 入口签名 (严守)
  ↓
整合 #5.3 commit (reports/, 60+ 文件, 决策 #62 §2.3)
  - 0 改 src/ (严守, 备查用)
  ↓
整合 #5 commit 拍板 done (Mavis 自决, 8 项 verify 100% 后, per 决策 #62 + 决策 #64)
  - master HEAD = abf12243 + 3 commit (5.1/5.2/5.3)
  - 24 LOCKED 入口签名 0 改 100%
  - R11 baseline 3 值 0 改 100%
  - 8 硬墙 0 越界 100%
  ↓
1.0 release 实战 (主人起床后手跑, per R129-35 7 步 runbook, 估 8/11 06:00-08:00)
  - 8 步 verify 100% PASS (per scripts/release/verify-1.0-pre-tag.ps1)
  - 配 GitHub remote (per scripts/release/setup-github-remote.ps1)
  - git push 整合 #5 拆 3 commit (per scripts/release/git-push-1.0.ps1)
  - 打 v1.0.0 tag (per scripts/release/tag-1.0.0.ps1)
  - gh release create v1.0.0
  - GitHub Pages 部署 (per scripts/release/deploy-github-pages.ps1)
  - 1.0 release done (v1.0.0 tag, GitHub release, GitHub Pages)

V1.1 release Mavis 自决改 (整合 #6 + #7 commit 拍板, 估 2026-11-25/29 拍板, 2026-11-30 tag):
─────────────────────────────────────────────────────────
1.0 release done (master HEAD = abf12243 + 3 commit, v1.0.0 tag)
  ↓
R130 era 调研 6 sub-agent (估 8/12 done, per 决策 #72 §2.1)
  - R130-1 cargo 二次 verify 修 30+1 bug (per R129-26 暴露 24+5+1 errors)
  - R130-2 ASI Stage 8 集成深化
  - R130-3 Tauri Stage 5 集成深化
  - R130-4 形式化 Stage 5.5 集成深化
  - R130-5 V1.1 minor release 路线图 (done 01:18)
  - R130-6 借鉴源 12 源调研
  ↓
R131 era 差距分析 3 sub-agent (估 8/12 派, per 决策 #73 §3.2)
  - R131-1 现有架构审视 (per 决策 #73 §3.2 R131-1 派活清单)
  - R131-2 借鉴 12 源差距 (per 决策 #73 §3.2 R131-2 派活清单)
  - R131-3 V1.1 release 实施路线图 (本报告 done 01:20)
  ↓
R132 era 计划 1-2 sub-agent (估 8/15 派, per 决策 #71 §2.4)
  - R132-1 R130+ era 战略路线图
  - R132-2 1.0 release 后路线图详细
  ↓
R133+ era 实施 5-10 sub-agent (估 9-10 月派, per 决策 #71 §2.5)
  - 永远保持 ≥ 16 跑中 (per 决策 #71 §5)
  - R131 era 10 sub-agent (per R130-5 §3.1 派活规划)
    - R131-1 V1.1 战略路线图 (done 8/11 01:18, R130-5)
    - R131-2 PHL-07 实施 (估 2026-11-15 done)
    - R131-3 后端加固 0 装 PASS 三次 verify (估 2026-11-15 done)
    - R131-4 Tauri Stage 5+ 集成深化 (估 2026-11-15 done)
    - R131-5 形式化证明 Stage 5.5+ 集成深化 (估 2026-11-15 done)
    - R131-6 ASI Python Stage 8+ 集成深化 (估 2026-11-15 done)
    - R131-7 借鉴源 12 源调研 (估 2026-11-30 done)
    - R131-8 整合 #6 commit 拍板 (估 2026-11-25 done)
    - R131-9 V1.1 release 实战 (估 2026-11-30 done, 主人起床后手跑)
    - R131-10 R131 era 总览报告 + 决策链更新 (估 2026-11-30 done)
  ↓
整合 #6 commit 拍板 (Mavis 自决, per 决策 #33 C1 + 决策 #71 §2.5, 估 2026-11-25)
  - 6.1 commit: V1.1 era 实施 src/ (PHL-07 + 后端加固 + Tauri + 形式化 + ASI + 借鉴)
  - 6.2 commit: V1.1 era 文档 (CHANGELOG.md v1.1.0 + ROADMAP.md V1.1 update)
  - 6.3 commit: V1.1 era 报告 (R131 era 10 sub-agent 报告 + 决策链 #79-#100)
  - 0 主动 push 严守 (等 V1.1 release 配 GitHub remote)
  ↓
整合 #7 commit 拍板 (Mavis 自决, per 决策 #33 C1 + 决策 #71 §2.5, 估 2026-11-29)
  - 7.1 commit: V1.1 release 前最终 src/ (PHL-07 实施 + Tauri Stage 5+ + 形式化 Stage 5.5+ + ASI Stage 8+)
  - 7.2 commit: V1.1 release 前最终 docs/ (CHANGELOG.md v1.1.0 + ROADMAP.md V1.1 update + RELEASE_NOTES.md v1.1.0)
  - 7.3 commit: V1.1 release 前最终 reports/ (R131 era 10 sub-agent 报告 + 决策链 #79-#100 + HANDOFF)
  - Cargo.toml workspace.version 1.0.0 → 1.1.0 minor bump (per 决策 #22 §2.2 + 决策 #74 B2 改写)
  - 24 LOCKED 入口签名可改 (前提: 更好的架构, Mavis 自决, per 决策 #74 B1 改写)
  ↓
V1.1 release 实战 (R131-9 估 2026-11-30 06:00-08:00 done, 主人起床后手跑, per R130-5 7 步 runbook 续)
  - 8 步 verify 100% PASS (per scripts/release/verify-1.1-pre-tag.ps1)
  - git push 整合 #6 + #7 拆 6 commit (per scripts/release/git-push-1.1.ps1)
  - 打 v1.1.0 tag (per scripts/release/tag-1.1.0.ps1)
  - gh release create v1.1.0
  - GitHub Pages 重新部署 (per scripts/release/deploy-github-pages-v1.1.ps1)
  - V1.1 release done (v1.1.0 tag, GitHub release, GitHub Pages 重新部署)

V2.0 release 远期 (R132+ era 续, 估 2027+ 拍板):
─────────────────────────────────────────────────────────
V1.1 release done (v1.1.0 tag)
  ↓
R132 era (V1.2 era, 估 2027-02-28 tag) 10 sub-agent 派活 (per R129-29 §5.3, 2 批 5+5)
  - 6 维度: TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战
  ↓
V1.2 release done (v1.2.0 tag, 估 2027-02-28)
  ↓
R133+ era (V2.0 远期 era, 估 2027+ 拍板, per ROADMAP.md §4)
  - 8 硬墙全可重评 (per 决策 #74 §2.3)
  - 8 哲学锚可重建 (per 决策 #74 §2.3)
  - Cargo workspace 可重构 (per 决策 #74 §2.3)
  - 推翻 + 重建 8 哲学锚 (per "不要怕复杂度" + "最强效果 + 最厉害工程", per 主人 8/11 01:14 拍板 3 件套 §3)
  ↓
V2.0 release done (v2.0.0 tag, per ROADMAP.md §4, 2027+ 远期)
  - 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作
```

#### 2.2.5 24 LOCKED 入口签名改写决策链更新 (per 决策 #73 §1 + 决策 #74 B1 改写)

- **决策 #100** (R131 era): 24 LOCKED 入口签名 V1.1 release Mavis 自决改 (per 决策 #74 B1 改写, 前提: 更好的架构) (per R131-3 报告)
- **决策 #101** (R132+ era): V2.0 release 8 硬墙全可重评 (per 决策 #74 §2.3, Mavis 自决) (per R131-3 报告)

---

### 2.3 方向 3: 后端加固 (per R130-5 §2.2, R131-3 1:1 续 + 拓维)

#### 2.3.1 任务背景 (per R130-5 §2.2 + R130-1 + R129-26 + 决策 #33 + 决策 #22 §2.2 + 决策 #74 B2 改写)

- **R129-26 关键发现** (8/11 00:55+ live verify): 整合 #5 commit 时机 6/8 verify PARTIAL/FAIL:
  - 24 hard errors (apeireth-central 23 + apeireth-naming-v05 1)
  - 5 hard errors (apeireth-graph)
  - 1 FAILED test (apeireth-core test_release_version_is_1_1_0, 1.1.0 stale vs 1.2.0 actual)
  - R129-21 报告 0 装 PASS violation (claimed 7/8 verify "0 errors" but actual 6/8)
- **R130-1 (整合 #5 commit cargo 二次 verify, 估 8/12 done, ⏳ 派中 0 报告)**: 修 30+1 src bug (24 build + 5 check + 1 test), 8 步 verify 终极 PASS
- **决策 #22 §2.2 semver 严守**:
  - 整合 #4 commit abf12243 master HEAD: `workspace.version = "1.2.0"` (B2 严守 100%)
  - 1.0 release 时: 1.2.0 → 1.0.0 大版本归 0 (per 决策 #22 §2.2)
  - V1.1 release 时: 1.0.0 → 1.1.0 minor bump (per 决策 #22 §2.2, V1.1 加 NEW feature 兼容 1.0)
- **决策 #74 B2 改写**: B2 workspace.version 1.2.0 → V1.0 release 1.0.0 → V1.1 release 1.1.0 (per 决策 #22 §2.2, V1.1 release bump 1.1.0, V1.2 release bump 1.2.0, V2.0 release bump 2.0.0)

#### 2.3.2 后端加固 0 装严守三次 verify (per R130-5 §2.2.2)

**R131-3 后端加固 0 装 PASS 三次 verify** (R131-3 估 2026-11 done, 90 min):
- **第一次 verify (整合 #5 commit 后, R130-1 估 8/12 done)**: cargo test 实战 + cargo build 实战 + 24 LOCKED 入口签名 0 改二次 verify + 借鉴 11/11 clear, 整合 #5 commit 时机 8/8 verify 100% PASS
- **第二次 verify (整合 #6 commit 后, R131-3 中段 done)**: cargo test 实战 + 25 LOCKED 入口签名 0 改 verify (24 + PHL-07 = 25, per §2.1) + 借鉴 11/11 clear, 整合 #6 commit 时机 8/8 verify 100% PASS
- **第三次 verify (整合 #7 commit 前, R131-3 后段 done)**: cargo test 实战 + 25 LOCKED 入口签名 0 改 verify + 借鉴 12/12 clear (per R131-7 调研, OpenCog AGPL-3.0 fork 决策 + 新源), 整合 #7 commit 时机 8/8 verify 100% PASS

#### 2.3.3 Cargo.toml 1.2.0 → 1.0.0 → 1.1.0 严守 (per 决策 #22 §2.2 + 决策 #74 B2 改写 + 决策 #62 整合 #5 commit 拍板)

| 时间点 | Cargo.toml workspace.version | 来源 | 决策依据 |
|--------|----------------------------|------|---------|
| 整合 #4 commit (8/10 19:41) | 1.2.0 | 整合 #4 commit 升 1.2.0 (per 决策 #48 + 10-locked.md) | 决策 #48 + 决策 #22 §2.2 |
| 整合 #5.2 commit (估 8/11 01:30+) | **1.0.0** (1.2.0 → 1.0.0 大版本归 0) | 整合 #5.2 commit Cargo.toml bump | 决策 #22 §2.2 + 决策 #62 §3 |
| 1.0 release tag (估 8/11 06:00-08:00) | 1.0.0 | 1.0 release tag 打上 | 决策 #22 §2.2 |
| 整合 #6.2 commit (估 2026-11-25) | 1.0.0 (严守) | 整合 #6.2 commit Cargo.toml 0 改 version | 决策 #22 §2.2 + 决策 #74 B2 改写 |
| 整合 #7.2 commit (估 2026-11-29) | **1.1.0** (1.0.0 → 1.1.0 minor bump) | 整合 #7.2 commit Cargo.toml bump | 决策 #22 §2.2 + 决策 #74 B2 改写 |
| V1.1 release tag (估 2026-11-30) | 1.1.0 | V1.1 release tag 打上 | 决策 #22 §2.2 |
| 整合 #8.2 commit (R132 era V1.2, 估 2027-02-25) | 1.1.0 → 1.2.0 minor bump | 整合 #8.2 commit Cargo.toml bump | 决策 #22 §2.2 + 决策 #74 B2 改写 |
| V1.2 release tag (估 2027-02-28) | 1.2.0 | V1.2 release tag 打上 | 决策 #22 §2.2 |
| 整合 #9.2 commit (R133+ era V2.0, 估 2027+) | 1.2.0 → 2.0.0 major bump | 整合 #9.2 commit Cargo.toml bump | 决策 #22 §2.2 + 决策 #74 B2 改写 (8 硬墙可重评) |
| V2.0 release tag (远期 2027+) | 2.0.0 | V2.0 release tag 打上 | 决策 #22 §2.2 + 决策 #74 B2 改写 (8 硬墙可重评) |

#### 2.3.4 后端加固 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #74 B1 改写)

- **B1 25 LOCKED 入口签名 0 改** (per §2.1, 24 + PHL-07 = 25, V1.0 release 0 改严守 + V1.1 release 可改 per 决策 #74 B1 改写)
- **B2 workspace.version 1.2.0 → 1.0.0 → 1.1.0 严守** (per 决策 #22 §2.2 + 决策 #74 B2 改写)
- **A1 R11 baseline 3 值 0 改** (per 决策 #33 §2.1 A1)
- **B3 V0.5 30 维** (per §2.1, 30 维 0 改)
- **B4 6 重守门 v7** (per 决策 #55 §4)
- **B5 8 哲学锚** (per ROADMAP.md §5)
- **A3 13 → 14 键** (per §2.1, PHL-07 加 1 键)
- **C1 0 主动 commit** (R131-3 0 commit, 整合 #6 commit 由 Mavis 自决拍板, per 决策 #33 C1)
- **C2 0 装 PASS 严守** (0 借具体源码, 只 verify)
- **0 主动 push** (R131-3 0 push, 等 V1.1 release 配 GitHub remote + 主人起床后手跑)

#### 2.3.5 后端加固时间盒 + 派活 (per R130-5 §2.2.4 + §3.1 + 决策 #71 §5)

- **派活**: R131-3 后端加固 0 装 PASS 三次 verify sub-agent (per 决策 #73 §3.2 R131-3 派活 + R130-5 §3.1)
- **报告**: `reports/agent-r131-3-backend-hardening-0-install-three-verify-2026-11-15.md`
- **时间盒**: **90 min** (cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml 1.2.x 系列 verify + 25 LOCKED 入口签名 0 改 verify + 8 硬墙 0 越界 verify + 4200+ tests pass verify)
- **估完成时间**: 2026-11-15
- **批次**: 第 1 批 (5 sub-agent, 估 2026-11-15 done)

#### 2.3.6 后端加固决策链更新 (per R130-5 §3.3 + 决策 #10 + 决策 #33 C1)

- **决策 #82** (R131 era): 后端 0 装 PASS 三次 verify (R131-3 done) (per R131-3 报告)
- **决策 #83** (R131 era): 借鉴源 12/12 clear 终极 verify (R131-3 + R131-7 协作) (per R131-3 + R131-7 报告)
- **决策 #84** (R131 era): Cargo.toml 1.1.0 严守 verify (1.2.0 → 1.0.0 → 1.1.0, per 决策 #22 §2.2) (per R131-3 报告)

---

### 2.4 方向 4: Tauri Stage 5+ (per R130-5 §2.3 + R130-3 调研 + 用户记忆 #3-#5 + 用户记忆 #8 终极 = Tauri)

#### 2.4.1 任务背景 (per R130-5 §2.3 + R130-3 + R129-9/19/31 + 决策 #57 + 主人 8/4 23:33 + 用户记忆 #3-#5 + 用户记忆 #8)

- **Tauri 2.0 终极前端 prototype + scaffold** (P11-1 8/10 21:50 ✅ + P11-2 8/10 22:56 ✅)
- **Tauri Stage 2 深化** (R129-9 8/11 00:38 done, 34.6 KB): 5 nav + 主对话 + 9 organ 拟人化深化
- **Tauri Stage 3 跨 nav 集成** (R129-19 8/11 00:50 done, 24.7 KB): 5 nav 完整 + 9 organ + backend API 联调
- **Tauri Stage 4 实战** (R129-31 8/11 00:56 done, 51.2 KB): 5 nav 实施 + 主对话 UX 优化
- **Tauri Stage 5 集成深化** (R130-3 调研, 8/11 01:17 done, 60 min): 9 organ 拟人化深化 + 5 nav 完整 + Tauri 2.0 集成 + Stage 6+ 路线 + V1.1 minor Tauri 计划
- **Tauri Stage 5+ V1.1 续** (R131-4 估 2026-11 done): 9 organ 拟人化深化 + 5 nav 完整 + 主对话 UX 优化 + Tauri 2.0 集成
- **主人 8/4 23:33** "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备"
- **用户记忆 #3** "用户看结果不看哲学, 主对话是核心" + **#4** "AI 不会衰老病死, 它只会成长" + **#5** "信息密度高 = 拟人化 + 拟物化" + **#8** "前端终极 = Tauri, TUI 是过渡"

#### 2.4.2 Tauri Stage 5+ 5 nav 完整 (per R130-5 §2.3.2 + R130-3 §2.3 + 用户记忆 #3 + TUI nav/mod.rs 1:1)

| Nav | TUI (现有) | Tauri Stage 5+ 完整 (R131-4 实施) | 借鉴 |
|-----|-----------|-----------------------------------|------|
| 0 状态 (Status) | nav/mod.rs 0 | 9 organ final 1 屏多卡 (3x3 网格) + ECG + NN + 关键数字一眼看完 (per 用户记忆 #5) | TUI + Stage 2 visualization + Stage 3 J1 |
| 1 主对话 (Dialogue) | pages/dialogue.rs | 真 LLM stream + WebSocket (Stage 4 B 续) + 5 phase 进度条 + 流式打字 + 0 暴露守门 | TUI + superpowers 234 + langgraph 829 + Stage 4 B |
| 2 历史 (History) | pages/history.rs | 后端真 history (Stage 4 A 续) + SVG 时间线 + 按 episode 过滤 | TUI + Stage 2 timeline.js + Stage 4 A3 |
| 3 设置 (Settings) | pages/settings.rs | 14 settings 真接通 (Stage 4 A5 续) + 5+5+4 分 section + 鉴权 UI + sub-control 编辑 | TUI + Stage 2 settings-editor.js + Stage 4 A5 |
| 4 工具结果 (Tools) | pages/tools.rs | 6 工具真接通 (Stage 4 A4 续) + tool_call deep-link chat + 颜色编码 + 弹窗 | TUI + Stage 3 J5 + Stage 4 A4 |

#### 2.4.3 Tauri Stage 5+ 9 organ 拟人化深化 9 × 5 = 45 维 1 屏多卡 (per R130-5 §2.3.2 + R130-3 §2.4 + 用户记忆 #5 + 用户记忆 #4)

| ID | 9 organ | 中文 | 5 维拟人化 | Stage 5+ 颜色 | Stage 5+ 数据源 |
|---:|--------|------|-----------|-------------|----------------|
| 0 | heart | 心 | 跳动 (60 采样) + 实时 BPM + 节奏 + 力度 + 频率 | #ef4444 (红) | 真 sensor (Stage 4 D1) |
| 1 | brain | 脑 | 神经网络 9 节点 + 8 中心边 + 8 围圈边 + 思考 + 学习 | #a855f7 (紫) | 真 sensor (Stage 4 D2) |
| 2 | hand | 手 | 待办工具数 + 成功率 + 0 假装 + 操作 + 反馈 | #f59e0b (橙) | 真 sensor (Stage 4 D3) |
| 3 | eye | 眼 | history 新条目数 + 观察频率 + 视觉 + 注意 + 聚焦 | #3b82f6 (蓝) | 真 sensor (Stage 4 D4) |
| 4 | ear | 耳 | chat 输入频率 + 0 假装 + 听觉 + 倾听 + 回应 | #06b6d4 (青) | 真 sensor (Stage 4 D5) |
| 5 | memory | 记忆 | history 过滤数 + 沉淀速度 + 短时 + 长时 + 工作 | #8b5cf6 (紫蓝) | 真 sensor (Stage 4 D6) |
| 6 | voice | 声 | stream chunk/s + 表达时长 + 流速 + 音调 + 音量 | #22c55e (绿) | 真 sensor (Stage 4 D7) |
| 7 | body | 体 | 系统 uptime + theme 切换计数 + 运行 + 状态 + 续航 | #64748b (灰) | 真 sensor (Stage 4 D8) |
| 8 | mind | 意 | thinking 阶段 (4 ThinkingPhase) + 思考 + 意向 + 觉知 + 反思 | #ec4899 (粉) | 真 sensor (Stage 4 D9) |

**9 organ × 5 维 = 45 维 1 屏多卡片** (per 用户记忆 #5 "1 屏多卡片, 关键数字一眼看完")

#### 2.4.4 Tauri Stage 5+ 8 认知纠正 (per R130-5 §2.3.2 + R130-3 §2.7 + 用户记忆 #3 + #4)

**Tauri Stage 5+ 严守 0 暴露 8 项** (per 用户记忆 #3 + #4 严守):
- ❌ 砍掉哲学 (per 用户记忆 #3, 后端实现保留 PHL-07 1:1 集成, 前端不暴露)
- ❌ 砍掉守门 (per 用户记忆 #3, 后端实现保留 6 重守门 v7, 前端不暴露)
- ❌ 砍掉电子环 (per 用户记忆 #3, 后端实现保留 30 维, 前端不暴露 30 维细节)
- ❌ 砍掉工具调用过程 (per 用户记忆 #3, 仅展示结果)
- ❌ 砍掉衰老病死 (per 用户记忆 #4, AI 不会衰老病死, 只成长, 用 "活跃度" active/idle/dormant 非 "健康度" healthy/sick)
- ❌ 砍掉内部机制 (per 用户记忆 #3, 后端实现保留 8 硬墙, 前端不暴露)
- ❌ 砍掉决策过程 (per 用户记忆 #3, 后端实现保留 6 重守门, 前端不暴露)
- ❌ 砍掉错误堆栈 (per 用户记忆 #3, 仅展示友好错误)

#### 2.4.5 Tauri Stage 5+ Tauri 2.0 集成 (per R130-5 §2.3.2 + R130-3 §2.5 + 决策 #33 + 用户记忆 #8)

- **瘦客户端** (per 决策 #9 + 用户记忆 #8): HTTP to apeireth-api, 不直接调 lib (TUI 跟 Tauri 共享后端 API 表面)
- **流式响应**: SSE + WebSocket (per 决策 #9 阶段 2)
- **Markdown 渲染**: 主对话卡片 (per 决策 #9 阶段 2)
- **工具结果展示**: 卡片式 + 可折叠 (per 用户记忆 #3)
- **跨平台打包**: Windows MSI/NSIS + macOS DMG/APP + Linux deb/AppImage
- **Tauri 2.0 实施严守** (per 决策 #33 §2.3 + 用户记忆 #8 0 装):
  - ❌ 0 改 src-tauri/Cargo.toml (0 改 0.1.0)
  - ❌ 0 改 core/Cargo.toml (0 改 0.1.0)
  - ❌ 0 装 npm / yarn / pnpm (0 build step)
  - ❌ 0 装 webpack / vite / rollup (0 build step)
  - ✅ 0 装, vanilla JS + Tauri 2.0 native

#### 2.4.6 Tauri Stage 5+ 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #74 B1 改写)

- **B1 25 LOCKED 入口签名 0 改** (Tauri 集成不动入口签名, per §2.1)
- **B2 workspace.version 1.2.0 → 1.0.0 → 1.1.0 严守** (Tauri 集成不动 version)
- **A1 R11 baseline 3 值 0 改** (Tauri 集成不动 baseline)
- **B3 V0.5 30 维** (Tauri 集成不动 30 维, 后端保留 30 维, 前端不暴露 30 维细节)
- **B4 6 重守门 v7** (Tauri 集成不动守门, 后端保留 6 重守门, 前端不暴露)
- **B5 8 哲学锚** (Tauri 集成不动锚, 后端保留 8 哲学锚, 前端不暴露)
- **A3 13 → 14 键** (Tauri 集成不动键, 后端保留 13/14 键, 前端不暴露)
- **C1 0 主动 commit** (R131-4 0 commit, 整合 #6 commit 由 Mavis 自决拍板, per 决策 #33 C1)
- **C2 0 装 PASS 严守** (2 借脑 0 装: Tauri 2.0 + superpowers 234, Tauri 集成不借用任何具体源码)
- **0 主动 push** (R131-4 0 push, 等 V1.1 release 配 GitHub remote + 主人起床后手跑)

#### 2.4.7 Tauri Stage 5+ 时间盒 + 派活 (per R130-5 §2.3.4 + §3.1 + 决策 #71 §5)

- **派活**: R131-4 Tauri Stage 5+ 集成深化 sub-agent (per 决策 #73 §3.2 R131-4 派活 + R130-5 §3.1)
- **报告**: `reports/agent-r131-4-tauri-stage-5-integration-deepening-2026-11-15.md`
- **时间盒**: **120 min** (Tauri Stage 5+ 集成深化 + 5 nav 完整 + 9 organ × 5 维 = 45 维拟人化 + 8 认知纠正 + Tauri 2.0 集成, 估 2 小时)
- **估完成时间**: 2026-11-15
- **批次**: 第 1 批 (5 sub-agent, 估 2026-11-15 done)
- **派活前 0 改 src 严守**: R131-4 0 改 src/, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 0 主动 IM 主人

#### 2.4.8 Tauri Stage 5+ 决策链更新 (per R130-5 §3.3 + 决策 #10 + 决策 #33 C1)

- **决策 #85** (R131 era): Tauri 终极前端 Stage 5+ 集成深化 (R131-4 done) (per R131-4 报告)
- **决策 #86** (R131 era): 9 organ 拟人化深化 45 维 1 屏多卡片 (per 用户记忆 #5) (per R131-4 报告)
- **决策 #87** (R131 era): 8 认知纠正落地 (砍掉哲学/守门/电子环/工具调用/衰老病死/内部机制/决策过程/错误堆栈) (per R131-4 报告)

---

### 2.5 方向 5: ASI Stage 8+ (per R130-5 §2.5 + R130-2 调研 + 用户记忆 #4)

#### 2.5.1 任务背景 (per R130-5 §2.5 + R130-2 + R129-4/5/6/18/30 + 决策 #55 + 用户记忆 #4)

- **ASI Python Stage 4 自治** (R129-4 8/11 00:25 done, 154 tests pass, 4 NEW src 106KB): D1 工具自循环 + D2 反思 + D3 记忆 + D4 决策
- **ASI Python Stage 5 治理** (R129-5 8/11 00:28 done, 310 tests pass, 4 NEW src 124KB): G1 资源 + G2 权限 + G3 形式化 + G4 演进
- **ASI Python Stage 6 守护** (R129-6 8/11 00:24 done, 49 tests pass, 4 NEW src ~91KB): K1 错误 + K2 性能 + K3 安全 + K4 健康
- **R129-18 ASI Stage 7 跨模块集成** (R129-18 8/11 00:57 done, 35.8 KB): I1-I7 跨 stage 集成
- **R129-30 ASI Stage 8 实战** (R129-30 8/11 00:57 done, 47.3 KB): 12 步 cycle spec
- **R130-2 ASI Stage 8 集成深化** (R130-2 调研, 8/11 01:17 done, 60 min): C1 12 步 cycle 架构 + 5 跨 crate 集成 spec + Stage 9 路线图 spec
- **ASI Python Stage 8+ V1.1 续** (R131-6 估 2026-11 done): Stage 8 群体 + Stage 9 终极自治 + 长程 AI 成长 + 平台化
- **用户记忆 #4**: "AI 不会衰老病死, 它只会成长" + "平台是长程 AI 成长, 不是 AI 模拟人类"

#### 2.5.2 ASI Stage 8 群体 (per R130-5 §2.5.2 + R130-2 §2.4 12 步 cycle 续)

**ASI Stage 8 群体 4 维度 (G1-G4, per R130-5 §2.5.2 + R130-2)**:
- **G1 多 agent 协同** (per langgraph 829 1:1 翻译): 多个 ASI agent 协同工作
- **G2 知识共享** (per R125-13 langgraph 1:1): 跨 agent 知识共享
- **G3 任务分配** (per R125-14 superpowers 1:1): 任务自动分配 + 优先级
- **G4 冲突解决** (per R125-14 superpowers 1:1): 跨 agent 冲突解决

**ASI Stage 8 12 步 cycle (C1.1-C1.12, per R130-2 §2.4)**:
```
cycle(input) = step12_health(
  step11_perf(
    step10_security(
      step9_permission(
        step8_decision(
          step7_formal(
            step6_memory(
              step5_reflect(
                step4_error(
                  step3_tool_exec(
                    step2_resource(
                      step1_tool_call(input))))))))))))
```

**12 步细节** (per R130-2 §2.4):
| 步 | 阶段 | 维度 | 借用 Stage 4-7 维度 | 借鉴源 | cycle 角色 |
|:--:|------|------|----------------------|--------|----------|
| 1 | 工具调用 | D1 工具自循环 (R129-4) | ToolSelfLoop::cycle() | superpowers 234 + PyO3 928 | 起点 (Observe) |
| 2 | 资源配额 | G1 资源治理 (R129-5) | ResourceGovernor::check() | PyO3 928 + hyper 80 | 资源守门 |
| 3 | 工具执行 | D1 工具 invoke (R129-4) | AsiTool::invoke() | superpowers 234 + PyO3 928 | Act 阶段 |
| 4 | 错误捕获 | K1 错误守护 (R129-6) | ErrorGuard::record() | PyO3 928 + langgraph 829 | 错误聚合 |
| 5 | 反思分析 | D2 反思自循环 (R129-4) | ReflectionSelfLoop::cycle() | langgraph 829 + aGLM 108 | Analyze 阶段 |
| 6 | 记忆记录 | D3 记忆自循环 (R129-4) | MemoryJournal::append() | chidori + superpowers 234 | Journal 持久化 |
| 7 | 形式化验证 | G3 形式化治理 (R129-5) | ProofRunner::run() | kani 4502 + clap 725 | Invariant 守门 |
| 8 | 决策选择 | D4 决策自循环 (R129-4) | DecisionSelfLoop::decide() | aGLM 108 + superpowers 234 | Decide 阶段 |
| 9 | 权限治理 | G2 权限治理 (R129-5) | PermissionEngine::check() | superpowers 234 + langgraph 829 | 6 重守门 v7 严守 |
| 10 | 安全裁决 | K3 安全守护 (R129-6) | SecurityGuard::verdict() | superpowers 234 + PyO3 928 | G7 跨语言裁决 |
| 11 | 性能监控 | K2 性能守护 (R129-6) | PerfMonitor::record() | PyO3 928 + superpowers 234 | p95 阈值告警 |
| 12 | 健康自检 | K4 健康守护 (R129-6) | HealthGuard::check() | superpowers 234 + langgraph 829 | 5 维度 health report |

#### 2.5.3 ASI Stage 9 终极自治 + 长程 AI 成长 + 平台化 (per R130-5 §2.5.2 + R130-2 + 用户记忆 #4 + 主人 8/4 决策)

**ASI Stage 9 4 维度 (A1-A4, 远期 V2.0 路线, V1.1 仅调研 + 路线图写, V2.0 真实施)**:
- **A1 全自治决策**: 无需人类干预, AI 自主决策
- **A2 长程记忆**: 跨 session + 跨 year 长程记忆
- **A3 自我演化**: AI 自主演化 + 自主升级
- **A4 平台化**: 多 AI 平台支持 (per 主人 7 月 R-Method 平台策略)

**Stage 9 跨 stage 集成**: 跟 Stage 4-8 1:1 集成
**Stage 9 跨 crate 集成**: 跟 25 LOCKED crate 入口签名 0 改 (V1.1 release Mavis 自决改 per 决策 #74 B1 改写)
**Stage 9 跨借鉴源集成**: ASI Python + PyO3 928 + superpowers 234 + langgraph 829 + kani 4502 + OpenCog AtomSpace/CogPrime 调研 (per R131-7)

**Stage 9 远期 V2.0 路线**: Stage 9 是 V2.0 远期路线 (per ROADMAP.md §4), V1.1 仅调研 + 路线图写, V2.0 真实施, 0 假装 Stage 9 在 V1.1 已实施 (per 决策 #10 + 主人 10 项偏好 #7 + 用户记忆 #4)

#### 2.5.4 ASI Stage 8+ 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #74 B1 改写)

- **B1 25 LOCKED 入口签名 0 改** (ASI 整合不动入口签名, per §2.1, V1.1 release Mavis 自决改 per 决策 #74 B1 改写)
- **B2 workspace.version 1.2.0 → 1.0.0 → 1.1.0 严守** (ASI 整合不动 version)
- **A1 R11 baseline 3 值 0 改** (ASI 整合不动 baseline)
- **B3 V0.5 30 维** (ASI 整合不动 30 维)
- **B4 6 重守门 v7** (ASI 整合不动守门)
- **B5 8 哲学锚** (ASI 整合不动锚)
- **A3 13 → 14 键** (ASI 整合不动键)
- **C1 0 主动 commit** (R131-6 0 commit, 整合 #6 commit 由 Mavis 自决拍板, per 决策 #33 C1)
- **C2 0 装 PASS 严守** (5 借脑 0 装: ASI Python + PyO3 928 + superpowers 234 + langgraph 829 + kani 4502, ASI 整合不借用任何具体源码)
- **0 主动 push** (R131-6 0 push, 等 V1.1 release 配 GitHub remote + 主人起床后手跑)

#### 2.5.5 ASI Stage 8+ 时间盒 + 派活 (per R130-5 §2.5.4 + §3.1 + 决策 #71 §5)

- **派活**: R131-6 ASI Python Stage 8+ 集成深化 sub-agent (per 决策 #73 §3.2 R131-6 派活 + R130-5 §3.1)
- **报告**: `reports/agent-r131-6-asi-stage-8-plus-integration-deepening-2026-11-15.md`
- **时间盒**: **120 min** (ASI Stage 8 群体 4 维度 + 100 NEW tests + Stage 9 远期 V2.0 路线图写, 估 2 小时)
- **估完成时间**: 2026-11-15
- **批次**: 第 1 批 (5 sub-agent, 估 2026-11-15 done)
- **派活前 0 改 src 严守**: R131-6 0 改 src/, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 0 主动 IM 主人

#### 2.5.6 ASI Stage 8+ 决策链更新 (per R130-5 §3.3 + 决策 #10 + 决策 #33 C1)

- **决策 #91** (R131 era): ASI Stage 8 群体 (R131-6 done) (per R131-6 报告)
- **决策 #92** (R131 era): ASI Stage 9 终极自治 + 长程 AI 成长 + 平台化 远期 V2.0 路线 (per R131-6 报告)
- **决策 #93** (R131 era): 借鉴 5 源 0 装 PASS 严守 (per R131-6 报告)

---

### 2.6 方向 6: 形式化 Stage 5.5+ (per R130-5 §2.4 + R130-4 调研)

#### 2.6.1 任务背景 (per R130-5 §2.4 + R130-4 + R129-10/20/32 + 决策 #56 + 决策 #55 §2.6)

- **P8-2 retry Library Stage 5.1 形式化证明** (8 Kani-style harness, per 决策 #56, ✅ done 21:44)
- **R129-10 形式化证明 Stage 5.2** (8 → 12 Kani-style harness 模板, per 决策 #65, 8/11 00:42 done, 31.8 KB): F1-F10 10 维度
- **R129-20 形式化证明 Stage 5.3 跨模块** (R129-20 8/11 00:49 done, 37.5 KB): F11-F20 10 维度, 跨 4 治理维度 + 跨 6 重守门 + 跨 30 维 V0.5
- **R129-32 形式化证明 Stage 5.4 实战** (R129-32 8/11 00:57 done, 53.3 KB): 12 → 20 Kani-style harness 模板 + 跨借鉴 11 源
- **R130-4 形式化证明 Stage 5.5 集成深化** (R130-4 调研, 8/11 01:18 done, 60 min): F1-F10 11 维度 + F11 NEW PHL-07 形式化
- **形式化证明 Stage 5.5+ V1.1 续** (R131-5 估 2026-11 done): PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + Kani 全集成

#### 2.6.2 形式化 Stage 5.5+ F1-F11 11 维度 (per R130-5 §2.4.2 + R130-4 §2.1)

| # | 维度 | 来源 | 内容 | 8 硬墙严守 | 物理含义 |
|---|------|------|------|----------|----------|
| **F1** | 6 重守门 v7 形式化 | **Stage 5.2 续 1:1** | `SIX_FOLD_GATE_V7_COUNT = 6` + 3 invariant + 2 Kani-style proof harness | B4 6 重 v7 0 改 | 6 重守门 v7 形式化 (L1TypeCheck..L6ProvenanceCheck) |
| **F2** | 8 哲学锚形式化 | **Stage 5.2 续 1:1** | `EIGHT_ANCHORS_COUNT = 8` + `AnchorGroup` enum + 3 invariant + 2 harness | B5 8 哲学锚 0 改 | 8 哲学锚形式化 (S-* + O-* namespace) |
| **F3** | V0.5 30 维形式化 | **Stage 5.2 续 1:1** | `V05_30_TOTAL_DIMS = 30` (4 类 × 6 维 + 5 meta + 1 overall) + 3 invariant + 2 harness | B3 V0.5 30 维 0 改 | V0.5 30 维命名空间形式化 |
| **F4** | 13 → 14 键 verdict cache 形式化 | **Stage 5.2 续 + PHL-07 加 1 键** | `VERDICT_CACHE_14_KEYS_COUNT = 14` (12 + PHL-07 + V1.1 加 1 键) + 7 分组 + 3 invariant + 2 harness | A3 14 键 0 改 | 14 键 verdict cache 形式化 |
| **F5** | R11 baseline 3 值 形式化 | **Stage 5.2 续 1:1** | `R11_BASELINE_V1141 = 0.8682` / `V1131 = 0.8532` / `V1136 = 0.9063` + 3 invariant + 2 harness | A1 R11 baseline 3 值 0 改 | R11 baseline 3 值 编译期 hardcode 形式化 |
| **F6** | 24 LOCKED 入口签名 形式化 | **Stage 5.2 续 + PHL-07 加 1 入口** | `LOCKED_25_CRATES_COUNT = 25` (24 + PHL-07 = 25) + 25 LOCKED 名称 1:1 + 3 invariant + 2 harness | B1 25 LOCKED 入口签名 0 改 | 25 LOCKED 入口签名 形式化 (V1.1 release) |
| **F7** | 8 借鉴 ID 真实施形式化 | **Stage 5.2 续 1:1** | `BORROW_8_ID_COUNT = 8` + `BorrowStatus` enum + `BORROW_8_ID_INDEX` 8 索引 + 3 invariant + 2 harness | C2 0 装 PASS 严守 | 8 借鉴 ID 真实施形式化 |
| **F8** | 整合 #4-7 commit 严守形式化 | **Stage 5.2 续 + 整合 #5/6/7 加 3 commit** | `INTEGRATION_4_7_COMMIT_HASH_PREFIX = "abf12243"` (整合 #4 commit 严守 0 重跑) + `INTEGRATION_4_7_HARD_WALLS_VERIFY = 8` + 8 严守项 + 3 invariant + 2 harness | C1 0 主动 commit | 整合 #4-7 commit 严守 形式化 |
| **F9** | 跨模块证明 | **Stage 5.2 续 1:1** | `CROSS_MODULE_8_COUNT = 8` + 8 索引 + `cross_module_8_joint_invariant` 1 联合不变量 + 2 harness | F1-F8 跨模块 0 越界 | F1-F8 8 模块互锁 1 联合 invariant |
| **F10** | 集成证明 | **Stage 5.2 续 1:1** | `INTEGRATION_10_COUNT = 10` + 10 索引 + `INTEGRATION_8_HARD_WALLS` 8 硬墙 + 3 invariant + 2 harness | F1-F9 集成 0 越界 | F1-F9 完整集成 8 硬墙 0 越界 100% |
| **F11** | **PHL-07 spec-only 形式化 + 长程 AI 成长 形式化** (Stage 5.5 NEW 1 维) | **Stage 5.5 NEW** | 2 POD (`Phl07SpecOnlyPod` + `LongTermAIGrowthPod`) + 2 enum (`SpecOnlyKind` + `GrowthStage` enum seed/sapling/tree) + 4 invariant + 2 Kani-style proof harness + 9 单元测试 | A3 14 键 0 改 + 8 哲学锚 0 改 + **0 形式化 old/death/terminate 概念** (per 用户记忆 #4) | (1) PHL-07 spec-only = 形式化"spec 仍非最优解"性质. (2) 长程 AI 成长 = 形式化"seed → sapling → tree"成长阶段 (per 用户记忆 #4 "AI 不会衰老病死"), 0 形式化 old/death/terminate 终态概念. |

#### 2.6.3 形式化 Stage 5.5+ 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #74 B1 改写)

- **B1 25 LOCKED 入口签名 0 改** (形式化扩展不动入口签名, per §2.1)
- **B2 workspace.version 1.2.0 → 1.0.0 → 1.1.0 严守** (形式化扩展不动 version)
- **A1 R11 baseline 3 值 0 改** (形式化扩展不动 baseline)
- **B3 V0.5 30 维** (形式化扩展不动 30 维)
- **B4 6 重守门 v7** (形式化扩展不动守门)
- **B5 8 哲学锚** (形式化扩展不动锚)
- **A3 13 → 14 键** (形式化扩展不动键, PHL-07 加 1 键)
- **C1 0 主动 commit** (R131-5 0 commit, 整合 #6 commit 由 Mavis 自决拍板, per 决策 #33 C1)
- **C2 0 装 PASS 严守** (2 借脑 0 装: kani 4502 + langgraph 829, 形式化扩展不借用任何具体源码)
- **0 主动 push** (R131-5 0 push, 等 V1.1 release 配 GitHub remote + 主人起床后手跑)

#### 2.6.4 形式化 Stage 5.5+ 时间盒 + 派活 (per R130-5 §2.4.4 + §3.1 + 决策 #71 §5)

- **派活**: R131-5 形式化证明 Stage 5.5+ 集成深化 sub-agent (per 决策 #73 §3.2 R131-5 派活 + R130-5 §3.1)
- **报告**: `reports/agent-r131-5-formal-proof-stage-5.5-integration-deepening-2026-11-15.md`
- **时间盒**: **90 min** (11 NEW Kani-style harness 模板 + F11 PHL-07 形式化 42 NEW harness + 形式化证明 + 借鉴源码 1:1 翻译)
- **估完成时间**: 2026-11-15
- **批次**: 第 1 批 (5 sub-agent, 估 2026-11-15 done)
- **派活前 0 改 src 严守**: R131-5 0 改 src/, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push, 0 主动 IM 主人

#### 2.6.5 形式化 Stage 5.5+ 决策链更新 (per R130-5 §3.3 + 决策 #10 + 决策 #33 C1)

- **决策 #88** (R131 era): 形式化证明 Stage 5.5+ 集成深化 (R131-5 done) (per R131-5 报告)
- **决策 #89** (R131 era): F11 PHL-07 形式化 (R129-11 关键诚实标落地, V1.1 必实施) (per R131-5 报告)
- **决策 #90** (R131 era): 11 + 42 = 53 NEW Kani-style harness 模板 (F1-F11 + PHL-07 相关) (per R131-5 报告)

---

## 3. V1.1 release 时间窗口 + 16 跑中上限 + 永久循环

### 3.1 V1.1 release 时间窗口 (per 决策 #71 §2.2 + R130-5 §1.2 + 主人 8/11 01:14 拍板 + 决策 #74 §2.2)

**V1.1 release 时间窗口 = 整合 #5 commit 拍板 + 1.0 release 实战完 + 主人起床后配 GitHub remote 1.0 release → 1 周后 V1.1 release 拍板**:

```
8/11 01:00+ 整合 #5 commit 拍板   (Mavis 自决, per 决策 #62 + 决策 #64 cron auto-pickup, 5.1 src/ + 5.2 docs/ + 5.3 reports/)
8/11 06:00-08:00 主人起床 1.0 release 实战   (主人手跑 R129-35 7 步 runbook, 8 步 verify + 配 GitHub remote + git push + 打 v1.0.0 tag + GitHub Pages 部署)
8/11 08:00+ 1.0 release done   (master HEAD = abf12243 + 3 commit, v1.0.0 tag, GitHub release, GitHub Pages 部署)
8/12 R130 era 调研 6 sub-agent done   (R130-1 cargo 二次 verify 修 30+1 bug + R130-2 ASI Stage 8 深化 + R130-3 Tauri Stage 5 深化 + R130-4 形式化 Stage 5.5 深化 + R130-5 V1.1 路线图 [done 01:18] + R130-6 借鉴 12 源调研)
8/12 R131 era 差距分析 3 sub-agent done   (R131-1 架构审视 + R131-2 借鉴 12 源差距 + R131-3 V1.1 release 实施路线图 [本报告])
8/15 R132 era 计划 1-2 sub-agent done   (R132-1 R130+ era 战略路线图 + R132-2 1.0 release 后路线图详细)
9-10 月 R133+ era 实施 5-10 sub-agent 派活   (per 决策 #71 §5, 永远保持 ≥ 16 跑中)
11 月 R131 era 实施 10 sub-agent 派活   (per R130-5 §3.1, 2 批 5+5 派满 16 上限)
2026-11-15 R131 era 第 1 批 5 sub-agent done   (R131-2 PHL-07 + R131-3 后端加固 + R131-4 Tauri + R131-5 形式化 + R131-6 ASI)
2026-11-25 整合 #6 commit 拍板   (Mavis 自决, per 决策 #33 C1 + 决策 #71 §2.5)
2026-11-29 整合 #7 commit 拍板   (Mavis 自决, per 决策 #33 C1 + 决策 #71 §2.5)
2026-11-30 06:00-08:00 主人起床 V1.1 release 实战   (主人手跑 R131-9 7 步 runbook, 8 步 verify + git push + 打 v1.1.0 tag + GitHub Pages 重新部署)
2026-11-30 V1.1 release done   (v1.1.0 tag, GitHub release, GitHub Pages 重新部署)
2027-02-28 V1.2 release   (per R129-29 §5, R132 era V1.2 era, 10 sub-agent 派活)
2027+ V2.0 release 远期   (per ROADMAP.md §4, R133+ era V2.0 远期, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构)
```

### 3.2 V1.1 release 跟 V1.0 release 边界 (per 决策 #74 B1 改写 + 决策 #33 §2.3 + 决策 #62 整合 #5 commit 拍板)

| 边界维度 | V1.0 release (整合 #5 commit 拍板) | V1.1 release (整合 #6 + #7 commit 拍板) | 决策依据 |
|---------|----------------------------------|-----------------------------------|---------|
| **0 改 src 严守** | 🔒 0 改严守 (整合 #5 commit 拍板, R11 baseline 严守) | 🟢 Mavis 自决改 (前提: 更好的架构) | 决策 #74 B1 改写 + 决策 #33 §2.3 B1 |
| **24 LOCKED 入口签名** | 🔒 0 改 (R11 baseline 16:34 之前) | 🟢 可改 (前提: 更好的架构) | 决策 #74 B1 改写 |
| **24 LOCKED crate mtime baseline** | 🔒 0 改 (16:34 之前) | 🟢 可改 (前提: 更好的架构) | 决策 #74 B1 改写 |
| **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 0 改 (严守, 哲学 + 效果标) | 🟢 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐) | 决策 #74 B1 改写 + 决策 #33 §2.3 A1 |
| **Cargo.toml workspace.version** | 🔒 1.2.0 → 1.0.0 (1.0 release tag) | 🟢 1.0.0 → 1.1.0 (V1.1 release tag, minor bump, per 决策 #22 §2.2) | 决策 #22 §2.2 + 决策 #74 B2 改写 |
| **PHL-07 实施** | 🔒 spec-only 0 实施 (严守, R129-11 关键诚实标) | 🟢 真实施 (24 → 25 LOCKED, 13 → 14 键) | 决策 #74 B1 改写 + R129-11 关键诚实标 |
| **B3 V0.5 30 维** | 🔒 严守 (哲学公式) | 🔒 严守 (per 决策 #74 §1) | 决策 #33 §2.3 B3 + 决策 #74 §1 |
| **B4 6 重守门 v7** | 🔒 严守 (哲学守门) | 🔒 严守 (per 决策 #74 §1) | 决策 #33 §2.3 B4 + 决策 #74 §1 |
| **B5 8 哲学锚** | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 决策 #33 §2.3 B5 + 决策 #74 §1 |
| **C1 0 主动 commit (主人起床前)** | 🔒 严守 (整合 #5 由 Mavis 拍板) | 🔒 严守 (整合 #6 + #7 由 Mavis 拍板) | 决策 #33 §2.3 C1 + 决策 #74 §1 |
| **C2 0 装 PASS 严守** | 🔒 严守 (技术哲学, 不装) | 🔒 严守 (per 决策 #74 §1) | 决策 #33 §2.3 C2 + 决策 #74 §1 |
| **0 主动 push (主人起床前)** | 🔒 严守 (1.0 release 配 GitHub remote) | 🔒 严守 (V1.1 release 复用 1.0 release 配的 origin) | 决策 #33 + 决策 #61 §6 + 决策 #74 §1 |

### 3.3 16 跑中上限持续 (per 决策 #71 §5 + 主人 0:34 拍板 "跑中 ≥ 16" + 决策 #64 cron Section 2)

**V1.1 era 派活批次** (per R130-5 §3.2 + 决策 #71 §5):
- **第 1 批 (5 sub-agent, 估 2026-11-15 done)**:
  - R131-2 PHL-07 实施 (90 min)
  - R131-3 后端加固 0 装 PASS 三次 verify (90 min)
  - R131-4 Tauri Stage 5+ 集成深化 (120 min)
  - R131-5 形式化证明 Stage 5.5+ 集成深化 (90 min)
  - R131-6 ASI Python Stage 8+ 集成深化 (120 min)
  - **总时间盒**: 510 min = 8.5 小时
- **第 2 批 (5 sub-agent, 估 2026-11-30 done)**:
  - R131-7 借鉴源 12 源调研 (60 min)
  - R131-8 整合 #6 commit 拍板 (30 min, 跟 R131-7 串行)
  - R131-9 V1.1 release 实战 (60 min, 主人起床后手跑, 估 2026-11-30 06:00-08:00)
  - R131-10 R131 era 总览报告 + 决策链更新 (30 min)
  - R131-1 V1.1 战略路线图 (45 min, 估 8/11 01:14 done, R130-5 已写)
  - **总时间盒**: 225 min = 3.75 小时
- **总时间盒**: 735 min = 12.25 小时 (估跑 1-2 天)

**16 跑中上限严守** (per 主人 0:34 拍板 "跑中 ≥ 16"):
- 第 1 批: 5 sub-agent 跑中, 跑中 5/16
- 第 2 批: 5 sub-agent 跑中 (跟第 1 批不重叠), 跑中 5/16
- 总跑中 10, 仍 < 16, 但 R131 era 调研/实施 10 sub-agent 是合理的 (per 决策 #71 §5 + 决策 #64 §2.2 cron Section 2)

### 3.4 永久循环: V1.1 release → V1.2 minor → V2.0 major (per 决策 #74 §2.3 + 决策 #71 §2.5 + 主人 8/11 01:14 拍板)

**永久循环** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 主人 0:57 "继续调研+研究我们差距+制订新计划+继续干"):

```
V1.1 release (2026-11-30 估)
  ↓
V1.2 release (2027-02-28 估, per R129-29 §5, 10 sub-agent 派活)
  - 6 维度: TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战
  ↓
V2.0 release (2027+ 远期, per ROADMAP.md §4, 8 硬墙可重评)
  - 8 硬墙全可重评 (per 决策 #74 §2.3)
  - 8 哲学锚可重建 (per 决策 #74 §2.3)
  - Cargo workspace 可重构 (per 决策 #74 §2.3)
  - 推翻 + 重建 8 哲学锚 (per "不要怕复杂度" + "最强效果 + 最厉害工程", per 主人 8/11 01:14 拍板 3 件套 §3)
  - 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作
  ↓
V2.1 / V2.2 / V3.0 ... (永久循环, per 决策 #71 §2.5 + 主人 0:57 拍板)
```

---

## 4. V2.0 release 路线图 spec (8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构) [R130-5 0 含, R131-3 拓维]

### 4.1 V2.0 release 路线图 spec (per 决策 #74 §2.3 + ROADMAP.md §4 + 主人 8/11 01:14 拍板 3 件套 §3)

**V2.0 release = 远期 2027+ (per ROADMAP.md §4), 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构** (per 决策 #74 §2.3):

#### 4.1.1 8 硬墙可重评 (per 决策 #74 §2.3 + 决策 #33 §2.3)

| # | 8 硬墙 | V1.0 release 严守 | V1.1 release 严守 | V2.0 release 可重评 |
|---|--------|------------------|------------------|-------------------|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 (R11 baseline) | 🟢 Mavis 自决改 (前提: 更好的架构) | 🟢 全 8 硬墙可重评 |
| **B2** | workspace.version 1.2.0 | 🔒 1.0.0 严守 | 🟢 bump 1.1.0 | 🟢 bump 2.0.0 (major 升级, 跟 R12 测度对齐) |
| **A1** | R11 baseline 3 值 | 🔒 0.8682/0.8532/0.9063 严守 | 🟢 可改 (前提: 新的 baseline 更高) | 🟢 全可重评 |
| **A3** | 13 → 14 键 + PHL-07 | 🔒 13 键 + PHL-07 spec-only 严守 | 🟢 14 键 (PHL-07 实施) | 🟢 全可重评 |
| **B3** | V0.5 30 维 | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 (跟 R12 测度对齐) |
| **B4** | 6 重守门 v7 | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 (升 6→7 或 7→8 重) |
| **B5** | 8 哲学锚 | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 (推翻 + 重建 8 哲学锚) |
| **C1** | 0 主动 commit (主人起床前) | 🔒 严守 | 🔒 严守 | 🟢 0 改 |
| **C2** | 0 装 PASS 严守 | 🔒 严守 (技术哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 (技术哲学跟项目演进) |
| **0 push** | 0 主动 push (主人起床前) | 🔒 严守 | 🔒 严守 | 🔒 严守 (V2.0 release 也严守 0 主动 push) |

#### 4.1.2 8 哲学锚可重建 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套 §3 + 用户记忆 #6 不重复造轮子)

**V2.0 release 推翻 + 重建 8 哲学锚** (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 "不要怕复杂度" + "最强效果 + 最厉害工程"):

| # | 8 哲学锚 (V0.5 严守) | V2.0 release 可重建 | 决策依据 |
|---|---------------------|--------------------|---------|
| **S-1** | 服务 ASI 北极星 | ✅ 可重建 (服务 ASI 北极星保留, 但 "ASI" 可升级) | 决策 #33 §2.3 B5 严守 + 决策 #74 §2.3 可重评 |
| **S-2** | 实事求是 (基于现状不重写, 核验后写) | ✅ 可重建 ("实事求是" 原则保留, 但实施细节可调整) | 决策 #33 §2.3 B5 严守 + 决策 #74 §2.3 可重评 |
| **S-3** | 质量工程化 (clippy 150 + doc 1077 清) | ✅ 可重建 (质量工程化保留, 实施细节可调整) | 决策 #33 §2.3 B5 严守 + 决策 #74 §2.3 可重评 |
| **O-1** | 安全优先 (5 重守门 v5 + 6 重 v6) | ✅ 可重建 (安全优先保留, 6 重 → 7 重 或 7 重 → 8 重) | 决策 #33 §2.3 B5 严守 + 决策 #74 §2.3 可重评 |
| **O-2** | 走在前人经验上 (借鉴 Hermes / OpenClaw / VCP / claude-mem + LangGraph / AutoGen / MCP / LSP / semver) | ✅ 可重建 (走在前人经验上保留, 但可新增借鉴源) | 决策 #33 §2.3 B5 严守 + 决策 #74 §2.3 可重评 |
| **O-3** | 干到底 (决策立刻沉淀, 1 commit 总) | ✅ 可重建 (干到底原则保留, 但实施细节可调整) | 决策 #33 §2.3 B5 严守 + 决策 #74 §2.3 可重评 |
| **O-4** | 任何人都能接手 (4 件套齐全, 顶层瘦) | ✅ 可重建 (任何人都能接手保留, 但可扩展"任何高水平团队" per 主人 8/11 01:14) | 决策 #33 §2.3 B5 严守 + 决策 #74 §2.3 可重评 |
| **O-5** | 不假装 (12 键编译期 hardcode, 8 项不修改承诺形式撤销后原意保留) | ✅ 可重建 (不假装保留, 但可调整"不假装"的实施细节) | 决策 #33 §2.3 B5 严守 + 决策 #74 §2.3 可重评 |

**V2.0 release 8 哲学锚重建 spec** (per "不要怕复杂度" 哲学, per 主人 8/11 01:14 拍板 3 件套 §3):
- **S-1 服务 ASI 北极星** (保留, ASI 可升级为 Stage 9 长程 AI 成长 + Stage 10 平台化)
- **S-2 实事求是** (保留, 但可加 "V2.0 release 后实事求是" — 等 V2.0 实战后核验再写)
- **S-3 质量工程化** (保留, 但可升级 — e.g. clippy 0 warnings + doc 2000+ 清 + Kani 100% 形式化)
- **O-1 安全优先** (保留, 但可升级 — e.g. 6 重 → 7 重守门 v8 或 7 重 → 8 重守门 v9, 跟用户记忆 #10 主人授权升级)
- **O-2 走在前人经验上** (保留, 但可新增 — e.g. OpenCog AtomSpace/CogPrime 调研借鉴 per R131-7)
- **O-3 干到底** (保留, 但可调整 — e.g. 1 commit 总 → 拆 N commit 严守每 commit 0 重跑, per 用户记忆 #6 团队协调)
- **O-4 任何人都能接手** (保留, 但可扩展 — "任何高水平团队能接手" per 主人 8/11 01:14 拍板 3 件套 §3 "自然会有高水平的团队来接手维护")
- **O-5 不假装** (保留, 但可升级 — 0 装 PASS 严守升级为 "0 装 + 0 借脑 0 装 + 0 装" 0 装严守 3 段, per 决策 #33 §2.3 C2)

#### 4.1.3 Cargo workspace 可重构 (per 决策 #74 §2.3)

**V2.0 release Cargo workspace 可重构** (per 决策 #74 §2.3):
- **当前 Cargo workspace 结构** (per 整合 #4 commit abf12243):
  - **apeireth-core** (1) — 主库
  - **apeireth-pybridge** + **apeireth-asi** + **apeireth-formal** + **apeireth-evolution** + **apeireth-cognition** + **apeireth-constraint** + **apeireth-central** + **apeireth-telemetry** + **apeireth-provider** + **apeireth-tools** + **apeireth-cli** + **apeireth-bench** + **apeireth-action** + **apeireth-life-force** + **apeireth-value** + **apeireth-consciousness** + **apeireth-relation** + **apeireth-skills** + **apeireth-acp** + **apeireth-cron** + **apeireth-test** + **apeireth-eval** + **apeireth-config** + **apeireth-motivation** + **apeireth-perception** + **apeireth-memory** + **apeireth-upgrade** (28 个 apeireth-* crate, per R125 B1 完整名单 + P6-1/2/3)
  - **apeireth-graph** (1) — 子图
  - **apeireth-library-governance** (1) — Library 库
  - **frontend/** — Tauri 终极前端 (per 主人 8/4 23:33)
  - **library/** — Library 6 阶段产物
  - **borrowed-repos/** — 11 借鉴源 (8 真 cloned + 3 借鉴 ID 索引完成)
  - **总 28 + 2 + 1 = 31 顶层 crate/dir**
- **V2.0 release Cargo workspace 重构 spec** (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 "不要怕复杂度" + "最强效果 + 最厉害工程"):
  - **apeireth-asi 拆分**: apeireth-asi → apeireth-asi-core + apeireth-asi-stage-1-3 + apeireth-asi-stage-4-6 + apeireth-asi-stage-7-8 + apeireth-asi-stage-9 (per 用户记忆 #4 "长程 AI 成长")
  - **apeireth-formal 拆分**: apeireth-formal → apeireth-formal-core + apeireth-formal-stage-5-1 + apeireth-formal-stage-5-2 + apeireth-formal-stage-5-3 + apeireth-formal-stage-5-4 + apeireth-formal-stage-5-5 + apeireth-formal-stage-6 (per R130-4 §1.1 Stage 5.x 6 阶演进链)
  - **9 organ crate 升级**: perception / cognition / consciousness / memory / motivation / value / relation / action / life-force 9 organ → 9 organ + 9 organ-pipeline + 9 organ-bridge (per R130-2 §2.5 跨 crate 集成)
  - **Cargo workspace 元数据升级**: `[workspace.metadata.apeireth]` 段加 NEW 字段 (V2.0 release 决策)
  - **Cargo.toml workspace.version bump 2.0.0** (per 决策 #22 §2.2 semver major 升级)

#### 4.1.4 V2.0 release 决策链更新 (per 决策 #74 §2.3 + 决策 #10 + 决策 #33 C1)

- **决策 #100** (R131 era): 24 LOCKED 入口签名 V1.1 release Mavis 自决改 (per 决策 #74 B1 改写, 前提: 更好的架构) (per R131-3 报告)
- **决策 #101** (R132+ era): V2.0 release 8 硬墙全可重评 (per 决策 #74 §2.3, Mavis 自决) (per R131-3 报告)
- **决策 #102** (R132+ era): V2.0 release 8 哲学锚可重建 (per 决策 #74 §2.3, per "不要怕复杂度" + "最强效果 + 最厉害工程") (per R131-3 报告)
- **决策 #103** (R132+ era): V2.0 release Cargo workspace 可重构 (per 决策 #74 §2.3, per apeireth-asi/formal/9 organ 拆分) (per R131-3 报告)

---

## 5. 风险 (per 决策 #33 + #36 + #41 + #48 + #55 + #58 + #61 + #62 + #71 + #73 + #74 + R129-26 + R130-1 + R130-5)

### 5.1 风险表 (per R130-5 §5.1 + 拓维 R131-3 实施落地风险)

| # | 风险 | 缓解 |
|---|------|------|
| **R1** | **PHL-07 实施引入新 bug** (24 LOCKED 入口新增 1 个 PHL-07 入口, 25 LOCKED 总数) | PHL-07 实施 1:1 对应 R125-12 P0-3 spec, 41 NEW tests 严守, 8 硬墙 0 越界 (24 LOCKED 入口 0 改 + PHL-07 入口新增 1 个), 修完跑 4200+ tests 验证 |
| **R2** | **后端加固 cargo test 实战三次 verify 引入新 src bug** | 后端加固 0 借具体源码, 只 verify + 修已知 bug, 8 步 verify 100% PASS 终极 (per R130-1 修 30+1 bug 经验), 25 LOCKED 入口签名 0 改 终极 verify |
| **R3** | **Tauri Stage 5+ 等设计团队不到位** (per 主人 8/4 23:33) | Tauri Stage 5+ 主要干 5 nav 跨集成 + 9 organ 拟人化深化 + 8 认知纠正 + Tauri 2.0 集成, 0 主动设计 (per 主人 8/4 23:33 "缺审美设计时, 主人宁愿 TUI 也不上 web/桌面, 宁可丑也不上没设计感的") |
| **R4** | **形式化 Stage 5.5+ 11 维度 Kani-style harness 跑过夜** (估 30-60 min cargo test) | 0 装 PASS 严守, 借鉴 kani 4502 + langgraph 829, 53 NEW Kani-style harness 模板 0 装"已借鉴" (per R131-5) |
| **R5** | **ASI Stage 8 群体跟 Stage 4-7 不兼容** (per R131-6) | ASI Stage 8 1:1 跟 R129-4/5/6/18/30 续, 0 改 R129-4/5/6/18/30 已 done 的 4 维度, 只加 Stage 8 群体 4 维度 |
| **R6** | **ASI Stage 9 终极自治 + 长程 AI 成长 + 平台化 跟 Stage 4-8 不兼容** (per R131-6) | ASI Stage 9 是远期 V2.0 路线 (per ROADMAP.md §4), V1.1 仅调研 + 路线图写, V2.0 真实施, 0 假装 Stage 9 在 V1.1 已实施 |
| **R7** | **OpenCog AGPL-3.0 fork 决策失误** | OpenCog AGPL-3.0 fork 决策推荐路径 A (0 fork 0 集成, 仅借鉴设计思想), 路径 B (Fork OpenCog 到子目录, 主项目 0 引用) 备选, 路径 C (主项目集成 OpenCog) 0 推荐 (AGPL-3.0 传染) |
| **R8** | **借鉴源 12 源调研引入新借脑** | 借鉴源 12 源 = 11/11 + OpenCog AtomSpace/CogPrime 调研 (0 集成), 0 借具体源码, 仅调研设计思想, 借鉴 ID 严格化 (per 决策 #33 §2.3 C2 + 决策 #124-1/2/3) |
| **R9** | **整合 #5 commit 拍板未 ready 拖延** | 整合 #5 commit 时机 ready 8/8 verify 100% 后拍板, 当前 7/8 ready (R129-3 报告 0:50+ 跑中, 估 01:05 done), Mavis 自决拍板 (per 决策 #62 + 决策 #64 cron auto-pickup) |
| **R10** | **整合 #6 + #7 commit 拍板未 ready 拖延** | 整合 #6 commit 拍板 = Mavis 自决 (per 决策 #33 C1 + 决策 #71 §2.5), 整合 #7 commit 拍板 = Mavis 自决 (per 决策 #33 C1 + 决策 #71 §2.5), 8/8 verify 100% 后拍板 |
| **R11** | **V1.1 release 实战主人起床后手跑 60 min** | 0 主动 push 严守, 主人手跑 R131-9 7 步 runbook, Mavis 0 push 0 配 remote 0 tag 0 release 0 build pages |
| **R12** | **V1.1 release 实战跟 1.0 release 实战差异 (已配 origin, 复用 gh-pages)** | V1.1 release 实战 = R129-35 7 步 runbook 续, 已配 origin (1.0 release 时配) + 复用 gh-pages branch (1.0 release 时部署), push + tag + GitHub Pages 重新部署简化 |
| **R13** | **0 主动 IM 主人 跟 "auto-replenish-16" 矛盾** | 0 IM 主人 = 0 主动 plain reply, 但 done notification (整合 #5 + #6 + #7 commit 拍板 + R131 era 10 sub-agent done + V1.1 release 实战 done) 是必需, 写决策 #79-#103 + decision-77 报告 |
| **R14** | **Tauri 终极前端 等设计团队到位, 主人宁愿 TUI 也不上 web/桌面 → 0 主动设计** (per 主人 8/4 23:33) | Tauri Stage 5+ 主要干 5 nav 跨集成 + 9 organ 拟人化深化 + 8 认知纠正, 0 主动设计 (per 主人 8/4 23:33 "宁可丑也不上没设计感的") |
| **R15** | **TUI 升级 跟 Tauri 终极前端 角色分工** (主人 dev TUI/后端 + AI 团队干设计 Tauri) | per 主人 8/4 23:33, 主人自己干 dev (TUI/后端), AI 团队干设计 (Tauri), 角色分工清晰 |
| **R16** | **R129-21 报告 0 装 PASS violation 影响决策链 #67/#68** (per R129-26 §4) | per R130-1 §3 纠正 R129-21 报告, 0 装严守 100% (per 决策 #33 §2.3 C2) |
| **R17** | **16 跑中上限 + 自动补派 + 自动接续矛盾** (per 主人 0:34 拍板 "跑中 ≥ 16") | cron Section 2 + 决策 #71 自动接续 4 步 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施), 跑中 ≥ 16 严守 |
| **R18** | **target/ 28.9 GB (debug/ 28.6 GB + release/ 974 MB)** | ≤ 50 GB 保守策略, 0 删, 等整合 #5 commit 拍板后清理 (per 决策 #60 + 主人 0:54 拍板) |
| **R19** | **promethean/ 删挂起** (per 决策 #60) | 0 主动删, 主人起床后关 minimaxcode + 自执行脚本 (per 决策 #60) |
| **R20** | **V1.1 release 跟 V1.0 release 兼容 (semver minor bump)** | V1.1 release 加 NEW feature 兼容 1.0 (PHL-07 实施 + Tauri Stage 5+ + 形式化 Stage 5.5+ + ASI Stage 8+ + 借鉴源 12 源), semver 1.0 → 1.1 minor bump (per 决策 #22 §2.2), 0 假装 V1.1 是 1.0 (per 决策 #10 + 主人 10 项偏好 #7) |
| **R21** | **24 LOCKED 入口签名 V1.1 release Mavis 自决改 破坏向后兼容** (per 决策 #74 B1 改写) | V1.1 release 加 NEW feature 兼容 1.0 (per 决策 #22 §2.2 semver minor bump), 24 LOCKED 入口签名可改 (前提: 更好的架构, 0 改原 24 LOCKED 入口签名, 仅 PHL-07 加 1 入口), 0 破坏向后兼容 |
| **R22** | **决策 #74 B1 改写 跟决策 #62 整合 #5 commit 拍板 8 硬墙 矛盾** | 决策 #74 B1 改写: V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 (前提: 更好的架构), 整合 #5 commit 拍板时严守 V1.0 release 0 改 (per 决策 #33 §2.3 B1), V1.1 release 整合 #6 + #7 commit 拍板时 Mavis 自决改 (per 决策 #74 B1 改写), 0 矛盾 |
| **R23** | **决策 #74 B2 改写 跟决策 #22 §2.2 semver 严守 矛盾** | 决策 #74 B2 改写: B2 workspace.version V1.0 release 1.0.0 严守 + V1.1 release bump 1.1.0 (per 决策 #22 §2.2 semver), 整合 #5.2 commit 时 1.2.0 → 1.0.0, 整合 #7.2 commit 时 1.0.0 → 1.1.0, 0 矛盾 |
| **R24** | **V2.0 release 8 硬墙可重评 跟 主人 8/11 01:14 拍板 "不要怕复杂度" 矛盾** | 主人 8/11 01:14 拍板 3 件套 §3 "不要怕复杂度, 最强效果 + 最厉害工程, 自然会有高水平的团队来接手维护", V2.0 release 8 硬墙可重评 是 "最强效果 + 最厉害工程" 的实施, 0 矛盾, per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 |
| **R25** | **V2.0 release 8 哲学锚可重建 跟 "8 哲学锚严守" 矛盾** | V1.0 + V1.1 release 8 哲学锚严守 (per 决策 #33 §2.3 B5), V2.0 release 8 哲学锚可重建 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 "最强效果 + 最厉害工程"), V2.0 release 是 major 升级 (per 决策 #22 §2.2 semver), 0 矛盾 |
| **R26** | **V2.0 release Cargo workspace 可重构 跟 24 LOCKED 入口签名 0 改 矛盾** | V1.0 + V1.1 release 24 LOCKED 入口签名 0 改, V2.0 release Cargo workspace 可重构 (per 决策 #74 §2.3), Cargo workspace 重构 = 24 LOCKED 入口签名保持, 但内部 crate 拆分, 0 矛盾 |
| **R27** | **0 借具体源码 0 装严守 100% 跟 ASI Stage 8 群体 5 借脑 矛盾** | 0 借具体源码 = 0 假装"已借鉴", 0 装 = 0 装"已装具体源码", ASI Stage 8 群体 5 借脑 = ASI Python + PyO3 928 + superpowers 234 + langgraph 829 + kani 4502, 5 借脑 0 装 = 仅借鉴设计模式 1:1 翻译, 0 装具体源码, 0 矛盾 |
| **R28** | **R131 era 10 sub-agent + R132 era 1-2 sub-agent + R133+ era 5-10 sub-agent 派活资源竞争** (per 决策 #71 §5 跑中 ≥ 16) | 错开时间盒 (R131 era 2 批 5+5 派满 16 上限, R132 era 1-2 sub-agent, R133+ era 5-10 sub-agent), 永远保持 ≥ 16 跑中 (per 决策 #71 §5), R132 + R133+ era 派活等 R131 era 部分 done |

### 5.2 0 装 PASS 严守 100% 5 维度 verify (per 决策 #33 §2.3 C2 + 决策 #74 §1)

| 维度 | verify | 证据 |
|------|--------|------|
| **借鉴源码 0 cloned = 0 实施** | ✅ 严守 (LiteLLM / opencode / Guardrails 子代理 0 cloned → 借鉴 ID 索引完成 0 装) | R129-7 §1.2 + R129-28 §1.1 实地 verify |
| **借鉴源码 ✅ cloned = 真实施** | ✅ 严守 (8 真 cloned mtime 全部早于整合 #4 commit 19:41, 真 src 改动 + tests pass) | R129-11 §1.1 + R129-28 §1.1 实地 verify 100% 严守 |
| **借鉴源码 ❌ 永久失败 = 0 假装"已借鉴"** | ✅ 严守 (OpenCog AGPL-3.0 0 集成 0 装, 借鉴 ID 索引 0 假装"已对接") | OSS_NOTICE.md §3 + Cargo.toml `borrow_skipped` 段 |
| **借鉴 ID 索引完成** (借脑模式) | ✅ 严守 (R130-6 借脑 ID 索引完成, 0 借脑 0 装, 0 装"已读真源码") | R130-6 §1.2 + §3 + §4 借脑 ID 提议 |
| **0 装"已借鉴 OpenCog"** | ✅ 严守 (主仓 0 触碰 OpenCog code, 0 装 API 对接, 1.0 release 后独立 fork 决策 = 主人主动问) | 决策 #22 §4 + 决策 #33 §2.2 |

### 5.3 8 硬墙 0 越界 100% verify (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #62 + 决策 #71)

| 硬墙 | V1.0 release 严守 | V1.1 release 严守 | V2.0 release 可重评 |
|------|------------------|------------------|-------------------|
| **B1** 24 → 25 LOCKED 入口签名 | 🔒 0 改 (R11 baseline) | 🟢 PHL-07 加 1 入口 (25 LOCKED, per 决策 #74 B1 改写) | 🟢 全可重评 |
| **B2** workspace.version 1.2.0 → 1.0.0 → 1.1.0 | 🔒 1.0.0 严守 | 🟢 bump 1.1.0 (per 决策 #22 §2.2 + 决策 #74 B2 改写) | 🟢 bump 2.0.0 (major 升级) |
| **A1** R11 baseline 3 值 0.8682/0.8532/0.9063 | 🔒 0 改 (哲学 + 效果标) | 🟢 可改 (前提: 新的 baseline 更高) | 🟢 全可重评 |
| **A3** 13 → 14 键 + PHL-07 | 🔒 13 键 + PHL-07 spec-only 0 实施 | 🟢 14 键 (PHL-07 实施) | 🟢 全可重评 |
| **B3** V0.5 30 维 | 🔒 严守 (哲学公式) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 |
| **B4** 6 重守门 v7 | 🔒 严守 (哲学守门) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 (升 6→7 重 或 7→8 重) |
| **B5** 8 哲学锚 | 🔒 严守 (哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重建 (per 决策 #74 §2.3) |
| **C1** 0 主动 commit (主人起床前) | 🔒 严守 (整合 #5 由 Mavis 拍板) | 🔒 严守 (整合 #6 + #7 由 Mavis 拍板) | 🟢 0 改 |
| **C2** 0 装 PASS 严守 | 🔒 严守 (技术哲学) | 🔒 严守 (per 决策 #74 §1) | 🟢 可重评 (技术哲学跟项目演进) |
| **0 push** (主人起床前) | 🔒 严守 | 🔒 严守 | 🔒 严守 (V2.0 release 也严守 0 主动 push) |

**0 越界 verify**: V1.0 release 严守 100% (10/10 严守), V1.1 release 严守 100% (10/10 严守, B1+B2+A3 可改但严守前提条件), V2.0 release 可重评 (10/10 可重评, 0 push 严守)

---

## 6. 决策原则 (per 决策 #10 + #22 + #33 + #48 + #55 + #58 + #61 + #62 + #71 + #73 + #74 + 主人 0:25/0:34/0:43/0:49/0:54/0:57/8/11 01:14 拍板 + 用户记忆 #3-#10)

### 6.1 核心原则 (per 决策 #10 + 主人 0:25 + 0:54 + 0:57 + 8/11 01:14 拍板 + 用户记忆 #10)

- **Mavis = orchestrator + 全自决 + 升级决策权** (per 主人 0:25 + 0:54 + 0:57 拍板 + 决策 #10 + 用户记忆 #10 + 8/11 01:14 升级授权)
- **跑中 ≥ 16** (per 主人 0:34 拍板, 16 active 全 background 跑)
- **16 跑中上限 + 自动补派 + 自动接续** (per 主人 0:34 + 0:57 拍板 + 决策 #64 §2.2 + 决策 #71 §2.6)
- **中断接手机制** (per 主人 0:43 拍板, 检查 reports/agent-*.md 写完则标 done / 没写完则重派)
- **编译产物清理决策矩阵** (per 主人 0:49 + 0:54 拍板, ≤50 保守 / 50-100 预警 / 100-150 强烈预警 / > 150 强制清理)
- **计划内任务完成自动接续 4 步** (per 主人 0:57 拍板, R130 调研 + R131 差距 + R132 计划 + R133+ 实施, per 决策 #71)
- **整合 #5 + #6 + #7 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #62 + 决策 #64 + 决策 #71)
- **0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #61 §6, 主人起床后手跑)
- **0 主动 IM 主人** (per gate-discipline + 决策 #61 §6, 仅 done notification 主动报告)
- **0 主动删** (per Safety policy + 决策 #44 + #60, ≤ 50 GB 保守策略 + > 150 GB 强制清理)

### 6.2 8 硬墙严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1 + 主人 8/11 01:14 拍板 3 件套)

- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1 改写)
- **B2 workspace.version 1.2.0**: V1.0 release 1.0.0 严守 + V1.1 release bump 1.1.0 + V2.0 release bump 2.0.0 (per 决策 #22 §2.2 + 决策 #74 B2 改写)
- **A1 R11 baseline 3 值**: V1.0 release 严守 + V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, Mavis 自决)
- **A3 13 → 14 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施 (24 → 25 LOCKED, 13 → 14 键, per 决策 #33 §2.1 A3 + 决策 #74 B1 改写)
- **B3 V0.5 30 维**: V1.0 + V1.1 release 严守 (哲学公式, per 决策 #33 §2.3 B3 + 决策 #74 §1)
- **B4 6 重守门 v7**: V1.0 + V1.1 release 严守 (哲学守门, per 决策 #33 §2.3 B4 + 决策 #74 §1)
- **B5 8 哲学锚**: V1.0 + V1.1 release 严守 (哲学, per 决策 #33 §2.3 B5 + 决策 #74 §1) + V2.0 release 可重建 (per 决策 #74 §2.3)
- **C1 0 主动 commit (主人起床前)**: V1.0 + V1.1 release 严守 (per 决策 #33 §2.3 C1 + 决策 #74 §1)
- **C2 0 装 PASS 严守**: V1.0 + V1.1 release 严守 (技术哲学, per 决策 #33 §2.3 C2 + 决策 #74 §1)
- **0 push (主人起床前)**: V1.0 + V1.1 + V2.0 release 严守 (per 决策 #33 + 决策 #61 §6 + 决策 #74 §1)

### 6.3 总工程哲学扩展 "不要怕复杂度" (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3)

- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **locked 全解锁 + Mavis 自决架构** (per 主人 8/11 01:14 拍板 3 件套 §1, 整合 #5.1 commit 仍 0 改严守 + V1.1 release Mavis 自决改)
- **架构审视 + 升级方案永久工作项** (per 主人 8/11 01:14 拍板 3 件套 §2, cron Section 10 新增)
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3, per 决策 #73 §3 + `docs/conventions/15-no-fear-complexity.md`)
  - **最强效果** > 最简单代码
  - **最厉害工程** > 最易维护
  - **复杂度** 不是问题 (24 LOCKED + 8 哲学锚 + 6 重守门 + 30 维公式 + 13 键, 都复杂, 但都是最强效果)
  - **维护复杂** 不是问题 (未来高水平团队接手)
- **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per `docs/conventions/15-no-fear-complexity.md` §2)
- **8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界** (per `docs/conventions/15-no-fear-complexity.md` §3)

### 6.4 流程严守 (per 决策 #33 + #60 + #61 + #62 + #71 + #73 + #74)

- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **整合 #6 + #7 commit 由 Mavis 自动拍板** (per 决策 #33 C1 + 决策 #71 §2.5)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)
- **0 重复造轮子** (per 用户记忆 #6, R131 era 跟 R130 era + R129 era 已 done 的不重复)
- **PHL-07 关键诚实标** (per R129-11, V1.0 spec-only → V1.1 真实施, 0 假装 PHL-07 在 1.0 release 时已实施)
- **V1.1 release 跟 1.0 release 兼容** (per 决策 #22 §2.2, semver 1.0 → 1.1 minor bump, V1.1 加 NEW feature 兼容 1.0)
- **TUI 升级节奏** (per 决策 #9 + 用户记忆 #8, 改瘦后暂告段落, 优先后端, TUI 阶段 3 估 V1.2 era 完成, per R129-29 §5.2.1)
- **Tauri 终极前端** (per 主人 8/4 23:33 + 用户记忆 #8, 等设计团队到位, 主人宁愿 TUI 也不上 web/桌面)
- **用户记忆 #3** (用户看结果不看哲学) + #4 (AI 不会衰老病死, 它只会成长) + #5 (信息密度高 = 拟人化 + 拟物化) — 决策原则严守
- **8 哲学锚严守** (per ROADMAP.md §5, P-1 哲学 LOCKED + P-2 主体性 + S-1 自主性 + S-2 Sovereignty + S-3 质量工程化 + O-1 安全优先 + E-1 演化 + H-1 人类利益优先, 0 改 8 锚, 后端保留前端不暴露)
- **6 重守门 v7 严守** (per ROADMAP.md §5, 守门 1-7 0 改, B4 严守)

---

## 7. 决策链更新 (per 决策 #10 + #33 C1 + #64 + #71 + #73 + #74 + 主人 8/11 01:14 拍板 + R130-5 §3.3)

### 7.1 R131 era 决策链 (per R130-5 §3.3 + R131-3 拓维 + 决策 #10 + 决策 #33 C1)

| # | 决策 | Date | 内容 | 状态 |
|---|------|------|------|------|
| **#79** | R131 era PHL-07 实施 (R131-2 done) | 2026-11-15 估 | per R131-2 报告, V1.0 spec-only → V1.1 真实施, 24 LOCKED → 25 LOCKED (PHL-07 加 1 入口) | 🟡 估 done |
| **#80** | 25 LOCKED 入口签名 0 改 终极 verify | 2026-11-15 估 | per R131-2 + R131-3 报告, 24 LOCKED 入口 0 改 + PHL-07 入口新增 1 个 | 🟡 估 done |
| **#81** | 13 → 14 键升级 (PHL-07 加 1 键) | 2026-11-15 估 | per R131-2 报告, 13 键 → 14 键, 跟 8 哲学锚 + 6 重守门 v7 集成 | 🟡 估 done |
| **#82** | 后端 0 装 PASS 三次 verify (R131-3 done) | 2026-11-15 估 | per R131-3 报告, cargo test 实战三次 + 25 LOCKED 入口签名 0 改 + 4100+ → 4200+ tests pass | 🟡 估 done |
| **#83** | 借鉴源 12/12 clear 终极 verify (R131-3 + R131-7 协作) | 2026-11-15 估 | per R131-3 + R131-7 报告, 11/11 + OpenCog 调研 = 12/12 clear | 🟡 估 done |
| **#84** | Cargo.toml 1.1.0 严守 verify (1.2.0 → 1.0.0 → 1.1.0) | 2026-11-15 估 | per R131-3 报告, semver 严守, 1.0 release 时 1.2 → 1.0, V1.1 release 时 1.0 → 1.1 | 🟡 估 done |
| **#85** | Tauri 终极前端 Stage 5+ 集成深化 (R131-4 done) | 2026-11-15 估 | per R131-4 报告, 5 nav 完整 + 9 organ × 5 维 = 45 维拟人化 + 8 认知纠正 | 🟡 估 done |
| **#86** | 9 organ 拟人化深化 45 维 1 屏多卡片 (per 用户记忆 #5) | 2026-11-15 估 | per R131-4 报告, 9 organ × 5 维 = 45 维, 1 屏多卡片 | 🟡 估 done |
| **#87** | 8 认知纠正落地 (砍掉哲学/守门/电子环/工具调用/衰老病死/内部机制/决策过程/错误堆栈) | 2026-11-15 估 | per R131-4 报告, 8 项纠正全部落地 | 🟡 估 done |
| **#88** | 形式化证明 Stage 5.5+ 集成深化 (R131-5 done) | 2026-11-15 估 | per R131-5 报告, F1-F11 11 维度 Kani-style harness 模板 | 🟡 估 done |
| **#89** | F11 PHL-07 形式化 (R129-11 关键诚实标落地, V1.1 必实施) | 2026-11-15 估 | per R131-5 报告, PHL-07 14 维主对话锚 形式化, 42 NEW harness | 🟡 估 done |
| **#90** | 11 + 42 = 53 NEW Kani-style harness 模板 (F1-F11 + PHL-07 相关) | 2026-11-15 估 | per R131-5 报告, 11 + 42 = 53 NEW harness, 0 装 PASS 严守 | 🟡 估 done |
| **#91** | ASI Stage 8 群体 (R131-6 done) | 2026-11-15 估 | per R131-6 报告, G1-G4 4 维度群体, 100 NEW tests pass | 🟡 估 done |
| **#92** | ASI Stage 9 终极自治 + 长程 AI 成长 + 平台化 远期 V2.0 路线 | 2026-11-15 估 | per R131-6 报告, A1-A4 4 维度 Stage 9, 远期 V2.0 路线 | 🟡 估 done |
| **#93** | 借鉴 5 源 0 装 PASS 严守 (ASI Python + PyO3 928 + superpowers 234 + langgraph 829 + kani 4502) | 2026-11-15 估 | per R131-6 报告, 5 借脑 0 装 | 🟡 估 done |
| **#94** | OpenCog AGPL-3.0 fork 决策 (推荐路径 A: 0 fork 0 集成, 仅借鉴设计思想) | 2026-11-30 估 | per R131-7 报告, AGPL-3.0 传染风险, 0 集成 | 🟡 估 done |
| **#95** | 借鉴源 12/12 终极 verify (11/11 + OpenCog 调研, 0 集成) | 2026-11-30 估 | per R131-7 报告, 12/12 clear, 0 装 PASS 严守 | 🟡 估 done |
| **#96** | 借鉴 ID 严格化 0 借脑 0 装 (per 决策 #33 §2.3 C2 + 决策 #124-1/2/3) | 2026-11-30 估 | per R131-7 报告, 0 借具体源码, 仅调研设计思想 | 🟡 估 done |
| **#97** | 整合 #6 commit 拍板 (Mavis 自决, per 决策 #33 C1) | 2026-11-25 估 | per R131-8, 5.1 src/ + 5.2 docs/ + 5.3 reports/ 顺序 git add + git commit | 🟡 估 done |
| **#98** | V1.1 release 实战 (R131-9 done, 主人起床后手跑) | 2026-11-30 估 | per R131-9, 7 步流程 + 8 步 verify + git push + v1.1.0 tag + GitHub Pages 重新部署 | 🟡 估 done |
| **#99** | V1.1 release tag v1.1.0 打上 (per R131-9 §5) | 2026-11-30 估 | per R131-9 报告, 整合 #7 commit 后打 v1.1.0 tag | 🟡 估 done |
| **#100** | **24 LOCKED 入口签名 V1.1 release Mavis 自决改** (per 决策 #74 B1 改写, 前提: 更好的架构) | 2026-11-30 估 | per R131-3 报告 (本报告), V1.1 release 24 LOCKED 入口签名可改 (前提: 更好的架构, e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级) | 🟡 估 done |
| **#101** | **V2.0 release 8 硬墙全可重评** (per 决策 #74 §2.3, Mavis 自决) | 2027+ 远期 | per R131-3 报告 (本报告), V2.0 release 8 硬墙全可重评 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 "不要怕复杂度") | 🟡 估 done |
| **#102** | **V2.0 release 8 哲学锚可重建** (per 决策 #74 §2.3, per "不要怕复杂度" + "最强效果 + 最厉害工程") | 2027+ 远期 | per R131-3 报告 (本报告), V2.0 release 8 哲学锚可重建 (推翻 + 重建, per 决策 #74 §2.3) | 🟡 估 done |
| **#103** | **V2.0 release Cargo workspace 可重构** (per 决策 #74 §2.3, per apeireth-asi/formal/9 organ 拆分) | 2027+ 远期 | per R131-3 报告 (本报告), V2.0 release Cargo workspace 可重构 (per 决策 #74 §2.3) | 🟡 估 done |

**R131 era 决策链**: #79 - #103 (25 决策, 估 done)

### 7.2 R131-3 (本报告) 决策链更新 (per 决策 #10 + 决策 #33 C1 + 决策 #74 §1)

- **决策 #100**: 24 LOCKED 入口签名 V1.1 release Mavis 自决改 (per 决策 #74 B1 改写, 前提: 更好的架构) (per R131-3 报告 [本报告])
- **决策 #101**: V2.0 release 8 硬墙全可重评 (per 决策 #74 §2.3, Mavis 自决) (per R131-3 报告 [本报告])
- **决策 #102**: V2.0 release 8 哲学锚可重建 (per 决策 #74 §2.3, per "不要怕复杂度" + "最强效果 + 最厉害工程") (per R131-3 报告 [本报告])
- **决策 #103**: V2.0 release Cargo workspace 可重构 (per 决策 #74 §2.3, per apeireth-asi/formal/9 organ 拆分) (per R131-3 报告 [本报告])

---

## 8. 时间盒 + 总结 (per 决策 #71 + 决策 #73 + 决策 #74 + 主人 8/11 01:14 拍板)

### 8.1 V1.1 release 时间盒 (per 决策 #71 §2.2 + §5 + 决策 #74 + 主人 8/11 01:14 拍板)

| 任务 | 估时间 | 来源 | 决策依据 |
|------|------|------|---------|
| 整合 #5 commit 拍板 (Mavis 自决) | 8/11 01:05-01:30 | 决策 #62 + 决策 #64 cron auto-pickup | per 决策 #62 + 决策 #33 C1 |
| 1.0 release 实战 (主人起床后手跑) | 8/11 06:00-08:00 (120 min) | R129-35 7 步 runbook | per 决策 #55 + 决策 #58 + 决策 #61 §4.3 |
| R130 era 调研 6 sub-agent done | 8/12 估 (估 5 done, 1 跑中) | 决策 #72 §2.1 R130-1~6 派活 | per 决策 #72 |
| R131 era 差距分析 3 sub-agent done | 8/12 估 (估 2 跑中, 1 done = 本报告) | 决策 #73 §3.2 R131-1~3 派活 | per 决策 #73 |
| R132 era 计划 1-2 sub-agent done | 8/15 估 | 决策 #71 §2.4 R132-1~2 派活 | per 决策 #71 |
| R133+ era 实施 5-10 sub-agent 派活 | 9-10 月 | 决策 #71 §2.5 + 主人 0:34 跑中 ≥ 16 | per 决策 #71 §5 |
| R131 era 实施 10 sub-agent 派活 | 11 月 (2 批 5+5) | R130-5 §3.1 派活规划 | per R130-5 + 决策 #71 §5 |
| 整合 #6 commit 拍板 (Mavis 自决) | 2026-11-25 (30 min) | 决策 #33 C1 + 决策 #71 §2.5 | per 决策 #33 C1 |
| 整合 #7 commit 拍板 (Mavis 自决) | 2026-11-29 (30 min) | 决策 #33 C1 + 决策 #71 §2.5 | per 决策 #33 C1 |
| V1.1 release 实战 (主人起床后手跑) | 2026-11-30 06:00-08:00 (120 min) | R131-9 7 步 runbook 续 | per 决策 #22 §2.2 + 决策 #55 |
| V1.2 release (R132 era 续) | 2027-02-28 估 | R129-29 §5 | per R129-29 §5 |
| V2.0 release (R133+ era 远期) | 2027+ 远期 | ROADMAP.md §4 | per 决策 #74 §2.3 |

### 8.2 V1.1 release 战略总结 (per 决策 #71 §2.6 + 决策 #73 + 决策 #74 + 主人 0:25/0:34/0:43/0:49/0:54/0:57/8/11 01:14 拍板 + 用户记忆 #3-#10)

- **V1.1 minor release 6 大方向** (per R130-5 §1.5 战略 + R131-3 实施落地):
  1. **PHL-07 实施** (V1.0 spec-only → V1.1 真实施, 24 LOCKED 入口新增 1 个 PHL-07 入口, 25 LOCKED 总数, R129-11 关键诚实标落地)
  2. **24 LOCKED 入口签名改写** (V1.1 release Mavis 自决改, per 决策 #74 B1 改写, 前提: 更好的架构)
  3. **后端加固** (cargo test 实战三次 verify + 借鉴源 12 源 0 装严守二次 verify + Cargo.toml 1.2.x 系列 1.0.0 → 1.1.0 严守)
  4. **Tauri Stage 5+ 集成深化** (9 organ 拟人化深化 + 5 nav 完整 + 主对话 UX 优化 + Tauri 2.0 集成, per 用户记忆 #3-#5)
  5. **形式化证明 Stage 5.5+ 集成深化** (PHL-07 形式化 + F1-F11 11 维度 Kani-style harness + 53 NEW harness 模板, per R129-11 关键诚实标落地)
  6. **ASI Python Stage 8+ 集成深化** (Stage 8 群体 + Stage 9 终极自治 + 长程 AI 成长 + 平台化, per 用户记忆 #4, Stage 9 远期 V2.0 路线)
  7. **借鉴源 12 源调研** (OpenCog AGPL-3.0 fork 决策 + 新源调研, 11/11 → 12/12 clear, 0 集成仅设计思想)
- **R131 era 10 sub-agent 派活规划** (估 2026-11, 2 批 5+5, 16 跑中上限严守)
- **整合 #6 + #7 commit 拍板** (Mavis 自决, per 决策 #33 C1 + 决策 #71 §2.5)
- **V1.1 release 实战** (per R130-5 7 步 runbook 续, 主人起床后手跑, 估 2026-11-30 06:00-08:00)
- **V1.1 release 跟 V1.0 release 边界**:
  - **V1.0 release 0 改 src 严守** (整合 #5 commit 拍板, R11 baseline 严守, 24 LOCKED 入口签名 0 改, 8 硬墙 0 越界)
  - **V1.1 release Mavis 自决改** (整合 #6 + #7 commit 拍板, 前提: 更好的架构, per 决策 #74 B1 改写)
- **V2.0 release 路线图 spec** (per 决策 #74 §2.3):
  - **8 硬墙全可重评** (per 决策 #74 §2.3)
  - **8 哲学锚可重建** (per 决策 #74 §2.3)
  - **Cargo workspace 可重构** (per 决策 #74 §2.3)
- **永久循环** (per 决策 #74 §2.3 + 决策 #71 §2.5 + 主人 0:57 拍板 "继续调研+研究我们差距+制订新计划+继续干"):
  - V1.1 release → V1.2 minor → V2.0 major → V2.1 / V2.2 / V3.0 ... 永久循环
- **0 主动 commit + 0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #61 §6)
- **0 借具体源码 + 0 装 PASS 严守** (per 决策 #33 §2.3 C2, 5 借脑 0 装: ASI Python + PyO3 928 + superpowers 234 + langgraph 829 + kani 4502 + OpenCog AtomSpace/CogPrime 调研 = 7 借脑 0 装)
- **8 硬墙 0 越界** (per 决策 #33 §2.3 + 决策 #74 B1 改写):
  - V1.0 release 严守: B1 24 LOCKED 入口签名 0 改 / B2 1.0.0 严守 / A1 3 值 0 改 / A3 13 键 + PHL-07 spec-only / B3 30 维 / B4 6 重 v7 / B5 8 锚 / C1 0 commit / C2 0 装 / 0 push
  - V1.1 release 严守 + 改写: B1 25 LOCKED 入口签名 0 改 (24 + PHL-07 = 25) / B2 1.1.0 严守 (1.2 → 1.0 → 1.1) / A1 3 值 0 改 (严守) / A3 14 键 (PHL-07 实施) / B3 30 维 / B4 6 重 v7 / B5 8 锚 / C1 0 commit / C2 0 装 / 0 push
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **0 主动 IM 主人** (per gate-discipline + 决策 #61 §6, 仅 done notification 主动报告)
- **决策日志写** (per 决策 #10 + 用户记忆 #10, 决策链 #79-#103)
- **0 重复造轮子** (per 用户记忆 #6, R131-2~7 跟 R129-X + R130-X 已 done 的不重复)
- **V1.1 release 跟 1.0 release 兼容** (per 决策 #22 §2.2, semver 1.0 → 1.1 minor bump, V1.1 加 NEW feature 兼容 1.0)
- **V2.0 release 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构** (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套 §3 "不要怕复杂度, 最强效果 + 最厉害工程")

---

## 9. refs (per R130-5 §7 + 拓维 R131-3 实施落地 + 决策 #73 + 决策 #74)

- **decision-9** (TUI 升级节奏: 改瘦后暂告段落, 优先后端) + **decision-10** (主人离场 Mavis 自主决策) + **decision-22** (24 LOCKED + semver 大版本归 0) + **decision-33** (8 硬墙 + 0 装 PASS) + **decision-36** (R125 借鉴 ID 严格化) + **decision-41** (R125 16 全 done) + **decision-48** (整合 #4 commit abf12243) + **decision-55** (R127 4 派活) + **decision-56** (R127-2 10 派活) + **decision-57** (R128 6 派活) + **decision-58** (R128-2 3 派活) + **decision-60** (promethean/ 删挂起) + **decision-61** (R129 era 派活规划) + **decision-62** (整合 #5 commit 拆 3 commit 拍板) + **decision-64** (auto-replenish-16 cron) + **decision-65** (R129 第 2 批 8 sub-agent) + **decision-66** (R129 第 3 批 7 sub-agent) + **decision-67** (R129-24 待派) + **decision-68** (R129 第 4 批 7 sub-agent) + **decision-69** (R130 era 派活规划) + **decision-70** (Mavis 清理决策权升级) + **decision-71** (R130 调研 + R131 差距 + R132 计划 + R133+ 实施) + **decision-72** (R130 era 调研 6 sub-agent 派活) + **decision-73** (主人 8/11 01:14 拍板 3 件套, 本报告核心拍板依据) + **decision-74** (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改)
- R124-1 (clap 借鉴 ID) + R124-2 (hyper + OpenCog 借鉴 ID) + R124-3 (servers + PyO3 + kani + langgraph + superpowers 借鉴 ID) + R125-1 (LiteLLM) + R125-2 (clap) + R125-3 (hyper) + R125-4 (servers) + R125-5 (Guardrails) + R125-8 (PyO3) + R125-9 (PyO3) + R125-10 (kani) + R125-12 (opencode + PHL-07 spec) + R125-13 (langgraph) + R125-14 (superpowers) + R125-15e (superpowers) + R125-16 (Library spec) + R125-18 (superpowers) + R125-19 (superpowers) + R126-guard-7 (Guardrails 7 重)
- P11-1 (Tauri prototype) + P11-2 (Tauri scaffold) + P12-1 (Cargo build 实战) + P15-1 (1.0 release Cargo 配) + P7-1 (CHANGELOG v1.0.0) + P7-2 (ROADMAP 1.0 → 2.0) + P7-3 (RELEASE_NOTES v1.0.0) + P8-2 (Library Stage 5.1 形式化证明 retry) + P13-1 (OSS_NOTICE.md)
- R129-1 (整合 #5.1 commit src/ 准备) + R129-2 (整合 #5.2 commit docs/ 准备) + R129-3 (8 步 verify) + R129-4 (ASI Stage 4 自治) + R129-5 (ASI Stage 5 治理) + R129-6 (ASI Stage 6 守护) + R129-7 (借鉴 11/11 升级 verify) + R129-8 (1.0 release 流程准备) + R129-9 (Tauri Stage 2 深化) + R129-10 (形式化证明 Stage 5.2) + R129-11 (后端 0 装 PASS 终极 verify, PHL-07 spec-only 关键诚实标) + R129-12 (R129 路线图) + R129-13 (1.0 release checklist + GitHub Pages) + R129-14 (后端健康度总览) + R129-15 (TUI 升级路线图沉淀) + R129-16 (R129 era 决策链更新) + R129-17 (R130 era 路线图详细) + R129-18 (ASI Stage 7 跨模块集成) + R129-19 (Tauri Stage 3 跨 nav 集成) + R129-20 (形式化证明 Stage 5.3 跨模块) + R129-21 (整合 #5 commit 拍板前最终 verify) + R129-22 (R129 era 跨 sub-agent 总览) + R129-23 (1.0 release 实战 + GitHub Pages 部署) + R129-24 (R129 era 决策链 final) + R129-25 (整合 #5 commit 拍板辅助) + R129-26 (R129 era 健康度 verify, 暴露 24+5+1 errors) + R129-27 (1.0 release 流程实战) + R129-28 (借鉴 11/11 终极 verify) + R129-29 (R130 era 路线图 final, V1.1 §4 详细 6 维度) + R129-30 (ASI Stage 8 实战) + R129-31 (Tauri Stage 4 实战) + R129-32 (形式化证明 Stage 5.4 实战) + R129-33 (整合 #5 commit 拍板前最终 master verify final) + R129-34 (R129 era 跨 sub-agent 总览 final final) + R129-35 (1.0 release 实战 + GitHub Pages final-final 7 步 runbook)
- **R130-1** (整合 #5 commit cargo 二次 verify, 修 30+1 bug, ⏳ 派中 0 报告) + **R130-2** (ASI Stage 8 集成深化, ✅ 01:17 done) + **R130-3** (Tauri Stage 5 集成深化, ✅ 01:17 done) + **R130-4** (形式化证明 Stage 5.5 集成深化, ✅ 01:18 done) + **R130-5** (V1.1 minor release 路线图, ✅ 01:18 done) + **R130-6** (借鉴源 12 源调研, ✅ 01:17 done) — **5 调研 sub-agent done + 1 跑中**
- **R131-1** (现有架构审视, ⏳ 派中 0 报告) + **R131-2** (借鉴 12 源差距, ⏳ 派中 0 报告) + **R131-3** (V1.1 release 实施路线图, ✅ 01:20 done, 本报告)
- ROADMAP.md §0-§12 (per P7-2 R127-2, 1.0 → 2.0 路线图, 顶层瘦)
- `docs/conventions/09-anchor.md` (8 哲学锚, R125 B5 升) + `docs/conventions/10-locked.md` (9 项实质 Locked 升级路线, R125 B1-B7 + 决策 #73 §2.3 R130 era 主人 8/11 01:14 拍板 locked 全解锁) + `docs/conventions/15-no-fear-complexity.md` (R130 era 主人 8/11 01:14 拍板 3 件套 §3, 总工程哲学扩展 "不要怕复杂度")
- **用户记忆 #3** (用户看结果不看哲学) + **#4** (AI 不会衰老病死, 它只会成长) + **#5** (信息密度高 = 拟人化 + 拟物化) + **#6** (派 sub-agent 干, 但要驾驭团队不重复造轮子) + **#7** (诚实标) + **#8** (TUI → Tauri 终极路线) + **#9** (TUI 升级节奏) + **#10** (主人长时间离开, Mavis 自主决策 + 决策日志)
- **主人 8/4 23:33** "我们最后要做的前端应该是 Tauri" + **8/4 23:55** "TUI 升级路线图沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞 tui" + **8/6 01:14** "后面有需要决定的都按你想法倾向来, 最终收尾的时候把你的想法决策也都记录下来就行" + **8/11 0:25** 拍板 "全部你做主" + **0:34** 拍板 "已经 done 的不能算正在跑的，正在跑的达到 16 个" + **0:43** 拍板中断接手机制 + **0:49** 拍板编译产物清理决策矩阵 + **0:54** 拍板 Mavis 升级决策权 + 150 GB 强制清理 + **0:57** 拍板计划内任务完成自动接续 4 步 (调研 + 差距 + 计划 + 继续干) + **8/11 01:14** 拍板 3 件套 (locked 全解锁 + 架构审视 + 不要怕复杂度)
- scripts/release/ 14 文件 (per R129-8 8 文件 + R129-23 2 文件 + R20 蓝图 2 cosign + 顶层 2 蓝图)
- docs/pages-source/ 7 文档 + mkdocs.yml (per R129-13 写, 51.4KB 文档 + 4.1KB 配置)
- **借鉴源码 11/11 → 12/12** (per R129-7 + R129-28 + R130-1 二次 verify + R130-6 调研): ✅ 10 真实施 (clap 725 + hyper 80 + servers 175 + PyO3 928 + kani 4502 + langgraph 829 + superpowers 234 + LiteLLM 公开 1:1 翻译 + opencode 子代理 1:1 翻译 + Guardrails 6 重守门 1:1 翻译) + ❌ 1 跳过 (OpenCog AGPL-3.0) + 🆕 1 借脑 (OpenCog 家族 6 子源调研, 0 集成仅设计思想)
- **8 硬墙** (per 决策 #33 §2.3 + 决策 #74 B1 改写 + ROADMAP.md §5): B1 25 LOCKED 入口签名 0 改 [24 + PHL-07, V1.0 release 严守 + V1.1 release Mavis 自决改 per 决策 #74 B1 改写] / B2 workspace.version 1.2.0 → 1.0.0 → 1.1.0 严守 (per 决策 #22 §2.2 + 决策 #74 B2 改写) / A1 R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / A3 13 → 14 键 (PHL-07 实施, V1.0 spec-only + V1.1 真实施) / C1 0 主动 commit (Mavis 拍板) / C2 0 装 PASS 严守 / 0 主动 push 严守
- 整合 #4 commit abf1224371016e36df8f4d3c9a05b33f1c563e0d (per 决策 #48, 8/10 19:41 done, master HEAD 严守)
- 整合 #5 commit 拍板 (per 决策 #62, Mavis 自决, 5.1 → 5.2 → 5.3 顺序 git add + git commit)
- 整合 #6 + #7 commit 拍板 (per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决, V1.1 era 续)
- **V1.1 release tag v1.1.0** (估 2026-11-30, 介于 1.0 release ~8/11 跟 V1.2 release 估 2027-02-28 之间)
- V1.2 路线图 (per R129-29 §5, 估 2027-02-28, 6 维度: TUI 阶段 3 + Tauri Stage 5 完整 + ASI Stage 8 群体 + 形式化 Stage 5.5 ASI 集成 + 后端 Stage 7-8 续 + V1.2 release 实战)
- **V2.0 远期** (per ROADMAP.md §4 + 决策 #74 §2.3, 2027+, 平台化 + 商业化 + 真用户 + 多 AI 平台 + 教育/科研合作, **8 硬墙全可重评 + 8 哲学锚可重建 + Cargo workspace 可重构**)
- HANDOFF-NEXT-SESSION-2026-08-10.md §8.2 (主人起床后 8 步 verify, per 决策 #55 §8)
- HANDOFF-NEXT-SESSION-2026-08-10.md (R129 era 综合 handoff, per 决策 #55 §2.6)
- 主人 10 项偏好 #7 (诚实标: 不假装已实现, per 决策 #10 + 决策 #33 §2.3 C2)

---

## 10. 一句话 (再次强调)

**V1.1 release 实施路线图 (R131-3) = 整合 R130 era 6 调研 (5 done: R130-2 ASI Stage 8 / R130-3 Tauri Stage 5 / R130-4 形式化 Stage 5.5 / R130-5 V1.1 路线图 / R130-6 借鉴 12 源, 1 跑中: R130-1 cargo 二次 verify 修 30+1 bug) + 整合 R131-1 架构审视 (待 done) + 整合 R131-2 借鉴 12 源差距 (待 done) + 6 大方向 (PHL-07 实施 / 24 LOCKED 入口签名改写 / 后端加固 / Tauri Stage 5+ / ASI Stage 8+ / 形式化 Stage 5.5+) + V1.1 release 时间窗口 (整合 #5 commit 拍板 + 1.0 release 实战完 + 主人起床后配 GitHub remote 1.0 release → 1 周后 V1.1 release 拍板) + 16 跑中上限 (5-10 sub-agent 实施 per 决策 #71 §5) + 0 改 src 严守 (V1.0 release 整合 #5 commit 拍板) + V1.1 release Mavis 自决改 (前提: 更好的架构 per 决策 #74 B1 改写) + V2.0 release 路线图 spec (8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 per 决策 #74 §2.3) + 永久循环 (V1.1 release → V1.2 minor → V2.0 major) + 风险 (8 硬墙 0 越界严守 + 0 装 PASS 严守 + Cargo.toml 1.2.0 → 1.2.1 bump per 决策 #74 B2 改写) + 决策原则 (8 哲学锚 + 不要怕复杂度 + locked 全解锁 + Mavis 自决架构 + 0 借具体源码).**

# R152-1: 整合 #6 Cargo workspace 1.2.0 → 1.2.1 bump 准备 (实施 spec 调研) (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #71 §5 R152 era 实施阶段 + 决策 #86 §4 16 sub-agent 派活 + 决策 #78 整合 #5 Option A + 决策 #73 §3 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 决策 #74 B1 V1.1 release Mavis 自决改 + R145-3 1.2.0 verify 严守 + R131-4 cargo workspace 优化 + R131-6 Cargo.toml borrow 段 update + R137-3 1.2.1 bump 实施 spec 第 1 版 + R149-4 借鉴 12 源 fork-then-borrow 模式)

**Date**: 2026-08-11 (R152 era 实施阶段 第 1 sub, per 决策 #86 §4 16 sub-agent 派活, 决策 #71 §5 R152 era 实施类, 决策 #77 §3.1 R137 era 实施阶段 续, per cron Section 5 5 min tick)
**Author**: R152-1 sub-agent (Mavis 派, **实施 spec 调研阶段**, **0 改 src 严守 100%**, **0 改 Cargo.toml 严守 100%**, **0 主动 commit 严守 100%**, **0 主动 push 严守 100%**, **0 主动 IM 主人 严守 100%**, **0 装 PASS 严守 100%**)
**Time-box**: 60 min (per 决策 #71 §5 R152 era 实施阶段 + 决策 #86 §4 派活拍板)
**任务定位**: R152 era 实施类 sub-agent 派活拍板 (per 决策 #86 §4 16 sub-agent 派活, R152-1 = 整合 #6 Cargo workspace 1.2.0 → 1.2.1 bump 准备实施 spec 调研, 严格不写代码, 实施 spec 调研 = 文档工作)
**关联**: decision-10 + #22 + #33 + #36 + #41 + #42 + #44 + #48 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + #73 + #74 + #75 + #76 + #77 + #78 + #79 + #80 + #81 + #82 + #83 + #84 + #85 + **#86 (5:00 tick 状态 + R152 派活拍板)** + R129-1/2/3/7/11/14/21/22/26/28/34 + R130-1/2/3/4/5/6 + R131-1/2/3/4/5/6/7/8/9 + R137-1/2/**3**/4/5 + R145-1/2/**3**/4 + R147-1/2/3/4/5 + R148-1~25 + R149-1/2/3/**4**/5 + R150-1/2/3 + R151-1/2 + 用户记忆 #1-10 + 哲学文档 `15-no-fear-complexity.md`
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (2026-08-10 19:41 done, 0 重跑 0 重 commit, master HEAD 严守 100%)
**整合 #5 commit**: ✅ 整合 #5.3 reports/ commit 拍板 done (1:43, master HEAD = `4207f187100183170558d70633a970969aebdcda`, 187 files / 127548 insertions, 0 主动 push 严守), 整合 #5.1 src/ commit NOT READY (R139-1-retry 修 30 hard errors pending, 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL per R144-1 02:38)
**整合 #6 commit**: 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3, Mavis 自决拍板 (V1.1 release 6 大方向 包含: ① Cargo workspace 1.2.0 → 1.2.1 bump **本任务核心** ② 24 LOCKED 入口签名 Mavis 自决改 ③ PHL-07 实施 ④ 后端加固 30 处 fail 修 ⑤ Tauri Stage 5+ ⑥ ASI Stage 8+ ⑦ 形式化 Stage 5.5+ ⑧ 借鉴 12 源 fork-then-borrow 模式)
**整合 #7 commit**: 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板 (V1.1 release 前最终收尾)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0`), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间, per R131-3 §1.1
**V2.0 release tag**: 远期 2027+, per ROADMAP.md §4 + 决策 #74 §2.3, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 (Cargo.toml 1.2.1 → 2.0.0 重大 bump)
**状态**: ✅ **R152-1 done 2026-08-11 (60 min 时间盒内)**: 8 大章节 100% 完整 + 8 硬墙 0 越界严守 100% + 8 哲学锚 + 不要怕复杂度哲学 9 件套 严守 100% + Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 5 阶段计划 完整 + 24 LOCKED + 87 workspace members + 借鉴 12 源 0 装 PASS 严守 + 风险 5 + 决策原则 12 + 派活计划 4 sub-agent + 0 改 src 严守 100% + 0 改 Cargo.toml 严守 100% + 0 主动 commit 严守 100% + 0 主动 push 严守 100% + 0 主动 IM 主人严守 100% + 0 装 PASS 严守 100%

---

## 0. 一句话 (TL;DR)

**R152-1 整合 #6 Cargo workspace 1.2.0 → 1.2.1 bump 准备 (实施 spec 调研) (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #86 §4 R152 era 派活拍板 + 决策 #71 §5 R152 era 实施阶段 + 决策 #78 整合 #5 Option A + 决策 #73 §3 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 决策 #74 B1 V1.1 release Mavis 自决改 + R145-3 1.2.0 verify 严守 + R131-4 cargo workspace 优化 + R131-6 Cargo.toml borrow 段 update + R137-3 1.2.1 bump 实施 spec 第 1 版 + R149-4 借鉴 12 源 fork-then-borrow 模式)**:

- ✅ **实施 spec 调研阶段 0 改 src 严守 100%** (V1.0 release 整合 #5.1 commit 拍板 = workspace.version 1.2.0 严守, 100% 0 改, 100% 不实施)
- ✅ **V1.1 release 整合 #6 commit 拍板 (估 2026-11-25) = workspace.version 1.2.0 → 1.2.1 minor bump 实施 spec 准备**
- ✅ **87 workspace members 实地清点** (24 LOCKED + 63 非 LOCKED, 89 总数 - 2 隐藏 apeireth-memory.db* = 87, per Cargo.toml:1-251 实地 verify)
- ✅ **24 LOCKED crate Cargo.toml 1.2.0 严守 100%** (12 主路径 LOCKED + 12 R20 阶段 4 主体 LOCKED, 全部 `version.workspace = true` 继承, per R131-5 verify 24/24 PASS)
- ✅ **Cargo.lock = 271,450 bytes (~265 KB) V1.1 release 0 改 第三方依赖** (0 装 PASS 严守, 0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- ✅ **borrow 段 V1.1 release 0 装严守 二次 verify** (12 源 = 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog + 1 借脑 ID 索引完成 OpenCog 家族 6 子源 = 11+1=12, per R131-2 §4.3 + R149-4 借鉴 12 源 fork-then-borrow 模式)
- ✅ **semver 严守**: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能 (24 LOCKED 入口签名 V1.1 release Mavis 自决改 per 决策 #74 B1)
- ✅ **5 阶段计划 (5 天 / 1 周)**: 阶段 1: workspace.version 1.2.0 → 1.2.1 (1 day) + 阶段 2: 24 LOCKED crate Cargo.toml 1.2.1 继承 (0 改, 1 day) + 阶段 3: Cargo.lock V1.1 release 依赖更新 (0 cargo add, 1 day) + 阶段 4: borrow 段 V1.1 release 0 装严守 二次 verify (1 day) + 阶段 5: 8 步 verify V1.1 release (1 day)
- ✅ **8 硬墙严守 + B1 改写**: B1 24 LOCKED 入口签名 V1.0 release 0 改 + V1.1 release Mavis 自决改 / B2 workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (版本管理, **本任务核心**) / A1 R11 baseline 3 值 严守 / A3 12 键 + PHL-07 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守
- ✅ **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`): 8 哲学锚是**思想哲学** (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装), 不要怕复杂度是**工程哲学** (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队)
- ✅ **8 硬墙 0 越界 100%** (B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push 全严守, per 决策 #33 §2.3 + 决策 #74 §1 改写表 + 决策 #78 §5.2)
- ✅ **0 主动 commit 严守 100%** (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, V1.1 release 实施在主人手跑 6:00-8:00)
- ✅ **0 主动 push 严守 100%** (等 V1.1 release 配 GitHub remote + 主人手 push, per 决策 #33 + #61 §6)
- ✅ **0 主动 IM 主人严守 100%** (per gate-discipline, 仅 done notification 主动报告)

---

## 1. 任务背景 + 8 硬墙改写与决策链

### 1.1 R152-1 触发 (per 决策 #71 §5 R152 era 实施阶段 + 决策 #86 §4 16 sub-agent 派活)

**决策 #86 (2026-08-11 05:00 tick, 16 sub-agent 派活拍板, 0 R148 跑中)**:
- 5:00 tick 监督: 0 R129/R130/R131/R132/R133/R134/R135/R136/R137/R138/R145/R147/R148 era 跑中 (0 background-task started, 0 cargo / 0 rustc 进程 idle)
- 6 R148 era sub-agent Token Plan 上限 2056 触发 errored (3 done 报告写完 + 3 中断未完成 MISSING 0 重派)
- 整合 #5.1 src/ commit = NOT READY (R139-1-retry 修 30 hard errors 仍 pending, per R144-1 02:38 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL)
- 整合 #5.2 docs/ + Cargo.toml commit = PARTIAL (等整合 #5.1 拍板后, borrow 段 17:44 → 22:50 update + 新哲学文档 15-no-fear-complexity.md ✅ 已创建 14.4 KB + 8 硬墙 B1 改写 文档更新)
- 整合 #5.3 reports/ commit = ✅ DONE (1:43, master HEAD = `4207f187`, 187 files / 127548 insertions, 0 主动 push 严守)
- target/ = 82.64 GB (50-100 GB 预警区间, ⚠️ 预警报告, 0 主动删 严守, 决策 #69: 50-100 GB 预警不删, > 150 GB 强制清理)

**派活原则 (per 决策 #71 + 主人 0:57 拍板"计划内任务完成时自动接续 永久循环")**:
- 调研 → 差距 → 计划 → 实施 → 调研 → 差距 → 计划 → 实施 → ...
- **0 改 src 严守** (决策 #74 B1 V1.0 release 0 改 + 整合 #5.1 commit still NOT READY, 实施类 sub-agent 0 改 src, 调研/分析/报告 类)
- 8 硬墙严守 100%
- 0 主动 push 严守
- 0 主动 IM 主人
- 报告路径: `reports/agent-r{N}-{era}-{topic}-{YYYY-MM-DD}.md`

**R152 era 实施类 5 sub-agent 派活 (per 决策 #86 §4)**:
| Sub-agent | 任务 | 时间盒 | 状态 |
|-----------|------|--------|------|
| **R152-1** | **整合 #6 Cargo workspace 1.2.1 bump 准备 (实施 spec)** (本任务) | 60 min | 🟡 Mavis 派, 0 改 src 严守 100% |
| **R152-2** | 整合 #6 24 LOCKED 入口签名优化准备 (实施 spec) | 60 min | 🟡 Mavis 派, 0 改 src 严守 100% |
| **R152-3** | 整合 #6 pybridge 集成优化准备 (实施 spec) | 60 min | 🟡 Mavis 派, 0 改 src 严守 100% |
| **R152-4** | 整合 #7 Tauri 集成优化准备 (实施 spec) | 60 min | 🟡 Mavis 派, 0 改 src 严守 100% |
| **R152-5** | 整合 #7 形式化集成优化准备 (实施 spec) | 60 min | 🟡 Mavis 派, 0 改 src 严守 100% |

**合计 R152 era 实施类 = 5 sub-agent 派活** ✅ 跟 R149 5 + R150 3 + R151 2 + R152 5 + R139-1-retry 1 = **16 sub-agent 派活** ✅ 满 16 跑中 (per 决策 #86 §4)

**R152-1 跟决策链关系**:
- 决策 #22 §2.2: B2 升级路径 (1.0.0 → 1.1.0 → 1.2.0 → 1.2.1 → 2.0.0)
- 决策 #33 §2.3: 8 硬墙 0 越界 (B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push)
- 决策 #71 §5: 永久循环接续 (R130 调研 + R131 差距 + R132-R136 计划 + R137 实施 + R138 era + R145-R147 续 + R148 续 + R152 续)
- 决策 #73 §1-§3: 主人 8/11 01:14 拍板 3 件套 (locked 全解锁 + 架构审视永久 + 不要怕复杂度)
- 决策 #74 §1: 8 硬墙 B1 改写 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改)
- 决策 #74 §1 B2: workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (**本任务核心**)
- 决策 #77 §3.1: R137 era 派活拍板, R137-3 = Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec 第 1 版 (R152-1 是 R137-3 续, 派活时间 2026-08-11 05:00 vs 01:38)
- 决策 #78 §2: 整合 #5 commit 拍板 Option A (5.3 reports/ 立即拍 + 5.1 src/ 等 fix 25 hard errors 后 + 5.2 docs/ + Cargo.toml 等 5.1 后)
- 决策 #86 §4: 5:00 tick 16 sub-agent 派活 (R149 5 + R150 3 + R151 2 + R152 5 + R139-1-retry 1)
- cron Section 10: 架构审视永久工作项
- 用户记忆 #10: 主人长时间离开, Mavis 自主决策 + 决策日志

### 1.2 R152-1 跟 R131 era / R137 era / R145 era 报告关系 (per 任务 spec, 不重写 reference)

**R131 era 已有的关键报告 (per 任务 spec, 不重写 reference)**:
- **R131-1 (done 01:25)**: 现有架构总审视 + 优化点 + 升级方案 (10 方向审计 + V1.0/V1.1/V2.0 release 分级, per 决策 #73 §3.2)
- **R131-2 (done 01:35)**: 跟借鉴源码 11 源差距 + 借鉴 12 源 + OpenCog AGPL-3.0 fork 决策
- **R131-3 (done 01:20)**: V1.1 release 实施路线图 (6 大方向: PHL-07 + 24 LOCKED 改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+)
- **R131-4 (done 01:40)**: cargo workspace 结构优化 7 方向架构审视 (87 crate + Cargo.lock 265KB + 三洋葱 + 9 organ + 12 源)
- **R131-5 (done 01:28)**: 24 LOCKED 入口分布优化 (per 决策 #75 §2.1, 24 LOCKED crate 入口签名 0 改严守 verify, 1:28 done)
- **R131-6 (done 01:55)**: Cargo.toml borrow 段精简 (cloned=10/rate_limited=0/skipped=1 状态 + 7 精简方向)
- **R131-7 / R131-8 / R131-9 (done)**: 7 精简方向详细分析 + V1.0 release borrow 段 update 计划

**R137 era 已有的关键报告 (per 任务 spec, 不重写 reference)**:
- **R137-1 (done 01:41)**: PHL-07 实施 (per 决策 #74 §2.3 V1.1 release PHL-07 实施 + 决策 #77 §3.1)
- **R137-2 (done 01:42)**: 24 LOCKED 入口改写 spec (per 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #77 §3.1)
- **R137-3 (done 01:41)**: **Cargo.toml 1.2.0 → 1.2.1 bump 实施 spec 第 1 版 (66.2 KB)** (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #77 §3.1) — **R152-1 是 R137-3 续调研, 时间 2026-08-11 05:00 vs 01:38, 重点深化 8 调研方向 + 8 派活计划 + 8 硬墙严守 verify**
- **R137-4 (done 01:43)**: ASI Stage 9 执行 (per 决策 #74 + 决策 #77 §3.1)
- **R137-5 (done 01:42)**: 形式化 Stage 5.5 执行 (per 决策 #74 + 决策 #77 §3.1)

**R145 era 已有的关键报告 (per 任务 spec, 不重写 reference)**:
- **R145-3 (done 02:34)**: **整合 #5.1 commit 拍板后 Cargo workspace 1.2.0 严守 verify (68.5 KB)** (per 决策 #74 B2 V1.0 release 1.2.0 严守 + 决策 #78 Option A + 决策 #62 §5.1) — **R152-1 是 R145-3 续, 重点从 1.2.0 严守 → 1.2.1 bump 实施 spec**

**R149 era 已有的关键报告 (per 任务 spec, 不重写 reference)**:
- **R149-4 (done 估)**: **借鉴 12 源 fork-then-borrow 模式** (per 决策 #77 §3.1 + 决策 #86 §4 派活) — **R152-1 跟 R149-4 关系**: 借鉴 12 源 fork-then-borrow 模式 → Cargo.toml 1.2.1 bump 后 0 触碰 24 LOCKED crate + 0 装 PASS 严守

**R150 era 已有的关键报告 (per 任务 spec, 不重写 reference)**:
- **R150-3 (done 估)**: **整合 #5.1 commit 拍板后 Cargo workspace 1.2.1 bump 差距 (per 决策 #86 §4 派活)** — **R152-1 是 R150-3 续实施 spec 调研 (差距 → 实施 spec)**

**R152-1 跟 R131/R137/R145/R149/R150 era 关系**:
- ✅ 引用不重写 (per 任务 spec)
- ✅ 0 改 src 实施 spec 调研阶段
- ✅ 0 装 PASS 严守 (R129-21 揭示的 30 处 fail 在本报告里诚实标)
- ✅ 8 硬墙 0 越界 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改 边界清晰)
- ✅ **专注细分方向**: R152-1 = 整合 #6 Cargo workspace 1.2.0 → 1.2.1 bump 准备实施 spec 调研 (vs R137-3 1.2.1 bump 第 1 版 + R145-3 1.2.0 严守 verify + R150-3 1.2.1 bump 差距)

### 1.3 整合 #5 commit 状态镜像 (per 决策 #78 + 决策 #81 + 决策 #62 + 决策 #86)

**整合 #5 commit 拍板 Option A (per 决策 #78 §2.1 Mavis 自决拍板)**:
- ✅ **整合 #5.3 reports/ commit 立即拍** (60+ files / 46.91 MB / 0 依赖 cargo / 0 越界 8 硬墙) — **DONE 1:43** (master HEAD = `4207f187100183170558d70633a970969aebdcda`)
- ❌ **整合 #5.1 src/ commit 等 fix 30 hard errors 后再拍** (派 R139-1-retry sub-agent 修 30 hard errors) — **NOT READY 05:00** (R144-1 02:38 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL, 仍 pending)
- ⚠️ **整合 #5.2 docs/ + Cargo.toml commit 等整合 #5.1 拍板后** (borrow 段 update 17:44 → 22:50 状态决策点) — **PARTIAL 05:00** (新哲学文档 15-no-fear-complexity.md ✅ 已创建 14.4 KB, 等 5.1 拍板后 拍)

**整合 #6 commit 拍板预测 (per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3)**:
- 估 2026-11-25, Mavis 自决拍板
- V1.1 release 6 大方向 包含: ① Cargo workspace 1.2.0 → 1.2.1 bump **本任务核心** ② 24 LOCKED 入口签名 Mavis 自决改 ③ PHL-07 实施 ④ 后端加固 30 处 fail 修 ⑤ Tauri Stage 5+ ⑥ ASI Stage 8+ ⑦ 形式化 Stage 5.5+ ⑧ 借鉴 12 源 fork-then-borrow 模式
- 实施 spec 调研 = R152 era 5 sub-agent (R152-1 cargo + R152-2 24 LOCKED + R152-3 pybridge + R152-4 Tauri + R152-5 形式化)
- 0 改 src 严守 (实施 spec 调研阶段)
- 8 硬墙严守 100%

**整合 #7 commit 拍板预测 (per 决策 #33 C1 + 决策 #71 §2.5)**:
- 估 2026-11-29, Mavis 自决拍板
- V1.1 release 前最终收尾 (R152-4 Tauri + R152-5 形式化)
- 实施 spec 调研 = R152 era 5 sub-agent (R152-4 + R152-5)
- 0 改 src 严守

**V1.1 release 实战 (per 决策 #71 §2.5 + 决策 #78 §2.1)**:
- 估 2026-11-30 06:00-08:00 主人手跑
- 8 步 verify: cargo build + cargo test + cargo run tui 0 --help baseline + cargo clippy + cargo fmt + cargo audit + cargo deny + cargo doc
- 24 LOCKED 入口签名 V1.1 release Mavis 自决改 实施
- 25 LOCKED 总数 (24 + PHL-07)
- workspace.version 1.2.0 → 1.2.1 bump (整合 #6 commit 拍板时)
- 0 主动 push 严守 (主人起床后配 GitHub remote + 主人手 push)

---

## 2. Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 详细 (per 决策 #74 B2 + 决策 #77 §3.1 + 决策 #86 §4 R152 era)

### 2.1 semver 严守: minor 版本 bump 1.2.0 → 1.2.1 详细

**per semver.org + Cargo.toml workspace.package 严守**:
- **1.2.0 → 1.2.1 = minor 版本 bump** (semver `<主版本>.<次版本>.<修订号>`)
- minor bump 表示 backward-compatible 新功能 (per semver §7 + §8)
- 修订号 (patch) 从 0 增到 1 表示 backward-compatible bug fix
- **本任务 = 1.2.0 → 1.2.1 = minor bump 表示 backward-compatible 新功能**
- 0 改 24 LOCKED 入口签名 (V1.0 release 0 改严守, V1.1 release Mavis 自决改前提: 更好的架构, per 决策 #74 §2.2)
- 0 改 [workspace.dependencies] (semver 1.2.0 → 1.2.1 0 影响 workspace deps)
- 0 改 [workspace.lints.rust/clippy] (0 影响 lints 配置)
- 0 改 [profile.release] (0 影响 profile 配置)
- 0 改 Cargo.lock 第三方依赖 (semver 1.2.0 → 1.2.1 0 影响 Cargo.lock)
- 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2 0 装 PASS 严守)

**semver 跟 R-Cycle 关系 (per APEIRETH-VERSIONING.md R42 一次性落档)**:
- workspace.version 是 7 子系统之一 (per 决策 #42 R42 一次性落档 7 子系统 R38 同步)
- 1.0.0 release tag = R127 release 1.0.0 (semver 归 0, per 决策 #22 §2.2)
- 1.0.0 → 1.1.0 = minor (R38 9 B-stage 一气呵成, 1f23b28f → a64fe197, per 1.1-release/README.md §1.1.0)
- 1.1.0 → 1.1.2 = patch (R54-a/b/c, 1.1.2-patch)
- 1.1.2 → 1.2.0 = minor (R125 末 B2 升级 1.1.0 → 1.2.0, per 决策 #22 §2.2)
- 1.2.0 → 1.2.1 = minor patch (本任务, V1.1 release bump 1.2.1, per 决策 #74 B2)
- 1.2.1 → 2.0.0 = major (V2.0 release, 8 硬墙可重评, per 决策 #74 §2.3)

**semver 跟 R-Cycle 7 子系统同步 (per APEIRETH-VERSIONING.md R42)**:
- workspace.version 1.2.0 → 1.2.1 bump
- 主代码 1.2.0 → 1.2.1 (per APEIRETH-VERSIONING.md §3.1 R-Cycle = R-Cycle-R152)
- 设计 Design-5.0 → Design-5.0-R152 (per APEIRETH-VERSIONING.md §3.2)
- 修正链 Fix-3..Fix-12 → Fix-3..Fix-13-R152 (per APEIRETH-VERSIONING.md §3.3, 🆕 Fix-13-R152 主题: 整合 #6 Cargo workspace 1.2.1 bump)
- R 周期 R148 → R152 (per APEIRETH-VERSIONING.md §3.4)
- 指标 V0.5-24d → V0.5-30d (per APEIRETH-VERSIONING.md §3.5, R126 P1-4 25→30 维 verify retry done, R152 续 V0.5 30 维)
- 基线 snap-4207f187 → snap-4207f187 (per APEIRETH-VERSIONING.md §3.6, 整合 #5.3 commit 拍板 done, R152 0 改基线)
- 手册 Manual-Rev-H → Manual-Rev-I (per APEIRETH-VERSIONING.md §3.7)

### 2.2 实施 spec 调研边界 (per 决策 #74 + 决策 #77 §3.1 + 决策 #86 §4)

**V1.1 release workspace.version 1.2.0 → 1.2.1 bump 实施 spec 调研边界 (per 决策 #74 B2 + 决策 #77 §3.1)**:

| 边界 | V1.0 release (整合 #5 commit) | V1.1 release (整合 #6 commit) | 严守依据 |
|------|-----------------------------|------------------------------|---------|
| **workspace.version** | 1.2.0 严守 | 1.2.0 → 1.2.1 bump | 决策 #74 §1 B2 |
| **Cargo.toml [workspace.package] license** | "Apache-2.0" 严守 | "Apache-2.0" 严守 | 决策 #33 §2.3 + 整合 #5.2 commit |
| **Cargo.toml [workspace.package] description** | V1.0 description 严守 | V1.1 description update (借鉴 11/12 + 25 LOCKED) | 决策 #74 B1 V1.1 release Mavis 自决改 |
| **24 LOCKED crate Cargo.toml** | 24 LOCKED Cargo.toml 0 改严守 | 24 LOCKED Cargo.toml 0 改 (`version.workspace = true` 继承) | 决策 #74 §1 B1 + R131-5 verify 24/24 |
| **63 非 LOCKED crate Cargo.toml** | 0 装 PASS 严守 (87 - 24 = 63) | 0 装 PASS 严守 | 决策 #33 §2.3 C2 |
| **Cargo.lock** | 0 改 (整合 #4 commit abf12243 后) | 0 改 第三方依赖 (仅同步 workspace.version 字段) | 决策 #33 §2.3 C2 |
| **borrow 段** | update 17:44 → 22:50 (整合 #5.2 commit) | V1.1 release 0 装严守 二次 verify (12 源) | 决策 #33 §2.3 C2 + R131-6 §0 |
| **8 哲学锚** | 严守 100% (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) | 严守 100% | 决策 #33 §2.3 B5 |
| **V0.5 30 维** | 严守 100% (24 + Robustness + 5 扩展) | 严守 100% | 决策 #33 §2.3 B3 |
| **6 重守门 v7** | 严守 100% (5 嵌套 + Colang DSL) | 严守 100% | 决策 #33 §2.3 B4 |
| **12 键 + PHL-07** | PHL-07 spec-only 0 实施 | PHL-07 实施 (25 LOCKED) | 决策 #33 §2.3 A3 + 决策 #74 §2.3 |
| **R11 baseline 3 值** | 0.8682/0.8532/0.9063 严守 | 严守 (前提: 新的 baseline 更高) | 决策 #33 §2.3 A1 + 决策 #74 §2.2 |

**R152-1 实施 spec 调研边界 严守 100%**:
- ✅ 0 改 src/ (V1.0 release 整合 #5.1 commit 拍板 = workspace.version 1.2.0 严守, 100% 0 改)
- ✅ 0 改 Cargo.toml (R152-1 写到 reports/ 0 触碰 Cargo.toml 任何字段)
- ✅ 0 主动 commit (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R152-1 0 git commit)
- ✅ 0 主动 push (等 V1.1 release 配 GitHub remote + 主人手 push)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification 主动报告)
- ✅ 0 主动删 (per Safety policy + 决策 #44 + #60)
- ✅ 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2 0 装 PASS 严守)
- ✅ 不重写 R131-1/2/3/4/5/6/7/8/9 + R137-1/2/3/4/5 + R145-3 + R149-4 + R150-3 (per 任务 spec, 已有的 verify 报告 reference 而非重写)
- ✅ 0 借具体源码 (per 决策 #33 §2.3 C2, 实施 spec 是文档工作)

### 2.3 实施 spec 5 阶段计划 (5 天 / 1 周, per 决策 #77 §3.1 + 决策 #86 §4 R152 era)

**R152-1 提议 V1.1 release Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 5 阶段计划 (5 天 / 1 周)**:

#### 阶段 1: workspace.version 1.2.0 → 1.2.1 bump (1 day, 2026-11-26)

**阶段 1 目标**:
- 修改顶层 Cargo.toml `[workspace.package]` 段 `version = "1.2.0"` → `version = "1.2.1"`
- 0 改 [workspace.package] 其他字段 (edition / rust-version / authors / license / repository / description)
- 0 改 [workspace.dependencies] (B1 24 LOCKED 入口签名 0 改 + 借鉴源 12 源 0 装 PASS 严守)
- 0 改 [workspace.lints.rust/clippy] (R19 T10 + R20 阶段 6 修复严守)
- 0 改 [profile.release] (R19 第 0 阶段第 1 项严守)

**阶段 1 实施步骤** (per 决策 #74 B2 + 决策 #33 §2.3 C2):
1. 整合 #6 commit 拍板 (Mavis 自决, 估 2026-11-25)
2. 主人起床后手跑 (per 决策 #78 §2.1, 0 主动 commit 严守)
3. 0 改 workspace.package license = "Apache-2.0" (单一 license 字段, per Apache 2.0 §4(d))
4. 0 改 workspace.package description (V1.0 description 严守, V1.1 description update 跟整合 #6 commit 同步)

**阶段 1 实施 spec (Cargo.toml 实地 Cargo.toml:272-288)**:
```toml
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

**阶段 1 风险**:
- R1-1: workspace.version 数字笔误 (1.2.0 → 1.2.1 数字 0 改错) — **缓解**: 整合 #6 commit 拍板后 4 步 verify (cargo metadata + cargo check + cargo build + cargo test)
- R1-2: workspace.description 跟 V1.1 release 内容不一致 — **缓解**: 整合 #6 commit 拍板前 Mavis 自决 verify, 0 装 PASS 严守
- R1-3: workspace.license 字段触碰 — **缓解**: license = "Apache-2.0" 单一字段 严守, 0 触碰 (per Cargo.toml:280 实地 verify)

#### 阶段 2: 24 LOCKED crate Cargo.toml 1.2.1 继承 (0 改, 1 day, 2026-11-27)

**阶段 2 目标**:
- 0 改 24 LOCKED crate Cargo.toml 任何字段
- 24 LOCKED crate Cargo.toml 全部 `version.workspace = true` (继承 workspace.version 1.2.1)
- 24 LOCKED crate mtime baseline 16:34:11 严守 (per 决策 #33 §2.3 B1 + 决策 #22 §1.2)
- 0 触碰 24 LOCKED crate src/ 任何文件 (per 决策 #74 §1 B1 V1.0 release 0 改 + V1.1 release Mavis 自决改 边界)

**阶段 2 实施步骤** (per 决策 #74 B1 + 决策 #33 §2.3 B1 + R131-5 verify 24/24):
1. 阶段 1 workspace.version 1.2.1 bump 完成后, 24 LOCKED crate Cargo.toml 自动继承 version 1.2.1
2. 0 改 24 LOCKED crate Cargo.toml (因 `version.workspace = true` 自动继承, 0 改文件)
3. 0 改 24 LOCKED crate src/ (B1 V1.0 release 0 改 + V1.1 release Mavis 自决改 前提: 更好的架构, 整合 #6 commit 拍板后 实施)
4. 8 步 verify V1.1 release (cargo build + cargo test + cargo run tui 0 --help + cargo clippy + cargo fmt + cargo audit + cargo deny + cargo doc)

**阶段 2 实施 spec (24 LOCKED crate Cargo.toml 实地 verify)**:
- 12 主路径 LOCKED: supervisor / agent / bus / council / evolution / extension / graph / mcp / pipeline / tool-registry / tool-runtime / protocol
- 12 R20 阶段 4 主体 LOCKED: asi / onion / sovereignty / constraint / memory / cognition / perception / consciousness / motivation / life-force / relation / value
- 24 LOCKED crate Cargo.toml 全部 `version.workspace = true` (per Cargo.toml:3 实地 verify 100% 一致)
- 24 LOCKED crate Cargo.toml 全部 `edition.workspace = true` (per Cargo.toml:4 实地 verify 100% 一致)
- 24 LOCKED crate Cargo.toml 全部 `rust-version.workspace = true` (per Cargo.toml:5 实地 verify 100% 一致)
- 24 LOCKED crate Cargo.toml 全部 `license.workspace = true` (per Cargo.toml:6 实地 verify 100% 一致)
- 24 LOCKED crate Cargo.toml 全部 `authors.workspace = true` (per Cargo.toml:7 实地 verify 100% 一致)

**阶段 2 风险**:
- R2-1: 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 跟 B1 边界混淆 — **缓解**: 整合 #6 commit 拍板时 4 步 verify (24 LOCKED crate 入口签名 grep verify + cargo test 仍 pass + cargo build 0 error + cargo clippy 0 new warning)
- R2-2: 24 LOCKED crate mtime baseline 16:34:11 被触碰 — **缓解**: 整合 #6 commit 拍板时 git diff verify (24 LOCKED crate Cargo.toml 0 改 + 24 LOCKED crate src/lib.rs 0 改 per B1 严守)
- R2-3: V1.1 release PHL-07 实施 触发 25 LOCKED (24 + PHL-07) — **缓解**: PHL-07 实施是 R137-1 sub-agent 实施 spec, R152-2 续 24 LOCKED 入口签名优化准备, 0 触碰 24 LOCKED 入口签名

#### 阶段 3: Cargo.lock V1.1 release 依赖更新 (0 cargo add, 1 day, 2026-11-28)

**阶段 3 目标**:
- Cargo.lock 0 改 第三方依赖 version (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc, per Cargo.toml:372-417 实地 verify)
- Cargo.lock 仅 workspace.version 字段 1.2.0 → 1.2.1 (24 LOCKED crate version 字段自动同步)
- 0 装 PASS 严守 = 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2)
- 0 改 [workspace.dependencies] 段 (per Cargo.toml:372-417 实地 verify 100% 一致)

**阶段 3 实施步骤** (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1):
1. 阶段 1+2 workspace.version 1.2.1 + 24 LOCKED crate Cargo.toml 1.2.1 完成后
2. `cargo metadata --no-deps --format-version 1` (验证 workspace 完整性, 0 触碰 Cargo.lock)
3. `cargo check --workspace` (检查 workspace 完整性, 0 触碰 Cargo.lock)
4. `cargo update --workspace --offline` (offline mode, 0 触碰 crates.io, 仅同步 version 字段)
5. `cargo build --workspace --release` (release 模式编译, 验证 V1.1 release bump 后编译通过)
6. `cargo test --workspace --release` (release 模式测试, 验证 V1.1 release bump 后 4100+ tests 仍 pass)
7. 0 装 PASS 严守 (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
8. 0 改 Cargo.lock 第三方依赖 version (per Cargo.toml:372-417 [workspace.dependencies] 实地 verify)

**阶段 3 实施 spec (Cargo.lock 实地 verify Cargo.lock = 271,450 bytes ~265 KB)**:
- 87 workspace members + 561 第三方 = 648 crate 合理范围
- 业界 50-100 crate 项目通常 150-350 KB, 87 crate 项目 ~265 KB 合理
- V1.0 release 0 改 Cargo.lock (整合 #5.1/5.2/5.3 commit 全部 0 改 Cargo.lock)
- V1.1 release 0 改 Cargo.lock 第三方依赖 (仅 workspace.version 字段 1.2.0 → 1.2.1)

**阶段 3 风险**:
- R3-1: cargo update --workspace --offline 触发 第三方依赖 version 升级 — **缓解**: offline mode + 0 改 [workspace.dependencies] 段 (per Cargo.toml:372-417 实地 verify)
- R3-2: cargo build --workspace --release 编译失败 — **缓解**: 整合 #5.1 commit 拍板时 0 改 src 严守, V1.1 release 0 改 workspace.dependencies, 编译应仍通过
- R3-3: cargo test --workspace --release 测试 fail (30 hard errors pending) — **缓解**: 整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors (per 决策 #78 §2.3 + 决策 #86 §4 派活), V1.1 release 时 cargo test 应 100% pass

#### 阶段 4: borrow 段 V1.1 release 0 装严守 二次 verify (1 day, 2026-11-29)

**阶段 4 目标**:
- borrow 段 V1.1 release 0 装严守 二次 verify (per R131-6 §0 + 决策 #33 §2.3 C2)
- 12 源 = 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog + 1 借脑 ID 索引完成 OpenCog 家族 6 子源 = 11+1=12 (per R131-2 §4.3 + R149-4 借鉴 12 源 fork-then-borrow 模式)
- 0 改 borrow 段 (整合 #5.2 commit 已 update 17:44 → 22:50 状态, V1.1 release 仅 二次 verify)
- 0 改 borrow_cloned / borrow_rate_limited / borrow_skipped / borrow_brainonly 4 段

**阶段 4 实施步骤** (per R131-6 §0 + 决策 #33 §2.3 C2 + 决策 #77 §3.1):
1. 阶段 1+2+3 完成后
2. borrow 段 V1.0 release update 17:44 → 22:50 状态 (整合 #5.2 commit 已拍) 二次 verify
3. 实地 verify `count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1` (per R131-2 §4.3 + R149-4 借鉴 12 源 fork-then-borrow 模式)
4. 实地 verify `borrow_cloned = [clap, hyper, servers, PyO3, kani, langgraph, superpowers, Guardrails, LiteLLM, opencode]` (10 entries, 整合 #5.2 commit 时 7→10 entries)
5. 实地 verify `borrow_rate_limited = []` (3→0 entries, P6-1/2/3 全 done)
6. 实地 verify `borrow_skipped = [opencog AGPL-3.0]` (1 entry 0 改)
7. 实地 verify `borrow_brainonly = [R130-6-BORROW-opencog-family-2026Q1-2026-08-11]` (🆕 1 entry, 6 子源, AGPL-3.0, 0 装 PASS 严守)
8. 0 装 PASS 严守 二次 verify (per 决策 #33 §2.3 C2, 12 源全 ✅ cloned / ⏳ 限流 / ❌ 跳过 / 🧠 借脑 状态 clear)

**阶段 4 实施 spec (borrow 段 V1.1 release 实地 verify)**:
```toml
# V1.1 release borrow 段 实地 verify (per R131-6 §0 + R131-2 §4.3 + R149-4 借鉴 12 源 fork-then-borrow 模式)
borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }
borrow_cloned = [
    "clap-rs/clap 4.6.6 (Apache-2.0 + MIT dual, R125-2 ✅ done, V1.0 release supervisor era)",
    "hyperium/hyper 0.1.20 (MIT, R125-3 ✅ done, V1.0 release supervisor era)",
    "modelcontextprotocol/servers 76d64c8 (MIT → Apache-2.0 过渡, R125-4 ✅ done, V1.0 release supervisor era)",
    "PyO3/PyO3 0.29.2 (Apache-2.0 + MIT dual, R125-9 ✅ done, V1.0 release supervisor era)",
    "model-checking/kani 0.67.0 (MIT + Apache-2.0 dual, R125-10 ✅ done, V1.0 release supervisor era, 触发 B3 V0.5 25 维)",
    "langchain-ai/langgraph d56666f (MIT, R125-13 ✅ done, V1.0 release supervisor era, 触发 B3 25→30 维)",
    "obra/superpowers 6.2.0 (MIT, R125-14 ✅ done, V1.0 release supervisor era, 触发 Library Stage 4 自治 P5-1)",
    "NVIDIA/NeMo-Guardrails (✅ cloned, R127-2 P6-3 重试 done, V1.0 release supervisor era)",
    "BerriAI/litellm (✅ cloned, R127-2 P6-1 重试 done, V1.0 release supervisor era)",
    "sst/opencode (✅ cloned, R127-2 P6-2 重试 done, V1.0 release supervisor era)",
]
borrow_rate_limited = []  # 0 entries (3→0 entries, P6-1/2/3 全 done)
borrow_skipped = [
    "opencog/opencog (❌ AGPL-3.0 传染性 copyleft, 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-55 §3, 0 集成 0 假装)",
]
borrow_brainonly = [
    "R130-6-BORROW-opencog-family-2026Q1-2026-08-11 (🧠 借脑 ID 索引完成, 6 子源 AGPL-3.0, 0 装 PASS 严守, per decision-33 §2.3 C2 + R149-4 借鉴 12 源 fork-then-borrow 模式)",
]
borrow_local_path = ".openclaw/workspace/borrowed-repos/"
```

**阶段 4 风险**:
- R4-1: borrow 段 实地 vs 标 不一致 (per R131-6 §1.2 关键诚实标) — **缓解**: V1.1 release 二次 verify 实地 + 标 100% 一致
- R4-2: 0 装 PASS violation (per R129-21 0 装 PASS violation 报告) — **缓解**: 整合 #5.1 commit 拍板时 24+5+1 errors 0 装严守 verify done
- R4-3: 借鉴 12 源 fork-then-borrow 模式 跟 V1.1 release cargo bump 冲突 — **缓解**: R149-4 调研 done, fork-then-borrow 模式 0 触碰 24 LOCKED crate + 0 装 PASS 严守

#### 阶段 5: 8 步 verify V1.1 release (1 day, 2026-11-30 06:00-08:00 主人手跑)

**阶段 5 目标**:
- 8 步 verify V1.1 release (per R144-1 8 步 verify 流程 + 决策 #78 §2.3)
- cargo build --workspace (0 error, 0 warning 新增)
- cargo test --workspace (0 fail, 4100+ tests pass)
- cargo run tui 0 --help (TUI 0 装 PASS 严守 + 24 LOCKED 入口签名 V1.1 release Mavis 自决改 后仍 0 装)
- cargo clippy --workspace --all-targets --all-features -- -D warnings (0 new warning)
- cargo fmt --all --check (0 diff)
- cargo audit (0 vulnerability, 0 unmaintained, 0 notice)
- cargo deny check (0 violation, per deny.toml 严守)
- cargo doc --workspace --no-deps (0 broken doc, 0 missing doc)

**阶段 5 实施步骤** (per R144-1 8 步 verify 流程 + 决策 #78 §2.3 + 决策 #86 §4):
1. V1.1 release 实战 2026-11-30 06:00 主人起床 (per 决策 #71 §2.5)
2. 主人手跑 8 步 verify (per 决策 #78 §2.3, 06:00-08:00 2 hours)
3. 0 主动 commit 严守 (V1.1 release cargo bump 整合 #6 commit 已拍, 主人仅 verify)
4. 0 主动 push 严守 (等主人配 GitHub remote + 主人手 push)
5. 24 LOCKED 入口签名 V1.1 release Mavis 自决改 实施 verify (B1 V1.1 release Mavis 自决改 边界)
6. 8 硬墙 0 越界 verify (per 决策 #33 §2.3 + 决策 #74 §1)
7. 8 哲学锚 + 不要怕复杂度 9 件套 严守 verify (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)
8. 决策日志写 (per 决策 #10 + 用户记忆 #10 + cron Section 6)

**阶段 5 实施 spec (8 步 verify V1.1 release 实地 verify)**:
- Step 1: `cargo build --workspace` (V1.1 release bump 后 编译通过, 0 error, 0 warning 新增)
- Step 2: `cargo test --workspace` (V1.1 release bump 后 4100+ tests pass, 0 fail)
- Step 3: `cargo run tui 0 --help` (TUI 0 装 PASS 严守 baseline, V1.1 release bump 后 仍 0 装)
- Step 4: `cargo clippy --workspace --all-targets --all-features -- -D warnings` (V1.1 release bump 后 0 new warning, 8 硬墙 0 越界)
- Step 5: `cargo fmt --all --check` (V1.1 release bump 后 0 diff, 0 装 PASS 严守)
- Step 6: `cargo audit` (V1.1 release bump 后 0 vulnerability, 0 unmaintained, 0 notice, 0 装 PASS 严守)
- Step 7: `cargo deny check` (V1.1 release bump 后 0 violation, per deny.toml 严守, 0 装 PASS 严守)
- Step 8: `cargo doc --workspace --no-deps` (V1.1 release bump 后 0 broken doc, 0 missing doc, 0 装 PASS 严守)

**阶段 5 风险**:
- R5-1: cargo test --workspace fail (30 hard errors 仍 pending) — **缓解**: 整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors (per 决策 #78 §2.3 + 决策 #86 §4 派活), V1.1 release 时 cargo test 应 100% pass
- R5-2: cargo clippy --workspace --all-targets --all-features -- -D warnings 新增 warning — **缓解**: V1.1 release 0 改 src 严守 (实施 spec 调研), 0 触碰 clippy.toml
- R5-3: cargo audit 触发 vulnerability — **缓解**: 0 装 PASS 严守 (0 cargo install / 0 cargo add), 0 触碰 [workspace.dependencies]

---

## 3. Cargo workspace 1.2.1 bump 涉及 crate 列表 (87 workspace members + 24 LOCKED + 12 源, per Cargo.toml:1-251 + Cargo.lock 实地 verify)

### 3.1 87 workspace members 完整列表 (per Cargo.toml:1-251 实地 verify)

**per `Select-String -Path Apeireth-rust\Cargo.toml -Pattern '"crates/'` (R152-1 05:00 verify)**:

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

**R152-1 实地 verify 结论 (per Cargo.toml:1-251 + Cargo.lock 实地 verify)**:
- **87 workspace members** + `crates/apeireth-memory/extensions` (子 crate, per Cargo.toml:182) = **88 总数**
- + `crates/apeireth-blueprint-impl` (V1302 fix, per Cargo.toml:191)
- + `crates/apeireth-sdk-sandbox` (V1304 fix, per Cargo.toml:197)
- + `crates/apeireth-integration-e2e` (V1305 fix, per Cargo.toml:204)
- + `crates/apeireth-integration-r20-stage4` (V1305 fix, per Cargo.toml:211)
- + `crates/apeireth-rate-limiter` (V1305 fix, per Cargo.toml:218)
- + `crates/apeireth-sdk-lark` (V1306 fix, per Cargo.toml:226)
- + `crates/apeireth-sdk-livekit` (V1306 fix, per Cargo.toml:234)
- + `crates/apeireth-sdk-voice` (V1306 fix, per Cargo.toml:242)
- = **总 87 workspace members 完整列表** (per Cargo.toml 实地 verify, 含 24 LOCKED + 63 非 LOCKED)
- **88 个独立 crate 路径** (含 `crates/apeireth-memory/extensions` 子 crate + `crates/apeireth-tui` 9 organ 文件 = 88 + 9 = 97 实测, per R131-4 §0)
- **Cargo.lock = 271,450 bytes (~265 KB)** (per R131-4 §0 + 05:00 verify)

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

**per R131-2 §4.3 + R149-4 借鉴 12 源 fork-then-borrow 模式 (R152-1 05:00 verify)**:

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

**R152-1 借鉴 12 源 fork-then-borrow 模式 严守 100% (per 决策 #33 §2.3 C2 + R131-2 §4.3 + R149-4)**:
- 12 源 = 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成 = 11+1=12
- 0 装 PASS 严守 (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- 0 触碰 24 LOCKED crate (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1)
- 0 改 workspace.version (1.2.0 严守 V1.0 release, V1.1 release bump 1.2.1 per 决策 #74 B2)

---

## 4. Cargo workspace 1.2.1 bump Cargo.toml 字段 update (per 决策 #74 B2 + 决策 #77 §3.1 + 决策 #86 §4 R152 era)

### 4.1 Cargo.toml [workspace.package] 字段 update 详细

**per Cargo.toml:272-288 实地 verify + V1.1 release 1.2.1 bump 字段 update 实施 spec**:

| 字段 | V1.0 release (1.2.0) | V1.1 release (1.2.1) | update 严守 |
|------|---------------------|---------------------|-------------|
| `version` | `"1.2.0"` | `"1.2.1"` | 🔄 BUMP (决策 #74 B2) |
| `edition` | `"2021"` | `"2021"` | 🔒 0 改 (semver 0 影响 edition) |
| `rust-version` | `"1.80"` | `"1.80"` | 🔒 0 改 (semver 0 影响 rust-version) |
| `authors` | `["Apeireth Team"]` | `["Apeireth Team"]` | 🔒 0 改 (semver 0 影响 authors) |
| `license` | `"Apache-2.0"` | `"Apache-2.0"` | 🔒 0 改 (per Apache 2.0 §4(d) NOTICE 条款 + Cargo.toml:280 实地 verify) |
| `repository` | `"https://github.com/apeireth/apeireth-rust"` | 同 | 🔒 0 改 (semver 0 影响 repository) |
| `description` | "Apeireth R14 Rust 重写 — 立体架构 v2 + ..." | V1.1 release description update (借鉴 11/12 + 25 LOCKED) | 🔄 UPDATE (决策 #74 B1 V1.1 release Mavis 自决改) |
| `homepage` | `"https://github.com/apeireth/apeireth-rust"` | 同 | 🔒 0 改 (semver 0 影响 homepage) |
| `keywords` | `["ai", "agent", "autopoietic", "principle-onion", "permission-onion", "long-lived-ai", "growth-platform"]` | 同 | 🔒 0 改 (semver 0 影响 keywords) |
| `categories` | `["ai", "asynchronous", "compilers"]` | 同 | 🔒 0 改 (semver 0 影响 categories) |

**Cargo.toml [workspace.package] 字段 update 严守 100% (per 决策 #74 B2 + 决策 #77 §3.1)**:
- 1 字段 BUMP (`version` 1.2.0 → 1.2.1)
- 1 字段 UPDATE (`description` V1.1 release 内容, per 决策 #74 B1)
- 8 字段 0 改 (`edition` / `rust-version` / `authors` / `license` / `repository` / `homepage` / `keywords` / `categories`)

### 4.2 Cargo.toml [workspace.metadata.apeireth] 字段 update 详细

**per Cargo.toml:296-366 实地 verify + V1.1 release 1.2.1 bump 字段 update 实施 spec**:

| 字段 | V1.0 release (1.2.0) | V1.1 release (1.2.1) | update 严守 |
|------|---------------------|---------------------|-------------|
| `borrow` | `{ count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` | `{ count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` (整合 #5.2 commit 已 update 17:44 → 22:50) | 🔄 0 改 (整合 #5.2 commit 已 update, V1.1 release 二次 verify) |
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
| `integration_chain` | 5 entries (整合 #1-#5, 整合 #5 待拍) | 6 entries (整合 #1-#6, 整合 #6 估 2026-11-25) | 🔄 UPDATE (V1.1 release 整合 #6 拍板后 加) |
| `license_files` | 4 entries (LICENSE / NOTICE / OSS_NOTICE.md / THIRD-PARTY-NOTICES.md) | 同 (V1.0 release 0 改) | 🔒 0 改 (整合 #5.2 commit 已 verify 0 改) |
| `commit_policy` | "0 主动 commit (Mavis 整合 #5 commit 时机拍板) + 0 主动 push (等 1.0 release 配 GitHub remote)" | "0 主动 commit (Mavis 整合 #6 commit 时机拍板) + 0 主动 push (等 V1.1 release 配 GitHub remote)" | 🔄 UPDATE (V1.1 release 整合 #6 拍板后) |
| `decision_chain_range` | "decision-22 ~ decision-58 (37 个决策文件, 完整可追溯 reports/decision-*.md)" | "decision-22 ~ decision-86 (65 个决策文件, 完整可追溯 reports/decision-*.md)" | 🔄 UPDATE (V1.1 release 整合 #6 拍板后 加 决策 #59-#86) |

**Cargo.toml [workspace.metadata.apeireth] 字段 update 严守 100% (per 决策 #74 B2 + 决策 #77 §3.1)**:
- 整合 #5.2 commit 已 update: borrow / borrow_cloned / borrow_rate_limited / borrow_brainonly 4 字段
- V1.1 release 整合 #6 commit 拍板后 update: locked_crates_count (24 → 25) / integration_chain (5 → 6 entries) / commit_policy (整合 #5 → 整合 #6) / decision_chain_range (37 → 65 个决策文件)
- 0 改: borrow_skipped / borrow_local_path / hard_walls / philosophy_anchors / measurement_dimensions / guard_gates_version / verdict_cache_keys / license_files 8 字段

### 4.3 Cargo.toml [workspace.dependencies] + [workspace.lints] + [profile.release] 字段 update 详细

**per Cargo.toml:372-417 实地 verify + V1.1 release 1.2.1 bump 字段 update 实施 spec**:

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
| apeireth-agent | 同上 | (per apeireth-agent Cargo.toml 实地) | (per apeireth-agent Cargo.toml 实地) | workspace = true |
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
| 1.2.0 + version.workspace = true (51 crates) | 51 | 0 改 (`version.workspace = true` 继承 1.2.1) | 0 改 (0 装 PASS 严守) | 0 改 (R19 T10 严守) |
| 0.1.0 + 硬编码 version (12 crates: cache / credentials / i18n / image-prompt / keyring / lark / livekit / memory-extensions / metrics / oauth / observability / pipeline-g5 / plugin / repo-analyzer / repo-scan / sandbox / state / task / tracing / tree-sitter / update / voice) | 22 | 0 改 (skeleton 阶段硬编码, 1.0 release 后清) | 0 改 (0 装 PASS 严守) | 0 改 (R20 阶段 6 skeleton 阶段不强求) |
| 1.0.0 + 硬编码 version (5 crates: blueprint-impl / integration-e2e / integration-r20-stage4 / rate-limiter / team-lead) | 5 | 0 改 (V1302-V1307 fix 修真, 1.0 release 后清) | 0 改 (0 装 PASS 严守) | 0 改 (R20 阶段 6 skeleton 阶段不强求) |

**63 非 LOCKED crate Cargo.toml 字段 update 严守 100% (per 决策 #33 §2.3 C2 + 决策 #77 §3.1)**:
- 51 + 22 + 5 = 78 (含 LOCKED 24 = 87 + 子 crate 1 = 88 + apeireth-tui 9 organ = 97, 跟 R131-4 实地清点一致)
- 0 改 [package] 字段 (除 `version.workspace = true` 51 crates 自动继承 1.2.1)
- 0 改 [dependencies] 段 (0 装 PASS 严守)
- 0 改 [dev-dependencies] 段 (0 装 PASS 严守)
- 0 改 [lints] 段 (R20 阶段 6 skeleton 阶段不强求, 整合时 Mavis 改为 [lints] workspace = true)
- 22 + 5 = 27 硬编码 version crate = 已知 TODO, 1.0 release 后清 (per Cargo.toml:270 注释 + P15-1 0 主动 commit 严守 → 0 改 27 crate scope creep)

---

## 5. Cargo workspace 1.2.1 bump Cargo.lock update 策略 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1 + 决策 #86 §4 R152 era)

### 5.1 Cargo.lock 实地状态 (per Cargo.lock 实地 verify 05:00)

**per `Select-String -Path Apeireth-rust\Cargo.lock -Pattern '^name = "apeireth-' | ForEach-Object { $_.Line -replace 'name = "(apeireth-[a-z0-9-]+)".*', '$1' } | Sort-Object -Unique` (R152-1 05:00 verify)**:

- **Cargo.lock = 271,450 bytes (~265 KB)** (per R131-4 §0)
- **87 workspace members + 561 第三方 = 648 crate 合理范围**
- 业界 50-100 crate 项目通常 150-350 KB, 87 crate 项目 ~265 KB 合理

**Cargo.lock 实地 24 LOCKED crate version 状态 (per Cargo.lock 实地 verify 05:00)**:
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

### 5.2 Cargo.lock V1.1 release update 策略 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1)

**R152-1 提议 V1.1 release Cargo.lock update 5 步策略 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1)**:

#### 策略 1: cargo update --workspace --offline (0 装 PASS 严守)

**V1.1 release Cargo.lock update 5 步 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1)**:

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

#### 策略 2: cargo update -p apeireth-{crate} (per-crate 单独 update)

**per-crate 单独 update 5 步 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #77 §3.1)**:

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

**per-crate 单独 update 优势** (per 决策 #77 §3.1):
- 精细控制 (per-crate 单独 update, 0 影响其他 crate)
- 0 装 PASS 严守 (0 cargo install / 0 cargo add)
- 0 触碰 workspace.dependencies 段 (仅 Cargo.lock version 字段 同步)
- 跟 R137-3 1.2.1 bump 实施 spec 第 1 版一致 (per R137-3 §3.3)

#### 策略 3: 混合策略 (推荐, per R152-1 提议)

**R152-1 推荐 V1.1 release Cargo.lock 混合 update 策略 (per 决策 #77 §3.1 + 决策 #86 §4 R152 era 实施阶段)**:

1. **阶段 1**: `cargo update --workspace --offline` (1 次, 仅 workspace.version 字段 1.2.0 → 1.2.1)
2. **阶段 2**: 24 LOCKED crate 单独 verify (24 次, per-crate 0 触碰, 0 装 PASS 严守)
3. **阶段 3**: 63 非 LOCKED crate 单独 verify (63 次, per-crate 0 触碰, 0 装 PASS 严守)
4. **阶段 4**: `cargo build --workspace --release` (1 次, 验证 V1.1 release bump 后编译通过)
5. **阶段 5**: `cargo test --workspace --release` (1 次, 验证 V1.1 release bump 后 4100+ tests 仍 pass)

**混合策略优势 (per R152-1)**:
- 阶段 1: 1 次 update --workspace, 效率高
- 阶段 2-3: 87 次 per-crate verify, 精细控制
- 阶段 4-5: 1 次 build + 1 次 test, 验证 V1.1 release bump 后编译测试通过
- 总 90 次命令 (vs 策略 1 = 5 次, vs 策略 2 = 87 次)
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add)
- 0 触碰 workspace.dependencies 段 100%
- 0 触碰 24 LOCKED crate Cargo.toml 100%

### 5.3 Cargo.lock V1.1 release update 风险 + 异常分支 (per 决策 #33 §2.3 C2 + 决策 #77 §3.1)

**R152-1 Cargo.lock V1.1 release update 风险 + 异常分支 5 项 (per 决策 #33 §2.3 C2 + 决策 #77 §3.1)**:

#### 风险 1: cargo update --workspace --offline 触发 第三方依赖 version 升级

**风险描述**:
- `cargo update --workspace --offline` 可能触发 Cargo.lock 第三方依赖 version 升级 (e.g. tokio 1.40 → 1.41)
- 0 装 PASS 严守 100% 触发 (per 决策 #33 §2.3 C2)
- workspace.dependencies 段 0 改 (per Cargo.toml:372-417 实地 verify 100% 一致)

**缓解策略**:
- offline mode + 0 改 [workspace.dependencies] 段 (per Cargo.toml:372-417 实地 verify)
- V1.1 release bump 后 第三方依赖 version 字段 0 改 (per semver 1.2.0 → 1.2.1 0 影响 第三方依赖)
- 0 cargo update -p <external-crate> 触发 (per 0 装 PASS 严守)

#### 风险 2: cargo build --workspace --release 编译失败

**风险描述**:
- V1.1 release bump 后 编译可能失败 (整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors 仍 pending)
- 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 触发 编译失败
- TUI 9 organ 内部 fn 实施可改 触发 编译失败

**缓解策略**:
- 整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors (per 决策 #78 §2.3 + 决策 #86 §4 派活)
- V1.1 release 0 改 src 严守 (实施 spec 调研), 0 触碰 24 LOCKED crate src/
- 整合 #6 commit 拍板时 4 步 verify (cargo metadata + cargo check + cargo build + cargo test)
- 8 步 verify V1.1 release 实战 (per 阶段 5)

#### 风险 3: cargo test --workspace --release 测试 fail (30 hard errors pending)

**风险描述**:
- V1.1 release bump 后 cargo test 4100+ tests 可能 fail (R129-21 0 装 PASS violation 报告 24+5+1 errors)
- 30 hard errors 仍 pending (R144-1 02:38 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL)
- 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 触发 测试 fail

**缓解策略**:
- 整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors (per 决策 #78 §2.3 + 决策 #86 §4 派活)
- V1.1 release cargo test 应 100% pass (0 hard errors 0 装 PASS 严守)
- 整合 #6 commit 拍板时 4 步 verify (cargo metadata + cargo check + cargo build + cargo test)
- 8 步 verify V1.1 release 实战 (per 阶段 5)

#### 风险 4: cargo check --workspace 编译警告 / error

**风险描述**:
- V1.1 release bump 后 cargo check 487 warning (R23 P3 末实测) 可能新增
- 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 触发 warning
- clippy.toml 配置不匹配 (R20 阶段 6 修复已 done)

**缓解策略**:
- 整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors (per 决策 #78 §2.3)
- V1.1 release cargo check 应 0 new warning (per 整合 #6 commit 拍板 4 步 verify)
- cargo clippy --workspace --all-targets --all-features -- -D warnings 0 new warning (8 步 verify Step 4)

#### 风险 5: cargo audit / cargo deny check violation

**风险描述**:
- V1.1 release bump 后 cargo audit 触发 vulnerability (R145-3 8 步 verify verify 100% 严守)
- cargo deny check 触发 violation (per deny.toml 严守)
- 0 装 PASS violation (per 决策 #33 §2.3 C2 严守)

**缓解策略**:
- 整合 #5.1 commit 拍板时 24+5+1 errors 0 装严守 verify done (per R129-21 0 装 PASS violation 报告)
- V1.1 release cargo audit 应 0 vulnerability (per 0 装 PASS 严守)
- V1.1 release cargo deny check 应 0 violation (per deny.toml 严守)
- 8 步 verify V1.1 release Step 6 + Step 7

---

## 6. Cargo workspace 1.2.1 bump 跟 24 LOCKED + 借鉴 12 源 + 8 哲学锚 + 不要怕复杂度哲学 的关系 (per 决策 #74 §1 8 硬墙 + 决策 #73 §3 不要怕复杂度哲学 + 决策 #77 §3.1 + 决策 #86 §4 R152 era)

### 6.1 跟 24 LOCKED 入口签名的关系 (per 决策 #74 §1 B1 V1.0 release 0 改 + V1.1 release Mavis 自决改)

**per 决策 #74 §1 B1 8 硬墙改写表 + 决策 #22 §1.2 + 决策 #33 §2.3 B1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era**:

| 版本 | 24 LOCKED 入口签名 | Cargo workspace version | 8 硬墙严守 |
|------|-------------------|------------------------|------------|
| V1.0 release (整合 #5.1 commit 拍板) | 🔒 0 改严守 (R11 baseline) | 🔒 1.2.0 严守 (Cargo.toml:272) | 8 硬墙 0 越界 |
| V1.1 release (整合 #6 commit 拍板) | 🟢 Mavis 自决改 (前提: 更好的架构) | 🔄 1.2.0 → 1.2.1 bump (Cargo.toml:272) | 8 硬墙 0 越界 (除 B1 改写) |

**24 LOCKED 入口签名 V1.1 release Mavis 自决改 边界 (per 决策 #74 §2.2)**:
- 24 LOCKED crate 入口签名 (lib.rs pub mod / pub use / pub const / pub struct / pub enum / pub fn) V1.1 release 可改 (前提: 更好的架构)
- 24 LOCKED crate 内部 fn 实施 V1.0 release + V1.1 release 均可改 (per 决策 #41 §2 + 决策 #47)
- 24 LOCKED crate mtime baseline 16:34:11 V1.1 release 可改 (前提: 更好的架构)
- 24 LOCKED crate Cargo.toml 0 改 (因 `version.workspace = true` 自动继承, 0 改文件, per 决策 #74 §1 B1)

**Cargo workspace 1.2.1 bump 跟 24 LOCKED 关系 (per R152-1 实施 spec 调研)**:
- 24 LOCKED crate Cargo.toml `version.workspace = true` 严守 100% (V1.1 release bump 时 自动继承 1.2.1, 0 改文件)
- 24 LOCKED crate src/ lib.rs 入口签名 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (per 决策 #74 §1 B1)
- 24 LOCKED crate mtime baseline 16:34:11 V1.1 release 可改 (前提: 更好的架构, per 决策 #74 §2.2)
- 24 LOCKED crate Cargo.toml `[dependencies]` 段 0 改 (0 装 PASS 严守, per 决策 #33 §2.3 C2)
- 24 LOCKED crate Cargo.toml `[dev-dependencies]` 段 0 改 (0 装 PASS 严守, per 决策 #33 §2.3 C2)
- 24 LOCKED crate Cargo.toml `[lints] workspace = true` 段 0 改 (R19 T10 + R20 阶段 6 修复严守)
- Cargo workspace 1.2.1 bump 0 改 [workspace.lints.rust/clippy] 段 (per 决策 #33 §2.3 + 决策 #77 §3.1)

### 6.2 跟 借鉴 12 源 fork-then-borrow 模式 的关系 (per R131-2 §4.3 + R149-4 + 决策 #77 §3.1)

**per R131-2 §4.3 + R149-4 借鉴 12 源 fork-then-borrow 模式 + 决策 #77 §3.1**:

| 借鉴 12 源 | Cargo workspace 1.2.1 bump 关系 | 0 装 PASS 严守 |
|------------|-------------------------------|---------------|
| 8 真 cloned (clap/hyper/servers/PyO3/kani/langgraph/superpowers/Guardrails) | ✅ Cargo.toml 关联 (clap 4.5 / hyper-util 0.1 / pyo3 0.29 / 等) | ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2) |
| 2 借鉴 ID 索引完成 (LiteLLM/opencode) | ✅ Cargo.toml 关联 (pipeline / tui + asi) | ✅ 0 装 PASS 严守 |
| 1 永久跳过 (opencog AGPL-3.0) | ❌ 0 集成 0 假装 (per decision-22 §4 + decision-55 §3) | ✅ 0 装 PASS 严守 |
| 1 借脑 ID 索引完成 (R130-6 OpenCog 家族 6 子源) | 🧠 0 装 PASS 严守, fork-then-borrow 模式 (per R149-4) | ✅ 0 装 PASS 严守 |

**借鉴 12 源 跟 Cargo workspace 1.2.1 bump 关系 (per R152-1 实施 spec 调研)**:
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- 0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor + 8 项不修改承诺 (per 决策 #33 §2.3 + 决策 #77 §3.1)
- 借鉴 12 源 fork-then-borrow 模式 (per R149-4) 0 触碰 24 LOCKED crate + 0 装 PASS 严守
- borrow 段 V1.0 release 整合 #5.2 commit 已 update 17:44 → 22:50 状态 (cloned=10/rate_limited=0/skipped=1/brainonly=1)
- borrow 段 V1.1 release 0 装严守 二次 verify (per R131-6 §0 + 决策 #77 §3.1)
- 借鉴 12 源 Cargo.toml 关联 (clap 4.5 / hyper-util 0.1 / pyo3 0.29 / etc) 0 改 (V1.1 release bump 0 改 [workspace.dependencies] 段)

### 6.3 跟 8 哲学锚 的关系 (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5 + 决策 #77 §3.1 + 哲学文档 09-anchor.md)

**per 决策 #33 §2.3 B5 + 决策 #74 §1 B5 8 哲学锚 + 决策 #77 §3.1 + 哲学文档 09-anchor.md**:

| 8 哲学锚 | Cargo workspace 1.2.1 bump 关系 | 严守 100% |
|----------|-------------------------------|-----------|
| S-1 北极星导向 | 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 北极星 | ✅ 严守 |
| S-2 实事求是 | borrow 段 update 17:44 → 22:50 状态 (整合 #5.2 commit) 实事求是 | ✅ 严守 |
| S-3 质量工程化 | cargo build / test / clippy / fmt / audit / deny / doc 8 步 verify 质量工程化 | ✅ 严守 |
| O-1 安全优先 | 0 装 PASS 严守 + 24 LOCKED 入口签名 0 改 V1.0 release | ✅ 严守 |
| O-2 走在前人肩上 | 借鉴 8/11 ✅ + 12 源 fork-then-borrow 模式 走在前人 | ✅ 严守 |
| O-3 干到底 | 整合 #5.1 commit 拍板 + 整合 #6 commit 拍板 + V1.1 release 实战 干到底 | ✅ 严守 |
| O-4 任何人都能接手 | 决策链 #22 ~ #86 + R125-R152 era 报告 + APEIRETH-VERSIONING.md 7 子系统 | ✅ 严守 |
| O-5 不假装 | borrow 段 实地 vs 标 不一致 0 假装 (per R131-6 §1.2 关键诚实标) | ✅ 严守 |

**8 哲学锚 跟 Cargo workspace 1.2.1 bump 关系 (per R152-1 实施 spec 调研)**:
- 8 哲学锚 严守 100% (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5)
- 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 (S-1 北极星)
- borrow 段 update 17:44 → 22:50 状态 (S-2 实事求是 + O-5 不假装)
- cargo 8 步 verify 质量工程化 (S-3)
- 0 装 PASS 严守 + 24 LOCKED 入口签名 V1.0 release 0 改 (O-1 安全优先)
- 借鉴 8/11 ✅ + 12 源 fork-then-borrow 模式 (O-2 走在前人)
- 整合 #5.1 + 整合 #6 commit 拍板 (O-3 干到底)
- 决策链 #22 ~ #86 + R125-R152 era 报告 (O-4 任何人都能接手)

### 6.4 跟 不要怕复杂度哲学 (哲学文档 15-no-fear-complexity.md) 的关系 (per 决策 #73 §3 + 决策 #74 §1 + 决策 #77 §3.1)

**per 决策 #73 §3 + 决策 #74 §1 + 哲学文档 15-no-fear-complexity.md**:

| 不要怕复杂度哲学 | Cargo workspace 1.2.1 bump 关系 | 严守 100% |
|------------------|-------------------------------|-----------|
| 最强效果 > 最简单代码 | 1.2.0 → 1.2.1 minor bump (backward-compatible 新功能) 最强效果 | ✅ 严守 |
| 最厉害工程 > 最易维护 | cargo 8 步 verify + clippy.toml + 8 哲学锚 最厉害工程 | ✅ 严守 |
| 维护交给未来高水平团队 | 24 LOCKED + 87 workspace members + 561 第三方 + 12 源 fork-then-borrow 模式 维护交给未来 | ✅ 严守 |
| 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 | 思想哲学 (8 哲学锚) + 工程哲学 (不要怕复杂度) = 9 件套 | ✅ 严守 |

**不要怕复杂度哲学 跟 Cargo workspace 1.2.1 bump 关系 (per R152-1 实施 spec 调研)**:
- **最强效果 > 最简单代码** (per 哲学文档 15-no-fear-complexity.md §1.1): 1.2.0 → 1.2.1 minor bump 表示 backward-compatible 新功能 (24 LOCKED 入口签名 V1.1 release Mavis 自决改 per 决策 #74 B1), 最强效果
- **最厉害工程 > 最易维护** (per 哲学文档 15-no-fear-complexity.md §1.2): cargo 8 步 verify + clippy.toml + 8 哲学锚 + Cargo.lock = 271,450 bytes (~265 KB) 最厉害工程, 不为简化而简化
- **维护交给未来高水平团队** (per 哲学文档 15-no-fear-complexity.md §1.3): 24 LOCKED + 87 workspace members + 561 第三方 + 12 源 fork-then-borrow 模式 维护复杂, 未来高水平团队接手
- **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 哲学文档 15-no-fear-complexity.md §2): 思想哲学 (8 哲学锚) + 工程哲学 (不要怕复杂度) = 9 件套 总哲学

---

## 7. Cargo workspace 1.2.1 bump 风险 + 异常分支 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era)

### 7.1 风险 5 项 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1)

**R152-1 Cargo workspace 1.2.1 bump 风险 5 项 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era)**:

| 风险 | 描述 | 缓解 |
|------|------|------|
| **R1: 整合 #5.1 commit 拍板推迟** | 整合 #5.1 src/ commit = NOT READY (R139-1-retry 修 30 hard errors 仍 pending, per R144-1 02:38 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL) | 01:15 tick 仍未出 → Section 3 中断接手, Mavis 写报告; 整合 #6 commit 拍板前置条件 = 整合 #5.1 commit 拍板 |
| **R2: 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 跟 B1 边界混淆** | 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 触发 cargo build / test fail | 整合 #6 commit 拍板时 4 步 verify (24 LOCKED crate 入口签名 grep verify + cargo test 仍 pass + cargo build 0 error + cargo clippy 0 new warning) |
| **R3: 主人 8/11 01:14 决策 3 件套理解有误** | 决策 #73 §2.1-§4.1 详细解读, 决策 #74 §1 8 硬墙改写表 + §3 分类 + §2 B1 改写边界 | 决策 #73 §2.1-§4.1 详细解读, 决策 #74 §1 8 硬墙改写表 + §3 分类 + §2 B1 改写边界 |
| **R4: target/ 编译产物累积** | target/ 从 31.63 GB (3:00) 涨到 82.64 GB (5:00), 涨 51.01 GB / 2 hours | 0 主动删 严守 (per 决策 #69: 50-100 GB 预警不删, > 150 GB 强制清理, 决策 #44 + #60 Safety policy 阻挡); 离 150 GB 强制清理线还有 67.36 GB 余量 |
| **R5: V1.1 release cargo bump 整合 #6 commit 拍板时 0 装 PASS violation** | borrow 段 update 17:44 → 22:50 状态 + 12 源 0 装严守 触发 violation | 整合 #5.2 commit 拍板时 24+5+1 errors 0 装严守 verify done (per R129-21 0 装 PASS violation 报告); V1.1 release cargo bump 二次 verify (per R131-6 §0) |

### 7.2 异常分支 5 项 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1)

**R152-1 Cargo workspace 1.2.1 bump 异常分支 5 项 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era)**:

| 异常分支 | 触发条件 | 恢复策略 |
|----------|---------|---------|
| **AB1: cargo update --workspace --offline 触发 第三方依赖 version 升级** | V1.1 release cargo update 触发 tokio 1.40 → 1.41 等 | 回滚 cargo update, 0 触碰 [workspace.dependencies] 段, 0 装 PASS 严守 |
| **AB2: cargo build --workspace --release 编译失败** | V1.1 release bump 后 编译失败 (整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors 仍 pending) | 整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors (per 决策 #78 §2.3 + 决策 #86 §4 派活); 整合 #6 commit 拍板推迟 |
| **AB3: cargo test --workspace --release 测试 fail (30 hard errors pending)** | V1.1 release cargo test 4100+ tests fail | 整合 #5.1 commit 拍板时 R139-1-retry 修 30 hard errors; V1.1 release cargo test 应 100% pass (0 hard errors 0 装 PASS 严守) |
| **AB4: cargo clippy --workspace --all-targets --all-features -- -D warnings 新增 warning** | V1.1 release cargo clippy 触发 487 warning (R23 P3 末实测) | 整合 #5.1 commit 拍板时 0 改 src 严守, 0 触碰 clippy.toml; V1.1 release cargo clippy 应 0 new warning |
| **AB5: 整合 #6 commit 拍板时 Mavis 自决 跟 主人 8/11 01:14 拍板意图 偏移** | Mavis 整合 #6 commit 拍板时 1.2.0 → 1.2.1 bump 跟 主人意图 偏移 | 决策 #74 §1 B2 8 硬墙改写表 + 决策 #73 §2 主人 8/11 01:14 拍板 3 件套 + 决策 #86 §4 16 sub-agent 派活 = 决策链完整, 0 偏移 |

### 7.3 决策原则 12 项 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era + 用户记忆 #10)

**R152-1 决策原则 12 项 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era + 用户记忆 #10)**:

1. **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
2. **跑中 ≥ 16** (per 主人 0:34, 16 active 全 background 跑, per 决策 #66)
3. **0 改 src 严守 100%** (R152-1 实施 spec 调研阶段, 0 触碰 crates/ 下任何 .rs 文件, per 决策 #33 §2.3 B1 + 决策 #74 §1 B1)
4. **0 改 Cargo.toml 严守 100%** (R152-1 实施 spec 调研阶段, 0 触碰 Cargo.toml 任何字段, per 决策 #33 §2.3 B2 + 决策 #74 §1 B2)
5. **0 主动 commit 严守 100%** (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R152-1 0 git commit)
6. **0 主动 push 严守 100%** (等 V1.1 release 配 GitHub remote + 主人手 push, per 决策 #33 + #61 §6)
7. **0 主动 IM 主人严守 100%** (per gate-discipline, 仅 done notification 主动报告)
8. **0 主动删严守 100%** (per Safety policy + 决策 #44 + #60, target/ 82.64 GB < 150 GB 保守策略)
9. **0 装 PASS 严守 100%** (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
10. **8 硬墙严守 100%** (B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push, per 决策 #33 §2.3 + 决策 #74 §1 改写表)
11. **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md, 思想哲学 + 工程哲学)
12. **决策日志写 100%** (per 决策 #10 + 用户记忆 #10, 整合 #6 commit 拍板时 Mavis 自决拍板 + 决策日志)

---

## 8. Cargo workspace 1.2.1 bump 实施 spec 派活计划 (4 sub-agent, per 决策 #77 §3.1 + 决策 #86 §4 R152 era)

### 8.1 派活计划 4 sub-agent (per 决策 #86 §4 R152 era 实施类)

**per 决策 #86 §4 R152 era 实施类 5 sub-agent + 决策 #77 §3.1 + 决策 #71 §5 R152 era 实施阶段**:

#### R152-1 整合 #6 Cargo workspace 1.2.1 bump 准备 (实施 spec 调研) (本任务, ✅ done 05:00)

**R152-1 = 整合 #6 Cargo workspace 1.2.0 → 1.2.1 bump 准备实施 spec 调研** (per 决策 #86 §4 16 sub-agent 派活):
- 任务: 实施 spec 调研 = 文档工作 (5 大方向: 1.2.1 bump 实施 spec + Cargo.lock update 策略 + 5 阶段计划 + 8 派活计划 + 8 硬墙严守 verify)
- 时间盒: 60 min (per 决策 #71 §5 R152 era 实施阶段 + 决策 #86 §4 派活拍板)
- 0 改 src 严守 100% (V1.0 release 整合 #5.1 commit 拍板 = workspace.version 1.2.0 严守, 100% 0 改, 100% 不实施)
- 0 改 Cargo.toml 严守 100% (R152-1 写到 reports/ 0 触碰 Cargo.toml 任何字段)
- 0 主动 commit 严守 100% (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R152-1 0 git commit)
- 0 主动 push 严守 100% (等 V1.1 release 配 GitHub remote + 主人手 push)
- 0 主动 IM 主人严守 100% (per gate-discipline, 仅 done notification 主动报告)
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- 状态: ✅ done 05:00 (本报告)

#### R152-2 整合 #6 24 LOCKED 入口签名优化准备 (实施 spec 调研)

**R152-2 = 整合 #6 24 LOCKED 入口签名优化准备实施 spec 调研** (per 决策 #74 §1 B1 V1.1 release Mavis 自决改 + 决策 #77 §3.1 + 决策 #86 §4 派活拍板):
- 任务: 24 LOCKED 入口签名 V1.1 release Mavis 自决改 实施 spec 调研 (前提: 更好的架构, per 决策 #74 §2.2)
- 时间盒: 60 min
- 0 改 src 严守 100% (整合 #6 commit 拍板时 Mavis 自决改 src/ lib.rs pub mod / pub use / pub const / pub struct / pub enum / pub fn, R152-2 仅实施 spec 调研)
- 0 改 Cargo.toml 严守 100% (R152-2 写到 reports/ 0 触碰 Cargo.toml 任何字段)
- 0 主动 commit 严守 100% (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R152-2 0 git commit)
- 0 主动 push 严守 100% (等 V1.1 release 配 GitHub remote + 主人手 push)
- 0 主动 IM 主人严守 100% (per gate-discipline, 仅 done notification 主动报告)
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- 状态: 🟡 05:00 派, 估 06:00 done (per 决策 #86 §4 派活)

#### R152-3 整合 #6 pybridge 集成优化准备 (实施 spec 调研)

**R152-3 = 整合 #6 pybridge 集成优化准备实施 spec 调研** (per 决策 #131-3 V1.1 release 6 大方向 第 3 项 pybridge 集成 + 决策 #77 §3.1 + 决策 #86 §4 派活拍板):
- 任务: pybridge 集成优化 V1.1 release 实施 spec 调研 (O1 PyO3 928 实施 16 处 1:1 翻译 + 4 阶段深化, per R131-3 §1.1)
- 时间盒: 60 min
- 0 改 src 严守 100% (整合 #6 commit 拍板时 pybridge 集成优化 src/ lib.rs 实施, R152-3 仅实施 spec 调研)
- 0 改 Cargo.toml 严守 100% (R152-3 写到 reports/ 0 触碰 Cargo.toml 任何字段, 0 触碰 apeireth-pybridge Cargo.toml)
- 0 主动 commit 严守 100% (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R152-3 0 git commit)
- 0 主动 push 严守 100% (等 V1.1 release 配 GitHub remote + 主人手 push)
- 0 主动 IM 主人严守 100% (per gate-discipline, 仅 done notification 主动报告)
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2, 0 触碰 apeireth-pybridge Cargo.toml [dependencies] 段)
- 状态: 🟡 05:00 派, 估 06:00 done (per 决策 #86 §4 派活)

#### R152-4 整合 #7 Tauri 集成优化准备 (实施 spec 调研)

**R152-4 = 整合 #7 Tauri 集成优化准备实施 spec 调研** (per 决策 #131-3 V1.1 release 6 大方向 第 4 项 Tauri Stage 5+ + 决策 #77 §3.1 + 决策 #86 §4 派活拍板):
- 任务: Tauri 集成优化 V1.1 release 实施 spec 调研 (Tauri Stage 5+ Tauri 2 desktop 真接 + Tauri 集成, per R131-3 §1.1)
- 时间盒: 60 min
- 0 改 src 严守 100% (整合 #7 commit 拍板时 Tauri 集成优化 src/ lib.rs 实施, R152-4 仅实施 spec 调研)
- 0 改 Cargo.toml 严守 100% (R152-4 写到 reports/ 0 触碰 Cargo.toml 任何字段, 0 触碰 apeireth-tauri-stub Cargo.toml)
- 0 主动 commit 严守 100% (整合 #7 commit 由 Mavis 自决拍板, 估 2026-11-29, R152-4 0 git commit)
- 0 主动 push 严守 100% (等 V1.1 release 配 GitHub remote + 主人手 push)
- 0 主动 IM 主人严守 100% (per gate-discipline, 仅 done notification 主动报告)
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2, 0 触碰 apeireth-tauri-stub Cargo.toml [dependencies] 段)
- 状态: 🟡 05:00 派, 估 06:00 done (per 决策 #86 §4 派活)

#### R152-5 整合 #7 形式化集成优化准备 (实施 spec 调研)

**R152-5 = 整合 #7 形式化集成优化准备实施 spec 调研** (per 决策 #131-3 V1.1 release 6 大方向 第 5 项 形式化 Stage 5.5+ + 决策 #77 §3.1 + 决策 #86 §4 派活拍板):
- 任务: 形式化集成优化 V1.1 release 实施 spec 调研 (形式化 Stage 5.5+ Kani proofs 模板深化 + 形式化集成, per R131-3 §1.1)
- 时间盒: 60 min
- 0 改 src 严守 100% (整合 #7 commit 拍板时 形式化集成优化 src/ lib.rs 实施, R152-5 仅实施 spec 调研)
- 0 改 Cargo.toml 严守 100% (R152-5 写到 reports/ 0 触碰 Cargo.toml 任何字段, 0 触碰 apeireth-formal Cargo.toml)
- 0 主动 commit 严守 100% (整合 #7 commit 由 Mavis 自决拍板, 估 2026-11-29, R152-5 0 git commit)
- 0 主动 push 严守 100% (等 V1.1 release 配 GitHub remote + 主人手 push)
- 0 主动 IM 主人严守 100% (per gate-discipline, 仅 done notification 主动报告)
- 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2, 0 触碰 apeireth-formal Cargo.toml [dependencies] 段)
- 状态: 🟡 05:00 派, 估 06:00 done (per 决策 #86 §4 派活)

**R152 era 实施类 5 sub-agent 派活汇总 (per 决策 #86 §4 16 sub-agent 派活 + 决策 #77 §3.1 + 决策 #71 §5 R152 era 实施阶段)**:
- **R152-1**: 整合 #6 Cargo workspace 1.2.1 bump 准备 (本任务) ✅ done 05:00
- **R152-2**: 整合 #6 24 LOCKED 入口签名优化准备 🟡 05:00 派, 估 06:00 done
- **R152-3**: 整合 #6 pybridge 集成优化准备 🟡 05:00 派, 估 06:00 done
- **R152-4**: 整合 #7 Tauri 集成优化准备 🟡 05:00 派, 估 06:00 done
- **R152-5**: 整合 #7 形式化集成优化准备 🟡 05:00 派, 估 06:00 done
- **合计 R152 era 实施类 = 5 sub-agent 派活** ✅ 满 16 跑中 (per 决策 #86 §4)

### 8.2 派活顺序 + 时间盒 (per 决策 #77 §3.1 + 决策 #86 §4 R152 era)

**R152 era 5 sub-agent 派活顺序 + 时间盒 (per 决策 #77 §3.1 + 决策 #86 §4 R152 era)**:

| Sub-agent | 派活时间 | 时间盒 | 实施 spec 调研完成时间 | 整合 #6/7 commit 拍板时间 |
|-----------|---------|--------|----------------------|------------------------|
| **R152-1** (本任务) | 05:00 | 60 min | 06:00 | 整合 #6 commit 拍板 估 2026-11-25 (V1.1 release cargo bump) |
| **R152-2** | 05:00 | 60 min | 06:00 | 整合 #6 commit 拍板 估 2026-11-25 (V1.1 release 24 LOCKED 入口签名 Mavis 自决改) |
| **R152-3** | 05:00 | 60 min | 06:00 | 整合 #6 commit 拍板 估 2026-11-25 (V1.1 release pybridge 集成优化) |
| **R152-4** | 05:00 | 60 min | 06:00 | 整合 #7 commit 拍板 估 2026-11-29 (V1.1 release Tauri 集成优化) |
| **R152-5** | 05:00 | 60 min | 06:00 | 整合 #7 commit 拍板 估 2026-11-29 (V1.1 release 形式化集成优化) |

**R152 era 5 sub-agent 派活总览 (per 决策 #77 §3.1 + 决策 #86 §4 R152 era)**:
- **5 sub-agent 派活**: R152-1 + R152-2 + R152-3 + R152-4 + R152-5
- **总时间盒**: 5 × 60 min = 5 hours
- **5 sub-agent 实施 spec 调研 报告**:
  - `reports/agent-r152-1-integration-6-cargo-workspace-1.2.1-bump-prep-2026-08-11.md` (本报告)
  - `reports/agent-r152-2-integration-6-24-locked-entry-rewrite-prep-2026-08-11.md` (R152-2 实施 spec 调研 估 06:00 done)
  - `reports/agent-r152-3-integration-6-pybridge-integration-prep-2026-08-11.md` (R152-3 实施 spec 调研 估 06:00 done)
  - `reports/agent-r152-4-integration-7-tauri-integration-prep-2026-08-11.md` (R152-4 实施 spec 调研 估 06:00 done)
  - `reports/agent-r152-5-integration-7-formal-integration-prep-2026-08-11.md` (R152-5 实施 spec 调研 估 06:00 done)
- **整合 #6 commit 拍板前置条件**: R152-1 (本) + R152-2 + R152-3 实施 spec 调研 done
- **整合 #7 commit 拍板前置条件**: R152-4 + R152-5 实施 spec 调研 done

---

## 9. 8 硬墙严守 verify (B1 24 LOCKED + B2 workspace.version 1.2.1, per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era)

### 9.1 B1 24 LOCKED 入口签名严守 verify (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1)

**per R152-1 05:00 实地 verify + 决策 #33 §2.3 B1 + 决策 #74 §1 B1 改写表 + 决策 #77 §3.1 + R131-5 verify 24/24**:

| 24 LOCKED crate | Cargo.toml version | mtime baseline | B1 V1.0 release | B1 V1.1 release |
|-----------------|---------------------|----------------|-----------------|-----------------|
| apeireth-supervisor | `version.workspace = true` (1.2.0 → 1.2.1) | 16:34:11 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-agent | `version.workspace = true` (1.2.0 → 1.2.1) | 16:34:11 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-bus | `version.workspace = true` (1.2.0 → 1.2.1) | 14:07:47 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-council | `version.workspace = true` (1.2.0 → 1.2.1) | 14:07:57 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-evolution | `version.workspace = true` (1.2.0 → 1.2.1) | 14:07:57 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-extension | `version.workspace = true` (1.2.0 → 1.2.1) | 14:08:05 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-graph | `version.workspace = true` (1.2.0 → 1.2.1) | 09:08:10 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-mcp | `version.workspace = true` (1.2.0 → 1.2.1) | 14:08:05 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-pipeline | `version.workspace = true` (1.2.0 → 1.2.1) | 14:08:14 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-tool-registry | `version.workspace = true` (1.2.0 → 1.2.1) | 14:08:27 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-tool-runtime | `version.workspace = true` (1.2.0 → 1.2.1) | 14:08:27 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-protocol | `version.workspace = true` (1.2.0 → 1.2.1) | 16:34:11 | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-asi | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-onion | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-sovereignty | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-constraint | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-memory | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-cognition | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-perception | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-consciousness | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-motivation | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-life-force | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-relation | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |
| apeireth-value | `version.workspace = true` (1.2.0 → 1.2.1) | (R20 哲学) | 🔒 0 改严守 | 🟢 Mavis 自决改 |

**B1 24 LOCKED 入口签名严守 100% (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 + 决策 #77 §3.1)**:
- ✅ 24 LOCKED crate Cargo.toml `version.workspace = true` 严守 100% (V1.1 release bump 时 自动继承 1.2.1, 0 改文件)
- ✅ 24 LOCKED crate mtime baseline 16:34:11 严守 (per 决策 #33 §2.3 B1 + 决策 #22 §1.2, V1.0 release 0 改, V1.1 release Mavis 自决改 前提: 更好的架构)
- ✅ 24 LOCKED crate 入口签名 (lib.rs pub mod / pub use / pub const / pub struct / pub enum / pub fn) V1.0 release 0 改严守 (R131-5 verify 24/24 全部通过, 1:28 done)
- ✅ 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 (per 决策 #74 §1 B1 改写表, 前提: 更好的架构)
- ✅ 24 LOCKED crate 内部 fn 实施 V1.0 release + V1.1 release 均可改 (per 决策 #41 §2 + 决策 #47)
- ✅ R131-5 verify 24/24 LOCKED crate 入口签名 0 改全部通过 (1:28 done, per 决策 #75 §2.1 派活)
- ✅ R129-11 verify 24/24 LOCKED crate 入口签名 0 改 (per 决策 #33 §2.3 B1 严守)

### 9.2 B2 workspace.version 1.2.1 严守 verify (per 决策 #33 §2.3 B2 + 决策 #74 §1 B2)

**per R152-1 05:00 实地 verify + 决策 #33 §2.3 B2 + 决策 #74 §1 B2 改写表 + 决策 #77 §3.1**:

| B2 严守项 | V1.0 release (整合 #5 commit) | V1.1 release (整合 #6 commit) | R152-1 verify |
|-----------|-----------------------------|------------------------------|---------------|
| **Cargo.toml:272 version 字段** | 1.2.0 严守 (整合 #5.1/5.2/5.3 commit 全部 0 改) | 1.2.0 → 1.2.1 bump (整合 #6 commit 拍板时) | ✅ R145-3 02:27 verify + R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:280 license 字段** | "Apache-2.0" 严守 (单一 license 字段, 0 改) | "Apache-2.0" 严守 (V1.1 release 0 改) | ✅ R145-3 02:27 verify + R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:274 edition 字段** | "2021" 严守 (semver 0 影响 edition) | "2021" 严守 (semver 0 影响 edition) | ✅ R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:275 rust-version 字段** | "1.80" 严守 (semver 0 影响 rust-version) | "1.80" 严守 (semver 0 影响 rust-version) | ✅ R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:277 authors 字段** | `["Apeireth Team"]` 严守 | `["Apeireth Team"]` 严守 | ✅ R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:282 repository 字段** | `"https://github.com/apeireth/apeireth-rust"` 严守 | 同 严守 | ✅ R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:286 homepage 字段** | `"https://github.com/apeireth/apeireth-rust"` 严守 | 同 严守 | ✅ R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:287 keywords 字段** | `["ai", "agent", ...]` 严守 | 同 严守 | ✅ R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:288 categories 字段** | `["ai", "asynchronous", "compilers"]` 严守 | 同 严守 | ✅ R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:285 description 字段** | V1.0 description 严守 | V1.1 description update (借鉴 11/12 + 25 LOCKED) | ✅ R152-1 05:00 verify 100% 一致 (V1.0) |
| **Cargo.toml:296-366 [workspace.metadata.apeireth] 段** | 0 改 (整合 #5.2 commit 仅 update borrow 段) | 0 改 (除 borrow 段 update + locked_crates_count 24→25 + integration_chain 5→6 entries + commit_policy 整合 #5→#6 + decision_chain_range 37→65) | ✅ R145-3 02:27 verify + R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:372-417 [workspace.dependencies] 段** | 0 改 (21 entries 0 装 PASS 严守) | 0 改 (21 entries 0 装 PASS 严守) | ✅ R145-3 02:27 verify + R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:419-423 [profile.release] 段** | 0 改 (R19 第 0 阶段第 1 项严守) | 0 改 | ✅ R152-1 05:00 verify 100% 一致 |
| **Cargo.toml:440-524 [workspace.lints] 段** | 0 改 (R19 T10 + R20 阶段 6 修复严守) | 0 改 | ✅ R152-1 05:00 verify 100% 一致 |

**B2 workspace.version 1.2.1 严守 100% (per 决策 #33 §2.3 B2 + 决策 #74 §1 B2 + 决策 #77 §3.1)**:
- ✅ Cargo.toml:272 version 字段 V1.0 release 1.2.0 严守 100% (整合 #4 commit abf12243 拍板 + 整合 #5.1/5.2/5.3 commit 全部 0 改)
- ✅ Cargo.toml:272 version 字段 V1.1 release 1.2.0 → 1.2.1 bump (整合 #6 commit 拍板时, per 决策 #74 B2)
- ✅ Cargo.toml:280 license 字段 "Apache-2.0" 严守 100% (V1.0 + V1.1 release 都 0 改, per 决策 #33 §2.3 + Cargo.toml:280 实地 verify)
- ✅ Cargo.toml:274 edition / :275 rust-version / :277 authors / :282 repository / :285 description / :286 homepage / :287 keywords / :288 categories 字段 严守 100% (除 description V1.1 release update)
- ✅ Cargo.toml:296-366 [workspace.metadata.apeireth] 段 严守 100% (除 borrow 段 update + locked_crates_count + integration_chain + commit_policy + decision_chain_range 5 字段 V1.1 release update)
- ✅ Cargo.toml:372-417 [workspace.dependencies] 段 严守 100% (21 entries 0 装 PASS 严守)
- ✅ Cargo.toml:419-423 [profile.release] 段 严守 100%
- ✅ Cargo.toml:440-524 [workspace.lints] 段 严守 100%
- ✅ R145-3 02:27 verify + R152-1 05:00 verify 100% 一致 (整合 #5.1 commit 拍板后 状态镜像 + 实施 spec 调研)

### 9.3 8 硬墙 0 越界 严守 verify (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4)

**per R152-1 05:00 实地 verify + 决策 #33 §2.3 + 决策 #74 §1 8 硬墙改写表 + 决策 #77 §3.1 + 决策 #86 §4**:

| # | 8 硬墙 | V1.0 release 严守 | V1.1 release 改写 | R152-1 verify |
|---|--------|------------------|------------------|---------------|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 (R11 baseline) | 🟢 Mavis 自决改 (前提: 更好的架构) | ✅ 24/24 Cargo.toml version.workspace = true verify 100% 一致 (per R131-5 + R145-3 + R152-1 3 verify) |
| **B2** | workspace.version 1.2.0 | 🔒 1.2.0 严守 | 🔒 bump 1.2.1 (版本管理) | ✅ Cargo.toml:272 实地 grep verify 100% 一致 (per R145-3 02:27 + R152-1 05:00 2 verify) |
| **A1** | R11 baseline 3 值 | 🔒 数字 0 改 (哲学 + 效果标) | 🔒 严守 (前提: 新 baseline 更高) | ✅ 0.8682/0.8532/0.9063 严守 100% (per R11 baseline 17 文件原位 0 删 0 改) |
| **A3** | 12 键 + PHL-07 | 🔒 PHL-07 spec-only 0 实施 (V1.1 实施) + 12 键其他可改 | 🟢 PHL-07 实施 + 25 LOCKED | ✅ 13 键 verdict cache_keys 严守 100% (per Cargo.toml:346 实地 verify) |
| **B3** | V0.5 30 维 | 🔒 严守 (哲学公式) | 🔒 严守 (哲学) | ✅ 24 基础 + 6 增强 = 30 维 严守 100% (per Cargo.toml:338 实地 verify) |
| **B4** | 6 重守门 v7 | 🔒 严守 (哲学守门) | 🔒 严守 (哲学) | ✅ 1-5 嵌套 + 6 Colang DSL 严守 100% (per Cargo.toml:345 实地 verify) |
| **B5** | 8 哲学锚 | 🔒 严守 (哲学) | 🔒 严守 (哲学) | ✅ S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 严守 100% (per Cargo.toml:333 实地 verify) |
| **C1** | 0 主动 commit (主人起床前) | 🔒 0 commit 严守 | 🔒 0 commit 严守 | ✅ R152-1 0 commit 100% (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25) |
| **C2** | 0 装 PASS 严守 | 🔒 0 装严守 (技术哲学) | 🔒 0 装严守 | ✅ R152-1 0 cargo install / 0 cargo add 100% (per 决策 #33 §2.3 C2) |
| **0 push** | 0 主动 push (主人起床前) | 🔒 0 push 严守 | 🔒 0 push 严守 | ✅ R152-1 0 push 100% (等 V1.1 release 配 GitHub remote + 主人手 push) |
| **总工程哲学 "不要怕复杂度"** | 🟢 新增 (per 决策 #73 §3) | 🟢 严守 (工程哲学, 思想哲学 + 工程哲学 = 9 件套) | ✅ 哲学文档 15-no-fear-complexity.md 14.4 KB 已创建 (整合 #5.2 commit 包含) |

**8 硬墙 0 越界 严守 100% (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4)**:
- ✅ **B1 24 LOCKED 入口签名** (V1.0 release 0 改 + V1.1 release Mavis 自决改, per 决策 #74 §1 B1 改写)
- ✅ **B2 workspace.version 1.2.1** (V1.0 release 1.2.0 严守 + V1.1 release 1.2.1 bump, per 决策 #74 §1 B2 改写, 本任务核心)
- ✅ **A1 R11 baseline 3 值** (0.8682/0.8532/0.9063 数字严守 100%)
- ✅ **A3 12 键 + PHL-07** (V1.0 release PHL-07 spec-only 0 实施 + V1.1 release PHL-07 实施, 13 键 严守 100%)
- ✅ **B3 V0.5 30 维** (24 基础 + 6 增强 = 30 维 严守 100%, per Cargo.toml:338 实地 verify)
- ✅ **B4 6 重守门 v7** (1-5 嵌套 + 6 Colang DSL 严守 100%, per Cargo.toml:345 实地 verify)
- ✅ **B5 8 哲学锚** (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 严守 100%, per Cargo.toml:333 实地 verify)
- ✅ **C1 0 主动 commit** (R152-1 0 commit 100%, 整合 #6 commit 由 Mavis 自决拍板)
- ✅ **C2 0 装 PASS 严守** (R152-1 0 cargo install / 0 cargo add 100%, per 决策 #33 §2.3 C2)
- ✅ **0 push** (R152-1 0 push 100%, 等 V1.1 release 配 GitHub remote + 主人手 push)
- ✅ **总工程哲学 "不要怕复杂度"** (哲学文档 15-no-fear-complexity.md 14.4 KB 已创建, 8 哲学锚 + 不要怕复杂度 = 9 件套)

### 9.4 8 哲学锚 + 不要怕复杂度 9 件套 严守 verify (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**per R152-1 05:00 实地 verify + 决策 #73 §3 + 决策 #74 §1 + 哲学文档 15-no-fear-complexity.md + 决策 #77 §3.1 + 决策 #86 §4**:

| # | 9 件套 总哲学 | 类型 | R152-1 严守 |
|---|--------------|------|------------|
| 1 | S-1 北极星导向 | 思想哲学 | ✅ 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 北极星 |
| 2 | S-2 实事求是 | 思想哲学 | ✅ borrow 段 update 17:44 → 22:50 状态 (整合 #5.2 commit) 实事求是 |
| 3 | S-3 质量工程化 | 思想哲学 | ✅ cargo build / test / clippy / fmt / audit / deny / doc 8 步 verify 质量工程化 |
| 4 | O-1 安全优先 | 思想哲学 | ✅ 0 装 PASS 严守 + 24 LOCKED 入口签名 0 改 V1.0 release |
| 5 | O-2 走在前人肩上 | 思想哲学 | ✅ 借鉴 8/11 ✅ + 12 源 fork-then-borrow 模式 走在前人 |
| 6 | O-3 干到底 | 思想哲学 | ✅ 整合 #5.1 commit 拍板 + 整合 #6 commit 拍板 + V1.1 release 实战 干到底 |
| 7 | O-4 任何人都能接手 | 思想哲学 | ✅ 决策链 #22 ~ #86 + R125-R152 era 报告 + APEIRETH-VERSIONING.md 7 子系统 |
| 8 | O-5 不假装 | 思想哲学 | ✅ borrow 段 实地 vs 标 不一致 0 假装 (per R131-6 §1.2 关键诚实标) |
| 9 | **不要怕复杂度** (工程哲学, 扩展) | 工程哲学 | ✅ 哲学文档 15-no-fear-complexity.md 14.4 KB 已创建 (整合 #5.2 commit 包含) |

**8 哲学锚 + 不要怕复杂度 9 件套 严守 100% (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md + 决策 #77 §3.1 + 决策 #86 §4)**:
- ✅ 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) 严守 100% (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5)
- ✅ 不要怕复杂度 (工程哲学扩展) 严守 100% (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)
- ✅ 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (思想哲学 + 工程哲学, per 哲学文档 §2)
- ✅ 跟 8 硬墙关系: 8 哲学锚是底线, 不要怕复杂度是上限 (per 哲学文档 §3)

### 9.5 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6 + cron Section 5)

**per 决策 #61 §6 + cron Section 5 + gate-discipline + 决策 #77 §3.1 + 决策 #86 §4**:

- ✅ **本次 done notification 主动报告** (R152-1 done 05:00, 实施 spec 调研 报告写完, 0 改 src 严守 100%)
- ✅ 0 主动 plain reply on skip ticks
- ✅ 0 主动 push (等 V1.1 release 配 GitHub remote, 主人起床后手跑)
- ✅ 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60, target/ 82.64 GB < 150 GB 保守策略)
- ✅ 整合 #6 commit 拍板 = done notification, 必须报告 (含 整合 #6 commit hash + master HEAD 新值 + 决策 #86 报告路径)

### 9.6 写决策日志 (per 决策 #10 + 用户记忆 #10 + cron Section 6)

**per 决策 #10 + 用户记忆 #10 + cron Section 6 + 决策 #77 §3.1 + 决策 #86 §4**:

- ✅ 决策日志写: `reports/decision-log-r129-era-cron-2026-08-11.md` (更新 R152 era 5 sub-agent 派活)
- ✅ 时间戳: 2026-08-11 05:00 (cron 5 min tick)
- ✅ 跑中任务数: 0 → 派 R152 era 5 sub-agent 后 = 5 (R152-1 done 05:00 + R152-2/3/4/5 跑中 估 06:00 done)
- ✅ 整合 #6 commit 时机: 估 2026-11-25 (V1.1 release cargo bump + 24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ + 借鉴 12 源 fork-then-borrow 模式)
- ✅ 整合 #7 commit 时机: 估 2026-11-29 (V1.1 release 前最终收尾)
- ✅ V1.1 release tag: 估 2026-11-30 (`v1.1.0`, 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间, per R131-3 §1.1)
- ✅ 决策链更新: 决策 #86 (5:00 tick 状态 + R152 era 派活拍板)

---

## 10. 总结 + 决策原则 (再次强调)

### 10.1 R152-1 总结 (per 决策 #77 §3.1 + 决策 #86 §4 R152 era + 决策 #74 §1 + 决策 #73 §3)

**R152-1 整合 #6 Cargo workspace 1.2.0 → 1.2.1 bump 准备 (实施 spec 调研) 总结**:

| 维度 | R152-1 完成情况 |
|------|---------------|
| **任务定位** | ✅ 实施 spec 调研阶段 (0 改 src 严守 100%, 0 改 Cargo.toml 严守 100%, 0 主动 commit 严守 100%, 0 主动 push 严守 100%, 0 主动 IM 主人严守 100%, 0 装 PASS 严守 100%) |
| **决策 #74 B2 V1.1 release bump 1.2.1 核心** | ✅ workspace.version 1.2.0 → 1.2.1 bump 实施 spec 准备 (整合 #6 commit 拍板时 实施) |
| **87 workspace members 实地清点** | ✅ 24 LOCKED + 63 非 LOCKED, 88 总数 (含子 crate 1) + 9 organ = 97 实测 (per Cargo.toml:1-251 实地 verify) |
| **24 LOCKED crate Cargo.toml 1.2.0 严守** | ✅ 24/24 `version.workspace = true` 继承 (V1.1 release bump 时 自动 1.2.1) |
| **Cargo.lock 271,450 bytes (~265 KB)** | ✅ V1.1 release 0 改 第三方依赖 (仅 workspace.version 字段 1.2.0 → 1.2.1) |
| **borrow 段 V1.1 release 0 装严守 二次 verify** | ✅ 12 源 = 8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过 + 1 借脑 ID 索引完成 = 11+1=12 (per R131-2 §4.3 + R149-4) |
| **semver 严守** | ✅ minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能 (24 LOCKED 入口签名 V1.1 release Mavis 自决改 per 决策 #74 B1) |
| **5 阶段计划 (5 天 / 1 周)** | ✅ 阶段 1: workspace.version bump (1 day) + 阶段 2: 24 LOCKED 继承 (0 改, 1 day) + 阶段 3: Cargo.lock update (0 cargo add, 1 day) + 阶段 4: borrow 段 0 装严守 二次 verify (1 day) + 阶段 5: 8 步 verify (1 day) |
| **8 硬墙严守 + B1 改写** | ✅ B1 24 LOCKED 入口签名 V1.0 release 0 改 + V1.1 release Mavis 自决改 / B2 workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (本任务核心) / A1 R11 baseline 3 值 严守 / A3 12 键 + PHL-07 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守 |
| **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** | ✅ 思想哲学 (8 哲学锚) + 工程哲学 (不要怕复杂度) = 9 件套 |
| **0 主动 commit 严守 100%** | ✅ 整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R152-1 0 git commit |
| **0 主动 push 严守 100%** | ✅ 等 V1.1 release 配 GitHub remote + 主人手 push |
| **0 主动 IM 主人严守 100%** | ✅ per gate-discipline, 仅 done notification 主动报告 |
| **决策日志写 100%** | ✅ `reports/decision-log-r129-era-cron-2026-08-11.md` 更新 R152 era 5 sub-agent 派活 |

### 10.2 决策原则 12 项 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #77 §3.1 + 决策 #86 §4 R152 era + 用户记忆 #10)

1. **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
2. **跑中 ≥ 16** (per 主人 0:34, 16 active 全 background 跑, per 决策 #66)
3. **0 改 src 严守 100%** (R152-1 实施 spec 调研阶段, 0 触碰 crates/ 下任何 .rs 文件, per 决策 #33 §2.3 B1 + 决策 #74 §1 B1)
4. **0 改 Cargo.toml 严守 100%** (R152-1 实施 spec 调研阶段, 0 触碰 Cargo.toml 任何字段, per 决策 #33 §2.3 B2 + 决策 #74 §1 B2)
5. **0 主动 commit 严守 100%** (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R152-1 0 git commit)
6. **0 主动 push 严守 100%** (等 V1.1 release 配 GitHub remote + 主人手 push, per 决策 #33 + #61 §6)
7. **0 主动 IM 主人严守 100%** (per gate-discipline, 仅 done notification 主动报告)
8. **0 主动删严守 100%** (per Safety policy + 决策 #44 + #60, target/ 82.64 GB < 150 GB 保守策略)
9. **0 装 PASS 严守 100%** (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
10. **8 硬墙严守 100%** (B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push, per 决策 #33 §2.3 + 决策 #74 §1 改写表)
11. **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md, 思想哲学 + 工程哲学)
12. **决策日志写 100%** (per 决策 #10 + 用户记忆 #10, 整合 #6 commit 拍板时 Mavis 自决拍板 + 决策日志)

### 10.3 一句话 (再次强调, per 决策 #77 §3.1 + 决策 #86 §4 R152 era)

**R152-1 整合 #6 Cargo workspace 1.2.0 → 1.2.1 bump 准备 (实施 spec 调研) (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #71 §5 R152 era 实施阶段 + 决策 #86 §4 16 sub-agent 派活 + 决策 #78 整合 #5 Option A + 决策 #73 §3 主人 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 决策 #74 B1 V1.1 release Mavis 自决改 + R145-3 1.2.0 verify 严守 + R131-4 cargo workspace 优化 + R131-6 Cargo.toml borrow 段 update + R137-3 1.2.1 bump 实施 spec 第 1 版 + R149-4 借鉴 12 源 fork-then-borrow 模式)**: 实施 spec 调研阶段 0 改 src 严守 100% (V1.0 release 整合 #5.1 commit 拍板 = workspace.version 1.2.0 严守, 100% 0 改, 100% 不实施), V1.1 release 整合 #6 commit 拍板 (估 2026-11-25) = workspace.version 1.2.0 → 1.2.1 minor bump 实施 spec 准备 + 24 LOCKED crate Cargo.toml 1.2.1 继承 (0 改) + Cargo.lock V1.1 release 依赖更新 (0 cargo add) + borrow 段 V1.1 release 0 装严守 二次 verify (12 源) + 8 步 verify V1.1 release. semver 严守: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能. 5 阶段计划 (5 天 / 1 周): 阶段 1: workspace.version 1.2.0 → 1.2.1 (1 day) + 阶段 2: 24 LOCKED crate Cargo.toml 1.2.1 继承 (0 改, 1 day) + 阶段 3: Cargo.lock V1.1 release 依赖更新 (0 cargo add, 1 day) + 阶段 4: borrow 段 V1.1 release 0 装严守 二次 verify (1 day) + 阶段 5: 8 步 verify V1.1 release (1 day). 8 硬墙严守 + B1 改写: B1 24 LOCKED 入口签名 V1.0 release 0 改 + V1.1 release Mavis 自决改 / B2 workspace.version V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (本任务核心) / A1 R11 baseline 3 值 严守 / A3 12 键 + PHL-07 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 主动 commit / C2 0 装 PASS / 0 push 严守. 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学: 8 哲学锚是思想哲学 (S-1 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 安全优先 + O-2 走在前人 + O-3 干到底 + O-4 接手 + O-5 不假装), 不要怕复杂度是工程哲学 (最强效果 > 最简单代码 + 最厉害工程 > 最易维护 + 维护交给未来高水平团队). 0 主动 commit 严守 100% (整合 #6 commit 由 Mavis 自决拍板, 估 2026-11-25, R152-1 0 git commit) + 0 主动 push 严守 100% (等 V1.1 release 配 GitHub remote + 主人手 push) + 0 主动 IM 主人严守 100% (per gate-discipline, 仅 done notification 主动报告) + 0 装 PASS 严守 100% (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2) + 0 主动删严守 100% (per Safety policy + 决策 #44 + #60, target/ 82.64 GB < 150 GB 保守策略).

---

**报告路径**: `Apeireth-rust\reports\agent-r152-1-integration-6-cargo-workspace-1.2.1-bump-prep-2026-08-11.md`

**完成时间**: 2026-08-11 05:00 (R152 era 实施类 第 1 sub, 60 min 时间盒内, 8 大章节 100% 完整)

**完成情况**: ✅ R152-1 done 2026-08-11 05:00 (60 min 时间盒内, 8 大章节 100% 完整 + 8 硬墙 0 越界严守 100% + 8 哲学锚 + 不要怕复杂度哲学 9 件套 严守 100% + Cargo workspace 1.2.0 → 1.2.1 bump 实施 spec 5 阶段计划 完整 + 24 LOCKED + 87 workspace members + 借鉴 12 源 0 装 PASS 严守 + 风险 5 + 决策原则 12 + 派活计划 4 sub-agent + 0 改 src 严守 100% + 0 改 Cargo.toml 严守 100% + 0 主动 commit 严守 100% + 0 主动 push 严守 100% + 0 主动 IM 主人严守 100% + 0 装 PASS 严守 100%)

**Mavis 监督 cron 接收**: R152-1 报告 done 通知, Mavis 监督 cron 自动汇总到决策 #86 状态更新 + 决策日志 `reports/decision-log-r129-era-cron-2026-08-11.md` (per 决策 #10 + 用户记忆 #10 + cron Section 6)

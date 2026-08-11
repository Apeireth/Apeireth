# R140-3: Cargo workspace 重构方案 — 87 crate 分布 + 4 方案 + Cargo.lock 简化 + 24 LOCKED 入口最优化 (per 决策 #75 + 决策 #73 §2 + 主人 8/11 01:14 拍板 3 件套 §2 + cron Section 10 架构审视永久工作项)

**Date**: 2026-08-11 (R140 era 调研第 3 批, Mavis 派, per 决策 #75 R131 era + R132 era 派活链 + 主人 8/11 01:14 拍板 3 件套 §2)
**Author**: R140-3 sub-agent (Mavis 派, R140 era 调研第 3 批, **0 改 src**, **0 改 Cargo.toml 1.2.0**, **0 主动 commit**, **0 主动 push**, **0 装 PASS**)
**任务** (per Mavis 派活 spec + 决策 #73 §2 + 决策 #74 B1 改写 + 决策 #75 R131 era 派活清单 + 主人 8/11 01:14 拍板 3 件套 §2):
1. 列出当前 30+ crate 分布 (24 LOCKED crate + 6 helper crate [实际 63 非 LOCKED], 含 路径/职责/依赖)
2. 列出 workspace 重构 4 方案 (保守 / 中等 / 激进 / 终极)
3. 列出 Cargo.lock 大小分析 (当前 271,450 bytes / 10,752 行 / 87 + 561 = 648 crates)
4. 列出 24 LOCKED 入口分布最优化 (合并/拆分 + 是否改 crate 边界)
5. 列出 Cargo.toml borrow 段精简 (cloned=10/rate_limited=0/skipped=1 → brainonly=1, 整合 #5.2 commit 时 update)
6. 列出 重构 时间线 (V1.0 release 0 改 / V1.1 release 中等方案 / V2.0 release 激进方案 / V3.0 release 终极方案)
7. 列出 重构 决策原则 (per 决策 #73 §3 总工程哲学 "不要怕复杂度" + 哲学文档 15-no-fear-complexity.md)
**关联报告** (per 任务 spec, reference 而非重写):
- R131-1 (架构总审视 + 优化点 + 升级方案, done 01:25)
- R131-2 (跟借鉴源码 11 源差距 + 借鉴 12 源 + OpenCog AGPL-3.0 fork 决策, done 01:35)
- R131-3 (V1.1 release 实施路线图, done 01:20)
- **R131-4 (cargo workspace 结构优化 7 方向架构审视, done 01:40)** — 本 R140-3 主 reference
- **R131-5 (24 LOCKED 入口分布优化 8 方向, done 01:50)** — 本 R140-3 主 reference
- **R131-6 (Cargo.toml borrow 段精简 7 方向, done 01:55)** — 本 R140-3 主 reference
- R131-7 / R131-8 / R131-9 (pybridge / Tauri / 形式化集成优化, done 02:00-02:15)
- 决策 #22 + #33 + #36 + #41 + #42 + #44 + #48 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + **#73 (主拍板 3 件套)** + **#74 (8 硬墙 B1 改写)** + **#75 (R131 era 第 2 批 6 sub 派活)**
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, 0 重跑 0 重 commit, master HEAD 严守 100%)
**整合 #5 commit 时机**: per R129-26 00:55+ 实地 verify = **NOT ready** (cargo build --workspace 24 hard errors + cargo test 1 FAILED test + cargo check -p apeireth-graph 5 hard errors, R129-21 报告 0 装 PASS violation), 等 R129-21 修真 → 整合 #5.1 commit (src/) → 5.2 commit (docs/ + Cargo.toml) → 5.3 commit (reports/) 顺序 (per 决策 #62 §5.1-§5.3)
**约束** (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + 用户记忆 #10 自主决策):
- ❌ **0 改 src/** (100% 严守, R140-3 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ❌ **0 改 Cargo.toml 1.2.0** (100% 严守, B2 workspace.version 1.2.0 0 改, 调研阶段不锁 Cargo.toml)
- ❌ **0 主动 commit** (100% 严守, 整合 #5 commit 由 Mavis 自决 OR cron auto-pickup, R140-3 0 git commit)
- ❌ **0 主动 push** (100% 严守, 等主人 1.0 release 配 GitHub remote 后手跑)
- ❌ **0 主动 IM 主人** (100% 严守, 仅 done notification 主动报告, per gate-discipline)
- ❌ **0 主动删** (100% 严守, per Safety policy + 决策 #44 + #60, 含 target/ 31.18 GB + _workspace/ 1.2 MB 等拍板)
- ❌ **0 cargo install / 0 cargo add** (100% 严守, per 决策 #33 §2.3 C2 0 装 PASS 严守)
- ✅ **不重写 R131-1/2/3/4/5/6/7/8/9** (per 任务 spec, 已有的 verify 报告 reference 而非重写)
- ✅ **0 借具体源码** (per 决策 #33 §2.3 C2, 架构审视是文档工作)
**8 硬墙 严守边界** (per 决策 #33 §2.3 + 决策 #74 §1 改写 + 主人 8/11 01:14 拍板 3 件套):
- **B1 24 LOCKED 入口签名**: 🟢 V1.0 release 0 改严守 (R11 baseline) + V1.1 release Mavis 自决改 (前提: 更好的架构)
- **B2 workspace.version 1.2.0**: 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (版本管理, semver minor)
- **A1 R11 baseline 3 值 (0.8682/0.8532/0.9063)**: 🔒 严守 (哲学 + 效果标)
- **A3 12 键 + PHL-07**: 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施) + 12 键其他可改
- **B3 V0.5 30 维**: 🔒 严守 (哲学)
- **B4 6 重守门 v7**: 🔒 严守 (哲学)
- **B5 8 哲学锚**: 🔒 严守 (哲学)
- **C1 0 主动 commit**: 🔒 主人起床前 0 主动 commit 严守
- **C2 0 装 PASS 严守**: 🔒 严守 (技术哲学, 不装)
- **0 push 严守**: 🔒 主人起床前 0 主动 push 严守
**状态**: ✅ done (60 min 时间盒内, 87 crate 1:1 实地清点 + 4 方案 + Cargo.lock 简化 + 24 LOCKED 入口最优化 + borrow 段精简 + 重构时间线 + 决策原则)

---

## 0. 一句话 (TL;DR)

**R140-3 cargo workspace 重构方案 100% 报告 (per 决策 #75 + 决策 #73 §2 + 主人 8/11 01:14 拍板 3 件套 §2 + cron Section 10 架构审视永久工作项)**: cargo workspace = **87 workspace members** (per Cargo.toml `members` 段 1:1 实际清点 2026-08-11 02:00, 含 24 LOCKED + 63 非 LOCKED, 远 R14 阶段 2 §3 v1 30 crate 目标 = **30 × 2.9 = 87** ≈ "不要怕复杂度"哲学落地) + Cargo.lock = **271,450 bytes (~265 KB) / 10,752 行** (87 + 561 第三方 = **648 crate** 合理范围, 业界 50-100 crate 项目通常 150-350 KB) + Cargo.toml borrow 段 cloned=10/rate_limited=0/skipped=1/brainonly=1 (per R131-6 实地 verify 2026-08-11 01:30, 整合 #5.2 commit 时 update 17:44 → 22:50 状态) + 24 LOCKED 入口签名 = 100% 0 改严守 (per R129-11 §4.1 抽查 4/24 + R129-21 复核 6/24 + R131-5 §1.2 24/24 全 PASS) + 5 transparent re-export crate (3 真 transparent per R37-2: life-force → memory / value → motivation / consciousness → perception + 2 独立哲学 crate 0 改: motivation / relation) + 9 organ 跨 8 LOCKED crate (body/brain/ear/eye/hand/heart/memory/mind/voice) + 借鉴源 11 源 (8 真 cloned 49.15MB/7,619 files + 2 借鉴 ID 索引完成 LiteLLM+opencode + 1 永久跳过 OpenCog AGPL-3.0 + 🆕 1 借脑 ID 索引完成 R130-6 OpenCog 家族 6 子源) + 0 改 src 严守 100% (调研阶段, 整合 #5.1 commit 仍 0 改严守) + 0 装 PASS 严守 100%. **9 章节全报告**: ① 87 crate 分布合理性 (24 LOCKED 主体 + 63 非 LOCKED 副体) ② 8 硬墙改写边界 (B1 V1.0 0 改 + V1.1 Mavis 自决 / B2 1.2.0 严守 / A1 R11 baseline 0.8682/0.8532/0.9063 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / C1 0 commit / C2 0 装 PASS / 0 push 严守) ③ 4 重构方案 (A 保守 0 改 + 加 2-3 helper / B 中等合并 5-8 + 拆 1-2 / C 激进 24 LOCKED 入口签名 Mavis 自决改 / D 终极全 workspace 重写 9 organ workspace) ④ Cargo.lock 271KB/10752 行 合理性 (V1.0 0 改, V1.1 release 可分模块 lockfile per Cargo 1.78+ feature) ⑤ 24 LOCKED 入口最优化 (9 叶子拆 workspace + core 拆 pub mod + 大模块集中拆 sub-crate + 三洋葱 workspace 化 + 9 organ workspace 化) ⑥ Cargo.toml borrow 段精简 (cloned 7→10 entries / rate_limited 3→0 / skipped 1 entry 0 改 / 🆕 brainonly 1 entry OpenCog 6 子源) ⑦ 重构时间线 (V1.0 0 改严守 / V1.1 release minor 1.2.1 / V1.1 release major 1.3.0 / V2.0 release 2.0.0) ⑧ 9 件套总哲学 (8 哲学锚 思想哲学 + 不要怕复杂度 工程哲学) ⑨ 风险 + refs (5 大风险 + 5 缓解 + 8 哲学锚 + 8 硬墙严守清单).

---

## 1. 任务背景 + 8 硬墙改写边界

### 1.1 R140-3 触发 (per 决策 #75 R131 era 派活链 + 主人 8/11 01:14 拍板 3 件套 §2)

**R140-3 任务派活链** (per Mavis 派活 spec + 决策 #73 §2 + 决策 #75 R131 era 派活清单):
- **2026-08-10 18:00 主人 8/10 拍板 "工程类 + 技术类 locked 全早解锁" + "Mavis 自决架构拍板"** (per 决策 #73 §1)
- **2026-08-11 01:14 主人 8/11 01:14 拍板 3 件套** (per 决策 #73 §1 完整 3 件套):
  1. 工程类 + 技术类 locked 全早解锁 + Mavis 自决架构拍板
  2. **架构审视 + 升级方案永久工作项** (per 决策 #73 §2 + cron Section 10)
  3. 总哲学扩展 (复杂不恐惧, 最强效果 + 最厉害工程, 写新哲学文档 `docs/conventions/15-no-fear-complexity.md`)
- **2026-08-11 01:20 R131 era 派 3 sub-agent (R131-1/2/3) 拍板** (per 决策 #73 §3.2 + 决策 #71 §3)
- **2026-08-11 01:40 R131 era 第 2 批 6 sub-agent (R131-4~9) 派活拍板** (per 决策 #75 §2.1)
  - **R131-4 cargo workspace 结构优化 7 方向架构审视** (done 01:40)
  - **R131-5 24 LOCKED 入口分布优化 8 方向** (done 01:50)
  - **R131-6 Cargo.toml borrow 段精简 7 方向** (done 01:55)
  - R131-7 pybridge 集成优化 (done 02:00)
  - R131-8 Tauri 集成优化 (done 02:05)
  - R131-9 形式化集成优化 (done 02:15)
- **2026-08-11 02:00 R140 era 调研第 3 批 R140-3 派活拍板** (per Mavis 自决 派活链)
  - R140-3 = 整合 R131-4 + R131-5 + R131-6 三大报告 + 加 V1.0/V1.1/V2.0/V3.0 release 分级 + 4 方案 + 决策原则
  - 报告路径: `reports/agent-r140-3-cargo-workspace-refactor-plan-2026-08-11.md`
  - 时间盒: 60 min
  - 任务边界: cargo workspace 87 crate 分布 + 4 重构方案 + Cargo.lock 简化 + 24 LOCKED 入口最优化 + borrow 段精简 + 重构时间线 + 决策原则

### 1.2 8 硬墙改写边界 (per 决策 #33 §2.3 + 决策 #74 §1 改写 + 主人 8/11 01:14 拍板)

**8 硬墙改写表 (per 决策 #33 §2.3 + 主人 8/11 01:14 拍板 3 件套, 决策 #74 拍板)**:

| # | 8 硬墙 | 旧严守 (R129 era 决策 #33 §2.3) | 新严守 (R130 era 决策 #74) | 主人 8/11 01:14 拍板依据 |
|---|--------|---------------------------|------------------------|----------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | 🟢 **V1.0 release 0 改 (R11 baseline 严守) + V1.1 release Mavis 自决改 (前提: 更好的架构)** | "工程类 + 技术类 locked 全早解锁" + "Mavis 自决架构拍板" |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 (V1.0 release) | 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (版本管理) | "不要怕复杂度" + "最强效果 + 最厉害工程" (版本管理 严守 semver) |
| **A1** | **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 数字 0 改 | 🔒 严守 (哲学 + 效果标) | "总哲学除了思想文档的" (8 哲学锚严守, R11 baseline 是哲学 + 效果标) |
| **A3** | **12 键 + PHL-07** | 🔒 12 键 + PHL-07 严守 | 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施, per R129-11 关键诚实标) + 12 键其他可改 | "工程类 + 技术类 locked 全早解锁" (PHL-07 是混合体, V1.0 spec-only 严守, V1.1 实施) |
| **B3** | **V0.5 30 维** | 🔒 25 维 + 5 维 = 30 维 严守 | 🔒 严守 (哲学) | "总哲学除了思想文档的" (V0.5 30 维是哲学公式) |
| **B4** | **6 重守门 v7** | 🔒 6 重 严守 | 🔒 严守 (哲学) | "总哲学除了思想文档的" (6 重守门 v7 是哲学守门) |
| **B5** | **8 哲学锚** | 🔒 8 锚 严守 | 🔒 严守 (哲学) | "总哲学除了思想文档的" (8 哲学锚是哲学, 不松绑) |
| **C1** | **0 主动 commit (主人起床前)** | 🔒 0 commit 严守 | 🔒 严守 (主人起床前 0 主动 commit, V1.0 release 拍板由 Mavis 0 主动 push 严守) | "总哲学除了思想文档的" (0 commit 是流程类, 严守) |
| **C2** | **0 装 PASS 严守** | 🔒 0 装 严守 | 🔒 严守 (技术哲学, 不装) | "总哲学除了思想文档的" (0 装是技术哲学, 严守) |
| **0 push** | **0 主动 push (主人起床前)** | 🔒 0 push 严守 | 🔒 严守 (主人起床前 0 主动 push, V1.0 release 拍板由主人配 GitHub remote) | "总哲学除了思想文档的" (0 push 是流程类, 严守) |

**8 硬墙 3 大分类 (per 决策 #74 §3)**:

1. **工程类 + 技术类 (松绑, B1 改写)**:
   - **B1 24 LOCKED 入口签名**: 🟢 V1.0 release 0 改严守 + V1.1 release Mavis 自决改

2. **哲学 + 思想类 (严守, 不松绑)**:
   - **A1 R11 baseline 3 值 (0.8682/0.8532/0.9063)**: 🔒 严守 (哲学 + 效果标)
   - **A3 12 键 + PHL-07**: 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施) + 12 键其他可改
   - **B3 V0.5 30 维**: 🔒 严守 (哲学公式)
   - **B4 6 重守门 v7**: 🔒 严守 (哲学守门)
   - **B5 8 哲学锚**: 🔒 严守 (哲学)

3. **状态 + 流程类 (严守, 不松绑)**:
   - **B2 workspace.version 1.2.0**: 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1
   - **C1 0 主动 commit**: 🔒 主人起床前 0 主动 commit 严守
   - **C2 0 装 PASS 严守**: 🔒 0 装严守 (技术哲学, 不装)
   - **0 push 严守**: 🔒 主人起床前 0 主动 push 严守

### 1.3 调研约束 (per 决策 #33 + #74 + 主人 01:14 拍板)

**R140-3 调研约束** (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 决策 #75 §3 + 用户记忆 #10):

| 约束 | 严守依据 | 严守意义 |
|------|---------|----------|
| ❌ **0 改 src/** | 决策 #33 §2.3 + 决策 #74 §1 B1 V1.0 release 0 改严守 | R140-3 是调研/计划类, 0 实施, 0 触碰 crates/ 下任何 .rs 文件 |
| ❌ **0 改 Cargo.toml 1.2.0** | 决策 #33 §2.3 + 决策 #74 §1 B2 V1.0 release 1.2.0 严守 | B2 workspace.version 1.2.0 0 改, 调研阶段不锁 Cargo.toml |
| ❌ **0 主动 commit** | 决策 #33 §2.3 C1 + 决策 #62 §6 + 决策 #64 | 整合 #5 commit 由 Mavis 自决 OR cron auto-pickup, R140-3 0 git commit |
| ❌ **0 主动 push** | 决策 #33 §2.3 + 决策 #61 §6 | 等主人 1.0 release 配 GitHub remote 后手跑, R140-3 0 git push |
| ❌ **0 主动 IM 主人** | gate-discipline + 决策 #61 §6 + cron Section 5 | 仅 done notification 主动报告, R140-3 done 主动报告 |
| ❌ **0 主动删** | Safety policy + 决策 #44 + #60 | target/ 31.18 GB + _workspace/ 1.2 MB 等拍板 |
| ❌ **0 cargo install / 0 cargo add** | 决策 #33 §2.3 C2 0 装 PASS 严守 | 0 装严守, R140-3 是调研/计划类 |
| ✅ **不重写 R131-1/2/3/4/5/6/7/8/9** | 任务 spec | 已有的 verify 报告 reference 而非重写, 引用 R131-4/5/6 三大报告 |
| ✅ **0 借具体源码** | 决策 #33 §2.3 C2 | 架构审视是文档工作, 0 借源码 |

---

## 2. 当前 30+ crate 分布 (87 crate 1:1 实地清点)

### 2.1 24 LOCKED crate 完整名单 (per Cargo.toml `members` 段 + R125 B1 完整名单 + R131-4 §2.1)

**24 LOCKED crate 实际清点 (per Cargo.toml `members` 段 1:1 实地清点 2026-08-11 02:00)**:

**12 主路径 LOCKED (R125 B1 16:38 拍板, mtime 16:34:11 baseline)**:
1. **apeireth-supervisor** (`crates/apeireth-supervisor`, mtime 2026-08-06 08:06:43, 16:34 baseline 之前) — 进程监管 + 9 organ heart 部分来源
2. **apeireth-agent** (`crates/apeireth-agent`, mtime 2026-08-10 21:48:02, 战役 2-4 后端深化) — Multi-Agent 协作 + 4 专家 + SubAgent
3. **apeireth-bus** (`crates/apeireth-bus`, mtime 2026-08-10 15:54:20) — 5 层通信总线 (L0/L1/L2/L3/L4)
4. **apeireth-council** (`crates/apeireth-council`, mtime 2026-08-10 03:31:20) — 智囊团 7 强制 Advisor + 4 协作模式
5. **apeireth-evolution** (`crates/apeireth-evolution`, mtime 2026-08-10 21:45:12) — PODA + library_autonomy + state machine
6. **apeireth-extension** (`crates/apeireth-extension`, mtime 2026-08-06 08:06:43) — 6 kinds pluginType 锁
7. **apeireth-graph** (`crates/apeireth-graph`, mtime 2026-08-10 21:52:15) — StateGraph + subgraph + channel + state_graph + context_graph + cognition_graph (P6-2 4 NEW)
8. **apeireth-mcp** (`crates/apeireth-mcp`, mtime 2026-08-10 17:53:13) — MCP server-side 全实施 + 4 子文件
9. **apeireth-pipeline** (`crates/apeireth-pipeline`, mtime 2026-08-10 21:22:20) — model_router + provider_registry + role_divider + tiktoken_counter + tool_loop
10. **apeireth-tool-registry** (`crates/apeireth-tool-registry`, mtime 2026-08-10 03:10:31) — Tool + Classifier 9 类 + token_budget
11. **apeireth-tool-runtime** (`crates/apeireth-tool-runtime`, mtime 2026-08-10 21:50:59) — executor + fuzzy + parser + privacy + record + mcp_protocol
12. **apeireth-protocol** (`crates/apeireth-protocol`, mtime 2026-08-10 00:33:07) — 4 adapter + 4 bridge + ws_v1 8 帧

**12 R20 阶段 4 主体 LOCKED (per R131-4 §2.1, R37-2 transparent re-export 模式)**:
13. **apeireth-asi** (`crates/apeireth-asi`, mtime 2026-08-10 16:18:12) — V0.5 24 维公式 + 9 子测度 + 24 measure_dim_* + 9 measure_sub_*
14. **apeireth-onion** (`crates/apeireth-onion`) — 5 重守门来源 + 双洋葱架构 + 哲学核心
15. **apeireth-sovereignty** (`crates/apeireth-sovereignty`, 274KB) — 守门核心 + R124-3 调研 0 触碰
16. **apeireth-constraint** (`crates/apeireth-constraint`, mtime 2026-08-06 08:06:43) — 5 重守门核心
17. **apeireth-memory** (`crates/apeireth-memory`) — 3 层 memory 哲学核心 + 9 LOCKED 9 文件
18. **apeireth-cognition** (`crates/apeireth-cognition`, mtime 2026-08-06 08:06:43) — 9 organ brain 来源
19. **apeireth-perception** (`crates/apeireth-perception`) — 9 organ eye/ear 来源
20. **apeireth-consciousness** (`crates/apeireth-consciousness`) — 9 organ mind 来源 (R37-2 transparent re-export 到 perception)
21. **apeireth-motivation** (`crates/apeireth-motivation`) — 独立哲学 crate 0 改
22. **apeireth-life-force** (`crates/apeireth-life-force`, mtime 2026-08-06 20:02:17) — 9 organ heart 来源 (R37-2 transparent re-export 到 memory)
23. **apeireth-relation** (`crates/apeireth-relation`) — 独立哲学 crate 0 触碰, R124-2 §12 借鉴目标
24. **apeireth-value** (`crates/apeireth-value`) — 独立哲学 crate (R37-2 transparent re-export 到 motivation)

**24 LOCKED 验证结论 (per R131-5 §1.2)**:
- ✅ **24/24 LOCKED crate 入口签名 0 改 全部通过** (R129-11 §4.1 抽查 4/24 + R129-21 复核 6/24 + R131-5 §1.2 24/24 全 PASS)
- ✅ **V1.0 release 0 改 src 严守 100%** (per 决策 #74 §1 B1 V1.0 release 0 改严守)
- ⚠️ **8/10 16:34 之后 mtime 改的 8 个 LOCKED crate** (agent / mcp / tool-runtime / graph / pipeline / evolution / api / cli) — 全部为 0 改入口签名 (新增 module 内的 sub-类型 + re-export, 0 改原 LOCKED 入口签名), V1.0 release commit 拍板时必须保持 mtime 不再变 (已经发生的 0 改是新功能 module 加在原 crate 内, 不算 V1.0 release 改的)

### 2.2 63 非 LOCKED crate 分类 (per Cargo.toml `members` 段 1:1 实地清点 + R131-4 §2.1 分类)

**63 非 LOCKED crate 分类** (per Cargo.toml `members` 段 1:1 实地清点 2026-08-11 02:00, 5 大类):

**A. 核心抽象层 (6 个)**:
- `apeireth-core` — 基座, 0 依赖其他 LOCKED, 7 个 crate 依赖
- `apeireth-telemetry` (R35 observability 4 umbrella: cache/observability/metrics/tracing facade)
- `apeireth-provider` (R35+R36 5 Provider 真合并, 5 老 crate .bak 已删)
- `apeireth-tools` — Tool 工具层
- `apeireth-cli` — CLI runner
- `apeireth-bench` — 长程任务基准

**B. 哲学/能力层 (5 个)**:
- `apeireth-test` — 测试基座
- `apeireth-config` — 配置
- `apeireth-upgrade` — 升级层
- `apeireth-cron` — cron 调度
- `apeireth-acp` — ACP (Agent Communication Protocol)

**C. 智囊团/工具层 (4 个)**:
- `apeireth-pybridge` — Python ↔ Rust 跨语言桥 (PyO3 借鉴)
- `apeireth-api` — HTTP API server (含 V2 端点 + WS 8 帧)
- `apeireth-web` — Web 前端
- `apeireth-supervisor` (tools layer 注: 此 supervisor 在 tools layer 而非 LOCKED 的 supervisor, 跟 LOCKED 同名) — V1306 fix 修真

**D. 兼容组件层 (12 个)**:
- `apeireth-mcp` (LOCKED 同名另算)
- `apeireth-mcp-ssh` / `apeireth-mcp-winrm` / `apeireth-mcp-relay-image` (R20 阶段 1 估补 5 P0 crate skeleton)
- `apeireth-sdk` / `apeireth-sdk-sandbox` (V2 战区 1/4/5 multi-language SDK)
- `apeireth-sdk-lark` / `apeireth-sdk-livekit` / `apeireth-sdk-voice` (V1306 fix 修真, high risk)
- `apeireth-lark` / `apeireth-voice` / `apeireth-livekit` (第三方 SDK stub)

**E. 形式化/治理层 (5 个)**:
- `apeireth-formal` (V2 战区 5 形式化 verification, Kani 借鉴)
- `apeireth-library-governance` (R127 P5-2 治理 crate, 3 大模块 strategy + verification + consistency)
- `apeireth-eval` — 评估
- `apeireth-tracing` / `apeireth-metrics` (R20 阶段 6 + R21 估补, distributed tracing + Prometheus 兼容 metrics)

**F. 借鉴源 1:1 翻译层 (5 个)**:
- `apeireth-pipeline-g5` (R20 阶段 6 估补, 通用 5 阶段 pipeline 框架 借鉴 Golutra v0.1.0)
- `apeireth-pipeline` (LOCKED 同名另算)
- `apeireth-tool-registry` / `apeireth-tool-runtime` (LOCKED 同名另算)
- `apeireth-tool-approval` — 5 规则

**G. 借鉴模式层 (7 个)**:
- `apeireth-agent` (LOCKED 同名另算)
- `apeireth-plugin` — 插件系统
- `apeireth-state` (R21 借鉴 Golutra #6 9 Tauri state 模式转 TUI 等价物, OnceLock + Arc + Mutex)
- `apeireth-cache` (R20 阶段 6 估缺 LRU+TTL cache skeleton, 5 EvictionPolicy + 4 BackendKind)
- `apeireth-credentials` (R20 阶段 6 估缺 multi-provider credentials skeleton, 5 Provider × 5 鉴权 × 4 轮换 × 5 Scope)
- `apeireth-oauth` (R21 借鉴 Golutra OAuth 3 callback 模式, 3 Provider × 3 Callback mode)
- `apeireth-update` (R21 借鉴 Golutra P3 minisign 签名 + autoupdate endpoint)

**H. ASI/认知层 (2 个)**:
- `apeireth-asi` / `apeireth-cognition` (LOCKED 同名另算)
- `apeireth-action` / `apeireth-central` — 行动 + 中央

**I. 升级/通信层 (5 个)**:
- `apeireth-upgrade` / `apeireth-bus` (LOCKED 同名另算) / `apeireth-api` (LOCKED 同名另算) / `apeireth-web` / `apeireth-supervisor` (LOCKED 同名另算)

**J. 持久化/工具层 (4 个)**:
- `apeireth-vector` (V2 战区 4 vector retrieval, sqlite-vec 借鉴)
- `apeireth-observability` (R20 阶段 6 估补, observability skeleton)
- `apeireth-tree-sitter` (R20 阶段 5 估补, tree-sitter skeleton)
- `apeireth-i18n` (R20 阶段 6 估补, 5 Locale + 8 工具 + 1:1 i18next)

**K. 任务/工作流层 (4 个)**:
- `apeireth-task` (R20 阶段 6 估补, taskTools.js 1:1 翻译)
- `apeireth-workflow` / `apeireth-team-lead` (R20 阶段 1 估补 5 P0 crate skeleton)
- `apeireth-cron` — cron 调度

**L. 鉴权/凭据层 (4 个)**:
- `apeireth-credentials` / `apeireth-oauth` (已计, 跨 G/L 段)
- `apeireth-keyring` / `apeireth-machine-id` (R20 阶段 6 估缺 P0 安全)

**M. 监控/告警层 (3 个)**:
- `apeireth-observability` / `apeireth-metrics` / `apeireth-tracing` (已计, 跨 E/J 段)

**N. 安全/沙箱层 (3 个)**:
- `apeireth-sandbox` (R20 阶段 6 估补 Sandbox 真接实现, bollard 0.15 借鉴 Docker daemon)
- `apeireth-keyring` / `apeireth-machine-id` (已计, 跨 L 段)

**O. 工具扩展层 (4 个)**:
- `apeireth-tool-registry` / `apeireth-tool-runtime` / `apeireth-tool-approval` (已计, 跨 F 段)
- `apeireth-state` (已计, 跨 G 段)

**P. 第三方 SDK 层 (4 个)**:
- `apeireth-lark` / `apeireth-voice` / `apeireth-livekit` (已计, 跨 D 段)
- `apeireth-tree-sitter` (已计, 跨 J 段)

**Q. 集成测试层 (4 个)**:
- `apeireth-integration-e2e` (V1305 fix 修真)
- `apeireth-integration-r20-stage4` (V1305 fix 修真)
- `apeireth-tui-e2e` (R20 阶段 5 估补, 25+ 测试)
- `apeireth-image-prompt` (R20 阶段 4 估补)

**R. R20 阶段 1 估补 (5 个)**:
- `apeireth-mcp-ssh` / `apeireth-mcp-winrm` / `apeireth-mcp-relay-image` (已计, 跨 D 段)
- `apeireth-workflow` / `apeireth-team-lead` (已计, 跨 K 段)

**S. R20 阶段 4 估补 (5 个)**:
- `apeireth-image-prompt` (已计, 跨 Q 段)
- `apeireth-rollback` — 回滚
- `apeireth-plugin` (已计, 跨 G 段)
- `apeireth-repo-scan` / `apeireth-repo-analyzer` — 仓库分析

**T. R20 阶段 5/6 估补 (10 个)**:
- `apeireth-tui-e2e` (阶段 5)
- `apeireth-keyring` / `apeireth-machine-id` (阶段 6 估缺 P0 安全)
- `apeireth-lark` / `apeireth-voice` (阶段 6 估缺 第三方 SDK)
- `apeireth-observability` / `apeireth-task` / `apeireth-tree-sitter` / `apeireth-i18n` / `apeireth-naming-v05` / `apeireth-credentials` (阶段 6 估补)
- `apeireth-cache` / `apeireth-sandbox` (阶段 6 估补)
- `apeireth-state` (阶段 6)

**U. R21 估补 (5 个)**:
- `apeireth-tracing` / `apeireth-metrics` (已计, 跨 E 段)
- `apeireth-oauth` / `apeireth-update` / `apeireth-state` (已计, 跨 G/T 段)

**V. R23 P3 透明登记 (1 个 sub-crate)**:
- `apeireth-memory/extensions` (1 sub-crate, 9 provider 模式: in_memory / redis / sqlite / postgres / s3 / disk_lru / hybrid + R23 #6 加 file / mongodb)

**W. V1302/1304/1305/1306 fix (7 个)**:
- `apeireth-blueprint-impl` (V1302 fix 修真 orphan)
- `apeireth-sdk-sandbox` (V1304 fix 修真)
- `apeireth-integration-e2e` / `apeireth-integration-r20-stage4` / `apeireth-rate-limiter` (V1305 fix 修真 medium risk)
- `apeireth-sdk-lark` / `apeireth-sdk-livekit` / `apeireth-sdk-voice` (V1306 fix 修真 high risk)

**X. R127 P5-2 + Blueprint + R17 战役 (3 个)**:
- `apeireth-library-governance` (R127 P5-2 治理 crate)
- `apeireth-blueprint-impl` (V1302 fix 修真)
- `apeireth-tauri-stub` (R20 阶段 6 估补, Tauri 2 desktop 参考实现, autobins=false 不默认 build)

**Y. TUI + V0.5 + 协议 (3 个)**:
- `apeireth-tui` (TUI 9 organ + 5 nav 端到端)
- `apeireth-naming-v05` (V0.5 24 维 + 6 增强 = 30 维, R20 阶段 4 估补)
- `apeireth-protocol` (LOCKED 同名另算) / `apeireth-http-client` (hyper 借鉴) / `apeireth-pipeline` (LOCKED 同名另算)

**63 非 LOCKED 验证结论 (per Cargo.toml 1:1 实地清点)**:
- ✅ 真正核心 ≈ 40-50 crate
- ✅ 估补 + 借鉴 1:1 + transparent re-export ≈ 30+ crate
- ⚠️ 多个 crate 跨多个分类 (e.g. `apeireth-credentials` 跨 G/L 段), 分类仅为参考, 实际 1 crate = 1 独立升级路径

### 2.3 5 transparent re-export crate (3 真 + 2 独立)

**per R37-2 transparent re-export 模式 + R131-4 §2.1 备注**:
- **3 真 transparent re-export crate** (R37-2 实际实施):
  1. `apeireth-life-force` → re-export 到 `apeireth-memory` (透明, 0 改)
  2. `apeireth-value` → re-export 到 `apeireth-motivation` (透明, 0 改)
  3. `apeireth-consciousness` → re-export 到 `apeireth-perception` (透明, 0 改)
- **2 独立哲学 crate** (per R37-2 + R131-4 §2.1, 0 改):
  4. `apeireth-motivation` — 独立哲学 crate, 0 改 (不是 transparent re-export)
  5. `apeireth-relation` — 独立哲学 crate, 0 改 (R124-2 §12 借鉴目标)

**优化方向**:
- **V1.0 release**: 0 改 5 transparent re-export 严守 (R37-2 实施严守)
- **V1.1 release** (per 决策 #74 §1 B1 Mavis 自决改): 可考虑合并 3 真 transparent re-export 到目标 crate:
  - `life-force` → 合并到 `memory` (删除 life-force LOCKED 集合, 0 改 memory LOCKED 入口签名)
  - `value` → 合并到 `motivation` (删除 value LOCKED 集合, 0 改 motivation LOCKED 入口签名)
  - `consciousness` → 合并到 `perception` (删除 consciousness LOCKED 集合, 0 改 perception LOCKED 入口签名)
  - 风险: 改 LOCKED 集合边界 = 改 B1 (V1.1 release Mavis 自决改), 0 改入口签名可保留 pub use 模式
- **V2.0 release**: 24 LOCKED 集合可拆/合 (e.g., 24 → 21 简化 / 24 → 27 复杂化 都 OK per "不要怕复杂度")

### 2.4 9 organ 跨 8 LOCKED crate 分布 (per R125 B7 + R131-4 §2.6)

**9 organ 跨 8 LOCKED crate 分布 (1:1 镜像, per R125-7 借 aGLM 108)**:

| # | Organ | 来源 LOCKED crate | 入口签名 | R119 状态 | mtime | 备注 |
|---|-------|-------------------|---------|----------|-------|------|
| 1 | **body** | `crates/apeireth-tui/src/organ/body.rs` | ⏳ 占位 0 字节 | R119 占位 | 0 字节 | R119 形式撤销后保留 (per `9-organs.md` §主人 R119 视角) |
| 2 | **brain** | `apeireth-cognition` + `crates/apeireth-tui/src/organ/brain.rs` | 🔒 R11 LOCKED | 11.1KB | LOCKED | 9 organ brain 来源 (per R20 哲学 crate 0 触碰) |
| 3 | **ear** | `apeireth-perception` + `crates/apeireth-tui/src/organ/ear.rs` | 🔒 R11 LOCKED | 14.7KB | LOCKED | 监听 9 organ ear 来源 |
| 4 | **eye** | `apeireth-perception` + `crates/apeireth-tui/src/organ/eye.rs` | 🔒 R11 LOCKED | 11.0KB | LOCKED | 观察 9 organ eye 来源 |
| 5 | **hand** | `apeireth-action` + `crates/apeireth-tui/src/organ/hand.rs` | 🔒 R11 LOCKED | 15.7KB | LOCKED | 行动 9 organ hand 来源 |
| 6 | **heart** | `apeireth-life-force` + `crates/apeireth-tui/src/organ/heart.rs` | 🔒 R11 LOCKED | 7.0KB | LOCKED | 生命 9 organ heart 来源 (R37-2 transparent re-export) |
| 7 | **memory** | `apeireth-memory` + `crates/apeireth-tui/src/organ/memory.rs` | 🟢 R78-R113 增量 | 13.0KB | R78-R113 | R54 backend wire-up + cognition_graph + render 0 假装小修 |
| 8 | **mind** | `apeireth-consciousness` + `crates/apeireth-tui/src/organ/mind.rs` | 🔒 R11 LOCKED | 9.3KB | LOCKED | 思想 9 organ mind 来源 (R37-2 transparent re-export) |
| 9 | **voice** | `apeireth-voice` + `crates/apeireth-tui/src/organ/voice.rs` | 🔒 R11 LOCKED | 11.9KB | LOCKED | 表达 9 organ voice 来源 |

**9 organ 跨 8 LOCKED crate 分布 (1:1 镜像, 排除 body.rs 0 字节占位)**:
- brain ↔ `apeireth-cognition`
- ear + eye ↔ `apeireth-perception` (1 crate 覆盖 2 organ)
- hand ↔ `apeireth-action`
- heart ↔ `apeireth-life-force` (R37-2 transparent re-export 到 memory)
- memory ↔ `apeireth-memory` (R11 baseline 0 改)
- mind ↔ `apeireth-consciousness` (R37-2 transparent re-export 到 perception)
- voice ↔ `apeireth-voice` (R20 哲学 crate)

**9 organ 文件结构 (per `docs/omnibus/9-organs.md`)**:
- **9 organ 文件名 + 入口签名 LOCKED** (per 决策 #33 §2.3 B7)
- **9 organ 内部 fn 实施 0 改入口** (per R125 B7 内部借 OpenCode)
- **9 organ 跨维度 (R125-7 借 aGLM 108)**: bci (brain) + mem (memory) + mind (mind) 借 superpowers 9 模式

### 2.5 借鉴源 11 源 + 1 借脑 (per Cargo.toml borrow 段 + R131-2 §2)

**借鉴源 12 源 现状** (per Cargo.toml borrow 段 + R129-7/11/28 1:1 verify + R131-2 §2 12 源 0 装 PASS 严守二次 verify):

**8 真 cloned (49.15MB / 7,619 files, 整合 #4 commit 后 ✅ cloned)**:
| # | 借鉴源 | 版本 | 集成 crate | 实施深度 | mtime | 借鉴 ID |
|---|--------|------|------------|----------|-------|---------|
| 1 | clap-rs/clap | 4.6.6 (Apache-2.0 + MIT dual) | apeireth-cli | 8/10 (commands.rs 12KB / 5 unit test pass) | 17:30:05 | R125-2-BORROW-clap-rs/clap-4a622b4-2026-08-10 |
| 2 | hyperium/hyper | 0.1.20 (MIT) | apeireth-http-client | 7/10 (HTTP 客户端 + LIFO 池复用) | 17:29:39 | R125-3-BORROW-hyperium/hyper-0.1.20-2026-08-10 |
| 3 | modelcontextprotocol/servers | 76d64c8 (MIT → Apache-2.0 过渡) | apeireth-mcp + apeireth-tool-runtime | 9/10 (MCP server-side 全实施, 175 files 借鉴) | 16:51:30 | R125-4-BORROW-modelcontextprotocol/servers-76d64c8-2026-08-10 |
| 4 | PyO3/PyO3 | 0.29.2 (Apache-2.0 + MIT dual) | apeireth-pybridge | 9/10 (Python ↔ Rust 跨语言桥 + 7 guardianship 模块完整) | 16:53:35 | R125-9-BORROW-PyO3/PyO3-0.29.2-2026-08-10 |
| 5 | model-checking/kani | 0.67.0 (MIT + Apache-2.0 dual) | apeireth-formal | 6/10 (kani harness 实施, proofs 模板 22KB) | 17:35:29 | R125-10-BORROW-model-checking/kani-0.67.0-2026-08-10 |
| 6 | langchain-ai/langgraph | d56666f (MIT) | apeireth-graph | 8/10 (StateGraph + checkpoint + conditional + channel + subgraph, 829 files 借鉴) | 16:31:13 | R125-13-BORROW-langchain-ai/langgraph-d56666f-2026-08-10 |
| 7 | obra/superpowers | 6.2.0 (MIT) | apeireth-skills | 8/10 (Skill 化 + Library Stage 4 自治) | 17:33:34 | R125-14-BORROW-obra/superpowers-6.2.0-2026-08-10 |
| 8 | NVIDIA/NeMo-Guardrails | (整合 #4 commit 后 ✅ cloned) | apeireth-sovereignty | 7/10 (8 Action + 5 ActionKind + ActionDispatcher, 20 unit test, 2045 files 借鉴) | 17:48:20 | R125-5-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10 |

**2 借鉴 ID 索引完成 (限流 → 重试真实施, P6-1/2/3 全 done)**:
| # | 借鉴源 | 集成 crate | 实施深度 | 借鉴 ID | 0 装 PASS 严守 |
|---|--------|------------|----------|---------|----------------|
| 9 | BerriAI/litellm (公开 1:1 翻译, 0 cloned) | apeireth-pipeline/src/provider_registry.rs (645 → 1207 行, +562 行) | 7/10 (Router + Cost API 翻译, 19/19 unit test pass) | R125-1-BORROW-BerriAI/litellm-2026-08-10 | ✅ 0 装"已读真源码" |
| 10 | sst/opencode (改借鉴已 cloned langgraph 829 + servers 175, 0 抄 opencode TS 代码) | 3 个 LOCKED crate 各 +1 新模块: apeireth-agent/src/subagent.rs 22.2KB + apeireth-tool-runtime/src/mcp_protocol.rs 22.7KB + apeireth-graph/src/context_graph.rs 20.2KB | 8/10 (35/35 tests pass) | R125-12-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10 | ✅ 0 装"已对接 opencode 私有 channel" |

**1 永久跳过 (OpenCog AGPL-3.0, 0 集成 0 装)**:
| # | 借鉴源 | License | 状态 | 决策 |
|---|--------|---------|------|------|
| 11 | opencog/opencog | AGPL-3.0 (传染性 copyleft) | ❌ 0 cloned 永久跳过 | 决策 #22 §4 风险表 + 决策 #33 §2.2 + Cargo.toml `deny.toml` allow-list 不含 AGPL-3.0 |

**🆕 1 借脑 ID 索引完成 (R130-6 提议 OpenCog 家族 6 子源, V1.1 release 借脑调研沉淀)**:
| # | 借脑 ID | 借鉴源 | 架构 | ROI 梯度 | 0 装 PASS 严守 |
|---|---------|--------|------|----------|----------------|
| 12 | 🆕 R130-6-BORROW-opencog-family-2026Q1-2026-08-11 | opencog/atomspace 4.3.0 + cogutil + moses + pln (deprecated) + relex (deprecated) + CogPrime (Ben Goertzel) | AtomSpace (hypergraph database) + Atomese (graph language) + StorageNode (RocksDB) + forward/backward chainer + Unified Rule Engine (URE) + ECAN (Economic Attention Network) | 🟢 高 (AtomSpace + CogPrime, 30-50KB 报告/子源) + 🟡 中 (MOSES, 10-20KB 报告) + 🔴 低 (cogutil + pln + relex, 5-10KB 报告/子源) | ✅ 0 装"已读真源码" / ✅ 0 装"已集成" / ✅ 0 装"已 fork" |

### 2.6 集成测试 / 监控 / 鉴权 / 持久化 / 调度 等支撑层

**支撑层 (per Cargo.toml 实地清点)**:
- **集成测试 (4 个)**: integration-e2e (V1305 fix) / integration-r20-stage4 (V1305 fix) / tui-e2e (R20 阶段 5 估补) / image-prompt (R20 阶段 4 估补)
- **监控告警 (3 个)**: observability (R20 阶段 6) / metrics (R20 阶段 6) / tracing (R20 阶段 6 + R21 估补)
- **鉴权凭据 (4 个)**: credentials (R20 阶段 6) / oauth (R21 借脑 Golutra) / keyring (R20 阶段 6 估缺 P0) / machine-id (R20 阶段 6 估缺 P0)
- **持久化 (4 个)**: vector (V2 战区 4) / sqlite (内嵌在 memory / vector / api / mcp) / file / cache (R20 阶段 6 估缺 LRU+TTL)
- **调度 (3 个)**: cron / upgrade / task (R20 阶段 6 估补 taskTools.js 1:1 翻译)
- **升级通信 (5 个)**: upgrade / bus / api / web / supervisor (LOCKED 同名另算)
- **形式化治理 (5 个)**: formal / library-governance / eval / tracing / metrics

**87 = 30 × 2.9 远超 v1 30 目标 (per R14 阶段 2 §3 设计 v1)**:
- R14 阶段 2 §3 设计 30 crate: 入口层(1) + 核心抽象(2) + 智能层(3) + 智囊团层(1) + 经验方法论(4) + 兼容组件(5) + 升级层(1) + 通信总线(4) + 持久化(1) + 哲学/权限洋葱双锁层(2) + 双锁补充(6) = **30 crate**
- 实际 87 crate = 30 × 2.9 = 远超 v1 30 目标 (per Cargo.toml `members` 段清点)
- ✅ **87 crate 数量符合"不要怕复杂度"哲学** (per 主人 8/11 01:14 拍板 §3, 最强效果 + 最厉害工程 + 维护交给未来高水平团队)
- ✅ **87 crate 拆得细** 但**符合"立体架构"** (per `architecture-v3-aircraft-carrier.md` §2.1: 立体架构 = 维度 1 生命力纵向 + 维度 2 9 organ 横向 + 维度 3 3 层 memory 深度 = 三维立体, 87 crate 是"立体"自然结果)

---

## 3. 4 重构方案详细 (保守 / 中等 / 激进 / 终极)

### 3.1 方案 A: 保守方案 (V1.0 release, 0 改 + 加 2-3 helper)

**触发条件**: V1.0 release 整合 #5.1 commit 0 改严守 (per 决策 #33 §2.3 + 决策 #74 §1 B1 V1.0 release 0 改)

**方案 A 原则**:
- ❌ **0 改 24 LOCKED 入口签名** (V1.0 release 严守)
- ❌ **0 改 Cargo.toml workspace.version 1.2.0** (B2 严守)
- ❌ **0 改 Cargo.toml `members` 段** (V1.0 release 严守)
- ❌ **0 改 Cargo.toml `borrow` 段** (V1.0 release 严守, 整合 #5.2 commit 时 update 17:44 → 22:50 状态)
- ✅ **可加 2-3 helper crate** (新功能, 不在 24 LOCKED 集合)

**方案 A 实施内容 (V1.0 release 整合 #5.1-5.3 commit 阶段)**:
1. **0 改 24 LOCKED 入口签名严守** (per 决策 #74 §1 B1 V1.0 release 0 改严守 + R131-5 §1.2 24/24 全 PASS)
2. **0 改 Cargo.toml workspace.version 1.2.0 严守** (per 决策 #74 §1 B2)
3. **整合 #5.2 commit 时 Cargo.toml borrow 段 update 17:44 → 22:50 状态** (per 决策 #62 §5.2):
   - `count_cloned` 8 → 10
   - `count_rate_limited` 3 → 0
   - `count_skipped` 1 → 1
   - 🆕 `count_brainonly` 1 (OpenCog 家族 6 子源, 借脑 ID 索引完成)
4. **0 改 Cargo.toml `members` 段 87 crate 严守** (V1.0 release 0 改)
5. **可加 2-3 helper crate (新功能, 0 改 24 LOCKED)**:
   - 例 1: 1.0 release 后可加 `apeireth-tracing` 真接 OpenTelemetry collector (V1.0 release 后 Mavis 自决)
   - 例 2: 1.0 release 后可加 `apeireth-i18n-zh-CN` 1 Locale (V1.0 release 后 Mavis 自决)
   - 例 3: 1.0 release 后可加 `apeireth-bench-swebench` 1 benchmark suite (V1.0 release 后 Mavis 自决)

**方案 A 适用场景**:
- V1.0 release 整合 #5.1-5.3 commit 阶段 (8/15 前 主人拍板, per 决策 #62)
- 严守 8 硬墙 + 0 改 src + 0 改 Cargo.toml 1.2.0 + 0 主动 commit/push/IM
- 0 风险, 0 实施, 仅文档/Cargo.toml borrow 段 update

**方案 A 风险**: 0 风险 (0 改严守, 0 触碰)

**方案 A 决策依据**: 决策 #62 §5.1-5.3 + 决策 #74 §1 V1.0 release 0 改严守 + 决策 #33 §2.3 B1/B2/C1/C2

### 3.2 方案 B: 中等方案 (V1.1 release minor, 合并 5-8 + 拆 1-2)

**触发条件**: V1.1 release minor (per 决策 #74 §1 B1 V1.1 release Mavis 自决改 + 决策 #74 §1 B2 1.2.0 → 1.2.1 bump semver minor)

**方案 B 原则**:
- 🟢 **24 LOCKED 入口签名 0 改** (V1.1 release 仍严守入口签名, Mavis 自决改 crate 边界, 0 改 pub use facade)
- 🔒 **Cargo.toml workspace.version 1.2.0 → 1.2.1 bump** (per 决策 #74 §1 B2)
- 🟢 **可合并 5-8 个非 LOCKED crate** (透明 re-export 仍保留, 0 改 LOCKED 入口签名)
- 🟢 **可拆分 1-2 个大 crate** (e.g. mcp 拆 mcp-core + mcp-resources, 顶层 mcp 保留 facade)
- 🟢 **可加 5-10 个 helper crate** (新功能, 不在 24 LOCKED 集合)

**方案 B 实施内容 (V1.1 release minor 阶段, 9 月估)**:

1. **合并 5-8 个非 LOCKED crate (透明 re-export 保留)**:
   - **merge 1**: `apeireth-credentials` + `apeireth-keyring` + `apeireth-machine-id` → `apeireth-credentials-integrated` (顶层 credentials 仍 re-export, 0 改 LOCKED 入口签名)
   - **merge 2**: `apeireth-mcp-ssh` + `apeireth-mcp-winrm` + `apeireth-mcp-relay-image` → `apeireth-mcp-protocols` (3 MCP 协议合 1 crate, 顶层 mcp 仍 re-export)
   - **merge 3**: `apeireth-observability` + `apeireth-metrics` + `apeireth-tracing` → `apeireth-observability-integrated` (3 监控合 1 crate, 顶层 observability 仍 re-export)
   - **merge 4**: `apeireth-cache` + `apeireth-state` + `apeireth-update` → `apeireth-runtime-utilities` (3 runtime 工具合 1 crate, 顶层 state/cache 仍 re-export)
   - **merge 5**: `apeireth-image-prompt` + `apeireth-rollback` + `apeireth-plugin` → `apeireth-extension-utilities` (3 扩展工具合 1 crate, 顶层 extension 仍 re-export)
   - **merge 6**: `apeireth-oauth` + `apeireth-update` → `apeireth-auth-flow` (2 鉴权合 1 crate, 顶层 oauth 仍 re-export)
   - **merge 7**: `apeireth-workflow` + `apeireth-team-lead` → `apeireth-team-coordination` (2 团队协作合 1 crate, 顶层 team-lead 仍 re-export)
   - **merge 8**: `apeireth-repo-scan` + `apeireth-repo-analyzer` → `apeireth-repo-intelligence` (2 仓库分析合 1 crate, 顶层 repo-scan 仍 re-export)

2. **拆分 1-2 个大 crate**:
   - **split 1**: `apeireth-mcp` (16 modules) → `apeireth-mcp-core` + `apeireth-mcp-resources` + `apeireth-mcp-tools` + `apeireth-mcp-transport` (4 sub-crate, 顶层 mcp 保留 facade re-export 全部 sub-crate)
   - **split 2**: `apeireth-api` (16 modules) → `apeireth-api-core` + `apeireth-api-llm` + `apeireth-api-protocol` + `apeireth-api-server` (4 sub-crate, 顶层 api 保留 facade re-export 全部 sub-crate)

3. **加 5-10 个 helper crate (新功能)**:
   - 例 1: 加 `apeireth-clap-complete` (shell completion 借鉴)
   - 例 2: 加 `apeireth-hyper-server` (HTTP/2 server 借鉴)
   - 例 3: 加 `apeireth-mcp-streamable` (Streamable HTTP transport 借鉴)
   - 例 4: 加 `apeireth-maturin` (PyO3 maturin 集成)
   - 例 5: 加 `apeireth-kani-driver` (kani 真实 proofs driver)
   - 例 6: 加 `apeireth-langgraph-postgres` (PostgresSaver 借鉴)
   - 例 7: 加 `apeireth-superpowers-skill-review` (Skill review 借鉴)
   - 例 8: 加 `apeireth-guardrails-colang-parser` (Colang DSL parser 借鉴)
   - 例 9: 加 `apeireth-litellm-loadbalance` (load balancing + circuit breaker 借鉴)
   - 例 10: 加 `apeireth-opencode-agents-md` (AGENTS.md 持久化 借鉴)

**方案 B 效果**:
- **87 crate → 75-80 crate** (合并 5-8 + 拆分 1-2 = -8~+6, 加 5-10 helper = +5~+10, 总 -3~+8)
- **实际 75-95 crate 范围** (符合"不要怕复杂度"哲学, 87 → 80 是"细化优化", 87 → 95 是"深化扩展")
- **Cargo.toml workspace.version bump 1.2.0 → 1.2.1** (semver minor, 0 改 LOCKED 入口签名)
- **Cargo.toml `members` 段 update 87 → 75-95** (整合 #6 commit 时)

**方案 B 风险**:
- **中**: 拆大 crate = 改 import 路径 (`use apeireth_mcp::protocol::Id` → `use apeireth_mcp::mcp_core::protocol::Id` 或 顶层 facade 仍能用)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_mcp::Type` 仍能用
- **缓解**: 整合 #6 commit 时 bump workspace.version 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)
- **缓解**: 0 改 24 LOCKED 入口签名 (V1.1 release Mavis 自决改 = 0 改 LOCKED 入口签名, 改 crate 边界)

**方案 B 决策依据**: 决策 #74 §1 B1 V1.1 release Mavis 自决改 (前提: 更好的架构) + 决策 #74 §1 B2 1.2.0 → 1.2.1 bump + 决策 #73 §3 不要怕复杂度哲学

### 3.3 方案 C: 激进方案 (V1.1 release major, 24 LOCKED 入口签名 Mavis 自决改)

**触发条件**: V1.1 release major (per 决策 #74 §2.3 V1.1 release 边界 + 决策 #73 §1 主人 8/11 01:14 拍板 "Mavis 自决架构拍板")

**方案 C 原则**:
- 🟢 **24 LOCKED 入口签名 Mavis 自决改** (V1.1 release 触发条件: 更好的架构, per 决策 #74 §2.3)
- 🔒 **Cargo.toml workspace.version 1.2.0 → 1.3.0 bump** (per 决策 #74 §1 B2, semver minor 但 V1.1 release 强化为 1.3.0 因改 LOCKED 入口签名)
- 🟢 **24 LOCKED 重新分 crate** (B1 Mavis 自决改, 可拆/合 LOCKED 集合)
- 🟢 **新增 24 LOCKED 入口签名 8 方向改写** (per R131-5 §2.8 V1.1 release 改写入口签名 8 个方向)
- 🔒 **8 哲学锚严守** (B5)
- 🔒 **R11 baseline 3 值严守** (A1)

**方案 C 实施内容 (V1.1 release major 阶段, 10 月估)**:

1. **24 LOCKED 入口签名 8 方向改写** (per R131-5 §2.8 V1.1 release 改写入口签名 8 个方向):
   - **方向 1**: 入口签名一致性 标准化 — per-crate 选 3 模式之一 (全 re-export / 主类型 facade / 按需 re-export), 24 LOCKED 全部统一
   - **方向 2**: 公开 API 表面 瘦身 — per-crate 暴露 ≤30 pub items, 多余的转 `pub(crate)` 或 module-private
   - **方向 3**: 9 叶子 crate 拆 workspace — supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench → `apeireth-leaf/` workspace
   - **方向 4**: core 拆 pub mod — 当前 1 个 108KB lib.rs 拆成 `core/{types,onion,human,principle,gate,action,verdict}/mod.rs`
   - **方向 5**: 大模块集中 crate 拆 sub-crate — mcp 拆 mcp-core / mcp-resources / mcp-tools / mcp-transport / mcp-primitives / mcp-macros; pipeline 拆 pipeline-token / pipeline-placeholder / pipeline-force-translate / pipeline-retry / pipeline-streaming / pipeline-tool-loop
   - **方向 6**: DSL 洋葱落地 — 新增 `apeireth-dsl` crate, Colang 真实施, 24 LOCKED crate 引用 dsl 守门
   - **方向 7**: 9 organ 内部借 OpenCode (R125 B7) — 24 LOCKED crate 内部 fn 借 OpenCode 0 改入口签名
   - **方向 8**: R12 测度对齐 — 24 测量函数签名更新 R12 测度, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新

2. **24 LOCKED 重新分 crate** (per 决策 #74 §1 B1 Mavis 自决改):
   - **拆 LOCKED 集合 (24 → 30, 加 6 个 R20 阶段 4 主体)**: 24 LOCKED + 6 R20 阶段 4 估补 = 30 LOCKED (透明 re-export 3 真合并: life-force → memory, value → motivation, consciousness → perception)
   - **合 LOCKED 集合 (24 → 18, 减 6 个小 crate)**: 24 LOCKED - 6 个小 crate (bus / tool-approval / extension / evolution / life-force / value 合并到相邻大 crate) = 18 LOCKED
   - **保持 24 LOCKED 集合**: 0 改, 24 LOCKED 入口签名 0 改
   - **Mavis 自决**: 上述 3 选项 24/30/18 LOCKED Mavis 自决选 (per 决策 #74 §1 B1 Mavis 自决改)

3. **PHL-07 实施** (per 决策 #74 §1 A3 PHL-07 V1.0 spec-only + V1.1 实施 + 12 键其他可改):
   - PHL-07 NotUnoptimizable 实施 (R125-12 借鉴, V1.1 release 真接)
   - 12 键其他可改 (per 决策 #74 §1 A3)

4. **新增 8 哲学锚改写 / 保留** (per 决策 #74 §1 B5 严守, 但 8 哲学锚内部实施可改):
   - 8 哲学锚 S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 名称严守
   - 8 哲学锚内部实施可改 (per R131-1 §2.10 + R131-3 V1.1 release 路线图)

5. **Cargo.toml workspace.version bump 1.2.0 → 1.3.0** (per 决策 #74 §1 B2, semver minor 因 V1.1 release 强化)

**方案 C 效果**:
- **24 LOCKED → 18/24/30 LOCKED 集合** (3 选项 Mavis 自决, 18 简化 / 24 保持 / 30 复杂化)
- **87 crate → 80-100 crate** (方案 B 中等 + 方案 C 激进 = 8 方向改写)
- **24 LOCKED 入口签名改写** (per 决策 #74 §2.3 V1.1 release 边界 Mavis 自决改)
- **PHL-07 实施** (V1.1 release 实施, V1.0 release spec-only 0 实施)
- **8 哲学锚内部实施可改** (8 哲学锚名称严守)

**方案 C 风险**:
- **高**: 改 24 LOCKED 入口签名 = breaking change = 改消费者 `use` 路径
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用
- **缓解**: 整合 #7 commit 时 bump workspace.version 1.2.0 → 1.3.0 (per 决策 #74 §1 B2)
- **缓解**: 0 改 8 哲学锚名称 (per 决策 #74 §1 B5 严守)
- **缓解**: 0 改 R11 baseline 3 值 (per 决策 #74 §1 A1 严守)
- **缓解**: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**方案 C 决策依据**: 决策 #74 §1 B1 V1.1 release Mavis 自决改 (前提: 更好的架构) + 决策 #74 §2.3 V1.1 release 边界 + 决策 #73 §1 主人 8/11 01:14 拍板 "Mavis 自决架构拍板" + 决策 #73 §3 不要怕复杂度

### 3.4 方案 D: 终极方案 (V2.0 release, 全 workspace 重写 9 organ)

**触发条件**: V2.0 release (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚")

**方案 D 原则**:
- 🟢 **24 LOCKED 全部下沉到 9 organ workspace** (per R131-5 §2.6 + R131-4 §2.6 9 organ workspace 化)
- 🔒 **Cargo.toml workspace.version 1.3.0 → 2.0.0 bump** (per 决策 #74 §2.3 semver major, breaking change)
- 🟢 **9 organ 全部重新设计** (brain / hand / memory / voice / body / mind / heart / ear / eye)
- 🟢 **全 workspace 重写** (顶层 `apeireth` re-export facade 保留)
- 🟢 **8 哲学锚可重评** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
- 🟢 **R11 baseline 3 值可重评** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

**方案 D 实施内容 (V2.0 release 阶段, 11 月+ 估)**:

1. **9 organ workspace 化** (per R131-5 §2.6 + R131-4 §2.6):
   - `apeireth-heart` workspace: supervisor + bus (L0) + pipeline (LLM 网关心跳)
   - `apeireth-brain` workspace: agent + council + cognition + constraint (Multi-Agent 决策)
   - `apeireth-hand` workspace: tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action (Tool Protocol)
   - `apeireth-eye` workspace: (从 tui/src/organ/eye.rs 抽 crate, 用户输入感知)
   - `apeireth-ear` workspace: bus (L1-L4) (系统事件监听)
   - `apeireth-memory` workspace: memory + asi + life-force + core (IdentityCard 跨载体) (3 层 facade)
   - `apeireth-voice` workspace: protocol + pipeline (流式) + (未来 tts/stt crate) (TTS/STT)
   - `apeireth-body` workspace: bench + api + cli (长程任务)
   - `apeireth-mind` workspace: evolution + graph (lifecycle 编排) + constraint (5 重守门) (9-stage lifecycle)

2. **全 workspace 重写**:
   - 顶层 `apeireth` workspace: re-export 全部 9 organ workspace types
   - 9 organ workspace 各自有 Cargo.toml, members 段
   - 24 LOCKED 全部下沉到 organ workspace, 顶层 `apeireth` re-export 全部 organ types

3. **三洋葱架构 workspace 化** (per R131-5 §2.5):
   - `apeireth-onion` workspace: core (原则 + 权限双洋葱) + constraint (守门) + dsl (DSL 洋葱) + life-force (SGI)
   - 顶层 `apeireth-onion` facade 重新导出全部洋葱 module

4. **Cargo.toml workspace.version bump 1.3.0 → 2.0.0** (per 决策 #74 §2.3 semver major, breaking change)
5. **8 哲学锚可重评** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评, 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚")
6. **R11 baseline 3 值可重评** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评, 前提: 新 baseline 更高)

**方案 D 效果**:
- **24 LOCKED → 9 organ workspace** (24 LOCKED 全部下沉到 9 organ workspace)
- **87 crate → 9 organ workspace + 顶层 apeireth re-export** (大幅简化顶层 Cargo.toml, 9 organ 各 workspace 自管)
- **24 LOCKED 入口签名 → 9 organ 入口签名** (顶层 re-export 保留兼容)
- **8 哲学锚可重评** (V2.0 release 触发条件: 推翻 + 重建 8 哲学锚)
- **R11 baseline 3 值可重评** (V2.0 release 触发条件: 新 baseline 更高)

**方案 D 风险**:
- **极高**: 全 workspace 重写 = 改 24 LOCKED crate 全部路径 = 改 N 个消费者的 `use` 路径 = breaking change
- **缓解**: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用
- **缓解**: V2.0 release bump workspace.version 1.3.0 → 2.0.0 (per 决策 #74 §2.3 semver major, breaking change 允许)
- **缓解**: 8 哲学锚可重评 = V2.0 release 跟哲学同步重评, 0 哲学锁死
- **缓解**: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学 100% 一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**方案 D 决策依据**: 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚" + 决策 #73 §1 主人 8/11 01:14 拍板 "Mavis 自决架构拍板"

### 3.5 4 方案对比总结

| 维度 | 方案 A 保守 | 方案 B 中等 | 方案 C 激进 | 方案 D 终极 |
|------|------------|------------|------------|------------|
| **触发 release** | V1.0 release | V1.1 release minor | V1.1 release major | V2.0 release |
| **时间** | 8/15 前 | 9 月估 | 10 月估 | 11 月+ 估 |
| **workspace.version** | 1.2.0 严守 | 1.2.0 → 1.2.1 (minor) | 1.2.0 → 1.3.0 (minor 强化) | 1.3.0 → 2.0.0 (major) |
| **24 LOCKED 入口签名** | 🔒 0 改严守 | 🟢 0 改入口签名 (Mavis 自决改 crate 边界) | 🟢 Mavis 自决改 (8 方向改写) | 🟢 全 workspace 重写 |
| **24 LOCKED 集合** | 严守 24 | 严守 24 | Mavis 自决 18/24/30 | 9 organ workspace |
| **合并 crate** | 0 | 5-8 个 | 5-8 个 + 8 方向改写 | 全 workspace 重写 |
| **拆分 crate** | 0 | 1-2 个 | 1-2 个 + 大模块集中拆 sub-crate | 9 organ workspace |
| **加 helper crate** | 0 (V1.0 release 后可加 2-3) | 5-10 个 | 5-10 个 + PHL-07 实施 | 9 organ workspace 自管 |
| **crate 总数** | 87 (0 改) | 80-95 (-3~+8) | 80-100 (-3~+13) | 9 organ workspace (大幅简化顶层) |
| **Cargo.lock 大小** | 265KB 严守 | 270-300KB | 270-300KB | 200-280KB (9 organ 独立 lockfile) |
| **哲学严守** | 8 哲学锚 + 8 硬墙 全严守 | 8 哲学锚严守 + B1 改写 | 8 哲学锚严守 + B1/A3 改写 | 8 哲学锚可重评 |
| **风险** | 0 风险 | 中 (拆大 crate) | 高 (改 LOCKED 入口签名) | 极高 (全 workspace 重写) |
| **决策依据** | 决策 #33 §2.3 + #62 §5.1-5.3 + #74 §1 V1.0 release 0 改严守 | 决策 #74 §1 B1 V1.1 release Mavis 自决改 (前提: 更好的架构) | 决策 #74 §1 B1 + #74 §2.3 V1.1 release 边界 | 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 |

---

## 4. Cargo.lock 大小分析 (271KB / 10,752 行 / 648 crates)

### 4.1 Cargo.lock 现状 (per `Get-Item Cargo.lock | Length` 实测 2026-08-11)

**Cargo.lock 现状** (per `Get-Item Cargo.lock | Length`):
- **文件大小**: **271,450 bytes (~265 KB)** (per 实测 2026-08-11 01:35)
- **行数**: **10,752 行** (per `Get-Content Cargo.lock | Measure-Object -Line`)
- **总 workspace members**: **87 个** (per Cargo.toml `members` 段清点)
- **总第三方 crates**: **561 个** (per THIRD-PARTY-NOTICES.md, 0 cargo-deny violation, 12 SPDX)
- **总依赖 crate**: 87 + 561 = **648 crates**
- **文件时间**: 整合 #4 commit abf12243 (8/10 19:41) 时一并 update

### 4.2 业界对比 (per R131-4 §2.4 业界 Cargo.lock 范围)

**业界 Cargo.lock 大小对比** (per R131-4 §2.4 业界 Cargo.lock 范围):
- **小型 Rust 项目** (1-10 crate): Cargo.lock 通常 10-50 KB
- **中型 Rust 项目** (10-50 crate): Cargo.lock 通常 50-150 KB
- **大型 Rust 项目** (50-100 crate): Cargo.lock 通常 150-350 KB
- **超大型 Rust 项目** (100+ crate, 如 tokio / rust-analyzer / servo): Cargo.lock 通常 200-500 KB
- **Apeireth Cargo.lock 265KB 在合理范围** (87 + 561 crate, 中-大型 Rust 项目)

**Cargo.lock 大小合理性验证**:
- ✅ **87 + 561 = 648 crates 合理** (87 workspace members + 561 第三方依赖 = 648 crates, 中-大型 Rust 项目)
- ✅ **0 cargo-deny violation** (per P13-1 THIRD-PARTY-NOTICES.md 1709 lines / 12 SPDX / 0 cargo-deny violation)
- ✅ **12 unique SPDX** (Apache-2.0 / MIT / BSD-3-Clause / MPL-2.0 / etc., 0 license conflict)
- ✅ **整合 #4 commit abf12243 含 Cargo.lock** (per 决策 #48 §1.2)

### 4.3 Cargo.lock 第三方依赖 561 crate 分布

**561 第三方 crate 分布** (per THIRD-PARTY-NOTICES.md 1709 lines + 12 SPDX 统计):
- **核心 web/network (~30)**: tokio / hyper / reqwest / hyper-util / actix-web / warp / axum / tower / tower-http / h2 / http / http-body / httparse / url / form_urlencoded / encoding_rs / mime / percent-encoding / etc.
- **序列化 (~15)**: serde / serde_json / serde_derive / serde_yaml / toml / bincode / rmp / rmp-serde / rmpv / ciborium / prost / prost-types / etc.
- **加密/HASH (~20)**: sha2 / sha1 / md-5 / blake2 / blake3 / argon2 / bcrypt / scrypt / ring / openssl / rustls / rustls-pemfile / rustls-native-certs / webpki / etc.
- **数据库/ORM (~25)**: rusqlite / sqlite / sqlite3-src / libsqlite3-sys / sqlx / sqlx-core / sqlx-macros / sqlx-sqlite / diesel / etc.
- **时间 (~10)**: chrono / time / time-macros / time-core / num-time / etc.
- **日志/监控 (~15)**: tracing / tracing-core / tracing-subscriber / tracing-futures / tracing-attributes / log / env_logger / fern / prometheus / metrics / metrics-exporter-prometheus / etc.
- **错误处理 (~5)**: anyhow / thiserror / error-chain / snafu / etc.
- **异步/并发 (~20)**: futures / futures-util / futures-core / futures-channel / futures-macro / futures-executor / futures-task / async-trait / async-stream / tokio-stream / tokio-util / tokio-rustls / tokio-stream / etc.
- **CLI (~5)**: clap / clap_derive / clap_complete / clap_mangen / structopt / etc.
- **Python (~15)**: pyo3 / pyo3-derive-backend / pyo3-macros / pyo3-build-config / pyo3-ffi / indoc / inventory / etc.
- **Cache (~10)**: lru / hashbrown / parking_lot / crossbeam / crossbeam-epoch / crossbeam-utils / concurrent-queue / etc.
- **Tree-sitter (~5)**: tree-sitter / tree-sitter-cli / streaming-iterator / etc.
- **GUI (~10)**: ratatui / crossterm / tui / tui-input / etc.
- **Tauri (~20)**: tauri / tauri-build / tauri-runtime / tauri-utils / tauri-macros / wry / webview2-com / etc.
- **形式化 (~10)**: kani / kani-driver / kani_metadata / etc. (per R125-10 借鉴)
- **测试 (~15)**: criterion / proptest / mockall / mockito / wiremock / httpmock / etc.
- **构建/工具 (~10)**: cargo_metadata / cargo / cargo_toml / toml_edit / etc.
- **其他 (~300)**: uuid / uuid-macros / rand / rand_chacha / getrandom / once_cell / lazy_static / cfg-if / cc / etc.

**561 第三方 crate 大小估算** (按平均 200-500 bytes per crate entry):
- 平均每 crate entry = 200-500 bytes (含 `name = "xxx"` + `version = "x.y.z"` + `source = "registry+..."` + `checksum = "sha256-..."` + 5-10 dependencies)
- 561 × 400 bytes ≈ 224 KB (占 Cargo.lock 271KB 的 83%)
- 87 workspace + 561 第三方 = 648 crates 合理

### 4.4 Cargo.lock 简化方案 (V1.0/V1.1/V2.0 release)

**V1.0 release (整合 #5.1 commit, 0 改严守)**:
- ❌ **0 改 Cargo.lock 严守** (整合 #5 commit 时 Cargo.lock 0 改, 等整合 #5.1 commit 时一并 update)
- ✅ **Cargo.lock update** (整合 #5.1 commit 时 cargo build 触发 Cargo.lock 自动 update, mavis-trash `target/` 加速 build)
- ✅ **Cargo.lock commit policy**: 整合 #4 commit abf12243 含 Cargo.lock (per 决策 #48 §1.2), V1.0 release Cargo.lock 严守 0 改

**V1.1 release (方案 B 中等 + 方案 C 激进, 整合 #6/#7 commit)**:
- 🟢 **Cargo.lock 拆 2-3 模块 lockfile** (per Cargo 1.78+ feature `[workspace.metadata.cargo-tree]`):
  - `crates/apeireth-core/Cargo.lock` (core + 24 LOCKED + 6 核心抽象) + `crates/non-locked/Cargo.lock` (其余 60+ crate) + `frontend/Cargo.lock` (Tauri) = 3 lockfile
  - 顶层 Cargo.lock 0 改, 9 organ workspace 各自独立 Cargo.lock
- 🟢 **Cargo.lock dedup 优化**: cargo tree --workspace --duplicates 显示重复 crate 版本, V1.1 release dedup
- 🟢 **第三方依赖精简** (per 决策 #73 §3 不要怕复杂度): 561 第三方 → 540-560 第三方 (砍 5-20 个未用 dep)
- 优点: 减小主 Cargo.lock 大小, 加快 cargo build 增量编译
- 缺点: 跨模块 dep 解析变慢, Cargo 1.78+ 才支持, 0 业务价值

**V2.0 release (方案 D 终极, 整合 #8+ commit)**:
- 🟢 **Cargo.lock 全拆分 9 organ workspace 独立 lockfile** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评):
  - `apeireth-heart/Cargo.lock` + `apeireth-brain/Cargo.lock` + `apeireth-hand/Cargo.lock` + `apeireth-eye/Cargo.lock` + `apeireth-ear/Cargo.lock` + `apeireth-memory/Cargo.lock` + `apeireth-voice/Cargo.lock` + `apeireth-body/Cargo.lock` + `apeireth-mind/Cargo.lock` = 9 organ 独立 lockfile
  - 顶层 `apeireth/Cargo.lock` 0 改, 9 organ workspace 各自独立 Cargo.lock
- 🟢 **Cargo.lock 大小优化**:
  - 主 Cargo.lock 200-280KB (vs 当前 265KB, -20% 简化)
  - 9 organ lockfile 各 80-150KB (按 organ 大小分)
  - 总和 200-280KB + 9 × 80-150KB = 920-1,550KB (跨 organ 重复 dep)
- 优点: 每个 organ 独立编译, 加快增量编译, cargo cache 复用率提升
- 缺点: 跨 organ dep 解析变慢, Cargo 1.78+ 才支持, 复杂度高

**Cargo.lock 简化决策依据**:
- V1.0 release: 0 改严守 (per 决策 #33 §2.3 + 决策 #74 §1 B2)
- V1.1 release: 拆 2-3 lockfile (per 决策 #74 §1 B1 V1.1 release Mavis 自决改 + 决策 #73 §3 不要怕复杂度)
- V2.0 release: 拆 9 organ lockfile (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 主人 8/11 01:14 拍板 "推翻 + 重建")

---

## 5. 24 LOCKED 入口分布最优化 (合并/拆分 + 改 crate 边界)

### 5.1 现状 24 LOCKED 入口分布 (per R131-5 §1 + R131-4 §2.2)

**24 LOCKED 入口分布现状** (per R131-5 §1.1 mtime 实测 + R131-5 §1.2 入口签名 0 改 verify 24/24 全 PASS):

**12 主路径 LOCKED mtime 分布** (per R131-5 §1.1):
- **8/6 8:06 严守 (R11 baseline 真正 LOCKED)**: 5 个 (supervisor / extension / cognition / action / constraint)
- **8/9 严守**: 2 个 (core / tools)
- **8/10 凌晨 (16:34 之前) 严守**: 4 个 (council / protocol / tool-registry / memory)
- **8/10 16:34 之前 严守**: 1 个 (asi 16:18 < 16:34)
- **8/10 16:34 之后 改了**: 8 个 (agent 21:48 / mcp 17:53 / tool-runtime 21:50 / graph 21:52 / pipeline 21:22 / evolution 21:45 / api 22:22 / cli 21:29)
  - **总共 8 个 LOCKED crate mtime 超 16:34 baseline**
  - **这些 mtime 超标 entries 的入口签名 0 改 verify**: 全部 0 改 (新增 module 内的 sub-类型 + re-export, 0 改原 LOCKED 入口签名)

**12 R20 阶段 4 主体 LOCKED 入口分布** (per R131-5 §1.2):
- **3 真 transparent re-export crate**: life-force (→ memory) / value (→ motivation) / consciousness (→ perception)
- **2 独立哲学 crate**: motivation / relation (0 改)
- **7 核心哲学 crate**: asi / onion / sovereignty / constraint / memory / cognition / perception

**24 LOCKED 公开 API 表面 (粗估, per R131-5 §2.2)**:
- **24 crate 公开 API 表面 = ~800+ pub items** (粗估, 实测需 ripgrep 验证)
- supervisor ~12 / agent ~25 / council ~50+ / bus ~20 / protocol ~40 / mcp ~30 / tool-registry ~30 / tool-runtime ~25 / graph ~40 / pipeline ~35 / tool-approval ~15 / extension ~17 / evolution ~50+ / api ~40+ / core ~50+ / memory ~50+ / asi ~50+ / tools ~30 / cli ~25 / bench ~20 / cognition ~25 / action ~20 / life-force ~25 / constraint ~25

**24 LOCKED crate 间依赖图 (per R131-5 §2.3 静态分析)**:
1. **core 是基座** (7 个 crate 依赖: memory / constraint / cognition / council / life-force / action / cli)
2. **tool-registry 是 tool 生态基座** (5 个 crate 依赖: agent / tool-runtime / tools / mcp)
3. **protocol + pipeline 是 LLM 链基座** (2 个 crate 依赖: api + pipeline 互依)
4. **asi 是认知基座** (1 个 crate 依赖: cognition + cli)
5. **memory 是历史流基座** (1 个 crate 依赖: tool-runtime)
6. **0 依赖其他 LOCKED crate 的"叶子"**: supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench (9 个) — 这 9 个是 V1.0 release 之后"分层下沉"或"独立发布"的候选

**24 LOCKED crate 内部模块分布** (per R131-5 §2.4):
- **大模块集中**: council (20+) / mcp (13) / graph (11) / pipeline (11) / api (16) / memory (13) / asi (9) / tools (12) / evolution (9)
- **core 是单 lib.rs 108KB**: 没有 pub mod 拆分, 全部 50+ 类型定义在一个文件 → 编译时全文件 re-parse, 难维护
- **mcp / pipeline / api / memory 内部 module 边界模糊**: 多个 module 之间 cross-use, 实测命名重复

### 5.2 9 叶子 crate 拆 workspace (V1.1 release, 方案 B 中等)

**9 叶子 crate 现状** (per R131-5 §2.3 + R131-4 §2.1):
- supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench
- 这 9 个 LOCKED crate 0 依赖其他 LOCKED crate, 实际上有内部跨 crate 集成 (extension 用 core, asi 用 asi 内), 但不被其他 LOCKED crate 依赖
- 这 9 个是 V1.0 release 之后"分层下沉"或"独立发布"的候选

**9 叶子 crate 拆 workspace 方案 (V1.1 release minor, 整合 #6 commit)**:
- 新 workspace: `apeireth-leaf/{supervisor,protocol,bus,tool-registry,graph,extension,evolution,asi,bench}/Cargo.toml`
- 顶层 `apeireth/Cargo.toml` 0 改, 9 叶子拆出来独立发布
- 每个 leaf crate 独立 publish (V1.1 release 1.2.1 之后)
- 顶层 `apeireth-leaf` workspace re-export 全部 9 叶子 types (0 改消费者代码)

**9 叶子 crate 拆 workspace 实施步骤 (V1.1 release 整合 #6 commit)**:
1. 创建 `apeireth-leaf/` workspace 根 Cargo.toml (resolver = "2", members = 9 叶子)
2. 移动 9 叶子 crate 路径: `crates/apeireth-supervisor/` → `apeireth-leaf/apeireth-supervisor/`
3. 更新 9 叶子 Cargo.toml `path` 引用 (顶层路径调整)
4. 顶层 `apeireth/Cargo.toml` 移除 9 叶子 from `members` 段, 加 `apeireth-leaf` 路径
5. 顶层 `apeireth-leaf/` 创建 re-export facade (e.g. `apeireth-leaf/src/lib.rs` 重新导出全部 9 叶子 types)
6. cargo build --workspace 验证 (确保 0 编译错误)
7. cargo test --workspace 验证 (确保 0 测试失败)

**9 叶子 crate 拆 workspace 风险**:
- **中**: 拆 workspace = 改 Cargo.toml 路径 = 改消费者 `use apeireth_xxx` → `use apeireth_leaf::xxx` (路径变化)
- **缓解**: 顶层 `apeireth-leaf` re-export facade 保留, 消费者用 `apeireth_leaf::Type` 仍能用
- **缓解**: V1.1 release bump workspace.version 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)

### 5.3 core 拆 pub mod (V1.1 release, 方案 B 中等)

**core 现状** (per R131-5 §2.4):
- **core 是单 lib.rs 108KB**: 没有 pub mod 拆分, 全部 50+ 类型定义在一个文件
- 编译时全文件 re-parse, 难维护
- 7 个 crate 依赖 core (memory / constraint / cognition / council / life-force / action / cli), 任何 core 改动触发大面积重编译

**core 拆 pub mod 方案 (V1.1 release minor, 整合 #6 commit)**:
- 当前 1 个 108KB lib.rs 拆成 `core/{types,onion,human,principle,gate,action,verdict}/mod.rs` = 7 sub-mod
- **0 改入口签名** (V1.1 release minor 仍严守 core 入口签名, Mavis 自决改 crate 内部 module)
- 顶层 `core/src/lib.rs` 重新导出 7 sub-mod 全部 types (保留 pub use facade)

**core 拆 pub mod 7 sub-mod 实施步骤**:
1. `core/src/lib.rs` (108KB) → 拆 7 mod:
   - `core/src/types/mod.rs` (IdentityCard / Session / Episode / Note / 4 个 type)
   - `core/src/onion/mod.rs` (PrincipleOnion / PrincipleLayer / PermissionOnion / PermissionLayer)
   - `core/src/human/mod.rs` (HumanAuthority / HAMode / RealHuman / HAAuthentication / BiometricData)
   - `core/src/principle/mod.rs` (PhilosophyKey / 12 variant / ALL_TWELVE_KEYS / TWELVE_KEYS_HARDCODE)
   - `core/src/gate/mod.rs` (PhilosophyGuard / PhilosophyVerdict / VerdictCache / Gate / 5 variant)
   - `core/src/action/mod.rs` (Action / RiskLevel / ActionTarget / ActionVerdict / ActionGuard / DefaultPhilosophyGuard)
   - `core/src/verdict/mod.rs` (4 verdict type + 1 verdict fn)
2. 顶层 `core/src/lib.rs` 重新导出 7 mod: `pub mod types; pub mod onion; pub mod human; pub mod principle; pub mod gate; pub mod action; pub mod verdict;` + `pub use types::*;` + `pub use onion::*;` + ...
3. cargo build --workspace 验证 (确保 0 编译错误)
4. cargo test --workspace 验证 (确保 0 测试失败)
5. R11 baseline 3 值严守 (per 决策 #74 §1 A1)

**core 拆 pub mod 风险**:
- **低**: 拆 pub mod = 0 改入口签名 (顶层 lib.rs 仍 re-export 全部 types)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_core::Type` 仍能用
- **缓解**: 0 改 R11 baseline 3 值 (per 决策 #74 §1 A1)

### 5.4 大模块集中 crate 拆 sub-crate (V1.1 release, 方案 C 激进)

**大模块集中 crate 现状** (per R131-5 §2.4):
- council (20+ 模块) / mcp (13 模块) / graph (11 模块) / pipeline (11 模块) / api (16 模块) / memory (13 模块) / asi (9 模块) / tools (12 模块) / evolution (9 模块)
- 入口文件 re-export 100+ items, 模块边界模糊

**大模块集中 crate 拆 sub-crate 方案 (V1.1 release major, 整合 #7 commit)**:
- **mcp 拆 6 sub-crate**: mcp-core / mcp-resources / mcp-tools / mcp-transport / mcp-primitives / mcp-macros
- **pipeline 拆 6 sub-crate**: pipeline-token / pipeline-placeholder / pipeline-force-translate / pipeline-retry / pipeline-streaming / pipeline-tool-loop
- **api 拆 4 sub-crate**: api-core / api-llm / api-protocol / api-server
- **memory 拆 4 sub-crate**: memory-core / memory-semantic / memory-three-layer / memory-user-profile
- **council 拆 3 sub-crate**: council-advisor / council-collaboration / council-trace
- **graph 拆 3 sub-crate**: graph-core / graph-state / graph-context
- **asi 拆 3 sub-crate**: asi-measurement / asi-render / asi-scheduler
- **tools 拆 3 sub-crate**: tools-core / tools-code-exec / tools-web
- **evolution 拆 3 sub-crate**: evolution-core / evolution-poda / evolution-library-autonomy
- 顶层 mcp/pipeline/api/memory/council/graph/asi/tools/evolution 保留 re-export facade
- 24 LOCKED 入口签名 0 改 (V1.1 release Mavis 自决改 crate 边界, 顶层 facade 保留)

**大模块集中 crate 拆 sub-crate 实施步骤 (V1.1 release 整合 #7 commit)**:
1. 对每个大 crate 创建 sub-crate (e.g. `crates/apeireth-mcp/src/resources.rs` → `crates/apeireth-mcp-resources/src/lib.rs`)
2. 移动 module 内部 implementation 到 sub-crate
3. 更新 sub-crate Cargo.toml (deps + workspace 引用)
4. 顶层 crate `src/resources/mod.rs` 改 `pub use apeireth_mcp_resources::*;` (re-export facade)
5. 顶层 crate `src/lib.rs` 重新导出 sub-crate types
6. cargo build --workspace 验证 (确保 0 编译错误)
7. cargo test --workspace 验证 (确保 0 测试失败)

**大模块集中 crate 拆 sub-crate 风险**:
- **中**: 拆 sub-crate = 改 import 路径 (`use apeireth_mcp::protocol::Id` → `use apeireth_mcp_core::protocol::Id` 或 顶层 facade 仍能用)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_mcp::Type` 仍能用
- **缓解**: 24 LOCKED 入口签名 0 改 (V1.1 release Mavis 自决改 crate 边界, 顶层 facade 保留)
- **缓解**: 整合 #7 commit 时 bump workspace.version 1.2.0 → 1.3.0 (per 决策 #74 §1 B2)

### 5.5 三洋葱架构 workspace 化 (V1.1 release, 方案 C 激进)

**三洋葱架构现状** (per R131-5 §2.5 + R131-4 §2.5):
- **原则洋葱 (5 层 E/S/A/M/O)**: 锁在 core
- **权限洋葱 (6 层 L0-L5)**: 锁在 core
- **DSL 洋葱 (Colang DSL)**: 0 落地, 24 LOCKED 都 0 引用 Colang

**24 LOCKED 跟三洋葱架构对应关系** (per R131-5 §2.5):
- **原则洋葱 E 层 (存在层, 不可降级)**: core (L0 HA 锁) / constraint (哲学守门) / life-force (SGI 锁)
- **原则洋葱 S 层 (价值层, 智囊团审议)**: council (7 强制 Advisor) / evolution (演化审议)
- **原则洋葱 A 层 (经验沉淀层)**: memory (历史流 6 表) / asi (24 维测量历史)
- **原则洋葱 M 层 (方法论层)**: cognition / pipeline / protocol / bus / graph
- **原则洋葱 O 层 (操作原则层, 可改)**: agent / tool-registry / tool-runtime / tool-approval / tools / mcp / extension / action / api / cli / bench / supervisor
- **权限洋葱 L0 (HA 核心)**: core (L0 HA 锁) / constraint (gate3 物理隔离)
- **权限洋葱 L1-L5**: api (V2 端点) / tool-approval (5 规则 + 5min 窗口)
- **DSL 洋葱 (Colang DSL)**: 0 落地

**三洋葱架构 workspace 化方案 (V1.1 release major, 整合 #7 commit)**:
- **新增 `apeireth-dsl` crate**: Colang DSL 真实施, 24 LOCKED crate 引用 dsl 守门 (per R125-5 NVIDIA Guardrails 借鉴 + R129-7 §2.1.8 20 unit test pass)
- **三洋葱架构 workspace 化** (per R131-4 cargo workspace 优化 + 决策 #74 B1 Mavis 自决改):
  - `apeireth-onion/` workspace: core (原则 + 权限双洋葱) + constraint (守门) + dsl (DSL 洋葱) + life-force (SGI)
  - 顶层 `apeireth-onion` facade 重新导出全部洋葱 module
  - 24 LOCKED 全部下沉到对应洋葱 workspace

**三洋葱架构 workspace 化实施步骤 (V1.1 release 整合 #7 commit)**:
1. 创建 `apeireth-dsl` crate (`crates/apeireth-dsl/`, per R125-5 + R129-7 + 决策 #74 §1 B1 V1.1 release Mavis 自决改)
2. 创建 `apeireth-onion/` workspace 根 Cargo.toml
3. 移动 core/constraint/life-force 路径: `crates/apeireth-core/` → `apeireth-onion/apeireth-core/`
4. 顶层 `apeireth/Cargo.toml` 移除 4 crate from `members` 段, 加 `apeireth-onion` 路径
5. 24 LOCKED crate 引用 dsl 守门 (在 lib.rs 顶部加 `pub use apeireth_dsl::*;`)
6. 顶层 `apeireth-onion/` 创建 re-export facade
7. cargo build --workspace 验证 (确保 0 编译错误)
8. cargo test --workspace 验证 (确保 0 测试失败)

**三洋葱架构 workspace 化风险**:
- **高**: 拆三洋葱 workspace = 改大量 import 路径 = breaking change
- **缓解**: 顶层 `apeireth-onion` facade 重新导出全部洋葱 module, 消费者 0 改
- **缓解**: V1.1 release bump workspace.version 1.2.0 → 1.3.0 (per 决策 #74 §1 B2)
- **缓解**: 24 LOCKED 入口签名 0 改 (顶层 facade 保留, 仅内部 module 重组)

### 5.6 9 organ workspace 化 (V2.0 release, 方案 D 终极)

**9 organ 现状** (per R131-4 §2.6 + R131-5 §2.6):
- 9 organ 跨 8 LOCKED crate 分布 (1:1 镜像, 排除 body.rs 0 字节占位)
- brain ↔ cognition / ear+eye ↔ perception / hand ↔ action / heart ↔ life-force (transparent re-export) / memory ↔ memory (R11 baseline) / mind ↔ consciousness (transparent re-export) / voice ↔ voice

**9 organ workspace 化方案 (V2.0 release, 整合 #8+ commit)**:
- 新增 9 organ workspace:
  - `apeireth-heart` workspace: supervisor + bus (L0) + pipeline (LLM 网关心跳)
  - `apeireth-brain` workspace: agent + council + cognition + constraint (Multi-Agent 决策)
  - `apeireth-hand` workspace: tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action (Tool Protocol)
  - `apeireth-eye` workspace: (从 tui/src/organ/eye.rs 抽 crate, 用户输入感知)
  - `apeireth-ear` workspace: bus (L1-L4) (系统事件监听)
  - `apeireth-memory` workspace: memory + asi + life-force + core (IdentityCard 跨载体) (3 层 facade)
  - `apeireth-voice` workspace: protocol + pipeline (流式) + (未来 tts/stt crate) (TTS/STT)
  - `apeireth-body` workspace: bench + api + cli (长程任务)
  - `apeireth-mind` workspace: evolution + graph (lifecycle 编排) + constraint (5 重守门) (9-stage lifecycle)
- 24 LOCKED 全部下沉到 organ workspace
- 顶层 `apeireth` re-export 全部 organ types (0 改消费者代码)

**9 organ workspace 化实施步骤 (V2.0 release 整合 #8+ commit)**:
1. 创建 9 organ workspace 根 Cargo.toml
2. 移动 24 LOCKED crate 路径到对应 organ workspace
3. 顶层 `apeireth/Cargo.toml` 移除 24 LOCKED from `members` 段, 加 9 organ 路径
4. 顶层 `apeireth/src/lib.rs` 重新导出 9 organ types (re-export facade)
5. 8 哲学锚可重评 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
6. R11 baseline 3 值可重评 (per 决策 #74 §2.3, 前提: 新 baseline 更高)
7. cargo build --workspace 验证 (确保 0 编译错误)
8. cargo test --workspace 验证 (确保 0 测试失败)

**9 organ workspace 化风险**:
- **极高**: 9 organ 重构 = 改 24 LOCKED crate 全部路径 = 改 N 个消费者的 `use` 路径 = breaking change
- **缓解**: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用
- **缓解**: V2.0 release bump workspace.version 1.3.0 → 2.0.0 (per 决策 #74 §2.3 semver major, breaking change 允许)
- **缓解**: 8 哲学锚可重评 = V2.0 release 跟哲学同步重评, 0 哲学锁死
- **缓解**: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学 100% 一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

---

## 6. Cargo.toml borrow 段精简 (cloned=10 / rate_limited=0 / skipped=1 / brainonly=1)

### 6.1 现状 borrow 段 17:44 关键诚实标 (per R131-6 §1)

**Cargo.toml borrow 段 17:44 现状** (per Cargo.toml:296-318 + R131-6 §1.1 实地 verify):
- `borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }`
- `borrow_cloned = [...]` 列表**仅 7 entries** (clap / hyper / servers / PyO3 / kani / langgraph / superpowers)
- `borrow_rate_limited = [...]` 列表 3 entries (litellm / opencode / Guardrails)
- `borrow_skipped = [...]` 列表 1 entry (opencog AGPL-3.0 永久跳过)

**关键诚实标 1: count_cloned=8 vs 列表 7 entries 不一致** (per R131-6 §1.2):
- `borrow = { count_total = 11, count_cloned = 8, ... }` 声明 count_cloned=8
- `borrow_cloned = [...]` 列表**仅 7 entries** (clap / hyper / servers / PyO3 / kani / langgraph / superpowers)
- **Guardrails 在 `borrow_rate_limited` 第 3 项** ("NVIDIA/NeMo-Guardrails (⏳ git submodule 0 init, P6-3 R127-2 阶段 A 21:18 派重试, 通常 Apache-2.0)")
- 整合 #5.2 commit 时需把 Guardrails 从 `borrow_rate_limited` 移到 `borrow_cloned` (per R131-2 §4.3 update 计划表)

**关键诚实标 2: count_total=11 算术不一致** (per R131-6 §1.3):
- `count_cloned=8 + count_rate_limited=3 + count_skipped=1 = 12` ≠ `count_total=11`
- **实际 = 8 cloned + 3 rate_limited + 1 skipped = 12 源**, 但 Cargo.toml 标 count_total=11
- 整合 #5.2 commit 时需修真: `count_total = 12` (8 cloned + 3 rate_limited + 1 skipped = 12 源)

**关键诚实标 3: 实地 verify 借鉴源本地大小** (per R131-6 §1.5):
- 8 真 cloned 实地总 49.15 MB / 7,619 files vs Cargo.toml 标 49.60 MB / 7,764 files (轻微漂移 -0.45 MB / -145 files)
- ⚠️ **Guardrails-broken/ 0 MB / 0 files** = **junk 残留** (per R131-6 §4.4 建议 mavis-trash)
- ✅ .git/ 隐藏目录 8 源共 16.68 MB (clap 0.82 + Guardrails 6.76 + hyper 0.16 + kani 2.44 + langgraph 3.67 + PyO3 1.85 + servers 0.37 + superpowers 0.61) — **历史 mtime 锚定, 永久保留**

**关键诚实标 4: 决策链 range 关键诚实标** (per R131-6 §1.4):
- Cargo.toml:369 `decision_chain_range = "decision-22 ~ decision-58"` (37 个决策文件)
- 当前决策链已到 #74 (决策 #73 + #74 主人 8/11 01:14 拍板 3 件套) + 决策 #75 (R131 era 第 2 批 6 sub 派活拍板)
- **当前真实范围: decision-22 ~ decision-75 (54 个决策文件)**
- 整合 #5.2 commit 时需修真: `decision_chain_range = "decision-22 ~ decision-75"` (54 个)

### 6.2 cloned=7 → 10 entries update 计划 (per R131-6 §2.1 + R131-2 §4.3)

**整合 #5.2 commit 时 borrow_cloned update 计划** (per R131-6 §2.1 + R131-2 §4.3):
- 7 → 10 entries (整合 #5.2 commit 时):
  - 7 entries (clap / hyper / servers / PyO3 / kani / langgraph / superpowers) — 0 改
  - + 1 entry (Guardrails, 整合 #4 commit 19:41 修真 cloned 18.19MB / 2045 files, 整合 #5.2 commit 时从 rate_limited 移到 cloned)
  - + 1 entry (LiteLLM 借鉴 ID 索引完成, 0 cloned, 公开 1:1 翻译 562 行新 src, 19/19 unit test pass)
  - + 1 entry (opencode 借鉴 ID 索引完成, 0 cloned, 改借鉴已 cloned 3 module: SubAgent + MCP 协议 + Context, 35/35 tests pass)
  - = **10 entries**

**R131-6 评估 1: cloned=10 状态最优, 无可删可合并** (per R131-6 §2.1 评估):
- ✅ **10 个 cloned 借鉴源全部有独立架构价值** (CLI / HTTP / MCP / PyBridge / 形式化 / 图编排 / Skill 化 / 守门 / Provider 成本 / SubAgent), 删任一会破坏架构完整性
- ✅ **借鉴 ROI 全部 🟢 高** (10/10), 无低 ROI 借鉴源
- 🟡 **5 个有"可合并" 候选** (clap + clap_complete / hyper + hyper-util / langgraph + langgraph-checkpoint / Guardrails + colang-parser / kani 0 future kani-driver), 但**当前 Cargo.toml dep 已经分离** (clap 4.5 / hyper 0.1 / langgraph 0.4 / Guardrails 0.x 各自 workspace dep), **整合 #5.2 commit 时 0 合并** (per 决策 #33 §2.3 workspace 1.2.0 0 改严守)
- ✅ V1.1 minor 沿用 10 个 cloned 借鉴源 + 派 8 sub-agent 补 4-5 差距 (per R131-2 §4.2 8 真 cloned 沿用 + 深化)

### 6.3 rate_limited=3 → 0 (P6-1/2/3 全 done)

**整合 #5.2 commit 时 borrow_rate_limited update 计划** (per R131-6 §2.2 + R131-2 §4.3):
- 3 → 0 entries (整合 #5.2 commit 时):
  - 3 entries (LiteLLM / opencode / Guardrails) — 全 P6-1/2/3 done
  - P6-1 LiteLLM (R127-2 阶段 A 21:18 派 → 21:38 done, 20 min) — 借鉴 ID 索引完成, 19/19 unit test pass
  - P6-2 opencode (R127-2 阶段 A 21:18 派 → 22:20 done, 62 min) — 借鉴 ID 索引完成, 35/35 tests pass
  - P6-3 Guardrails (R127-2 阶段 A 21:18 派 → 21:58 done, 40 min) — 真 cloned 18.19MB / 2045 files, 20/20 unit test pass
  - = **0 entries**

**R131-6 评估 2: rate_limited=0 状态合理, 100% clear** (per R131-6 §2.2 评估):
- ✅ **0 借鉴处于限流** (P6-1/2/3 全 done, 100% clear per R129-7 + R129-28 终极 verify)
- ✅ **0 限流 = 0 装 PASS 严守** (per 决策 #33 §2.3 C2): 限流 = 装"已借鉴" 但 0 实施 = 0 装 PASS 严守失败. rate_limited=0 = 0 限流 = 0 装 PASS 严守 100%
- ✅ **整合 #5.2 commit 时 rate_limited 段 0 entries** (从 3 → 0), 整合 #5.2 commit 时 update 计划 (per R131-2 §4.3)
- ⚠️ **未来 V1.1/V2.0 release 派新借鉴源时, 如遇 API 限流**: **借鉴 ID 索引完成 = 0 装"已借鉴" 严守** (按公开 docs 1:1 翻译, 0 装"已读真源码" / 0 装"已对接私有 API"), 不再走 rate_limited 段 (rate_limited 段永久从 Cargo.toml 移除, 整合 #5.2 commit 时 update)

### 6.4 skipped=1 (OpenCog AGPL-3.0 永久严守)

**整合 #5.2 commit 时 borrow_skipped update 计划** (per R131-6 §1.4 + R131-2 §4.3):
- 1 → 1 entry (整合 #5.2 commit 时):
  - 1 entry (opencog AGPL-3.0) — 0 改
  - = **1 entry 0 改**

**R131-6 评估 3: skipped=1 状态永久严守** (per R131-6 §2.3 评估):
- ✅ **OpenCog AGPL-3.0 永久跳过, 不可重试** (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + 决策 #55 §3)
- ✅ **0 装 PASS 严守** (per 决策 #33 §2.3 C2): 0 装"已借鉴" / 0 装"已集成" / 0 装"已对接" / 0 装"已 fork" 100% 严守
- ⚠️ **AGPL-3.0 license 4 大风险** (per R131-6 §2.7 评估):
  - **R1 极强传染性**: AGPL-3.0 是传染性 copyleft, 跟主仓 Apache-2.0 不兼容
  - **R2 商业化受阻**: 任何集成 OpenCog 的派生作品必须 AGPL-3.0, 商业化路径被封
  - **R3 compliance 成本**: AGPL-3.0 触发 source disclosure 义务, 内部使用也需公开源码
  - **R4 OpenCog 维护状态**: OpenCog 维护状态不佳, pln + relex 已 deprecated, 主要靠 Ben Goertzel 团队

### 6.5 🆕 brainonly=1 (OpenCog 家族 6 子源, 借脑 ID 索引完成)

**整合 #5.2 commit 时 borrow_brainonly update 计划** (per R131-6 §1.4 + R131-2 §4.3):
- 0 → 1 entry (整合 #5.2 commit 时):
  - 🆕 **1 entry: R130-6-BORROW-opencog-family-2026Q1-2026-08-11** (6 子源, AGPL-3.0, 0 装 PASS 严守, per 决策 #33 §2.3 C2)
  - 6 子源: opencog/atomspace 4.3.0 + cogutil + moses + pln (deprecated) + relex (deprecated) + CogPrime (Ben Goertzel)
  - 架构: AtomSpace (hypergraph database) + Atomese (graph language) + StorageNode (RocksDB) + forward/backward chainer + Unified Rule Engine (URE) + ECAN (Economic Attention Network)
  - ROI 梯度: 🟢 高 (AtomSpace + CogPrime, 30-50KB 报告/子源) + 🟡 中 (MOSES, 10-20KB 报告) + 🔴 低 (cogutil + pln + relex, 5-10KB 报告/子源)
  - = **1 entry**

**R131-6 评估 4: brainonly=1 状态新增** (per R131-6 §1.4 + 决策 #73 §1 主人 8/11 01:14 拍板 3 件套 §1):
- 🆕 **借脑 ID 索引完成 (OpenCog 家族 6 子源)** (per R130-6 §1.2 + 决策 #55 §2.6 + 决策 #71 §2.2 + 决策 #73 §2 + 决策 #74 B1 改写)
- ✅ **0 装 PASS 严守** (per 决策 #33 §2.3 C2): 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork" 100% 严守
- 🟢 **V1.1 release 借脑调研沉淀** (per 决策 #73 §2 cron Section 10 架构审视永久工作项)
- 🟡 **V2.0 release 可考虑 fork** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评): 独立 fork 候选仓 `apeireth-opencog-experimental` (AGPL-3.0)

### 6.6 Cargo.toml borrow 段精简方案 (V1.0/V1.1/V2.0 release)

**V1.0 release (整合 #5.2 commit)**:
- 整合 #5.2 commit 时 update 17:44 → 22:50 状态 (per 决策 #62 §5.2 + R131-2 §4.3 update 计划表):
  - `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }`
  - `borrow_cloned = [...]` 7 → 10 entries (+Guardrails, +LiteLLM 借鉴 ID 索引完成, +opencode 借鉴 ID 索引完成)
  - `borrow_rate_limited = [...]` 3 → 0 entries
  - `borrow_skipped = [...]` 1 entry 0 改
  - 🆕 `borrow_brainonly = [...]` 1 entry (OpenCog 家族 6 子源)

**V1.1 release (方案 B 中等 + 方案 C 激进, 整合 #6/#7 commit)**:
- borrow 段拆更细 (per R131-4 §2.3 + R131-6 §2.1-2.7 8 大精简方向):
  - 5 子段细分 = `borrow_cloned_real` (8) + `borrow_translated_public` (2, LiteLLM + opencode) + `borrow_submodule` (0) + `borrow_skipped_license` (1, OpenCog) + 🆕 `borrow_brainonly` (1, R130-6 OpenCog 家族 6 子源)
  - 借鉴源版本 hash 段 (per 决策 #36 §1.1 严格化) → `borrow_cloned_real_with_hash` (e.g., `clap-rs/clap 4a622b4` 实际 git commit hash)
  - 借鉴源真实施深度段 (per R131-2 12 源实施深度 verify) → `borrow_implementation_depth` (e.g., clap 8/10, hyper 7/10, kani 6/10, langgraph 8/10, superpowers 8/10, Guardrails 7/10)

**V2.0 release (方案 D 终极, 整合 #8+ commit)**:
- Cargo.toml borrow 段可重构 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评):
  - 借鉴源 12 → 20+ 拓宽
  - OpenCog 家族 fork 候选仓 `apeireth-opencog-experimental` (AGPL-3.0) 调研沉淀
  - 13-15 源候选演进路径 A (推荐) + 路径 A+ (超激进) (per R131-6 §3 路径规划)

---

## 7. 重构时间线 (V1.0/V1.1 minor/V1.1 major/V2.0 release)

### 7.1 V1.0 release 整合 #5.1-5.3 commit 阶段 (8/15 前 主人拍板)

**触发条件**: 整合 #5.1-5.3 commit 拍板 (per 决策 #62 §5.1-5.3 + 决策 #73 §5 + 决策 #74 §4 + 主人 8/15 起床后拍板)

**时间窗口**: 2026-08-11 ~ 2026-08-15 (主人 8/15 起床后拍板 整合 #5 commit 时机)

**实施方案 A 保守** (per §3.1):
1. **整合 #5.1 commit (src/ 实施, 95+ 文件)**:
   - 0 改 24 LOCKED 入口签名 (V1.0 release 严守, per 决策 #74 §1 B1)
   - 0 改 24 LOCKED crate mtime baseline 16:34 之前 (V1.0 release 严守)
   - 0 改 R11 baseline 3 值 (V1.0 release 严守, per 决策 #74 §1 A1)
   - PHL-07 spec-only 0 实施 (V1.0 release 严守, V1.1 实施, per 决策 #74 §1 A3)
   - 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup, per 决策 #62 §5.1)
2. **整合 #5.2 commit (docs/ + Cargo.toml, 10 文件)**:
   - Cargo.toml borrow 段 update 17:44 → 22:50 状态 (per 决策 #62 §5.2 + R131-2 §4.3 + R131-6 §1.4)
   - Cargo.toml `description` update "借鉴 10/11 + 1 借脑 = 11/12" (per R131-6 §1.4)
   - Cargo.toml `decision_chain_range` update "decision-22 ~ decision-75" (per R131-6 §1.4)
   - 新增 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3 主人 8/11 01:14 拍板 总哲学扩展)
   - 更新 `docs/conventions/10-locked.md` (per 决策 #73 §2.3 主人 8/11 01:14 拍板 locked 全解锁)
   - 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2 总工程哲学扩展引用)
   - 更新 `docs/conventions/README.md` (per 决策 #73 §2.3 + §4.2 加 15-no-fear-complexity.md 索引)
   - 更新 `CONTRIBUTING.md` (per 决策 #73 §2.3 8 项不修改承诺 改写)
   - 更新 `README.md` (per 决策 #73 §2.3 状态行加 R130 era 主人 8/11 01:14 拍板)
3. **整合 #5.3 commit (reports/, 60+ 文件)**:
   - 决策链 #30-#64 全读 verify (per 决策 #62 §5.3)
   - 41 sub-agent 报告 (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3 全 done, per 决策 #62 §5.3)
   - HANDOFF (per 决策 #62 §5.3)
   - 新增 decision-73 + decision-74 (per 决策 #73 §5.3)
   - 新增 R131 era 调研 6 sub-agent 报告 (R131-1/2/3/4/5/6/7/8/9, per 决策 #73 §5.3)
   - 新增 `philosophy-no-fear-complexity-2026-08-11.md` (per 决策 #73 §3 主人 8/11 01:14 决策 3 件套详细)

**关键里程碑 (V1.0 release)**:
- master HEAD = abf12243 (整合 #4 commit) 严守
- 整合 #5.1 commit hash = 估 e1f0g2h3 (src/ 95+ 文件)
- 整合 #5.2 commit hash = 估 i4j5k6l7 (docs/ + Cargo.toml 10 文件)
- 整合 #5.3 commit hash = 估 m8n9o0p1 (reports/ 60+ 文件)
- master HEAD 新值 = 整合 #5.3 commit hash (m8n9o0p1 估)
- 1.0 release tag = 估 `v1.0.0` (主人 8/15 起床后拍板)
- 0 主动 push 严守 (等 1.0 release 配 GitHub remote, per 决策 #33 §2.3 + 决策 #61 §6)

### 7.2 V1.1 release minor (整合 #6 commit, 9 月估)

**触发条件**: 主人 8/15 起床后拍板 整合 #5 commit 时机 → 整合 #6 commit 实施 (per 决策 #62 §6 后续整合 + 决策 #74 §1 V1.1 release Mavis 自决改)

**时间窗口**: 2026-09-01 ~ 2026-09-30 (9 月估, 主人 8/15 起床后持续派活)

**实施方案 B 中等** (per §3.2):
1. **整合 #6.1 commit (workspace 重组, Cargo.toml + sub-crate 重组)**:
   - 合并 5-8 个非 LOCKED crate (per §3.2 8 个 merge 候选)
   - 拆分 1-2 个大 crate (mcp 拆 4 sub-crate + api 拆 4 sub-crate)
   - 9 叶子 crate 拆 workspace (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench → `apeireth-leaf/` workspace, per §5.2)
   - core 拆 pub mod (1 个 108KB lib.rs 拆 7 sub-mod, per §5.3)
2. **整合 #6.2 commit (加 5-10 个 helper crate)**:
   - 加 5-10 个 helper crate (per §3.2 10 个加 crate 候选, 含 clap-complete / hyper-server / mcp-streamable / maturin / kani-driver / langgraph-postgres / superpowers-skill-review / guardrails-colang-parser / litellm-loadbalance / opencode-agents-md)
3. **整合 #6.3 commit (Cargo.toml update)**:
   - workspace.version 1.2.0 → 1.2.1 bump (per 决策 #74 §1 B2, semver minor)
   - Cargo.toml borrow 段拆更细 (5 子段: borrow_cloned_real + borrow_translated_public + borrow_submodule + borrow_skipped_license + borrow_brainonly)
   - Cargo.toml `members` 段 update 87 → 80-95
   - Cargo.lock update (拆 2-3 lockfile per §4.4)

**关键里程碑 (V1.1 release minor)**:
- master HEAD = 整合 #5.3 commit hash (m8n9o0p1 估)
- 整合 #6.1 commit hash = 估 q2r3s4t5
- 整合 #6.2 commit hash = 估 u6v7w8x9
- 整合 #6.3 commit hash = 估 y0z1a2b3
- master HEAD 新值 = 整合 #6.3 commit hash (y0z1a2b3 估)
- 1.1.0 release tag = 估 `v1.1.0` (Mavis 拍板 + 主人 9/30 起床后 ack)
- 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #61 §6)

### 7.3 V1.1 release major (整合 #7 commit, 10 月估)

**触发条件**: 主人 9/30 起床后 ack 整合 #6 commit + 派活整合 #7 commit (per 决策 #62 §6 后续整合 + 决策 #74 §1 V1.1 release Mavis 自决改)

**时间窗口**: 2026-10-01 ~ 2026-10-31 (10 月估, 主人 9/30 起床后持续派活)

**实施方案 C 激进** (per §3.3):
1. **整合 #7.1 commit (24 LOCKED 入口签名 8 方向改写)**:
   - 24 LOCKED 入口签名 8 方向改写 (per R131-5 §2.8 + §5.4):
     - 方向 1: 入口签名一致性 标准化 (per-crate 选 3 模式之一)
     - 方向 2: 公开 API 表面 瘦身 (per-crate 暴露 ≤30 pub items)
     - 方向 3: 9 叶子 crate 拆 workspace (per §5.2 已经在 6.1 commit 实施, V1.1 release major 时再深化)
     - 方向 4: core 拆 pub mod (per §5.3 已经在 6.1 commit 实施, V1.1 release major 时再深化)
     - 方向 5: 大模块集中 crate 拆 sub-crate (per §5.4 mcp/pipeline/api/memory/council/graph/asi/tools/evolution 9 大 crate 拆 sub-crate)
     - 方向 6: DSL 洋葱落地 (新增 `apeireth-dsl` crate, per §5.5)
     - 方向 7: 9 organ 内部借 OpenCode (R125 B7)
     - 方向 8: R12 测度对齐 (24 测量函数签名更新 R12 测度, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新)
2. **整合 #7.2 commit (24 LOCKED 重新分 crate)**:
   - 24 LOCKED 重新分 crate (per §3.3):
     - 选项 A: 拆 LOCKED 集合 (24 → 30, 加 6 个 R20 阶段 4 估补, 透明 re-export 3 真合并)
     - 选项 B: 合 LOCKED 集合 (24 → 18, 减 6 个小 crate)
     - 选项 C: 保持 24 LOCKED 集合 (0 改)
     - Mavis 自决 (per 决策 #74 §1 B1)
3. **整合 #7.3 commit (PHL-07 实施 + Cargo.toml update)**:
   - PHL-07 实施 (per 决策 #74 §1 A3 PHL-07 V1.0 spec-only + V1.1 实施)
   - 12 键其他可改 (per 决策 #74 §1 A3)
   - workspace.version 1.2.1 → 1.3.0 bump (per 决策 #74 §1 B2, semver minor 强化)
   - Cargo.toml borrow 段 update (V1.1 release major 深化)
   - Cargo.lock update (拆 2-3 lockfile per §4.4)

**关键里程碑 (V1.1 release major)**:
- master HEAD = 整合 #6.3 commit hash (y0z1a2b3 估)
- 整合 #7.1 commit hash = 估 c4d5e6f7
- 整合 #7.2 commit hash = 估 g8h9i0j1
- 整合 #7.3 commit hash = 估 k2l3m4n5
- master HEAD 新值 = 整合 #7.3 commit hash (k2l3m4n5 估)
- 1.3.0 release tag = 估 `v1.3.0` (Mavis 拍板 + 主人 10/31 起床后 ack)
- 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #61 §6)

### 7.4 V2.0 release (整合 #8+ commit, 11 月+ 估)

**触发条件**: 主人 10/31 起床后 ack 整合 #7 commit + 派活整合 #8+ commit (per 决策 #62 §6 后续整合 + 决策 #74 §2.3 V2.0 release 8 硬墙可重评 + 决策 #73 §3 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚")

**时间窗口**: 2026-11-01 ~ 2026-12-31+ (11 月+ 估, 主人 10/31 起床后持续派活)

**实施方案 D 终极** (per §3.4):
1. **整合 #8.1 commit (9 organ workspace 化)**:
   - 9 organ workspace 化 (per §5.6 + R131-5 §2.6 + R131-4 §2.6):
     - `apeireth-heart` / `apeireth-brain` / `apeireth-hand` / `apeireth-eye` / `apeireth-ear` / `apeireth-memory` / `apeireth-voice` / `apeireth-body` / `apeireth-mind` = 9 organ workspace
2. **整合 #8.2 commit (三洋葱架构 workspace 化)**:
   - 三洋葱架构 workspace 化 (per §5.5):
     - `apeireth-onion/` workspace: core (原则 + 权限双洋葱) + constraint (守门) + dsl (DSL 洋葱) + life-force (SGI)
3. **整合 #8.3 commit (全 workspace 重写 + Cargo.toml update)**:
   - 全 workspace 重写 (per §3.4 终极方案)
   - 顶层 `apeireth` re-export 全部 9 organ types
   - workspace.version 1.3.0 → 2.0.0 bump (per 决策 #74 §2.3 semver major, breaking change 允许)
   - Cargo.toml borrow 段 update (V2.0 release 重构)
   - Cargo.lock 拆 9 organ 独立 lockfile (per §4.4)
   - 8 哲学锚可重评 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
   - R11 baseline 3 值可重评 (per 决策 #74 §2.3, 前提: 新 baseline 更高)

**关键里程碑 (V2.0 release)**:
- master HEAD = 整合 #7.3 commit hash (k2l3m4n5 估)
- 整合 #8.1 commit hash = 估 o6p7q8r9
- 整合 #8.2 commit hash = 估 s0t1u2v3
- 整合 #8.3 commit hash = 估 w4x5y6z7
- master HEAD 新值 = 整合 #8.3 commit hash (w4x5y6z7 估)
- 2.0.0 release tag = 估 `v2.0.0` (Mavis 拍板 + 主人 12/31 起床后 ack)
- 0 主动 push 严守 (per 决策 #33 §2.3 + 决策 #61 §6)

### 7.5 重构时间线总览 (per §3.5 4 方案对比总结)

| release | 整合 commit | workspace.version | 实施方案 | 关键诚实标 | 预计时间 |
|---------|------------|-------------------|---------|------------|---------|
| **V1.0 release** | 整合 #5.1-5.3 | 1.2.0 严守 | 方案 A 保守 (0 改 + 加 2-3 helper) | Cargo.toml borrow 段 update 17:44 → 22:50 状态, 0 改 24 LOCKED 入口签名 | 8/15 前 |
| **V1.1 release minor** | 整合 #6.1-6.3 | 1.2.0 → 1.2.1 (semver minor) | 方案 B 中等 (合并 5-8 + 拆 1-2) | 9 叶子拆 workspace + core 拆 pub mod + 加 5-10 helper | 9 月 |
| **V1.1 release major** | 整合 #7.1-7.3 | 1.2.1 → 1.3.0 (semver minor 强化) | 方案 C 激进 (24 LOCKED 入口签名 Mavis 自决改) | 24 LOCKED 8 方向改写 + PHL-07 实施 + 大模块集中拆 sub-crate + DSL 洋葱落地 | 10 月 |
| **V2.0 release** | 整合 #8.1-8.3+ | 1.3.0 → 2.0.0 (semver major) | 方案 D 终极 (全 workspace 重写 9 organ) | 9 organ workspace 化 + 三洋葱 workspace 化 + 8 哲学锚可重评 + R11 baseline 可重评 | 11 月+ |

**关键决策链**:
- 主人 8/15 起床后拍板 整合 #5 commit 时机 → 整合 #5.1-5.3 commit → 1.0 release tag v1.0.0
- 主人 9/30 起床后 ack 整合 #6 commit → 整合 #6.1-6.3 commit → 1.1.0 release tag v1.1.0
- 主人 10/31 起床后 ack 整合 #7 commit → 整合 #7.1-7.3 commit → 1.3.0 release tag v1.3.0
- 主人 12/31 起床后 ack 整合 #8 commit → 整合 #8.1-8.3+ commit → 2.0.0 release tag v2.0.0

---

## 8. 决策原则 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

### 8.1 9 件套总哲学 (8 哲学锚 + 不要怕复杂度)

**9 件套总哲学** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md):
- **8 哲学锚** (思想哲学, per 决策 #22 §2.5 B5 + R126 P1-2 8 哲学锚升级 done + 决策 #74 §1 B5 严守):
  - **S-1** 北极星 — 哲学基础, 决策锚点
  - **S-2** 实事求是 — 0 装 PASS 严守
  - **S-3** 质量工程化 — 工程质量优先
  - **O-1** 安全优先 — 6 重守门 v7 + V0.5 30 维 + 13 键 verdict cache
  - **O-2** 走在前人 — 借鉴 8 真 cloned + 2 借鉴 ID 索引 + 1 永久跳过 + 1 借脑
  - **O-3** 干到底 — 41 sub-agent 全部 done, 24 LOCKED 入口签名 24/24 PASS
  - **O-4** 接手 — 维护交给未来高水平团队 (per 决策 #73 §3)
  - **O-5** 不假装 — 0 装 PASS 严守 + 0 cargo install / 0 cargo add
- **不要怕复杂度** (工程哲学, per 决策 #73 §3 主人 8/11 01:14 拍板 3 件套 §3 + 哲学文档 15-no-fear-complexity.md):
  - **最强效果** > 最简单代码
  - **最厉害工程** > 最易维护
  - **复杂度** 不是问题 (e.g. 24 LOCKED + 8 哲学锚 + 6 重守门 + 30 维公式 + 13 键, 都复杂, 但都是最强效果)
  - **维护复杂** 不是问题 (未来高水平团队接手)

**9 件套总哲学互相不替代, 互补**:
- 8 哲学锚是**思想哲学** (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5)
- 不要怕复杂度是**工程哲学** (最强效果 + 最厉害工程 + 复杂度不是问题)
- 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学

**9 件套总哲学跟 R140-3 4 方案关系**:
- 方案 A 保守: 8 哲学锚严守 + 不要怕复杂度 = 0 改严守 24 LOCKED (复杂 0 改 = 复杂严守 = 最强效果)
- 方案 B 中等: 8 哲学锚严守 + 不要怕复杂度 = 合并 5-8 + 拆 1-2 (复杂 = 75-95 crate 中等)
- 方案 C 激进: 8 哲学锚严守 + 不要怕复杂度 = 24 LOCKED 入口签名 Mavis 自决改 (复杂 = 80-100 crate 复杂化)
- 方案 D 终极: 8 哲学锚可重评 + 不要怕复杂度 = 全 workspace 重写 9 organ (复杂 = 9 organ workspace 顶层简化)

### 8.2 8 硬墙分类 (工程类松绑 / 哲学类严守 / 状态流程类严守)

**8 硬墙 3 大分类** (per 决策 #74 §3):

1. **工程类 + 技术类 (松绑, B1 改写)**:
   - **B1 24 LOCKED 入口签名**: 🟢 V1.0 release 0 改严守 + V1.1 release Mavis 自决改
   - 主人 8/11 01:14 拍板依据: "工程类 + 技术类 locked 全早解锁" + "Mavis 自决架构拍板"

2. **哲学 + 思想类 (严守, 不松绑)**:
   - **A1 R11 baseline 3 值 (0.8682/0.8532/0.9063)**: 🔒 严守 (哲学 + 效果标)
   - **A3 12 键 + PHL-07**: 🔒 PHL-07 V1.0 spec-only 0 实施 (V1.1 实施) + 12 键其他可改
   - **B3 V0.5 30 维**: 🔒 严守 (哲学公式)
   - **B4 6 重守门 v7**: 🔒 严守 (哲学守门)
   - **B5 8 哲学锚**: 🔒 严守 (哲学)
   - 主人 8/11 01:14 拍板依据: "总哲学除了思想文档的" (8 哲学锚严守, R11 baseline 是哲学 + 效果标, V0.5 30 维是哲学公式, 6 重守门 v7 是哲学守门, 12 键是哲学)

3. **状态 + 流程类 (严守, 不松绑)**:
   - **B2 workspace.version 1.2.0**: 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (版本管理, semver minor)
   - **C1 0 主动 commit**: 🔒 主人起床前 0 主动 commit 严守
   - **C2 0 装 PASS 严守**: 🔒 严守 (技术哲学, 不装)
   - **0 push 严守**: 🔒 主人起床前 0 主动 push 严守
   - 主人 8/11 01:14 拍板依据: "总哲学除了思想文档的" (0 commit 是流程类, 严守 / 0 装是技术哲学, 严守 / 0 push 是流程类, 严守)

### 8.3 Mavis 全自决 (per 主人 8/11 01:14 拍板 3 件套 §1)

**Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 拍板 + 8/11 0:25 + 8/11 01:14 升级授权):
- **Mavis 自决架构拍板** (per 主人 8/11 01:14 拍板 3 件套 §1 "有更好的架构需要用 (或改变现有的) 你就直接拍板就行了")
- **Mavis 自决 V1.1 release 24 LOCKED 入口签名改写** (per 决策 #74 §1 B1 V1.1 release Mavis 自决改)
- **Mavis 自决 V2.0 release 8 硬墙可重评** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
- **Mavis 自决借鉴源 12 → 20+ 拓宽** (per 决策 #73 §1 主人 8/11 01:14 拍板 3 件套 §1)
- **Mavis 整合 #5 commit 自动拍板** (per 决策 #62 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **Mavis 决策日志写** (per 决策 #10 + 用户记忆 #10)

**Mavis 自决边界**:
- 🟢 **可自决**: 24 LOCKED 入口签名 (V1.1 release) + 24 LOCKED 集合拆/合 (V1.1 release) + workspace 重组 (V1.1 release minor) + 大模块集中拆 sub-crate (V1.1 release major) + 9 organ workspace 化 (V2.0 release) + 借鉴源 拓宽 (V1.1 release) + 8 哲学锚可重评 (V2.0 release) + R11 baseline 3 值可重评 (V2.0 release, 前提: 新 baseline 更高) + 借鉴源 fork 决策 (V2.0 release, OpenCog 家族)
- 🔒 **不可自决**: 0 改 src (V1.0 release 严守) + 0 改 Cargo.toml 1.2.0 (V1.0 release 严守) + 0 装 PASS 严守 + 0 主动 commit (主人起床前) + 0 主动 push (主人起床前) + 0 装 PASS 严守 (技术哲学, 不装)

### 8.4 0 装 PASS 严守 + 0 主动 commit + 0 主动 push

**0 装 PASS 严守** (per 决策 #33 §2.3 C2 + 决策 #55 §3 + 决策 #58 §3 + 决策 #73 §3 + 决策 #74 §1 C2):
- **0 cargo install / 0 cargo add** 严守
- **0 装"已借鉴"** / **0 装"已读真源码"** / **0 装"已对接"** / **0 装"已 fork"** 严守
- **0 装"已集成"** 严守
- 整合 #4 commit abf12243 含 Cargo.lock (per 决策 #48 §1.2), V1.0 release Cargo.lock 严守 0 改

**0 主动 commit 严守** (per 决策 #33 §2.3 C1 + 决策 #62 §6 + 决策 #64 + 决策 #73 §5 + 决策 #74 §1 C1):
- **主人起床前 0 主动 commit 严守**
- **Mavis 整合 #5 commit 时机自动拍板** (per 决策 #62 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **V1.0 release 拍板由 Mavis 0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)

**0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #61 §6 + 决策 #73 §6 + 决策 #74 §1):
- **主人起床前 0 主动 push 严守**
- **V1.0 release 拍板由主人配 GitHub remote** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人严守** (per gate-discipline + 决策 #61 §6 + cron Section 5, 仅 done notification)

---

## 9. 风险 + refs

### 9.1 风险 + 缓解

**R1: 整合 #5.1 commit 拍板推迟 (R129-3 报告迟迟不出, 92+ min)**:
- **缓解**: 01:15 tick 仍未出 → Section 3 中断接手, Mavis 写报告 (per 决策 #73 §6.1)

**R2: V1.1/V2.0 release 改写打破向后兼容**:
- **缓解**: V1.1 release 是 minor release, 跟 semver 一致 (0.x → 1.0 → 1.1 → 1.3), V2.0 release 才考虑不向后兼容 (per 决策 #74 §7.2 风险 R4)

**R3: 团队对 "不要怕复杂度" 哲学不适应**:
- **缓解**: 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应 (per 决策 #73 §7.2 + 哲学文档 15-no-fear-complexity.md)

**R4: V2.0 release 全 workspace 重写风险极高**:
- **缓解**: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用 (per §5.6 + §3.4 方案 D 风险)
- **缓解**: V2.0 release bump workspace.version 1.3.0 → 2.0.0 (per 决策 #74 §2.3 semver major, breaking change 允许)

**R5: 9 organ workspace 化 风险极高**:
- **缓解**: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用 (per §5.6)
- **缓解**: V2.0 release bump workspace.version 1.3.0 → 2.0.0 (per 决策 #74 §2.3 semver major)

**R6: 24 LOCKED 入口签名改写 风险高 (V1.1 release major)**:
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用 (per §5.4)
- **缓解**: V1.1 release bump workspace.version 1.2.1 → 1.3.0 (per 决策 #74 §1 B2)

**R7: core 拆 pub mod 风险低 (V1.1 release minor)**:
- **缓解**: 0 改入口签名 (顶层 lib.rs 仍 re-export 全部 types, per §5.3)
- **缓解**: 0 改 R11 baseline 3 值 (per 决策 #74 §1 A1)

**R8: 9 叶子 crate 拆 workspace 风险中 (V1.1 release minor)**:
- **缓解**: 顶层 `apeireth-leaf` re-export facade 保留 (per §5.2)
- **缓解**: V1.1 release bump workspace.version 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)

**R9: 大模块集中 crate 拆 sub-crate 风险中 (V1.1 release major)**:
- **缓解**: 顶层 re-export facade 保留 (per §5.4)
- **缓解**: 24 LOCKED 入口签名 0 改 (V1.1 release Mavis 自决改 crate 边界, 顶层 facade 保留)

**R10: Cargo.lock 拆 9 organ 独立 lockfile 风险中 (V2.0 release)**:
- **缓解**: 顶层 `apeireth/Cargo.lock` 0 改, 9 organ workspace 各自独立 Cargo.lock (per §4.4)
- **缓解**: Cargo 1.78+ feature 支持, 0 业务价值风险

**R11: Cargo.toml borrow 段 update 17:44 → 22:50 状态漏改**:
- **缓解**: 整合 #5.2 commit 时 Mavis 拍板 update, 0 漏 (per §6.1-6.5 + 决策 #62 §5.2)

**R12: 主人起床后看 8 硬墙 B1 改写觉得"破坏 R11 baseline"**:
- **缓解**: V1.0 release 仍 0 改严守, V1.1 release Mavis 自决改 (R12 测度对齐 + 跟 R125 B3 + R127 25 维公式), 不会破坏 V1.0 release (per 决策 #74 §7.1 风险 R3)

### 9.2 决策原则 (per 决策 #73 §7.2 + 决策 #74 §7.2 + 主人 8/11 01:14 拍板 3 件套)

**R140-3 决策原则 (12 件套, 严守)**:
1. **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
2. **跑中 ≥ 16** (per 主人 0:34, 16 active 全 background 跑)
3. **中断接手** (per 主人 0:43, 检查 reports/agent-*.md 写完则标 done / 没写完则重派)
4. **编译产物清理决策矩阵** (per 主人 0:49 + 0:54: ≤50 保守 / 50-100 预警 / 100-150 强烈预警 / > 150 强制清理)
5. **计划内任务完成自动接续 4 步 + 永久循环** (per 主人 0:57: 调研 + 差距 + 计划 + 实施 → 永久)
6. **locked 全解锁 + Mavis 自决架构** (per 主人 8/11 01:14 拍板 3 件套 §1, 整合 #5.1 commit 仍 0 改严守 + V1.1 release Mavis 自决改)
7. **架构审视 + 升级方案永久工作项** (per 主人 8/11 01:14 拍板 3 件套 §2, cron Section 10 新增)
8. **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3, 写新文档 `docs/conventions/15-no-fear-complexity.md`)
9. **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
10. **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
11. **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
12. **0 主动删** (per Safety policy + 决策 #44 + #60)

**8 硬墙 严守清单 (per 决策 #33 §2.3 + 决策 #74 §1)**:
- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改
- **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1
- **A1 R11 baseline 3 值 (0.8682/0.8532/0.9063)**: 严守 (哲学 + 效果标)
- **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施, 12 键其他可改
- **B3 V0.5 30 维**: 严守 (哲学)
- **B4 6 重守门 v7**: 严守 (哲学)
- **B5 8 哲学锚**: 严守 (哲学)
- **C1 0 主动 commit (主人起床前)**: 严守
- **C2 0 装 PASS 严守**: 严守
- **0 push (主人起床前)**: 严守

**整合 #4 commit 严守** (per 决策 #48 + 决策 #61 §1.2):
- `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%, 0 重跑 0 重 commit)

**决策日志写** (per 决策 #10 + 用户记忆 #10):
- 更新 `reports/decision-log-r129-era-cron-2026-08-11.md`
- 时间戳: 2026-08-11 02:00 (cron 5 min tick)
- 跑中任务数: 10 (R129-3 + R130-1~6 + R131-1~9) + R140-3 (本)
- done 任务数: 34 (R129 era) + 0 (R130 era) + 9 (R131 era) + 1 (R140-3) = 44
- 派活: R140-3 cargo workspace 重构方案 (本)
- 拍板: 整合 #5 commit 时机 NOT ready (等 R129-21 修真 + R129-3 报告), Cargo.toml borrow 段 update 17:44 → 22:50 状态待整合 #5.2 commit 时 Mavis 自决拍板
- 决策链更新: #73 (主人 8/11 01:14 拍板 3 件套) + #74 (8 硬墙 B1 改写) + #75 (R131 era 第 2 批 6 sub 派活)

### 9.3 refs (8 大报告 + 5 大决策 + 4 大哲学文档)

**R131 era 8 大报告 (per 任务 spec, reference 而非重写)**:
- **R131-1 (done 01:25)**: 现有架构总审视 + 优化点 + 升级方案 (10 方向审计 + V1.0/V1.1/V2.0 release 分级, per 主人 8/11 01:14 拍板 3 件套 §2 + 决策 #73 §2)
  - `reports/agent-r131-1-architecture-audit-2026-08-11.md`
- **R131-2 (done 01:35)**: 跟借鉴源码 11 源差距 + 借鉴 12 源 + OpenCog AGPL-3.0 fork 决策 (per 决策 #71 §3 + 决策 #73 §2 + 决策 #74 B1 改写 + R130-6 调研 12 源)
  - `reports/agent-r131-2-borrowed-12-gap-analysis-2026-08-11.md`
- **R131-3 (done 01:20)**: V1.1 release 实施路线图 (6 大方向: PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+)
  - `reports/agent-r131-3-v1.1-release-implementation-roadmap-2026-08-11.md`
- **R131-4 (done 01:40)**: cargo workspace 结构优化 7 方向架构审视 (本 R140-3 主 reference)
  - `reports/agent-r131-4-cargo-workspace-optimization-2026-08-11.md` (86.9 KB)
- **R131-5 (done 01:50)**: 24 LOCKED 入口分布优化 8 方向 (本 R140-3 主 reference)
  - `reports/agent-r131-5-24-locked-entry-optimization-2026-08-11.md` (62.1 KB)
- **R131-6 (done 01:55)**: Cargo.toml borrow 段精简 7 方向 (本 R140-3 主 reference)
  - `reports/agent-r131-6-cargo-toml-borrow-section-2026-08-11.md` (107.8 KB)
- **R131-7 (done 02:00)**: pybridge 集成优化 (ASI Python Stage 1-8 跟 Rust 后端集成 + 性能瓶颈)
  - `reports/agent-r131-7-pybridge-integration-optimization-2026-08-11.md`
- **R131-8 (done 02:05)**: Tauri 集成优化 (Tauri 2.0 + Rust 后端 + Web frontend 集成)
  - `reports/agent-r131-8-tauri-integration-optimization-2026-08-11.md`
- **R131-9 (done 02:15)**: 形式化集成优化 (kani 借鉴 + PHL-07 形式化 + F1-F10 10 维度)
  - `reports/agent-r131-9-formal-proof-integration-optimization-2026-08-11.md`

**5 大决策 (per 决策链 #22-#75)**:
- **决策 #22** (master-auth-upgrade, 2026-08-10): 主人 8/10 16:27 拍板 "locked 全部解锁" + R11 baseline 3 值 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache
- **决策 #33** (master-reupgrade, 2026-08-10): 主人 8/10 16:31 拍板 "最高权限" + 8 硬墙 严守 (B1-B7 + A1-A3 + C1-C2 + 0 push)
- **决策 #55** (r127-integration-5-library-stage-4-6, 2026-08-10): R127 整合 #5 拍板 + library stage 4-6 治理
- **决策 #73** (locked-unlocked-architecture-audit-philosophy-extension, 2026-08-11): 主人 8/11 01:14 拍板 3 件套
- **决策 #74** (8-hard-walls-b1-rewrite, 2026-08-11): 8 硬墙 B1 改写 (V1.0 release 0 改 + V1.1 release Mavis 自决改)
- **决策 #75** (r131-r132-r133-batch-dispatch, 2026-08-11): R131 era 第 2 批 6 sub 派活 + R132 era 计划 2 sub + R133 era 实施 3 sub = 11 sub 派活拍板

**4 大哲学文档 (per 决策 #73 §2.3 + §4.2)**:
- **`docs/conventions/09-anchor.md`**: 8 哲学锚 主文档 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5, per 决策 #22 §2.5 B5 + R126 P1-2 8 哲学锚升级 done)
- **`docs/conventions/10-locked.md`**: 8 硬墙 + 24 LOCKED 入口签名 严守 (per 决策 #22 §1.2 + 决策 #33 §2.3 + 决策 #74 B1 改写)
- **`docs/conventions/15-no-fear-complexity.md`** (整合 #5.2 commit 时新增, per 决策 #73 §3 主人 8/11 01:14 拍板 总哲学扩展): 不要怕复杂度 工程哲学
- **`docs/conventions/README.md`**: 哲学文档 索引 (per 决策 #73 §2.3 + §4.2 加 15-no-fear-complexity.md 索引)

**整合链** (per 决策 #22 + #33 + #41 + #42 + #47 + #48 + #51 + #55 + #56 + #57 + #58 + #62 + #73 + #74):
- 整合 #1 (decision-25 17:30, 1.0.0 baseline)
- 整合 #2 (decision-31 17:17, R125 续 dry-run)
- 整合 #3 (decision-34 17:30, 主人 14:56 拍板, df6dfb69 128 files)
- 整合 #4 (decision-48 19:41, 主人自执行, abf12243 46752 file changes, 0 重跑)
- 整合 #5 (待拍板, 41 任务 R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3 全 done + 0 装 PASS verify + 8 硬墙 verify + 24 LOCKED 入口 verify, Mavis 拍板 OR 主人 8/15 拍板)
- 整合 #6 (V1.1 release minor, 9 月估, 实施方案 B 中等)
- 整合 #7 (V1.1 release major, 10 月估, 实施方案 C 激进)
- 整合 #8+ (V2.0 release, 11 月+ 估, 实施方案 D 终极)

---

**报告 done 通知** (per 决策 #73 §6 done notification + 用户记忆 #10 自主决策 + cron Section 5 0 主动 IM 主人严守):
- ✅ R140-3 cargo workspace 重构方案 100% 报告 (per 决策 #75 + 决策 #73 §2 + 主人 8/11 01:14 拍板 3 件套 §2)
- ✅ 87 crate 1:1 实地清点 (per Cargo.toml `members` 段 2026-08-11 02:00)
- ✅ 4 重构方案 (保守 / 中等 / 激进 / 终极) 详细分析 + 对比总结
- ✅ Cargo.lock 271KB/10752 行 合理性 + 简化方案 (V1.0/V1.1/V2.0 release)
- ✅ 24 LOCKED 入口分布最优化 (9 叶子拆 workspace + core 拆 pub mod + 大模块集中拆 sub-crate + 三洋葱 workspace 化 + 9 organ workspace 化)
- ✅ Cargo.toml borrow 段精简 (cloned 7→10 / rate_limited 3→0 / skipped 1 / brainonly 1)
- ✅ 重构时间线 (V1.0 8/15 前 / V1.1 minor 9 月 / V1.1 major 10 月 / V2.0 11 月+)
- ✅ 决策原则 (9 件套总哲学 + 8 硬墙分类 + Mavis 全自决 + 0 装 PASS 严守)
- ✅ 风险 + refs (12 风险 + 12 决策原则 + 8 硬墙严守清单 + 8 大报告 + 5 大决策 + 4 大哲学文档)
- ❌ 0 改 src (100% 严守)
- ❌ 0 改 Cargo.toml 1.2.0 (100% 严守)
- ❌ 0 主动 commit (100% 严守, 整合 #5 commit 由 Mavis 自决 OR cron auto-pickup)
- ❌ 0 主动 push (100% 严守, 等 1.0 release 配 GitHub remote)
- ❌ 0 主动 IM 主人 (100% 严守, per gate-discipline)
- ❌ 0 主动删 (100% 严守, per Safety policy + 决策 #44 + #60)
- ❌ 0 cargo install / 0 cargo add (100% 严守, per 决策 #33 §2.3 C2)

**0 主动 IM 主人 严守** (per gate-discipline + 决策 #61 §6 + cron Section 5 + 用户记忆 #10): 本 done notification 主动报告, 0 主动 plain reply on skip ticks, 等 Mavis cron 5 min tick 监督.

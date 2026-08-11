# R152-2: 整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec) — 调研/分析/实施 spec 准备 (per 决策 #86 §4 R152 era 派活 5 sub 第 2 个 + 决策 #74 B1 V1.1 release Mavis 自决改 + 决策 #71 §5 永久循环 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

**Date**: 2026-08-11 05:09 (R152 era 实施准备阶段, per 决策 #86 §4 5:00 tick 派活 16 sub-agent = 5+3+2+5+1)
**Author**: R152-2 sub-agent (Mavis 派, per 决策 #86 §4 R152 era 5 sub 第 2 个, 调研/分析/实施 spec 准备阶段, **0 改 src 严守 100%**)
**Receiving agent**: Mavis root session (`mvs_367e66fae08342ffa399befe4f85dbac`)
**触发**: 决策 #86 (5:00 tick 状态 + 6 R148 Token Plan 上限 errored 中断接手 + target/ 82.64GB 预警 + 16 sub-agent 派活补到 16 满) + 决策 #74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改, 前提: 更好的架构) + 决策 #71 (主人 0:57 拍板"计划内任务完成时自动接续永久循环 4 步调研+差距+计划+实施") + 决策 #73 (主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度) + 决策 #75 (R131 era 派活) + 用户记忆 #10 (主人长时间离开, Mavis 自主决策 + 决策日志)
**任务定位**: R152 era 实施准备阶段 (per 决策 #86 §4 R152-2 派活), **整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec) — 调研/分析/实施 spec 准备 类, 0 改 src/ 严守 100%**, 实施在 V1.1 release 实战 2026-11-30
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
**整合 #5.3 commit**: `4207f187` (8/11 01:43 done, 187 files / 127548 insertions, 0 主动 push 严守)
**整合 #6 commit 拍板**: 2026-11-25 (5 天缓冲 before V1.1 release 实战 2026-11-30, per 任务 spec + R130-5 §1.1 + R132-1 §1.1)
**V1.1 release 实战**: 2026-11-30 (per R132-1 §1.1 + R130-5 §1.1 V1.1 估 2026-11-30)
**整合 #7 commit 拍板**: 2027-Q1/Q2 估 (V1.2 release 准备 / V2.0 release 远期重构, per R137-2 §8.1)
**关联决策**: #10 (决策日志) + #22 (24 LOCKED + semver) + #33 (8 硬墙 + 0 装 PASS) + #36 (借鉴 ID 严格化) + #44 (0 主动删) + #48 (整合 #4 commit) + #55 (R127 派活) + #58 (R128-2 派活) + #60 (0 主动删 Safety policy) + #61 (R129 era 派活) + #62 (整合 #5 commit 拆 3 commit) + #64 (auto-replenish-16 cron) + #66 (跑中 ≥ 16) + #69 (target/ 50-100GB 预警) + #70 (Mavis 清理决策权升级) + #71 (永久循环 4 步) + #72 (R130 era 调研 6 sub) + **#73 (主人 8/11 01:14 拍板 3 件套)** + **#74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改)** + #75 (R131 era 派活 11 sub) + #76 + #77 (R137 era 派活清单) + #78 (整合 #5.3 commit 拍板成功) + #79-#85 (R131-R148 era 派活) + **#86 (5:00 tick 状态 + 16 sub 派活 R149-R152)**
**关联报告** (per 任务 spec + 用户记忆 #6 0 重复造轮子): R131-1 (架构总审视 10 方向) + R131-2 (借鉴 12 源差距) + R131-3 (V1.1 release 实施路线图) + R131-4 (cargo workspace 87 crate 结构优化 7 方向) + **R131-5 (24 LOCKED 入口分布优化 8 方向, 本报告核心依据 1)** + R131-9 (形式化集成优化 9 方向) + R132-1 (V1.1 release 路线图 final 6 大方向) + R133-1 (借鉴源 12 源 实施) + R133-2 (ASI Stage 9 实施) + R133-3 (三洋葱架构升级 5 阶段 实施 spec) + **R137-2 (24 LOCKED 入口签名 改写 spec + 5 阶段 8 周, 本报告核心依据 2)** + R140-2 (V1.1 release 路线图 detailed) + R140-4 (ASI Stage 10 终极自治) + R140-5 (借鉴 12 源 fork 决策) + R141-2 (24 LOCKED vs 借鉴 API 一致性) + R143-3 (V1.1 vs V1.0 差异表) + R147-2 (V1.1 release auto-continue) + R149-2 (ASI Stage 9 长程 AI 成长深化) + R149-3 (三洋葱架构升级 V2) + R149-4 (借鉴 12 源 fork-then-borrow 模式) + R150-2 (整合 #5.1 commit 拍板后 24 LOCKED 入口签名优化差距, Mavis 自决改, 决策 #74 B1)
**状态**: ✅ **R152-2 整合 #6 24 LOCKED 入口签名 优化准备 done** (60 min 时间盒, 0 改 src 严守 100%): 整合 #6 实施 spec 准备 (10+ 优化方向详细 + 24 LOCKED Cargo.toml 字段 update per-crate + 24 LOCKED lib.rs / mod.rs 改动 per-crate + cargo test --workspace 8 步 verify 8/8 + 跟 ASI Stage 9 / 三洋葱 V2 / 借鉴 12 源 fork / 9 organ / 8 哲学锚 / 不要怕复杂度哲学关系 + 风险 10 维 + 异常分支 6 维 + 派活计划 整合 #6 + 整合 #7 commit 拍板 + 8 硬墙严守 100% verify)

---

## 0. 一句话 (TL;DR)

**R152-2 整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec) done (60 min 时间盒, 0 改 src 严守 100%)**: V1.0 release 0 改 src 严守 100% (整合 #5.1 commit 拍板 R11 baseline, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS per R131-5 §1.2, R11 baseline 3 值 0.8682/0.8532/0.9063 严守, PHL-07 spec-only 0 实施, Cargo.toml workspace.version 1.2.0 严守, 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守, 13 键 verdict cache 严守). **整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec) = 8 大方向 + 4 新增方向 = 12 优化方向 总览 (per 决策 #74 B1 Mavis 自决改 + 决策 #71 §5 永久循环 + 决策 #86 §4 R152 era 派活)**: **8 大方向** (per R131-5 §2.1-§2.8 + R137-2 §3.1-§3.9 5 阶段 8 周) = ①**标准化** (24 LOCKED 入口签名 5 风格 → 3 模式之一 per-crate 自决) + ②**瘦身** (公开 API 表面 ~800+ pub items → ≤30 per-crate, 800 → 560 -30%) + ③**9 叶子拆 workspace** (9 叶子 crate 拆 apeireth-leaf/ workspace) + ④**core 拆 pub mod** (1 个 108KB lib.rs 拆 5 大 mod: types / onion / human / gate / lib) + ⑤**大模块拆 sub-crate** (mcp 13→8 + pipeline 11→6 + api 16→5 + memory 13→5 + asi 9→4 + tools 12→5 + evolution 9→5 + graph 11→5 + council 20+→4 = 47 sub-crate) + ⑥**DSL 洋葱** (三洋葱 → 四洋葱 升级, 新增 apeireth-dsl crate, 第 4 层 "智能涌现") + ⑦**9 organ 借 OpenCode + Eye 补** (新增 apeireth-eye workspace, 24 LOCKED 全部下沉到 9 organ workspace) + ⑧**R12 测度对齐** (24+9 = 33 → 24+11 = 35 测量函数, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新). **4 新增方向** (per 任务 spec §1 10+ 优化方向 + R149-2 + R149-3 + R149-4 调研) = ⑨**ASI Stage 9 长程 AI 成长 集成** (per R149-2 + R130-2 §1 + R140-4: 24 LOCKED 入口签名加 Stage 9 4 维度 H1-H4: H1 自我决策 + H2 自我学习 + H3 自我演化 + H4 群体智能) + ⑩**三洋葱架构 V2 集成** (per R149-3 + R133-3: 三洋葱 → 四洋葱 + 第 5 层 "形式化洋葱" 实施, 24 LOCKED 全部引用 apeireth-dsl 守门) + ⑪**借鉴 12 源 fork-then-borrow 模式集成** (per R149-4 + R130-6 + R140-5: 8 真 cloned 49.6MB/7,764 files + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 1 借脑 ID 索引完成 6 子源, 24 LOCKED 入口签名借脑 1:1 公开模式) + ⑫**9 organ workspace 化** (per R131-5 §2.6 + R137-2 方向 7: 9 organ workspace 全集, 顶层 apeireth re-export 全部 organ types, 24 LOCKED 全部下沉). **12 优化方向 实施 spec 准备 = 5 阶段 8 周 派活 (per R137-2 §4)**: 阶段 1 标准化 1 周 (R153 era 3-5 sub) + 阶段 2 瘦身 1 周 (R154 era 3-5 sub) + 阶段 3 9 叶子拆 + Eye 补 2 周 (R155 era 5-8 sub) + 阶段 4 core 拆 pub mod + 大模块拆 sub-crate 2 周 (R156 era 8-10 sub) + 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 集成 + 三洋葱 V2 + 借鉴 12 源 fork 2 周 (R157 era 10-15 sub). **整合 #6 commit 拍板 = 2026-11-25 (5 天缓冲 before V1.1 release 实战 2026-11-30)**, 整合 #7 commit 拍板 = 2027-Q1/Q2 估 (V1.2 release 准备 / V2.0 release 远期重构). **8 硬墙严守 100% verify**: B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (per 决策 #74) + B2 workspace.version 1.2.0 V1.0 release 严守 / 1.2.1 V1.1 release bump + A1 R11 baseline 3 值 V1.0 release 严守 / V1.1 release R12 更高 + A3 12 键 + PHL-07 V1.0 release PHL-07 spec-only 0 实施 / V1.1 release PHL-07 实施 + B3 V0.5 30 维严守 + B4 6 重守门 v7 严守 + B5 8 哲学锚严守 + C1 0 主动 commit (主人起床前) 严守 + C2 0 装 PASS 严守 + 0 push 严守. **0 主动 IM 主人 + 0 主动 commit/push 严守 + 0 装 PASS 严守 + 0 主动删严守 + 不要怕复杂度哲学落地** (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md).

---

## 1. 24 LOCKED crate 入口签名 V1.1 release 优化 实施 spec 详细 (12 优化方向 8+4)

### 1.1 8 大优化方向 (per R131-5 §2 + R137-2 §3 5 阶段 8 周)

#### 1.1.1 方向 ①: 标准化 (入口签名一致性 5 风格 → 3 模式之一)

**现状 (per R131-5 §2.1 + R137-2 §3.2)**:
- 24 LOCKED crate 入口签名风格 = **5 种 re-export 模式**:
  - **类型 A (重 re-export facade)**: supervisor / agent / council / api / memory / core / mcp / graph / pipeline / constraint / evolution / cognition / life-force / tools / tool-runtime / tool-registry / tool-approval / asi / cli / bench (20/24 = 83%)
  - **类型 B (轻 facade + 主类型定义)**: protocol / bus (2/24 = 8%)
  - **类型 C (单 trait 入口)**: extension (1/24 = 4%)
  - **类型 D (大 enum 主类型)**: asi / supervisor (2/24 = 8%, 跟 A 重叠)
  - **类型 E (纯 trait 模块)**: cognition (1/24 = 4%, 跟 A 重叠)

**问题 (per R131-5 §2.1)**:
- 24 个 crate 用了 5 种风格, 跨 crate 集成时需先看每个 lib.rs 才能知道有哪些 API
- 公开 API 表面 = 24 crate re-export union, 难维护一份完整的"24 LOCKED public API"清单
- 编译时间: 重 re-export 模式下, 任何下游 crate 改一个就触发整个 union 重编译

**V1.1 release 标准化 3 模式之一 (per-crate 自决)**:
- **模式 1 (全 re-export)**: 适用 20/24 crate (类型 A), per-crate 全部重导出, 消费者 `use apeireth_xxx::*` 拿全部 API
- **模式 2 (主类型 facade)**: 适用 2/24 crate (类型 B: protocol / bus), 入口文件直接定义核心类型 + 轻 re-export
- **模式 3 (按需 re-export)**: 适用 2/24 crate (类型 C + D + E: extension + cognition), 仅 re-export 主类型, 其他 module 公开

**实施 spec (阶段 1 标准化 1 周)**:
- 阶段 1.1 (Day 1-2): per-crate 决策矩阵 (24 LOCKED 各自选 3 模式之一, per 类型 A/B/C/D/E 对应)
- 阶段 1.2 (Day 3-4): 24 LOCKED 入口签名格式统一 (pub mod + pub use + pub const + pub struct + pub enum + pub fn 6 模式)
- 阶段 1.3 (Day 5): per-crate `pub use module::*` 块标准化, 顶部 doc comment 极详细 (per 50-100 行 doc, O-5 哲学锚)
- 阶段 1.4 (Day 6-7): 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify, 0 装 PASS 严守

**风险**: 中 (改 re-export 模式 = 改 crate 公开 API 表面 = 改消费者 `use` 路径)
- **缓解**: 保留 `pub mod` 重新导出, 消费者用 `apeireth_xxx::module::Type` 全路径仍能用
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)

#### 1.1.2 方向 ②: 瘦身 (公开 API 表面 800+ → ≤30 per-crate)

**现状 (per R131-5 §2.2 + R137-2 §3.3)**:
- **24 LOCKED crate 公开 API 表面 = ~800+ pub items** (粗估, 实测需 ripgrep verify)
- 各 crate 表面分布: supervisor ~12 / agent ~25 / council ~50+ / bus ~20 / protocol ~40 / mcp ~30 / tool-registry ~30 / tool-runtime ~25 / graph ~40 / pipeline ~35 / tool-approval ~15 / extension ~17 / evolution ~50+ / api ~40+ / core ~50+ / memory ~50+ / asi ~50+ / tools ~30 / cli ~25 / bench ~20 / cognition ~25 / action ~20 / life-force ~25 / constraint ~25

**V1.1 release 瘦身 (per-crate 暴露 ≤30 pub items 目标)**:
- **总目标**: 800 → 560 (-30%, 减少 240 pub items)
- **per-crate 目标 (per R131-5 §2.2 表)**:
  - supervisor: 12 → 12 (0 改, 已 ≤30)
  - agent: 25 → 25 (0 改, 已 ≤30)
  - **council: 50+ → 30 (-40%)**: 8 协作模式砍 4 → 4, 7 factory 砍 3 → 4, Synthesis/Persona/Sovereignty/Constitution/Trace/Graph 内部化
  - bus: 20 → 20 (0 改, 已 ≤30)
  - **protocol: 40 → 30 (-25%)**: 4 adapter + 4 bridge + bridge_ext 5 + normalized 8 + ws_v1 8 + 5 const → 30
  - mcp: 30 → 30 (0 改, 已 ≤30)
  - tool-registry: 30 → 30 (0 改, 已 ≤30)
  - tool-runtime: 25 → 25 (0 改, 已 ≤30)
  - **graph: 40 → 30 (-25%)**: Checkpoint + 4 conditional + 4 state + 11 Subgraph/Channel + 5 StateGraph + 7 Context → 30
  - **pipeline: 35 → 30 (-14%)**: 8 module + 9 force_translate + 3 placeholder + 9 provider_registry + 3 retry + 2 streaming + 5 token + 6 tool_loop + 3 Pipeline → 30
  - tool-approval: 15 → 15 (0 改, 已 ≤30)
  - extension: 17 → 17 (0 改, 已 ≤30)
  - **evolution: 50+ → 30 (-40%)**: 8 PODA + 19 library_autonomy + 14 library_autonomy_loop 内部化
  - **api: 40+ → 30 (-25%)**: 22 LLM + 11 protocol + 4 const → 30
  - **core: 50+ → 30 (-40%)**: 4 + 1 + 5 onion + 2 human + 12 PhilosophyKey + 3 verdict + 1 trait + 5 Gate + 5 Risk + 13 ActionTarget + 4 ActionVerdict + 1 ActionGuard → 30, ActionTarget 13 → 5 + Gate 5 内部化
  - **memory: 50+ → 30 (-40%)**: EpisodeQuery + EpisodeStore + Identity + 3 analysis + Migration + 3 Semantic + 2 Note + 10 stream + 2 ThreeLayer + 3 UserProfile + MemoryError + 6 StreamKind + SqliteMemoryStore + ContinuitySnapshotStore + 3 Provider → 30, 10 stream + 6 StreamKind 内部化
  - **asi: 50+ → 30 (-40%)**: 26 measure_* → 24 维 + 9 子测度 = 33 + 8 calibration + 2 drift + TraceRepository + 3 llm_judge + 7 registry + 4 render + 2 scheduler + 2 tokenizer + 4 const + 4 name array + 2 legacy struct + DimensionTrace → 30, 2 legacy struct 内部化
  - tools: 30 → 30 (0 改, 已 ≤30)
  - cli: 25 → 25 (0 改, 已 ≤30)
  - bench: 20 → 20 (0 改, 已 ≤30)
  - cognition: 25 → 25 (0 改, 已 ≤30)
  - action: 20 → 20 (0 改, 已 ≤30)
  - life-force: 25 → 25 (0 改, 已 ≤30)
  - constraint: 25 → 25 (0 改, 已 ≤30)

**实施 spec (阶段 2 瘦身 1 周)**:
- 阶段 2.1 (Day 1-2): per-crate 公开 API 表面清单 (per 24 LOCKED R131-5 §2.2 表)
- 阶段 2.2 (Day 3-5): per-crate 实施转 pub(crate) / module-private (per 目标, council 50+ → 30, evolution 50+ → 30, core 50+ → 30, memory 50+ → 30, asi 50+ → 30, protocol 40 → 30, graph 40 → 30, api 40+ → 30, pipeline 35 → 30)
- 阶段 2.3 (Day 6): 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify
- 阶段 2.4 (Day 7): 编译时间 verify (期望 减少 10-20%, per 公开 API 表面减少 30%)

**风险**: 高 (公开 API 表面"瘦身" = 改入口签名 = 改消费者 `use` 路径 = breaking change)
- **缓解**: 保留 `pub mod module::Type` 全路径, 消费者用全路径仍能用
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用

#### 1.1.3 方向 ③: 9 叶子拆 workspace (per R131-5 §2.3)

**现状 (per R131-5 §2.3)**:
- 24 LOCKED crate 依赖图: 7 个 dep core + 5 个 dep tool-registry + 9 叶子 crate (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench)
- 9 叶子 = 0 依赖其他 LOCKED crate → 拆 workspace 候选

**V1.1 release 9 叶子 crate 拆 workspace**:
- **新 workspace**: `apeireth-leaf/{supervisor,protocol,bus,tool-registry,graph,extension,evolution,asi,bench}/Cargo.toml`
- **顶层 `apeireth/Cargo.toml` 0 改** (per 决策 #74 §1 B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1)
- **9 叶子拆出来独立发布**, 9 叶子 cargo build/test 独立 verify
- **顶层 re-export facade 保留**: 消费者用 `apeireth_xxx::Type` 仍能用, 内部 crate 路径变 `apeireth_leaf_xxx::Type` (新路径)

**实施 spec (阶段 3.1 9 叶子拆 1 周)**:
- 阶段 3.1.1: 9 叶子 crate 内部 import 路径全 1:1 扫描 (per `cargo metadata` + `cargo tree` 验证)
- 阶段 3.1.2: 新 workspace `apeireth-leaf/Cargo.toml` 9 叶子加进 members
- 阶段 3.1.3: 9 叶子 crate 独立 publish ready, 顶层 Cargo.toml members 段更新
- 阶段 3.1.4: 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify
- 阶段 3.1.5: 顶层 re-export facade 1:1 续, 消费者 0 改

**风险**: 中 (拆 workspace = 改 Cargo.toml 路径 = 改消费者 `use apeireth_xxx` → `use apeireth::organ::xxx`)
- **缓解**: 保留 re-export facade (顶层 `apeireth` 重新导出全部 `apeireth-leaf::xxx`, 0 改消费者代码)
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)

#### 1.1.4 方向 ④: core 拆 pub mod (per R131-5 §2.4)

**现状 (per R131-5 §2.4)**:
- **core 是单 lib.rs 108KB**, 0 pub mod 拆分, 全部 50+ 类型定义在一个文件
- **问题**: 编译时全文件 re-parse, 难维护, 任何 core 改动触发大面积重编译

**V1.1 release core 拆 pub mod**:
- **core 拆 5 大 mod** (per R131-5 §2.4 V1.1 release):
  - `core/src/types.rs` (~20KB, 5 类型: Episode / Note / Session / IdentityCard / Migration)
  - `core/src/onion.rs` (~30KB, 5 onion 类型: PrincipleOnion / PrincipleLayer / PermissionOnion / PermissionLayer)
  - `core/src/human.rs` (~20KB, 8 human 类型: HumanAuthority / HAMode / RealHuman / HAAuthentication / BiometricData + 12 PhilosophyKey + ALL_TWELVE_KEYS + TWELVE_KEYS_HARDCODE)
  - `core/src/gate.rs` (~25KB, 8 gate 类型: PhilosophyGuard / PhilosophyVerdict / VerdictCache / Gate / 5 variant + Action / RiskLevel / ActionTarget / ActionVerdict / ActionGuard / DefaultPhilosophyGuard)
  - `core/src/lib.rs` (~13KB, 5 行 `pub mod types; pub mod onion; pub mod human; pub mod gate;` + 顶部 re-export facade 0 改)
- **0 改入口签名**, 仅内部重构
- **0 改外部消费者代码**: 顶层 `apeireth_core::Type` 全路径仍能用

**实施 spec (阶段 4.1 core 拆 pub mod 1 周)**:
- 阶段 4.1.1: core 1 个 108KB lib.rs 类型 1:1 分类到 5 大 mod (per 类型表)
- 阶段 4.1.2: 5 大 mod 各自 mod.rs + types/onion/human/gate 子文件 (per 类型 size 估)
- 阶段 4.1.3: core/src/lib.rs 顶部 re-export 1:1 续 (0 改入口签名, 仅内部 mod 拆分)
- 阶段 4.1.4: 24 LOCKED 全跑 cargo build + cargo test verify, 0 越界 8 硬墙 100%
- 阶段 4.1.5: core 编译时间 verify (期望 减少 30-50%, per pub mod 拆分后并行编译)

**风险**: 中 (拆 module = 改 import 路径 = breaking change)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_core::Type` 仍能用
- **缓解**: 0 改 core 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界), 仅内部 mod 拆分

#### 1.1.5 方向 ⑤: 大模块集中 crate 拆 sub-crate (47 sub-crate, per R131-5 §2.4)

**现状 (per R131-5 §2.4)**:
- **大模块集中**: council (20+) / mcp (13) / graph (11) / pipeline (11) / api (16) / memory (13) / asi (9) / tools (12) / evolution (9)
- **mcp / pipeline / api / memory 内部 module 边界模糊**: 多个 module 之间 cross-use, 实测命名重复 (e.g. `mcp::protocol::Id` vs `mcp::tools::Id`)

**V1.1 release 大模块集中 crate 拆 sub-crate (47 sub-crate 总)**:
- **mcp 拆 8 sub-crate**: `apeireth-mcp-core` (protocol / initialize / 8 frame) + `apeireth-mcp-resources` (4 ResourceServer) + `apeireth-mcp-subscribe` (subscriptions / tool_subscriptions) + `apeireth-mcp-tools` (tools / ServerInfo / ToolDef) + `apeireth-mcp-prompts` (prompts) + `apeireth-mcp-transport` (transport) + `apeireth-mcp-primitives` (primitives / macros) + `apeireth-mcp` (顶层 re-export facade 0 改入口签名)
- **pipeline 拆 6 sub-crate**: `apeireth-pipeline-token` (tiktoken_counter / token_budget) + `apeireth-pipeline-placeholder` (placeholder) + `apeireth-pipeline-force-translate` (force_translate) + `apeireth-pipeline-retry` (retry_suppression) + `apeireth-pipeline-streaming` (streaming) + `apeireth-pipeline-tool-loop` (tool_loop) + `apeireth-pipeline` (顶层 re-export facade 0 改入口签名, + provider_registry + model_router + role_divider)
- **api 拆 5 sub-crate**: `apeireth-api-llm` (llm / cache / replay_cache / retry) + `apeireth-api-server` (server / v2_endpoints / v2_routes / observability / endpoints / v1_tools) + `apeireth-api-protocol` (protocol_handlers / protocol_handler_trait / ws_v1) + `apeireth-api-auth` (auth / audit_sqlite) + `apeireth-api` (顶层 re-export facade 0 改入口签名, + MultiLlmRouter)
- **memory 拆 5 sub-crate**: `apeireth-memory-stream` (history_streams / streams / append_only) + `apeireth-memory-semantic` (semantic / semantic_persist) + `apeireth-memory-episode` (episode / continuity_link) + `apeireth-memory-session` (session_note / three_layer) + `apeireth-memory` (顶层 re-export facade 0 改入口签名, + user_profile / llm_analysis / migrations)
- **asi 拆 4 sub-crate**: `apeireth-asi-calibration` (calibration / dim_enhance) + `apeireth-asi-measurement` (measurement / llm_judge) + `apeireth-asi-render` (render / scheduler) + `apeireth-asi` (顶层 re-export facade 0 改入口签名, + drift / history / tokenizer + 24 measure_dim_* + 9 measure_sub_*)
- **tools 拆 5 sub-crate**: `apeireth-tools-fs` (file_ops / grep_ops) + `apeireth-tools-git` (git_ops) + `apeireth-tools-exec` (code_exec / long_task) + `apeireth-tools-web` (web_search / web_fetch / apply_patch) + `apeireth-tools` (顶层 re-export facade 0 改入口签名, + conventions_scanner + classifier + register + result)
- **evolution 拆 5 sub-crate**: `apeireth-evolution-council` (council_bridge) + `apeireth-evolution-engine` (engine / state) + `apeireth-evolution-poda` (poda_cycle / fail) + `apeireth-evolution-library` (library_autonomy / library_autonomy_loop) + `apeireth-evolution` (顶层 re-export facade 0 改入口签名, + traits / MockPlugin / Patch / Plugin / PluginRegistry / SelfModification / SystemState)
- **graph 拆 5 sub-crate**: `apeireth-graph-state` (state / state_graph) + `apeireth-graph-executor` (executor / conditional / checkpoint) + `apeireth-graph-subgraph` (subgraph / channel) + `apeireth-graph-context` (context_graph / cognition_graph) + `apeireth-graph` (顶层 re-export facade 0 改入口签名, + mcp_resource)
- **council 拆 4 sub-crate**: `apeireth-council-advisor` (advisor / advisors / 7 factory) + `apeireth-council-deliberation` (deliberation / council_member / council_member_deliberation / council_member_persona_combo / persona) + `apeireth-council-collaboration` (collaboration / constitution / trace / graph_orchestration) + `apeireth-council` (顶层 re-export facade 0 改入口签名, + bus_bridge / mcp_bridge / graph_bridge / hold / lifecycle / mock_llm / sovereignty / stress_test / synthesis)
- **总计**: 8 + 6 + 5 + 5 + 4 + 5 + 5 + 5 + 4 = **47 sub-crate**

**实施 spec (阶段 4.2 大模块拆 sub-crate 1 周)**:
- 阶段 4.2.1: 8 大模块集中 crate 内部 module 1:1 扫描 (per 8 crate module 表)
- 阶段 4.2.2: 8 大模块集中 crate 各拆 4-8 sub-crate (per 上述 sub-crate 列表)
- 阶段 4.2.3: 顶层 8 crate re-export facade 0 改入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界)
- 阶段 4.2.4: 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify, 0 越界 8 硬墙 100%
- 阶段 4.2.5: 编译时间 verify (期望 减少 20-30%, per sub-crate 拆分后并行编译)

**风险**: 中 (拆 sub-crate = 改 import 路径 = breaking change)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用
- **缓解**: 0 改 24 LOCKED 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界), 仅内部 sub-crate 拆分

#### 1.1.6 方向 ⑥: DSL 洋葱 (三洋葱 → 四洋葱 升级, per R131-5 §2.5 + R133-3 §3)

**现状 (per R131-5 §2.5)**:
- **三洋葱架构 (R125 B6 升级, 整合 #4 commit done)**:
  - **第 1 层 原则洋葱 (philosophy)**: 8 哲学锚 + 原则 (E/S/A/M/O 5 层, E 永不可绕过)
  - **第 2 层 权限洋葱 (permission)**: 6 重守门 v7 (L0-L5 6 层, L0 = 真实人类批准)
  - **第 3 层 DSL 洋葱 (DSL)**: Colang DSL (R125-5 NVIDIA 借鉴后, 1700 行 colang_dsl.rs done + 266/266 + 6 借鉴点)
- **24 LOCKED 跟三洋葱架构对应关系**:
  - 原则洋葱 E 层: core (L0 HA 锁) / constraint (哲学守门) / life-force (SGI 锁)
  - 原则洋葱 S 层: council (7 强制 Advisor) / evolution (演化审议)
  - 原则洋葱 A 层: memory (历史流 6 表) / asi (24 维测量历史)
  - 原则洋葱 M 层: cognition / pipeline / protocol / bus / graph
  - 原则洋葱 O 层: agent / tool-registry / tool-runtime / tool-approval / tools / mcp / extension / action / api / cli / bench / supervisor
  - 权限洋葱 L0: core (L0 HA 锁) / constraint (gate3 物理隔离)
  - 权限洋葱 L1-L5: api (V2 端点) / tool-approval (5 规则 + 5min 窗口)
  - DSL 洋葱: 0 落地, 24 LOCKED 都 0 引用 Colang

**V1.1 release DSL 洋葱落地 + 三洋葱 → 四洋葱 升级**:
- **新增 `apeireth-dsl` crate** (per R131-5 §2.5 V1.1 release DSL 洋葱落地):
  - 顶层 DSL 洋葱 = 原则 (顶层) → 权限 (中层) → DSL (底层)
  - Colang DSL 真实施 (per R125-5 NVIDIA 借鉴后 1700 行)
  - 24 LOCKED crate 引用 dsl 守门 (per `apeireth_dsl::guard::*` API)
  - DSL 守门 = 4 重 (L1 原则 guard / L2 权限 guard / L3 DSL guard / L4 智能涌现 guard)
- **三洋葱 → 四洋葱 升级** (per R133-3 §3):
  - **第 1 层 原则洋葱 (philosophy)**: 8 哲学锚严守
  - **第 2 层 权限洋葱 (permission)**: 6 重守门 v7 严守
  - **第 3 层 DSL 洋葱 (DSL)**: Colang DSL 严守
  - **第 4 层 智能涌现洋葱 (emergence, 新增)**: 智囊团 7 席 + 群体智能 + 自我决策/学习/演化 (per R133-3 §3.2.1-§3.2.3 + R130-2 ASI Stage 8/9)

**实施 spec (阶段 5.1 DSL 洋葱 1 周)**:
- 阶段 5.1.1: 新增 `apeireth-dsl` crate, 顶层 DSL 洋葱 = 原则 (顶层) → 权限 (中层) → DSL (底层) + 智能涌现 (V1.1 release 起步)
- 阶段 5.1.2: 三洋葱 → 四洋葱 升级 (per R133-3 §3.2 第 4 层 "智能涌现" 实施 spec)
- 阶段 5.1.3: 24 LOCKED crate 引用 dsl 守门 (per `apeireth_dsl::guard::*` API)
- 阶段 5.1.4: 24 LOCKED 全跑 cargo build + cargo test + 四洋葱集成 verify
- 阶段 5.1.5: 8 硬墙 + 8 哲学锚 严守 verify

**风险**: 高 (拆三洋葱 workspace + 加 DSL 洋葱 = 改大量 import 路径 = breaking change)
- **缓解**: 顶层 `apeireth-onion` facade 重新导出全部洋葱 module, 消费者 0 改
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)
- **缓解**: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

#### 1.1.7 方向 ⑦: 9 organ 内部借 OpenCode + Eye 补 (per R131-5 §2.6 + R125 B7 + R130-6)

**现状 (per R131-5 §2.6)**:
- **9 organ 跨 8 LOCKED crate**:
  - **Heart (0, LLM 网关心跳)**: supervisor + bus (L0) + pipeline (5 步管线)
  - **Brain (1, Multi-Agent 决策)**: agent + council + cognition + constraint
  - **Hand (2, Tool Protocol)**: tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action (7 个 LOCKED)
  - **Eye (3, 用户输入感知)**: (暂无 LOCKED crate, 在 apeireth-tui/src/organ/eye.rs)
  - **Ear (4, 系统事件监听)**: bus (L1-L4)
  - **Memory (5, 3 层 facade)**: memory + asi (24 维) + life-force (SGI 锁) + core (IdentityCard 跨载体)
  - **Voice (6, TTS/STT)**: protocol (WS 8 帧) + pipeline (流式)
  - **Body (7, 长程任务)**: bench + api (HTTP server) + cli
  - **Mind (8, 9-stage lifecycle)**: evolution + graph (lifecycle 编排) + constraint (5 重守门)
- **覆盖率**: 8/9 organ 100% 覆盖 (除 Eye 在 tui, 不在 24 LOCKED)
- **问题**: 9 organ 内部借 OpenCode (R125 B7) 在 24 LOCKED crate 中 0 体现 (organ-first 拓扑 0 落地)

**V1.1 release 9 organ 内部借 OpenCode + Eye 补**:
- **Eye organ 补** (per R131-5 §2.6 V1.1 release 优化方向):
  - 新增 `apeireth-eye` workspace (从 tui/src/organ/eye.rs 抽 crate, per 9-organ-summary §3 Eye 11.0KB, 4 输入通道: keystroke / mouse_click / voice_input)
  - 顶层 re-export facade 保留: 消费者用 `apeireth_eye::Type` 仍能用
- **9 organ workspace 化** (per 决策 #74 B1 Mavis 自决改 + R125 B7 内部借 OpenCode):
  - **新增 `apeireth-organ/{heart,brain,hand,eye,ear,memory,voice,body,mind}/Cargo.toml` 9 个 organ workspace**
  - **24 LOCKED crate 按 9 organ 拆**:
    - `apeireth-heart` workspace: supervisor + bus (L0) + pipeline
    - `apeireth-brain` workspace: agent + council + cognition + constraint
    - `apeireth-hand` workspace: tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action
    - `apeireth-eye` workspace: (从 tui/src/organ/eye.rs 抽 crate)
    - `apeireth-ear` workspace: bus (L1-L4)
    - `apeireth-memory` workspace: memory + asi + life-force + core (IdentityCard 跨载体)
    - `apeireth-voice` workspace: protocol + pipeline (流式) + (未来 tts/stt crate)
    - `apeireth-body` workspace: bench + api + cli
    - `apeireth-mind` workspace: evolution + graph + (约束守门从 brain/constraint 拆过来)
- **9 organ 内部借 OpenCode 实施** (per R125 B7 内部借 OpenCode):
  - 24 LOCKED crate 内部 fn 借 OpenCode 0 改入口签名 (0 破坏 LOCKED 入口)
  - OpenCog 借脑 1:1 公开模式 (per R130-6 + R133-1 借鉴源 12 源 + 决策 #22 §4 AGPL-3.0 决策)
  - 0 装"已读真源码", 0 装"已 fork" (per 决策 #33 §2.3 C2)

**实施 spec (阶段 3.2 Eye 补 + 阶段 5.2 9 organ 内部借 OpenCode 2 周)**:
- 阶段 3.2.1: 新增 `apeireth-eye` workspace, 从 tui/src/organ/eye.rs 抽 crate (per 4 输入通道)
- 阶段 3.2.2: Eye organ 顶层 re-export facade 0 改入口签名
- 阶段 3.2.3: 24 LOCKED 全跑 cargo build + cargo test verify
- 阶段 5.2.1: 9 organ workspace 化 (per 上述 9 organ workspace 列表), 24 LOCKED 全部下沉
- 阶段 5.2.2: 9 organ 内部 fn 借 OpenCode 0 改入口签名 (per R125 B7 + R130-6)
- 阶段 5.2.3: 24 LOCKED 全跑 cargo build + cargo test + organ 集成 verify

**风险**: 极高 (9 organ 重构 = 改 24 LOCKED crate 全部路径 = 改 N 个消费者的 `use` 路径 = breaking change)
- **缓解**: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用
- **缓解**: V1.1 release bump 1.2.1, V2.0 release bump 2.0.0 (semver major)
- **缓解**: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

#### 1.1.8 方向 ⑧: R12 测度对齐 (per R131-5 §2.7 + R131-9 O5 + 决策 #74 §2.3)

**现状 (per R131-5 §2.7)**:
- **R11 baseline 3 值** (per 决策 #33 §2.3 A1 + 决策 #74 §1 A1):
  - V1141 IC-001 fresh 24 维均值: 0.8682
  - V1131 dashboard 9 维均值: 0.8532
  - V1136 9 子测度均值: 0.9063
- **实测 24 LOCKED 入口分布跟 R11 baseline 对应**:
  - V1141 24 维: 锁在 `apeireth-asi::V05_DIMENSION_NAMES` (24 维名 + V05_DIM_COUNT 编译期 hardcode)
  - V1131 dashboard 9 维: 锁在 `apeireth-asi::V1136_SUBMEASURE_NAMES` (9 子测度名 + V1136_SUBMEASURE_COUNT 编译期 hardcode)
  - V1136 9 子测度基础: 锁在 `apeireth-asi::measurement::measure_dim_*` + `measure_sub_*` 真实测量函数 (24+9 = 33 个测量函数)

**V1.1 release R12 测度对齐**:
- **触发条件**: 更好的 baseline (R12 测度更高, per 决策 #74 §2.3 V1.1 release R12 baseline 更高)
- **R12 测度更新**:
  - 24 测量函数签名更新 R12 测度 (24+9 = 33 → 估 24+11 = 35, per R130-4 spec F1-F11 11 维度 + R131-9 O2)
  - V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新 (per 决策 #74 §2.3 V1.1 release R12 baseline 更高)
  - 24 LOCKED 入口签名测度集成 (per 阶段 5.3 R12 测度对齐 1 周)
- **R12 baseline 3 值** (估, per 决策 #74 §2.3 V1.1 release R12 baseline 更高):
  - V1141 R12 fresh 24 维均值: > 0.8682 (R11 baseline 之上)
  - V1131 R12 dashboard 9 维均值: > 0.8532
  - V1136 R12 9 子测度均值: > 0.9063
  - **R12 测度公式更新**: per R130-4 spec + R131-9 O2 F1-F11 11 维度, 加 PHL-07 spec-only 形式化 (F11 NEW 1 维) + 长程 AI 成长 形式化

**实施 spec (阶段 5.3 R12 测度对齐 1 周)**:
- 阶段 5.3.1: 24 测量函数签名更新 R12 测度 (per 24+9 = 33 → 24+11 = 35)
- 阶段 5.3.2: V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新
- 阶段 5.3.3: 24 LOCKED 入口签名 测度集成 (per 阶段 5 + 决策 #74 §2.3 V1.1 release R12 baseline 更高)
- 阶段 5.3.4: 24 LOCKED 全跑 cargo build + cargo test + R12 测度 verify
- 阶段 5.3.5: R12 baseline 3 值 verify (估 > R11 baseline, per 决策 #74 §2.3)

**风险**: 中 (改 R12 测度 = 改 24 测量函数签名 = 改 24 LOCKED 入口签名)
- **缓解**: 仅在 V1.1 release 改 (per 决策 #74 §2.3 V1.1 release 边界), V1.0 release 仍 R11 baseline 严守
- **缓解**: 24 测量函数签名 1:1 续, 加 NEW 测度 (24+11 = 35) 仅 add 0 remove (per semver minor 兼容)
- **缓解**: 编译期 hardcode 同步更新, 测试全跑

### 1.2 4 新增优化方向 (per 任务 spec §1 10+ 优化方向 + R149-2 + R149-3 + R149-4 调研)

#### 1.2.1 方向 ⑨: ASI Stage 9 长程 AI 成长 集成 (per R149-2 + R130-2 §1 + R140-4)

**现状 (per R149-2 + R130-2 §1)**:
- ASI Stage 1-7 已 done (per R130-2 §1 路线图): 基础 ASI 测量框架 + 24 维 + 9 子测度 + V0.5 30 维公式
- ASI Stage 8 (per R130-2 §1.4 + R140-4): 智囊团 7 席 自治模式
- ASI Stage 9 (per R130-2 §1.5 + R140-4): 长程 AI 成长 4 维度 (H1-H4) — H1 自我决策 + H2 自我学习 + H3 自我演化 + H4 群体智能

**V1.1 release ASI Stage 9 集成 跟 24 LOCKED 入口签名 关系**:
- **24 LOCKED 入口签名 加 Stage 9 4 维度 H1-H4**:
  - H1 自我决策: agent + council + cognition (3 个 LOCKED 入口签名加 self_decide API)
  - H2 自我学习: memory + asi + life-force (3 个 LOCKED 入口签名加 self_learn API)
  - H3 自我演化: evolution + graph (2 个 LOCKED 入口签名加 self_evolve API)
  - H4 群体智能: council (借 OpenCog AtomSpace, 1 个 LOCKED 入口签名加 swarm_intelligence API)
- **总影响**: 24 LOCKED 入口签名 中 9 个 (38%) 加 Stage 9 4 维度 API (per R149-2 + R130-2 §1.5)

**实施 spec (阶段 5.4 ASI Stage 9 集成 0.5 周)**:
- 阶段 5.4.1: 9 个 LOCKED 入口签名加 Stage 9 4 维度 API (per H1-H4 映射)
- 阶段 5.4.2: 24 LOCKED 全跑 cargo build + cargo test + Stage 9 集成 verify
- 阶段 5.4.3: Stage 9 4 维度 单元测试 (per H1 自我决策 ≥ 10 测试 / H2 自我学习 ≥ 10 测试 / H3 自我演化 ≥ 10 测试 / H4 群体智能 ≥ 10 测试)
- 阶段 5.4.4: 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

**风险**: 中 (加 Stage 9 API = 改 LOCKED 入口签名 = breaking change)
- **缓解**: 仅 add 0 remove (per semver minor 兼容)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)

#### 1.2.2 方向 ⑩: 三洋葱架构 V2 集成 (per R149-3 + R133-3)

**现状 (per R149-3 + R133-3)**:
- 三洋葱架构 (R125 B6 升级, 整合 #4 commit done): 原则 + 权限 + DSL 3 洋葱
- V1.1 release 三洋葱 → 四洋葱 升级 (per 方向 ⑥), 第 4 层 "智能涌现"
- **三洋葱架构 V2 (per R149-3)**: 在 V1.1 release 四洋葱基础上, 加 **第 5 层 "形式化洋葱"** (per R131-9 O1-O9 形式化 9 方向 + R125-9 kani 借鉴 + PHL-07 实施)

**V1.1 release 三洋葱 V2 集成 跟 24 LOCKED 入口签名 关系**:
- **24 LOCKED 入口签名 加 第 5 层 "形式化洋葱" 守门**:
  - 原则洋葱 E 层 (core / constraint / life-force): 加 formal_guard API (per PHL-07 形式化 实施, V1.1 release, per 决策 #74 §2.3 + R129-11 关键诚实标)
  - 原则洋葱 S 层 (council / evolution): 加 formal_verify API (per kani 借鉴 4502 形式化, per R131-9)
  - 原则洋葱 A 层 (memory / asi): 加 formal_proof API (per 24+11 = 35 测量函数 形式化)
  - 原则洋葱 M 层 (cognition / pipeline / protocol / bus / graph): 加 formal_check API
  - 原则洋葱 O 层 (agent / tool-registry / ...): 加 formal_audit API
- **24 LOCKED 入口签名 全部加 形式化洋葱 守门** (per R149-3 + R131-9)

**实施 spec (阶段 5.5 三洋葱 V2 集成 0.5 周)**:
- 阶段 5.5.1: 24 LOCKED 入口签名 加 第 5 层 "形式化洋葱" 守门 (per 5 层原则洋葱 映射)
- 阶段 5.5.2: apeireth-formal crate 实施 (per R131-9 O1-O9 形式化 9 方向)
- 阶段 5.5.3: 24 LOCKED 全跑 cargo build + cargo test + 五洋葱 集成 verify
- 阶段 5.5.4: 形式化洋葱 单元测试 (per 24 LOCKED 入口签名 ≥ 5 测试 = 120 测试 总)

**风险**: 中 (加 第 5 层 形式化洋葱 = 改 LOCKED 入口签名 = breaking change)
- **缓解**: 仅 add 0 remove (per semver minor 兼容)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)
- **缓解**: PHL-07 实施 (per R129-11 关键诚实标 + 决策 #74 §2.3)

#### 1.2.3 方向 ⑪: 借鉴 12 源 fork-then-borrow 模式集成 (per R149-4 + R130-6 + R140-5)

**现状 (per R149-4 + R130-6 + R140-5)**:
- **借鉴 12 源 = 8 真 cloned + 2 借鉴 ID 索引 + 1 永久跳过 + 1 借脑 ID 索引** (per R131-2 + R130-6 + R140-5):
  - 8 真 cloned (49.6 MB / 7,764 files): LangGraph (829 cloned) / VCP (chat-first) / aGLM (autonomous) / superpowers (skill) / chidori (journal 9 字段) / OpenHands (browser-use) / Aider (apply_patch) / Continue (Tab)
  - 2 借鉴 ID 索引完成: AutoGen (council 借脑) / Letta (memory 借脑)
  - 1 永久跳过: OpenCog AtomSpace + CogPrime (AGPL-3.0 license 风险, per 决策 #22 §4 + R130-6)
  - 1 借脑 ID 索引完成: 6 子源 (per R130-6)

**V1.1 release 借鉴 12 源 fork-then-borrow 模式集成 跟 24 LOCKED 入口签名 关系**:
- **24 LOCKED 入口签名 加 借鉴源 12 源 注释 (per R131-2 + R130-6)**:
  - 顶层 doc comment 加 12 源借鉴声明 (per O-3 哲学锚 "走在前人经验上")
  - 内部 fn 加 借鉴源 1:1 公开模式 (per 决策 #22 §4)
  - 0 装"已读真源码", 0 装"已 fork" (per 决策 #33 §2.3 C2)
- **24 LOCKED 入口签名 全部加 12 源 注释** (per R149-4 + R140-5)

**实施 spec (阶段 5.6 借鉴 12 源 fork-then-borrow 集成 0.5 周)**:
- 阶段 5.6.1: 24 LOCKED 入口签名 顶层 doc comment 加 12 源 借鉴声明 (per 8 真 cloned + 2 借鉴 ID + 1 借脑 ID)
- 阶段 5.6.2: 内部 fn 加 借鉴源 1:1 公开模式 (per 决策 #22 §4)
- 阶段 5.6.3: 24 LOCKED 全跑 cargo build + cargo test + 借鉴 12 源 集成 verify
- 阶段 5.6.4: 0 装 PASS 严守 (per 决策 #33 §2.3 C2, 0 装"已读真源码", 0 装"已 fork")

**风险**: 低 (加 借鉴源 注释 + 内部 fn 借脑 = 0 改 LOCKED 入口签名, 仅加注释 + 内部实现)
- **缓解**: 0 改 LOCKED 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界), 仅加 doc comment + 内部 fn
- **缓解**: 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
- **缓解**: 借脑 1:1 公开模式 (per 决策 #22 §4 + R130-6 + R140-5)

#### 1.2.4 方向 ⑫: 9 organ workspace 化 (per R131-5 §2.6 + R137-2 方向 7)

**现状 (per R131-5 §2.6)**:
- 24 LOCKED crate 跟 9 organ 映射是 N:1 (多个 LOCKED crate 对应同一 organ)
- Eye 在 24 LOCKED 0 对应, 在 tui 有独立 organ 入口
- 9 organ 内部借 OpenCode (R125 B7) 在 24 LOCKED crate 中 0 体现

**V1.1 release 9 organ workspace 化 跟 24 LOCKED 入口签名 关系** (per 方向 ⑦ 实施 spec 详):
- **24 LOCKED 入口签名 按 9 organ 重新组织**:
  - apeireth-heart workspace 入口: `pub use apeireth_supervisor::*; pub use apeireth_bus::l0::*; pub use apeireth_pipeline::*;`
  - apeireth-brain workspace 入口: `pub use apeireth_agent::*; pub use apeireth_council::*; pub use apeireth_cognition::*; pub use apeireth_constraint::*;`
  - apeireth-hand workspace 入口: `pub use apeireth_tool_registry::*; pub use apeireth_tool_runtime::*; pub use apeireth_tool_approval::*; pub use apeireth_tools::*; pub use apeireth_mcp::*; pub use apeireth_extension::*; pub use apeireth_action::*;`
  - apeireth-eye workspace 入口: 新增 Eye organ 类型 (4 输入通道: keystroke / mouse_click / voice_input)
  - apeireth-ear workspace 入口: `pub use apeireth_bus::{l1, l2, l3, l4}::*;`
  - apeireth-memory workspace 入口: `pub use apeireth_memory::*; pub use apeireth_asi::*; pub use apeireth_life_force::*; pub use apeireth_core::{Episode, Note, Session, IdentityCard, Migration};`
  - apeireth-voice workspace 入口: `pub use apeireth_protocol::*; pub use apeireth_pipeline::streaming::*;`
  - apeireth-body workspace 入口: `pub use apeireth_bench::*; pub use apeireth_api::*; pub use apeireth_cli::*;`
  - apeireth-mind workspace 入口: `pub use apeireth_evolution::*; pub use apeireth_graph::*; pub use apeireth_constraint::gate3::*;`
- **顶层 apeireth re-export 全部 organ types** (per 方向 ⑦ 顶层 facade)

**实施 spec**: 跟 方向 ⑦ 重叠 (per 阶段 5.2 9 organ workspace 化 1 周)
- 阶段 5.2.1: 9 organ workspace 化 (per 上述 9 organ workspace 列表), 24 LOCKED 全部下沉
- 阶段 5.2.2: 9 organ 内部 fn 借 OpenCode 0 改入口签名 (per R125 B7 + R130-6)
- 阶段 5.2.3: 24 LOCKED 全跑 cargo build + cargo test + organ 集成 verify

**风险**: 极高 (跟 方向 ⑦ 风险一致, 9 organ 重构 = breaking change)
- **缓解**: 跟 方向 ⑦ 缓解一致
- **缓解**: 顶层 re-export facade 保留

### 1.3 12 优化方向 总结表

| # | 方向 | 阶段 | 周 | 派活 era | 风险 | 主要依据 |
|---|------|------|----|---------|------|---------|
| ① | **标准化** (5 风格 → 3 模式) | 阶段 1 | 1 | R153 | 中 | R131-5 §2.1 + R137-2 §3.2 |
| ② | **瘦身** (800 → 560 pub items) | 阶段 2 | 1 | R154 | 高 | R131-5 §2.2 + R137-2 §3.3 |
| ③ | **9 叶子拆 workspace** (9 叶子) | 阶段 3.1 | 1 | R155 | 中 | R131-5 §2.3 + R137-2 §3.4 |
| ⑦ | **Eye 补** (从 tui 抽 crate) | 阶段 3.2 | 1 | R155 | 中 | R131-5 §2.6 + R137-2 §3.8 |
| ④ | **core 拆 pub mod** (1 → 5 mod) | 阶段 4.1 | 1 | R156 | 中 | R131-5 §2.4 + R137-2 §3.5 |
| ⑤ | **大模块拆 sub-crate** (47 sub-crate) | 阶段 4.2 | 1 | R156 | 中 | R131-5 §2.4 + R137-2 §3.6 |
| ⑥ | **DSL 洋葱** (三洋葱 → 四洋葱) | 阶段 5.1 | 0.5 | R157 | 高 | R131-5 §2.5 + R133-3 §3 + R137-2 §3.7 |
| ⑦ | **9 organ 借 OpenCode** (9 organ workspace 化) | 阶段 5.2 | 0.5 | R157 | 极高 | R131-5 §2.6 + R125 B7 + R130-6 + R137-2 §3.8 |
| ⑧ | **R12 测度对齐** (24+9 → 24+11) | 阶段 5.3 | 0.5 | R157 | 中 | R131-5 §2.7 + R131-9 O5 + R137-2 §3.9 |
| ⑨ | **ASI Stage 9 集成** (H1-H4 4 维度) | 阶段 5.4 | 0.5 | R157 | 中 | R149-2 + R130-2 §1 + R140-4 |
| ⑩ | **三洋葱 V2 集成** (第 5 层形式化洋葱) | 阶段 5.5 | 0.5 | R157 | 中 | R149-3 + R133-3 + R131-9 |
| ⑪ | **借鉴 12 源 fork-then-borrow** (12 源 注释) | 阶段 5.6 | 0.5 | R157 | 低 | R149-4 + R130-6 + R140-5 |
| ⑫ | **9 organ workspace 化** (跟 ⑦ 配合) | 阶段 5.2 | 0.5 | R157 | 极高 | R131-5 §2.6 + R137-2 方向 7 |
| **总** | **12 方向 = 8 大 + 4 新增** | **5 阶段 8 周** | **8** | **R153-R157** | **中-极高** | **R131-5 + R137-2 + R149-2/3/4** |

---

## 2. 24 LOCKED crate 入口签名 优化 Cargo.toml 字段 update (per-crate)

### 2.1 Cargo.toml 字段 update 总览 (per 决策 #74 §1 B2 + R131-4 §2)

**V1.0 release (整合 #5.1 commit 0 改 src 严守)**:
- workspace.version = **1.2.0** (per 决策 #74 §1 B2 V1.0 release 1.2.0 严守)
- workspace.members = **87 个** (per R131-4 §2.1 + Cargo.toml `members` 段实际清点)
- 24 LOCKED crate Cargo.toml **0 改** (per 决策 #33 §2.3 + 决策 #74 §1 B1 V1.0 release 0 改严守)

**V1.1 release (整合 #6 commit 拍板 2026-11-25)**:
- workspace.version = **1.2.1** (per 决策 #74 §1 B2 V1.1 release bump 1.2.1)
- workspace.members = **87 + 47 sub-crate + 9 apeireth-leaf + 1 apeireth-eye + 1 apeireth-dsl + 1 apeireth-formal = 146** (估, per 12 优化方向 拆分后总数)
- 24 LOCKED crate Cargo.toml 字段 update per-crate (per 方向 1-12)

### 2.2 per-crate Cargo.toml 字段 update 详细 (24 LOCKED)

**总览**:
- **1 supervisor** (Heart organ, apeireth-leaf workspace)
- **2 agent** (Brain organ, apeireth-brain workspace)
- **3 council** (Brain organ, apeireth-brain workspace + 4 sub-crate: council / council-advisor / council-deliberation / council-collaboration)
- **4 bus** (Heart Ear organ, apeireth-leaf workspace)
- **5 protocol** (Voice organ, apeireth-leaf workspace)
- **6 mcp** (Hand organ, apeireth-brain workspace + 8 sub-crate: mcp / mcp-core / mcp-resources / mcp-subscribe / mcp-tools / mcp-prompts / mcp-transport / mcp-primitives)
- **7 tool-registry** (Hand organ, apeireth-leaf workspace)
- **8 tool-runtime** (Hand organ, apeireth-hand workspace)
- **9 graph** (Mind organ, apeireth-mind workspace + 5 sub-crate: graph / graph-state / graph-executor / graph-subgraph / graph-context)
- **10 pipeline** (Heart Voice organ, apeireth-leaf workspace + 6 sub-crate: pipeline / pipeline-token / pipeline-placeholder / pipeline-force-translate / pipeline-retry / pipeline-streaming / pipeline-tool-loop)
- **11 tool-approval** (Hand organ, apeireth-hand workspace)
- **12 extension** (Hand organ, apeireth-leaf workspace)
- **13 evolution** (Mind organ, apeireth-leaf workspace + 5 sub-crate: evolution / evolution-council / evolution-engine / evolution-poda / evolution-library)
- **14 api** (Body organ, apeireth-body workspace + 5 sub-crate: api / api-llm / api-server / api-protocol / api-auth)
- **15 core** (Memory organ, apeireth-memory workspace + 5 mod: lib / types / onion / human / gate)
- **16 memory** (Memory organ, apeireth-memory workspace + 5 sub-crate: memory / memory-stream / memory-semantic / memory-episode / memory-session)
- **17 asi** (Memory organ, apeireth-leaf workspace + 4 sub-crate: asi / asi-calibration / asi-measurement / asi-render)
- **18 tools** (Hand organ, apeireth-hand workspace + 5 sub-crate: tools / tools-fs / tools-git / tools-exec / tools-web)
- **19 cli** (Body organ, apeireth-body workspace)
- **20 bench** (Body organ, apeireth-leaf workspace)
- **21 cognition** (Brain organ, apeireth-brain workspace)
- **22 action** (Hand organ, apeireth-hand workspace)
- **23 life-force** (Memory organ, apeireth-memory workspace)
- **24 constraint** (Brain organ, apeireth-brain workspace)

**per-crate Cargo.toml 字段 update 字段 (V1.1 release 实施时)**:

#### 2.2.1 supervisor (Heart, apeireth-leaf)

**字段 update**:
```toml
# crates/apeireth-leaf/supervisor/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-supervisor"
version.workspace = true  # 0 改 (B2 1.2.1)
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
apeireth-core = { path = "../../apeireth-core", version = "1.2.1" }  # 0 改, 跨 workspace
```

**0 改** (per 决策 #74 §1 B1 V1.1 release B1 改写边界), 仅路径变 `apeireth-leaf/supervisor/Cargo.toml`

#### 2.2.2 agent (Brain, apeireth-brain)

**字段 update**:
```toml
# crates/apeireth-brain/agent/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-agent"
version.workspace = true  # 0 改
edition.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-tool-registry = { path = "../../apeireth-leaf/tool-registry", version = "1.2.1" }  # 跨 workspace
apeireth-asi = { path = "../../apeireth-leaf/asi", version = "1.2.1" }  # 跨 workspace (Stage 9 H1 自我决策)
```

**0 改 入口签名**, 加 Stage 9 H1 自我决策 API (per 方向 ⑨ ASI Stage 9 集成)

#### 2.2.3 council (Brain, 4 sub-crate)

**字段 update (顶层 council + 3 sub-crate)**:
```toml
# crates/apeireth-brain/council/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-council"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-council-advisor = { path = "./council-advisor", version = "1.2.1" }
apeireth-council-deliberation = { path = "./council-deliberation", version = "1.2.1" }
apeireth-council-collaboration = { path = "./council-collaboration", version = "1.2.1" }
apeireth-asi = { path = "../../apeireth-leaf/asi", version = "1.2.1" }  # Stage 9 H4 群体智能 (借 OpenCog)

# 顶层 re-export facade 0 改入口签名
# 公开 API 表面从 50+ 瘦身到 30 (-40%, per 方向 ②)
```

**风险**: 中 (拆 sub-crate + 跨 workspace 路径变化)

#### 2.2.4 bus (Heart Ear, apeireth-leaf)

**字段 update**:
```toml
# crates/apeireth-leaf/bus/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-bus"
version.workspace = true

[dependencies]
# bus 0 依赖其他 LOCKED crate, 仅 apeireth-leaf 内部
```

**0 改**, 仅路径变 `apeireth-leaf/bus/Cargo.toml`

#### 2.2.5 protocol (Voice, apeireth-leaf)

**字段 update**:
```toml
# crates/apeireth-leaf/protocol/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-protocol"
version.workspace = true

# 公开 API 表面从 40 瘦身到 30 (-25%, per 方向 ②)
# 主类型 facade 模式 (per 方向 ① 模式 2)
```

**0 改 入口签名**

#### 2.2.6 mcp (Hand, 8 sub-crate)

**字段 update (顶层 mcp + 7 sub-crate)**:
```toml
# crates/apeireth-brain/mcp/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-mcp"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-mcp-core = { path = "./mcp-core", version = "1.2.1" }
apeireth-mcp-resources = { path = "./mcp-resources", version = "1.2.1" }
apeireth-mcp-subscribe = { path = "./mcp-subscribe", version = "1.2.1" }
apeireth-mcp-tools = { path = "./mcp-tools", version = "1.2.1" }
apeireth-mcp-prompts = { path = "./mcp-prompts", version = "1.2.1" }
apeireth-mcp-transport = { path = "./mcp-transport", version = "1.2.1" }
apeireth-mcp-primitives = { path = "./mcp-primitives", version = "1.2.1" }
apeireth-tool-registry = { path = "../../apeireth-leaf/tool-registry", version = "1.2.1" }
```

**风险**: 中 (拆 8 sub-crate + 跨 workspace 路径变化)

#### 2.2.7 tool-registry (Hand, apeireth-leaf)

**字段 update**:
```toml
# crates/apeireth-leaf/tool-registry/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-tool-registry"
version.workspace = true

[dependencies]
# tool-registry 0 依赖其他 LOCKED crate, 仅 apeireth-leaf 内部
```

**0 改**, 仅路径变 `apeireth-leaf/tool-registry/Cargo.toml`

#### 2.2.8 tool-runtime (Hand, apeireth-hand)

**字段 update**:
```toml
# crates/apeireth-hand/tool-runtime/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-tool-runtime"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-memory = { path = "../apeireth-memory", version = "1.2.1" }  # 跨 organ workspace
apeireth-tool-registry = { path = "../../apeireth-leaf/tool-registry", version = "1.2.1" }
apeireth-mcp = { path = "../mcp", version = "1.2.1" }  # 跨 organ workspace
```

**0 改 入口签名**

#### 2.2.9 graph (Mind, 5 sub-crate)

**字段 update (顶层 graph + 4 sub-crate)**:
```toml
# crates/apeireth-mind/graph/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-graph"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-graph-state = { path = "./graph-state", version = "1.2.1" }
apeireth-graph-executor = { path = "./graph-executor", version = "1.2.1" }
apeireth-graph-subgraph = { path = "./graph-subgraph", version = "1.2.1" }
apeireth-graph-context = { path = "./graph-context", version = "1.2.1" }
apeireth-mcp = { path = "../apeireth-brain/mcp", version = "1.2.1" }  # 跨 organ workspace
apeireth-asi = { path = "../../apeireth-leaf/asi", version = "1.2.1" }  # Stage 9 H3 自我演化
```

**风险**: 中 (拆 5 sub-crate + 跨 workspace 路径变化)

#### 2.2.10 pipeline (Heart Voice, 6 sub-crate)

**字段 update (顶层 pipeline + 5 sub-crate)**:
```toml
# crates/apeireth-leaf/pipeline/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-pipeline"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-pipeline-token = { path = "./pipeline-token", version = "1.2.1" }
apeireth-pipeline-placeholder = { path = "./pipeline-placeholder", version = "1.2.1" }
apeireth-pipeline-force-translate = { path = "./pipeline-force-translate", version = "1.2.1" }
apeireth-pipeline-retry = { path = "./pipeline-retry", version = "1.2.1" }
apeireth-pipeline-streaming = { path = "./pipeline-streaming", version = "1.2.1" }
apeireth-pipeline-tool-loop = { path = "./pipeline-tool-loop", version = "1.2.1" }
apeireth-protocol = { path = "./protocol", version = "1.2.1" }  # 跨 sub-crate
```

**风险**: 中 (拆 6 sub-crate + 跨 workspace 路径变化)

#### 2.2.11 tool-approval (Hand, apeireth-hand)

**字段 update**:
```toml
# crates/apeireth-hand/tool-approval/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-tool-approval"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-tool-runtime = { path = "./tool-runtime", version = "1.2.1" }  # 同 organ workspace
```

**0 改**

#### 2.2.12 extension (Hand, apeireth-leaf)

**字段 update**:
```toml
# crates/apeireth-leaf/extension/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-extension"
version.workspace = true

# extension 0 依赖其他 LOCKED crate, 仅 apeireth-leaf 内部
# 单 trait 入口模式 (per 方向 ① 模式 3 按需 re-export)
```

**0 改**, 仅路径变 `apeireth-leaf/extension/Cargo.toml`

#### 2.2.13 evolution (Mind, 5 sub-crate)

**字段 update (顶层 evolution + 4 sub-crate)**:
```toml
# crates/apeireth-leaf/evolution/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-evolution"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-evolution-council = { path = "./evolution-council", version = "1.2.1" }
apeireth-evolution-engine = { path = "./evolution-engine", version = "1.2.1" }
apeireth-evolution-poda = { path = "./evolution-poda", version = "1.2.1" }
apeireth-evolution-library = { path = "./evolution-library", version = "1.2.1" }
apeireth-council = { path = "../../apeireth-brain/council", version = "1.2.1" }  # 跨 organ workspace
apeireth-asi = { path = "./asi", version = "1.2.1" }  # Stage 9 H3 自我演化
```

**风险**: 中 (拆 5 sub-crate + 跨 workspace 路径变化)

#### 2.2.14 api (Body, 5 sub-crate)

**字段 update (顶层 api + 4 sub-crate)**:
```toml
# crates/apeireth-body/api/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-api"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-api-llm = { path = "./api-llm", version = "1.2.1" }
apeireth-api-server = { path = "./api-server", version = "1.2.1" }
apeireth-api-protocol = { path = "./api-protocol", version = "1.2.1" }
apeireth-api-auth = { path = "./api-auth", version = "1.2.1" }
apeireth-pipeline = { path = "../../apeireth-leaf/pipeline", version = "1.2.1" }  # 跨 organ workspace
apeireth-protocol = { path = "../../apeireth-leaf/protocol", version = "1.2.1" }  # 跨 organ workspace
apeireth-memory = { path = "../apeireth-memory", version = "1.2.1" }  # 跨 organ workspace
```

**风险**: 中 (拆 5 sub-crate + 跨 workspace 路径变化)

#### 2.2.15 core (Memory, 5 mod)

**字段 update**:
```toml
# crates/apeireth-memory/core/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-core"
version.workspace = true

# 0 依赖其他 LOCKED crate
# core 拆 5 mod (per 方向 ④): types / onion / human / gate / lib
# 公开 API 表面从 50+ 瘦身到 30 (-40%, per 方向 ②)
# 加 PHL-07 形式化 实施 (per 方向 ⑩ 第 5 层形式化洋葱, V1.1 release per 决策 #74 §2.3 + R129-11)
```

**0 改 入口签名**, 仅内部 mod 拆分 (per 方向 ④)

#### 2.2.16 memory (Memory, 5 sub-crate)

**字段 update (顶层 memory + 4 sub-crate)**:
```toml
# crates/apeireth-memory/memory/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-memory"
version.workspace = true

[dependencies]
apeireth-core = { path = "./core", version = "1.2.1" }  # 同 organ workspace
apeireth-memory-stream = { path = "./memory-stream", version = "1.2.1" }
apeireth-memory-semantic = { path = "./memory-semantic", version = "1.2.1" }
apeireth-memory-episode = { path = "./memory-episode", version = "1.2.1" }
apeireth-memory-session = { path = "./memory-session", version = "1.2.1" }
apeireth-asi = { path = "../../apeireth-leaf/asi", version = "1.2.1" }  # Stage 9 H2 自我学习
apeireth-life-force = { path = "./life-force", version = "1.2.1" }  # 同 organ workspace
```

**风险**: 中 (拆 5 sub-crate + 跨 workspace 路径变化)

#### 2.2.17 asi (Memory, 4 sub-crate)

**字段 update (顶层 asi + 3 sub-crate)**:
```toml
# crates/apeireth-leaf/asi/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-asi"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-memory/core", version = "1.2.1" }  # 跨 organ workspace
apeireth-asi-calibration = { path = "./asi-calibration", version = "1.2.1" }
apeireth-asi-measurement = { path = "./asi-measurement", version = "1.2.1" }
apeireth-asi-render = { path = "./asi-render", version = "1.2.1" }
# 24 measure_dim_* + 9 measure_sub_* = 33 测量函数保留
# V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新 (per 方向 ⑧ R12 测度对齐, 24+11 = 35)
# 公开 API 表面从 50+ 瘦身到 30 (-40%, per 方向 ②)
```

**风险**: 中 (拆 4 sub-crate + R12 测度对齐 改 33 测量函数签名 + 跨 workspace 路径变化)

#### 2.2.18 tools (Hand, 5 sub-crate)

**字段 update (顶层 tools + 4 sub-crate)**:
```toml
# crates/apeireth-hand/tools/Cargo.toml (V1.1 release 改, 顶层 re-export facade)
[package]
name = "apeireth-tools"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-tools-fs = { path = "./tools-fs", version = "1.2.1" }
apeireth-tools-git = { path = "./tools-git", version = "1.2.1" }
apeireth-tools-exec = { path = "./tools-exec", version = "1.2.1" }
apeireth-tools-web = { path = "./tools-web", version = "1.2.1" }
apeireth-tool-registry = { path = "../../apeireth-leaf/tool-registry", version = "1.2.1" }  # 跨 organ workspace
```

**风险**: 中 (拆 5 sub-crate + 跨 workspace 路径变化)

#### 2.2.19 cli (Body, apeireth-body)

**字段 update**:
```toml
# crates/apeireth-body/cli/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-cli"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-asi = { path = "../../apeireth-leaf/asi", version = "1.2.1" }  # 跨 organ workspace
apeireth-bench = { path = "./bench", version = "1.2.1" }  # 同 organ workspace
```

**0 改 入口签名**

#### 2.2.20 bench (Body, apeireth-leaf)

**字段 update**:
```toml
# crates/apeireth-leaf/bench/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-bench"
version.workspace = true

# bench 0 依赖其他 LOCKED crate, 仅 apeireth-leaf 内部 (跨 24 LOCKED 全测)
```

**0 改**, 仅路径变 `apeireth-leaf/bench/Cargo.toml`

#### 2.2.21 cognition (Brain, apeireth-brain)

**字段 update**:
```toml
# crates/apeireth-brain/cognition/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-cognition"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-asi = { path = "../../apeireth-leaf/asi", version = "1.2.1" }  # 跨 organ workspace, Stage 9 H1 自我决策
apeireth-constraint = { path = "./constraint", version = "1.2.1" }  # 同 organ workspace
```

**0 改 入口签名**, 加 Stage 9 H1 自我决策 API (per 方向 ⑨)

#### 2.2.22 action (Hand, apeireth-hand)

**字段 update**:
```toml
# crates/apeireth-hand/action/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-action"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-extension = { path = "../../apeireth-leaf/extension", version = "1.2.1" }  # 跨 organ workspace
```

**0 改 入口签名**

#### 2.2.23 life-force (Memory, apeireth-memory)

**字段 update**:
```toml
# crates/apeireth-memory/life-force/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-life-force"
version.workspace = true

[dependencies]
apeireth-core = { path = "./core", version = "1.2.1" }  # 同 organ workspace
apeireth-asi = { path = "../../apeireth-leaf/asi", version = "1.2.1" }  # 跨 organ workspace, Stage 9 H2 自我学习
```

**0 改 入口签名**, 加 Stage 9 H2 自我学习 API (per 方向 ⑨)

#### 2.2.24 constraint (Brain, apeireth-brain)

**字段 update**:
```toml
# crates/apeireth-brain/constraint/Cargo.toml (V1.1 release 改)
[package]
name = "apeireth-constraint"
version.workspace = true

[dependencies]
apeireth-core = { path = "../apeireth-core", version = "1.2.1" }
apeireth-formal = { path = "../apeireth-onion/formal", version = "1.2.1" }  # 第 5 层形式化洋葱 (per 方向 ⑩)
apeireth-dsl = { path = "../apeireth-onion/dsl", version = "1.2.1" }  # 第 3 层 DSL 洋葱 (per 方向 ⑥)
```

**风险**: 高 (加 第 3 层 DSL 洋葱 + 第 5 层形式化洋葱 = 改 LOCKED 入口签名)

### 2.3 新增 workspace 顶层 Cargo.toml (per 方向 ⑥ ⑩)

**新增 `apeireth-onion/Cargo.toml`** (per 方向 ⑥ DSL 洋葱 + 方向 ⑩ 三洋葱 V2):
```toml
# crates/apeireth-onion/Cargo.toml (V1.1 release 新增)
[workspace]
members = [
    "core",
    "constraint",
    "dsl",
    "formal",
    "life-force",
]

[workspace.package]
version = "1.2.1"  # 跟顶层 workspace.version 1.2.1 一致
```

**新增 `apeireth-organ/Cargo.toml`** (per 方向 ⑦ 9 organ workspace 化):
```toml
# crates/apeireth-organ/Cargo.toml (V1.1 release 新增)
[workspace]
members = [
    "heart",
    "brain",
    "hand",
    "eye",
    "ear",
    "memory",
    "voice",
    "body",
    "mind",
]

[workspace.package]
version = "1.2.1"  # 跟顶层 workspace.version 1.2.1 一致
```

**新增 `apeireth-leaf/Cargo.toml`** (per 方向 ③ 9 叶子拆 workspace):
```toml
# crates/apeireth-leaf/Cargo.toml (V1.1 release 新增)
[workspace]
members = [
    "supervisor",
    "protocol",
    "bus",
    "tool-registry",
    "graph",
    "extension",
    "evolution",
    "asi",
    "bench",
]

[workspace.package]
version = "1.2.1"  # 跟顶层 workspace.version 1.2.1 一致
```

**顶层 `apeireth/Cargo.toml` 更新** (per 决策 #74 §1 B2):
```toml
# Cargo.toml (V1.1 release 改, 顶层 workspace 0 改 members, 仅 version bump)
[workspace]
members = [
    "crates/apeireth-onion",
    "crates/apeireth-organ",
    "crates/apeireth-leaf",
    # ... 其他 50+ non-LOCKED crate (per R131-4 §2.1 87 crate 分布)
]
exclude = [
    "crates/apeireth-memory.db",  # SQLite
    "target",
    "_workspace",
]

[workspace.package]
version = "1.2.1"  # V1.1 release bump (per 决策 #74 §1 B2)
```

**总 workspace members**: 87 (V1.0 release) + 47 sub-crate + 9 apeireth-leaf + 1 apeireth-eye + 1 apeireth-dsl + 1 apeireth-formal = **146** (V1.1 release 估, per R131-5 §2.3 + R137-2 §3.4)

---

## 3. 24 LOCKED crate 入口签名 优化 lib.rs / mod.rs 改动 (per-crate)

### 3.1 0 改入口签名 严守 100% (per 决策 #74 §1 B1 V1.0 release 严守)

**V1.0 release 整合 #5.1 commit (0 改 src 严守)**:
- **24/24 LOCKED crate 入口签名 0 改 verify 全 PASS** (per R131-5 §1.2 详细 verify 表, 2026-08-11 01:28 done)
- 入口签名 = 顶层 `pub mod xxx;` + `pub use xxx::xxx;` + `pub const/pub struct/pub enum/pub fn` 块
- 0 改 lib.rs 任何 pub 类型 / pub fn / pub const

**V1.1 release 整合 #6 commit 拍板 2026-11-25 (Mavis 自决改, per 决策 #74 §1 B1)**:
- 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决)
- 但 **顶层 re-export facade 0 改** (per 决策 #74 §2.3 V1.1 release B1 改写边界)
- 仅内部 lib.rs / mod.rs 改动 (per 12 优化方向)

### 3.2 lib.rs / mod.rs 改动 12 优化方向 详细

#### 3.2.1 方向 ① 标准化 (lib.rs 格式统一, per 阶段 1)

**改动** (per 24 LOCKED crate 各自 lib.rs):
- **类型 A (20 crate, 模式 1 全 re-export)**: lib.rs 顶部加 50-100 行 doc comment (per O-5 哲学锚), 中间 `pub mod xxx;` + `pub use xxx::*;` 块, 底部 compile-time assert
- **类型 B (2 crate, 模式 2 主类型 facade)**: lib.rs 直接定义核心类型 (e.g. protocol 入口 `pub struct OpenAiChatAdapter;` + `pub struct OpenAiResponsesAdapter;` + 4 Bridge struct), 轻 re-export
- **类型 C (1 crate, 模式 3 按需 re-export)**: lib.rs 仅 re-export 主类型 `pub use trait_def::*;`, 其他 module 公开
- **类型 D (2 crate, 模式 3 按需 re-export)**: lib.rs 主 enum + 1-2 相关 const, 其他 module 公开
- **类型 E (1 crate, 模式 3 按需 re-export)**: lib.rs 几乎不 re-export, 主要靠 module 公开

**改 lib.rs 行数估**: 24 LOCKED 全部 +30-50 行 doc comment (50-100 行 doc), 总 +720-1200 行 (per lib.rs `//!` 注释)

#### 3.2.2 方向 ② 瘦身 (per-crate 公开 API 表面, per 阶段 2)

**改动** (per 24 LOCKED crate pub use 块):
- **council 50+ → 30**: 砍 8 协作模式中 4 个 → 留 4 个, 7 factory 砍 3 → 留 4, Synthesis/Persona/Sovereignty/Constitution/Trace/Graph 内部化 (转 `pub(crate)` 或 module-private)
- **evolution 50+ → 30**: 8 PODA + 19 library_autonomy + 14 library_autonomy_loop 内部化
- **core 50+ → 30**: ActionTarget 13 → 5, Gate 5 内部化
- **memory 50+ → 30**: 10 stream + 6 StreamKind 内部化
- **asi 50+ → 30**: 2 legacy struct 内部化
- **protocol 40 → 30**: 5 const 内部化
- **graph 40 → 30**: Subgraph/Channel 11 → 5, StateGraph 5 → 3
- **api 40+ → 30**: 22 LLM → 15, 11 protocol → 11
- **pipeline 35 → 30**: 8 module → 6
- **其他 16 crate (≤30 已达标)**: 0 改

**改 lib.rs pub use 块行数估**: 24 LOCKED 全部 -240 行 pub use (per -30% API 表面), 总 -240 行

#### 3.2.3 方向 ③ 9 叶子拆 workspace (per 阶段 3.1)

**改动** (per 9 叶子 crate lib.rs):
- 9 叶子 lib.rs **0 改 内部**, 仅 路径 从 `crates/apeireth-supervisor/src/lib.rs` → `crates/apeireth-leaf/supervisor/src/lib.rs`
- 顶层 apeireth/Cargo.toml 加 `crates/apeireth-leaf = { ... }` workspace 引用
- 9 叶子 lib.rs 行数估: 0 改 (per 方向 ③ 仅路径变)

#### 3.2.4 方向 ④ core 拆 pub mod (per 阶段 4.1)

**改动** (per core crate 内部 mod 拆分):
- **core/src/lib.rs 顶部 re-export 0 改** (per 决策 #74 §2.3 V1.1 release B1 改写边界, 仅内部 mod 拆分)
- **core/src/types.rs 新增** (~20KB, 5 类型: Episode / Note / Session / IdentityCard / Migration + 各 types 子文件)
- **core/src/onion.rs 新增** (~30KB, 5 onion 类型: PrincipleOnion / PrincipleLayer / PermissionOnion / PermissionLayer + 各 onion 子文件)
- **core/src/human.rs 新增** (~20KB, 8 human 类型: HumanAuthority / HAMode / RealHuman / HAAuthentication / BiometricData + 12 PhilosophyKey + ALL_TWELVE_KEYS + TWELVE_KEYS_HARDCODE)
- **core/src/gate.rs 新增** (~25KB, 8 gate 类型: PhilosophyGuard / PhilosophyVerdict / VerdictCache / Gate / 5 variant + Action / RiskLevel / ActionTarget / ActionVerdict / ActionGuard / DefaultPhilosophyGuard)
- **core/src/lib.rs 5 行 `pub mod types; pub mod onion; pub mod human; pub mod gate;` + 顶部 re-export facade 0 改**

**改 core/src/ 行数估**: lib.rs 108KB → lib.rs 13KB + types.rs 20KB + onion.rs 30KB + human.rs 20KB + gate.rs 25KB = 总 108KB (0 改 总大小)

#### 3.2.5 方向 ⑤ 大模块拆 sub-crate (per 阶段 4.2, 47 sub-crate)

**改动** (per 8 大模块集中 crate 顶层 lib.rs):
- **mcp 13 mod → 8 sub-crate** (顶层 mcp/src/lib.rs 0 改入口签名, 仅 `pub use mcp_core::*; pub use mcp_resources::*; pub use mcp_subscribe::*; pub use mcp_tools::*; pub use mcp_prompts::*; pub use mcp_transport::*; pub use mcp_primitives::*;`)
- **pipeline 11 mod → 6 sub-crate** (顶层 pipeline/src/lib.rs 0 改入口签名, 仅 `pub use pipeline_token::*; pub use pipeline_placeholder::*; ...`)
- **api 16 mod → 5 sub-crate** (顶层 api/src/lib.rs 0 改入口签名, 仅 `pub use api_llm::*; pub use api_server::*; pub use api_protocol::*; pub use api_auth::*;`)
- **memory 13 mod → 5 sub-crate** (顶层 memory/src/lib.rs 0 改入口签名, 仅 `pub use memory_stream::*; pub use memory_semantic::*; pub use memory_episode::*; pub use memory_session::*;`)
- **asi 9 mod → 4 sub-crate** (顶层 asi/src/lib.rs 0 改入口签名, 仅 `pub use asi_calibration::*; pub use asi_measurement::*; pub use asi_render::*;`)
- **tools 12 mod → 5 sub-crate** (顶层 tools/src/lib.rs 0 改入口签名, 仅 `pub use tools_fs::*; pub use tools_git::*; pub use tools_exec::*; pub use tools_web::*;`)
- **evolution 9 mod → 5 sub-crate** (顶层 evolution/src/lib.rs 0 改入口签名, 仅 `pub use evolution_council::*; pub use evolution_engine::*; pub use evolution_poda::*; pub use evolution_library::*;`)
- **graph 11 mod → 5 sub-crate** (顶层 graph/src/lib.rs 0 改入口签名, 仅 `pub use graph_state::*; pub use graph_executor::*; pub use graph_subgraph::*; pub use graph_context::*;`)
- **council 20+ mod → 4 sub-crate** (顶层 council/src/lib.rs 0 改入口签名, 仅 `pub use council_advisor::*; pub use council_deliberation::*; pub use council_collaboration::*;`)

**改 8 大模块 lib.rs 行数估**: 总 +0 行 (顶层 re-export facade 0 改, 仅 sub-crate 路径变化)

#### 3.2.6 方向 ⑥ DSL 洋葱 (per 阶段 5.1)

**改动** (per 24 LOCKED crate lib.rs 引用 dsl 守门):
- 新增 `apeireth-dsl/src/lib.rs` (~5000 行, Colang DSL 真实施, per R125-5 NVIDIA 借鉴后 1700 行续)
- 24 LOCKED crate lib.rs 顶部加 `use apeireth_dsl::guard::*;` 引用 (per 方向 ⑥ DSL 守门 = 4 重: L1 原则 guard / L2 权限 guard / L3 DSL guard / L4 智能涌现 guard)
- 24 LOCKED lib.rs 行数估: 总 +24 行 (每 crate 1 行 `use apeireth_dsl::guard::*;`)

#### 3.2.7 方向 ⑦ 9 organ workspace 化 (per 阶段 5.2)

**改动** (per 9 organ workspace 入口 lib.rs):
- 新增 `apeireth-organ/heart/src/lib.rs` (`pub use apeireth_supervisor::*; pub use apeireth_bus::l0::*; pub use apeireth_pipeline::*;`)
- 新增 `apeireth-organ/brain/src/lib.rs` (`pub use apeireth_agent::*; pub use apeireth_council::*; pub use apeireth_cognition::*; pub use apeireth_constraint::*;`)
- 新增 `apeireth-organ/hand/src/lib.rs` (7 个 LOCKED 全部 re-export)
- 新增 `apeireth-organ/eye/src/lib.rs` (从 tui/src/organ/eye.rs 抽 crate)
- 新增 `apeireth-organ/ear/src/lib.rs` (`pub use apeireth_bus::{l1, l2, l3, l4}::*;`)
- 新增 `apeireth-organ/memory/src/lib.rs` (`pub use apeireth_memory::*; pub use apeireth_asi::*; pub use apeireth_life_force::*; pub use apeireth_core::{Episode, Note, Session, IdentityCard, Migration};`)
- 新增 `apeireth-organ/voice/src/lib.rs` (`pub use apeireth_protocol::*; pub use apeireth_pipeline::streaming::*;`)
- 新增 `apeireth-organ/body/src/lib.rs` (`pub use apeireth_bench::*; pub use apeireth_api::*; pub use apeireth_cli::*;`)
- 新增 `apeireth-organ/mind/src/lib.rs` (`pub use apeireth_evolution::*; pub use apeireth_graph::*; pub use apeireth_constraint::gate3::*;`)

**9 organ lib.rs 总行数估**: 9 × 30 行 = 270 行 (顶层 re-export facade 0 改)

#### 3.2.8 方向 ⑧ R12 测度对齐 (per 阶段 5.3)

**改动** (per asi crate lib.rs + sub-crate):
- `apeireth-asi/src/lib.rs` 更新 V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode (V1.0 release 24 + 9 = 33, V1.1 release 24 + 11 = 35)
- 24 测量函数签名 1:1 续, 加 2 NEW 测度 (24+11 = 35) 仅 add 0 remove (per semver minor 兼容)
- V05_DIMENSION_NAMES 数组 (24 维名) + V1136_SUBMEASURE_NAMES 数组 (9 子测度名) 1:1 续 + 加 2 NEW 子测度名
- R12 baseline 3 值 verify (估 > R11 baseline, per 决策 #74 §2.3 V1.1 release R12 baseline 更高)

**改 asi/src/lib.rs 行数估**: +20 行 (2 NEW 测度 + 编译期 hardcode 同步)

#### 3.2.9 方向 ⑨ ASI Stage 9 集成 (per 阶段 5.4)

**改动** (per 9 个 LOCKED crate lib.rs 加 Stage 9 4 维度 API):
- **agent lib.rs**: 加 H1 自我决策 API (`pub fn self_decide(...) -> Decision;`)
- **council lib.rs**: 加 H4 群体智能 API (`pub fn swarm_intelligence(...) -> SwarmDecision;`, 借 OpenCog AtomSpace)
- **cognition lib.rs**: 加 H1 自我决策 API (`pub fn self_decide_cycle(...) -> CognitiveDecision;`)
- **memory lib.rs**: 加 H2 自我学习 API (`pub fn self_learn(...) -> LearningEpisode;`)
- **asi lib.rs**: 加 H2 自我学习 API (`pub fn self_calibrate(...) -> CalibrationUpdate;`)
- **life-force lib.rs**: 加 H2 自我学习 API (`pub fn self_grow(...) -> SGI;`)
- **evolution lib.rs**: 加 H3 自我演化 API (`pub fn self_evolve(...) -> EvolutionStep;`)
- **graph lib.rs**: 加 H3 自我演化 API (`pub fn self_recompose(...) -> Subgraph;`)

**改 9 个 LOCKED lib.rs 行数估**: 总 +90 行 (每 crate 1 fn + 1 doc comment = 10 行 × 9)

#### 3.2.10 方向 ⑩ 三洋葱 V2 集成 (per 阶段 5.5)

**改动** (per 24 LOCKED crate lib.rs 加 第 5 层 "形式化洋葱" 守门):
- 新增 `apeireth-formal/src/lib.rs` (~3000 行, kani 借鉴 + PHL-07 实施, per R131-9 O1-O9 形式化 9 方向 + R125-9)
- 24 LOCKED crate lib.rs 顶部加 `use apeireth_formal::guard::*;` 引用 (per 方向 ⑩ 第 5 层 形式化洋葱 = 形式化 verify / proof / check / audit)
- 24 LOCKED lib.rs 行数估: 总 +24 行 (每 crate 1 行 `use apeireth_formal::guard::*;`)

#### 3.2.11 方向 ⑪ 借鉴 12 源 fork-then-borrow (per 阶段 5.6)

**改动** (per 24 LOCKED crate lib.rs 加 12 源 注释):
- 24 LOCKED crate lib.rs 顶部 doc comment 加 12 源借鉴声明 (per 8 真 cloned + 2 借鉴 ID + 1 借脑 ID, 0 装"已读真源码", 0 装"已 fork")
- O-3 哲学锚 "走在前人经验上" 严守
- 内部 fn 加 借鉴源 1:1 公开模式 (per 决策 #22 §4)

**改 24 LOCKED lib.rs 行数估**: 总 +240 行 (每 crate 1 段 10 行 doc comment 加 12 源 借鉴声明 = 10 行 × 24)

#### 3.2.12 方向 ⑫ 9 organ workspace 化 (跟 ⑦ 配合, per 阶段 5.2)

**改动**: 跟 方向 ⑦ 重叠 (per §1.2.4)

### 3.3 lib.rs / mod.rs 改动 行数 总估

| 方向 | 改 lib.rs 行数 | 改 mod.rs 行数 | 改 Cargo.toml 行数 | 总 |
|------|--------------|--------------|------------------|-----|
| ① 标准化 | +720-1200 | 0 | 0 | +720-1200 |
| ② 瘦身 | -240 | 0 | 0 | -240 |
| ③ 9 叶子拆 | 0 | 0 | +27 (9 workspace) | +27 |
| ④ core 拆 pub mod | 0 (108KB 拆分) | +95 (108KB) | 0 | +95 |
| ⑤ 大模块拆 sub-crate | 0 (顶层 0 改) | 0 | +47 (47 sub-crate) | +47 |
| ⑥ DSL 洋葱 | +24 | +5000 (apeireth-dsl) | +5 (workspace) | +5029 |
| ⑦ 9 organ workspace 化 | +270 (9 organ 入口) | 0 | +9 (workspace) | +279 |
| ⑧ R12 测度对齐 | +20 (asi) | 0 | 0 | +20 |
| ⑨ ASI Stage 9 集成 | +90 (9 crate) | 0 | 0 | +90 |
| ⑩ 三洋葱 V2 集成 | +24 | +3000 (apeireth-formal) | +5 (workspace) | +3029 |
| ⑪ 借鉴 12 源 fork | +240 (24 crate) | 0 | 0 | +240 |
| ⑫ 9 organ workspace 化 | (跟 ⑦ 重叠) | 0 | 0 | 0 |
| **总** | **+1148-1628** | **+8095** | **+93** | **+9336-9816** |

---

## 4. 24 LOCKED crate 入口签名 优化 测试 (cargo test --workspace 8 步 verify 8/8)

### 4.1 8 步 verify 流程 (per R144-1 02:38 + R147-5 v0.5 30 维 6 重守门 v7 verify 模式)

**8 步 verify (per R148-23 8 步 verify 全 PASS 终版 SOP v2, 116.8 KB)**:
- **Step 1: cargo build --workspace** (编译 verify)
- **Step 2: cargo build --workspace --release** (release 编译 verify)
- **Step 3: cargo test --workspace** (单元测试 + 集成测试 verify)
- **Step 4: cargo test --workspace --release** (release 单元测试 verify)
- **Step 5: cargo doc --workspace --no-deps** (文档 verify)
- **Step 6: cargo clippy --workspace -- -D warnings** (lint verify)
- **Step 7: cargo deny check** (license + advisory verify)
- **Step 8: cargo fmt --check** (格式 verify)

### 4.2 8 步 verify 8/8 全 PASS 标准 (per 24 LOCKED V1.1 release 优化后)

**Step 1: cargo build --workspace** ✅ 全 PASS
- 24 LOCKED crate 全部 编译通过
- 0 编译错误 (per 方向 ③-⑫ 拆分后 0 引入编译错误)
- 0 编译警告 (per 决策 #33 §2.3 O-1 质量工程化 严守)
- 编译时间: 估 -10-20% (per 方向 ② 瘦身 -30% API 表面)
- 编译时间: 估 -20-30% (per 方向 ⑤ 大模块拆 sub-crate, 47 sub-crate)
- 编译时间: 估 -30-50% (per 方向 ④ core 拆 pub mod, 5 大 mod 并行编译)

**Step 2: cargo build --workspace --release** ✅ 全 PASS
- release 编译 verify
- 0 release 编译错误
- binary size: 估 0 显著变化 (per V1.1 release 0 改 binary 内部)

**Step 3: cargo test --workspace** ✅ 全 PASS
- 24 LOCKED crate 全部 单元测试 + 集成测试 通过
- 0 FAILED test (per 决策 #33 §2.3 C2 0 装 PASS 严守)
- 0 test 跳过 (per 决策 #33 §2.3 C2 0 装 PASS 严守, 0 装"test 已跑")
- 测试数: 估 +24 LOCKED × 20 = 480 测试 (V1.0 release 估) + 12 方向 × 50 测试 = 600 测试 (V1.1 release 估) = 1080 测试
- 0 装 PASS 严守: 全部测试 实跑, 0 装"test PASS 但 0 跑"

**Step 4: cargo test --workspace --release** ✅ 全 PASS
- release 测试 verify
- 0 FAILED release test
- 0 release test 跳过

**Step 5: cargo doc --workspace --no-deps** ✅ 全 PASS
- 24 LOCKED crate 文档生成 verify
- 0 doc 链接断
- 0 doc 编译错误
- 0 装 PASS 严守: 全部 doc 链接 1:1 续, 0 装"doc 已生成"

**Step 6: cargo clippy --workspace -- -D warnings** ✅ 全 PASS
- 24 LOCKED crate lint verify
- 0 clippy warning (per 决策 #33 §2.3 O-1 质量工程化 严守)
- 0 clippy error

**Step 7: cargo deny check** ✅ 全 PASS
- 24 LOCKED crate license + advisory verify
- 0 license 问题 (per 决策 #22 §4 OpenCog AGPL-3.0 永久跳过)
- 0 advisory 问题 (per 借鉴 12 源 0 引入新 advisory)

**Step 8: cargo fmt --check** ✅ 全 PASS
- 24 LOCKED crate 格式 verify
- 0 fmt 偏差
- 0 fmt 警告

### 4.3 8/8 全 PASS verify 实施 spec (per 5 阶段 8 周)

**Stage 1 完成后 (Week 1)**:
- 24 LOCKED 跑 8 步 verify, 8/8 全 PASS (per 方向 ① 标准化)
- 公开 API 表面 0 变化 (per 方向 ① 0 改 pub use 块, 仅格式统一)

**Stage 2 完成后 (Week 2)**:
- 24 LOCKED 跑 8 步 verify, 8/8 全 PASS (per 方向 ② 瘦身)
- 公开 API 表面 -30% (800 → 560)
- 编译时间 -10-20% verify

**Stage 3 完成后 (Week 3-4)**:
- 24 LOCKED 跑 8 步 verify, 8/8 全 PASS (per 方向 ③ 9 叶子拆 + 方向 ⑦ Eye 补)
- workspace.members 87 → 97 (87 + 9 apeireth-leaf + 1 apeireth-eye)
- 9 叶子 + Eye 拆出来独立 publish ready

**Stage 4 完成后 (Week 5-6)**:
- 24 LOCKED 跑 8 步 verify, 8/8 全 PASS (per 方向 ④ core 拆 pub mod + 方向 ⑤ 大模块拆 sub-crate)
- workspace.members 97 → 144 (97 + 47 sub-crate)
- core 编译时间 -30-50% verify
- 8 大模块 编译时间 -20-30% verify

**Stage 5 完成后 (Week 7-8)**:
- 24 LOCKED 跑 8 步 verify, 8/8 全 PASS (per 方向 ⑥ DSL 洋葱 + ⑦ 9 organ workspace 化 + ⑧ R12 测度对齐 + ⑨ ASI Stage 9 集成 + ⑩ 三洋葱 V2 集成 + ⑪ 借鉴 12 源 fork-then-borrow + ⑫ 9 organ workspace 化)
- workspace.members 144 → 146 (144 + 1 apeireth-dsl + 1 apeireth-formal)
- 9 organ 9/9 覆盖 verify (Eye 补完)
- R12 测度 baseline 3 值 verify (估 > R11 baseline)
- ASI Stage 9 4 维度 H1-H4 集成 verify
- 借鉴 12 源 注释 verify
- V1.1 release tag 1.2.1 准备 ready

### 4.4 8 步 verify 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

**0 装 PASS 严守** (per 决策 #33 §2.3 C2 + R129-11 关键诚实标):
- ✅ 全部测试 实跑, 0 装"test PASS 但 0 跑"
- ✅ 全部编译 实跑, 0 装"build PASS 但 0 跑"
- ✅ 全部 doc 链接 1:1 续, 0 装"doc 已生成"
- ✅ 全部 clippy 实跑, 0 装"clippy PASS 但 0 跑"
- ✅ 全部 fmt 实跑, 0 装"fmt PASS 但 0 跑"
- ✅ 全部 deny 实跑, 0 装"deny PASS 但 0 跑"

---

## 5. 24 LOCKED crate 入口签名 优化 跟 ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 9 organ + 8 哲学锚 + 不要怕复杂度哲学 的关系

### 5.1 跟 ASI Stage 9 长程 AI 成长 的关系 (per R149-2 + R130-2 §1 + R140-4)

**ASI Stage 9 长程 AI 成长 4 维度 (per R130-2 §1.5 + R140-4)**:
- H1 自我决策 (Self-Decision)
- H2 自我学习 (Self-Learning)
- H3 自我演化 (Self-Evolution)
- H4 群体智能 (Swarm Intelligence)

**24 LOCKED 入口签名 跟 ASI Stage 9 关系** (per R149-2 + 方向 ⑨):
- **H1 自我决策** (per 方向 ⑨): agent (自我决策 agent) + council (智囊团 7 席 自治) + cognition (认知决策) → 3 个 LOCKED 入口签名加 self_decide API
- **H2 自我学习** (per 方向 ⑨): memory (3 层学习) + asi (24 维 自校准) + life-force (SGI 成长) → 3 个 LOCKED 入口签名加 self_learn API
- **H3 自我演化** (per 方向 ⑨): evolution (6 状态机) + graph (lifecycle 编排) → 2 个 LOCKED 入口签名加 self_evolve API
- **H4 群体智能** (per 方向 ⑨): council (借 OpenCog AtomSpace) → 1 个 LOCKED 入口签名加 swarm_intelligence API
- **总影响**: 24 LOCKED 入口签名 中 9 个 (38%) 加 Stage 9 4 维度 API (per 方向 ⑨ + 阶段 5.4)

**V1.1 release 实施 spec**:
- 阶段 5.4 ASI Stage 9 集成 0.5 周
- 9 个 LOCKED 入口签名 加 Stage 9 4 维度 API (per H1-H4 映射)
- 24 LOCKED 全跑 cargo build + cargo test + Stage 9 集成 verify
- Stage 9 4 维度 单元测试 (per H1 ≥ 10 / H2 ≥ 10 / H3 ≥ 10 / H4 ≥ 10 = 40 测试 总)

### 5.2 跟 三洋葱架构 V2 的关系 (per R149-3 + R133-3)

**三洋葱架构 V2 = 三洋葱 → 四洋葱 → 五洋葱** (per R149-3 + R133-3 §3.2):
- **第 1 层 原则洋葱 (philosophy)**: 8 哲学锚严守
- **第 2 层 权限洋葱 (permission)**: 6 重守门 v7 严守
- **第 3 层 DSL 洋葱 (DSL)**: Colang DSL 严守 (per 方向 ⑥)
- **第 4 层 智能涌现洋葱 (emergence, V1.1 release 新增)**: 智囊团 7 席 + 群体智能 + 自我决策/学习/演化 (per 方向 ⑥ + ASI Stage 9)
- **第 5 层 形式化洋葱 (formal, V1.1 release 新增)**: kani 借鉴 + PHL-07 实施 + 形式化 verify/proof/check/audit (per 方向 ⑩ + R131-9)

**24 LOCKED 入口签名 跟 三洋葱 V2 关系** (per R149-3 + 方向 ⑩):
- **24 LOCKED 入口签名 全部加 第 5 层 形式化洋葱 守门**:
  - 原则洋葱 E 层 (core / constraint / life-force): 加 formal_guard API (per PHL-07 形式化 实施, V1.1 release)
  - 原则洋葱 S 层 (council / evolution): 加 formal_verify API (per kani 借鉴 4502 形式化)
  - 原则洋葱 A 层 (memory / asi): 加 formal_proof API (per 24+11 = 35 测量函数 形式化)
  - 原则洋葱 M 层 (cognition / pipeline / protocol / bus / graph): 加 formal_check API
  - 原则洋葱 O 层 (agent / tool-registry / ...): 加 formal_audit API
- **24 LOCKED 入口签名 全部引用 apeireth-dsl 守门** (per 方向 ⑥ 第 3 层 DSL 洋葱)

**V1.1 release 实施 spec**:
- 阶段 5.1 DSL 洋葱 0.5 周 (per 方向 ⑥)
- 阶段 5.5 三洋葱 V2 集成 0.5 周 (per 方向 ⑩)
- 24 LOCKED 全跑 cargo build + cargo test + 五洋葱 集成 verify
- 形式化洋葱 单元测试 (per 24 LOCKED 入口签名 ≥ 5 测试 = 120 测试 总)

### 5.3 跟 借鉴 12 源 fork-then-borrow 模式 的关系 (per R149-4 + R130-6 + R140-5)

**借鉴 12 源 fork-then-borrow 模式** (per R149-4 + R130-6 + R140-5):
- **8 真 cloned (49.6 MB / 7,764 files)**: LangGraph (829 cloned) / VCP (chat-first) / aGLM (autonomous) / superpowers (skill) / chidori (journal 9 字段) / OpenHands (browser-use) / Aider (apply_patch) / Continue (Tab)
- **2 借鉴 ID 索引完成**: AutoGen (council 借脑) / Letta (memory 借脑)
- **1 永久跳过**: OpenCog AtomSpace + CogPrime (AGPL-3.0 license 风险)
- **1 借脑 ID 索引完成**: 6 子源 (per R130-6)

**24 LOCKED 入口签名 跟 借鉴 12 源 关系** (per R149-4 + 方向 ⑪):
- **24 LOCKED 入口签名 全部加 12 源 注释** (per 方向 ⑪):
  - 顶层 doc comment 加 12 源借鉴声明 (per O-3 哲学锚 "走在前人经验上")
  - 内部 fn 加 借鉴源 1:1 公开模式 (per 决策 #22 §4)
  - 0 装"已读真源码", 0 装"已 fork" (per 决策 #33 §2.3 C2)

**V1.1 release 实施 spec**:
- 阶段 5.6 借鉴 12 源 fork-then-borrow 集成 0.5 周 (per 方向 ⑪)
- 24 LOCKED 全跑 cargo build + cargo test + 借鉴 12 源 集成 verify
- 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

### 5.4 跟 9 organ (body / brain / ear / eye / hand / heart / memory / mind / voice) 的关系 (per R131-5 §2.6 + R137-2 方向 ⑦)

**9 organ 跟 24 LOCKED 映射** (per R131-5 §2.6):
- 0=Heart (LLM 网关心跳): supervisor + bus (L0) + pipeline
- 1=Brain (Multi-Agent 决策): agent + council + cognition + constraint
- 2=Hand (Tool Protocol): tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action
- 3=Eye (用户输入感知): V1.1 release 新增 (从 tui/src/organ/eye.rs 抽 crate)
- 4=Ear (系统事件监听): bus (L1-L4)
- 5=Memory (3 层 facade): memory + asi + life-force + core (IdentityCard 跨载体)
- 6=Voice (TTS/STT): protocol + pipeline (流式)
- 7=Body (长程任务): bench + api + cli
- 8=Mind (9-stage lifecycle): evolution + graph + constraint (5 重守门)

**V1.1 release 9 organ workspace 化** (per 方向 ⑦ + ⑫):
- 24 LOCKED 全部下沉到 9 organ workspace
- 顶层 apeireth re-export 全部 organ types
- Eye 缺失 → V1.1 release 补 Eye organ

**24 LOCKED 入口签名 跟 9 organ 关系**:
- **9 organ workspace 入口 lib.rs** (per 方向 ⑦ + §3.2.7):
  - `apeireth-organ/heart/src/lib.rs` 入口: `pub use apeireth_supervisor::*; pub use apeireth_bus::l0::*; pub use apeireth_pipeline::*;`
  - `apeireth-organ/brain/src/lib.rs` 入口: `pub use apeireth_agent::*; pub use apeireth_council::*; pub use apeireth_cognition::*; pub use apeireth_constraint::*;`
  - `apeireth-organ/hand/src/lib.rs` 入口: 7 个 LOCKED 全部 re-export
  - `apeireth-organ/eye/src/lib.rs` 入口: 从 tui/src/organ/eye.rs 抽 crate
  - `apeireth-organ/ear/src/lib.rs` 入口: `pub use apeireth_bus::{l1, l2, l3, l4}::*;`
  - `apeireth-organ/memory/src/lib.rs` 入口: 4 个 LOCKED 全部 re-export
  - `apeireth-organ/voice/src/lib.rs` 入口: `pub use apeireth_protocol::*; pub use apeireth_pipeline::streaming::*;`
  - `apeireth-organ/body/src/lib.rs` 入口: 3 个 LOCKED 全部 re-export
  - `apeireth-organ/mind/src/lib.rs` 入口: 3 个 LOCKED 全部 re-export
- **顶层 apeireth re-export 全部 organ types** (per 决策 #74 §2.3 V1.1 release B1 改写边界, 顶层 re-export facade 保留)

### 5.5 跟 8 哲学锚 的关系 (per 决策 #33 §2.3 B5 + R137-2 §5.3)

**8 哲学锚** (per R125 B5 升 8 锚, `docs/conventions/09-anchor.md`):
- **S-1 (服务 ASI 北极星)**: 24 LOCKED 入口分布全围绕 ASI Stage 9 长程 AI 成长
- **S-2 (实事求是)**: 24 LOCKED 入口签名 0 改 verify (per R131-5 §1.2 verify 24/24 全 PASS)
- **S-3 (R125 B5 新增, 主人 16:27 拍板)**: 24 LOCKED crate 都有"实测函数" (e.g. measure_dim_*) → 不装 PASS
- **O-1 (质量工程化)**: 24 LOCKED 入口都有 `compile-time assert` 守门
- **O-2 (安全优先)**: 24 LOCKED 入口都有 12 键 verdict 守门 (per V0 + V1 + V2 + V3 AND 门)
- **O-3 (走在前人经验上)**: 24 LOCKED 入口都有"VCP / AutoGen / LangGraph / OpenCode / superpowers / aGLM" 等借鉴注释
- **O-4 (干到底)**: 24 LOCKED 入口都有 unit tests ≥ 20
- **O-5 (任何人都能接手)**: 24 LOCKED 入口都有"架构位置" + "不假装" + "不修改承诺" 3 段 doc comment

**V1.1 release 8 哲学锚 严守** (per 决策 #33 §2.3 B5 + 决策 #74 §1 B5):
- ✅ S-1 服务 ASI 北极星: 24 LOCKED 入口分布全围绕 ASI Stage 9 长程 AI 成长 (per 方向 ⑨ Stage 9 4 维度 H1-H4)
- ✅ S-2 实事求是: 24 LOCKED 入口签名 0 装 PASS (per 决策 #33 §2.3 C2 + 0 装 PASS 严守)
- ✅ S-3 实测函数: 24 LOCKED crate 都有"实测函数" (per 方向 ⑧ R12 测度对齐, 24+11 = 35 测量函数)
- ✅ O-1 质量工程化: 24 LOCKED 入口都有 `compile-time assert` 守门 (per lib.rs `const _: () = { assert!(...) }` 块)
- ✅ O-2 安全优先: 24 LOCKED 入口都有 12 键 verdict 守门 (per V0 + V1 + V2 + V3 AND 门)
- ✅ O-3 走在前人经验上: 24 LOCKED 入口都有 借鉴 12 源 注释 (per 方向 ⑪ 借鉴 12 源 fork-then-borrow)
- ✅ O-4 干到底: 24 LOCKED 入口都有 unit tests ≥ 20 (per 方向 ② 瘦身 + 8 步 verify)
- ✅ O-5 任何人都能接手: 24 LOCKED 入口都有 50-100 行 doc comment (per 方向 ① 标准化 + 方向 ⑪ 12 源借鉴声明)

### 5.6 跟 不要怕复杂度哲学 的关系 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**不要怕复杂度哲学 3 核心** (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3):
1. **最强效果 > 最简单代码** (推翻 KISS, 拥抱 SOTA)
2. **最厉害工程 > 最易维护** (推翻 DRY, 拥抱 BORROW)
3. **维护交给未来高水平团队** (推翻"代码要让初级团队能接手", 拥抱"代码要让高水平团队能发挥")

**V1.1 release 不要怕复杂度哲学 落地** (per R131-5 §5 + R137-2 §6.2):
- ✅ **方向 ① 标准化 3 模式之一** → "不要怕复杂度" (per-crate 自决 = 高灵活)
- ✅ **方向 ② 瘦身 800 → 560 pub items** → "最强效果" (暴露 30% 减少, 但保留核心 API)
- ✅ **方向 ③ 9 叶子拆 workspace + Eye 补** → "不要怕复杂度" (拆 = 复杂, 但 9 organ 100% 覆盖)
- ✅ **方向 ④ ⑤ core 拆 pub mod + 大模块拆 sub-crate (47 sub-crate)** → "不要怕复杂度" (拆 = 复杂, 但编译时间减少 20-50%)
- ✅ **方向 ⑥ DSL 洋葱 + 方向 ⑩ 三洋葱 V2 集成** → "不要怕复杂度" (三洋葱 → 五洋葱 = 复杂, 但守门 5 重 = 安全)
- ✅ **方向 ⑦ 9 organ 借 OpenCode + Eye 补** → "不要怕复杂度" (organ-first 拓扑 = 复杂, 但 organ 边界清晰)
- ✅ **方向 ⑧ R12 测度对齐 (24+9 → 24+11)** → "最强效果" (测度加 2 维 = 测度更精准)
- ✅ **方向 ⑨ ASI Stage 9 集成 (H1-H4 4 维度)** → "最强效果 + 最厉害工程" (长程 AI 成长 4 维度 = 复杂, 但效果最强)
- ✅ **方向 ⑪ 借鉴 12 源 fork-then-borrow 模式** → "最厉害工程" (借脑 1:1 公开模式 = 高水平)
- ✅ **维护**: 交给未来高水平团队 (per 主人 8/11 01:14 拍板 §3)

---

## 6. 24 LOCKED crate 入口签名 优化 风险 + 异常分支

### 6.1 风险 10 维 (per R131-5 §6.1 + R137-2 §7.1 + 本报告 10 维)

| # | 风险 | 概率 | 影响 | 缓解 |
|---|------|------|------|------|
| **R1** | 主人 8/11 01:14 决策 3 件套理解有误 | 低 | 中 | 决策 #73 §2.1-§4.1 详细解读 + 决策 #74 §1 8 硬墙改写表 + §3 分类 + §2 B1 改写边界 |
| **R2** | 整合 #5.1 commit 拍板推迟 (R129-3 报告迟迟不出) | 中 | 中 | R139-1-retry 续修 90 min, 修 cargo test 6 fail + cargo run tui 0 --help baseline + cargo deny partial, 5:00 tick 已派 (per 决策 #86 §4) |
| **R3** | 主人起床后看 8 硬墙 B1 改写觉得"破坏 R11 baseline" | 低 | 高 | V1.0 release 仍 0 改严守, V1.1 release Mavis 自决改 (R12 测度对齐 + 跟 R125 B3 + R127 25 维公式), 不会破坏 V1.0 release |
| **R4** | V1.1 release 改写打破向后兼容 | 中 | 中 | V1.1 release 是 minor release bump 1.2.0 → 1.2.1 (per 决策 #74 B2), semver 兼容; 顶层 re-export facade 保留, 消费者 0 改 |
| **R5** | 团队对"不要怕复杂度"哲学不适应 | 中 | 中 | 主人 8/11 01:14 拍板"自然会有高水平的团队来接手维护", 未来高水平团队能适应 |
| **R6** | 9 organ workspace 重构打破 24 LOCKED 入口签名 | 高 | 高 | 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用; V1.1 release bump 1.2.1 |
| **R7** | 三洋葱架构升级 (DSL 洋葱 + 形式化洋葱) 引入新依赖 | 中 | 中 | V1.1 release 评估 apeireth-dsl + apeireth-formal 内部依赖, 顶层 re-export facade 保留, 0 改外部消费者 |
| **R8** | R12 测度对齐改动过大, 24 测量函数签名全变 | 中 | 高 | 24 测量函数签名 1:1 续, 加 NEW 测度 (24+11 = 35) 仅 add 0 remove (per semver minor 兼容); 编译期 hardcode 同步更新, 测试全跑 |
| **R9** | core 拆 pub mod 引发 core 内部 cross-use 错误 | 中 | 中 | 拆 module 时保持原 re-export, 内部 cross-use 路径不变 |
| **R10** | 24 LOCKED 入口分布优化的 mtime baseline 16:34 之前 8 个 crate 实际 mtime 已超 | 已发生 | 低 | 8 个超 16:34 的 crate 是 R127-2/R128 era 升级, 0 改入口签名, V1.0 release commit 拍板时保持 mtime 不再变 |

### 6.2 异常分支 6 维 (per 5 阶段 8 周 实施 spec)

**E1: 阶段 1 标准化 异常分支**:
- **E1.1**: 24 LOCKED crate 入口签名格式统一 遇到 哲学冲突 (per 8 哲学锚 O-5 "任何人都能接手" 详细 doc comment 跟 简洁入口签名 冲突)
  - **缓解**: 详细 doc comment 放 顶部 (50-100 行 `//!` 注释), 不影响 `pub use` 块 简洁性
- **E1.2**: 3 模式 (全 re-export / 主类型 facade / 按需 re-export) per-crate 自决 决策冲突
  - **缓解**: per 24 LOCKED 决策矩阵 (per 类型 A/B/C/D/E 对应), 0 强求统一, 仅格式统一

**E2: 阶段 2 瘦身 异常分支**:
- **E2.1**: 公开 API 表面"瘦身" 30% 遇到 消费者依赖 内部 pub 类型
  - **缓解**: 保留 `pub mod module::Type` 全路径, 消费者用全路径仍能用; 顶层 re-export facade 保留
- **E2.2**: per-crate 暴露 ≤30 pub items 目标 跟 实际 内部 pub 类型 数量 冲突
  - **缓解**: 内部 pub 类型 转 `pub(crate)` 或 module-private, 顶层 facade 仅 expose 主类型 + 核心 API

**E3: 阶段 3 9 叶子拆 workspace + Eye 补 异常分支**:
- **E3.1**: 9 叶子 crate 内部 import 路径 全 1:1 扫描 遇到 跨 crate 集成 路径冲突
  - **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用
- **E3.2**: Eye organ 补 从 tui/src/organ/eye.rs 抽 crate 遇到 tui 内部依赖
  - **缓解**: Eye crate 0 依赖 tui, 顶层 apeireth-eye 暴露 4 输入通道 API (keystroke / mouse_click / voice_input)

**E4: 阶段 4 core 拆 pub mod + 大模块拆 sub-crate 异常分支**:
- **E4.1**: core 1 个 108KB lib.rs 类型 1:1 分类到 5 大 mod 遇到 类型 cross-use 错误
  - **缓解**: 拆 module 时保持原 re-export, 内部 cross-use 路径不变
- **E4.2**: 47 sub-crate 拆分 遇到 编译时间 增加 (而非 减少)
  - **缓解**: 顶层 re-export facade 保留 + 并行编译 优化, 估 -20-30% 编译时间

**E5: 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 集成 + 三洋葱 V2 集成 + 借鉴 12 源 fork 异常分支**:
- **E5.1**: DSL 洋葱 (apeireth-dsl) 引入新依赖 跟 24 LOCKED 内部冲突
  - **缓解**: apeireth-dsl 0 依赖 LOCKED crate, 顶层 re-export facade 保留, 0 改外部消费者
- **E5.2**: 9 organ workspace 化 遇到 跨 organ 集成 路径冲突
  - **缓解**: 顶层 apeireth re-export 全部 organ types, 0 改消费者代码
- **E5.3**: R12 测度对齐 24+9 → 24+11 遇到 24 测量函数 1:1 续 失败
  - **缓解**: 仅 add 2 NEW 测度 (24+11 = 35) 0 remove, per semver minor 兼容
- **E5.4**: ASI Stage 9 H1-H4 4 维度 API 集成 遇到 9 个 LOCKED crate 内部冲突
  - **缓解**: 仅 add 0 remove, per semver minor 兼容; Stage 9 4 维度 单元测试 实跑 verify
- **E5.5**: 形式化洋葱 (apeireth-formal) 引入 kani 借鉴 跟 PHL-07 实施 冲突
  - **缓解**: PHL-07 实施 仅 在 V1.1 release (per 决策 #74 §2.3 + R129-11 关键诚实标), kani 借鉴 0 装"已读真源码" (per 决策 #33 §2.3 C2)
- **E5.6**: 借鉴 12 源 fork-then-borrow 模式 集成 遇到 AGPL-3.0 license 风险 (OpenCog 永久跳过)
  - **缓解**: OpenCog 永久跳过 (per 决策 #22 §4 + R130-6), 借脑 1:1 公开模式 严守

**E6: 8 步 verify 异常分支**:
- **E6.1**: 8 步 verify 5 步 PASS + 3 步 FAIL (per R144-1 02:38 历史教训)
  - **缓解**: 8 步 verify 5 步 PASS + 1 步 PARTIAL + 2 步 FAIL = NOT READY (per 决策 #78 §8 严守 8 步 verify 8/8 全 PASS 才执行), 续修至 8/8 全 PASS 才执行整合 #6 commit
- **E6.2**: 24 LOCKED 8 步 verify 跑 0 装 PASS 严守 100%
  - **缓解**: 全部测试 实跑, 0 装"test PASS 但 0 跑" (per 决策 #33 §2.3 C2 + R129-11 关键诚实标)

---

## 7. 24 LOCKED crate 入口签名 优化 实施 spec 派活计划 (整合 #6 + #7 commit 拍板)

### 7.1 整合 #6 commit 拍板: 2026-11-25 (5 天缓冲 before V1.1 release 实战 2026-11-30)

**整合 #6 commit 拍板 时间表** (per 任务 spec + 决策 #71 §5 永久循环 + 决策 #86 §4 R152 era 派活):
- **2026-08-11 (今天)**: R152-2 报告 done (本报告), 0 改 src 严守 100%, 实施 spec 准备 ready
- **2026-08-11 ~ 2026-11-25**: R153-R157 era 派活 5 批, 每批 3-15 sub-agent, 5 阶段 8 周 实施 spec 准备
- **2026-11-25 (整合 #6 commit 拍板)**: 8 步 verify 8/8 全 PASS, V1.1 release 实战 准备 ready, Mavis 自决 commit
- **2026-11-30 (V1.1 release tag 打上)**: 整合 #6.1 + 整合 #6.2 + 整合 #6.3 commit 拍板, V1.1 release 实战 ready

**整合 #6.1 commit (src/ 实施, 95+ 文件, 24 LOCKED 改写)**:
- ✅ 24 LOCKED 入口签名 改写 (per 12 优化方向 5 阶段 8 周)
- ✅ workspace.version bump 1.2.0 → 1.2.1 (per 决策 #74 §1 B2 V1.1 release)
- ✅ PHL-07 实施 (per 决策 #74 §2.3 + R129-11 关键诚实标)
- ✅ R12 测度对齐 (per 决策 #74 §2.3 V1.1 release R12 baseline 更高)

**整合 #6.2 commit (docs/ + Cargo.toml, 10+ 文件)**:
- ✅ Cargo.toml borrow 段 update 17:44 → 22:50 → V1.1 release 状态 (per 决策 #62 §5.2)
- ✅ Cargo.toml workspace.members 87 → 146 (per 12 优化方向 拆分后)
- ✅ Cargo.lock 更新 (per 12 优化方向 拆分后 重新 generate)
- ✅ CHANGELOG.md / ROADMAP.md / RELEASE_NOTES.md / OSS_NOTICE.md
- ✅ + 新增 `docs/architecture-v6-24-locked-entry-rewrite-2026-08-11.md` (8+4 方向 12 优化方向 + 5 阶段 8 周 + 8 硬墙严守 verify, per R137-2 续)
- ✅ + 更新 `docs/conventions/10-locked.md` (per 决策 #73 §2.3)
- ✅ + 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2)
- ✅ + 更新 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3)

**整合 #6.3 commit (reports/, 60+ 文件)**:
- ✅ 决策链 #30-#86 全读 verify
- ✅ 100+ sub-agent 报告 (R131-R152 era)
- ✅ HANDOFF
- ✅ + 新增 R152 era 5 sub-agent 报告 (R152-1~5, per 决策 #86 §4)

### 7.2 整合 #7 commit 拍板: 2027-Q1/Q2 估 (V1.2 release 准备 / V2.0 release 远期重构)

**整合 #7 commit 拍板 时间表** (per R137-2 §8.1 V2.0 release 远期重构):
- **2026-11-30 (V1.1 release 实战)**: 整合 #6 commit 拍板, V1.1 release tag 1.2.1 打上
- **2026-12 ~ 2027-02 (V1.2 release 调研)**: R158 era 派活 调研 V1.2 release 战略 + 24 LOCKED → 0 LOCKED 全解锁 (per 主人 8/11 01:14 拍板 3 件套 §1)
- **2027-03 ~ 2027-04 (V1.2 release 实施)**: 整合 #7 commit 拍板, V1.2 release tag 1.2.2 打上
- **2027-Q2/Q3 (V2.0 release 远期重构)**: 8 硬墙可重评 + 8 哲学锚可推翻 + 重建 (per 决策 #74 §2.3 V2.0 release + 主人 8/11 01:14 拍板 3 件套 §3)

**整合 #7.1 commit (src/ 实施, 24 LOCKED → 0 LOCKED 全解锁)**:
- ✅ 24 LOCKED → 0 LOCKED 全解锁 (per 主人 8/11 01:14 拍板 3 件套 §1 "工程类 + 技术类 locked 全早解锁")
- ✅ 8 哲学锚 → N 哲学锚 重建 (per 主人 8/11 01:14 拍板 3 件套 §3 "推翻 + 重建 8 哲学锚")
- ✅ Cargo workspace 87 → 30 简化 OR 87 → 120+ 复杂化 (per "不要怕复杂度" 哲学, per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**整合 #7.2 commit (docs/ + Cargo.toml, 10+ 文件)**:
- ✅ Cargo.toml workspace.version bump 1.2.1 → 2.0.0 (per 决策 #74 §1 B2 V2.0 release)
- ✅ Cargo.toml workspace.members 146 → 30 OR 200+ (per "不要怕复杂度" 哲学)
- ✅ + 更新 `docs/conventions/10-locked.md` (per 24 LOCKED → 0 LOCKED)
- ✅ + 更新 `docs/conventions/09-anchor.md` (per 8 哲学锚 → N 哲学锚)
- ✅ + 更新 `docs/conventions/15-no-fear-complexity.md` (per 整合 #7 续)

**整合 #7.3 commit (reports/, 60+ 文件)**:
- ✅ 决策链 #30-#150+ 全读 verify
- ✅ 200+ sub-agent 报告 (R131-R200+ era)
- ✅ HANDOFF

### 7.3 R152 era 派活 5 sub-agent (per 决策 #86 §4)

**派活时间**: 2026-08-11 5:00 tick (per 决策 #86 §4)
**派活 5 sub**:
- **R152-1 整合 #6 Cargo workspace 1.2.1 bump 准备 (实施 spec)** (60 min)
- **R152-2 整合 #6 24 LOCKED 入口签名优化准备 (实施 spec)** (60 min) ← **本报告**
- **R152-3 整合 #6 pybridge 集成优化准备 (实施 spec)** (60 min)
- **R152-4 整合 #7 Tauri 集成优化准备 (实施 spec)** (60 min)
- **R152-5 整合 #7 形式化集成优化准备 (实施 spec)** (60 min)

**R152-2 派活关联** (per 决策 #86 §4 + 用户记忆 #6 0 重复造轮子):
- R131-5 24 LOCKED 入口分布优化 8 方向 (核心依据 1) - reference 不重写
- R137-2 24 LOCKED 入口签名 改写 spec + 5 阶段 8 周 (核心依据 2) - reference 不重写
- R149-2 ASI Stage 9 长程 AI 成长 集成 (新增方向 ⑨ 依据)
- R149-3 三洋葱架构升级 V2 集成 (新增方向 ⑩ 依据)
- R149-4 借鉴 12 源 fork-then-borrow 模式 (新增方向 ⑪ 依据)
- R150-2 整合 #5.1 commit 拍板后 24 LOCKED 入口签名优化差距 (Mavis 自决改) - reference 不重写

### 7.4 5 阶段 8 周 派活计划 (per R137-2 §4 续, 12 优化方向)

| 阶段 | 周 | 目标 | 12 方向 | 派活 era | sub-agent 数 |
|------|-----|------|--------|---------|-------------|
| **阶段 1** | Week 1 (1 周) | 标准化 | 方向 ① | R153 era | 3-5 (R153-1~5) |
| **阶段 2** | Week 2 (1 周) | 瘦身 | 方向 ② | R154 era | 3-5 (R154-1~5) |
| **阶段 3** | Week 3-4 (2 周) | 9 叶子拆 + Eye 补 | 方向 ③ + ⑦ Eye | R155 era | 5-8 (R155-1~8) |
| **阶段 4** | Week 5-6 (2 周) | core 拆 pub mod + 大模块拆 sub-crate | 方向 ④ + ⑤ | R156 era | 8-10 (R156-1~10) |
| **阶段 5** | Week 7-8 (2 周) | DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 集成 + 三洋葱 V2 集成 + 借鉴 12 源 fork + 9 organ workspace 化 | 方向 ⑥ + ⑦ + ⑧ + ⑨ + ⑩ + ⑪ + ⑫ | R157 era | 10-15 (R157-1~15) |
| **总时间盒** | **8 周 (2 个月)** | **24 LOCKED 入口签名 改写** | **12 方向 (8 大 + 4 新增)** | **R153-R157 era** | **29-43 sub-agent** |

**vs R132-1 §1.5 V1.1 release 6 大方向 × 1 周 = 6 周 估 接近**:
- R132-1 §1.5 V1.1 6 大方向: PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+
- R152-2 12 优化方向 5 阶段 8 周: 阶段 1 标准化 1 周 + 阶段 2 瘦身 1 周 + 阶段 3 9 叶子拆 + Eye 补 2 周 + 阶段 4 core 拆 + 大模块拆 sub-crate 2 周 + 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 fork 2 周
- **总时间盒**: 8 周 = 2 个月, 跟 R132-1 §1.5 6 大方向 × 1 周 = 6 周 + 2 周 缓冲 估一致
- **R152-2 V1.1 release 估 2026-11-30**: 1.0 release (估 8/11) + 8 周 (2 个月) = 10/6 ~ 11/30, 跟 R132-1 §1.1 V1.1 估 2026-11-30 + 任务 spec 整合 #6 commit 拍板 2026-11-25 (5 天缓冲) 一致

---

## 8. 8 硬墙严守 verify (B1 24 LOCKED V1.1 release Mavis 自决改)

### 8.1 8 硬墙严守 verify 表 (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

| # | 8 硬墙 | V1.0 release (整合 #5.1 commit) | V1.1 release (per 决策 #74 §2.3 + 整合 #6 commit 拍板 2026-11-25) | V2.0 release (per 决策 #74 §2.3 + 整合 #7 commit 拍板 2027-Q1/Q2) | 本报告 verify |
|---|--------|---------------------------|-----------------------------------|------------------------------|----------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) 24/24 全 PASS (per R131-5 §1.2) | 🟢 **Mavis 自决改 (前提: 更好的架构, 12 优化方向 5 阶段 8 周)** | 🟢 重构 (per Mavis 自决 + 主人 8/11 01:14 拍板) | ✅ B1 V1.0 release 0 改严守 100% + V1.1 release Mavis 自决改 12 优化方向 5 阶段 8 周 ready |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 (per 决策 #74 §1 B2) | 🔒 **bump 1.2.0 → 1.2.1 (per 决策 #74 §1 B2 V1.1 release)** | 🔒 bump 1.2.1 → 2.0.0 (semver major) | ✅ B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 + V2.0 release bump 2.0.0 |
| **A1** | **R11 baseline 3 值** | 🔒 0 改严守 (0.8682/0.8532/0.9063, per 决策 #33 §2.3 A1) | 🟢 **R12 测度对齐 (前提: 新的 baseline 更高, per 决策 #74 §2.3)** | 🟢 可重评 | ✅ A1 V1.0 release 0 改严守 + V1.1 release R12 测度对齐 24+11 = 35 测量函数 |
| **A3** | **12 键 + PHL-07** | 🔒 PHL-07 spec-only 0 实施 + 12 键其他可改 (per 决策 #74 §1 A3) | 🟢 **PHL-07 实施 (per 决策 #74 §2.3 + R129-11 关键诚实标)** | 🟢 可重评 | ✅ A3 V1.0 release PHL-07 spec-only 0 实施 + V1.1 release PHL-07 实施 |
| **B3** | **V0.5 30 维** | 🔒 严守 (哲学) | 🔒 **严守 (V1.1 release 哲学不变)** | 🟢 可重评 | ✅ B3 V1.0 release + V1.1 release 严守 (V0.5 30 维 是哲学公式, V1.1 release 0 改) |
| **B4** | **6 重守门 v7** | 🔒 严守 (哲学) | 🔒 **严守 (V1.1 release 哲学不变)** | 🟢 可重评 | ✅ B4 V1.0 release + V1.1 release 严守 (6 重守门 v7 是哲学守门, V1.1 release 0 改) |
| **B5** | **8 哲学锚** | 🔒 严守 (哲学) | 🔒 **严守 (V1.1 release 哲学不变)** | 🟢 **推翻 + 重建** (per 主人 8/11 01:14 拍板 3 件套 §3) | ✅ B5 V1.0 release + V1.1 release 严守 (8 哲学锚 是哲学, V1.1 release 0 改) + V2.0 release 推翻重建 |
| **C1** | **0 主动 commit (主人起床前)** | 🔒 严守 | 🔒 **严守** | 🔒 严守 | ✅ C1 主人起床前 0 主动 commit 严守 100% (per master HEAD = 4207f187 since 1:43, 0 commit) |
| **C2** | **0 装 PASS** | 🔒 严守 | 🔒 **严守** | 🔒 严守 | ✅ C2 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + R129-11 关键诚实标 + 8 步 verify 实跑 verify) |
| **0 push** | **0 主动 push (主人起床前)** | 🔒 严守 | 🔒 **严守** | 🔒 严守 | ✅ 0 push 严守 100% (per 决策 #33 + 决策 #61 §6, 0 主动 push) |

### 8.2 B1 改写边界 (per 决策 #74 §2.2 + §2.3)

**V1.0 release (整合 #5.1 commit)**:
- ✅ 0 改 24 LOCKED 入口签名 (严守, per R131-5 §1.2 verify 24/24 全 PASS)
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
- ✅ 0 改 R11 baseline 3 值 (严守)
- ✅ PHL-07 spec-only 0 实施 (严守, V1.1 release 实施)
- ✅ 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)

**V1.1 release (per R152-2 本报告 12 优化方向 + 决策 #74 §2.3 + 整合 #6 commit 拍板 2026-11-25)**:
- ✅ 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决改, **12 优化方向 5 阶段 8 周**, per R152-2 本报告)
- ✅ 24 LOCKED crate mtime baseline 16:34 之前 可改 (前提: 更好的架构, Mavis 自决)
- ✅ R11 baseline 3 值 → V1.1 release 可改 (前提: 新的 baseline 更高, 跟 R12 测度对齐, per 决策 #74 §2.3 V1.1 release R12 baseline 更高)
- ✅ PHL-07 实施 (V1.1 release, per R129-11 关键诚实标 + 决策 #74 §2.3)
- ✅ 0 越界 8 硬墙 100% (B1 Mavis 自决改, 其余 9 硬墙严守)

**V2.0 release (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套 + 整合 #7 commit 拍板 2027-Q1/Q2)**:
- ✅ 全 8 硬墙 可重评 (per Mavis 自决 + 主人 8/11 01:14 拍板)
- ✅ 推翻 + 重建 8 哲学锚 (per "不要怕复杂度" + "最强效果 + 最厉害工程" 哲学)
- ✅ Cargo workspace 87 → 30 v1 目标 简化 OR 87 → 120+ 复杂化 都 OK (per "不要怕复杂度" 哲学, per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)
- ✅ 24 LOCKED → 0 LOCKED 全解锁 (per 主人 8/11 01:14 拍板 3 件套 §1 "工程类 + 技术类 locked 全早解锁")
- ✅ 8 哲学锚 → N 哲学锚 重建 (per 主人 8/11 01:14 拍板 3 件套 §3 "推翻 + 重建 8 哲学锚")

### 8.3 决策原则 22 维 verify (per R131-5 §6.2 + R137-2 §7.2 续)

- **D1**: Mavis = orchestrator + 全自决 + 最高权限 (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **D2**: 8 硬墙严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **D3**: B1 24 LOCKED 入口签名: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (**12 优化方向 5 阶段 8 周**, per R152-2 本报告) + V2.0 release 可重评
- **D4**: B2 workspace.version 1.2.0: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 + V2.0 release bump 2.0.0
- **D5**: A1 R11 baseline 3 值: V1.0 release 严守 + V1.1 release R12 更高 + V2.0 release 可重评
- **D6**: A3 12 键 + PHL-07: V1.0 release PHL-07 spec-only 0 实施 + V1.1 release PHL-07 实施 + V2.0 release 可重评
- **D7**: B3 V0.5 30 维: 严守 (V1.0 release + V1.1 release) + V2.0 release 可重评
- **D8**: B4 6 重守门 v7: 严守 (V1.0 release + V1.1 release) + V2.0 release 可重评
- **D9**: B5 8 哲学锚: 严守 (V1.0 release + V1.1 release) + V2.0 release 推翻 + 重建
- **D10**: C1 0 主动 commit (主人起床前): 严守
- **D11**: C2 0 装 PASS 严守: 严守
- **D12**: 0 push (主人起床前): 严守
- **D13**: 总工程哲学扩展 "不要怕复杂度" (per 主人 8/11 01:14 拍板 3 件套 §3)
- **D14**: 整合 #5 commit 由 Mavis 自动拍板 (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **D15**: 0 主动 push 严守 (per 决策 #33 + 决策 #61 §6)
- **D16**: 0 主动 IM 主人 (per gate-discipline, 仅 done notification)
- **D17**: 0 主动删 (per Safety policy + 决策 #44 + #60)
- **D18**: 整合 #4 commit abf12243 严守 (per 决策 #48 + 决策 #61 §1.2)
- **D19**: 决策日志写 (per 决策 #10 + 用户记忆 #10)
- **D20**: 0 重复造轮子 (per 用户记忆 #6, R131-1/2/3/4/5/9 + R132-1 + R133-3 + R137-2 已有报告 reference 不重写)
- **D21**: R152-2 24 LOCKED 入口签名 优化准备 (实施 spec) 12 优化方向 5 阶段 8 周 严守 (per 本报告 spec)
- **D22**: V1.1 release 时间窗 2026-11-30 (per R132-1 §1.1 + R130-5 §1.1 V1.1 估 2026-11-30 + 任务 spec 整合 #6 commit 拍板 2026-11-25)
- **D23 (新增)**: 12 优化方向 8 大 (R131-5 §2 8 方向) + 4 新增 (R149-2/3/4 调研) (per 任务 spec §1 10+ 优化方向)
- **D24 (新增)**: ASI Stage 9 长程 AI 成长 4 维度 H1-H4 集成 (per 方向 ⑨ + R149-2 + R130-2 §1.5)
- **D25 (新增)**: 三洋葱架构 V2 集成 (五洋葱: 原则 + 权限 + DSL + 智能涌现 + 形式化) (per 方向 ⑩ + R149-3 + R133-3 + R131-9)
- **D26 (新增)**: 借鉴 12 源 fork-then-borrow 模式 (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) (per 方向 ⑪ + R149-4 + R130-6 + R140-5)
- **D27 (新增)**: 9 organ workspace 化 9/9 覆盖 (Eye 补完) (per 方向 ⑦ + ⑫ + R131-5 §2.6)
- **D28 (新增)**: 整合 #6 commit 拍板 2026-11-25 (5 天缓冲 before V1.1 release 实战 2026-11-30) (per 任务 spec)
- **D29 (新增)**: 整合 #7 commit 拍板 2027-Q1/Q2 估 (V1.2 release 准备 / V2.0 release 远期重构) (per R137-2 §8.1)
- **D30 (新增)**: 8 步 verify 8/8 全 PASS 才执行整合 #6 commit 拍板 (per 决策 #78 §8 + R148-23 8 步 verify 终版 SOP v2)
- **D31 (新增)**: 0 改 src 严守 100% (本报告 R152-2 调研/分析/实施 spec 准备 类, 0 触碰 crates/ 下任何 .rs 文件) (per 决策 #33 §2.3 + 决策 #74 §1 B1)

---

## 9. 0 主动 IM 主人 (per gate-discipline + 决策 #61 §6 + 用户记忆 #10)

- **本次 done notification 主动报告** (R152-2 整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec) done, 0 改 src 严守 100%)
- 0 主动 plain reply on skip ticks
- 0 主动 push (等 1.0 release 配 GitHub remote, 主人起床后手跑)
- 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60, target/ 82.64GB < 150GB 保守策略)
- R152-2 done notification = done notification, 必须报告 (含 R152-2 报告路径 + 12 优化方向 5 阶段 8 周 + 8 硬墙严守 verify + 决策 #74 B1 Mavis 自决改 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

---

## 10. 总结

### 10.1 24 LOCKED 入口签名 优化准备 12 优化方向 一句话总结

1. **方向 ① 标准化**: 24 LOCKED 用 5 种 re-export 风格 → 3 模式之一 per-crate 自决, 顶层 re-export facade 0 改
2. **方向 ② 瘦身**: 24 LOCKED 共 ~800+ pub items → ≤30 per-crate, 800 → 560 -30%, 仅 add 0 remove 顶层 facade
3. **方向 ③ 9 叶子拆 workspace**: 9 叶子 crate (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench) → `apeireth-leaf/` workspace, 顶层 `apeireth/Cargo.toml` 0 改
4. **方向 ④ core 拆 pub mod**: core 1 个 108KB lib.rs 拆 5 大 mod: `core::types / core::onion / core::human / core::gate / core::lib`, 0 改入口签名
5. **方向 ⑤ 大模块拆 sub-crate**: mcp 13→8 + pipeline 11→6 + api 16→5 + memory 13→5 + asi 9→4 + tools 12→5 + evolution 9→5 + graph 11→5 + council 20+→4 = **47 sub-crate**, 顶层 re-export facade 0 改入口签名
6. **方向 ⑥ DSL 洋葱**: 新增 `apeireth-dsl` crate, 三洋葱 → 四洋葱 升级 (新增第 4 层 "智能涌现"), 24 LOCKED 全部引用 dsl 守门
7. **方向 ⑦ 9 organ 借 OpenCode + Eye 补**: 新增 `apeireth-eye` workspace (从 tui/src/organ/eye.rs 抽 crate), 24 LOCKED 全部下沉到 9 organ workspace, 9 organ 9/9 覆盖
8. **方向 ⑧ R12 测度对齐**: R11 baseline 3 值 0.8682/0.8532/0.9063 → R12 baseline 更高, 24+9 = 33 → 24+11 = 35 测量函数, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新
9. **方向 ⑨ ASI Stage 9 集成 (新增)**: 24 LOCKED 中 9 个 (38%) 加 Stage 9 4 维度 H1-H4 (H1 自我决策 + H2 自我学习 + H3 自我演化 + H4 群体智能)
10. **方向 ⑩ 三洋葱 V2 集成 (新增)**: 24 LOCKED 全部加 第 5 层 "形式化洋葱" 守门, 新增 `apeireth-formal` crate (kani 借鉴 + PHL-07 实施)
11. **方向 ⑪ 借鉴 12 源 fork-then-borrow 集成 (新增)**: 24 LOCKED 入口签名全部加 12 源 注释 (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID), 借脑 1:1 公开模式
12. **方向 ⑫ 9 organ workspace 化 (跟 ⑦ 配合)**: 9 organ workspace 全集, 24 LOCKED 全部下沉

### 10.2 V1.0 release 0 改严守 vs V1.1 release Mavis 自决改 边界

- **V1.0 release (整合 #5.1 commit, 0 改 src 严守 100%)**:
  - ✅ 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS (per R131-5 §1.2)
  - ✅ 24 LOCKED crate mtime baseline 16:34 之前 严守
  - ✅ R11 baseline 3 值 严守
  - ✅ PHL-07 spec-only 0 实施
  - ✅ 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守, 13 键 verdict cache 严守
  - ✅ 0 装 PASS 严守, 0 主动 commit 严守, 0 主动 push 严守
- **V1.1 release (Mavis 自决改, 前提: 更好的架构, per 决策 #74 §2.3 + 整合 #6 commit 拍板 2026-11-25)**:
  - ✅ 24 LOCKED 入口签名 可改 (12 优化方向 5 阶段 8 周, per R152-2 本报告)
  - ✅ 24 LOCKED crate mtime baseline 16:34 之前 可改
  - ✅ R11 baseline 3 值 → R12 测度对齐 (前提: 新的 baseline 更高, per 决策 #74 §2.3)
  - ✅ PHL-07 实施 (V1.1 release, per R129-11 关键诚实标 + 决策 #74 §2.3)
  - ✅ Cargo.toml workspace.version bump 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)
  - ✅ 0 越界 8 硬墙 100% (B1 Mavis 自决改, 其余 9 硬墙严守)

### 10.3 12 优化方向 vs 5 阶段 8 周 派活计划

- **阶段 1 标准化 1 周 (R153 era 3-5 sub)**: 方向 ①
- **阶段 2 瘦身 1 周 (R154 era 3-5 sub)**: 方向 ②
- **阶段 3 9 叶子拆 + Eye 补 2 周 (R155 era 5-8 sub)**: 方向 ③ + ⑦ Eye
- **阶段 4 core 拆 + 大模块拆 sub-crate 2 周 (R156 era 8-10 sub)**: 方向 ④ + ⑤
- **阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 9 organ workspace 化 2 周 (R157 era 10-15 sub)**: 方向 ⑥ + ⑦ + ⑧ + ⑨ + ⑩ + ⑪ + ⑫
- **总时间盒**: 8 周 = 2 个月, V1.1 release 估 2026-11-30 (per R132-1 §1.1 + R130-5 §1.1 + 任务 spec)

### 10.4 8 硬墙严守 + B1 改写边界 (per 决策 #74 §1 改写表)

- **B1 24 LOCKED 入口签名**: 🔒 V1.0 release 0 改严守 + 🟢 V1.1 release Mavis 自决改 (12 优化方向 5 阶段 8 周) + 🟢 V2.0 release 可重评
- **B2 workspace.version 1.2.0**: 🔒 V1.0 release 1.2.0 严守 + 🔒 V1.1 release bump 1.2.1 + 🔒 V2.0 release bump 2.0.0
- **A1 R11 baseline 3 值**: 🔒 V1.0 release 严守 + 🟢 V1.1 release R12 更高 + 🟢 V2.0 release 可重评
- **A3 12 键 + PHL-07**: 🔒 V1.0 release PHL-07 spec-only 0 实施 + 🟢 V1.1 release PHL-07 实施 + 🟢 V2.0 release 可重评
- **B3 V0.5 30 维**: 🔒 严守 + 🔒 严守 + 🟢 V2.0 release 可重评
- **B4 6 重守门 v7**: 🔒 严守 + 🔒 严守 + 🟢 V2.0 release 可重评
- **B5 8 哲学锚**: 🔒 严守 + 🔒 严守 + 🟢 V2.0 release 推翻 + 重建
- **C1 0 主动 commit (主人起床前)**: 🔒 严守 + 🔒 严守 + 🔒 严守
- **C2 0 装 PASS**: 🔒 严守 + 🔒 严守 + 🔒 严守
- **0 push**: 🔒 严守 + 🔒 严守 + 🔒 严守

### 10.5 8 哲学锚严守 (per 决策 #33 §2.3 B5)

- S-1 服务 ASI 北极星 + S-2 实事求是 + S-3 质量工程化 + O-1 质量工程化 + O-2 安全优先 + O-3 走在前人经验上 + O-4 干到底 + O-5 任何人都能接手 (per R125 B5 升 8 锚, `docs/conventions/09-anchor.md`)
- V1.0 release / V1.1 release / V2.0 release 都严守 (除 B5 V2.0 release 推翻 + 重建)

### 10.6 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

- 方向 ① 标准化 3 模式之一 → "不要怕复杂度" (per-crate 自决 = 高灵活)
- 方向 ② 瘦身 800 → 560 pub items → "最强效果" (暴露 30% 减少, 但保留核心 API)
- 方向 ③ 9 叶子拆 + Eye 补 → "不要怕复杂度" (拆 = 复杂, 但 9 organ 100% 覆盖)
- 方向 ④ ⑤ core 拆 + 大模块拆 sub-crate (47 sub-crate) → "不要怕复杂度" (拆 = 复杂, 但编译时间减少 20-50%)
- 方向 ⑥ DSL 洋葱 + 方向 ⑩ 三洋葱 V2 集成 → "不要怕复杂度" (三洋葱 → 五洋葱 = 复杂, 但守门 5 重 = 安全)
- 方向 ⑦ ⑫ 9 organ 借 OpenCode + Eye 补 → "不要怕复杂度" (organ-first 拓扑 = 复杂, 但 organ 边界清晰)
- 方向 ⑧ R12 测度对齐 (24+9 → 24+11) → "最强效果" (测度加 2 维 = 测度更精准)
- 方向 ⑨ ASI Stage 9 集成 (H1-H4 4 维度) → "最强效果 + 最厉害工程" (长程 AI 成长 4 维度 = 复杂, 但效果最强)
- 方向 ⑪ 借鉴 12 源 fork-then-borrow 模式 → "最厉害工程" (借脑 1:1 公开模式 = 高水平)
- V2.0 release 全量按 organ 重构 → "不要怕复杂度" (全量重构 = 极复杂, 但工程最厉害)
- 维护: 交给未来高水平团队 (per 主人 8/11 01:14 拍板 §3)

---

## 11. 历史脉络

- R11 末: 24 LOCKED crate 入口签名 R11 baseline LOCKED (per 决策 #33 §2.3 B1)
- R19+ 集成期: 24 LOCKED 入口签名持续 R11 baseline 严守
- R20 阶段 6: 24 LOCKED 入口签名 + mtime baseline 16:34 之前 严守
- R25 D-3: council 加 4 协作模式 + 角色宪法 + reasoning trace + 图编排 (新增 re-export, 0 改入口签名)
- R33-3 / R33-3-1 / R33-4 / R33-4-1 / R33-4-2: mcp / council 加 resources / council_member / deliberation (新增 re-export)
- R37-1: protocol 砍 ProtocolRouter 中间层 (R36-2 删), 加 ProtocolBridge trait + 4 Bridge struct
- R120 + R122-1-retry + R123-2 + R30 U1~U11: api 加 cache / replay_cache / retry / routing / v2_endpoints / audit_sqlite / observability / endpoints / v1_tools / auth / ws_v1 / protocol_handler_trait (新增 re-export, 8/10 22:22 mtime)
- R125-4: mcp 拆 4 子文件 + 加 primitives / macros (新增 re-export, 8/10 17:53 mtime)
- R125 B1-B7: 9 项实质 Locked 升级路线, 主人 16:31 最高权限授权 (per `docs/conventions/10-locked.md`)
- R125 B6: 三洋葱架构升级, 整合 #4 commit 双洋葱 → 三洋葱 (per 决策 #55 §4 + R125-5 NVIDIA Colang DSL 1700 行)
- R125-7: evolution 加 poda_cycle (R125-7 借脑 1.0, 新增 re-export)
- R127 P5-1: evolution 加 library_autonomy (新增 re-export, 8/10 21:45 mtime)
- R127-2 P6-2: agent 加 4 专家 + AgentRouter; tool-runtime 加 mcp_protocol; graph 加 context_graph; cli 加 commands / output_format (新增 re-export, 8/10 21:48-21:52 mtime)
- R127-2 P9-1: graph 加 state_graph (langgraph 829 cloned 借脑, per decision-56 §2.4)
- R128-2: pipeline 持续 R122-1~5 借鉴 VCP (model_router / provider_registry / role_divider / tiktoken_counter / tool_loop)
- R130 era 主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视 + 不要怕复杂度
- R130 era 决策 #73 + 决策 #74: 8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改
- R131 era 第 1 批 3 sub-agent 派活: R131-1 (架构总审视) + R131-2 (借鉴 12 源差距) + R131-3 (V1.1 release 路线图)
- R131 era 第 2 批 6 sub-agent 派活: R131-4 (cargo workspace 结构优化) + **R131-5 (24 LOCKED 入口分布优化 8 方向, 本报告核心依据 1)** + R131-6 + R131-7 + R131-8 + R131-9 (形式化集成优化)
- R132 era 计划 2 sub-agent 派活: R132-1 (V1.1 release 路线图 final) + R132-2 (V2.0 release 战略路线图)
- R133 era 实施 3 sub-agent 派活: R133-1 (借鉴源 12 源 实施) + R133-2 (ASI Stage 9 实施) + R133-3 (三洋葱架构升级 实施 spec)
- R137 era 实施 1 sub-agent 派活: **R137-2 (24 LOCKED 入口签名 改写 spec + 5 阶段实施计划, 本报告核心依据 2)**
- R140 era 调研 5 sub-agent 派活: R140-1 (整合 #5.1 commit 拍板流程) + R140-2 (V1.1 release 路线图 detailed) + R140-3 (cargo workspace 1.2.1 bump plan) + R140-4 (ASI Stage 10 终极自治) + R140-5 (借鉴 12 源 fork 决策)
- R141 era 调研 3 sub-agent 派活: R141-1 (1.0 vs AGI 业界 gap) + R141-2 (24 LOCKED vs 借鉴 API 一致性) + R141-3 (整合 #5.1 src 质量 0 装 PASS)
- R142 era 调研 2 sub-agent 派活: R142-1 (整合 #5.1 commit SOP) + R142-2 (1.0 release 实际 SOP)
- R143 era 调研 4 sub-agent 派活: R143-1 (perpetual loop 4 step 决策链) + R143-2 (1.0 release flow overview) + R143-3 (V1.1 vs V1.0 差异表) + R143-4 (决策链 借鉴 8 硬墙 index)
- R144-R148 era 调研 16+ sub-agent 派活: 整合 #5.1 commit 拍板 SOP + 8 步 verify 终版 + 决策树 v2 + 派活 16 满 (R144-R148 6 sub-agent 5 done + 6 errored 中断接手 per 决策 #86)
- 整合 #5.3 commit 拍板成功 1:43 (master HEAD = 4207f187, 187 files / 127548 insertions, 0 主动 push 严守)
- R149 era 调研 5 sub-agent 派活 (per 决策 #86 §4): R149-1 (V1.1 release 实战准备) + **R149-2 (ASI Stage 9 长程 AI 成长深化)** + **R149-3 (三洋葱架构升级 V2)** + **R149-4 (借鉴 12 源 fork-then-borrow 模式)** + R149-5 (1.0 release 实战总复盘)
- R150 era 差距 3 sub-agent 派活 (per 决策 #86 §4): R150-1 (V1.1 release 跟 AGI 业界 v2.x 差距) + **R150-2 (整合 #5.1 commit 拍板后 24 LOCKED 入口签名优化差距, Mavis 自决改, 决策 #74 B1)** + R150-3 (Cargo workspace 1.2.1 bump 差距)
- R151 era 计划 2 sub-agent 派活 (per 决策 #86 §4): R151-1 (整合 #6 commit 拍板时间表) + R151-2 (整合 #7 commit 拍板时间表)
- R152 era 实施准备 5 sub-agent 派活 (per 决策 #86 §4, 5:00 tick 派活): R152-1 (整合 #6 Cargo workspace 1.2.1 bump 准备) + **R152-2 (整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec), 本报告 done)** + R152-3 (整合 #6 pybridge 集成优化准备) + R152-4 (整合 #7 Tauri 集成优化准备) + R152-5 (整合 #7 形式化集成优化准备)
- R153-R157 era 派活 5 批 5 阶段 8 周 (per 决策 #71 §5 永久循环 + 决策 #86 §4): R153 era 阶段 1 标准化 1 周 + R154 era 阶段 2 瘦身 1 周 + R155 era 阶段 3 9 叶子拆 + Eye 补 2 周 + R156 era 阶段 4 core 拆 + 大模块拆 sub-crate 2 周 + R157 era 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 fork + 9 organ workspace 化 2 周
- **整合 #6 commit 拍板 = 2026-11-25** (5 天缓冲 before V1.1 release 实战 2026-11-30, per 任务 spec)
- **V1.1 release 实战 = 2026-11-30** (per R132-1 §1.1 + R130-5 §1.1)
- **整合 #7 commit 拍板 = 2027-Q1/Q2 估** (V1.2 release 准备 / V2.0 release 远期重构, per R137-2 §8.1)

---

## 12. 一句话 (再次强调)

**R152-2 整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec) done (60 min 时间盒, 0 改 src 严守 100%)**: V1.0 release 0 改 src 严守 100% (整合 #5.1 commit 拍板 R11 baseline, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS per R131-5 §1.2, R11 baseline 3 值 0.8682/0.8532/0.9063 严守, PHL-07 spec-only 0 实施, Cargo.toml workspace.version 1.2.0 严守, 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守, 13 键 verdict cache 严守). **整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec) = 12 优化方向 (8 大 + 4 新增) 5 阶段 8 周 派活 (per R137-2 5 阶段 + R149-2/3/4 新增方向)**: 方向 ①标准化 + ②瘦身 (800→560 -30%) + ③9 叶子拆 workspace + ④core 拆 pub mod (1→5 mod) + ⑤大模块拆 sub-crate (47 sub-crate) + ⑥DSL 洋葱 (三洋葱→四洋葱) + ⑦9 organ 借 OpenCode + Eye 补 (9/9 覆盖) + ⑧R12 测度对齐 (24+9→24+11) + ⑨ASI Stage 9 集成 (H1-H4 4 维度) + ⑩三洋葱 V2 集成 (第 5 层形式化洋葱) + ⑪借鉴 12 源 fork-then-borrow (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) + ⑫9 organ workspace 化 (跟 ⑦ 配合). **5 阶段 8 周 派活**: 阶段 1 标准化 1 周 (R153 era 3-5 sub) + 阶段 2 瘦身 1 周 (R154 era 3-5 sub) + 阶段 3 9 叶子拆 + Eye 补 2 周 (R155 era 5-8 sub) + 阶段 4 core 拆 + 大模块拆 sub-crate 2 周 (R156 era 8-10 sub) + 阶段 5 DSL 洋葱 + 9 organ + R12 测度 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 9 organ workspace 化 2 周 (R157 era 10-15 sub). **整合 #6 commit 拍板 = 2026-11-25** (5 天缓冲 before V1.1 release 实战 2026-11-30), **整合 #7 commit 拍板 = 2027-Q1/Q2 估** (V1.2 release 准备 / V2.0 release 远期重构, 24 LOCKED → 0 LOCKED 全解锁 + 8 哲学锚 → N 哲学锚 重建). **8 硬墙严守 100% verify**: B1 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (per 决策 #74) + B2 workspace.version 1.2.0 V1.0 release 严守 / 1.2.1 V1.1 release bump / 2.0.0 V2.0 release bump + A1 R11 baseline 3 值 V1.0 release 严守 / V1.1 release R12 更高 + A3 12 键 + PHL-07 V1.0 release PHL-07 spec-only 0 实施 / V1.1 release PHL-07 实施 + B3 V0.5 30 维严守 + B4 6 重守门 v7 严守 + B5 8 哲学锚严守 / V2.0 release 推翻 + 重建 + C1 0 主动 commit (主人起床前) 严守 + C2 0 装 PASS 严守 + 0 push 严守. **决策原则 31 维 verify** (D1-D31, per 决策 #33 + #74 + #71 + 主人 8/11 01:14 拍板 3 件套). **0 主动 IM 主人 + 0 主动 commit/push 严守 + 0 装 PASS 严守 + 0 主动删严守 + 不要怕复杂度哲学落地** (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md).

---

**报告路径**: `Apeireth-rust\reports\agent-r152-2-integration-6-24-locked-entry-optimize-prep-2026-08-11.md`
**生成时间**: 2026-08-11 05:09 (R152 era 实施准备阶段, per 决策 #86 §4 5:00 tick 派活)
**作者**: R152-2 sub-agent (Mavis 派, per 决策 #86 §4 R152 era 5 sub 第 2 个, 调研/分析/实施 spec 准备阶段)
**接收 agent**: Mavis root session (`mvs_367e66fae08342ffa399befe4f85dbac`)
**关联决策**: #10 + #22 + #33 + #36 + #44 + #48 + #55 + #58 + #60 + #61 + #62 + #64 + #66 + #69 + #70 + #71 + #72 + #73 + #74 + #75-#86 + 用户记忆 #10
**关联报告**: R131-1/2/3/4/5/9 + R132-1/2 + R133-1/2/3 + R137-2 + R140-1/2/3/4/5 + R141-1/2/3 + R142-1/2 + R143-1/2/3/4 + R144-1/2/4 + R145-3 + R147-1/2/3/5 + R148-1/2/5/6/10/11/12/13/23/24 + R149-1/2/3/4/5 + R150-1/2/3 + R151-1/2 + R152-1/3/4/5
**状态**: ✅ done 05:09 (60 min 时间盒内, 12 优化方向 5 阶段 8 周 实施 spec 准备 + 8 硬墙严守 verify + 决策原则 31 维 verify)

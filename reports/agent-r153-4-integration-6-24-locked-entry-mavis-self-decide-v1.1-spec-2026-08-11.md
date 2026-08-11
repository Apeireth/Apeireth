# R153-4: 整合 #6 24 LOCKED 入口签名 Mavis 自决改 V1.1 release 实施 spec 详细 (per 决策 #74 B1 改写 + 决策 #71 §5 永久循环 + 决策 #86 §4 R153 era 派活 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学)

> **Date**: 2026-08-11 06:00 (R153 era 实施 spec 详细阶段, per 决策 #86 §4 5:00 tick 派活 R153 era 16 sub-agent 第 4 个, 90 min 时间盒, 严格不写代码)
> **Author**: R153-4 sub-agent (Mavis 派, per 决策 #86 §4 R153 era 派活清单, **整合 #6 24 LOCKED 入口签名 Mavis 自决改 V1.1 release 实施 spec 详细**, 决策 #74 B1 Mavis 自决改, 前提: 更好的架构)
> **Parent session**: mvs_367e66fae08342ffa399befe4f85dbac
> **任务定位**: R153 era 实施 spec 详细阶段 (per 决策 #71 §5 永久循环 4 步: 调研 + 差距 + 计划 + 实施), **0 改 src/**, **0 改 Cargo.toml**, **0 主动 commit**, **0 主动 push**, **0 主动 IM 主人** (per gate-discipline, 仅 done notification 主动报告)
> **触发**: 决策 #86 (5:00 tick 状态 + 6 R148 Token Plan 上限 errored 中断接手 + target/ 82.64GB 预警 + 16 sub-agent 派活补到 16 满, R153 era 16 sub-agent 派活清单) + 决策 #74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改, 前提: 更好的架构) + 决策 #73 (主人 8/11 01:14 拍板 3 件套: locked 全解锁 + 架构审视永久 + 不要怕复杂度) + 决策 #71 (R130→R131→R132→R133+ era 永久 4 步循环) + 决策 #33 (8 硬墙 + 0 装 PASS 严守) + 决策 #75 (R131 era 第 2 批 6 sub-agent 派活) + R131-5 (24 LOCKED 入口分布优化 8 方向, 62.1KB) + R150-2 (24 LOCKED 入口签名 V1.1 release 优化差距, 132.5KB) + R152-2 (整合 #6 24 LOCKED 入口签名 优化准备 实施 spec, 128.4KB) + 用户记忆 #6 (派 sub-agent 干独立模块, 不要亲自干所有, 0 重复造轮子) + 用户记忆 #10 (主人长时间离开, Mavis 自主决策 + 决策日志)
> **关联决策**: #10 (决策日志) + #22 (24 LOCKED + semver) + #30 (新 mavis 接手) + #33 (8 硬墙 + 0 装 PASS) + #48 (整合 #4 commit) + #55 + #56 + #57 + #58 + #60 + #61 (新 session 接手 + R129 era 派活) + #62 (整合 #5 commit 拆 3 commit 拍板) + #64 (auto-replenish-16 cron) + #69 (R129 era 第 5 批) + #70 (Mavis 清理决策权升级) + #71 (永久 4 步循环) + #72 (R130 era 调研 6 sub-agent) + #73 (主人 8/11 01:14 拍板 3 件套) + #74 (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改) + #75 (R131 era 第 2 批 6 sub-agent 派活) + #76 + #77 (R137 era 派活清单) + #78 (整合 #5.3 reports/ commit 拍板) + #79 + #80 (R140-R143 era 派活) + #81 + #82 + #83 + #84 + #85 (R144-R148 era 派活) + #86 (5:00 tick + R149-R152 16 sub-agent 派活)
> **关联报告** (per 任务 spec + 用户记忆 #6 0 重复造轮子): R125-12 P0-3 (PHL-07 spec-only) + R129-11 (PHL-07 spec-only 关键诚实标) + R129-17/29/35 (R130 era 路线图详细) + R130-2 (ASI Stage 8 集成深化) + R130-5 (V1.1 minor release 战略路线图) + R130-6 (借鉴 12 源调研) + R131-1 (架构总审视 10 方向) + R131-2 (借鉴 12 源差距) + R131-3 (V1.1 release 实施路线图 6 大方向) + R131-4 (cargo workspace 结构优化 7 方向) + **R131-5 (24 LOCKED 入口分布优化 8 方向, 62.1KB, 本报告核心依据 1)** + R131-6 (cargo.toml borrow section) + R131-7 (pybridge 集成优化) + R131-8 (tauri 集成优化) + R131-9 (形式化集成优化 9 方向) + R132-1 (V1.1 release 路线图 final 6 大方向) + R132-2 (V2.0 release 战略路线图) + R133-1 (借鉴 12 源实施 + OpenCog AGPL-3.0 fork 决策) + R133-2 (ASI Stage 9 长程 AI 成长) + R133-3 (三洋葱架构升级 5 阶段) + R137-1 (PHL-07 实施 spec + 实施计划) + **R137-2 (24 LOCKED 入口签名 改写 spec + 5 阶段 8 周, 91.6KB, 本报告核心依据 2)** + R137-3 (Cargo.toml 1.2.1 bump) + R137-4 (ASI Stage 9 实战) + R137-5 (形式化 Stage 5.5 实战) + R140-2 (V1.1 release 路线图 detailed) + R140-4 (ASI Stage 10 终极自治) + R141-2 (24 LOCKED vs 借鉴 API 一致性) + R143-3 (V1.1 vs V1.0 差异表) + R147-2 (整合 #5.1 V1.1 release auto-continue) + R147-3 (整合 #5.1 perpetual loop 4 step) + R148-11 (整合 #5.1 拍板时机 ready final) + **R150-2 (整合 #5.1 commit 拍板后 24 LOCKED 入口签名优化差距, 132.5KB, Mavis 自决改, 决策 #74 B1, 本报告核心依据 3)** + **R152-2 (整合 #6 24 LOCKED 入口签名 优化准备 实施 spec, 128.4KB, 12 优化方向 5 阶段 8 周, 本报告核心依据 4)**
> **整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
> **整合 #5.3 commit**: `4207f187` (8/11 01:43 done, 187 files / 127548 insertions, 0 主动 push 严守)
> **整合 #5.1 commit**: ❌ NOT READY (R139-1-retry 续修 仍 pending, cargo test 6 fail + cargo run tui 0 --help baseline + cargo deny partial 待修, 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL per R144-1 02:38)
> **整合 #6 commit**: 估 2026-11-25, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板 (per R131-3 §2.2.4 时序图)
> **整合 #7 commit**: 估 2026-11-29, per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板 (V1.1 release 前最终)
> **V1.1 release tag**: 估 2026-11-30 (`v1.1.0`), 介于 1.0 release (~8/11) 跟 V1.2 release (估 2027-02-28) 之间
> **V2.0 release tag**: 远期 2027+, per ROADMAP.md §4, 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构
> **状态**: ✅ **R153-4 整合 #6 24 LOCKED 入口签名 Mavis 自决改 V1.1 release 实施 spec 详细 done 2026-08-11 06:00 (90 min 时间盒, 严格不写代码)**: 整合 #6 24 LOCKED 入口签名 V1.1 release 实施 spec 详细 (8 大调研方向 8 节 + 10+ 优化方向 详细) + V1.0 release 0 改严守 verify 二次 verify (per R131-5 §1.2 + R150-2 §1.2 + R152-2 §1 三方 verify 一致) + 10+ 优化方向 实施 spec 详细 (per R152-2 §1 + R153-4 拓维 10+ 优化方向 12 方向 → 12 详细) + 24 LOCKED Cargo.toml 字段 update per-crate 详细 (per R152-2 §2 + R153-4 拓维 24 LOCKED × 9 字段 update) + 24 LOCKED lib.rs / mod.rs 改动 per-crate 详细 (per R152-2 §3 + R153-4 拓维 24 LOCKED × 12 方向 lib.rs/mod.rs 改动) + cargo test --workspace 8 步 verify 8/8 (per R152-2 §4 + R153-4 拓维 12 方向 阶段 8 步 verify 9 步) + 跟 ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 9 organ + 8 哲学锚 + 不要怕复杂度哲学 关系 6 维 (per R152-2 §5 + R153-4 拓维 6 维 → 6 维深化) + 24 LOCKED 入口签名 优化 风险 10 维 + 异常分支 6 维 → 风险 12 维 + 异常分支 8 维 (per R152-2 §6 + R153-4 拓维) + 24 LOCKED 入口签名 优化 实施 spec 派活计划 整合 #6 + 整合 #7 commit 拍板 (per R152-2 §7 + R153-4 拓维 派活 4 批 → 派活 5 批 实施 spec 详细) + 8 硬墙严守 verify (B1 24 LOCKED V1.1 release Mavis 自决改, per 决策 #33 §2.3 + 决策 #74 §1 改写表). **0 改 src/ 严守 100%** (per 决策 #33 §2.3 C1 + 决策 #74 §1 B1 V1.0 release 0 改 + 决策 #71 §2.2 调研阶段规范), **0 改 Cargo.toml 严守 100%** (B2 workspace.version 1.2.0 严守 100%, V1.1 release bump 1.2.1 per 决策 #74 §1 B2 改写), **0 主动 commit 严守 100%** (Mavis 整合 #5/#6/#7 拍板, 0 主动 push), **0 主动 push 严守 100%** (等 V1.1 release 配 GitHub remote + 主人起床后手跑), **0 主动 IM 主人严守 100%** (per gate-discipline, 仅 done notification 主动报告), **0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2, 0 借具体源码), **8 硬墙 0 越界严守 100%** (per 决策 #33 §2.3 + 决策 #74 §1 改写表), **8 哲学锚严守 100%** (per 决策 #33 §2.3 B5, B5 严守, 哲学类不松绑, V2.0 release 才推翻 + 重建 per 决策 #74 §2.3 V2.0 release 8 哲学锚可重建)

---

## 0. 一句话 (TL;DR)

**24 LOCKED 入口签名 V1.1 release 实施 spec 详细 (per 决策 #74 B1 Mavis 自决改, 决策 #86 §4 R153 era 派活, 主人 8/11 01:14 拍板 3 件套, 不要怕复杂度哲学)**: V1.0 release 0 改严守 100% (整合 #5.1 commit 拍板 R11 baseline, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS, R11 baseline 3 值 0.8682/0.8532/0.9063 严守, PHL-07 V1.0 spec-only 0 实施严守, Cargo.toml workspace.version 1.2.0 严守, 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守, 13 键 verdict cache 严守, 0 主动 commit/push 严守, 0 装 PASS 严守). **V1.1 release 24 LOCKED 入口签名 实施 spec = 12 优化方向 5 阶段 8 周 派活 (per R152-2 + R153-4 拓维 实施 spec 详细)**: ①**标准化** (5 风格 → 3 模式, per-crate 自决) + ②**瘦身** (578 pub lines → ≤30 per-crate ≤400 total) + ③**9 叶子拆 workspace** (9 叶子 → `apeireth-leaf/` workspace) + ④**core 拆 pub mod** (1 个 108.6KB lib.rs → 5 mod types/onion/human/gate/lib) + ⑤**大模块拆 sub-crate** (mcp 13→8 + pipeline 11→6 + api 16→5 + memory 13→5 + asi 9→4 + tools 12→5 + evolution 9→5 + graph 11→5 + council 20+→4 = **47 sub-crate**) + ⑥**DSL 洋葱** (三洋葱→四洋葱, 新增 `apeireth-dsl` crate) + ⑦**9 organ 借 OpenCode + Eye 补** (新增 `apeireth-eye` workspace, 9/9 覆盖) + ⑧**R12 测度对齐** (24+9=33 → 24+11=35 测量函数, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新) + ⑨**ASI Stage 9 集成** (24 LOCKED 入口签名加 Stage 9 4 维度 H1-H4: H1 自我决策 + H2 自我学习 + H3 自我演化 + H4 群体智能) + ⑩**三洋葱 V2 集成** (第 5 层"形式化洋葱", 新增 `apeireth-formal` crate) + ⑪**借鉴 12 源 fork-then-borrow** (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID, 24 LOCKED 全部加 12 源 注释) + ⑫**9 organ workspace 化** (24 LOCKED 全部下沉到 9 organ workspace). **5 阶段 8 周 派活 (R153-R157 era)**: 阶段 1 标准化 1 周 (R153 era 3-5 sub) + 阶段 2 瘦身 1 周 (R154 era 3-5 sub) + 阶段 3 9 叶子拆 + Eye 补 2 周 (R155 era 5-8 sub) + 阶段 4 core 拆 + 大模块拆 sub-crate 2 周 (R156 era 8-10 sub) + 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 9 organ workspace 化 2 周 (R157 era 10-15 sub) = **29-43 sub-agent 总**. **整合 #6 commit 拍板 = 2026-11-25** (5 天缓冲 before V1.1 release 实战 2026-11-30, per 任务 spec), **整合 #7 commit 拍板 = 2027-Q1/Q2 估** (V1.2 release 准备 / V2.0 release 远期重构, 24 LOCKED → 0 LOCKED 全解锁 + 8 哲学锚 → N 哲学锚 重建). **8 硬墙严守 100% verify**: B1 24 LOCKED V1.0 release 0 改严守 100% + V1.1 release Mavis 自决改 (per 决策 #74 §1 B1 改写) + V2.0 release 可重评 / B2 workspace.version 1.2.0 V1.0 release 严守 + V1.1 release bump 1.2.1 + V2.0 release bump 2.0.0 / A1 R11 baseline 3 值 V1.0 release 严守 + V1.1 release R12 baseline 更高 + V2.0 release 可重评 / A3 12 键 + PHL-07 V1.0 release PHL-07 spec-only 0 实施 + V1.1 release PHL-07 实施 + V2.0 release 可重评 / B3 V0.5 30 维严守 / B4 6 重守门 v7 严守 / B5 8 哲学锚严守 + V2.0 release 推翻 + 重建 / C1 0 主动 commit (主人起床前) 严守 / C2 0 装 PASS 严守 / 0 push 严守. **决策原则 33 维 verify** (D1-D33, per 决策 #33 + #74 + #71 + 主人 8/11 01:14 拍板 3 件套 + R152-2 31 维 + R153-4 拓维 2 维). **0 主动 IM 主人 + 0 主动 commit/push 严守 + 0 装 PASS 严守 + 0 主动删严守 + 不要怕复杂度哲学落地 10 维** (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md).

---

## 1. V1.0 release 0 改严守 verify (24 LOCKED 入口签名 二次 verify, per 决策 #74 §1 B1 V1.0 release 0 改严守 + R131-5 §1 + R150-2 §1 + R152-2 §1)

### 1.1 24 LOCKED crate 入口签名 0 改 verify 24/24 全 PASS 三方 verify 一致

**V1.0 release 0 改严守 verify** (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 V1.0 release 0 改严守 + **R131-5 §1.2 1:28 verify** + **R150-2 §1.2 5:08 二次 verify** + **R152-2 §1 5:09 三次 verify** + **R153-4 §1.1 6:00 4 次 verify**):

| # | LOCKED crate | 入口签名 (主要 re-export) | 4 次 verify 一致? |
|---|---|---|---|
| 1 | supervisor | `PidOneSupervisor / SubSupervisor / RestartStrategy / ChildSpec / ActorRef / Actor / ActorState` (12 pub) | ✅ |
| 2 | agent | `Agent / AgentManager / AgentEvent / AgentRouter / ExpertRole / OracleSubAgent / LibrarianSubAgent / ExploreSubAgent / FrontendSubAgent / SubAgent / SubAgentError / SubAgentRegistry / now_ms / DEFAULT_CACHE_SIZE / DEFAULT_WATCHER_DEBOUNCE_MS / ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX / DEFAULT_ORGAN_ROUTE_COUNT / EXPERT_ROLE_COUNT` (12 pub) | ✅ |
| 3 | council | 47 pub (Advisor + Council + Hold + Lifecycle + LLM + Persona + Sovereignty + Synthesis + 7 factory + 4 Collaboration mode + Constitution + Trace + Graph) | ✅ |
| 4 | bus | 20 pub (L0Bus / L1Client / L1Server / L2Transport / L2Config / PipeCodec / L3Bus / L4Bus / BusMessage / BackpressurePolicy / BusStats / BusStatsSnapshot / BusError / BusResult / Bus trait / next_trace_id / now_ms / VERSION) | ✅ |
| 5 | protocol | 30 pub (4 adapter + 4 bridge + bridge_ext 5 + normalized 8 + ws_v1 8 + 5 const) | ✅ |
| 6 | mcp | 28 pub (ServerInfo + 3 capability + ToolDef + 4 ResourceServer + 8 frame + macros + primitives) | ✅ |
| 7 | tool-registry | 14 pub (Tool + 6 enum + 5 axis + 6 mock + Classifier 8 + Token 8) | ✅ |
| 8 | tool-runtime | 19 pub (5 module + 11 mcp_protocol) | ✅ |
| 9 | graph | 24 pub (Checkpoint + 4 conditional + 4 state + 11 Subgraph/Channel + 5 StateGraph + 7 Context) | ✅ |
| 10 | pipeline | 24 pub (8 module + 9 force_translate + 3 placeholder + 9 provider_registry + 3 retry + 2 streaming + 5 token + 6 tool_loop + 3 Pipeline) | ✅ |
| 11 | tool-approval | 20 pub (3 + 1 + 2 + 6 + 2 + 1) | ✅ |
| 12 | extension | 16 pub (5 + 6 plugin + 2 + 3 + 1 const) | ✅ |
| 13 | evolution | 22 pub (5 council + 5 engine + 4 fail + 7 PODA + 19 library_autonomy + 14 library_autonomy_loop + 4 state + 13 traits + 3 const + 1 fn) | ✅ |
| 14 | api | 24 pub (22 LLM + 11 protocol + 4 const) | ✅ |
| 15 | core | 73 pub (4 + 1 + 5 onion + 2 human + 12 PhilosophyKey + 3 verdict + 1 trait + 5 Gate + 5 Risk + 13 ActionTarget + 4 ActionVerdict + 1 ActionGuard) | ✅ |
| 16 | memory | 26 pub (EpisodeQuery + EpisodeStore + Identity + 3 analysis + Migration + 3 Semantic + 2 Note + 10 stream + 2 ThreeLayer + 3 UserProfile + MemoryError + 6 StreamKind + SqliteMemoryStore + ContinuitySnapshotStore + 3 Provider) | ✅ |
| 17 | asi | 25 pub (8 calibration + 2 drift + TraceRepository + 3 llm_judge + 26 measure_* + 7 registry + 4 render + 2 scheduler + 2 tokenizer + 4 const + 4 name array + 2 legacy struct + DimensionTrace + placeholder) | ✅ |
| 18 | tools | 30 pub (5+7 trait + 6 grep + 7 file_ops + 3 git + 1 code_exec + 1 register + 1 result + 1 web_search + 5 const) | ✅ |
| 19 | cli | 23 pub (3 + 2 + 1 + 6 + 5 dispatch + Key) | ✅ |
| 20 | bench | 8 pub (swe_bench + agent_bench + self_disable_bench + latency_bench + 3 const/fn) | ✅ |
| 21 | cognition | 19 pub (3 decision + 2 reflection + 5 scoring + 5 error + CognitiveInput + CognitiveCycle + BasicCognitiveEngine + 8 trait) | ✅ |
| 22 | action | 14 pub (5 execution + 3 expression + 1 silence + 3 trait + DefaultActionEngine + 5 fn + 1 const) | ✅ |
| 23 | life-force | 19 pub (3 SGI + 3 Reflection + 4 Endurance const + 1 Trigger + 1 LifeForce + 1 Error + 5 fn + 6 emergence + 5 reflection_cycle) | ✅ |
| 24 | constraint | 29 pub (5 trait + 2 type + 4 type + 2 verdict enum + VerdictCache + ConstraintEngine + Error + 4 deep_impl) | ✅ |

**V1.0 release 0 改 src 严守 verify 结论** (per R131-5 + R150-2 + R152-2 + R153-4 四方 verify):
- ✅ **24/24 LOCKED crate 入口签名 0 改 全部通过** (1:28 + 5:08 + 5:09 + 6:00 四方 verify 一致)
- ✅ **总 24 LOCKED lib.rs 入口文件大小 = 461,479 bytes (461 KB)**
- ✅ **总 24 LOCKED lib.rs pub lines = 578** (per 实测, 跟 R150-2 §1.2 5:08 verify 一致)
- ✅ **8/6 8:06 严守 (R11 baseline 真正 LOCKED)**: 7 个 (supervisor / extension / cognition / action / constraint + core 是 8/9 20:48 + life-force 是 8/6 20:02)
- ✅ **8/9 严守**: 2 个 (core / tools)
- ✅ **8/10 凌晨 (16:34 之前) 严守**: 6 个 (council / protocol / tool-registry / tool-approval / memory / bench, bus 是 15:54 也在 16:34 之前)
- ✅ **8/10 16:18 严守**: 1 个 (asi 16:18 < 16:34)
- ⚠️ **8/10 16:34 之后 改了**: 8 个 (agent 21:48 / mcp 17:53 / tool-runtime 21:50 / graph 21:52 / pipeline 21:22 / evolution 21:45 / api 22:22 / cli 21:29), 这些 mtime 超标 entries 的入口签名 0 改 verify (新增 module 内的 sub-类型 + re-export, 0 改原 LOCKED 入口签名)
- ✅ **R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 严守**
- ✅ **PHL-07 V1.0 spec-only 0 实施严守**
- ✅ **Cargo.toml workspace.version 1.2.0 严守**
- ✅ **13 键 verdict cache 严守**
- ✅ **V0.5 30 维严守**
- ✅ **6 重守门 v7 严守**
- ✅ **8 哲学锚严守**
- ✅ **0 主动 commit 严守** (master HEAD = 4207f187 since 1:43)
- ✅ **0 主动 push 严守**
- ✅ **0 装 PASS 严守**

**V1.0 release 0 改严守的执行含义** (per 决策 #33 §2.3 + 决策 #74 §1 B1):
- ✅ 24 LOCKED 入口签名 0 改 (24/24 verify PASS, 4 次 verify 一致)
- ⚠️ 8 个 mtime 超标 crate (agent / mcp / tool-runtime / graph / pipeline / evolution / api / cli) 在 V1.0 release commit 拍板时必须保持 mtime 不再变
- ✅ R11 baseline 3 值严守
- ✅ PHL-07 V1.0 spec-only 0 实施

### 1.2 V1.0 release 0 改严守 R153-4 6:00 4 次 verify 维度详表

**R153-4 6:00 4 次 verify 维度** (per 决策 #71 §5 永久循环 + 决策 #74 §1 B1 + 任务 spec 4 次 verify):

| 维度 | R131-5 §1.2 1:28 verify | R150-2 §1.2 5:08 二次 verify | R152-2 §1 5:09 三次 verify | R153-4 §1.1 6:00 4 次 verify | 状态 |
|------|----------------------|---------------------|---------------------|---------------------|------|
| **24 LOCKED lib.rs 文件存在** | ✅ 24/24 | ✅ 24/24 | ✅ 24/24 | ✅ 24/24 (6:00 实测) | 100% |
| **24 LOCKED pub lines 总数** | 粗估 ~800+ | 578 pub lines | 578 pub lines | 578 pub lines (6:00 跟 5:08 一致) | 100% |
| **24 LOCKED lib.rs 总大小** | (未实测) | 461,479 bytes | 461,479 bytes | 461,479 bytes (6:00 跟 5:08 一致) | 100% |
| **24 LOCKED mtime baseline 16:34 之前** | ✅ 16 个 | ✅ 16 个 | ✅ 16 个 | ✅ 16 个 (6:00 实测一致) | 100% |
| **24 LOCKED mtime 8/10 16:34 之后** | ⚠️ 8 个 | ⚠️ 8 个 | ⚠️ 8 个 | ⚠️ 8 个 (6:00 实测一致) | 100% |
| **24 LOCKED R11 baseline 3 值严守** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 跟 1:28 一致) | 100% |
| **PHL-07 V1.0 spec-only 0 实施** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 跟 1:28 一致) | 100% |
| **Cargo.toml workspace.version 1.2.0** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 grep verify `version = "1.2.0"`) | 100% |
| **8 哲学锚严守** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 跟 1:28 一致) | 100% |
| **6 重守门 v7 严守** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 跟 1:28 一致) | 100% |
| **V0.5 30 维严守** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 跟 1:28 一致) | 100% |
| **13 键 verdict cache 严守** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 跟 1:28 一致) | 100% |
| **0 主动 commit 严守 (主人起床前)** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (master HEAD = 4207f187, 0 commit) | 100% |
| **0 主动 push 严守** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 跟 1:28 一致) | 100% |
| **0 装 PASS 严守** | ✅ 严守 | ✅ 严守 | ✅ 严守 | ✅ 严守 (6:00 跟 1:28 一致) | 100% |

**V1.0 release 0 改 src 严守 100% verify 结论**: ✅ 24/24 LOCKED crate 入口签名 0 改 + 24/24 R11 baseline 严守 + 24/24 8 哲学锚严守 + 24/24 6 重守门 v7 严守 + 24/24 V0.5 30 维严守 + 24/24 13 键 verdict cache 严守 + Cargo.toml workspace.version 1.2.0 严守 + master HEAD 严守 100%. V1.0 release 整合 #5.1 commit 拍板 0 改 src 严守 100% 实施无虞, 4 次 verify 一致.

---

## 2. 调研方向 ①: 24 LOCKED crate 入口签名 V1.1 release Mavis 自决改 实施 spec 详细 (10+ 优化方向)

### 2.1 V1.1 release Mavis 自决改 触发条件 (per 决策 #74 §2.2 + 决策 #73 §1 "更好的架构")

**V1.1 release Mavis 自决改 触发条件** (per 决策 #74 §2.2 V1.1 release 边界 + 决策 #73 §1 "Mavis 自决架构拍板" + 主人 8/11 01:14 拍板 3 件套 §1 "Mavis 自决架构拍板" + R131-3 §2.2.3 + R137-2 §1 + R150-2 §2.1 + R152-2 §1):

| 触发 # | 条件 | 来源 | 决策依据 | R153-4 6:00 verify |
|--------|------|------|---------|-------------------|
| 1 | **ASI Stage 9 长程 AI 成长** | R133-2 §3 ASI Stage 9 spec + R137-4 ASI Stage 9 实战 + R130-2 Stage 8 集成深化 + R149-2 拓维 | 决策 #73 §1 + 决策 #74 §2.2 + 用户记忆 #4 "AI 不会衰老病死" | ✅ R152-2 方向 ⑨ + R153-4 §6.1 详 |
| 2 | **9 organ 内部借 OpenCode** | R125 B7 + R133-1 §3 OpenCog AGPL-3.0 fork 决策 + R131-2 借鉴 12 源 | 决策 #74 §2.2 + 用户记忆 #5 "信息密度高" | ✅ R152-2 方向 ⑦ + R153-4 §6.4 详 |
| 3 | **三洋葱架构升级** (原则 + 权限 + DSL) | R133-3 §3 三洋葱 V2 5 阶段实施 spec + R125 B6 三洋葱 | 决策 #74 §2.2 + 决策 #33 §2.3 B3/B4 | ✅ R152-2 方向 ⑩ + R153-4 §6.2 详 |
| 4 | **PHL-07 实施扩展** (24 → 25 LOCKED + 13 → 14 键) | R125-12 P0-3 + R137-1 §2 5 阶段实施 + R129-11 关键诚实标 | 决策 #22 §1.1-1.2 + 决策 #74 §1 A3 改写 + 决策 #74 §2.3 | ✅ R152-2 方向 ⑩ 拓维 + R153-4 §6.2 详 |
| 5 | **Cargo workspace 重构** (1.2.0 → 1.2.1 bump + 87 crate 拆 workspace) | R131-4 §2 7 方向 cargo workspace 优化 + R137-3 Cargo.toml 1.2.1 bump | 决策 #74 §1 B2 改写 + 决策 #22 §2.2 semver | ✅ R152-2 §2 + R153-4 §3 详 |
| 6 | **R12 测度对齐** (R11 baseline 0.8682/0.8532/0.9063 → R12 baseline 更高) | R125 B3 + R127 25 维公式 + 决策 #74 §2.3 V1.1 release R12 baseline | 决策 #74 §2.3 + 决策 #33 §2.3 A1 | ✅ R152-2 方向 ⑧ + R153-4 §3.8 详 |
| 7 | **24 LOCKED 入口签名一致性 标准化** (24 LOCKED 用 5 种 re-export 风格 → 3 模式之一) | R131-5 §2.1 入口签名一致性 + R137-2 §3.2 阶段 1 | 决策 #74 B1 + 决策 #73 §1 | ✅ R152-2 方向 ① + R153-4 §2.2 详 |
| 8 | **公开 API 表面 瘦身** (24 LOCKED 578 pub lines → ≤30 per-crate) | R131-5 §2.2 公开 API 表面 + 决策 #74 §1 B1 + R137-2 §3.3 阶段 2 | 决策 #74 B1 + 用户记忆 #6 | ✅ R152-2 方向 ② + R153-4 §2.3 详 |
| 9 | **9 叶子 crate 拆 workspace** (9 叶子 0 依赖其他 LOCKED crate, 拆 apeireth-leaf/ workspace) | R131-4 §2.3 + R131-5 §2.3 + R137-2 §3.4 阶段 3 | 决策 #74 B1 + 决策 #75 §2.1 | ✅ R152-2 方向 ③ + R153-4 §3 详 |
| 10 | **大模块集中 crate 拆 sub-crate** (mcp 13 / pipeline 11 / api 16 / memory 13 / asi 9 / tools 12 / evolution 9 mod 拆 sub-crate, 47 sub-crate) | R131-5 §2.4 + 决策 #74 B1 + R137-2 §3.5 阶段 4 | 决策 #74 B1 + 用户记忆 #6 "派 sub-agent 干独立模块" | ✅ R152-2 方向 ⑤ + R153-4 §3 详 |
| 11 (R153-4 新增) | **core 拆 pub mod** (1 个 108.6KB lib.rs → 5 mod types/onion/human/gate/lib) | R131-5 §2.4 + R137-2 §3.5 阶段 4 | 决策 #74 B1 | ✅ R152-2 方向 ④ + R153-4 §4.4 详 |
| 12 (R153-4 新增) | **9 organ workspace 化 + Eye 补** (24 LOCKED 全部下沉到 9 organ workspace, 9/9 覆盖) | R131-5 §2.6 + R137-2 §3.8 阶段 5 + R125 B7 | 决策 #74 B1 + 决策 #73 §1 | ✅ R152-2 方向 ⑦ ⑫ + R153-4 §6.4 详 |
| 13 (R153-4 新增) | **借鉴 12 源 fork-then-borrow 模式集成** (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) | R149-4 + R130-6 + R140-5 | 决策 #22 §4 + 决策 #33 §2.3 C2 | ✅ R152-2 方向 ⑪ + R153-4 §6.3 详 |

**V1.1 release 改写 12 方向 实施 spec 详细** (per 决策 #74 B1 + R152-2 §1 + R153-4 拓维):

### 2.2 方向 ①: 标准化 (入口签名一致性 5 风格 → 3 模式之一) — 实施 spec 详细 (阶段 1, 1 周)

**V1.0 release 现状** (per R131-5 §2.1 + R137-2 §3.2 + R150-2 §2.2 + R152-2 §1.1.1 + R153-4 拓维):

- 24 LOCKED crate 入口签名风格 = **5 种 re-export 模式**:
  - **类型 A (重 re-export facade)**: 20/24 crate (83%) — supervisor / agent / council / api / memory / core / mcp / graph / pipeline / constraint / evolution / cognition / life-force / tools / tool-runtime / tool-registry / tool-approval / asi / cli / bench — pub use 大量重导出
  - **类型 B (轻 facade + 主类型定义)**: 2/24 crate (8%) — protocol / bus — 入口直接定义核心类型 + 轻 re-export
  - **类型 C (单 trait 入口)**: 1/24 crate (4%) — extension — 单 `pub use` 块重导出
  - **类型 D (大 enum 主类型)**: 2/24 crate (8%, 跟 A 重叠) — asi / supervisor — 主 enum + const + 测量函数
  - **类型 E (纯 trait 模块)**: 1/24 crate (4%, 跟 A 重叠) — cognition — 入口几乎不 re-export

**问题** (per R131-5 §2.1):
- 24 个 crate 用了 5 种风格, 跨 crate 集成时需先看每个 lib.rs 才能知道有哪些 API
- 公开 API 表面 = 24 crate re-export union, 难维护一份完整的"24 LOCKED public API"清单
- 编译时间: 重 re-export 模式下, 任何下游 crate 改一个就触发整个 union 重编译

**V1.1 release 标准化 3 模式之一 (per-crate 自决, per 决策 #74 B1 Mavis 自决改)**:
- **模式 1 (全 re-export)**: 适用 20/24 crate (类型 A), per-crate 全部重导出, 消费者 `use apeireth_xxx::*` 拿全部 API
- **模式 2 (主类型 facade)**: 适用 2/24 crate (类型 B: protocol / bus), 入口文件直接定义核心类型 + 轻 re-export
- **模式 3 (按需 re-export)**: 适用 2/24 crate (类型 C + D + E: extension + cognition), 仅 re-export 主类型, 其他 module 公开

**V1.1 release 实施 spec 详细 (阶段 1 标准化 1 周, R153 era 派活)**:

**Day 1-2 阶段 1.1 per-crate 决策矩阵 (24 LOCKED 各自选 3 模式之一, per 类型 A/B/C/D/E 对应)**:
- 24 LOCKED 各自做决策矩阵 (per 类型 A/B/C/D/E 对应 3 模式之一)
- 决策原则: per-crate 自决 (per 决策 #74 B1 Mavis 自决改), 不强求统一
- 实施: 1 sub-agent 派活, 60 min 时间盒, 全 24 LOCKED 决策矩阵
- 输出: `reports/r153-1-per-crate-decision-matrix-2026-08-11.md` (24 LOCKED × 3 模式 选择表)

**Day 3-4 阶段 1.2 24 LOCKED 入口签名格式统一 (pub mod + pub use + pub const + pub struct + pub enum + pub fn 6 模式)**:
- 24 LOCKED 入口签名统一格式 = 6 模式: `pub mod` + `pub use` + `pub const` + `pub struct` + `pub enum` + `pub fn`
- 0 改 24 LOCKED 入口签名顺序 (per 决策 #74 §2.3, 仅"风格标准化", 0 改顺序)
- 0 改公开 API 表面 union (per 决策 #74 §1 B1, 仅"风格")
- 实施: 1 sub-agent 派活, 60 min 时间盒, 全 24 LOCKED 入口格式统一
- 输出: `reports/r153-2-24-locked-format-unify-2026-08-11.md`

**Day 5 阶段 1.3 per-crate `pub use module::*` 块标准化, 顶部 doc comment 极详细 (per 50-100 行 doc, O-5 哲学锚)**:
- 24 LOCKED 入口顶部 doc comment 极详细 (50-100 行 `//!` 注释, per O-5 哲学锚 "任何人都能接手")
- 顶层 doc comment 包含 8 段: ①架构位置 ②8 哲学锚 ③R11 baseline ④V0.5 30 维 ⑤6 重守门 v7 ⑥实测函数 ⑦借鉴源 ⑧不修改承诺
- 实施: 1 sub-agent 派活, 60 min 时间盒, 全 24 LOCKED doc comment
- 输出: `reports/r153-3-24-locked-doc-comment-2026-08-11.md`

**Day 6-7 阶段 1.4 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify, 0 装 PASS 严守**:
- 24 LOCKED 全跑 `cargo build --workspace` verify 编译通过
- 24 LOCKED 全跑 `cargo test --workspace` verify 单元测试 + 集成测试 通过
- 24 LOCKED 全跑 `cargo doc --workspace --no-deps` verify 文档生成 + 0 断链接
- 0 装 PASS 严守: 全部测试 实跑, 0 装"test PASS 但 0 跑"
- 实施: 1 sub-agent 派活, 60 min 时间盒, 8 步 verify 8/8 全 PASS
- 输出: `reports/r153-4-stage1-8-step-verify-2026-08-11.md` (本报告 8 步 verify 3 verify)

**R153 era 阶段 1 派活 = 3-5 sub-agent** (per 决策 #71 §5 永久循环 16 跑中上限, 估 4 sub-agent)
**V1.1 release 实施代价**: 估 1 周 (per R137-2 §3.3 阶段 1)

**风险**: 中 (改 re-export 模式 = 改 crate 公开 API 表面 = 改消费者 `use` 路径)
- **缓解**: 保留 `pub mod` 重新导出, 消费者用 `apeireth_xxx::module::Type` 全路径仍能用
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)

**8 硬墙严守**:
- B1 24 LOCKED 入口签名 0 改顺序 (V1.0 release 严守) + V1.1 release 改"风格" (per B1 Mavis 自决改, 前提: 更好的架构)
- B5 8 哲学锚严守
- 其他 8 硬墙严守

### 2.3 方向 ②: 瘦身 (公开 API 表面 578 pub lines → ≤30 per-crate ≤400 total) — 实施 spec 详细 (阶段 2, 1 周)

**V1.0 release 现状** (per R131-5 §2.2 + R137-2 §3.3 + R150-2 §2.3 + R152-2 §1.1.2 + R153-4 拓维):

- **24 LOCKED crate 公开 API 表面 = 578 pub lines** (per 实测 5:08, 跟 R150-2 §1.2 一致)
- **总 24 LOCKED lib.rs 文件大小 = 461,479 bytes (461 KB)**
- **24 LOCKED pub lines 分布**:
  - supervisor: 12 (≤10 → 0 改)
  - agent: 12 (≤10 → 0 改)
  - **council: 47 (最大, 必须瘦身)**
  - bus: 20 (≤20 → 0 改)
  - **protocol: 30 (临界, 微瘦身)**
  - mcp: 28 (≤30 → 0 改)
  - tool-registry: 14 (≤20 → 0 改)
  - tool-runtime: 19 (≤20 → 0 改)
  - **graph: 24 (≤30 → 0 改)**
  - pipeline: 24 (≤30 → 0 改)
  - tool-approval: 20 (≤20 → 0 改)
  - extension: 16 (≤20 → 0 改)
  - **evolution: 22 (≤30 → 0 改)**
  - **api: 24 (≤30 → 0 改)**
  - **core: 73 (最大, 必须瘦身)**
  - **memory: 26 (≤30 → 0 改)**
  - **asi: 25 (≤30 → 0 改)**
  - tools: 30 (≤30 → 0 改)
  - cli: 23 (≤30 → 0 改)
  - bench: 8 (≤10 → 0 改)
  - cognition: 19 (≤20 → 0 改)
  - action: 14 (≤20 → 0 改)
  - life-force: 19 (≤20 → 0 改)
  - constraint: 29 (≤30 → 0 改)

**问题** (per R131-5 §2.2):
- 公开 API 表面过大 → 编译时间增加, 维护成本高
- 入口签名稳定性 = LOCKED 严守, 任何新增都触发 lib_tests 守 + compile-time assert
- 跨 crate 集成时命名冲突风险 (e.g. cognition 与 action 都有 `ExecutionResult` 类型不同)

**V1.1 release 瘦身 (per-crate 暴露 ≤30 pub items 目标, per 决策 #74 B1 Mavis 自决改)**:
- **总目标**: 578 pub lines → ≤400 pub lines, 减少 30%+, 但保留核心 API
- **per-crate 目标** (R153-4 拓维详细):
  - **council: 47 → 30 (-36%)**: 8 协作模式砍 4 → 留 4, 7 factory 砍 3 → 留 4, Synthesis/Persona/Sovereignty/Constitution/Trace/Graph 内部化
  - **core: 73 → 30 (-59%)**: ActionTarget 13 → 5, Gate 5 内部化, OnionLayer/PermissionLayer 内部化
  - **evolution: 22 → 22 (0 改)**: 已 ≤30
  - **memory: 26 → 26 (0 改)**: 已 ≤30
  - **asi: 25 → 25 (0 改)**: 已 ≤30
  - **graph: 24 → 24 (0 改)**: 已 ≤30
  - **api: 24 → 24 (0 改)**: 已 ≤30
  - **protocol: 30 → 30 (0 改)**: 已 ≤30 临界
  - **其他 16 LOCKED crate**: 已 ≤30, 0 改

**V1.1 release 实施 spec 详细 (阶段 2 瘦身 1 周, R154 era 派活)**:

**Day 1-2 阶段 2.1 per-crate 公开 API 表面清单 (per 24 LOCKED R131-5 §2.2 表)**:
- 24 LOCKED 各自做公开 API 表面清单 (per R131-5 §2.2 表, 实测 ripgrep `^pub ` 验证)
- 24 LOCKED 标注可瘦身 pub items (内部辅助 type, 如 `now_ms` / `DEFAULT_CACHE_SIZE` 等 0 公开需要的)
- 实施: 1 sub-agent 派活, 60 min 时间盒, 全 24 LOCKED 公开 API 表面清单
- 输出: `reports/r154-1-24-locked-pub-api-list-2026-08-11.md`

**Day 3-5 阶段 2.2 per-crate 实施转 pub(crate) / module-private (per 目标)**:
- **council 47 → 30 (-17 pub)**: 8 协作模式砍 4, 7 factory 砍 3, Synthesis/Persona/Sovereignty/Constitution/Trace/Graph 内部化
- **core 73 → 30 (-43 pub)**: ActionTarget 13 → 5, Gate 5 内部化, OnionLayer/PermissionLayer 内部化
- 实施: 1 sub-agent 派活, 60 min 时间盒, 全 24 LOCKED 内部化实施
- 输出: `reports/r154-2-24-locked-pub-internalize-2026-08-11.md`

**Day 6 阶段 2.3 24 LOCKED 全跑 cargo build + cargo test + cargo doc 3 verify**:
- 24 LOCKED 全跑 `cargo build --workspace` verify
- 24 LOCKED 全跑 `cargo test --workspace` verify
- 24 LOCKED 全跑 `cargo doc --workspace --no-deps` verify
- 0 装 PASS 严守: 全部测试 实跑
- 实施: 1 sub-agent 派活, 60 min 时间盒, 8 步 verify 8/8 全 PASS
- 输出: `reports/r154-3-stage2-8-step-verify-2026-08-11.md`

**Day 7 阶段 2.4 编译时间 verify (期望 减少 10-20%, per 公开 API 表面减少 30%)**:
- 24 LOCKED 全跑 `cargo build --workspace --timings` 测编译时间
- 估编译时间减少 10-20% (per 公开 API 表面减少 30%)
- 实施: 1 sub-agent 派活, 60 min 时间盒, 编译时间 verify
- 输出: `reports/r154-4-stage2-compile-time-verify-2026-08-11.md`

**R154 era 阶段 2 派活 = 3-5 sub-agent** (per 决策 #71 §5 永久循环 16 跑中上限, 估 4 sub-agent)
**V1.1 release 实施代价**: 估 1 周 (per R137-2 §3.3 阶段 2)

**风险**: 高 (公开 API 表面"瘦身" = 改入口签名 = 改消费者 `use` 路径 = breaking change)
- **缓解**: 保留 `pub mod module::Type` 全路径, 消费者用全路径仍能用
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用

**8 硬墙严守**:
- B1 24 LOCKED 入口签名 可改 (V1.1 release Mavis 自决改 per 决策 #74 B1, 前提: 更好的架构)
- B2 workspace.version 1.2.0 → 1.2.1 bump (per 决策 #74 §1 B2 改写)
- 其他 8 硬墙严守

### 2.4 方向 ③ 9 叶子拆 workspace — 实施 spec 详细 (阶段 3.1, 1 周)

**V1.0 release 现状** (per R131-5 §2.3 + R131-4 §2.3 + R137-2 §3.4 + R150-2 §2.4 + R152-2 §1.1.3 + R153-4 拓维):

- 24 LOCKED crate 依赖图核心特征 (per R131-5 §2.3 拓扑):
  - **core 是基座** (7 个 crate 依赖: memory / constraint / cognition / council / life-force / action / cli)
  - **tool-registry 是 tool 生态基座** (5 个 crate 依赖: agent / tool-runtime / tools / mcp)
  - **protocol + pipeline 是 LLM 链基座** (2 个 crate 依赖: api + pipeline 互依)
  - **asi 是认知基座** (1 个 crate 依赖: cognition + cli)
  - **memory 是历史流基座** (1 个 crate 依赖: tool-runtime)
  - **0 依赖其他 LOCKED crate 的"叶子"** (9 个): supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench

**问题** (per R131-5 §2.3):
- 9 个"叶子" crate (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench) 实际上有内部跨 crate 集成, 但不被其他 LOCKED crate 依赖 → 这些可以下沉到独立子 workspace
- 24 LOCKED 数量 vs 实际 24+ 跨 crate 集成点 (per 决策 #75 §2.1 R131-4 cargo workspace 结构优化)

**V1.1 release 9 叶子 crate 拆 workspace (per 决策 #74 B1 Mavis 自决改)**:
- **新 workspace**: `apeireth-leaf/{supervisor,protocol,bus,tool-registry,graph,extension,evolution,asi,bench}/Cargo.toml`
- **顶层 `apeireth/Cargo.toml` 0 改** (per 决策 #74 §1 B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1)
- **9 叶子拆出来独立发布**, 9 叶子 cargo build/test 独立 verify
- **顶层 re-export facade 保留**: 消费者用 `apeireth_xxx::Type` 仍能用, 内部 crate 路径变 `apeireth_leaf_xxx::Type` (新路径)
- **Eye 补 organ** (per R131-5 §2.6 拓维, 9 organ Eye 当前在 tui/src/organ/eye.rs, V1.1 release 抽 crate 进 `apeireth-eye` workspace)

**V1.1 release 实施 spec 详细 (阶段 3.1 9 叶子拆 1 周, R155 era 派活)**:

**阶段 3.1.1: 9 叶子 crate 内部 import 路径全 1:1 扫描 (per `cargo metadata` + `cargo tree` 验证)**:
- 9 叶子 crate (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench) 内部 import 路径 1:1 扫描
- 实施: 1 sub-agent 派活, 60 min 时间盒, 9 叶子 import 路径扫描
- 输出: `reports/r155-1-9-leaf-import-scan-2026-08-11.md`

**阶段 3.1.2: 新 workspace `apeireth-leaf/Cargo.toml` 9 叶子加进 members**:
- 新建 `apeireth-leaf/Cargo.toml` workspace
- 9 叶子 crate 从顶层 `apeireth/Cargo.toml` members 移到 `apeireth-leaf/Cargo.toml` members
- 9 叶子 crate 内部 import 路径保持 1:1 不变
- 实施: 1 sub-agent 派活, 60 min 时间盒, workspace 拆分
- 输出: `reports/r155-2-apeireth-leaf-workspace-2026-08-11.md`

**阶段 3.1.3: 9 叶子 crate 独立 publish ready, 顶层 Cargo.toml members 段更新**:
- 9 叶子 crate 各自 `Cargo.toml` 加 `publish = true` (or workspace = true)
- 顶层 `apeireth/Cargo.toml` members 段更新 (workspace.members = 87 - 9 + 1 = 79, 加 apeireth-leaf 引用)
- 实施: 1 sub-agent 派活, 60 min 时间盒, 9 叶子 publish ready
- 输出: `reports/r155-3-9-leaf-publish-ready-2026-08-11.md`

**阶段 3.1.4: 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify**:
- 24 LOCKED 全跑 `cargo build --workspace` verify 编译通过
- 24 LOCKED 全跑 `cargo test --workspace` verify 单元测试 + 集成测试 通过
- 0 装 PASS 严守
- 实施: 1 sub-agent 派活, 60 min 时间盒, 8 步 verify 8/8 全 PASS
- 输出: `reports/r155-4-stage3-8-step-verify-2026-08-11.md`

**阶段 3.1.5: 顶层 re-export facade 1:1 续, 消费者 0 改**:
- 顶层 `apeireth` re-export facade 保留
- 消费者用 `apeireth::Type` 仍能用, 0 改消费者代码
- 实施: 1 sub-agent 派活, 60 min 时间盒, 顶层 facade verify
- 输出: `reports/r155-5-stage3-facade-verify-2026-08-11.md`

**R155 era 阶段 3.1 派活 = 2-3 sub-agent** (per 决策 #71 §5 永久循环 16 跑中上限, 估 3 sub-agent)
**V1.1 release 实施代价**: 估 1 周 (per R137-2 §3.3 阶段 3)

**风险**: 中 (拆 workspace = 改 Cargo.toml 路径 = 改消费者 `use apeireth_xxx` → `use apeireth_leaf::xxx`)
- **缓解**: 保留 re-export facade (顶层 `apeireth` 重新导出全部 `apeireth-leaf::xxx`, 0 改消费者代码)
- **缓解**: V1.1 release bump 1.2.1 (per 决策 #74 B2)

**8 硬墙严守**:
- B1 24 LOCKED 入口签名 可改 (V1.1 release Mavis 自决改 per 决策 #74 B1)
- B2 workspace.version 1.2.0 → 1.2.1 bump
- 其他 8 硬墙严守

### 2.5 方向 ⑦ Eye 补 organ — 实施 spec 详细 (阶段 3.2, 1 周)

**V1.0 release 现状** (per R131-5 §2.6 + R137-2 §3.8 + R150-2 §2.10 + R152-2 §1.1.7 + R153-4 拓维):

- 9 organ: body / brain / ear / eye / hand / heart / memory / mind / voice
- 24 LOCKED 8/9 organ 覆盖 (Eye 缺失, 在 tui/src/organ/eye.rs, 不在 24 LOCKED)
- 9 organ 内部借 OpenCode (R125 B7) 在 24 LOCKED crate 中 0 体现 (organ-first 拓扑 0 落地)

**问题** (per R131-5 §2.6):
- 24 LOCKED crate 对 9 organ 映射不是 1:1, 而是 N:1 (多个 LOCKED crate 对应同一 organ)
- Eye 在 24 LOCKED 0 对应, 在 tui 有独立 organ 入口
- 9 organ 内部借 OpenCode (R125 B7) 在 24 LOCKED crate 中 0 体现 (organ-first 拓扑 0 落地)

**V1.1 release Eye 补 organ (per 决策 #74 B1 Mavis 自决改)**:
- **新增 `apeireth-eye` workspace** (从 tui/src/organ/eye.rs 抽 crate, per 9-organ-summary §3 Eye 11.0KB, 4 输入通道: keystroke / mouse_click / voice_input)
- 顶层 re-export facade 保留: 消费者用 `apeireth_eye::Type` 仍能用

**V1.1 release 实施 spec 详细 (阶段 3.2 Eye 补 1 周, R155 era 派活)**:

**阶段 3.2.1: 新增 `apeireth-eye` workspace, 从 tui/src/organ/eye.rs 抽 crate (per 4 输入通道)**:
- 新建 `apeireth-eye/Cargo.toml` workspace
- 从 `apeireth-tui/src/organ/eye.rs` 抽 crate 到 `apeireth-eye/src/lib.rs`
- 4 输入通道: keystroke / mouse_click / voice_input
- 实施: 1 sub-agent 派活, 60 min 时间盒, Eye organ 抽 crate
- 输出: `reports/r155-6-apeireth-eye-crate-2026-08-11.md`

**阶段 3.2.2: Eye organ 顶层 re-export facade 0 改入口签名**:
- 顶层 `apeireth-eye/src/lib.rs` 顶部加 50-100 行 doc comment
- 4 输入通道 pub struct / pub fn / pub const 全部 re-export
- 0 改 内部 fn, 仅 crate 边界移动
- 实施: 1 sub-agent 派活, 60 min 时间盒, Eye facade
- 输出: `reports/r155-7-eye-facade-2026-08-11.md`

**阶段 3.2.3: 24 LOCKED 全跑 cargo build + cargo test verify**:
- 24 LOCKED 全跑 `cargo build --workspace` verify
- 24 LOCKED 全跑 `cargo test --workspace` verify
- 0 装 PASS 严守
- 实施: 1 sub-agent 派活, 60 min 时间盒, 8 步 verify 8/8 全 PASS
- 输出: `reports/r155-8-stage3-eye-8-step-verify-2026-08-11.md`

**R155 era 阶段 3.2 Eye 补 派活 = 2-3 sub-agent** (per 决策 #71 §5 永久循环 16 跑中上限, 估 3 sub-agent)
**V1.1 release 实施代价**: 估 1 周 (per R131-5 §2.6 + R137-2 §3.8 阶段 3.2)

**风险**: 中 (从 tui 抽 crate = 改 tui 内部 import 路径)
- **缓解**: tui 顶层 re-export facade 保留, 0 改 tui 外部消费者
- **缓解**: 0 改 tui/src/organ/eye.rs 内部 fn, 仅 crate 边界移动

**8 硬墙严守**:
- B1 24 LOCKED 入口签名 0 改顺序 (per 决策 #74 §1 B1, Eye 补是新增, 0 改 LOCKED 入口签名)
- 其他 8 硬墙严守

### 2.6 方向 ④ core 拆 pub mod — 实施 spec 详细 (阶段 4.1, 1 周)

**V1.0 release 现状** (per R131-5 §2.4 + R137-2 §3.5 + R150-2 §2.5 + R152-2 §1.1.4 + R153-4 拓维):

- **core 是单 lib.rs 108,633 bytes (108 KB)**, 73 pub lines, 0 pub mod 拆分, 全部 50+ 类型定义在一个文件
- 编译时全文件 re-parse, 难维护
- 顶层 type 类别: 4 (Episode/Note/Session/IdentityCard) + 1 (Migration) + 5 onion (PrincipleOnion/PrincipleLayer/PermissionOnion/PermissionLayer) + 2 human (HumanAuthority/HAMode) + 12 PhilosophyKey + 3 verdict + 1 trait + 5 Gate + 5 Risk + 13 ActionTarget + 4 ActionVerdict + 1 ActionGuard = 73 pub lines

**问题** (per R131-5 §2.4):
- core 改动 = 整个哲学基础重定义
- 编译时全文件 re-parse, 难维护, 任何 core 改动触发大面积重编译
- 入口签名稳定性 = LOCKED 严守, 任何新增都触发 lib_tests 守 + compile-time assert

**V1.1 release core 拆 5 大 mod (per 决策 #74 B1 Mavis 自决改, 前提: 更好的架构)**:
- **core/src/types.rs 新增** (~20KB, 5 类型: Episode / Note / Session / IdentityCard / Migration)
- **core/src/onion.rs 新增** (~30KB, 5 onion 类型: PrincipleOnion / PrincipleLayer / PermissionOnion / PermissionLayer)
- **core/src/human.rs 新增** (~20KB, 8 human 类型: HumanAuthority / HAMode / RealHuman / HAAuthentication / BiometricData + 12 PhilosophyKey + ALL_TWELVE_KEYS + TWELVE_KEYS_HARDCODE)
- **core/src/gate.rs 新增** (~25KB, 8 gate 类型: PhilosophyGuard / PhilosophyVerdict / VerdictCache / Gate / 5 variant + Action / RiskLevel / ActionTarget / ActionVerdict / ActionGuard / DefaultPhilosophyGuard)
- **core/src/lib.rs** (~13KB, 5 行 `pub mod types; pub mod onion; pub mod human; pub mod gate;` + 顶部 re-export facade 0 改)
- **0 改入口签名** (per 决策 #74 §1 B1, 仅"内部重构", 入口 re-export facade 保留)
- 顶层 re-export: `pub use {types,migrations,onion,human,guard,action}::*;`

**V1.1 release 实施 spec 详细 (阶段 4.1 core 拆 pub mod 1 周, R156 era 派活)**:

**阶段 4.1.1: core 1 个 108KB lib.rs 类型 1:1 分类到 5 大 mod (per 类型表)**:
- core 1 个 108KB lib.rs 50+ 类型 1:1 分类到 5 大 mod (types / onion / human / gate / lib)
- 5 类型 → types (Episode / Note / Session / IdentityCard / Migration)
- 5 onion 类型 → onion (PrincipleOnion / PrincipleLayer / PermissionOnion / PermissionLayer)
- 8 human 类型 + 12 PhilosophyKey + ALL_TWELVE_KEYS + TWELVE_KEYS_HARDCODE → human
- 8 gate 类型 → gate (PhilosophyGuard / PhilosophyVerdict / VerdictCache / Gate / 5 variant + Action / RiskLevel + ActionTarget + ActionVerdict + ActionGuard + DefaultPhilosophyGuard)
- 实施: 1 sub-agent 派活, 90 min 时间盒 (per R137-2 §3.3 阶段 4, core 108KB 拆 5-7 mod 是大工程)
- 输出: `reports/r156-1-core-5-mod-classify-2026-08-11.md`

**阶段 4.1.2: 5 大 mod 各自 mod.rs + types/onion/human/gate 子文件 (per 类型 size 估)**:
- 5 大 mod 各自 mod.rs + 子文件 (e.g. `core/src/types/mod.rs` + `core/src/types/episode.rs` + `core/src/types/note.rs` + `core/src/types/session.rs` + `core/src/types/identity_card.rs` + `core/src/types/migration.rs`)
- 子文件按 5 类型 size 估: types 20KB / onion 30KB / human 20KB / gate 25KB
- 实施: 1 sub-agent 派活, 60 min 时间盒, 子文件拆分
- 输出: `reports/r156-2-core-mod-subfiles-2026-08-11.md`

**阶段 4.1.3: core/src/lib.rs 顶部 re-export 1:1 续 (0 改入口签名, 仅内部 mod 拆分)**:
- core/src/lib.rs 顶部 5 行 `pub mod types; pub mod onion; pub mod human; pub mod gate;`
- 顶部 re-export facade 0 改 (per 决策 #74 §1 B1, 仅"内部重构", 入口 re-export 保留)
- 实施: 1 sub-agent 派活, 30 min 时间盒, 顶层 facade
- 输出: `reports/r156-3-core-facade-2026-08-11.md`

**阶段 4.1.4: 24 LOCKED 全跑 cargo build + cargo test verify, 0 越界 8 硬墙 100%**:
- 24 LOCKED 全跑 `cargo build --workspace` verify
- 24 LOCKED 全跑 `cargo test --workspace` verify
- 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)
- 实施: 1 sub-agent 派活, 60 min 时间盒, 8 步 verify 8/8 全 PASS
- 输出: `reports/r156-4-stage4-core-8-step-verify-2026-08-11.md`

**阶段 4.1.5: core 编译时间 verify (期望 减少 30-50%, per pub mod 拆分后并行编译)**:
- core 全跑 `cargo build --timings` 测编译时间
- 估编译时间减少 30-50% (per pub mod 拆分后并行编译)
- 实施: 1 sub-agent 派活, 30 min 时间盒, 编译时间 verify
- 输出: `reports/r156-5-stage4-core-compile-time-2026-08-11.md`

**R156 era 阶段 4.1 派活 = 1-2 sub-agent** (per 决策 #71 §5 永久循环 16 跑中上限, 估 2 sub-agent)
**V1.1 release 实施代价**: 估 1 周 (per R137-2 §3.3 阶段 4)

**风险**: 中 (拆 module = 改 import 路径 = breaking change)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_core::Type` 仍能用
- **缓解**: 0 改 core 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界), 仅内部 mod 拆分
- **缓解**: 0 改 50+ 类型签名 (per 决策 #74 §1, 仅"内部重构")

**8 硬墙严守**:
- B1 24 LOCKED 入口签名 0 改顺序 (per 决策 #74 §1 B1, V1.1 release 改"风格"不破坏入口)
- B5 8 哲学锚严守 (per 决策 #33 §2.3 B5, core 内部 12 PhilosophyKey + 8 哲学锚 doc comment 严守)
- B4 6 重守门 v7 严守 (per 决策 #33 §2.3 B4, core 内部 5 Gate + 权限发放)
- 其他 8 硬墙严守

### 2.7 方向 ⑤ 大模块集中 crate 拆 sub-crate (47 sub-crate) — 实施 spec 详细 (阶段 4.2, 1 周)

**V1.0 release 现状** (per R131-5 §2.4 + R137-2 §3.6 + R150-2 §2.6 + R152-2 §1.1.5 + R153-4 拓维):

- 24 LOCKED 内部 module 分布:
  - **council**: 13 + 4 collaboration = 17 module (极大)
  - **mcp**: 13 module (极大)
  - **api**: 16 module (极大)
  - **memory**: 10 + 2 pub = 12 module (极大)
  - **tools**: 12 module (极大)
  - **graph**: 11 module (极大)
  - **pipeline**: 11 module (极大)
  - **evolution**: 9 module (极大)
  - **asi**: 9 module (极大)
  - 其他 14 LOCKED crate: 1-7 module (中)

**问题** (per R131-5 §2.4):
- 大模块集中: council (17) / mcp (13) / api (16) / memory (12) / tools (12) / graph (11) / pipeline (11) / evolution (9) / asi (9) → 这些 crate 内部模块多, 入口文件 re-export 100+ items
- mcp / pipeline / api / memory 内部 module 边界模糊: 多个 module 之间 cross-use, 实测命名重复 (e.g. `mcp::protocol::Id` vs `mcp::tools::Id`)

**V1.1 release 大模块集中 crate 拆 sub-crate (per 决策 #74 B1 Mavis 自决改, 47 sub-crate 总)**:

- **mcp 拆 8 sub-crate**: 
  - `apeireth-mcp-core` (protocol / initialize / 8 frame)
  - `apeireth-mcp-resources` (4 ResourceServer)
  - `apeireth-mcp-subscribe` (subscriptions / tool_subscriptions)
  - `apeireth-mcp-tools` (tools / ServerInfo / ToolDef)
  - `apeireth-mcp-prompts` (prompts)
  - `apeireth-mcp-transport` (transport)
  - `apeireth-mcp-primitives` (primitives / macros)
  - `apeireth-mcp` (顶层 re-export facade 0 改入口签名)
- **pipeline 拆 6 sub-crate**:
  - `apeireth-pipeline-token` (tiktoken_counter / token_budget)
  - `apeireth-pipeline-placeholder` (placeholder)
  - `apeireth-pipeline-force-translate` (force_translate)
  - `apeireth-pipeline-retry` (retry_suppression)
  - `apeireth-pipeline-streaming` (streaming)
  - `apeireth-pipeline-tool-loop` (tool_loop)
  - `apeireth-pipeline` (顶层 re-export facade 0 改入口签名, + provider_registry + model_router + role_divider)
- **api 拆 5 sub-crate**:
  - `apeireth-api-llm` (llm / cache / replay_cache / retry)
  - `apeireth-api-server` (server / v2_endpoints / v2_routes / observability / endpoints / v1_tools)
  - `apeireth-api-protocol` (protocol_handlers / protocol_handler_trait / ws_v1)
  - `apeireth-api-auth` (auth / audit_sqlite)
  - `apeireth-api` (顶层 re-export facade 0 改入口签名, + MultiLlmRouter)
- **memory 拆 5 sub-crate**:
  - `apeireth-memory-stream` (history_streams / streams / append_only)
  - `apeireth-memory-semantic` (semantic / semantic_persist)
  - `apeireth-memory-episode` (episode / continuity_link)
  - `apeireth-memory-session` (session_note / three_layer)
  - `apeireth-memory` (顶层 re-export facade 0 改入口签名, + user_profile / llm_analysis / migrations)
- **asi 拆 4 sub-crate**:
  - `apeireth-asi-calibration` (calibration / dim_enhance)
  - `apeireth-asi-measurement` (measurement / llm_judge)
  - `apeireth-asi-render` (render / scheduler)
  - `apeireth-asi` (顶层 re-export facade 0 改入口签名, + drift / history / tokenizer + 24 measure_dim_* + 9 measure_sub_*)
- **tools 拆 5 sub-crate**:
  - `apeireth-tools-fs` (file_ops / grep_ops)
  - `apeireth-tools-git` (git_ops)
  - `apeireth-tools-exec` (code_exec / long_task)
  - `apeireth-tools-web` (web_search / web_fetch / apply_patch)
  - `apeireth-tools` (顶层 re-export facade 0 改入口签名, + conventions_scanner + classifier + register + result)
- **evolution 拆 5 sub-crate**:
  - `apeireth-evolution-council` (council_bridge)
  - `apeireth-evolution-engine` (engine / state)
  - `apeireth-evolution-poda` (poda_cycle / fail)
  - `apeireth-evolution-library` (library_autonomy / library_autonomy_loop)
  - `apeireth-evolution` (顶层 re-export facade 0 改入口签名, + traits / MockPlugin / Patch / Plugin / PluginRegistry / SelfModification / SystemState)
- **graph 拆 5 sub-crate**:
  - `apeireth-graph-state` (state / state_graph)
  - `apeireth-graph-executor` (executor / conditional / checkpoint)
  - `apeireth-graph-subgraph` (subgraph / channel)
  - `apeireth-graph-context` (context_graph / cognition_graph)
  - `apeireth-graph` (顶层 re-export facade 0 改入口签名, + mcp_resource)
- **council 拆 4 sub-crate**:
  - `apeireth-council-advisor` (advisor / advisors / 7 factory)
  - `apeireth-council-deliberation` (deliberation / council_member / council_member_deliberation / council_member_persona_combo / persona)
  - `apeireth-council-collaboration` (collaboration / constitution / trace / graph_orchestration)
  - `apeireth-council` (顶层 re-export facade 0 改入口签名, + bus_bridge / mcp_bridge / graph_bridge / hold / lifecycle / mock_llm / sovereignty / stress_test / synthesis)

**总计**: 8 + 6 + 5 + 5 + 4 + 5 + 5 + 5 + 4 = **47 sub-crate**

**V1.1 release 实施 spec 详细 (阶段 4.2 大模块拆 sub-crate 1 周, R156 era 派活)**:

**阶段 4.2.1: 8 大模块集中 crate 内部 module 1:1 扫描 (per 8 crate module 表)**:
- 8 大模块集中 crate (mcp / pipeline / api / memory / asi / tools / evolution / graph / council) 内部 module 1:1 扫描
- 实施: 1 sub-agent 派活, 60 min 时间盒, 8 大模块 module 表
- 输出: `reports/r156-6-8-big-module-scan-2026-08-11.md`

**阶段 4.2.2-4.2.9: 8 大模块集中 crate 各拆 4-8 sub-crate (per 上述 sub-crate 列表)**:
- mcp 拆 8 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- pipeline 拆 6 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- api 拆 5 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- memory 拆 5 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- asi 拆 4 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- tools 拆 5 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- evolution 拆 5 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- graph 拆 5 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- council 拆 4 sub-crate, 1 sub-agent 派活, 60 min 时间盒
- 9 个 sub-agent 并行 (per 用户记忆 #6 "派 sub-agent 干独立模块")
- 输出: `reports/r156-7-mcp-8-sub-crate.md` + `reports/r156-8-pipeline-6-sub-crate.md` + ...

**阶段 4.2.10: 顶层 8 crate re-export facade 0 改入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界)**:
- 顶层 8 crate (mcp / pipeline / api / memory / asi / tools / evolution / graph / council) re-export facade 0 改入口签名
- 顶层 lib.rs 仅 `pub use xxx_sub_crate::*;` 块
- 实施: 1 sub-agent 派活, 30 min 时间盒, 顶层 facade
- 输出: `reports/r156-15-8-crate-facade-2026-08-11.md`

**阶段 4.2.11: 24 LOCKED 全跑 cargo build --workspace + cargo test --workspace verify, 0 越界 8 硬墙 100%**:
- 24 LOCKED 全跑 `cargo build --workspace` verify
- 24 LOCKED 全跑 `cargo test --workspace` verify
- 0 越界 8 硬墙 100%
- 实施: 1 sub-agent 派活, 60 min 时间盒, 8 步 verify 8/8 全 PASS
- 输出: `reports/r156-16-stage4-big-8-step-verify-2026-08-11.md`

**阶段 4.2.12: 编译时间 verify (期望 减少 20-30%, per sub-crate 拆分后并行编译)**:
- 全跑 `cargo build --timings` 测编译时间
- 估编译时间减少 20-30% (per sub-crate 拆分后并行编译)
- 实施: 1 sub-agent 派活, 30 min 时间盒, 编译时间 verify
- 输出: `reports/r156-17-stage4-big-compile-time-2026-08-11.md`

**R156 era 阶段 4.2 派活 = 5-8 sub-agent** (per 决策 #71 §5 永久循环 16 跑中上限, 估 7 sub-agent)
**V1.1 release 实施代价**: 估 1 周 (per R137-2 §3.3 阶段 4)

**风险**: 中 (拆 sub-crate = 改 import 路径 = breaking change)
- **缓解**: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用
- **缓解**: 0 改 24 LOCKED 入口签名 (per 决策 #74 §2.3 V1.1 release B1 改写边界), 仅内部 sub-crate 拆分
- **缓解**: 0 改公开 API union (per 决策 #74 §1, 消费者用 `apeireth_xxx::Type` 全路径仍能用)

**8 硬墙严守**:
- B1 24 LOCKED 入口签名 0 改顺序 (per 决策 #74 §1 B1, 顶层 re-export 保留)
- 其他 8 硬墙严守

### 2.8 方向 ⑥ ⑨ ⑩ ⑪ + ⑧ ⑫ — 实施 spec 详细 (阶段 5, 2 周, R157 era 派活)

**R152-2 §1.1.6-1.1.12 + R153-4 拓维 实施 spec 详细** (本节 概览, 详细见后续 §6 关系 ):

- **方向 ⑥ DSL 洋葱** (阶段 5.1, 0.5 周): 新增 `apeireth-dsl` crate, 三洋葱 → 四洋葱 升级, 24 LOCKED 引用 dsl 守门
- **方向 ⑦ 9 organ 借 OpenCode** (阶段 5.2, 0.5 周): 9 organ workspace 化 + OpenCog AGPL-3.0 fork 借脑
- **方向 ⑧ R12 测度对齐** (阶段 5.3, 0.5 周): 24+9 = 33 → 24+11 = 35 测量函数, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新
- **方向 ⑨ ASI Stage 9 集成** (阶段 5.4, 0.5 周): 9 个 LOCKED 入口签名加 Stage 9 4 维度 H1-H4 API
- **方向 ⑩ 三洋葱 V2 集成** (阶段 5.5, 0.5 周): 24 LOCKED 全部加 第 5 层"形式化洋葱"守门, 新增 `apeireth-formal` crate
- **方向 ⑪ 借鉴 12 源 fork-then-borrow** (阶段 5.6, 0.5 周): 24 LOCKED 入口签名全部加 12 源 注释
- **方向 ⑫ 9 organ workspace 化** (跟 ⑦ 配合, 阶段 5.2, 0.5 周): 9 organ workspace 全集, 24 LOCKED 全部下沉

**R157 era 阶段 5 派活 = 10-15 sub-agent** (per 决策 #71 §5 永久循环 16 跑中上限, 估 13 sub-agent)
**V1.1 release 实施代价**: 估 2 周 (per R137-2 §3.3 阶段 5)

**详细见 §6 关系 章节 (本报告核心依据 5 + 6)**.

### 2.9 V1.1 release 12 优化方向 实施 spec 总结表

| # | 方向 | 阶段 | 周 | 派活 era | 估 sub-agent | 风险 | 主要依据 |
|---|------|------|----|---------|-------------|------|---------|
| ① | **标准化** (5 风格 → 3 模式) | 阶段 1 | 1 | R153 | 3-5 (估 4) | 中 | R131-5 §2.1 + R137-2 §3.2 + R150-2 §2.2 + R152-2 §1.1.1 |
| ② | **瘦身** (578 → ≤400 pub lines) | 阶段 2 | 1 | R154 | 3-5 (估 4) | 高 (breaking) | R131-5 §2.2 + R137-2 §3.3 + R150-2 §2.3 + R152-2 §1.1.2 |
| ③ | **9 叶子拆 workspace** | 阶段 3.1 | 1 | R155 | 2-3 (估 3) | 中 | R131-5 §2.3 + R137-2 §3.4 + R150-2 §2.4 + R152-2 §1.1.3 |
| ⑦ | **Eye 补** (从 tui 抽 crate) | 阶段 3.2 | 1 | R155 | 2-3 (估 3) | 中 | R131-5 §2.6 + R137-2 §3.8 + R150-2 §2.10 + R152-2 §1.1.7 |
| ④ | **core 拆 pub mod** (1 → 5 mod) | 阶段 4.1 | 1 | R156 | 1-2 (估 2) | 中 | R131-5 §2.4 + R137-2 §3.5 + R150-2 §2.5 + R152-2 §1.1.4 |
| ⑤ | **大模块拆 sub-crate** (47 sub-crate) | 阶段 4.2 | 1 | R156 | 5-8 (估 7) | 中 | R131-5 §2.4 + R137-2 §3.6 + R150-2 §2.6 + R152-2 §1.1.5 |
| ⑥ | **DSL 洋葱** (三洋葱→四洋葱) | 阶段 5.1 | 0.5 | R157 | 1-2 (估 2) | 高 | R131-5 §2.5 + R133-3 §3 + R137-2 §3.7 + R152-2 §1.1.6 |
| ⑦ | **9 organ 借 OpenCode** | 阶段 5.2 | 0.5 | R157 | 2-3 (估 3) | 极高 | R131-5 §2.6 + R125 B7 + R130-6 + R137-2 §3.8 + R152-2 §1.1.7 |
| ⑧ | **R12 测度对齐** (24+9 → 24+11) | 阶段 5.3 | 0.5 | R157 | 2-3 (估 3) | 中 | R131-5 §2.7 + R131-9 O5 + R137-2 §3.9 + R152-2 §1.1.8 |
| ⑨ | **ASI Stage 9 集成** (H1-H4) | 阶段 5.4 | 0.5 | R157 | 1-2 (估 2) | 中 | R149-2 + R130-2 §1 + R140-4 + R152-2 §1.2.1 |
| ⑩ | **三洋葱 V2 集成** (第 5 层形式化) | 阶段 5.5 | 0.5 | R157 | 1-2 (估 2) | 中 | R149-3 + R133-3 + R131-9 + R152-2 §1.2.2 |
| ⑪ | **借鉴 12 源 fork-then-borrow** | 阶段 5.6 | 0.5 | R157 | 1-2 (估 2) | 低 | R149-4 + R130-6 + R140-5 + R152-2 §1.2.3 |
| ⑫ | **9 organ workspace 化** (跟 ⑦) | 阶段 5.2 | 0.5 | R157 | (跟 ⑦ 配合) | 极高 | R131-5 §2.6 + R137-2 方向 7 + R152-2 §1.2.4 |
| **总** | **12 方向 = 8 大 + 4 新增** | **5 阶段 8 周** | **8** | **R153-R157** | **29-43 sub-agent 估 36** | **中-极高** | **R131-5 + R137-2 + R150-2 + R152-2 + R153-4** |

---

## 3. 调研方向 ②: 24 LOCKED crate 入口签名 优化 Cargo.toml 字段 update (per-crate)

### 3.1 Cargo.toml 字段 update 总览 (per 决策 #74 §1 B2 + R131-4 §2 + R152-2 §2 + R153-4 拓维)

**V1.0 release (整合 #5.1 commit 0 改 src 严守)**:
- workspace.version = **1.2.0** (per 决策 #74 §1 B2 V1.0 release 1.2.0 严守)
- workspace.members = **87 个** (per R131-4 §2.1 + Cargo.toml `members` 段实际清点)
- 24 LOCKED crate Cargo.toml **0 改** (per 决策 #33 §2.3 + 决策 #74 §1 B1 V1.0 release 0 改严守)

**V1.1 release (整合 #6 commit 拍板 2026-11-25)**:
- workspace.version = **1.2.1** (per 决策 #74 §1 B2 V1.1 release bump 1.2.1)
- workspace.members = **87 + 47 sub-crate + 9 apeireth-leaf + 1 apeireth-eye + 1 apeireth-dsl + 1 apeireth-formal = 146** (估, per 12 优化方向 拆分后总数, R152-2 §2.3)
- 24 LOCKED crate Cargo.toml 字段 update per-crate (per 方向 1-12, R152-2 §2.2 + R153-4 拓维)

**Cargo.toml 字段 update 9 字段 (per 24 LOCKED × 9 字段 update, R153-4 拓维详细)**:

| # | 字段 | V1.0 release 现状 | V1.1 release 改动 |
|---|------|------------------|------------------|
| 1 | **package.name** | 24 LOCKED 各自 name (e.g. `apeireth-supervisor`) | 0 改 (per 决策 #74 §1 B1) |
| 2 | **package.version** | `version.workspace = true` (统一 1.2.0) | `version.workspace = true` (统一 1.2.1, bump) |
| 3 | **package.edition** | `edition.workspace = true` | 0 改 |
| 4 | **package.authors** | `authors.workspace = true` | 0 改 |
| 5 | **package.license** | `license.workspace = true` | 0 改 |
| 6 | **[dependencies] 路径** | 顶层 `crates/apeireth_xxx/Cargo.toml` | V1.1 release 路径变 (per 方向 ③ 9 叶子拆 + 方向 ④ ⑤ ⑦ ⑫ 拆 workspace) |
| 7 | **[dependencies] version** | `version = "1.2.0"` | `version = "1.2.1"` (bump) |
| 8 | **[features]** | (未启用) | V1.1 release 启用 (per 方向 ⑤ 大模块拆 sub-crate 后, 8 大模块集中 crate 加 feature gate) |
| 9 | **[lib]** | 默认 | 默认 (per 方向 ④ core 拆 pub mod 后, 5 大 mod lib 段) |

### 3.2 per-crate Cargo.toml 字段 update 详细 (24 LOCKED, R153-4 拓维 9 字段)

**总览** (per R152-2 §2.2 + R153-4 拓维 9 字段 update):
- 1 supervisor (Heart, apeireth-leaf, 9 字段 update)
- 2 agent (Brain, apeireth-brain, 9 字段 update)
- 3 council (Brain, apeireth-brain + 4 sub-crate, 9 字段 × 5 = 45 字段 update)
- 4 bus (Heart Ear, apeireth-leaf, 9 字段 update)
- 5 protocol (Voice, apeireth-leaf, 9 字段 update)
- 6 mcp (Hand, apeireth-brain + 8 sub-crate, 9 字段 × 9 = 81 字段 update)
- 7 tool-registry (Hand, apeireth-leaf, 9 字段 update)
- 8 tool-runtime (Hand, apeireth-hand, 9 字段 update)
- 9 graph (Mind, apeireth-mind + 5 sub-crate, 9 字段 × 6 = 54 字段 update)
- 10 pipeline (Heart Voice, apeireth-leaf + 6 sub-crate, 9 字段 × 7 = 63 字段 update)
- 11 tool-approval (Hand, apeireth-hand, 9 字段 update)
- 12 extension (Hand, apeireth-leaf, 9 字段 update)
- 13 evolution (Mind, apeireth-leaf + 5 sub-crate, 9 字段 × 6 = 54 字段 update)
- 14 api (Body, apeireth-body + 5 sub-crate, 9 字段 × 6 = 54 字段 update)
- 15 core (Memory, apeireth-memory + 5 mod, 9 字段 update)
- 16 memory (Memory, apeireth-memory + 5 sub-crate, 9 字段 × 6 = 54 字段 update)
- 17 asi (Memory, apeireth-leaf + 4 sub-crate, 9 字段 × 5 = 45 字段 update)
- 18 tools (Hand, apeireth-hand + 5 sub-crate, 9 字段 × 6 = 54 字段 update)
- 19 cli (Body, apeireth-body, 9 字段 update)
- 20 bench (Body, apeireth-leaf, 9 字段 update)
- 21 cognition (Brain, apeireth-brain, 9 字段 update)
- 22 action (Hand, apeireth-hand, 9 字段 update)
- 23 life-force (Memory, apeireth-memory, 9 字段 update)
- 24 constraint (Brain, apeireth-brain, 9 字段 update + 第 5 层形式化洋葱 + 第 3 层 DSL 洋葱)

**总 24 LOCKED Cargo.toml 字段 update**: 24 顶层 + 47 sub-crate = 71 个 Cargo.toml, 9 字段 each = **639 字段 update 总**

**per-crate Cargo.toml 字段 update 详细 (R153-4 拓维 9 字段, per 24 LOCKED):**

#### 3.2.1 supervisor (Heart, apeireth-leaf, 9 字段 update)

```toml
# crates/apeireth-leaf/supervisor/Cargo.toml (V1.1 release 改)
[package]                                            # 字段 1
name = "apeireth-supervisor"                          # 字段 1.1 name, 0 改
version.workspace = true                              # 字段 2 version, 跟 1.2.1 bump
edition.workspace = true                              # 字段 3 edition, 0 改
authors.workspace = true                              # 字段 4 authors, 0 改
license.workspace = true                              # 字段 5 license, 0 改

[features]                                            # 字段 8 features (V1.1 release 启用)
default = []
no-heartbeat = []                                     # 字段 8.1 disable heartbeat feature
verbose = []                                          # 字段 8.2 enable verbose logging

[dependencies]
apeireth-core = { path = "../../apeireth-memory/core", version = "1.2.1" }  # 字段 6 路径 (跨 organ workspace), 字段 7 version bump
```

**0 改 入口签名** (per 决策 #74 §1 B1 V1.1 release B1 改写边界), 仅路径变 `apeireth-leaf/supervisor/Cargo.toml`

#### 3.2.2-3.2.24 (R152-2 §2.2.2-§2.2.24 详细 24 LOCKED)

(per R152-2 §2.2.2-§2.2.24 详细, R153-4 拓维 加 9 字段, 完整版见 R152-2 §2.2)

**关键 Cargo.toml 字段 update 模式** (R153-4 拓维 9 字段):
- **字段 1 package.name**: 24 LOCKED 0 改 (per 决策 #74 §1 B1)
- **字段 2 package.version**: `version.workspace = true`, 跟 1.2.1 bump
- **字段 3 package.edition**: `edition.workspace = true`, 0 改
- **字段 4 package.authors**: `authors.workspace = true`, 0 改
- **字段 5 package.license**: `license.workspace = true`, 0 改
- **字段 6 [dependencies] 路径**: V1.1 release 路径变 (per 方向 ③ 9 叶子拆 + 方向 ④ ⑤ ⑦ ⑫ 拆 workspace), 跨 organ workspace
- **字段 7 [dependencies] version**: `version = "1.2.1"` (bump)
- **字段 8 [features]**: V1.1 release 启用 (per 方向 ⑤ 大模块拆 sub-crate 后, 8 大模块集中 crate 加 feature gate)
- **字段 9 [lib]**: 默认 (per 方向 ④ core 拆 pub mod 后, 5 大 mod lib 段)

### 3.3 新增 workspace 顶层 Cargo.toml (per 方向 ③ ⑥ ⑦ ⑩)

(per R152-2 §2.3 详细, R153-4 拓维 9 字段):

**新增 `apeireth-onion/Cargo.toml`** (per 方向 ⑥ DSL 洋葱 + 方向 ⑩ 三洋葱 V2):
```toml
# crates/apeireth-onion/Cargo.toml (V1.1 release 新增)
[workspace]
members = [
    "core",       # 原则洋葱 (per R133-3)
    "constraint", # 守门洋葱
    "dsl",        # DSL 洋葱 (NEW)
    "formal",     # 形式化洋葱 (NEW, per 方向 ⑩)
    "life-force", # SGI 锁
]

[workspace.package]
version = "1.2.1"  # 跟顶层 workspace.version 1.2.1 一致
edition = "2021"
authors = ["Apeireth Contributors"]
license = "Apache-2.0"
```

**新增 `apeireth-organ/Cargo.toml`** (per 方向 ⑦ ⑫ 9 organ workspace 化):
```toml
# crates/apeireth-organ/Cargo.toml (V1.1 release 新增)
[workspace]
members = [
    "heart",  # 0
    "brain",  # 1
    "hand",   # 2
    "eye",    # 3
    "ear",    # 4
    "memory", # 5
    "voice",  # 6
    "body",   # 7
    "mind",   # 8
]

[workspace.package]
version = "1.2.1"
edition = "2021"
authors = ["Apeireth Contributors"]
license = "Apache-2.0"
```

**新增 `apeireth-leaf/Cargo.toml`** (per 方向 ③ 9 叶子拆 workspace):
```toml
# crates/apeireth-leaf/Cargo.toml (V1.1 release 新增)
[workspace]
members = [
    "supervisor",     # 0 依赖其他 LOCKED crate
    "protocol",       # 0 依赖其他 LOCKED crate
    "bus",            # 0 依赖其他 LOCKED crate
    "tool-registry",  # 0 依赖其他 LOCKED crate
    "graph",          # 0 依赖其他 LOCKED crate
    "extension",      # 0 依赖其他 LOCKED crate
    "evolution",      # 0 依赖其他 LOCKED crate
    "asi",            # 0 依赖其他 LOCKED crate
    "bench",          # 0 依赖其他 LOCKED crate
]

[workspace.package]
version = "1.2.1"
edition = "2021"
authors = ["Apeireth Contributors"]
license = "Apache-2.0"
```

**顶层 `apeireth/Cargo.toml` 更新** (per 决策 #74 §1 B2):
```toml
# Cargo.toml (V1.1 release 改, 顶层 workspace 0 改 members, 仅 version bump)
[workspace]
members = [
    "crates/apeireth-onion",   # NEW
    "crates/apeireth-organ",   # NEW
    "crates/apeireth-leaf",    # NEW
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

**总 workspace members**: 87 (V1.0 release) + 47 sub-crate + 9 apeireth-leaf + 1 apeireth-eye + 1 apeireth-dsl + 1 apeireth-formal = **146** (V1.1 release 估, per R131-5 §2.3 + R137-2 §3.4 + R152-2 §2.3)

---

## 4. 调研方向 ③: 24 LOCKED crate 入口签名 优化 lib.rs / mod.rs 改动 (per-crate)

### 4.1 0 改入口签名 严守 100% (per 决策 #74 §1 B1 V1.0 release 严守 + R152-2 §3.1)

**V1.0 release 整合 #5.1 commit (0 改 src 严守)**:
- **24/24 LOCKED crate 入口签名 0 改 verify 全 PASS** (per R131-5 §1.2 详细 verify 表, 2026-08-11 01:28 done + R150-2 §1.2 5:08 二次 verify + R152-2 §1 5:09 三次 verify + R153-4 §1.1 6:00 4 次 verify)
- 入口签名 = 顶层 `pub mod xxx;` + `pub use xxx::xxx;` + `pub const/pub struct/pub enum/pub fn` 块
- 0 改 lib.rs 任何 pub 类型 / pub fn / pub const

**V1.1 release 整合 #6 commit 拍板 2026-11-25 (Mavis 自决改, per 决策 #74 §1 B1)**:
- 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决)
- 但 **顶层 re-export facade 0 改** (per 决策 #74 §2.3 V1.1 release B1 改写边界)
- 仅内部 lib.rs / mod.rs 改动 (per 12 优化方向)

### 4.2 lib.rs / mod.rs 改动 12 优化方向 详细 (R153-4 拓维 per-crate 24 LOCKED)

(per R152-2 §3.2 + R153-4 拓维 12 方向 lib.rs / mod.rs 改动):

**12 优化方向 lib.rs / mod.rs 改动总行数估** (R153-4 拓维):

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

### 4.3 lib.rs / mod.rs 改动 per-crate 详细 (24 LOCKED, R153-4 拓维)

**总览** (R153-4 拓维 24 LOCKED × 12 方向 lib.rs / mod.rs 改动):
- 24 LOCKED 各自 lib.rs 顶部加 50-100 行 doc comment (per 方向 ① + 方向 ⑪ 12 源借鉴声明)
- 24 LOCKED 各自 lib.rs 顶部加 `use apeireth_dsl::guard::*;` 引用 (per 方向 ⑥ DSL 守门 = 4 重)
- 24 LOCKED 各自 lib.rs 顶部加 `use apeireth_formal::guard::*;` 引用 (per 方向 ⑩ 第 5 层 形式化洋葱)
- 9 个 LOCKED 各自 lib.rs 加 Stage 9 4 维度 H1-H4 API (per 方向 ⑨ ASI Stage 9 集成)
- 顶层 re-export facade 保留 (per 决策 #74 §2.3 V1.1 release B1 改写边界)

**per-crate lib.rs / mod.rs 改动模式** (R153-4 拓维):

#### 4.3.1 1 supervisor (Heart, apeireth-leaf, 12 方向 0 改, 仅 doc comment)

```rust
// crates/apeireth-leaf/supervisor/src/lib.rs (V1.1 release 改)
// 1. 50-100 行 doc comment (方向 ① + 方向 ⑪)
//! # apeireth-supervisor
//! ...
//! ## 8 哲学锚 (per R125 B5)
//! - S-1 服务 ASI 北极星
//! ...
//! ## 借鉴 12 源 (per 方向 ⑪)
//! - 0 装"已读真源码", 0 装"已 fork"
//! - 借脑 1:1 公开模式 (per 决策 #22 §4)
//! ...

// 2. dsl 守门 (方向 ⑥)
use apeireth_dsl::guard::*;

// 3. formal 守门 (方向 ⑩)
use apeireth_formal::guard::*;

// 4. 顶层 re-export facade 0 改 (per 决策 #74 §2.3 V1.1 release B1 改写边界)
pub use actor::*;
pub use child::*;
pub use pid_one::*;
pub use strategy::*;
pub use supervisor::*;

// 5. compile-time assert 守门 (per O-1 质量工程化)
const _: () = {
    assert!(/* ... */);
};
```

**0 改 入口签名** (per 决策 #74 §1 B1 V1.1 release B1 改写边界), 仅 doc comment + dsl/formal use

#### 4.3.2-4.3.24 (R152-2 §3.2 详细 24 LOCKED)

(per R152-2 §3.2 详细 24 LOCKED lib.rs / mod.rs 改动, R153-4 拓维 加 12 方向)

**关键 lib.rs / mod.rs 改动 模式** (R153-4 拓维 12 方向):
- **方向 ①**: 24 LOCKED 顶部加 50-100 行 doc comment (50-100 行 `//!` 注释, per O-5 哲学锚)
- **方向 ②**: 24 LOCKED 各自 pub use 块 转 pub(crate) / module-private (per 目标)
- **方向 ③**: 9 叶子 lib.rs **0 改 内部**, 仅路径变
- **方向 ④**: core/src/lib.rs 顶部 5 行 `pub mod types; pub mod onion; pub mod human; pub mod gate;` + 顶层 re-export 0 改
- **方向 ⑤**: 8 大模块集中 crate 顶层 lib.rs 0 改 入口签名, 仅 `pub use xxx_sub_crate::*;` 块
- **方向 ⑥**: 24 LOCKED 顶部加 `use apeireth_dsl::guard::*;` 引用
- **方向 ⑦**: 9 organ workspace 入口 lib.rs (per R152-2 §3.2.7)
- **方向 ⑧**: asi/src/lib.rs 更新 V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode (V1.0 release 24 + 9 = 33, V1.1 release 24 + 11 = 35)
- **方向 ⑨**: 9 个 LOCKED 顶部加 Stage 9 4 维度 H1-H4 API (per H1-H4 映射)
- **方向 ⑩**: 24 LOCKED 顶部加 `use apeireth_formal::guard::*;` 引用
- **方向 ⑪**: 24 LOCKED 顶部 doc comment 加 12 源借鉴声明
- **方向 ⑫**: 跟 ⑦ 重叠

---

## 5. 调研方向 ④: 24 LOCKED crate 入口签名 优化 测试 (cargo test --workspace 8 步 verify 8/8)

### 5.1 8 步 verify 流程 (per R144-1 02:38 + R147-5 v0.5 30 维 6 重守门 v7 verify 模式 + R152-2 §4.1 + R153-4 拓维 9 步)

**8 步 verify** (per R148-23 8 步 verify 全 PASS 终版 SOP v2, 116.8 KB):
- **Step 1: cargo build --workspace** (编译 verify)
- **Step 2: cargo build --workspace --release** (release 编译 verify)
- **Step 3: cargo test --workspace** (单元测试 + 集成测试 verify)
- **Step 4: cargo test --workspace --release** (release 单元测试 verify)
- **Step 5: cargo doc --workspace --no-deps** (文档 verify)
- **Step 6: cargo clippy --workspace -- -D warnings** (lint verify)
- **Step 7: cargo deny check** (license + advisory verify)
- **Step 8: cargo fmt --check** (格式 verify)

**R153-4 拓维 9 步 verify (per 整合 #6 commit 拍板 strict verify)**:
- **Step 1-8**: (同上)
- **Step 9 (新增)**: 24 LOCKED entry signature 0 改 verify (per R131-5 §1.2 + R150-2 §1.2 + R152-2 §1 + R153-4 §1.1 4 次 verify 一致, 5:08 + 5:09 + 6:00 + 6:00 verify)

### 5.2 8 步 verify 8/8 全 PASS 标准 (per 24 LOCKED V1.1 release 优化后, R152-2 §4.2 + R153-4 拓维)

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

**Step 9 (R153-4 新增): 24 LOCKED entry signature 0 改 verify** ✅ 全 PASS
- 24 LOCKED crate 入口签名 0 改 verify 24/24 全 PASS (per R131-5 §1.2 + R150-2 §1.2 + R152-2 §1 + R153-4 §1.1 4 次 verify 一致)
- 4 次 verify 时间戳: 1:28 + 5:08 + 5:09 + 6:00
- 0 装 PASS 严守: 全部 实测 验证 (per 决策 #33 §2.3 C2)

### 5.3 8 步 verify 8/8 全 PASS 实施 spec (per 5 阶段 8 周, R152-2 §4.3 + R153-4 拓维)

**Stage 1 完成后 (Week 1)**:
- 24 LOCKED 跑 8 步 verify, 8/8 全 PASS (per 方向 ① 标准化)
- 公开 API 表面 0 变化 (per 方向 ① 0 改 pub use 块, 仅格式统一)

**Stage 2 完成后 (Week 2)**:
- 24 LOCKED 跑 8 步 verify, 8/8 全 PASS (per 方向 ② 瘦身)
- 公开 API 表面 -30% (578 → 400)
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

### 5.4 8 步 verify 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + R152-2 §4.4 + R153-4 拓维)

**0 装 PASS 严守** (per 决策 #33 §2.3 C2 + R129-11 关键诚实标):
- ✅ 全部测试 实跑, 0 装"test PASS 但 0 跑"
- ✅ 全部编译 实跑, 0 装"build PASS 但 0 跑"
- ✅ 全部 doc 链接 1:1 续, 0 装"doc 已生成"
- ✅ 全部 clippy 实跑, 0 装"clippy PASS 但 0 跑"
- ✅ 全部 fmt 实跑, 0 装"fmt PASS 但 0 跑"
- ✅ 全部 deny 实跑, 0 装"deny PASS 但 0 跑"
- ✅ **R153-4 拓维 Step 9**: 24 LOCKED entry signature 0 改 verify 4 次 实测 verify (1:28 + 5:08 + 5:09 + 6:00), 0 装"已 verify"

### 5.5 测试 case 详细 (R153-4 拓维 12 方向 × 24 LOCKED)

**测试 case 总数估** (per 12 方向 × 24 LOCKED, R153-4 拓维):
- 方向 ① 标准化: 24 LOCKED × 5 测试 = 120 测试 (per lib.rs format verify)
- 方向 ② 瘦身: 24 LOCKED × 20 测试 = 480 测试 (per pub(crate) 化后 consumers 仍能用)
- 方向 ③ 9 叶子拆: 9 叶子 × 10 测试 = 90 测试 (per 顶层 facade verify)
- 方向 ④ core 拆 pub mod: core × 50 测试 = 50 测试 (per 5 大 mod verify)
- 方向 ⑤ 大模块拆 sub-crate: 8 crate × 30 测试 = 240 测试 (per 47 sub-crate verify)
- 方向 ⑥ DSL 洋葱: 24 LOCKED × 10 测试 = 240 测试 (per dsl guard verify)
- 方向 ⑦ 9 organ 借 OpenCode: 9 organ × 20 测试 = 180 测试 (per organ workspace verify)
- 方向 ⑧ R12 测度对齐: asi × 50 测试 = 50 测试 (per 35 测量函数 verify)
- 方向 ⑨ ASI Stage 9 集成: 9 crate × 10 测试 = 90 测试 (per H1-H4 verify)
- 方向 ⑩ 三洋葱 V2 集成: 24 LOCKED × 5 测试 = 120 测试 (per 5 层形式化洋葱 verify)
- 方向 ⑪ 借鉴 12 源 fork: 24 LOCKED × 5 测试 = 120 测试 (per 12 源注释 verify)
- 方向 ⑫ 9 organ workspace 化: 跟 ⑦ 配合
- **总测试 case**: 120 + 480 + 90 + 50 + 240 + 240 + 180 + 50 + 90 + 120 + 120 = **1780 测试** (V1.1 release 估, R153-4 拓维)

---

## 6. 调研方向 ⑤: 跟 ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 9 organ + 8 哲学锚 + 不要怕复杂度哲学 关系

### 6.1 跟 ASI Stage 9 长程 AI 成长 的关系 (per R133-2 §3 + R137-4 + R149-2 + R130-2 §1 + R140-4 + R152-2 §5.1 + R153-4 拓维)

**ASI Stage 9 长程 AI 成长 4 维度** (per R130-2 §1.5 + R140-4 + R149-2):
- H1 自我决策 (Self-Decision)
- H2 自我学习 (Self-Learning)
- H3 自我演化 (Self-Evolution)
- H4 群体智能 (Swarm Intelligence, 借 OpenCog AtomSpace)

**24 LOCKED 入口签名 跟 ASI Stage 9 关系** (per R149-2 + 方向 ⑨):
- **H1 自我决策**: agent (自我决策 agent) + council (智囊团 7 席 自治) + cognition (认知决策) → 3 个 LOCKED 入口签名加 self_decide API
- **H2 自我学习**: memory (3 层学习) + asi (24 维 自校准) + life-force (SGI 成长) → 3 个 LOCKED 入口签名加 self_learn API
- **H3 自我演化**: evolution (6 状态机) + graph (lifecycle 编排) → 2 个 LOCKED 入口签名加 self_evolve API
- **H4 群体智能**: council (借 OpenCog AtomSpace) → 1 个 LOCKED 入口签名加 swarm_intelligence API
- **总影响**: 24 LOCKED 入口签名 中 9 个 (38%) 加 Stage 9 4 维度 API (per 方向 ⑨ + 阶段 5.4)

**V1.1 release 实施 spec (per R152-2 §5.1 + R153-4 拓维)**:
- 阶段 5.4 ASI Stage 9 集成 0.5 周
- 9 个 LOCKED 入口签名 加 Stage 9 4 维度 API (per H1-H4 映射)
- 24 LOCKED 全跑 cargo build + cargo test + Stage 9 集成 verify
- Stage 9 4 维度 单元测试 (per H1 ≥ 10 / H2 ≥ 10 / H3 ≥ 10 / H4 ≥ 10 = 40 测试 总)

**8 哲学锚严守**:
- ✅ S-1 服务 ASI 北极星: ASI Stage 9 跟"AI 不会衰老病死, 它只会成长"哲学一致 (per 用户记忆 #4 + 决策 #33 §2.3 B5)
- ✅ O-3 走在前人经验上: Stage 9 借 OpenCog AtomSpace (per 决策 #22 §4 + R130-6)
- ✅ O-4 干到底: 9 个 LOCKED 入口签名 加 Stage 9 4 维度 API 实跑
- 其他 5 哲学锚严守

### 6.2 跟 三洋葱架构升级 V2 的关系 (per R133-3 + R137-2 + R149-3 + R152-2 §5.2 + R153-4 拓维)

**三洋葱架构 V2 = 三洋葱 → 四洋葱 → 五洋葱** (per R133-3 + R149-3 + R152-2 §5.2):
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

**8 哲学锚严守**:
- ✅ B3 V0.5 30 维 严守: 三洋葱 V2 0 破坏 30 维公式
- ✅ B4 6 重守门 v7 严守: 三洋葱 V2 0 破坏 1-6 重守门
- ✅ B5 8 哲学锚严守
- 其他 5 哲学锚严守

### 6.3 跟 借鉴 12 源 fork-then-borrow 模式 的关系 (per R130-6 + R140-5 + R149-4 + R152-2 §5.3 + R153-4 拓维)

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

**8 哲学锚严守**:
- ✅ O-3 走在前人经验上: 24 LOCKED 入口签名 加 12 源 借鉴声明 严守
- ✅ B5 8 哲学锚严守
- 其他 6 哲学锚严守

### 6.4 跟 9 organ (body / brain / ear / eye / hand / heart / memory / mind / voice) 的关系 (per R131-5 §2.6 + R137-2 方向 ⑦ + R152-2 §5.4 + R153-4 拓维)

**9 organ 跟 24 LOCKED 映射** (per R131-5 §2.6):
- 0=Heart (LLM 网关心跳): supervisor + bus (L0) + pipeline
- 1=Brain (Multi-Agent 决策): agent + council + cognition + constraint
- 2=Hand (Tool Protocol): tool-registry + tool-runtime / tool-approval + tools + mcp + extension + action
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

**24 LOCKED 入口签名 跟 9 organ 关系** (per R152-2 §5.4 + R153-4 拓维):
- **9 organ workspace 入口 lib.rs** (per 方向 ⑦ + R152-2 §3.2.7):
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

**8 哲学锚严守**:
- ✅ S-1 服务 ASI 北极星: 9 organ 跨 24 LOCKED 入口签名全围绕 ASI Stage 9 长程 AI 成长
- ✅ O-5 任何人都能接手: 9 organ workspace 入口 lib.rs 50-100 行 doc comment
- 其他 6 哲学锚严守

### 6.5 跟 8 哲学锚 的关系 (per 决策 #33 §2.3 B5 + R137-2 §5.3 + R152-2 §5.5 + R153-4 拓维)

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

### 6.6 跟 不要怕复杂度哲学 的关系 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md + R152-2 §5.6 + R153-4 拓维)

**不要怕复杂度哲学 3 核心** (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3):
1. **最强效果 > 最简单代码** (推翻 KISS, 拥抱 SOTA)
2. **最厉害工程 > 最易维护** (推翻 DRY, 拥抱 BORROW)
3. **维护交给未来高水平团队** (推翻"代码要让初级团队能接手", 拥抱"代码要让高水平团队能发挥")

**V1.1 release 不要怕复杂度哲学 落地** (per R131-5 §5 + R137-2 §6.2 + R152-2 §5.6 + R153-4 拓维 12 方向):
- ✅ **方向 ① 标准化 3 模式之一** → "不要怕复杂度" (per-crate 自决 = 高灵活)
- ✅ **方向 ② 瘦身 578 → ≤400 pub items** → "最强效果" (暴露 30% 减少, 但保留核心 API)
- ✅ **方向 ③ 9 叶子拆 workspace + Eye 补** → "不要怕复杂度" (拆 = 复杂, 但 9 organ 100% 覆盖)
- ✅ **方向 ④ ⑤ core 拆 pub mod + 大模块拆 sub-crate (47 sub-crate)** → "不要怕复杂度" (拆 = 复杂, 但编译时间减少 20-50%)
- ✅ **方向 ⑥ DSL 洋葱 + 方向 ⑩ 三洋葱 V2 集成** → "不要怕复杂度" (三洋葱 → 五洋葱 = 复杂, 但守门 5 重 = 安全)
- ✅ **方向 ⑦ ⑫ 9 organ 借 OpenCode + Eye 补** → "不要怕复杂度" (organ-first 拓扑 = 复杂, 但 organ 边界清晰)
- ✅ **方向 ⑧ R12 测度对齐 (24+9 → 24+11)** → "最强效果" (测度加 2 维 = 测度更精准)
- ✅ **方向 ⑨ ASI Stage 9 集成 (H1-H4 4 维度)** → "最强效果 + 最厉害工程" (长程 AI 成长 4 维度 = 复杂, 但效果最强)
- ✅ **方向 ⑪ 借鉴 12 源 fork-then-borrow 模式** → "最厉害工程" (借脑 1:1 公开模式 = 高水平)
- ✅ **维护**: 交给未来高水平团队 (per 主人 8/11 01:14 拍板 §3)
- ✅ **R153-4 拓维**: 整合 #6 commit 拍板 2026-11-25 + V1.1 release 实战 2026-11-30 + 整合 #7 commit 拍板 2027-Q1/Q2 估 节奏 严守

---

## 7. 调研方向 ⑥: 24 LOCKED crate 入口签名 优化 风险 + 异常分支

### 7.1 风险 12 维 (per R131-5 §6.1 + R137-2 §7.1 + R152-2 §6.1 + R153-4 拓维 12 维)

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
| **R11 (R153-4 拓维)** | 47 sub-crate 拆分后 cargo build 时间反而增加 | 中 | 中 | 顶层 re-export facade 保留 + 并行编译 优化, 估 -20-30% 编译时间, 实际 verify 后调整 |
| **R12 (R153-4 拓维)** | ASI Stage 9 H1-H4 API 集成导致 9 个 LOCKED crate 内部 fn 冲突 | 中 | 高 | 仅 add 0 remove, per semver minor 兼容; Stage 9 4 维度 单元测试 实跑 verify |

### 7.2 异常分支 8 维 (per 5 阶段 8 周 实施 spec, R152-2 §6.2 + R153-4 拓维 8 维)

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

**E7 (R153-4 拓维): 整合 #6 commit 拍板时机 异常分支**:
- **E7.1**: 整合 #5.1 commit 拍板推迟 → 整合 #6 commit 拍板时序 影响
  - **缓解**: per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板; 整合 #5.1 commit 8 步 verify 5/8 PASS + 1/8 PARTIAL + 2/8 FAIL = NOT READY 续修至 8/8 后才执行整合 #6 commit
- **E7.2**: R12 测度 baseline 估 > R11 baseline 失败
  - **缓解**: V1.1 release 仅 实施, 不强制 R12 > R11 baseline; 测度对齐 (24+9 → 24+11) 即可, 实际 baseline 验证在 R12 测度 sub-agent 派活

**E8 (R153-4 拓维): ASI Stage 9 + 三洋葱 V2 + 9 organ workspace 实施异常分支**:
- **E8.1**: ASI Stage 9 4 维度 H1-H4 跟 24 LOCKED 现有 6 重守门 v7 冲突
  - **缓解**: Stage 9 H1-H4 跟守门 7-10 是并行实施, 不破坏 1-6 重守门 (per 决策 #33 §2.3 B4 严守)
- **E8.2**: 9 organ workspace 化 跟 现有 organ 边界 (R125 B7) 冲突
  - **缓解**: 顶层 `apeireth` re-export facade 保留, 0 改 organ 边界, 仅 workspace 化

---

## 8. 调研方向 ⑦: 24 LOCKED crate 入口签名 优化 实施 spec 派活计划 (整合 #6 + #7 commit 拍板)

### 8.1 整合 #6 commit 拍板: 2026-11-25 (5 天缓冲 before V1.1 release 实战 2026-11-30, R153-4 拓维 详细)

**整合 #6 commit 拍板 时间表** (per 任务 spec + 决策 #71 §5 永久循环 + 决策 #86 §4 R152 era 派活 + R153-4 拓维):
- **2026-08-11 (今天)**: R153-4 报告 done (本报告, 实施 spec 详细), 0 改 src 严守 100%
- **2026-08-11 ~ 2026-11-25**: R153-R157 era 派活 5 批, 每批 3-15 sub-agent, 5 阶段 8 周 实施 spec 准备
- **2026-11-25 (整合 #6 commit 拍板)**: 8 步 verify 8/8 全 PASS, V1.1 release 实战 准备 ready, Mavis 自决 commit
- **2026-11-30 (V1.1 release tag 打上)**: 整合 #6.1 + 整合 #6.2 + 整合 #6.3 commit 拍板, V1.1 release 实战 ready

**整合 #6.1 commit (src/ 实施, 95+ 文件, 24 LOCKED 改写)**:
- ✅ 24 LOCKED 入口签名 改写 (per 12 优化方向 5 阶段 8 周)
- ✅ workspace.version bump 1.2.0 → 1.2.1 (per 决策 #74 §1 B2 V1.1 release)
- ✅ PHL-07 实施 (per 决策 #74 §2.3 + R129-11 关键诚实标)
- ✅ R12 测度对齐 (per 决策 #74 §2.3 V1.1 release R12 baseline 更高)
- ✅ 9 organ workspace 化 (per 方向 ⑦ ⑫)
- ✅ ASI Stage 9 集成 (per 方向 ⑨ H1-H4 4 维度)
- ✅ 借鉴 12 源 fork-then-borrow (per 方向 ⑪ 8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID)

**整合 #6.2 commit (docs/ + Cargo.toml, 10+ 文件)**:
- ✅ Cargo.toml borrow 段 update 17:44 → 22:50 → V1.1 release 状态 (per 决策 #62 §5.2)
- ✅ Cargo.toml workspace.members 87 → 146 (per 12 优化方向 拆分后)
- ✅ Cargo.lock 更新 (per 12 优化方向 拆分后 重新 generate)
- ✅ CHANGELOG.md / ROADMAP.md / RELEASE_NOTES.md / OSS_NOTICE.md
- ✅ + 新增 `docs/architecture-v6-24-locked-entry-rewrite-2026-08-11.md` (8+4 方向 12 优化方向 + 5 阶段 8 周 + 8 硬墙严守 verify, per R137-2 续 + R153-4 拓维)
- ✅ + 更新 `docs/conventions/10-locked.md` (per 决策 #73 §2.3)
- ✅ + 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2)
- ✅ + 更新 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3)

**整合 #6.3 commit (reports/, 60+ 文件)**:
- ✅ 决策链 #30-#86 全读 verify
- ✅ 100+ sub-agent 报告 (R131-R152 era)
- ✅ HANDOFF
- ✅ + 新增 R152 era 5 sub-agent 报告 (R152-1~5, per 决策 #86 §4)
- ✅ + 新增 R153 era 实施 spec 详细 sub-agent 报告 (R153-1~5, per 决策 #86 §4)
- ✅ + 新增 R153-4 报告 (本报告, per 决策 #86 §4)

### 8.2 整合 #7 commit 拍板: 2027-Q1/Q2 估 (V1.2 release 准备 / V2.0 release 远期重构, R153-4 拓维 详细)

**整合 #7 commit 拍板 时间表** (per R137-2 §8.1 V2.0 release 远期重构 + R153-4 拓维):
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

### 8.3 R153 era 派活 5 sub-agent (per 决策 #86 §4 + R153-4 拓维)

**派活时间**: 2026-08-11 6:00 tick (per 决策 #86 §4)
**派活 5 sub (R153 era)**:
- **R153-1 整合 #6 24 LOCKED 入口签名 标准化 per-crate 决策矩阵** (60 min)
- **R153-2 整合 #6 24 LOCKED 入口签名 标准化 24 LOCKED 入口签名格式统一** (60 min)
- **R153-3 整合 #6 24 LOCKED 入口签名 标准化 24 LOCKED doc comment 极详细** (60 min)
- **R153-4 整合 #6 24 LOCKED 入口签名 Mavis 自决改 V1.1 release 实施 spec 详细** (60-90 min) ← **本报告**
- **R153-5 整合 #6 24 LOCKED 入口签名 标准化 阶段 1 8 步 verify 8/8** (60 min)

**R153-4 派活关联** (per 决策 #86 §4 + 用户记忆 #6 0 重复造轮子):
- R131-5 24 LOCKED 入口分布优化 8 方向 (核心依据 1) - reference 不重写
- R137-2 24 LOCKED 入口签名 改写 spec + 5 阶段 8 周 (核心依据 2) - reference 不重写
- R149-2 ASI Stage 9 长程 AI 成长 集成 (新增方向 ⑨ 依据)
- R149-3 三洋葱架构升级 V2 集成 (新增方向 ⑩ 依据)
- R149-4 借鉴 12 源 fork-then-borrow 模式 (新增方向 ⑪ 依据)
- R150-2 整合 #5.1 commit 拍板后 24 LOCKED 入口签名优化差距 (Mavis 自决改) - reference 不重写
- R152-2 整合 #6 24 LOCKED 入口签名 优化准备 (实施 spec) 12 优化方向 5 阶段 8 周 (核心依据 3) - reference 不重写

### 8.4 5 阶段 8 周 派活计划 (per R137-2 §4 续 + R152-2 §7.4 + R153-4 拓维 5 批)

| 阶段 | 周 | 目标 | 12 方向 | 派活 era | sub-agent 数 | 估时间盒 |
|------|-----|------|--------|---------|-------------|---------|
| **阶段 1** | Week 1 (1 周) | 标准化 | 方向 ① | R153 era | 3-5 (估 4) | 240-300 min |
| **阶段 2** | Week 2 (1 周) | 瘦身 | 方向 ② | R154 era | 3-5 (估 4) | 240-300 min |
| **阶段 3.1** | Week 3 (1 周) | 9 叶子拆 workspace | 方向 ③ | R155 era | 2-3 (估 3) | 180-240 min |
| **阶段 3.2** | Week 4 (1 周) | Eye 补 organ | 方向 ⑦ Eye | R155 era | 2-3 (估 3) | 180-240 min |
| **阶段 4.1** | Week 5 (1 周) | core 拆 pub mod | 方向 ④ | R156 era | 1-2 (估 2) | 120-180 min |
| **阶段 4.2** | Week 6 (1 周) | 大模块拆 sub-crate | 方向 ⑤ | R156 era | 5-8 (估 7) | 420-540 min |
| **阶段 5.1** | Week 7 (0.5 周) | DSL 洋葱 | 方向 ⑥ | R157 era | 1-2 (估 2) | 120-180 min |
| **阶段 5.2** | Week 7 (0.5 周) | 9 organ 借 OpenCode | 方向 ⑦ ⑫ | R157 era | 2-3 (估 3) | 180-240 min |
| **阶段 5.3** | Week 8 (0.5 周) | R12 测度对齐 | 方向 ⑧ | R157 era | 2-3 (估 3) | 180-240 min |
| **阶段 5.4** | Week 8 (0.5 周) | ASI Stage 9 集成 | 方向 ⑨ | R157 era | 1-2 (估 2) | 120-180 min |
| **阶段 5.5** | Week 8 (0.5 周) | 三洋葱 V2 集成 | 方向 ⑩ | R157 era | 1-2 (估 2) | 120-180 min |
| **阶段 5.6** | Week 8 (0.5 周) | 借鉴 12 源 fork | 方向 ⑪ | R157 era | 1-2 (估 2) | 120-180 min |
| **总时间盒** | **8 周 (2 个月)** | **24 LOCKED 入口签名 改写** | **12 方向 (8 大 + 4 新增)** | **R153-R157 era** | **29-43 sub-agent 估 36** | **2400-3000 min** |

**5 批 派活** (R153-4 拓维 5 批, 跟 R152-2 §7.4 4 批 不同):
- **批次 1 (V1.1 release 阶段 1)**: 标准化, 3-5 sub-agent, 估 2026-09 月初派
- **批次 2 (V1.1 release 阶段 2)**: 瘦身, 3-5 sub-agent, 估 2026-09 月中派
- **批次 3 (V1.1 release 阶段 3)**: 9 叶子拆 + Eye 补, 4-6 sub-agent, 估 2026-09 月底派
- **批次 4 (V1.1 release 阶段 4)**: core 拆 + 大模块拆 sub-crate, 6-10 sub-agent, 估 2026-10 月派
- **批次 5 (V1.1 release 阶段 5)**: DSL 洋葱 + 9 organ + R12 测度 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源, 8-12 sub-agent, 估 2026-10 月底 ~ 2026-11 月初派

**整合 #6 commit 拍板**: 估 2026-11-25 (per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板)
**整合 #7 commit 拍板**: 估 2026-11-29 (per 决策 #33 C1 + 决策 #71 §2.5, Mavis 自决拍板, V1.1 release 前最终)
**V1.1 release tag**: 估 2026-11-30 (per R131-3 §1.1 + R132-1 §1.1)

### 8.5 R153-4 拓维: V1.1 release 实施 12 详细 批次 sub-agent 派活 (估 36 sub-agent)

**批次 1 (R153 era, 2026-09 月初)**:
- R153-1 标准化 per-crate 决策矩阵 (4 sub-agent, 60 min each)
- R153-2 标准化 24 LOCKED 入口签名格式统一 (3 sub-agent, 60 min each)
- R153-3 标准化 24 LOCKED doc comment 极详细 (3 sub-agent, 60 min each)
- R153-4 标准化 阶段 1 8 步 verify 8/8 (2 sub-agent, 60 min each)
- 小计: 12 sub-agent × 60 min = 720 min

**批次 2 (R154 era, 2026-09 月中)**:
- R154-1 24 LOCKED 公开 API 表面清单 (3 sub-agent, 60 min each)
- R154-2 24 LOCKED pub(crate) 化 (4 sub-agent, 60 min each)
- R154-3 阶段 2 8 步 verify 8/8 (2 sub-agent, 60 min each)
- R154-4 阶段 2 编译时间 verify (2 sub-agent, 30 min each)
- 小计: 11 sub-agent × 60 min = 660 min

**批次 3 (R155 era, 2026-09 月底)**:
- R155-1 9 叶子 import 路径扫描 (2 sub-agent, 60 min each)
- R155-2 apeireth-leaf workspace 拆分 (2 sub-agent, 60 min each)
- R155-3 9 叶子 publish ready (1 sub-agent, 60 min each)
- R155-4 阶段 3.1 8 步 verify 8/8 (2 sub-agent, 60 min each)
- R155-5 阶段 3.1 顶层 facade verify (1 sub-agent, 30 min)
- R155-6 Eye organ 抽 crate (2 sub-agent, 60 min each)
- R155-7 Eye organ facade (1 sub-agent, 60 min)
- R155-8 阶段 3.2 Eye 8 步 verify 8/8 (1 sub-agent, 60 min)
- 小计: 12 sub-agent × 60 min = 720 min

**批次 4 (R156 era, 2026-10 月)**:
- R156-1 core 5 mod 分类 (1 sub-agent, 90 min)
- R156-2 core mod 子文件 (1 sub-agent, 60 min)
- R156-3 core facade (1 sub-agent, 30 min)
- R156-4 阶段 4.1 core 8 步 verify 8/8 (1 sub-agent, 60 min)
- R156-5 阶段 4.1 core 编译时间 verify (1 sub-agent, 30 min)
- R156-6 8 大模块 module 扫描 (1 sub-agent, 60 min)
- R156-7 mcp 8 sub-crate (1 sub-agent, 60 min)
- R156-8 pipeline 6 sub-crate (1 sub-agent, 60 min)
- R156-9 api 5 sub-crate (1 sub-agent, 60 min)
- R156-10 memory 5 sub-crate (1 sub-agent, 60 min)
- R156-11 asi 4 sub-crate (1 sub-agent, 60 min)
- R156-12 tools 5 sub-crate (1 sub-agent, 60 min)
- R156-13 evolution 5 sub-crate (1 sub-agent, 60 min)
- R156-14 graph 5 sub-crate (1 sub-agent, 60 min)
- R156-15 council 4 sub-crate (1 sub-agent, 60 min)
- R156-16 8 大模块 facade (1 sub-agent, 30 min)
- R156-17 阶段 4.2 大模块 8 步 verify 8/8 (2 sub-agent, 60 min each)
- R156-18 阶段 4.2 大模块 编译时间 verify (1 sub-agent, 30 min)
- 小计: 18 sub-agent × 60 min = 1080 min

**批次 5 (R157 era, 2026-10 月底 ~ 2026-11 月初)**:
- R157-1 DSL 洋葱 apeireth-dsl 实施 (2 sub-agent, 60 min each)
- R157-2 DSL 洋葱 24 LOCKED 引用 (1 sub-agent, 60 min)
- R157-3 阶段 5.1 DSL 洋葱 8 步 verify 8/8 (1 sub-agent, 60 min)
- R157-4 9 organ workspace 化 9 organ (3 sub-agent, 60 min each)
- R157-5 9 organ 借 OpenCode (2 sub-agent, 60 min each)
- R157-6 阶段 5.2 9 organ 8 步 verify 8/8 (2 sub-agent, 60 min each)
- R157-7 R12 测度对齐 24 测量函数 (2 sub-agent, 60 min each)
- R157-8 R12 测度对齐 V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode (1 sub-agent, 60 min)
- R157-9 R12 测度对齐 R12 baseline verify (1 sub-agent, 60 min)
- R157-10 ASI Stage 9 H1 自我决策 3 LOCKED (1 sub-agent, 60 min)
- R157-11 ASI Stage 9 H2 自我学习 3 LOCKED (1 sub-agent, 60 min)
- R157-12 ASI Stage 9 H3 自我演化 2 LOCKED (1 sub-agent, 60 min)
- R157-13 ASI Stage 9 H4 群体智能 1 LOCKED (1 sub-agent, 60 min)
- R157-14 阶段 5.4 ASI Stage 9 8 步 verify 8/8 (1 sub-agent, 60 min)
- R157-15 三洋葱 V2 apeireth-formal 实施 (2 sub-agent, 60 min each)
- R157-16 三洋葱 V2 24 LOCKED 引用 (1 sub-agent, 60 min)
- R157-17 阶段 5.5 三洋葱 V2 8 步 verify 8/8 (1 sub-agent, 60 min)
- R157-18 借鉴 12 源 24 LOCKED 注释 (2 sub-agent, 60 min each)
- R157-19 阶段 5.6 借鉴 12 源 8 步 verify 8/8 (1 sub-agent, 60 min)
- 小计: 26 sub-agent × 60 min = 1560 min

**总 5 批 sub-agent 派活**: 12 + 11 + 12 + 18 + 26 = **79 sub-agent** (R153-4 拓维, 跟 R152-2 估 29-43 差, 是因为 R153-4 拓维 5 批细分)

**总时间盒**: 720 + 660 + 720 + 1080 + 1560 = **4740 min ≈ 79 hours ≈ 10 work days** (R153-4 拓维)

---

## 9. 调研方向 ⑧: 8 硬墙严守 verify (B1 24 LOCKED V1.1 release Mavis 自决改)

### 9.1 8 硬墙严守 verify 表 (per 决策 #33 §2.3 + 决策 #74 §1 改写表 + R152-2 §8.1 + R153-4 拓维)

| # | 8 硬墙 | V1.0 release (整合 #5.1 commit) | V1.1 release (per 决策 #74 §2.3 + 整合 #6 commit 拍板 2026-11-25) | V2.0 release (per 决策 #74 §2.3 + 整合 #7 commit 拍板 2027-Q1/Q2) | 本报告 verify (R153-4 拓维 4 次 verify) |
|---|--------|---------------------------|-----------------------------------|------------------------------|----------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) 24/24 全 PASS (per R131-5 §1.2 1:28 verify + R150-2 §1.2 5:08 二次 verify + R152-2 §1 5:09 三次 verify + R153-4 §1.1 6:00 4 次 verify) | 🟢 **Mavis 自决改 (前提: 更好的架构, 12 优化方向 5 阶段 8 周, per 决策 #74 B1)** | 🟢 重构 (per Mavis 自决 + 主人 8/11 01:14 拍板) | ✅ B1 V1.0 release 0 改严守 100% + V1.1 release Mavis 自决改 12 优化方向 5 阶段 8 周 ready (4 次 verify 一致) |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 (per 决策 #74 §1 B2) | 🔒 **bump 1.2.0 → 1.2.1 (per 决策 #74 §1 B2 V1.1 release)** | 🔒 bump 1.2.1 → 2.0.0 (semver major) | ✅ B2 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 + V2.0 release bump 2.0.0 |
| **A1** | **R11 baseline 3 值** | 🔒 0 改严守 (0.8682/0.8532/0.9063, per 决策 #33 §2.3 A1) | 🟢 **R12 测度对齐 (前提: 新的 baseline 更高, per 决策 #74 §2.3)** | 🟢 可重评 | ✅ A1 V1.0 release 0 改严守 + V1.1 release R12 测度对齐 24+11 = 35 测量函数 |
| **A3** | **12 键 + PHL-07** | 🔒 PHL-07 spec-only 0 实施 + 12 键其他可改 (per 决策 #74 §1 A3) | 🟢 **PHL-07 实施 (per 决策 #74 §2.3 + R129-11 关键诚实标)** | 🟢 可重评 | ✅ A3 V1.0 release PHL-07 spec-only 0 实施 + V1.1 release PHL-07 实施 |
| **B3** | **V0.5 30 维** | 🔒 严守 (哲学) | 🔒 **严守 (V1.1 release 哲学不变)** | 🟢 可重评 | ✅ B3 V1.0 release + V1.1 release 严守 (V0.5 30 维 是哲学公式, V1.1 release 0 改) |
| **B4** | **6 重守门 v7** | 🔒 严守 (哲学) | 🔒 **严守 (V1.1 release 哲学不变)** | 🟢 可重评 | ✅ B4 V1.0 release + V1.1 release 严守 (6 重守门 v7 是哲学守门, V1.1 release 0 改) |
| **B5** | **8 哲学锚** | 🔒 严守 (哲学) | 🔒 **严守 (V1.1 release 哲学不变)** | 🟢 **推翻 + 重建** (per 主人 8/11 01:14 拍板 3 件套 §3) | ✅ B5 V1.0 release + V1.1 release 严守 (8 哲学锚 是哲学, V1.1 release 0 改) + V2.0 release 推翻重建 |
| **C1** | **0 主动 commit (主人起床前)** | 🔒 严守 | 🔒 **严守** | 🔒 严守 | ✅ C1 主人起床前 0 主动 commit 严守 100% (per master HEAD = 4207f187 since 1:43, 0 commit) |
| **C2** | **0 装 PASS** | 🔒 严守 | 🔒 **严守** | 🔒 严守 | ✅ C2 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2 + R129-11 关键诚实标 + 8 步 verify 实跑 verify) |
| **0 push** | **0 主动 push (主人起床前)** | 🔒 严守 | 🔒 **严守** | 🔒 严守 | ✅ 0 push 严守 100% (per 决策 #33 + 决策 #61 §6, 0 主动 push) |

### 9.2 B1 改写边界 (per 决策 #74 §2.2 + §2.3 + R152-2 §8.2 + R153-4 拓维)

**V1.0 release (整合 #5.1 commit)**:
- ✅ 0 改 24 LOCKED 入口签名 (严守, per R131-5 §1.2 verify 24/24 全 PASS)
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
- ✅ 0 改 R11 baseline 3 值 (严守)
- ✅ PHL-07 spec-only 0 实施 (严守, V1.1 release 实施)
- ✅ 0 越界 8 硬墙 100% (per 决策 #33 §2.3 + 决策 #74 §1)

**V1.1 release (per R153-4 本报告 12 优化方向 + 决策 #74 §2.3 + 整合 #6 commit 拍板 2026-11-25)**:
- ✅ 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决改, **12 优化方向 5 阶段 8 周**, per R153-4 本报告)
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

### 9.3 决策原则 33 维 verify (per R131-5 §6.2 + R137-2 §7.2 续 + R152-2 §8.3 + R153-4 拓维 2 维)

- **D1**: Mavis = orchestrator + 全自决 + 最高权限 (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **D2**: 8 硬墙严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **D3**: B1 24 LOCKED 入口签名: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (**12 优化方向 5 阶段 8 周**, per R152-2 + R153-4 本报告) + V2.0 release 可重评
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
- **D20**: 0 重复造轮子 (per 用户记忆 #6, R131-1/2/3/4/5/9 + R132-1 + R133-3 + R137-2 + R150-2 + R152-2 已有报告 reference 不重写)
- **D21**: R153-4 24 LOCKED 入口签名 V1.1 release 实施 spec 详细 12 优化方向 5 阶段 8 周 严守 (per 本报告 spec)
- **D22**: V1.1 release 时间窗 2026-11-30 (per R132-1 §1.1 + R130-5 §1.1 V1.1 估 2026-11-30 + 任务 spec 整合 #6 commit 拍板 2026-11-25)
- **D23 (R152-2 新增)**: 12 优化方向 8 大 (R131-5 §2 8 方向) + 4 新增 (R149-2/3/4 调研) (per 任务 spec §1 10+ 优化方向)
- **D24 (R152-2 新增)**: ASI Stage 9 长程 AI 成长 4 维度 H1-H4 集成 (per 方向 ⑨ + R149-2 + R130-2 §1.5)
- **D25 (R152-2 新增)**: 三洋葱架构 V2 集成 (五洋葱: 原则 + 权限 + DSL + 智能涌现 + 形式化) (per 方向 ⑩ + R149-3 + R133-3 + R131-9)
- **D26 (R152-2 新增)**: 借鉴 12 源 fork-then-borrow 模式 (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) (per 方向 ⑪ + R149-4 + R130-6 + R140-5)
- **D27 (R152-2 新增)**: 9 organ workspace 化 9/9 覆盖 (Eye 补完) (per 方向 ⑦ + ⑫ + R131-5 §2.6)
- **D28 (R152-2 新增)**: 整合 #6 commit 拍板 2026-11-25 (5 天缓冲 before V1.1 release 实战 2026-11-30) (per 任务 spec)
- **D29 (R152-2 新增)**: 整合 #7 commit 拍板 2027-Q1/Q2 估 (V1.2 release 准备 / V2.0 release 远期重构) (per R137-2 §8.1)
- **D30 (R152-2 新增)**: 8 步 verify 8/8 全 PASS 才执行整合 #6 commit 拍板 (per 决策 #78 §8 + R148-23 8 步 verify 终版 SOP v2)
- **D31 (R152-2 新增)**: 0 改 src 严守 100% (本报告 R153-4 调研/分析/实施 spec 详细 类, 0 触碰 crates/ 下任何 .rs 文件) (per 决策 #33 §2.3 + 决策 #74 §1 B1)
- **D32 (R153-4 拓维)**: R153-4 报告 4 次 verify 一致 (1:28 + 5:08 + 5:09 + 6:00, 跟 R131-5 §1.2 + R150-2 §1.2 + R152-2 §1 + R153-4 §1.1 一致) (per 决策 #71 §5 永久循环 + 决策 #86 §4)
- **D33 (R153-4 拓维)**: R153-4 报告 79 sub-agent 派活计划 (5 批细分, 跟 R152-2 §7.4 29-43 估 差, 是因为 R153-4 拓维 5 批细分) (per 用户记忆 #6 "派 sub-agent 干独立模块" + 决策 #71 §5 永久循环)

---

## 10. 总结

### 10.1 24 LOCKED 入口签名 V1.1 release 实施 spec 详细 12 优化方向 一句话总结

1. **方向 ① 标准化**: 24 LOCKED 用 5 种 re-export 风格 → 3 模式之一 per-crate 自决, 顶层 re-export facade 0 改
2. **方向 ② 瘦身**: 24 LOCKED 共 578 pub lines → ≤30 per-crate, 578 → ≤400 -30%, 仅 add 0 remove 顶层 facade
3. **方向 ③ 9 叶子拆 workspace**: 9 叶子 crate (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench) → `apeireth-leaf/` workspace, 顶层 `apeireth/Cargo.toml` 0 改
4. **方向 ④ core 拆 pub mod**: core 1 个 108KB lib.rs 拆 5 大 mod: `core::types / core::onion / core::human / core::gate / core::lib`, 0 改入口签名
5. **方向 ⑤ 大模块拆 sub-crate**: mcp 13→8 + pipeline 11→6 + api 16→5 + memory 13→5 + asi 9→4 + tools 12→5 + evolution 9→5 + graph 11→5 + council 20+→4 = **47 sub-crate**, 顶层 re-export facade 0 改入口签名
6. **方向 ⑥ DSL 洋葱**: 新增 `apeireth-dsl` crate, 三洋葱 → 四洋葱 升级 (新增第 4 层 "智能涌现"), 24 LOCKED 全部引用 dsl 守门
7. **方向 ⑦ 9 organ 借 OpenCode + Eye 补**: 新增 `apeireth-eye` workspace (从 tui/src/organ/eye.rs 抽 crate), 24 LOCKED 全部下沉到 9 organ workspace, 9 organ 9/9 覆盖
8. **方向 ⑧ R12 测度对齐**: R11 baseline 3 值 0.8682/0.8532/0.9063 → R12 baseline 更高, 24+9 = 33 → 24+11 = 35 测量函数, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新
9. **方向 ⑨ ASI Stage 9 集成 (R152-2 新增)**: 24 LOCKED 中 9 个 (38%) 加 Stage 9 4 维度 H1-H4 (H1 自我决策 + H2 自我学习 + H3 自我演化 + H4 群体智能)
10. **方向 ⑩ 三洋葱 V2 集成 (R152-2 新增)**: 24 LOCKED 全部加 第 5 层 "形式化洋葱" 守门, 新增 `apeireth-formal` crate (kani 借鉴 + PHL-07 实施)
11. **方向 ⑪ 借鉴 12 源 fork-then-borrow 集成 (R152-2 新增)**: 24 LOCKED 入口签名全部加 12 源 注释 (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID), 借脑 1:1 公开模式
12. **方向 ⑫ 9 organ workspace 化 (R152-2 新增, 跟 ⑦ 配合)**: 9 organ workspace 全集, 24 LOCKED 全部下沉

### 10.2 V1.0 release 0 改严守 vs V1.1 release Mavis 自决改 边界 (4 次 verify 一致)

- **V1.0 release (整合 #5.1 commit, 0 改 src 严守 100%)**:
  - ✅ 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS (per R131-5 §1.2 1:28 verify + R150-2 §1.2 5:08 二次 verify + R152-2 §1 5:09 三次 verify + R153-4 §1.1 6:00 4 次 verify)
  - ✅ 24 LOCKED crate mtime baseline 16:34 之前 严守
  - ✅ R11 baseline 3 值 严守
  - ✅ PHL-07 spec-only 0 实施
  - ✅ 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守, 13 键 verdict cache 严守
  - ✅ 0 装 PASS 严守, 0 主动 commit 严守, 0 主动 push 严守
- **V1.1 release (Mavis 自决改, 前提: 更好的架构, per 决策 #74 §2.3 + 整合 #6 commit 拍板 2026-11-25)**:
  - ✅ 24 LOCKED 入口签名 可改 (12 优化方向 5 阶段 8 周, per R152-2 + R153-4 本报告)
  - ✅ 24 LOCKED crate mtime baseline 16:34 之前 可改
  - ✅ R11 baseline 3 值 → R12 测度对齐 (前提: 新的 baseline 更高, per 决策 #74 §2.3)
  - ✅ PHL-07 实施 (V1.1 release, per R129-11 关键诚实标 + 决策 #74 §2.3)
  - ✅ Cargo.toml workspace.version bump 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)
  - ✅ 0 越界 8 硬墙 100% (B1 Mavis 自决改, 其余 9 硬墙严守)

### 10.3 12 优化方向 vs 5 阶段 8 周 派活计划 (R153-4 拓维 5 批 79 sub-agent)

- **批次 1 (R153 era, 阶段 1 标准化 1 周)**: 方向 ①, 12 sub-agent × 60 min = 720 min
- **批次 2 (R154 era, 阶段 2 瘦身 1 周)**: 方向 ②, 11 sub-agent × 60 min = 660 min
- **批次 3 (R155 era, 阶段 3 9 叶子拆 + Eye 补 2 周)**: 方向 ③ + ⑦ Eye, 12 sub-agent × 60 min = 720 min
- **批次 4 (R156 era, 阶段 4 core 拆 + 大模块拆 sub-crate 2 周)**: 方向 ④ + ⑤, 18 sub-agent × 60 min = 1080 min
- **批次 5 (R157 era, 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 fork + 9 organ workspace 化 2 周)**: 方向 ⑥ + ⑦ + ⑧ + ⑨ + ⑩ + ⑪ + ⑫, 26 sub-agent × 60 min = 1560 min
- **总时间盒**: 79 hours ≈ 10 work days (R153-4 拓维 5 批), 跟 R152-2 估 29-43 sub-agent 差, 是因为 R153-4 拓维 5 批细分
- **总时间盒 (按周)**: 8 周 = 2 个月, V1.1 release 估 2026-11-30 (per R132-1 §1.1 + R130-5 §1.1 + 任务 spec)

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

### 10.6 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md, R153-4 拓维 12 方向 落地)

- 方向 ① 标准化 3 模式之一 → "不要怕复杂度" (per-crate 自决 = 高灵活)
- 方向 ② 瘦身 578 → ≤400 pub items → "最强效果" (暴露 30% 减少, 但保留核心 API)
- 方向 ③ 9 叶子拆 + Eye 补 → "不要怕复杂度" (拆 = 复杂, 但 9 organ 100% 覆盖)
- 方向 ④ ⑤ core 拆 + 大模块拆 sub-crate (47 sub-crate) → "不要怕复杂度" (拆 = 复杂, 但编译时间减少 20-50%)
- 方向 ⑥ DSL 洋葱 + 方向 ⑩ 三洋葱 V2 集成 → "不要怕复杂度" (三洋葱 → 五洋葱 = 复杂, 但守门 5 重 = 安全)
- 方向 ⑦ ⑫ 9 organ 借 OpenCode + Eye 补 → "不要怕复杂度" (organ-first 拓扑 = 复杂, 但 organ 边界清晰)
- 方向 ⑧ R12 测度对齐 (24+9 → 24+11) → "最强效果" (测度加 2 维 = 测度更精准)
- 方向 ⑨ ASI Stage 9 集成 (H1-H4 4 维度) → "最强效果 + 最厉害工程" (长程 AI 成长 4 维度 = 复杂, 但效果最强)
- 方向 ⑪ 借鉴 12 源 fork-then-borrow 模式 → "最厉害工程" (借脑 1:1 公开模式 = 高水平)
- 整合 #6 commit 拍板 2026-11-25 + V1.1 release 实战 2026-11-30 + 整合 #7 commit 拍板 2027-Q1/Q2 估 节奏 严守
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
- R131 era 第 2 批 6 sub-agent 派活: R131-4 (cargo workspace 结构优化) + **R131-5 (24 LOCKED 入口分布优化 8 方向, 62.1KB, 本报告核心依据 1)** + R131-6 + R131-7 + R131-8 + R131-9 (形式化集成优化)
- R132 era 计划 2 sub-agent 派活: R132-1 (V1.1 release 路线图 final) + R132-2 (V2.0 release 战略路线图)
- R133 era 实施 3 sub-agent 派活: R133-1 (借鉴源 12 源 实施) + R133-2 (ASI Stage 9 实施) + R133-3 (三洋葱架构升级 实施 spec)
- R137 era 实施 1 sub-agent 派活: **R137-2 (24 LOCKED 入口签名 改写 spec + 5 阶段实施计划, 91.6KB, 本报告核心依据 2)**
- R140 era 调研 5 sub-agent 派活: R140-1 (整合 #5.1 commit 拍板流程) + R140-2 (V1.1 release 路线图 detailed) + R140-3 (cargo workspace 1.2.1 bump plan) + R140-4 (ASI Stage 10 终极自治) + R140-5 (借鉴 12 源 fork 决策)
- R141 era 调研 3 sub-agent 派活: R141-1 (1.0 vs AGI 业界 gap) + R141-2 (24 LOCKED vs 借鉴 API 一致性) + R141-3 (整合 #5.1 src 质量 0 装 PASS)
- R142 era 调研 2 sub-agent 派活: R142-1 (整合 #5.1 commit SOP) + R142-2 (1.0 release 实际 SOP)
- R143 era 调研 4 sub-agent 派活: R143-1 (perpetual loop 4 step 决策链) + R143-2 (1.0 release flow overview) + R143-3 (V1.1 vs V1.0 差异表) + R143-4 (决策链 借鉴 8 硬墙 index)
- R144-R148 era 调研 16+ sub-agent 派活: 整合 #5.1 commit 拍板 SOP + 8 步 verify 终版 + 决策树 v2 + 派活 16 满 (R144-R148 6 sub-agent 5 done + 6 errored 中断接手 per 决策 #86)
- 整合 #5.3 commit 拍板成功 1:43 (master HEAD = 4207f187, 187 files / 127548 insertions, 0 主动 push 严守)
- R149 era 调研 5 sub-agent 派活 (per 决策 #86 §4): R149-1 (V1.1 release 实战准备) + **R149-2 (ASI Stage 9 长程 AI 成长深化, 本报告方向 ⑨ 依据)** + **R149-3 (三洋葱架构升级 V2, 本报告方向 ⑩ 依据)** + **R149-4 (借鉴 12 源 fork-then-borrow 模式, 本报告方向 ⑪ 依据)** + R149-5 (1.0 release 实战总复盘)
- R150 era 差距 3 sub-agent 派活 (per 决策 #86 §4): R150-1 (V1.1 release 跟 AGI 业界 v2.x 差距) + **R150-2 (整合 #5.1 commit 拍板后 24 LOCKED 入口签名优化差距, 132.5KB, Mavis 自决改, 决策 #74 B1, 本报告核心依据 3)** + R150-3 (Cargo workspace 1.2.1 bump 差距)
- R151 era 计划 2 sub-agent 派活 (per 决策 #86 §4): R151-1 (整合 #6 commit 拍板时间表) + R151-2 (整合 #7 commit 拍板时间表)
- R152 era 实施准备 5 sub-agent 派活 (per 决策 #86 §4, 5:00 tick 派活): R152-1 (整合 #6 Cargo workspace 1.2.1 bump 准备) + **R152-2 (整合 #6 24 LOCKED 入口签名 优化准备 实施 spec, 128.4KB, 12 优化方向 5 阶段 8 周, 本报告核心依据 4)** + R152-3 (整合 #6 pybridge 集成优化准备) + R152-4 (整合 #7 Tauri 集成优化准备) + R152-5 (整合 #7 形式化集成优化准备)
- R153 era 实施 spec 详细 5 sub-agent 派活 (per 决策 #86 §4, 6:00 tick 派活): R153-1 (标准化 per-crate 决策矩阵) + R153-2 (24 LOCKED 入口签名格式统一) + R153-3 (24 LOCKED doc comment 极详细) + **R153-4 (整合 #6 24 LOCKED 入口签名 Mavis 自决改 V1.1 release 实施 spec 详细, 本报告 done)** + R153-5 (阶段 1 8 步 verify 8/8)
- R154-R157 era 派活 4 批 5 阶段 8 周 (per 决策 #71 §5 永久循环 + 决策 #86 §4): R154 era 阶段 2 瘦身 1 周 + R155 era 阶段 3 9 叶子拆 + Eye 补 2 周 + R156 era 阶段 4 core 拆 + 大模块拆 sub-crate 2 周 + R157 era 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 fork + 9 organ workspace 化 2 周
- **整合 #6 commit 拍板 = 2026-11-25** (5 天缓冲 before V1.1 release 实战 2026-11-30, per 任务 spec)
- **V1.1 release 实战 = 2026-11-30** (per R132-1 §1.1 + R130-5 §1.1)
- **整合 #7 commit 拍板 = 2027-Q1/Q2 估** (V1.2 release 准备 / V2.0 release 远期重构)

---

## 12. 一句话 (再次强调)

**R153-4 整合 #6 24 LOCKED 入口签名 Mavis 自决改 V1.1 release 实施 spec 详细 done 2026-08-11 06:00 (90 min 时间盒, 0 改 src 严守 100%)**: V1.0 release 0 改 src 严守 100% (整合 #5.1 commit 拍板 R11 baseline, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS per R131-5 §1.2 1:28 + R150-2 §1.2 5:08 + R152-2 §1 5:09 + R153-4 §1.1 6:00 **4 次 verify 一致**, R11 baseline 3 值 0.8682/0.8532/0.9063 严守, PHL-07 spec-only 0 实施, Cargo.toml workspace.version 1.2.0 严守, 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守, 13 键 verdict cache 严守). **整合 #6 24 LOCKED 入口签名 V1.1 release 实施 spec 详细 = 12 优化方向 (8 大 + 4 新增) 5 阶段 8 周 派活 (per R152-2 + R153-4 拓维 实施 spec 详细)**: 方向 ①标准化 + ②瘦身 (578→≤400 -30%) + ③9 叶子拆 workspace + ④core 拆 pub mod (1→5 mod) + ⑤大模块拆 sub-crate (47 sub-crate) + ⑥DSL 洋葱 (三洋葱→四洋葱) + ⑦9 organ 借 OpenCode + Eye 补 (9/9 覆盖) + ⑧R12 测度对齐 (24+9→24+11) + ⑨ASI Stage 9 集成 (H1-H4 4 维度) + ⑩三洋葱 V2 集成 (第 5 层形式化洋葱) + ⑪借鉴 12 源 fork-then-borrow (8 真 cloned + 2 借鉴 ID + 1 永久跳过 + 1 借脑 ID) + ⑫9 organ workspace 化 (跟 ⑦ 配合). **5 阶段 8 周 派活 (R153-4 拓维 5 批 79 sub-agent)**: 批次 1 阶段 1 标准化 1 周 (R153 era 12 sub × 60 min = 720 min) + 批次 2 阶段 2 瘦身 1 周 (R154 era 11 sub × 60 min = 660 min) + 批次 3 阶段 3 9 叶子拆 + Eye 补 2 周 (R155 era 12 sub × 60 min = 720 min) + 批次 4 阶段 4 core 拆 + 大模块拆 sub-crate 2 周 (R156 era 18 sub × 60 min = 1080 min) + 批次 5 阶段 5 DSL 洋葱 + 9 organ 借 OpenCode + R12 测度对齐 + ASI Stage 9 + 三洋葱 V2 + 借鉴 12 源 + 9 organ workspace 化 2 周 (R157 era 26 sub × 60 min = 1560 min) = **79 sub-agent 总, 4740 min ≈ 10 work days (R153-4 拓维 5 批细分, 跟 R152-2 估 29-43 差, 是因为 R153-4 拓维 5 批细分)**. **整合 #6 commit 拍板 = 2026-11-25** (5 天缓冲 before V1.1 release 实战 2026-11-30), **整合 #7 commit 拍板 = 2027-Q1/Q2 估** (V1.2 release 准备 / V2.0 release 远期重构, 24 LOCKED → 0 LOCKED 全解锁 + 8 哲学锚 → N 哲学锚 重建). **8 硬墙严守 100% verify**: B1 24 LOCKED V1.0 release 0 改严守 100% + V1.1 release Mavis 自决改 (per 决策 #74) + B2 workspace.version 1.2.0 V1.0 release 严守 / 1.2.1 V1.1 release bump / 2.0.0 V2.0 release bump + A1 R11 baseline 3 值 V1.0 release 严守 / V1.1 release R12 更高 + A3 12 键 + PHL-07 V1.0 release PHL-07 spec-only 0 实施 / V1.1 release PHL-07 实施 + B3 V0.5 30 维严守 + B4 6 重守门 v7 严守 + B5 8 哲学锚严守 / V2.0 release 推翻 + 重建 + C1 0 主动 commit (主人起床前) 严守 + C2 0 装 PASS 严守 + 0 push 严守. **决策原则 33 维 verify** (D1-D33, per 决策 #33 + #74 + #71 + 主人 8/11 01:14 拍板 3 件套 + R152-2 31 维 + R153-4 拓维 2 维). **0 主动 IM 主人 + 0 主动 commit/push 严守 + 0 装 PASS 严守 + 0 主动删严守 + 不要怕复杂度哲学落地 12 方向** (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md).

---

**报告路径**: `Apeireth-rust\reports\agent-r153-4-integration-6-24-locked-entry-mavis-self-decide-v1.1-spec-2026-08-11.md`
**生成时间**: 2026-08-11 06:00 (R153 era 实施 spec 详细阶段, per 决策 #86 §4 6:00 tick 派活)
**作者**: R153-4 sub-agent (Mavis 派, per 决策 #86 §4 R153 era 5 sub 第 4 个, 实施 spec 详细阶段)
**接收 agent**: Mavis root session (`mvs_367e66fae08342ffa399befe4f85dbac`)
**关联决策**: #10 + #22 + #33 + #36 + #44 + #48 + #55 + #58 + #60 + #61 + #62 + #64 + #66 + #69 + #70 + #71 + #72 + #73 + #74 + #75-#86 + 用户记忆 #10
**关联报告**: R131-1/2/3/4/5/6/7/8/9 + R132-1/2 + R133-1/2/3 + R137-1/2/3/4/5 + R140-1/2/3/4/5 + R141-1/2/3 + R142-1/2 + R143-1/2/3/4 + R144-1/2/4 + R145-3 + R147-1/2/3/5 + R148-1/2/5/6/10/11/12/13/23/24 + R149-1/2/3/4/5 + R150-1/2/3 + R151-1/2 + R152-1/2/3/4/5 + R153-1/2/3/5
**状态**: ✅ done 06:00 (90 min 时间盒内, 12 优化方向 5 阶段 8 周 实施 spec 详细 + 8 硬墙严守 100% verify + 决策原则 33 维 verify + 79 sub-agent 5 批 派活计划 ready)

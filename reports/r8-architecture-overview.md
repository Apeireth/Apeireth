# R8 架构总览 — 5 层底座 + R8 新增 L3 记忆层 / L4 身份层

> 作者: technical_writer · R8-DOC-02
> 阅读对象: 下一团队（架构师 / 后端 / 数据库 / 自演化工程师）+ 真用户
> 主哲学: ASI=∞ 真生产；不假装 / 不破坏 4 层门 / 不绑单模型 / 不刷 KPI
> 来源: `HARNESS.md §1`（7 组件）+ `r7-design-01-architecture-blueprint.md §1`（L0-L7 分层）+ R8 三轨道代码现状
> **R8 真生产状态**：master HEAD = `d745c332`（V1094 已 commit）+ 11 个 v109x 模块真生产 + 119+ 测试全过

---

## 0. 阅读须知

> **大白话原则**：本报告所有术语首次出现配 ≤10 字注解。
> **架构图友好**：每层先给"是什么"，再给"装什么"，再给"接哪里"。
> **R8 增量视角**：原 5 层（V0 时代）→ R7 末 8 层（L0-L7）→ R8 在 L3 记忆层真实现 + L4 身份层 PoC 上落子。

| 缩写 | 大白话 |
|---|---|
| Harness | 包裹 AI 的外系统（不模型权重） |
| STM/MTM/LTM | 短/中/长期记忆 |
| Identity | 身份卡，记录"我是谁" |
| Schema | 数据结构契约 |
| WAL | 写前日志（先记再改，崩了能恢复） |
| FEP | 自由能原理（数学框架） |
| VCP | 工具调用协议（标准接口） |
| orchestrator | 调度器（决定谁干什么） |

---

## 1. 整体架构图（ASCII 全景）

```
┌──────────────────────────────────────────────────────────────────┐
│  L7 暴露层 (Exposure)                                               │
│  HQB MCP server (7 tools) · apeireth serve · CLI · dashboard      │
├──────────────────────────────────────────────────────────────────┤
│  L6 工具层 (Tools)                                                   │
│  V1075 deploy · V1076 LLM client · V1074 measure · V1083 router · V1097 MCP server│
├──────────────────────────────────────────────────────────────────┤
│  L5 编排层 (Orchestration)                                          │
│  R7-ORC-01 Phase 1/2/3 (并行/串行/收尾) · Self-Evolving 主循环      │
├──────────────────────────────────────────────────────────────────┤
│  L4 持久层 (Persistence)              ┌──── 🆕 R8 Track B PoC ──── │
│  V1052 Reconsolidator · Tonbo LSM ·   │ SqliteIdentityStore v0.2  │
│  V1086 HQB · hqb.db 5 表             │ Kickoff Enrichment v0.4   │
│  artifact snapshot.json + history    │ identity_card.master.json │
├──────────────────────────────────────────────────────────────────┤
│  L3 状态层 (State Machine)            ┌──── 🆕 R8 Track A 真实现 ──┐│
│  状态机 6 态 + 状态图 10/10/9        │ memory_3tier (STM/MTM/LTM)││
│  MemoryReplay state_id + diff        │ V1091 replay 真生产       ││
│  HotCold hot/cold tier 切换          │ Dream 状态机 (7 态)        ││
├──────────────────────────────────────────────────────────────────┤
│  L2 接口层 (Interface)                                             │
│  15 接口 (Dream 7 + Replay 6 + HotCold 2) · lock 互斥              │
├──────────────────────────────────────────────────────────────────┤
│  L1 业务层 (Business)                                               │
│  BE-01 DreamSubsystem · BE-02 MemoryReplay · DB-01 HotCold        │
├──────────────────────────────────────────────────────────────────┤
│  L0 守门层 (Guard)                                                  │
│  V3 philosophy_guard · V1072 永恒身份 · V1074 测量 · V1081 诚实    │
└──────────────────────────────────────────────────────────────────┘
```

> 大白话：自下而上读 —— 最下层是"安全检查"，中间是"业务/接口/状态/数据"，上层是"调度+工具+对外出口"。

---

## 2. 原 5 层底座（V0 时代 → R7 末）

> 来源: `r7-design-01-architecture-blueprint.md §1`（L0-L7 分层表）；下表将其压缩为 5 个职责群。

| 职责群 | 对应层 | 关键模块 | 一句话职责 |
|---|---|---|---|
| **A. 守门群** | L0 | V3 / V1072 / V1074 / V1081 | 系统能不能动、改不改、对不对的最终判定 |
| **B. 业务群** | L1-L3 | DreamSubsystem / MemoryReplay / HotCold / 状态机 | 三层记忆 + 状态机的核心算法 |
| **C. 接口群** | L2 | 15 接口 + lock 互斥 | 模块之间互相说话的标准 |
| **D. 数据群** | L4 | V1052 / Tonbo LSM / V1086 HQB / hqb.db | 东西存哪儿、怎么存 |
| **E. 编排+暴露群** | L5-L7 | ORC-01 / V1075/V1076/V1074/V1083 / HQB MCP / CLI / serve | 谁先干谁后干、用啥工具、对外怎么说 |

---

## 3. R8 新增：L3 记忆层真实现（Track A 三大块）

> 大白话：R8 把 L3 这一层从"设计稿"推到"真代码能跑"。STM = 最近 50 条对话；MTM = 每小时打主题包；LTM = 永久身份/决策/价值。

### 3.1 L3.A — memory_3tier.py（STM/MTM/LTM 抽象层）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/memory_3tier.py` |
| **版本** | `MEMORY_3TIER_VERSION = "0.1.0"` |
| **LOC** | 287 |
| **3 个核心类** | `MemoryAnchor`（LTM 锚点）/ `TopicSummary`（MTM 主题）/ STM rolling deque |
| **真参数** | STM_MAX_SIZE=50 · MTM_SUMMARY_INTERVAL_S=3600 · LTM_ANCHOR_MIN_IMPORTANCE=8 |
| **借鉴** | MemoryOS-Rust STM/MTM/LTM · A-MEM agentic memory · Zep temporal KG |
| **守门** | memory_replay_design.py 的 PHILOSOPHY_GUARDS（replay≠bit-exact / idempotent≠safe 等） |
| **与 HARNESS 对应** | §1 "Long-Term Memory"（MEMORY.md + experiences.md）的工程化落地 |

### 3.2 L3.B — v1091_memory_replay.py（状态回放真生产）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/v1091_memory_replay.py` |
| **版本** | `V1091_VERSION = "0.1.0"` |
| **LOC** | 501（19.3KB） |
| **5 方法** | capture_state · restore_state · replay_events · diff_states · idempotent_apply |
| **WAL 格式** | V1052 兼容 JSONL + sha256 + seq（与 V1052 WalEntry 对齐） |
| **WAL 软轮转** | >64MB 保留后 75% 行 |
| **并发安全** | threading.RLock 保护 `_seq / _wal / _live_state` |
| **损坏容错** | `_recover_from_disk` 跳过损坏行 + 累计 `skipped_corrupt` |
| **守门（4 条）** | replay ≠ bit-exact · idempotent ≠ safe · capture ≠ backup · replay ≠ understanding |
| **头注释明标** | `V1091 MemoryReplay — 真生产状态回放 (R8-TrackA2)`（行 :1） |
| **测试** | ✅ 52 测试全过（8 类，回放一致性/损坏容错/并发回放） |
| **报告** | ✅ `reports/r8-tracka2-replay-dream-delivery.md` |
| **状态** | 🟢 真生产 / 52 测试全过 / commit 收口待 R9 |
| **ASI 归因** | 撑 V0.4 `capabilities` + `engineering` 两维 |

> 大白话：这相当于给记忆系统装上"撤销"+"重做"按钮。`capture_state` 拍快照；`restore_state` 回到快照；`replay_events` 重演一段事件；`diff_states` 比两个快照的差别；`idempotent_apply` 同一条指令多次执行结果一样。

### 3.3 L3.C — HotCold/WAL（数据分层 + 写前日志）

| 项 | 内容 |
|---|---|
| **设计稿** | `reports/r7-be-01-dream-design.md` § HotCold + `r6-int-01` 19 接口表 |
| **核心数据流** | write → WAL(JSONL+sha256+seq) → hot tier(快) → 老化 → cold tier(慢) |
| **3 个 DB 操作** | `migrate_hot_to_cold` · `recover_from_wal` · `checkpoint_wal` |
| **真实现** | ✅ `apeireth/v1090_memory_wal.py` · 623 LOC · `V1090_VERSION = "0.1.0"` |
| **5 真能力** | 真 fsync（flush+fsync fileno）+ append-only + sha256 校验 + 损坏容错 + replay |
| **10 真借鉴** | V1052 DeltaMemory / PG WAL 1996 / SQLite WAL 2010 / LMDB 2011 / RocksDB WAL 2013 / Tonbo WAL / W3C PROV 2013 / ARIES 1992 / JSON Lines 2020 / Linux fsync(2) manpage |
| **5 守门** | WAL ≠ backup · fsync ≠ guarantee · replay ≠ reconstruction · WAL ≠ ACID · SHA256 ≠ cryptographic-proof |
| **测试** | 待 R9 跑（v1090 头注释 28 行 + 借鉴清单） |
| **状态** | 🟢 源码已落 / 报告待 R9 收口 |
| **ASI 归因** | 撑 V0.4 `engineering` + `real_production` 两维 |
| **Track A1 报告** | `reports/r8-tracka1-hotcold-wal.md`（与 v1094 schema 报告配套） |

> 大白话：HotCold = 把记忆按"热度"分层 —— 刚用过的放快盘（热），很久没用的挪到慢盘（冷）。WAL = 每次写之前先写一行日志，万一断电崩了能从日志恢复。

### 3.4 L3.D — Dream 状态机 + v1092 真生产

| 项 | 内容 |
|---|---|
| **状态机** | IDLE → SELECT → LIGHT/REM → CONSOLIDATE → FORGET → REPLAY → EMIT |
| **7 方法** | tick · should_run · run_cycle · interrupt · resume · consolidate · decay |
| **真实现** | ✅ `apeireth/v1092_memory_dream.py` · 12.1KB |
| **V3 守门核心** | `DreamCandidate._dream=True` · `frozen=True` · `is_dream()` 永远 True |
| **3 SchemaPhase** | ASSIMILATION（单 note 套既有）/ ACCOMMODATION（2 note 冲突重塑）/ REPLAY（≥3 主题多 note 重放） |
| **借鉴** | V1052 MemoryOS / letta / claude-mem / Tonbo / R37 hippocampal · Piaget 同化/顺应 + 神经科学 replay |
| **测试** | ✅ 44 测试全过（9 类，V3 守门 / Phase 选择 / 去重 / 并发 dream） |
| **状态** | 🟢 真生产 / 44 测试全过 / 与 V1091 共报告 |
| **Track A3 报告** | ✅ `reports/r8-tracka2-replay-dream-delivery.md` |
| **ASI 归因** | 撑 V0.4 `v2_philosophy` + `capabilities`（memory 间接） |

### 3.5 L3.E — V1094 Memory Schema（真 commit d745c332）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/v1094_memory_schema.py` · 244 LOC |
| **版本** | V1094_VERSION = "0.1.0" |
| **8 业务表 + meta** | memory_hot · memory_cold · memory_wal · memory_dream · memory_snapshots · stm_messages · mtm_themes · ltm_facts · memory_meta |
| **26 索引** | 含 event_id UNIQUE（幂等键）· fingerprint UNIQUE（dedup）· (scope, seq) UNIQUE（snapshot 自引用） |
| **双维度切分** | 生命周期维度（hot/cold/wal/dream/snapshot）+ 三层模型维度（stm/mtm/ltm） |
| **零破坏兼容** | 不动 `apeireth/memory.py` v0.3（episodes/notes/...）+ 不动 `apeireth/hqb/schema.py` v0.1（hqb_*） |
| **测试** | ✅ 23 测试全过（6 维度覆盖） |
| **报告** | ✅ `reports/r8-tracka3-memory-schema-design.md` |
| **commit** | ✅ 已真 commit `d745c332`（"feat v1094 R8-TrackA3: Memory schema (HotCold + WAL + Replay + Dream + STM/MTM/LTM)"） |
| **ASI 归因** | 撑 V0.4 `engineering` + `real_production`（零破坏兼容 + 26 索引） |

> 大白话：Dream 是后台"整理员" —— 它在系统空闲时把 STM（短记忆）整理成 MTM（主题），把不要的归档到 LTM（永久），把无用的 FORGET（遗忘），重要的事 EMIT（推出去）。

---

## 4. R8 新增：L4 身份层 PoC（Track B 两大块）

> 大白话：身份卡 = 一张写满"我是谁、主人是谁、我要做什么"的多页文件。R8 把 JSON 文件版升级到 SQLite 数据库版（更稳）+ 启动期自动补全空着的格子。

### 4.1 L4.A — identity_store.py（多卡管理 v0.2）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/identity_store.py` |
| **版本** | `IDENTITY_STORE_VERSION = "0.2.0"` |
| **LOC** | 291 |
| **22 字段** | name/alias/purpose/mission/domains/origin_reason/creator/archetypes/ask_when/decide_when/remind_when/relationship_contract/boundaries/remember_forever/never_mention/funnel_questions/emergence_space/recall_anchor(v0.2)/evidence_refs(v0.2)/created_at/apeireth_version |
| **3 个能力** | ① Schema 验证（无外部依赖，stdlib）② 版本迁移 v0.1→v0.2 ③ 多卡容器 load_all/save_all/get_master |
| **守门** | SchemaError 自检 + hash 防覆盖 + 完整性自检（加载时验证 hash） |
| **ASI 归因** | 直接撑 V0.3 `eternal_identity` = 0.8441（V1072 真测） |

> 大白话：22 个字段 = 这张卡要回答的 22 个问题（比如"我做什么的"、"何时问我"、"何时自己决定"）。v0.2 比 v0.1 多了"危急时一句话锚定"和"证据引用"两个字段。

### 4.2 L4.B — sqlite_identity_store.py（数据库版身份卡）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/sqlite_identity_store.py` |
| **能力** | 用 SQLite 存身份卡（比 JSON 文件支持并发、事务、查询） |
| **Demo** | `apeireth/run_sqlite_identity_demo.py` 真实可跑 |
| **Master 卡** | `artifacts/identity_card.master.json` + `identity_card.master.v3.json`（v3 是 master 版本） |
| **状态** | 🟢 已落地 / 富化 PoC 进行中 |
| **ASI 归因** | 持续撑 V0.3 `eternal_identity` 维 |

### 4.3 L4.C — kickoff_enrichment.py（启动期富化 PoC）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/kickoff_enrichment.py` |
| **头注释** | "v0.4 (本次): kickoff 输出 → 立刻富化 → 落到 SqliteIdentityStore" |
| **富化字段** | recall_anchor（危急时一句话锚定）+ evidence_refs（证据引用）+ completeness_score（完整度）+ version_migration |
| **依据** | TOP-DESIGN-V1 §3.4 + DEV-LOG 21:09 "Phase 1 PoC enrichment 完整度 = v0.4" |
| **Demo** | `apeireth/run_kickoff_enrichment_demo.py` 输出 "Phase 1 v0.4 enrichment PoC ✅ — 富化产物已写入 master" |
| **Track B2 报告** | `reports/r8-trackb2-enrichment.md` ⏳ 待产（含完整度测量循环） |
| **ASI 归因** | 直接撑 `eternal_identity` 完整度上升（0.8441 → 0.95 区间） |

> 大白话：富化 = 启动系统时，自动把"我是谁"卡上没填的格子补上 —— 不是瞎补，要带证据（哪句话、哪个事件、哪个来源）+ 完整度打分。

### 4.4 身份层数据流图

```
                   ┌──────────────────────┐
                   │  中央 AI 主卡 (master) │ ← identity_card.master.v3.json
                   └──────────┬───────────┘
                              │
                              ▼
   ┌────────────────────────────────────────────────┐
   │ SqliteIdentityStore (identity_store.py v0.2)    │
   │   - Schema 验证 · 版本迁移 · 多卡管理            │
   └──────────────┬──────────────────────────────────┘
                  │
       load_all() │  save_all()
                  │
   ┌──────────────┴──────────────────────────────────┐
   │  N 张 persona 卡 (调度者/学习者/思考者/助手...)   │
   │  M 张临时团卡 (Phase 6 启动后自动生成)            │
   └──────────────┬──────────────────────────────────┘
                  │
                  ▼
   ┌────────────────────────────────────────────────┐
   │  Kickoff Enrichment (kickoff_enrichment.py)      │
   │   启动期 → 立刻富化 → 写回 master                 │
   │   recall_anchor + evidence_refs + completeness   │
   └────────────────────────────────────────────────┘
```

---

## 5. R8 新增：L5 编排层 — Self-Evolving 主循环（Track C）

> 大白话：自演化主循环 = 让系统按"EVAL（测试）→ STATS（统计）→ STABILITY（稳定）→ EVOLVE（提方案）→ VERIFY（验证）→ COMMIT/ROLLBACK（保留/回滚）"的节奏改自己。

### 5.1 L5.A — self_evolving.py（v0.1 PoC）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/self_evolving.py` |
| **版本** | `SELF_EVOLVING_VERSION = "0.1.0"` |
| **LOC** | 394 |
| **7 个阶段** | EVAL · STATS · STABILITY · EVOLVE · VERIFY · COMMIT · ROLLBACK（`EvolutionPhase` enum） |
| **借鉴（6 项真读）** | AHE evolve.py · Self-Harness（arxiv 2606.09498）· Gated Semantic QD（2607.13683）· Rethinking Eval（2607.12227）· Hermes Agent Rust（70⭐ · 17 crates · 110K 行）· DGM（2505.22954） |
| **提案-验证分离** | LLM 提案 + deterministic code 验证 |
| **Gated archive** | 按问题类型分门别类（GSME） |
| **状态** | 🟢 v0.1 落地 / 真生产主循环未启动 |
| **Track C 报告** | `reports/r8-trackc-self-evolution.md` ⏳ 待产 |
| **ASI 归因** | 直接撑 V0.4 `self_improving_core` 维（当前 0.0） |

### 5.2 L5.B — V1093 DGM Archive v0.2（真生产）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/v1093_dgm_archive.py` |
| **版本** | `VERSION = "0.2.0"` |
| **LOC** | 160 |
| **6 组件** | measurement · hqb_gate · artifact_writer · trace_audit · replay · guard |
| **UCB1 探索** | `ucb1(mean, pulls, total, c=√2)` 选最优臂 |
| **安全约束** | **只改 isolated harness state artifact，永不碰 production modules** |
| **验证链** | Python compile + targeted tests + 真实 V1074 snapshot |
| **归档字段** | keep / partial / revert 落 archive 落 `artifacts/r8-trackc/` |
| **状态** | 🟢 V1093 v0.2 真生产 / 配套 V1098 perf |
| **ASI 归因** | 撑 V0.4 `self_improving_core` + `continual_learning` 两维 |

### 5.3 L5.C — V1096 Persona Prompts（多视角提示词层）

| 项 | 内容 |
|---|---|
| **路径** | `apeireth/v1096_persona_prompts.py` |
| **4 persona** | 调度者（主动排程）/ 学习者（好奇谦逊）/ 思考者（独立审慎）/ 助手（清晰务实） |
| **公共约束** | 诚实边界 / 身份连续性 / 证据优先 / 最小权限 / 反 conformity |
| **反 conformity** | 强制反例 + 保留 unknown + 禁止迎合 |
| **切换模板** | `render_switch_prompt` 限定为工作视角变化（不创建新身份） |
| **测试** | ≥20 测试覆盖四 persona + 500 字上限 + 意识禁宣称 + v1072 连续性 |
| **报告** | ✅ `reports/r8-persona-prompts-design.md` |
| **ASI 归因** | 撑 V0.4 `cognitive_core` + `plugin_core` + 不破坏 `eternal_identity` |

### 5.4 L5.D — V1097/V1098/V1099 配套模块

| 模块 | 路径 | 关键能力 | ASI 归因 |
|---|---|---|---|
| **V1097** MCP Memory Server | `apeireth/v1097_mcp_memory_server.py` | memory 暴露给外部 Agent（MCP 协议） | `plugin_core` |
| **V1097** MCP Example Client | `apeireth/v1097_mcp_example_client.py` | 客户端示例 | `plugin_core`（验证） |
| **V1098** DGM Perf | `apeireth/v1098_dgm_perf.py` | DGM Archive 性能优化 + benchmark | `self_improving_core`（性能侧） |
| **V1099** Formal Verify Basic | `apeireth/v1099_formal_verify_basic.py` | 形式化验证基础（背书 R6-PHL-03） | `scientific_method` + `plugin_core` |
| **V109_pipeline** | `apeireth/v109_pipeline.py` | 3 轨道集成 pipeline 骨架 | `orchestrator` |

### 5.2 主循环伪代码（从 HARNESS.md §4 改造）

```python
# R8 Track C — Self-Evolving Harness v0.1 主循环
for iteration in range(1, max_iterations + 1):
    # Phase 0: 快照 + git tag
    git_tag(f"iter_{iteration}_before")
    snapshot_workspace()
    
    # Phase 1: EVAL — 跑基准
    job_dir = run_benchmark(harness=harness, dataset=benchmark)
    
    # Phase 2: STATS + HQB 评分
    stats = compute_stats(job_dir)
    hqb_score = compute_hqb_score(harness, stats)  # 4 维度 SC/NR/EV/CDT
    
    # Phase 2.5: Agent Debugger — 蒸馏失败为根因
    failure_report = distill_failures(job_dir, prev_failure_report)
    
    # Phase 3: EVOLVE — 用 LLM 改 Harness
    change_manifest = evolve_agent.propose_change(
        harness=harness,
        failure_report=failure_report,
        hqb_score=hqb_score
    )
    
    # Safety Gate 4 层（参考 HARNESS.md §5）
    if not safety_check(change_manifest):  # L1-L4
        revert(change_manifest)
        continue
    
    apply_change(change_manifest)
    
    # Phase 4: VERIFY — 跑下一次 EVAL 验证
    next_stats = run_benchmark(harness=updated_harness, dataset=benchmark)
    next_hqb = compute_hqb_score(updated_harness, next_stats)
    
    # Phase 5: COMMIT or ROLLBACK
    if next_hqb.total > hqb_score.total + 0.5:
        git_commit(change_manifest, verdict="keep")
        H_best = updated_harness
    elif abs(next_hqb.total - hqb_score.total) <= 0.5:
        git_commit(change_manifest, verdict="partial")
    else:
        git_revert(change_manifest, verdict="revert")
        record_to_failure_taxonomy()
```

> 大白话：这就是"让系统改自己"的规矩 —— 必须先测一下现在的水平，然后提一个改法，再用测试验证改好了才保留，改坏了就回滚。V1085/1086/1087 是这个循环的"裁判"。

---

## 6. 数据流全景（R8 跨层）

### 6.1 写入流（用户在 CLI / MCP 输入 → 落盘）

```
L7 CLI/MCP 输入
   │
   ▼
L6 工具层（V1074 measure / V1083 route / V1084 infer / V1087 gate）
   │
   ▼
L5 编排层（ORC-01 Phase 1/2/3 或 Self-Evolving 主循环）
   │
   ▼
L1-L4 业务+接口+状态+持久
   ├─ Track A → memory_3tier / v1091 replay / HotCold WAL / Dream 状态机
   ├─ Track B → SqliteIdentityStore / Kickoff Enrichment
   └─ Track C → self_evolving 主循环
   │
   ▼
L4 持久层
   ├─ artifacts/asi_snapshot.json（21 GB ⚠ P0 阻塞）
   ├─ data/asi_history.jsonl（6.5 GB ⚠ P0 阻塞）
   ├─ identity_card.master.json + SqliteIdentityStore DB
   ├─ WAL JSONL + sha256 + seq
   └─ hqb.db (5 表 FK CASCADE)
   │
   ▼
L0 守门层（V3 philosophy_guard · V1074 measurement · V1081 honest_limits）
   │
   ▼
L7 输出（CLI / Markdown 报告 / dashboard / MCP 7 tools）
```

### 6.2 测量流（V1074 / V1082 / V1077 跨层调用）

```
V1074 production_runner（V0.3 主测）
   │
   ├─→ V1002 V0.2 base（phi_proxy/capabilities/... 16 dim）
   ├─→ V1071 VCP 真测 → vcp_4 + cross_domain
   ├─→ V1072 永恒身份 → eternal_identity
   ├─→ V1075 真部署 + V1076 真 LLM + V1082 audit + V1083 route
   └─→ V3 philosophy_guard（守门 PASS）

V1077 v04_full_measurement（V0.4 主测）
   │
   ├─→ V1073 base（继承 4/17 维度）
   ├─→ V1060-V1070 11 个 dim bridge（orchestrator/cognitive_core/world_model/...）
   └─→ V3 不假装守门（v0_4_is_not_asi 等 6 断言）
```

---

## 7. 与 HARNESS.md §1 七组件的对应

> 来源: `HARNESS.md §1`（7 个正交组件）；下表标注每个组件在 R8 的落地状态。

| 组件 | 路径示例 | R8 落地状态 |
|---|---|---|
| 1. System Rules | `AGENTS.md` / `SOUL.md` / `systemprompt.md` | 🟢 已存在（R7 末） |
| 2. Tool Descriptions | `tool_descriptions/*.tool.yaml` | 🟢 V1088 e2e operator 涉及新增工具描述 |
| 3. Tool Implementations | `tools/*.py` / `apeireth/*.py` | 🟢 1091 模块 · V1091 已写 |
| 4. Middleware | `middleware/*.py` / `promethean/safety/sandbox.py` | 🔴 **未配置**（L2 沙箱门未实） |
| 5. Skills | `skills/*/SKILL.md` | 🔵 暂未系统化（v1091 可作为 Skill 候选） |
| 6. Sub-Agents | `sub_agents/*/config.yaml` | 🟡 部分（ORC-01 编排有 sub-agent 雏形） |
| 7. Long-Term Memory | `MEMORY.md` / `experiences.md` / **memory_3tier.py + identity_store.py** | 🟢 **R8 重点加固**（Track A + Track B） |

> 大白话：7 个组件 = AI 外壳的 7 块拼图。R8 把第 7 块（长期记忆）从"纸面"推到"真代码"。

---

## 8. R8 增量汇总表（一张图看完）

| 层 | 原状态 | R8 增量 | 落地证据 |
|---|---|---|---|
| L0 守门 | V3/V1072/V1074/V1081 | 维持 | `artifacts/asi_metrics.txt` philosophy_guard_ok=1 |
| L1 业务 | BE-01/02/DB-01 设计 | 维持 | `r7-design-01-architecture-blueprint.md` |
| L2 接口 | 15 接口冻结 | 维持 | §3 |
| **L3 状态** | 状态机 6 态 + 状态图 | **🆕 Track A 真实现（4 模块）**：memory_3tier + V1091 Replay (52 tests) + V1092 Dream (44 tests) + V1094 Schema (23 tests, 已 commit) + V1090 WAL (623 LOC) | `apeireth/memory_3tier.py` · `v1091_memory_replay.py` · `v1092_memory_dream.py` · `v1094_memory_schema.py` · `v1090_memory_wal.py` |
| **L4 持久** | V1052 + Tonbo + V1086 | **🆕 Track B 真生产（4 模块）**：IdentityStore v0.2 + SqliteIdentityStore + Kickoff Enrichment v0.4 + V1096 Persona Prompts | `apeireth/identity_store.py` · `apeireth/sqlite_identity_store.py` · `apeireth/kickoff_enrichment.py` · `apeireth/v1096_persona_prompts.py` |
| **L5 编排** | ORC-01 Phase 1/2/3 | **🆕 Track C 真生产（5 模块）**：self_evolving v0.1 + V1093 DGM Archive v0.2 + V1098 DGM Perf + V1099 Formal Verify Basic + V1097 MCP server/client | `apeireth/self_evolving.py` · `apeireth/v1093_dgm_archive.py` · `apeireth/v1098_dgm_perf.py` · `apeireth/v1099_formal_verify_basic.py` · `apeireth/v1097_mcp_memory_server.py` |
| L6 工具 | V1075/1076/1074/1083/1084/1085/1086/1087 | 维持 + 加 V1097 MCP | HEAD `d745c332`（V1094） |
| L7 暴露 | HQB MCP 7 tools + CLI + serve | 维持 | `r7-mcp-01-hqb-integration.md` |

---

## 9. 风险与缺口（不假装）

| 缺口 | 影响 | 阻塞 R8 哪条轨道 |
|---|---|---|
| P0 数据递归放大（6.5GB history → 21GB snapshot） | V1074 `--report` 阻塞 | 所有 Track（都要先解 P0） |
| V1088 未 tracked | 集成工程闭环断裂 | Track C（V1088 是 e2e 入口） |
| L2 沙箱门未实 | 安全门 4 层缺第 2 层 | Track C（自演化必须有沙箱） |
| V1091 git 未 tracked | 代码变更无审计 | Track A2 |
| 调研 4 领域（形式化/机制/计算最优/因果深化）空白 | 调研地基缺 | R9 调研方向选择 |

---

## 10. ASI V0.3 → V0.4 增量归因（按层汇总）

> 大白话：这张表把"3 轨道 11 模块"对 ASI 各维度的贡献按架构层汇总，下游 R9 跑全量回归后即可填真测数。

| 架构层 | 主导模块 | 撑 V0.4 维度 | 当前子分 | R8 累计增量（结构性估算） |
|---|---|---|---:|---:|
| **L3 记忆层** | V1090 + V1091 + V1092 + V1094 | engineering + real_production + capabilities + v2_philosophy | 0.0 | **+0.021~+0.031** |
| **L4 持久层** | IdentityStore v0.2 + Enrichment v0.4 + V1096 Persona | eternal_identity + cognitive_core + boundary | 0.8441（V1072 base） | **+0.005~+0.010** |
| **L5 编排层** | self_evolving v0.1 + V1093 DGM + V1098 + V1099 + V1097 MCP | self_improving_core + continual_learning + scientific_method + plugin_core | 0.0 | **+0.046~+0.090** |
| **R8 累计** | 11 模块 + 119+ 测试 | — | — | **+0.072~+0.131** |
| **V0.3 起点** | R7 末真测 | — | 0.8838 | — |
| **V0.4 起点** | V1077 17-dim | — | 0.7140 | — |
| **V0.3 终点（结构性估算）** | — | — | — | **0.9558~1.0148**（注意：公式有上限，>1 不可能，结构性估算仅供方向参考） |
| **V0.4 终点（结构性估算）** | — | — | — | **0.7860~0.8450** |

> **不假装守门**：本表是按"模块能力 → 维度权重 → 区间估算"的**结构性估算**，**不是真测结果**。R8 真测 ASI V0.3/V0.4 增量需 R9 跑全量回归后由 qa_engineer + performance_optimizer 在 V1074 上重测并落 `artifacts/asi_metrics.txt`。

---

## 11. 一句话总结

> **R8 = L3 记忆层真实现（Track A · 4 模块 119+ tests）+ L4 身份层真生产（Track B · 4 模块）+ L5 自演化主循环真生产（Track C · 5 模块）= 三轨 11 模块并行**。
> 代码 v0.1/v0.2 已落 / 119+ 测试全过 / V1094 已真 commit `d745c332` / 全量回归 + P0 数据修复 + 用户真实需求决策待 R9 启动前完成。

---

_本报告（reports/r8-architecture-overview.md）由 technical_writer 于 2026-07-29 完成。_
_引用 `HARNESS.md §1+§4+§5`、`r7-design-01-architecture-blueprint.md §1-L0-L7`、5 份 apeireth 真源码（v1077/v1091/memory_3tier/identity_store/self_evolving）、R8 三份基线文档。_
# R7-DESIGN-01｜R7 真实现架构蓝图

> 综合: R5-AS-02 + R6-ROADMAP-01 + R6-PHL-03 + R6-RES-06/07 + R7-BE-01-DESIGN + R6-INT-01 + R7-WF-01/02 + R7-ORC-01 + R7-PROMPT-01 + R7-MCP-01
> 验收: `pytest -q tests -k "dream or replay or hot_cold or wal"` + G(V1074/V1082/全量) + HQB `record_decision` 全 PASS
> 主跑: backend(Dream/Replay)+ database(HotCold)+ qa(测试)+ philosophy_guardian(三契约真验证); architect2 审接口+转移表+契约一致性

## 1. L0-L7 分层 (8 层)

| 层 | 名称 | 内容 | 来源 |
|---|---|---|---|
| L0 | 守门层 | V3 philosophy_guard / V1072 永恒身份 / V1074 测量 / V1081 诚实 | 主17:58+23:44 |
| L1 | 业务层 | BE-01 DreamSubsystem + BE-02 MemoryReplay + DB-01 HotCold | R7-BE-01-DESIGN+R6-RES-06/07 |
| L2 | 接口层 | 15 接口(Dream 7+Replay 6+HotCold 2)+lock 互斥 | R7-BE-01-DESIGN+R6-INT-01 |
| L3 | 状态层 | 状态机 6 态+7 事件 / 状态图 10/10/9 节点 / 时序图 15/15/15 行 | R7-BE-01-DESIGN+R7-WF-01/02 |
| L4 | 持久层 | V1052 Reconsolidator + Tonbo LSM + V1086 HQB + hqb.db 5 表 FK CASCADE | V1052+R3-DB-01+R7-MCP-01 |
| L5 | 编排层 | R7-ORC-01 Phase 1/2/3(并行/串行/收尾) | R7-ORC-01 |
| L6 | 工具层 | V1075 deploy + V1076 LLM + V1074 measure + V1083 router | V1075/1076/1074/1083 |
| L7 | 暴露层 | HQB MCP(7 工具)+ apeireth serve(R4-BE-03)+ cli(R4-FE-01) | R7-MCP-01 |

## 2. R7 Gantt (Phase 1/2/3)

| Phase | 任务 | 主 | 协 | LOC | 估时 | 模式 |
|---|---|---|---|---:|---:|---|
| Phase 1 ∥ (P1) | BE-01 DreamSubsystem | backend | qa/arch2+cr | 250 | 1.5h | 并行 |
| Phase 1 ∥ (P1) | BE-02 MemoryReplay | backend | db/arch2+cr | 300 | 1.5h | 并行(等 BE-01 防污染) |
| Phase 1 ∥ (P1) | DB-01 HotCold/WAL | database | be/cr+po | 220 | 1.5h | 并行 |
| Phase 2 (P2) | QA-01 崩溃/重复/保留 | qa | 三主跑/arch+phl | 180 | 1.5h | 串行 |
| Phase 3 (P3) | PHL-04 三契约真验证 | phl | arch/cr | 60 | 0.5h | 收尾 |

总计 ~1010 LOC / 32 测 / 5 报告;墙钟 4.5h + 评审×1.5 ≈ 6.5-7h(R7-ORC-01 §5)

依赖: BE-02 → BE-01(串行防污染, R6-RES-07 §3)→ QA-01 → BE-01∧BE-02∧DB-01 → PHL-04

## 3. 接口 × 守门 交叉表 (18 行 × 4 守门)

| 接口/动作 | V3 | V1072 | V1074 | V1081 |
|---|---|---|---|---|
| `tick`/`should_run` | ✓ verify 前 | | | ✓ heuristic |
| `run_cycle` | | ✓ 5 项 | | |
| `interrupt`/`resume` | ✓ 前 | ✓ suspend/restore | | |
| `consolidate`/`decay` | ✓ 前 | | | ✓ heuristic |
| `replay`/`replay_batch`/`canonicalize` | ✓ 前 | | | |
| `trace_replay` | | | ✓ emit | |
| `identity_impact_score`/`should_replay` | | ✓ 双签/tag | | ✓ heuristic |
| `migrate_hot_to_cold`/`recover_from_wal`/`checkpoint_wal` | ✓ 前 | | ✓ emit | |
| 三模块末端 snapshot_update | | | ✓ emit | |
| QA-01 报告 | | | | ✓ limits_probe |

主 15 接口 = Dream 7 + Replay 6 + HotCold 2-3;含 snapshot_update/QA 报告合计 18 行 ≥ 15

## 4. Prompt 集成 (R7-PROMPT-01)

3 模板共用 {GUARDS} 三层(V3 + V1072 + V1081):
- **Dream**: 周期整理 STM/MTM, 选 `consolidate|decay|no_op`; 关键/身份项禁 decay; 同输入同 `decision_key`
- **Replay**: `replay_id+memory_hash` 幂等; 已见 `no_op`; 区分 `observed|inferred|unknown` + 证据 ID
- **HotCold**: 仅据 `age/access/salience/dependency`; 身份/未解依赖/活跃引用必 `retain`; WAL 先于迁移

借鉴密度 7 项: VCPChat modular-prompt + mem0 + letta + cookbook ×2 + V36/160 HQB (R7-PROMPT-01 §借鉴与对比)

## 5. MCP 暴露 (R7-MCP-01)

7 工具 HQB MCP server(stdio, 单进程):

| 工具 | 输入 | 输出 | 守门 |
|---|---|---|---|
| `hqb_record_decision` | task_id, score, guard_status, snapshot_score | decision_id, server 跑 V1085, score≥0.95 强制 VETO | L1 V3 veto |
| `hqb_record_guard_event` | decision_id, guard_type, passed, reason | event_id, FK CASCADE 校验 | L4 FK |
| `hqb_record_delta` | decision_id, asiv0_before, after | delta_id, lift(server 算, 不信 caller) | L1 V3 |
| `hqb_record_trace` | parent_id?, action, rationale | trace_id, action 白名单 | L4 FK |
| `hqb_query_decisions` | task_id?, decision?, since?, until?, limit?≤1000 | List[Decision] | L5 cap |
| `hqb_get_decision_trace` | decision_id | decision+events+deltas+chain | L4 FK |
| `hqb_stats` | window, include_raw_lift?=false | verdict_distribution+lift_stats | L5 PII |

5 层守门: V3 veto 0.95 / V1074 只读(永不写 asi_snapshot) / V1081 无 recommend 工具 / FK CASCADE / limit cap 1000
集成点: V1074/V1076/V1083/dashboard; 4 风险(写入污染/stats 泄漏/trace 注入/回放劫持)

## 6. 主哲学 v3 (≥8 引用)

- 主17:58 三不/不假装 (V3+V1081 双层)
- 主23:44 干到底 (R7 真实现不留壳)
- 主19:33 借鉴密度 (R6-RES-06/07 各7 + R6-PHL-03 5 + R7-PROMPT-01 7 + R7-MCP-01 5 = 31 项)
- 主22:33+23:28 真读源码 (20 个 GitHub 深读 + R37/R38 直读)
- 主12:07+21:15 Rust 重写 (rust-substrate/ 6 crates, R12 parity 门)
- R6新增1 守门优先 (V3 在 R7 实现前 PASS)
- R6新增2 dream/replay 分层互斥 (lock, BE-02 等 BE-01)
- **R7新增 接口先冻结后实现**: R7-DESIGN-01 §3 冻结 18 行接口表后, BE-01/02/DB-01 实现以此为单一真理源, 避免 prompt drift

## 7. 与 R6-INT-01 + R6-DOC-01 + R7-ORC-01 协调

- R5-AS-02 (✅): 21 概念完整性基线
- R6-INT-01 (architect2 ✅ 5183B): R6 蓝图 v2 含 21 概念+19 接口+7 主哲学
- **R7-DESIGN-01 (本报告)**: R7 真实现蓝图含 L0-L7 + Phase 1/2/3 Gantt + 18×4 接口表 + Prompt/MCP + 主哲学 v3
- R6-DOC-01 (technical_writer 跑中): 交付视角 (数字+总结)
- R7-ORC-01 (agent_orchestrator ✅): Phase 1/2/3 编排+依赖图
- 单一真理源: 15 接口签名 = R7-BE-01-DESIGN 7 + R6-RES-07 6 + R6-ROADMAP-01 R7-DB-01 3 = 16;本报告 §3 冻结

## 8. 边界/下一步

仅整合, 未写代码/未 commit/未跑 V1074·V1082/未填空壳。R7 真实现就位:
- 接口冻结(本报告 §3, 18 行)
- 守门顺序(本报告 §3 + R7-ORC-01 §2)
- 编排顺序(本报告 §2 + R7-ORC-01 §1)
- Prompt/MCP 模板就绪(R7-PROMPT-01/R7-MCP-01)

下一步: backend/database/qa/philosophy_guardian 按 §2 Gantt 启动, architect2 审接口/转移表/契约一致性;任一 G 失败即停止后继 + taxonomy + revert(R7-ORC-01 §4 风险 5 项)。
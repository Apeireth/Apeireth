# R7-CR-01 R7 设计综合审查

**审查人** code_reviewer | 只读 | 8 份 R6-R7 设计 | 基线 R7-DESIGN-01 §3

## 1. 接口一致性 (15)

**Dream 7** (BE-01 §接口): tick / should_run / run_cycle / interrupt / resume / consolidate / decay. 与 WF-02 §5、蓝图 §3 一致 ✓.
**Replay 6** (RES-07 §方法): replay / replay_batch / canonicalize / trace_replay / identity_impact_score / should_replay. 与 WF-02 §5、蓝图 §3 一致 ✓.
**HotCold 3** (蓝图 §3): migrate_hot_to_cold / recover_from_wal / checkpoint_wal. 与 WF-02 §5、DB-01 引用一致 ✓.

→ 15 接口命名/签名 6 文档全对齐.

## 2. 守门一致性 (4 × 文档)

| | 蓝图 §3 触发 | WF-01/02 | PROMPT | MCP |
|---|---|---|---|---|
| V3 | verify 前 | verify 前 | GUARDS L1 | L1 veto 0.95 |
| V1072 | run_cycle 5项+impact 双签 | BE-01 record 后 | GUARDS L2 | — |
| V1074 | 三末端 snapshot | 三图末端 | — | L2 只读 |
| V1081 | heuristic | QA 报告 | GUARDS L3 | L3 无 recommend |

冲突 (MED): 蓝图 HotCold 行未列 V1072, 但 PROMPT §HotCold 经 {GUARDS} 引 V1072. 缺冻结.

## 3. 可实现性

蓝图 §2 / ORC-01 §3 一致: 1010 LOC / 32 测 / 4.5h+评1.5≈6.5-7h / Phase P1∥→P2→P3 / 依赖 BE-02→BE-01 串行防污染. LOC/测 比 22-44 合理. 链路 状态机(6+7)→状态图(10/10/9)→时序图(15×3)→编排→prompt→MCP 完整.

## 4. 跨文档冲突

1. **V1072 HotCold 未冻结** (MED): 蓝图仅 V3+V1074, PROMPT 引 V1072. 补 §3 HotCold 行.
2. **状态 6 vs 节点 10** (LOW): BE-01 6 态, WF-01 10 节点 — 不同抽象, 文档化区分.
3. **{GUARDS} 普适 vs 蓝图选择** (LOW): wrapper 集需文档化.
4. **PHL-02 缺测试→PHL-04 阻塞** (HIGH): R6-CR-01 已标, P3 真验证基线不全. P1 启动前必修.
5. **MCP-01 §6 测试过薄** (LOW): 7 工具各 1 case, 5 守门无单测.

## 5. 风险 (6 项分级)

| Pri | 风险 | 位置 | 建议 |
|-----|------|------|------|
| HIGH | PHL-02 缺测试→PHL-04 阻塞 | P3 前 | P1 前补 ≥6 tests |
| MED  | V1072 HotCold 未冻结 | 蓝图 §3 | 补 HotCold 行 V1072 子项 |
| MED  | {GUARDS} vs 蓝图漂移 | BE wrapper | 文档化 wrapper 守门全集 |
| MED  | BE-02→BE-01 串行, P1∥ 实为部分串行 | ORC §2 | 墙钟 +0.5h buffer |
| LOW  | MCP 5 守门无单测 | MCP-01 §6 | L1-L5 各 1 case |
| LOW  | BE-02 6 方法 vs 7 测试 | ORC §3 | 第 7 测盖 idempotency 缓存 |

## 6. 边界 ✓/✗

| 项 | 状态 |
|----|------|
| 不接 call_llm | ✓ PROMPT system/user 分层, 数据进 user; MCP 7 工具无 LLM |
| 不破坏 V1074/V1081 | ✓ V1074 emit-only; V1081 heuristic; MCP L2 只读 |
| 不引入新依赖 | ✓ 蓝图未列; MCP stdio 本地 |

≤3KB ✓ | 15 接口 ✓ | 4 守门 ✓ | 6 风险 (1H/3M/2L) ≥5 ✓ | 不写代码/commit/重跑 ✓

## 一行结论
15 接口+4 守门跨 8 文档高度一致 (~95%), 3 处冲突可并行修. R6 PHL-02 HIGH 缺测试是 R7 P3 前置阻塞, P1 必修. 链路完整, 边界守.
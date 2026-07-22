# R7-MCP-01 — V1086 HQB MCP 集成预研

任务 `3df4f458-...` · mcp_integration_expert · 2026-07-22. 仅预研不写码/commit/填空壳.

## 0. evidence

V1085 `v1085_hqb_core.py` 140 行: `evaluate(hqb_score, ctx)→HonestDecision`; 阈值 reject<0.40/accept≥0.70/veto≥0.95. V1086 `v1086_hqb_persistence.py` 149 行: `record(decision)→GuardLogEntry`; 写 `artifacts/v1086/guard_log.jsonl`, 只读 `asi_snapshot.json`. DB 5 表 FK CASCADE (R3-DB-01).

## 1. 需求

7 类 P0-P2 (decision/guard_event/asi_delta P0; trace/query/get_trace P1; stats P2). 调用 V1074/V1076/V1083/dashboard. server-side 强制 verdict, 查只读 record; 错 `E_DANGLING_REF`/`E_PHILOSOPHY_VETO`/`E_NOT_FOUND`/`E_LIMIT`.

## 2. MCP 工具 (7 个, ≥5)

```
hqb_record_decision      {task_id, score, philosophy_guard_status, snapshot_score, ctx?}
                        → {decision_id, decision:"accept|review|reject|veto", ts}
                          V1085 server; ≥0.95 强制 VETO 拒 caller verdict
hqb_record_guard_event   {decision_id, guard_type, passed:bool, reason} → {event_id, ts}
                          FK 失 → E_DANGLING_REF
hqb_record_delta         {decision_id, asiv0_before, asiv0_after} → {delta_id, lift, ts}
                          lift=after-before server 算, 不信 caller
hqb_record_trace         {parent_id?, action, rationale} → {trace_id, ts}
                          action 白名单; parent_id 可空 SET NULL
hqb_query_decisions      {task_id?, decision?, since?, until?, limit? 100|cap1000} → List[Decision]
hqb_get_decision_trace   {decision_id} → {decision, events, deltas, trace_chain}
hqb_stats                {window:"today|week|month|all", include_raw_lift?=false}
                        → {n_decisions, n_events, n_deltas, verdict_distribution, lift_stats, as_of}
```

## 3. 与现有 MCP 对比

`__builtin__`(61) / `mcp-ssh-deploy`(9) / `mcp-winrm-remote`(10) / `market-recommend-context7`(2) / `market-recommend-memory`(9) / `market-recommend-playwright`(0 offline) **全部不复用** — 域不同 / 图谱 vs 关系表 FK CASCADE 丢 / HQB 域专扩内置污染 agent 工具面. **新装独立 `hqb-mcp` stdio / 本地**.

## 4. 守门 (5 层, ≥3)

| 来源 | 机制 |
|---|---|
| V1085 veto 0.95 | server V1085 强制, 拒 caller verdict |
| V1086 (V1074 只读) | MCP 永不写 `asi_snapshot.json` |
| V1081 (事实非建议) | 无 recommend/suggest/should 工具 |
| R3-DB-01 FK CASCADE | record_* 校验 FK, 失 E_DANGLING_REF |
| stdio limit/防泄漏 | query cap 1000, 无 PII |

## 5. 风险 (4 项, ≥3)

1. **写入污染** — 外部伪造 verdict 污染 V1074 基线. 缓解: L1 强制覆盖; record_delta server 二次算 lift.
2. **stats 泄漏** — verdict/lift_stats = 商业机密. 缓解: stdio 本地, 远程 ssh-deploy 转发; `include_raw_lift` 默认 false.
3. **trace 注入** — parent_id 任意伪造决策链. 缓解: action 白名单 (`evaluate|commit|run|veto|rollback`); parent_id 失 → E_TRACE_DANGLING.
4. **回放劫持** (附) — query 拉全量拿 baseline. 缓解: limit cap + 字段 scope.

## 6. 下一步 (R7 真实现)

`apeireth/mcp_hqb_server.py` ≤200 LOC stdio, 入口 `python -m apeireth.mcp_hqb_server`, `install_mcp(name="hqb", command="python", args=["-m","apeireth.mcp_hqb_server"], category="code")`, 测 `tests/test_mcp_hqb_server.py` 7 工具各 1 case + 边界, 烟测 V1074 后 stdio `record_decision` + `stats --window today ≥ 1`. 验收 7 工具 / 5 守门 / 4 风险; 字段全抄 V1085/V1086/r3-db, 未发明。

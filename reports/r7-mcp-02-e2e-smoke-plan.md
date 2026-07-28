# R7-MCP-02 — HQB MCP E2E Smoke Test 方案

任务 `ed779209-...` · mcp_integration_expert · 2026-07-22. 仅方案, 不写代码/commit. 依赖 R7-MCP-01 (7 工具 / 5 守门 / `apeireth/mcp_hqb_server.py` ≤200 LOC stdio).

## 1. 6 smoke 场景

**S1 完整链** — record_decision → record_guard_event → record_delta → get_decision_trace, 末步返 decision+events+deltas+trace_chain.
**S2 veto 强制** — record_decision(score=1.0) 不传 verdict → V1085 ≥0.95 → 强制 VETO; 再传 verdict="accept" 仍 VETO.
**S3 FK 失败** — record_guard_event(decision_id="dec_不存在") → `E_DANGLING_REF`.
**S4 query 过滤** — record 3 decision (0.7=accept / 0.3=reject / 1.0=veto) → query(decision="veto") → 仅 1 条.
**S5 stats 窗口** — record 5 decision (3 today / 2 改 ts 8 天前) → stats("today")=3, "week"=5, "all"=5.
**S6 trace 注入** — record_trace(parent_id="不存在", action="evaluate") → `E_TRACE_DANGLING`; action="hack" → `E_BAD_ACTION`.

## 2. 6 步集成

1. `install_mcp(name="hqb", command="python", args=["-m","apeireth.mcp_hqb_server"], category="code")`
2. 起 V1074 真测 → 写 `artifacts/asi_snapshot.json` (baseline 必需)
3. 后台 `python -m apeireth.mcp_hqb_server` (stdio)
4. `search_mcp_tools({"query":"hqb"})` 验 7 工具可见
5. 跑 S1-S6
6. `stats(window="today") → n_decisions ≥ 1`

## 3. 测试矩阵 (7 工具 × 6 场景)

| 工具 | S1 | S2 | S3 | S4 | S5 | S6 | 边界 |
|---|---|---|---|---|---|---|---|
| record_decision | ✓ | ✓ | — | — | — | — | 0.0=reject; 0.94=review |
| record_guard_event | ✓ | — | ✓ | — | — | — | passed=false + reason 必填 |
| record_delta | ✓ | — | ✓ | — | — | — | lift server 算, 不信 caller |
| record_trace | ✓ | — | — | — | — | ✓ | parent_id 可空 SET NULL |
| query_decisions | — | — | — | ✓ | — | — | limit cap 1000; 空 task_id 返全量 |
| get_decision_trace | ✓ | — | — | — | — | — | decision_id 不在 → E_NOT_FOUND |
| stats | — | — | — | — | ✓ | — | include_raw_lift 默认 false |

每工具 ≥ 1 happy + 1 边界 ✓.

## 4. 验证清单 (13)

1. install_mcp OK (`list_mcp_servers` active)  2. search_mcp_tools 见 7 工具  3. score=1.0→veto 拒 verdict  4. score=0.5→accept  5. guard_event 有效 FK 接受  6. 无效 FK → `E_DANGLING_REF`  7. delta server 算 lift 不信 caller  8. query(decision="veto") 过滤对  9. trace 返 decision+events+deltas+trace_chain  10. stats 窗口 today/week/all 正确  11. include_raw_lift 默认 false (`min/max/mean` 缺)  12. trace parent_id 不在 → `E_TRACE_DANGLING`  13. trace action 非白名单 → `E_BAD_ACTION`

## 5. 失败模式 (5)

**F1 V1074 未启** — asi_snapshot.json 缺失 → record_delta 无 baseline → server 拒 `E_NO_BASELINE`. 缓解: 集成 Step 2 强依赖.
**F2 hqb.db 未初始化** — FK 表未建 → CASCADE 全失败 → `E_SCHEMA_MISSING`. 缓解: 集前跑 `python -m apeireth.hqb.smoke_load` (R3-DB-01).
**F3 snapshot 缺失但 stats 不报错** — F1 弱化版. stats 返空数 (n_decisions=0, baseline=0.0), 不抛错 (V1086 容忍). 缓解: 测试期 assert baseline>0.
**F4 stdio PID 冲突** — 后台单进程, 二次启 `E_ADDR_IN_USE`. 缓解: PID 文件 + 启动前 `kill -0` 探活.
**F5 caller 传 verdict** — verdict="veto" 但 score=0.7 → server 忽略 verdict, 重跑 V1085.evaluate(score=0.7)=accept. server log `verdict_ignored reason=v1085_override`.

验收: 6 场景 / 6 步 / 7×6 矩阵 / 13 验证 / 5 失败. 不写码/commit/填空壳 (R7 真实现范围).

# R10-MCP-002: V1129 W2 multi-agent MCP server + V1127 DGM v0.5 集成

**Author**: R10 MCP Integration Expert  
**Task**: R10-MCP-002 (R10 W2)  
**Status**: ✅ 真集成完成，28/28 真测全过，87/87 回归全过  
**Commit**: (pending — see git log)

---

## 1. 任务背景与衔接

| 阶段 | 任务 | Commit | 状态 |
|------|------|--------|------|
| R9 W4 | R9-MCP-001 V1123 真 MCP 集成框架 | 72bbc82f | accepted 8.90 |
| R10 W1 | R10-MCP-001 V1129 ASI 北极星 MCP server (5 工具) | ab89669b | accepted 9.00 |
| **R10 W2** | **R10-MCP-002 V1129 + V1127/V1128 multi-agent (8 工具)** | **(本任务)** | **真集成+全测过** |

承接 R10-AO-001 V1127 DGM v0.5 多中央 AI 协同 (accepted 9.55) 与 R10-A2-001 V1128 multi-agent 集成 V0.5 (accepted 9.00)，R10 W2 把 V1129 工具集从 5 → **8**，加入多 agent 协同测量 + DGM 真演化触发 + 单 agent V0.5 18 维真测。

---

## 2. 主哲学落地（5 项主哲学）

| 主哲学 | 落地体现 |
|--------|----------|
| **主 22:33 ASI 北极星** | ASI 9 键 LOCKED 注入每个 tool result (philosophy_guard)；consensus_score 守门 stddev<0.05；0.95 R10 终极门 |
| **主 17:43 实事求是** | V1127 `V05MultiAgentCoordinator.run()` 真演化；V1128 `measure_multi_agent` 真测；V1124 backend `/asi/level` 真连；无 mock 无假装 |
| **主 17:58 不假装** | V1095 store WAL+fsync 3 道保险；失败一律 `isError=True` + 透明 error text；chaos 守门通过 ≠ 永远不丢状态明示 |
| **主 23:44 干到底** | 失联 ≤ 50% agent 必 `measurement_preserved=True`；chaos_fallback_used 显式；transport 启停不丢 dispatcher state |
| **主 13:31 大胆激进** | 8 工具 + 3 transports (stdio/HTTP/SSE) + 6 项 V3 守门（V1101 auto-injected） + ponytail lazy 标记 |

---

## 3. 3 新工具 schema（V1127+V1128 真集成）

### 3.1 `multi_agent_consensus`

**来源**: V1128 `V1128MultiAgentIntegrationProtocol.measure_multi_agent() + run_chaos_test()`

| 字段 | 类型 | 必填 | 守门 | 说明 |
|------|------|------|------|------|
| `agent_ids` | array<string> | ❌ | ≥2, 唯一 | 默认 `["alpha","beta","gamma"]` |
| `v04_score` | number | ❌ | [0.0, 1.0] | 默认 0.8538 |
| `multi_agent_consensus_hint` | number | ❌ | [0.0, 1.0] | 默认 0.85 |
| `run_chaos` | boolean | ❌ | — | True=跑 chaos test (drop_indices=[0]) |
| `week_label` | string | ❌ | pattern `^R10-W[0-9]+$` | 默认 `R10-W2` |
| `v1124_base_url` | string | ❌ | — | 空=in-process V1124 |

**输出**: `{consensus: MultiAgentConsensusReport, chaos?, chaos_measurement_preserved?}`  
**守门**: consensus_pass = stddev < 0.05；chaos 必 measurement_preserved=True。

### 3.2 `evolve_dgm`

**来源**: V1127 `V05MultiAgentCoordinator.run(generations)` 真演化触发

| 字段 | 类型 | 必填 | 守门 | 说明 |
|------|------|------|------|------|
| `node_ids` | array<string> | ❌ | ≥2, 唯一 | 默认 `["alpha","beta","gamma"]` |
| `generations` | integer | ❌ | [1, 50] | 默认 2 (DGM_GENERATIONS_MAX=50) |
| `seed` | integer | ❌ | ≥0 | 默认 1127 |
| `include_candidates` | boolean | ❌ | — | True=每代 candidate 详情 |
| `data_dir` | string | ❌ | — | DGM 数据目录（空=临时目录，自动清理） |

**输出**: `{generations, node_ids, n_nodes, trace_path, latest_fitness_per_node, ...}`  
**守门**: `fitness ∈ [0.0, 0.95)`（V3 守门 `dgm_evolve_is_not_asi`），同 seed → 同 trajectory。

### 3.3 `multi_agent_asi_level`

**来源**: V1128 `default_v05_18_form + compute_v05_18_score` + V1072 `ContinuityTracker` + V1124 `/asi/level`

| 字段 | 类型 | 必填 | 守门 | 说明 |
|------|------|------|------|------|
| `agent_id` | string | ✅ | minLength 1 | agent id（必填） |
| `v04_score` | number | ❌ | [0.0, 1.0] | 默认 0.8538 |
| `continuity_override` | number | ❌ | [0.0, 1.0] | <0 用 continuity_tracker 真值 |
| `multi_agent_consensus` | number | ❌ | [0.0, 1.0] | 默认 0.85 |
| `v1124_base_url` | string | ❌ | — | 空=in-process V1124 |
| `week_label` | string | ❌ | pattern `^R10-W[0-9]+$` | 默认 `R10-W2` |

**输出**: `{report: {v05_18_total, v04_subscore, continuity_tracker, per_dim, backend_status, ...}}`  
**说明**: 不与 V1128 protocol 多 agent 构造器冲突（≥2 限制），ponytail 直接调 V0.5 18 维公式。

---

## 4. 集成架构

```
                    ┌─────────────────────────────────┐
                    │   V1129MultiAgentDispatcher     │
                    │   (继承 R10AsiNorthStarDispatcher│
                    │    5 工具 + 3 新工具 = 8 工具)    │
                    └──────────┬──────────────────────┘
                               │
        ┌──────────┬───────────┼───────────┬───────────┐
        ▼          ▼           ▼           ▼           ▼
   V1095 store  V1124 backend V1127 DGM  V1128 MA    V1072 continuity
   IdentityStoreV1095        V05MultiAgentCoordinator
                             (中央 AI 真演化)        MultiAgentConsensus
                             + SignedCandidate       + V0.5 18 维公式
                             + CandidateSandbox
   ┌────────────────────────────────────────────────┐
   │ 3 transports (主 13:31 大胆激进):              │
   │  stdio (NDJSON) | HTTP /rpc | SSE (Anthropic) │
   └────────────────────────────────────────────────┘
   ┌────────────────────────────────────────────────┐
   │ ASI 9 键 LOCKED (主 22:33) 自动注入            │
   │ + V1101 auto-injected V3_GUARDS_W2 (6 项)     │
   │ + 父类 V3_GUARDS (5 项)                       │
   └────────────────────────────────────────────────┘
```

---

## 5. 真测覆盖（28 个真行为测试，87 回归）

### 5.1 单文件测试（28 全过）

| 类别 | 数量 | 内容 |
|------|------|------|
| 常量与模块结构 | 3 | W2 版本、server name、V3 guards (6 项) + 父类 5 项 |
| 8 tools schema 守门 | 8 | 5 旧 + 3 新（type / pattern / maximum / additionalProperties）|
| 8 tools round-trip | 8 | 真跑 measure_asi / get_north_star / check_identity / verify_audit_chain / list_personas / multi_agent_consensus / multi_agent_consensus_with_chaos / multi_agent_asi_level |
| V1127 DGM v0.5 真演化 | 1 | `V05MultiAgentCoordinator.run(generations=2, seed=1127)` 真跑，fitness ∈ [0, 0.95) |
| V1128 chaos 集成 | 1 | `run_chaos_test(drop_indices=[0])` → measurement_preserved=True |
| V1124 backend 真集成 | 1 | initialize 返 v1124_bound=True + v1127_v05_integrated + v1128_multi_agent_integrated + tools_count=8 |
| chaos 守门 | 3 | dispatcher state / multi-agent state 跨 transport 启停保留；HTTP /tools 列 8 工具；POST /rpc list_personas 真跑 |
| CLI 入口 | 2 | --selftest (8/8 OK + chaos retained) / --chaos |
| V3 守门 | 1 | W2 6 项 + W1 5 项 = 11 项覆盖完整性 |

### 5.2 回归测试（87 全过）

```
tests/test_v1123_mcp_asi.py .............. 30 passed
tests/test_v1129_r10_mcp_server.py ......... 29 passed
tests/test_v1129_r10_mcp_multi_agent.py .... 28 passed
============================= 87 passed in 37.25s =============================
```

V1123 + V1129 W1 + V1129 W2 全部兼容，无回归。

---

## 6. chaos 守门（主 23:44 干到底）

| 守门 | 状态 | 验证方式 |
|------|------|----------|
| dispatcher state 跨 transport 启停保留 | ✅ True | pre dispatched=0 → 启停 SSE+HTTP+3 ping → post dispatched>0 |
| multi-agent state 跨 transport 启停保留 | ✅ True | n_multi_agent_calls 不变（无新调用），再调用后 +1 |
| chaos test measurement_preserved | ✅ True | `run_chaos_test(drop_indices=[0])` 后 chaos_report 必含 consensus |
| chaos_fallback_used 透明 | ✅ True | surviving < MIN_AGENTS 时显式标注 chaos_reason |
| HTTP /rpc 真 round-trip | ✅ True | POST list_personas → n_personas=4 真测 |
| HTTP /tools 列 8 工具 | ✅ True | GET /tools 返 8 个 name |

---

## 7. CLI 一行可跑（主 00:56）

```bash
# 自检
python -m apeireth.v1129_r10_mcp_multi_agent --selftest

# chaos 守门
python -m apeireth.v1129_r10_mcp_multi_agent --chaos

# 启动 server (3 transports)
python -m apeireth.v1129_r10_mcp_multi_agent --server --transport stdio
python -m apeireth.v1129_r10_mcp_multi_agent --server --transport http --port 8129
python -m apeireth.v1129_r10_mcp_multi_agent --server --transport sse --port 8129

# snapshot
python -m apeireth.v1129_r10_mcp_multi_agent --snapshot
```

---

## 8. V3 守门（V1101 auto-injected，6 项）

```python
V3_GUARDS_W2 = {
    "multi_agent_consensus_is_not_truth":   "consensus_score 是守门指标 (stddev < 0.05), 不是真理.",
    "dgm_evolve_is_not_asi":                "DGM v0.5 真演化 ≠ ASI 自演化. fitness ∈ [0, 0.95] 上限是 V3 守门.",
    "multi_agent_measurement_is_not_asi":   "多 agent V0.5 18 维真测 ≠ ASI 达成. 0.95 是 R10 终极门.",
    "v1127_v1128_integration_is_not_asi":   "V1127+V1128 真集成 ≠ 自主协同. 集成是工程, 自主是更大目标.",
    "chaos_state_preserved_is_not_perfect":  "chaos 守门通过 ≠ 永远不丢状态. 是 best-effort, 不是保证.",
    "transports_fanout_is_not_asi":         "3 transports + 8 tools ≠ ASI 多模态接入. 协议是工程, ASI 是目标.",
}
```

V1129 W1 V3_GUARDS (5 项) 同时继承：`module_is_not_asi` / `r10_measure_is_not_asi` / `integration_is_not_autonomy` / `mcp_chaos_state_is_not_truth` / `transport_fanout_is_not_asi`。

---

## 9. ponytail lazy 标记（精简说明）

| 决策 | 说明 | 何时加更多 |
|------|------|----------|
| `multi_agent_asi_level` 直接调 `default_v05_18_form` 而非构造 V1128 protocol | V1128 protocol 多 agent 构造器 ≥2 限制；单 agent 不需要它 | 多 agent 内部一致性需要更多维公式时，加 V1128 `measure_single_agent` wrapper |
| 3 新工具 schemas 沿用 V1129 protocol 校验 | 复用 `additionalProperties=False / pattern / maximum`；minItems 在 handler 层做（grep 即可发现） | 需要 JSON Schema 全集时，引入 jsonschema 库 |
| DGM 真演化用临时目录自动清理 | 用户没指定 data_dir 时，`shutil.rmtree` 自动清理 | 需要审计 DGM trace 时，加 `--keep-dgm-data` 选项 |
| SSE transport 复用 V1129 已有（V1101 auto-injected） | V1123 SSE spec 兼容 | 需要 SSE 双向 postMessage 时，加 `transport/postMessage` endpoint |

---

## 10. R9/R10 retry 任务状态（不在本任务范围，仅备查）

- **R9-MCP-001** (e89819f8): V1123 commit `72bbc82f` 已在 master HEAD 祖先，无需重做（系统自动重派的 retry 不需重复操作）。
- **R10-MCP-001** (7ac568a6): V1129 W1 commit `ab89669b` 已在 master HEAD 中，accepted 9.00，无需重做。

两者作为 V1129 W2 的基础服务持续在 master 中可用。

---

## 11. 与其他 R10 W2 任务的协同

| 协同任务 | 关系 | 集成点 |
|----------|------|--------|
| R10-AO-002 V1127 DGM v0.5 真跑验证 | 上下游 | `evolve_dgm` tool 调用 `V05MultiAgentCoordinator.run()` 真演化；AO-002 验证 fitness 单调性 |
| R10-A2-002 V1129 multi-agent 验证 | 平级 | `multi_agent_consensus` tool 调用 `V1128MultiAgentIntegrationProtocol.measure_multi_agent`；A2-002 验证 V0.5 18 维公式 |
| R10-DEV-001 V1130 release window guard | 旁路 | V1129 W2 作为 W2 release candidate，触发 V1130 release window 检查 |
| R10-ATE-001 V1127 跨小模型 CI | 旁路 | V1129 W2 CI 包含 CI 框架的 ASI 北极星守护测试 |

---

## 12. ASI 北极星 0.95 路径

| 阶段 | 工具 | ASI 测量 |
|------|------|----------|
| V1123 | R9 MCP base | n/a |
| V1129 W1 | measure_asi / get_north_star / check_identity / verify_audit_chain / list_personas | V0.4=0.8538 |
| **V1129 W2** | **+ multi_agent_consensus / evolve_dgm / multi_agent_asi_level** | **V0.5 multi_agent_consensus ≥ 0.85 → 共识通过** |
| R10 终极门 | measure_asi (V0.5 18 维) | **≥ 0.95** |

本任务不声称达到 0.95；0.95 是 R10 终极门，需要 R10-W3+ 真实中央 AI 协同持续达到。本任务负责把多 agent 真协同测量 + DGM 真演化触发通过 MCP 工具暴露给上游 AI agents。

---

## 13. 文件清单

| 文件 | 行数 | 内容 |
|------|------|------|
| `apeireth/v1129_r10_mcp_multi_agent.py` | 681 | V1129 W2 multi-agent module（要求 ≥300 ✓）|
| `tests/test_v1129_r10_mcp_multi_agent.py` | 503 | 28 个真行为测试（要求 ≥25 ✓）|
| `reports/r10-mcp-integration-expert-w2-multi-agent-report.md` | 本报告 | R10 W2 验收 |

总 LOC: 1184 + 报告。

---

## 14. 真 commit（待 git commit 提交）

```
(pending) R10-MCP-002: V1129 R10 W2 multi-agent MCP server + V1127 DGM v0.5 集成
  - apeireth/v1129_r10_mcp_multi_agent.py               681L
  - tests/test_v1129_r10_mcp_multi_agent.py             503L / 28 真测试
  - reports/r10-mcp-integration-expert-w2-multi-agent-report.md
```

---

主 22:33 + 主 17:43 + 主 17:58 + 主 23:44 + 主 13:31 + 主 00:56 + 主 19:33 七大主哲学落地。

ASI 北极星 0.95 LOCKED。V1129 W2 是 R10 MCP 多 agent 接入层，不是 ASI 本身（V3 守门明示）。
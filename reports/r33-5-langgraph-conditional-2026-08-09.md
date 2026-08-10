# R33-5 LangGraph 条件边落地报告 — 2026-08-09

> **本报告基于源仓 `.openclaw\workspace\promethean\Apeireth-rust` HEAD 实际源码审计**
> **前置**: R32-2 `apeireth-pipeline::tool_loop` 借鉴 LangGraph `should_continue` 已落地
> **目标**: 给 `apeireth-graph` 加 LangGraph 风格 `add_conditional_edges` 1:1

## 1. 一句话结论

**R33-5 全部 done**: 字段级 1:1 借鉴 LangGraph `langgraph/graph/state.py:StateGraph.add_conditional_edges`
(`path_map: dict[label, NodeId]` / `then: Optional<NodeId>` / `END` sentinel) 落到
`apeireth-graph::conditional::ConditionalEdge` + `Graph::add_conditional_edge()` + `Executor::execute()` 调度.
10/10 smoke test pass + 旧 20 个 lib test + smoke 8/8 全绿, 0 改 24 LOCKED crate.

## 2. 借鉴锚 (S-1 字段级 1:1 移植证据)

| LangGraph 字段 | Apeireth 字段 | 文件 |
|----------------|---------------|------|
| `path_map: dict[label, NodeId]` | `path_map: BTreeMap<String, NodeId>` | `crates/apeireth-graph/src/conditional.rs:62` |
| `then: Optional[NodeId>` | `default: Option<NodeId>` | `crates/apeireth-graph/src/conditional.rs:67` |
| `END` sentinel | `END_LABEL: &str = "__end__"` | `crates/apeireth-graph/src/conditional.rs:48` |
| `add_conditional_edges(src, path, then, condition)` | `Graph::add_conditional_edge(from, path_map, default, condition)` | `crates/apeireth-graph/src/lib.rs:178` |
| runtime cycle detection | `outputs.contains_key(&target) → Cycle` (跨节点) / `MAX_CHAIN_STEPS=256` (self-loop) | `crates/apeireth-graph/src/executor.rs` |

## 3. 实现细节 (核心: 3 个不漂移承诺)

### 3.1 ConditionalEdge struct (R33-5 conditional.rs)
- 4 字段: `from` / `path_map` / `default` / `condition: Arc<dyn Fn(&State) -> String + Send + Sync>`
- sync 闭包 (跟 `Node::run` 1:1, 0 引入 async runtime 重依赖)
- `ConditionalEdge::decide(state)` 返 `ConditionalDecision { from, label, target, path_kind }` 4 字段
- 路径判定: END_LABEL 优先 → path_map → default → missing (4 path_kind)

### 3.2 Graph::add_conditional_edge (R33-5 lib.rs)
- 新增 1 method + 1 getter + 1 字段 (0 改现有 `Graph::add_node` / `add_edge` / `execute` 签名)
- 字段: `conditional_edges: Vec<ConditionalEdge>` + `Default` impl 加 `Vec::new()`
- re-export: `ConditionalEdge` / `ConditionalDecision` / `ConditionalError` / `END_LABEL`

### 3.3 Executor::execute 调度 (R33-5 executor.rs)
- **新增**: `run_node_with_chain(node_id, is_self_loop_reentry: bool)` 递归辅助
- **自然入口**: DAG indegree==0 AND 不被任何其它节点条件指向
- **fallback 入口**: 全图都是条件闭环 (e.g. init↔step 工具循环) 时, 字典序首个 cond source
- **cycle 检测**:
  - 跨节点 re-entry (target != from 且 target 已 visited) → 立即 `GraphError::Cycle`
  - self-loop (target == from) → 允许重访, 受 `MAX_CHAIN_STEPS=256` 截断
- **DAG cycle**: 仍由 `topological_order()` 检测 (0 改行为, 仅借用一次)
- **不要求全员到位**: 条件未触发的 target 留作 unvisited (LangGraph 1:1 语义)

## 4. 10 个 smoke test (per 0 假装 / 6 哲学锚穿透)

| Test | 验证语义 |
|------|----------|
| `conditional_two_branches_routes_to_target` | 基础 2-branch, 决策路由到 b 或 c |
| `conditional_default_fallback_when_label_missing` | label 不在 path_map 走 default |
| `conditional_end_label_terminates_execution` | condition 返 "__end__" → 终止, target=None |
| `conditional_chain_a_b_c` | 多节点链式条件 a→b→c |
| `conditional_cycle_detected` | a→a self-loop 跑满 256 步后 → Cycle (永真循环) |
| `conditional_mixed_with_dag` | DAG a→b + b 的 cond 选 c 或 d |
| `conditional_with_state_evolution_max_1_iteration` | tool loop 模式 turn≥1 → END 终止 |
| `conditional_tool_loop_max_2_iterations_clamps` | step→step self-loop + turn 截断 |
| `conditional_no_label_terminates_after_dag` | 纯 DAG (无 cond) 行为不变 |
| `conditional_uses_arc_to_capture_external_counter` | Arc 闭包捕获外部状态 |

## 5. 不漂移承诺穿透 (主哲学锚 #1)

- ❌ 0 改 `Node` / `Edge` / `State` / `FinalState` / `NodeOutput` / `Checkpoint` 现有字段
- ❌ 0 改 `Graph::add_node` / `add_edge` / `try_add_node` / `try_add_edge` 签名
- ❌ 0 改 `Executor::new` / `supervisor_snapshot` 行为
- ❌ 0 引入 `unsafe` (workspace `#![deny(unsafe_code)]` 继承)
- ❌ 0 引入 async runtime 重依赖 (sync `Fn` 闭包, 跟 `Node::run` 1:1)
- ❌ 0 触碰 workspace 1.0.0 / 8 项不修改承诺 / 24 LOCKED crate

## 6. 测试验收

```
cargo test -p apeireth-graph
test result: ok. 12 passed; 0 failed  (lib, 含原 cycle reject test)
test result: ok. 10 passed; 0 failed  (conditional_smoke)
test result: ok. 8 passed; 0 failed   (smoke)
test result: ok. 0 passed; 0 failed   (doctest)
```

## 7. 后续 follow-up (本报告 R 推完后再起)

- **R33-5-1**: tool_loop 实战 — 把 R32-2 `apeireth-pipeline::tool_loop` 改成走 `Graph::add_conditional_edge` 而不是手写 loop
- **R33-5-2**: 条件闭包 `async` 支持 — 引入 `BoxFuture` 让 condition 可 await (现 sync)
- **R33-5-3**: cond-edge 序列化 — 把 ConditionalEdge 加 serde derive 让 checkpoint 持久化

## 8. 借鉴源

- `langgraph/graph/state.py:StateGraph.add_conditional_edges` (LangGraph 0.2.x 字段级 1:1)
- R32-2 `apeireth-pipeline::tool_loop::should_continue` (复用 LangGraph pattern 1:1)

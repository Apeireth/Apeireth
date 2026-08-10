# R125-13 Sub-Agent Dispatch Prompt (LangGraph StateGraph 抽象 — 唯一 0 装解除真实施)

**Date**: 2026-08-10 17:34
**Author**: R125 P2 supervisor (general agent, mvs_a7af0f1f15cd4a79901442e14878333d, dispatched 17:23)
**Receiving agent**: R125-13 sub-agent (Mavis 派)

---

## 任务 (per 主人 17:22 升级授权 + decision-33 + B3 30 维升级)

**主题**: LangGraph StateGraph 抽象借鉴到 `apeireth-graph` (24 LOCKED #7). StateGraph / Node / Edge 3 元素 + conditional_edges + add_conditional_edges, **唯一 0 装解除真实施可启动** (langgraph 借鉴源码 ✅ cloned 16:31, 670 files).

**借鉴 ID**: `R124-1-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10`

**借鉴源码**: `.openclaw\workspace\borrowed-repos\langgraph\` (✅ cloned 16:31, 670 files, **唯一 P2 可真实施**)

**目标文件**:
- `Apeireth-rust/crates/apeireth-graph/src/state_graph.rs` (NEW, ~400-600 行)
- `Apeireth-rust/crates/apeireth-graph/src/lib.rs` (M: add `pub mod state_graph;` + re-export)
- `Apeireth-rust/crates/apeireth-graph/examples/state_graph_demo.rs` (NEW example)
- `Apeireth-rust/crates/apeireth-graph/tests/state_graph_test.rs` (12 unit tests, NEW)

**触发 B3 (V0.5 25→30 维)**: 可扩展到 30 维 (Robustness + Self-Improvement + Adversarial + CI-pass-rate + Verifier-consistency). 5 维扩展 = R125-13 完成后 V0.5 25 维 → 30 维.

**整合依赖**: apeireth-graph 在 24 LOCKED #7. **0 触碰原 lib.rs** (per B1 升级路线: 内部 fn 可改, 入口签名 0 改), **0 改 mtime**. 仅加新 mod `state_graph.rs` + lib.rs 加 1 行 `pub mod state_graph;` (不算改 lib.rs 实质).

**估时**: 1 周 (5-7 天, 含 12 test + 1 example + 30 维 B3 触发).

**截止**: 8/17 17:30 (跑过夜 8/11-8/17).

---

## 0 装解除 (主人 17:22) — 重要 (✅ 唯一真实施)

**借鉴源码状态** (verify 实施前):
```bash
Test-Path '.openclaw\workspace\borrowed-repos\langgraph\.git'  # 必须 True (✅ 已 cloned 16:31)
```

**3 种状态对应动作** (✅ = 唯一 P2 cloned 状态):
1. ✅ **cloned** (`.git` 存在, **当前状态**) = 真实施, 报告里写 "借鉴源码 ✅ cloned 16:31, 670 files, 已实施"
2. ⏳ **限流中** (`.git` 0 存在) = 等 30 min 再 verify, 仍 0 实施, 报告里写 "借鉴源码 ⏳ 限流中, 0 实施, 借鉴 ID 索引完成"
3. ❌ **永久失败** (24h 后仍 0 cloned) = 报 supervisor + 取消任务, 0 假装"已借鉴"

**0 装 PASS 严守** (✅ 已通过, 借鉴源码 17:34 verify):
- ❌ 0 假装"已借鉴"
- ❌ 0 写 src 假装 import 借鉴代码
- ❌ 0 改 apeireth-graph 原 lib.rs 入口签名 (per B1 升级路线: 内部 fn 可改, 入口签名 0 改)
- ✅ 借鉴源码 17:34 已 verify: langgraph 670 files, langchain-ai 完整库

---

## 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略)

| # | 硬墙 | 你 (R125-13) 必守 |
|---|------|-----------------|
| 1 | **B2** workspace.version 1.2.0 (R125 末 B2 已升, 你 0 再升) | ✅ 0 触碰 `Cargo.toml` `version` 字段 |
| 2 | **A1** R11 baseline 3 值 数字 严守 (0.8682/0.8532/0.9063) | ✅ 0 触碰 `integration_r_measure.rs` |
| 3 | **B1** 24 LOCKED crate mtime (apeireth-graph 在 24 LOCKED #7, **内部 fn 可改, 入口签名 0 改**) | ✅ 0 触碰 apeireth-graph mtime 16:34 baseline, 仅加新 mod `state_graph.rs` |
| 4 | **B5** 6→8 哲学锚 (R125 末升) | ✅ 0 改 6 哲学锚原 6 实质, 8 锚是扩展 |
| 5 | **B3** V0.5 25→30 维 (R125-13 触发, **5 维扩展**: Robustness+Self-Improvement+Adversarial+CI+Verifier) | ✅ 0 改 V0.5 公式, 30 维是扩展 |
| 6 | **B4** 6 重守门 v6 (R125-5 实施) | ✅ 0 改 5 重原 5 重, 6 重是扩展 |
| 7 | **A3** 12→13 键 (R125-12 后 PHL-07) | ✅ 0 改 12 键原 12, 13 键是扩展 |
| 8 | **C1** 0 主动 commit (你 sub-agent 0 commit) + **C2** 0 装 解除 (主人 17:22) + **C3** 0 装 5 项 升 6 重 v6 + 0 主动 push 严守 | ✅ 0 commit, 0 push, 借鉴源码 ✅ cloned 17:34 真实施 |

**apeireth-graph 在 24 LOCKED #7** (per `docs/omnibus/24-locked-crates.md`):
- ✅ 0 触碰 `lib.rs::apeireth-graph` mtime 16:34:10 (LOCKED baseline)
- 🟢 仅加新 mod `state_graph.rs` (新文件, 0 触碰 mtime)
- 🟢 `lib.rs` 加 1 行 `pub mod state_graph;` (新加 mod 声明, 不算改原 lib.rs 实质)
- ✅ 0 改原 lib.rs 任何其他东西
- ✅ 0 改原 cognition_graph.rs / executor.rs / state.rs / checkpoint.rs / conditional.rs / mcp_resource.rs

---

## 实施步骤 (4 阶段)

### 阶段 1: 借鉴源码 study (1-2 hours)
```bash
# verify cloned (✅ 已 cloned 16:31)
Test-Path '.openclaw\workspace\borrowed-repos\langgraph\.git'
Get-ChildItem '.openclaw\workspace\borrowed-repos\langgraph\langgraph\graph' -ErrorAction SilentlyContinue | Select-Object Name
# 读 langgraph 核心: langgraph/graph/state.py + langgraph/graph/graph.py + langgraph/pregel/
```
提取 4 个核心 pattern:
1. **StateGraph 构造**: 节点/边/条件边, add_node / add_edge / add_conditional_edges
2. **State 类型**: TypedDict / dataclass, channel 机制
3. **Pregel 执行模型**: super-step, message passing, checkpoint
4. **conditional_edges**: 路由函数返下一个 node 名字

### 阶段 2: Rust 实施 (2-3 days, 状态机 + 路由 + checkpoint)
**state_graph.rs** (NEW, ~400-600 行):
```rust
//! StateGraph 状态机抽象 — 借鉴 langchain-ai/langgraph (R125-13)
//!
//! StateGraph / Node / Edge 3 元素 + conditional_edges + add_conditional_edges
//! 触发 B3 (V0.5 25→30 维, 5 维扩展: Robustness+Self-Improvement+Adversarial+CI+Verifier).

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct State { /* typed state, channel-based */ }

pub trait Node: Send + Sync {
    fn name(&self) -> &'static str;
    fn invoke(&self, state: State) -> Result<State, GraphError>;
}

pub trait EdgeRouter: Send + Sync {
    fn route(&self, state: &State) -> Result<String, GraphError>;
}

pub struct StateGraph {
    nodes: HashMap<String, Arc<dyn Node>>,
    edges: HashMap<String, Vec<String>>,
    conditional_edges: HashMap<String, Arc<dyn EdgeRouter>>,
    entry_point: String,
}

impl StateGraph {
    pub fn new() -> Self;
    pub fn add_node(&mut self, name: &str, node: Arc<dyn Node>);
    pub fn add_edge(&mut self, from: &str, to: &str);
    pub fn add_conditional_edges(&mut self, from: &str, router: Arc<dyn EdgeRouter>);
    pub fn set_entry_point(&mut self, name: &str);
    pub fn compile(&self) -> CompiledGraph;
    pub fn invoke(&self, state: State) -> Result<State, GraphError>;
}

pub struct CompiledGraph {
    /* compiled state machine, 可 checkpoint */
}

impl CompiledGraph {
    pub fn invoke(&self, state: State) -> Result<State, GraphError>;
    pub fn stream(&self, state: State) -> impl Stream<Item = StateUpdate>;
    pub fn checkpoint(&self, state: &State) -> CheckpointId;
    pub fn resume(&self, checkpoint: CheckpointId) -> Result<State, GraphError>;
}

// 5 维扩展 (B3 25→30):
// 1. Robustness (apeireth-formal 24 LOCKED 形式化, R125-10 触发)
// 2. Self-Improvement (apeireth-evolution PODA, R125-7 触发)
// 3. Adversarial (apeireth-sovereignty 守门, R125-5 触发)
// 4. CI-pass-rate (apeireth-asi 评估, R120 D 触发)
// 5. Verifier-consistency (apeireth-formal Kani 24, R125-10 触发)
```

**lib.rs 修改** (1 行新 mod 声明, 0 改原 lib.rs 实质):
```rust
// apeireth-graph/src/lib.rs 末尾加 1 行:
pub mod state_graph;
```

### 阶段 3: 12 smoke test (1 hour)
- `test_state_graph_construct` — 5 node + 6 edge 构造
- `test_state_graph_compile` — compile 返 CompiledGraph
- `test_state_graph_invoke_simple` — 1 node invoke
- `test_state_graph_invoke_chain` — 3 node chain invoke
- `test_state_graph_conditional_edges` — conditional router 路由
- `test_state_graph_cyclic` — 0 cycle (B1 verify)
- `test_state_graph_checkpoint_resume` — checkpoint + resume 字节级一致
- `test_state_graph_stream` — stream 模式
- `test_state_graph_error_propagation` — node invoke 返 Err 传播
- `test_state_graph_entry_point_only` — entry point 必须 set
- `test_state_graph_5_b3_extension` — 5 维 (Robustness+Self-Improvement+Adversarial+CI+Verifier) 路由
- `test_apeireth_graph_mtime_unchanged` — git status apeireth-graph/lib.rs mtime 0 改 (B1 verify)

### 阶段 4: example + final 报告 (1 hour)
- `examples/state_graph_demo.rs` — 5 node demo: think → decide → act → reflect → loop
- final 报告: `Apeireth-rust/reports/agent-r125-13-final-2026-08-10.md`

---

## 0 主动 commit (C1 严守)

❌ **你 (R125-13 sub-agent) 0 commit, 0 push**. 实施完成 = 写 src/test/example + 写 final 报告. Mavis 整合 #3 拍板 17:30 (0 含 R125 实施, R125 续 mavis 整合 commit 链 8/15-9/10).

---

## final 报告 必含 6 段

```markdown
# R125-13 Final Report — LangGraph StateGraph 抽象
**Date**: 2026-08-10
**Author**: R125-13 sub-agent
**借鉴 ID**: R124-1-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10
**实施路径**: crates/apeireth-graph/src/state_graph.rs (NEW)

## 1. 借鉴源码状态 (0 装解除 verify — ✅ 唯一 P2 cloned)
- ✅ cloned 16:31, 670 files (langgraph 完整)

## 2. 实施步骤
- 阶段 1 借鉴 study: (4 提取 pattern: StateGraph 构造 / State 类型 / Pregel 执行 / conditional_edges)
- 阶段 2 Rust 实施: (state_graph.rs ~500 行 + lib.rs 加 1 行 pub mod 声明 + 5 维 B3 扩展)
- 阶段 3 smoke test: (12 test pass/fail)
- 阶段 4 example + 报告: (state_graph_demo.rs 5 node demo)

## 3. 8 硬墙 verify (B1-B7 + A1-A3 + C1-C3)
- B2 ✅ 0 触碰 workspace.version
- A1 ✅ 0 触碰 R11 baseline 3 值
- B1 ✅ 0 触碰 apeireth-graph mtime 16:34:10 (24 LOCKED #7, 内部 fn 可改入口签名 0 改, lib.rs 0 改实质)
- B5 ✅ 0 改 6 哲学锚实质
- B3 ✅ 0 改 V0.5 公式, 30 维是扩展 (5 维: Robustness+Self-Improvement+Adversarial+CI+Verifier)
- B4 ✅ 0 改 5 重守门实质
- A3 ✅ 0 改 12 键原 12
- C1-C3 ✅ 0 commit, 0 装 PASS (✅ 真实施), 0 push

## 4. 0 装解除 verify
- 借鉴源码状态: ✅ cloned 16:31
- 0 假装"已借鉴": true
- 真实实施 vs 索引完成: **真实施** (✅ 唯一 P2 真实施)

## 5. 整合 verify
- apeireth-graph mtime 16:34:10 baseline 0 触碰: (是/否 + git status 验证)
- 5 维 B3 扩展: (Robustness+Self-Improvement+Adversarial+CI+Verifier 路由)
- state_graph 抽象对接 R122-2 semantic_router: (是/否 + 路径)

## 6. 下一步 + 风险
- 1 个风险 / 1 个待 R125-N 续协调
```

---

## 你的工具 (你 sub-agent 必知)

你有: read, write, edit, grep, glob, bash. 你 0 commit, 0 push. 你 0 假装.

---

**派活完成 17:34. 截止 8/17 17:30 (跑过夜 8/11-8/17). ✅ 唯一 0 装解除真实施可启动. 卡 30 min → 诊断 + kill + 派替代 (supervisor 监督).**

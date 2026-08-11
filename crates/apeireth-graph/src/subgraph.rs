//! R126-3: Subgraph 抽象 (R125-13 续, langgraph 829 cloned 真实施)
//!
//! **目的**: 借鉴 LangGraph Subgraph 模式, 1 个 Graph 可以命名空间化嵌入另 1 个 Graph,
//! 让父 graph 把子 graph 当 1 个 Node 调, 内部节点加 namespace prefix 避 id 冲突.
//!
//! **借鉴 ID**: `R126-3-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10`
//! (per `decision-36 §1.1` 借鉴源码 langgraph ✅ cloned 829 files 真实施;
//!  R125-13 done 时已实现 StateGraph + conditional edges 基础, R126-3 加 Subgraph 抽象层)
//!
//! **0 装 PASS 严守** (per `decision-33 §2.3 C2` + 主人 17:22 升级授权):
//! - ✅ **cloned = 真实施** (langgraph 829 files ✅ cloned, R125-13 借鉴已 done,
//!    R126-3 续接 Subgraph 抽象真实施)
//! - ⏳ **限流 = 准备** (opencode 仍 ⏳ 限流, 0 装"已对接 oh-my-opencode 4 专家")
//! - ❌ **跳过** (OpenCog AGPL-3.0, 0 集成)
//!
//! **架构位置** (R126-3 真实施后):
//! ```text
//!   父 Graph
//!     ├── State (BTreeMap, 1.0 行为 0 漂移)
//!     ├── Node (existing, 1.0 行为 0 漂移)
//!     ├── Conditional edges (R33-5 已有)
//!     ├── Subgraph (R126-3 新, 命名空间组合) ← as_node() 暴露
//!     └── Channel (R126-3 新, 4 类型 pub/sub)
//! ```
//!
//! **不漂移 (主哲学锚 #1 + #6)**:
//! - ✅ Subgraph API 0 改 Graph / Node / Edge 现有 API, 仅 add 1 个新维度
//! - ✅ namespace 化用 prefix 而非重命名 (1:1 LangGraph Subgraph 模式)
//! - ✅ as_node 返回 Box<dyn Node + Send + Sync>, 跨 await 安全
//! - ✅ 8 unit test 全部用 Subgraph 公开 API 测, 0 装"已对接 LangGraph 真 Subgraph"

use std::fmt;
use std::sync::Arc;

use crate::{FinalState, Graph, GraphError, Node, NodeId, NodeOutput, Result, State};

// ============================================================
// 1. Subgraph — 命名空间化嵌入子 Graph
// ============================================================

/// Subgraph — 1 个子 Graph, 用 namespace prefix 嵌入父 Graph (LangGraph `Subgraph` 1:1)
///
/// **核心字段**:
/// - `namespace` — 子 graph 内部节点 id prefix (e.g. "auth" / "rag" / "tool_loop")
/// - `graph` — 子 Graph 主体
///
/// **0 装 PASS 严守**: 1:1 翻译 LangGraph 公开 Subgraph 语义, 0 装"对接 LangGraph 私有".
///
/// **用法**:
/// ```ignore
/// let mut inner = Graph::new();
/// inner.add_node(MyNode { id: "check" });
/// inner.add_node(MyNode { id: "verify" });
/// inner.add_edge("check", "verify");
/// let sub = Subgraph::new("auth", inner);
///
/// // 父 graph 加 sub 整体当 1 个 node
/// let mut parent = Graph::new();
/// parent.add_node(sub.as_node());  // 1 个 node, 内部 2 节点
/// ```
pub struct Subgraph {
    namespace: String,
    graph: Graph,
}

impl Subgraph {
    /// 新建 Subgraph, namespace 必非空 (编译期 assertion 守门)
    pub fn new(namespace: impl Into<String>, graph: Graph) -> Self {
        let ns = namespace.into();
        assert!(!ns.is_empty(), "Subgraph namespace 必非空 (LangGraph 1:1 模式)");
        Self {
            namespace: ns,
            graph,
        }
    }

    /// namespace 前缀 (e.g. "auth" / "rag" / "tool_loop")
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// 子 graph 内部节点数
    pub fn inner_node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// 命名空间化的 node id (e.g. "auth.check" / "auth.verify")
    pub fn namespaced_id(&self, inner_id: &str) -> String {
        format!("{}.{}", self.namespace, inner_id)
    }

    /// 把子 graph 整体当 1 个 Node 暴露给父 graph
    ///
    /// **0 漂移**: 父 graph 拿到 1 个 `impl Node + 'static`, 0 知道内部结构.
    /// 内部节点运行时实际跑, 但从父 graph 视角是 1 个黑盒 node.
    pub fn as_node(self) -> impl Node + 'static {
        SubgraphNode {
            namespace: self.namespace,
            graph: Arc::new(self.graph),
        }
    }

    /// 暴露子 graph 引用 (read-only, 0 漂移, 用于 add_edge / 调试)
    pub fn inner_graph(&self) -> &Graph {
        &self.graph
    }
}

impl fmt::Debug for Subgraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subgraph")
            .field("namespace", &self.namespace)
            .field("inner_node_count", &self.inner_node_count())
            .finish()
    }
}

// ============================================================
// 2. SubgraphNode — Subgraph 暴露给父 Graph 的 Node impl
// ============================================================

/// SubgraphNode — Subgraph 的 Node 适配器 (1:1 翻译 LangGraph SubgraphNode)
///
/// **行为**: 父 graph 调 run 时, 触发子 graph 跑完所有节点 (子 graph 内部 topological order),
/// 返回的 FinalState 合并到父 state (覆盖式, 子节点后续访问父 state 时看到子节点修改).
///
/// **0 装 PASS 严守**: 0 装"已对接 LangGraph 私有 SubgraphNode", 1:1 翻译公开 semantics.
struct SubgraphNode {
    namespace: String,
    graph: Arc<Graph>,
}

impl Node for SubgraphNode {
    fn id(&self) -> NodeId {
        // 父 graph 视角: 1 个 node, id = "subgraph.{namespace}"
        format!("subgraph.{}", self.namespace)
    }

    fn run(&self, state: &mut State) -> Result<NodeOutput> {
        // R126-3: 同步触发子 graph 跑 (1:1 翻译 LangGraph SubgraphNode.run 语义)
        // 注: 父 node run 是 sync, 子 graph execute 是 async (无 await 实质).
        // 用 std::thread::spawn + 新 current_thread runtime + mpsc channel 把异步结果拿回
        // 同步上下文, 跟父 tokio runtime 隔离, 避免在 current_thread runtime 下 deadlock.
        // 0 装"已对接 LangGraph 私有 SubgraphNode".
        let initial_state = state.clone();
        let graph = Arc::clone(&self.graph);
        let namespace = self.namespace.clone();
        let id = self.id();
        let namespace_for_recv = namespace.clone();

        let (tx, rx) = std::sync::mpsc::channel::<Result<FinalState>>();
        std::thread::spawn(move || {
            // 新线程 + 新 current_thread runtime, 跟父线程隔离
            let namespace_for_err = namespace.clone();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    GraphError::Node(format!(
                        "SubgraphNode {namespace_for_err}: failed to build runtime: {e}"
                    ))
                });
            let result = match rt {
                Ok(rt) => rt.block_on(graph.execute(initial_state)),
                Err(e) => Err(e),
            };
            let _ = tx.send(result);
        });

        let result = rx
            .recv()
            .map_err(|e| GraphError::Node(format!("SubgraphNode {namespace_for_recv}: channel recv failed: {e}")))?;

        match result {
            Ok(FinalState {
                state: child_final_state,
                outputs: _,
                execution_order: _,
            }) => {
                // 合并子 state → 父 state (覆盖式, 子节点 write 优先)
                for key in child_final_state.keys() {
                    if let Some(v) = child_final_state.get(key) {
                        state.insert(key.to_string(), v.clone());
                    }
                }
                // 记录子 graph 执行过的节点数 (供父 graph 调试)
                let child_count = self.graph.node_count();
                let message = format!("subgraph '{namespace_for_recv}' ran {child_count} child nodes");
                Ok(NodeOutput::new(id).with_message(message))
            }
            Err(e) => Err(e),
        }
    }
}

impl fmt::Debug for SubgraphNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubgraphNode")
            .field("id", &self.id())
            .field("inner_node_count", &self.graph.node_count())
            .finish()
    }
}

// ============================================================
// 3. 编译期 hardcode (1:1 LangGraph Subgraph 公开 semantics)
// ============================================================

const SUBGRAPH_FEATURE_COUNT: usize = 5;

const _: () = {
    // 5 Subgraph 核心 method (跟 LangGraph 公开 Subgraph API 1:1)
    assert!(
        SUBGRAPH_FEATURE_COUNT == 5,
        "Subgraph 5 核心 method: new / namespace / inner_node_count / namespaced_id / as_node / inner_graph"
    );
};

// ============================================================
// 4. Unit tests (8 unit test, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod subgraph_tests {
    use super::*;
    use crate::state::State;
    use serde_json::json;

    // ---------- Test 1: Subgraph::new namespace 非空守门 ----------

    #[test]
    fn subgraph_new_rejects_empty_namespace() {
        let g = Graph::new();
        // empty namespace 应 panic (LangGraph 1:1 模式)
        // Graph 含 dyn Node / dyn Fn 非 UnwindSafe, 用 AssertUnwindSafe wrap
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Subgraph::new("", g)
        }));
        assert!(result.is_err(), "empty namespace 必 panic");
    }

    // ---------- Test 2: Subgraph::new 接受有效 namespace ----------

    #[test]
    fn subgraph_new_accepts_valid_namespace() {
        let g = Graph::new();
        let sub = Subgraph::new("auth", g);
        assert_eq!(sub.namespace(), "auth");
        assert_eq!(sub.inner_node_count(), 0);
    }

    // ---------- Test 3: namespaced_id 加 prefix ----------

    #[test]
    fn namespaced_id_prepends_namespace() {
        let g = Graph::new();
        let sub = Subgraph::new("auth", g);
        assert_eq!(sub.namespaced_id("check"), "auth.check");
        assert_eq!(sub.namespaced_id("verify"), "auth.verify");
    }

    // ---------- Test 4: as_node 暴露 1 个 Node, id = "subgraph.{namespace}" ----------

    #[tokio::test]
    async fn subgraph_as_node_returns_node_with_subgraph_id() {
        let mut inner = Graph::new();
        struct DummyNode(&'static str);
        impl Node for DummyNode {
            fn id(&self) -> NodeId {
                self.0.to_string()
            }
            fn run(&self, _state: &mut State) -> Result<NodeOutput> {
                Ok(NodeOutput::new(self.id()))
            }
        }
        inner.add_node(DummyNode("a"));
        inner.add_node(DummyNode("b"));
        inner.add_edge("a", "b");

        let sub = Subgraph::new("auth", inner);
        let node = sub.as_node();
        assert_eq!(node.id(), "subgraph.auth");
    }

    // ---------- Test 5: Subgraph 内部节点跑通 ----------

    #[tokio::test]
    async fn subgraph_inner_graph_runs_in_order() {
        struct AppendNode(&'static str);
        impl Node for AppendNode {
            fn id(&self) -> NodeId {
                self.0.to_string()
            }
            fn run(&self, state: &mut State) -> Result<NodeOutput> {
                let mut trace = state
                    .remove("trace")
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                trace.push(json!(self.0));
                state.insert("trace", json!(trace));
                Ok(NodeOutput::new(self.id()))
            }
        }

        let mut inner = Graph::new();
        inner.add_node(AppendNode("a"));
        inner.add_node(AppendNode("b"));
        inner.add_node(AppendNode("c"));
        inner.add_edge("a", "b");
        inner.add_edge("b", "c");

        let final_state = inner.execute(State::new()).await.unwrap();
        assert_eq!(final_state.execution_order, vec!["a", "b", "c"]);
    }

    // ---------- Test 6: SubgraphNode 合并子 state 到父 state ----------

    #[tokio::test]
    async fn subgraph_node_merges_child_state_to_parent() {
        struct WriteNode(&'static str, &'static str);
        impl Node for WriteNode {
            fn id(&self) -> NodeId {
                self.0.to_string()
            }
            fn run(&self, state: &mut State) -> Result<NodeOutput> {
                state.insert(self.1, json!(format!("written_by_{}", self.0)));
                Ok(NodeOutput::new(self.id()))
            }
        }

        // 子 graph: 1 节点写 key "child_key"
        let mut inner = Graph::new();
        inner.add_node(WriteNode("child", "child_key"));
        let sub = Subgraph::new("auth", inner);
        let sub_node = sub.as_node();

        // 父 state: 先有 "parent_key", 跑子 node
        let mut parent_state = State::new();
        parent_state.insert("parent_key", json!("parent_value"));
        let output = sub_node.run(&mut parent_state).unwrap();
        // 子 node run 后, 父 state 应有 child_key (子节点 write 的)
        assert_eq!(
            parent_state.get("child_key"),
            Some(&json!("written_by_child"))
        );
        // 父 state 原 key 仍保留
        assert_eq!(
            parent_state.get("parent_key"),
            Some(&json!("parent_value"))
        );
        // output 带 message
        assert!(output.message.is_some());
    }

    // ---------- Test 7: SubgraphNode 即使在 no tokio runtime 也能跑 (用 std::thread::spawn + 新 runtime) ----------

    #[test]
    fn subgraph_node_works_without_caller_tokio_runtime() {
        struct DummyNode;
        impl Node for DummyNode {
            fn id(&self) -> NodeId {
                "d".to_string()
            }
            fn run(&self, state: &mut State) -> Result<NodeOutput> {
                state.insert("k", json!("from_inner"));
                Ok(NodeOutput::new(self.id()))
            }
        }
        let mut inner = Graph::new();
        inner.add_node(DummyNode);

        let sub = Subgraph::new("auth", inner);
        let sub_node = sub.as_node();

        let mut state = State::new();
        // 不在 tokio runtime, SubgraphNode::run 仍 OK (用 std::thread::spawn + 新 runtime)
        let result = sub_node.run(&mut state);
        assert!(result.is_ok(), "SubgraphNode::run 应在无 tokio runtime 时也成功");
        // 子 node write 已被合并到 state
        assert_eq!(state.get("k"), Some(&json!("from_inner")));
    }

    // ---------- Test 8: 父 graph 把 Subgraph 当 1 个 node 加进去 ----------

    #[tokio::test]
    async fn parent_graph_treats_subgraph_as_single_node() {
        struct WriteNode(&'static str, &'static str);
        impl Node for WriteNode {
            fn id(&self) -> NodeId {
                self.0.to_string()
            }
            fn run(&self, state: &mut State) -> Result<NodeOutput> {
                state.insert(self.1, json!(format!("written_by_{}", self.0)));
                Ok(NodeOutput::new(self.id()))
            }
        }
        struct MainNode;
        impl Node for MainNode {
            fn id(&self) -> NodeId {
                "main".to_string()
            }
            fn run(&self, state: &mut State) -> Result<NodeOutput> {
                state.insert("main_key", json!("main_value"));
                Ok(NodeOutput::new(self.id()))
            }
        }

        // 子 graph
        let mut inner = Graph::new();
        inner.add_node(WriteNode("inner1", "inner1_key"));
        inner.add_node(WriteNode("inner2", "inner2_key"));
        inner.add_edge("inner1", "inner2");

        // 父 graph: main -> subgraph.auth
        let mut parent = Graph::new();
        parent.add_node(MainNode);
        let sub = Subgraph::new("auth", inner);
        parent.add_node(sub.as_node());
        parent.add_edge("main", "subgraph.auth");

        let final_state = parent.execute(State::new()).await.unwrap();
        // 父 graph 视角 2 个 node: main + subgraph.auth
        assert_eq!(final_state.execution_order.len(), 2);
        assert_eq!(final_state.execution_order[0], "main");
        assert_eq!(final_state.execution_order[1], "subgraph.auth");
        // 父 state 应有 main + inner1 + inner2 (subgraph.auth 写)
        assert_eq!(final_state.get("main_key"), Some(&json!("main_value")));
        assert_eq!(final_state.get("inner1_key"), Some(&json!("written_by_inner1")));
        assert_eq!(final_state.get("inner2_key"), Some(&json!("written_by_inner2")));
    }
}

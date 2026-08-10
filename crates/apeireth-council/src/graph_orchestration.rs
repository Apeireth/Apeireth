//! R25 D-3: 图编排支持 (per v2.0 strategy §2B "加图编排支持")
//!
//! **职责**:
//! - 把 4 协作模式包装成 `apeireth-graph::Node` impl
//! - 提供 `CouncilGraph` 工厂: 1 模式 → 1 Graph (含 nodes + edges)
//! - 复用 R10 synthesize + 4 mode 0 漂移
//!
//! **借鉴锚** (S-1):
//! - LangGraph `Graph.add_node` + `Graph.add_edge` 1:1 复用
//! - LangGraph `subgraph` 子图作为节点 (per Hierarchical 模式)
//! - AutoGen GroupChat manager 作为 supervisor
//!
//! **0 漂移**:
//! - 0 改 apeireth-graph (R113 LOCKED)
//! - 0 改 4 协作模式 (D-3-2/3 LOCKED)
//! - 0 引入 I/O / 网络
//! - Node::run 内部走 sync 路径 (per 现有 council 哲学锚 0 引入 async LLM HTTP)

#![deny(unsafe_code)]

use crate::collaboration::types::{CollaborationMode, CollaborationVerdict};
use crate::deliberation::CouncilQuery;
use crate::synthesis::SynthesisReport;
use apeireth_graph::{Edge, Graph, Node, NodeId, NodeOutput, State};
use std::sync::Arc;

/// 把 4 协作模式包装成 apeireth-graph::Node
pub struct CollaborationNode {
    /// 节点 ID (per mode + instance)
    pub id: NodeId,
    /// 协作模式
    pub mode: CollaborationMode,
    /// query 描述 (per run-time inject)
    pub query_desc: String,
    /// 4 mode driver — 用 trait object 装 4 模式
    driver: Arc<dyn CollaborationDriver>,
}

impl CollaborationNode {
    /// 构造
    pub fn new(
        id: impl Into<NodeId>,
        mode: CollaborationMode,
        query_desc: impl Into<String>,
        driver: Arc<dyn CollaborationDriver>,
    ) -> Self {
        Self {
            id: id.into(),
            mode,
            query_desc: query_desc.into(),
            driver,
        }
    }
}

impl Node for CollaborationNode {
    fn id(&self) -> NodeId {
        self.id.clone()
    }

    fn run(&self, state: &mut State) -> apeireth_graph::Result<NodeOutput> {
        // 1. 构造 CouncilQuery (从 state 读 query_id, fallback "graph-q")
        let query_id = state
            .get("query_id")
            .and_then(|v| v.as_str())
            .unwrap_or("graph-q")
            .to_string();
        let query = CouncilQuery::new(query_id, self.query_desc.clone(), 0);

        // 2. 调 driver 跑 4 模式
        let verdict = self.driver.run(&query, self.mode);

        // 3. 写 state (per D-3 5.2 约定)
        state.insert(
            "d3.collaboration_mode".to_string(),
            serde_json::json!(verdict.mode.as_str()),
        );
        state.insert(
            "d3.collaboration_verdict".to_string(),
            serde_json::json!({
                "session_id": verdict.session_id,
                "query_id": verdict.query_id,
                "steps": verdict.steps,
                "elapsed_ms": verdict.elapsed_ms,
                "termination_reason": verdict.termination_reason,
                "weighted_score": verdict.report.weighted_score,
                "is_allowed": verdict.is_allowed(),
            }),
        );

        Ok(NodeOutput::new(self.id.clone()))
    }
}

/// 4 模式 driver trait — `CollaborationNode` 通过此 trait 调模式
pub trait CollaborationDriver: Send + Sync {
    /// 跑协作模式
    fn run(&self, query: &CouncilQuery, mode: CollaborationMode) -> CollaborationVerdict;
}

/// CouncilGraph 工厂 — 把 4 模式包成 Graph
pub struct CouncilGraph;

impl CouncilGraph {
    /// 构造 Planner+Executor 线性图 (1 planner → 3 executor 节点)
    ///
    /// **节点 ID 约定** (per LangGraph 风格):
    /// - `plan` (planner 节点)
    /// - `execute.1` / `execute.2` / `execute.3` (3 个 executor 节点)
    pub fn planner_executor_graph(driver: Arc<dyn CollaborationDriver>) -> Graph {
        let mut g = Graph::new();
        g.add_node(CollaborationNode::new(
            "plan",
            CollaborationMode::PlannerExecutor,
            "deploy auth system",
            driver.clone(),
        ));
        for i in 1..=3 {
            g.add_node(CollaborationNode::new(
                format!("execute.{i}"),
                CollaborationMode::PlannerExecutor,
                format!("step {i}"),
                driver.clone(),
            ));
        }
        g.add_edge("plan", "execute.1");
        g.add_edge("execute.1", "execute.2");
        g.add_edge("execute.2", "execute.3");
        g
    }

    /// 构造 Voting 平行图 (3 voter 节点, 0 edge)
    pub fn voting_graph(driver: Arc<dyn CollaborationDriver>) -> Graph {
        let mut g = Graph::new();
        for i in 1..=3 {
            g.add_node(CollaborationNode::new(
                format!("vote.{i}"),
                CollaborationMode::Voting,
                "vote on proposal",
                driver.clone(),
            ));
        }
        g
    }

    /// 构造 Hierarchical 图 (1 root + 2 sub 节点, root → sub.1, root → sub.2)
    pub fn hierarchical_graph(driver: Arc<dyn CollaborationDriver>) -> Graph {
        let mut g = Graph::new();
        g.add_node(CollaborationNode::new(
            "root",
            CollaborationMode::Hierarchical,
            "delegate to subs",
            driver.clone(),
        ));
        g.add_node(CollaborationNode::new(
            "sub.1",
            CollaborationMode::Hierarchical,
            "sub task 1",
            driver.clone(),
        ));
        g.add_node(CollaborationNode::new(
            "sub.2",
            CollaborationMode::Hierarchical,
            "sub task 2",
            driver.clone(),
        ));
        g.add_edge("root", "sub.1");
        g.add_edge("root", "sub.2");
        g
    }
}

/// Mock driver — 给 tests/examples 用
pub struct MockDriver {
    /// 固定 stance (per 简化, 0 调 4 模式真实 driver)
    pub fixed_stance: crate::advisor::StanceKind,
}

impl Default for MockDriver {
    fn default() -> Self {
        Self {
            fixed_stance: crate::advisor::StanceKind::Approve,
        }
    }
}

impl CollaborationDriver for MockDriver {
    fn run(&self, query: &CouncilQuery, mode: CollaborationMode) -> CollaborationVerdict {
        use crate::advisor::{AdvisorId, Stance};
        use crate::synthesis::SynthesisWeights;

        let op = crate::advisor::AdvisorOpinion::new(
            AdvisorId::new(format!("mock-{:?}", mode)),
            Stance::new(self.fixed_stance, "mock"),
            0.7,
            format!("mock run for mode {mode}"),
            0,
        )
        .with_weight(0.8);

        let report = crate::synthesis::synthesize(&[op.clone()], &SynthesisWeights::default());

        CollaborationVerdict {
            session_id: format!("mock-{}-000001", mode.as_str()),
            mode,
            query_id: query.query_id.clone(),
            report,
            opinions: vec![op],
            steps: 1,
            elapsed_ms: 0,
            termination_reason: "mock".to_string(),
        }
    }
}

// ============================================================
// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_graph::State;
    use crate::advisor::StanceKind;

    #[test]
    fn collaboration_node_new_basic() {
        let driver: Arc<dyn CollaborationDriver> = Arc::new(MockDriver::default());
        let node = CollaborationNode::new(
            "node.1",
            CollaborationMode::Voting,
            "test query",
            driver,
        );
        assert_eq!(node.id(), "node.1");
        assert_eq!(node.query_desc, "test query");
    }

    #[test]
    fn collaboration_node_run_writes_state() {
        let driver: Arc<dyn CollaborationDriver> = Arc::new(MockDriver::default());
        let node = CollaborationNode::new(
            "test.node",
            CollaborationMode::Voting,
            "test query",
            driver,
        );
        let mut state = State::new();
        state.insert("query_id".to_string(), serde_json::json!("q-graph-test"));
        let result = node.run(&mut state);
        assert!(result.is_ok());
        // state 必含 d3.collaboration_mode
        assert!(state.get("d3.collaboration_mode").is_some());
        assert!(state.get("d3.collaboration_verdict").is_some());
    }

    #[test]
    fn planner_executor_graph_has_4_nodes_3_edges() {
        let driver: Arc<dyn CollaborationDriver> = Arc::new(MockDriver::default());
        let g = CouncilGraph::planner_executor_graph(driver);
        assert_eq!(g.node_count(), 4); // plan + 3 executor
        assert_eq!(g.edges().len(), 3); // plan → execute.1 → execute.2 → execute.3
    }

    #[test]
    fn voting_graph_has_3_nodes_0_edges() {
        let driver: Arc<dyn CollaborationDriver> = Arc::new(MockDriver::default());
        let g = CouncilGraph::voting_graph(driver);
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edges().len(), 0);
    }

    #[test]
    fn hierarchical_graph_has_3_nodes_2_edges() {
        let driver: Arc<dyn CollaborationDriver> = Arc::new(MockDriver::default());
        let g = CouncilGraph::hierarchical_graph(driver);
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edges().len(), 2);
    }

    #[test]
    fn mock_driver_run_returns_approve_for_approve_stance() {
        let driver = MockDriver {
            fixed_stance: StanceKind::Approve,
        };
        let query = CouncilQuery::new("q1", "test", 0);
        let v = driver.run(&query, CollaborationMode::Voting);
        assert!(v.is_allowed());
    }

    #[test]
    fn mock_driver_run_returns_disapprove_for_disapprove_stance() {
        let driver = MockDriver {
            fixed_stance: StanceKind::Disapprove,
        };
        let query = CouncilQuery::new("q1", "test", 0);
        let v = driver.run(&query, CollaborationMode::Voting);
        assert!(!v.is_allowed());
    }

    #[tokio::test]
    async fn planner_executor_graph_executes_end_to_end() {
        let driver: Arc<dyn CollaborationDriver> = Arc::new(MockDriver::default());
        let g = CouncilGraph::planner_executor_graph(driver);
        let mut state = State::new();
        state.insert("query_id".to_string(), serde_json::json!("q-e2e"));
        let result = g.execute(state).await;
        assert!(result.is_ok());
        let final_state = result.unwrap();
        assert!(final_state.get("d3.collaboration_mode").is_some());
    }
}

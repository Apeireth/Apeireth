//! Deterministic topological graph executor.

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "supervisor-integration")]
use apeireth_supervisor::PidOneSupervisor;

use crate::{FinalState, Graph, GraphError, NodeId, Result, State};

/// Read-only evidence of the executor's supervisor wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorSnapshot {
    /// Supervisor plan observed when the executor was created.
    pub plan_version: u64,
    /// Number of child specs observed in the plan.
    pub child_count: usize,
    /// Always true while execution remains a zero-process mock integration.
    pub mocked: bool,
}

/// Executes one graph in deterministic topological order.
pub struct Executor<'graph> {
    graph: &'graph Graph,
    supervisor: SupervisorSnapshot,
}

impl<'graph> Executor<'graph> {
    /// Creates an executor with a lightweight supervisor integration.
    pub fn new(graph: &'graph Graph) -> Self {
        Self {
            graph,
            supervisor: supervisor_snapshot(),
        }
    }

    /// Returns the supervisor plan snapshot used by this executor.
    pub fn supervisor_snapshot(&self) -> SupervisorSnapshot {
        self.supervisor
    }

    /// Runs every node reachable from the graph in deterministic order.
    ///
    /// R33-5 调度语义 (per LangGraph `add_conditional_edges` 1:1):
    /// 1. **自然入口**: DAG 入度=0 且不被任何其它节点条件指向的节点.
    /// 2. **fallback 入口**: 全图都是条件闭环 (e.g. tool-loop) 时, 按字典序挑首个 cond source.
    /// 3. **运行**: 自然入口 + DAG 推进 + 条件触发并行; 条件 source 节点不推动 DAG 后继.
    /// 4. **runtime cycle**: 跨节点 re-entry (target != from 且 target 已 visited) 立即 cycle.
    /// 5. **self-loop**: target == from 允许递归重访, 受 `MAX_CHAIN_STEPS` 截断.
    /// 6. **不要求全员到位**: 条件未触发的 target 留作 unvisited (LangGraph 语义).
    pub async fn execute(&self, mut state: State) -> Result<FinalState> {
        const MAX_CHAIN_STEPS: usize = 256;
        // R33-5: 纯 DAG cycle 仍由 `topological_order` 检测 — 0 改行为, 仅借用一次.
        let _ = self.topological_order()?;
        let mut outputs: BTreeMap<NodeId, crate::NodeOutput> = BTreeMap::new();
        let mut execution_order: Vec<NodeId> = Vec::new();
        let cond_edges_by_from = self.cond_edges_by_from();

        // R33-5: cond source 集合 — 这些节点的出口由 cond 决定, 不推动 DAG 后继.
        let cond_sources: BTreeSet<NodeId> = self
            .graph
            .conditional_edges
            .iter()
            .map(|e| e.from.clone())
            .collect();

        // R33-5: 来自其它节点的 cond 指向 — 用于识别 natural entry. 自环忽略.
        let mut incoming_cond: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for edge in &self.graph.conditional_edges {
            for target in edge.path_map.values().chain(edge.default.as_ref()) {
                if *target != edge.from {
                    incoming_cond
                        .entry(target.clone())
                        .or_default()
                        .insert(edge.from.clone());
                }
            }
        }

        // 经典 Kahn: 算 DAG indegree / successors
        let mut indegree: BTreeMap<NodeId, usize> = self
            .graph
            .nodes
            .keys()
            .cloned()
            .map(|n| (n, 0usize))
            .collect();
        let mut successors: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for edge in &self.graph.edges {
            if !self.graph.nodes.contains_key(&edge.from) {
                return Err(GraphError::MissingNode(edge.from.clone()));
            }
            if !self.graph.nodes.contains_key(&edge.to) {
                return Err(GraphError::MissingNode(edge.to.clone()));
            }
            successors
                .entry(edge.from.clone())
                .or_default()
                .insert(edge.to.clone());
            *indegree
                .get_mut(&edge.to)
                .expect("edge destination validated above") += 1;
        }

        // 自然入口
        let natural_entries: BTreeSet<NodeId> = self
            .graph
            .nodes
            .keys()
            .filter(|n| {
                *indegree.get(*n).unwrap_or(&1) == 0
                    && incoming_cond.get(*n).map_or(true, |s| s.is_empty())
            })
            .cloned()
            .collect();

        // 初始 ready
        let mut ready: BTreeSet<NodeId> = if !natural_entries.is_empty() {
            let cond_source_entries: BTreeSet<NodeId> = natural_entries
                .intersection(&cond_sources)
                .cloned()
                .collect();
            if !cond_source_entries.is_empty() {
                cond_source_entries
            } else {
                natural_entries
            }
        } else if let Some(first) = cond_sources.iter().next() {
            BTreeSet::from([first.clone()])
        } else if let Some(first) = self.graph.nodes.keys().next() {
            BTreeSet::from([first.clone()])
        } else {
            BTreeSet::new()
        };

        let mut chain_steps: usize = 0;
        while let Some(node_id) = ready.pop_first() {
            self.run_node_with_chain(
                &node_id,
                &mut state,
                &mut outputs,
                &mut execution_order,
                &mut ready,
                &mut chain_steps,
                MAX_CHAIN_STEPS,
                &cond_edges_by_from,
                &cond_sources,
                &successors,
                &mut indegree,
                false,
            )?;
        }

        Ok(FinalState {
            state,
            outputs,
            execution_order,
        })
    }

    /// R33-5: 跑单节点 + 推 DAG + 触发 cond edges.
    /// 自环走 inline 递归 (受 `MAX_CHAIN_STEPS` 截断), 跨节点入 ready.
    #[allow(clippy::too_many_arguments)]
    fn run_node_with_chain(
        &self,
        node_id: &NodeId,
        state: &mut crate::State,
        outputs: &mut BTreeMap<NodeId, crate::NodeOutput>,
        execution_order: &mut Vec<NodeId>,
        ready: &mut BTreeSet<NodeId>,
        chain_steps: &mut usize,
        max_chain_steps: usize,
        cond_edges_by_from: &BTreeMap<NodeId, Vec<&crate::conditional::ConditionalEdge>>,
        cond_sources: &BTreeSet<NodeId>,
        successors: &BTreeMap<NodeId, BTreeSet<NodeId>>,
        indegree: &mut BTreeMap<NodeId, usize>,
        is_self_loop_reentry: bool,
    ) -> Result<()> {
        if !is_self_loop_reentry && outputs.contains_key(node_id) {
            return Ok(());
        }
        if *chain_steps >= max_chain_steps {
            return Err(GraphError::Cycle {
                nodes: vec![node_id.clone()],
            });
        }
        *chain_steps += 1;

        let node = self
            .graph
            .nodes
            .get(node_id)
            .ok_or_else(|| GraphError::MissingNode(node_id.clone()))?;
        let output = node.run(state).map_err(|error| GraphError::NodeExecution {
            node_id: node_id.clone(),
            message: error.to_string(),
        })?;
        outputs.insert(node_id.clone(), output);
        execution_order.push(node_id.clone());

        // 推动 DAG 后继 (cond source 不推动, 出口由 cond 决定)
        if !cond_sources.contains(node_id) {
            if let Some(children) = successors.get(node_id) {
                for child in children {
                    if outputs.contains_key(child) || ready.contains(child) {
                        continue;
                    }
                    let d = indegree
                        .get_mut(child)
                        .expect("edge destination validated above");
                    if *d > 0 {
                        *d -= 1;
                    }
                    if *d == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }

        // 触发 conditional edges
        if let Some(edges) = cond_edges_by_from.get(node_id) {
            for cond_edge in edges {
                let decision = cond_edge.decide(state);
                let Some(target) = decision.target else {
                    continue;
                };
                if !self.graph.nodes.contains_key(&target) {
                    return Err(GraphError::MissingNode(target.clone()));
                }
                // 跨节点 re-entry → cycle
                if target != *node_id && outputs.contains_key(&target) {
                    return Err(GraphError::Cycle {
                        nodes: vec![node_id.clone(), target],
                    });
                }
                if target == *node_id {
                    // self-loop: inline 递归, 走 cond source 自身的 cond.
                    // 跳过 visited check, 允许重跑, 受 chain_steps 截断.
                    self.run_node_with_chain(
                        &target,
                        state,
                        outputs,
                        execution_order,
                        ready,
                        chain_steps,
                        max_chain_steps,
                        cond_edges_by_from,
                        cond_sources,
                        successors,
                        indegree,
                        true,
                    )?;
                } else if !outputs.contains_key(&target) {
                    ready.insert(target);
                }
            }
        }

        Ok(())
    }

    /// R33-5 helper: 把 conditional edges 按 `from` 索引.
    fn cond_edges_by_from(&self) -> BTreeMap<NodeId, Vec<&crate::conditional::ConditionalEdge>> {
        let mut map: BTreeMap<NodeId, Vec<&crate::conditional::ConditionalEdge>> = BTreeMap::new();
        for edge in &self.graph.conditional_edges {
            map.entry(edge.from.clone()).or_default().push(edge);
        }
        map
    }

    fn topological_order(&self) -> Result<Vec<NodeId>> {
        let mut indegree = self
            .graph
            .nodes
            .keys()
            .cloned()
            .map(|node_id| (node_id, 0usize))
            .collect::<BTreeMap<_, _>>();
        let mut successors = BTreeMap::<NodeId, BTreeSet<NodeId>>::new();

        for edge in &self.graph.edges {
            if !self.graph.nodes.contains_key(&edge.from) {
                return Err(GraphError::MissingNode(edge.from.clone()));
            }
            if !self.graph.nodes.contains_key(&edge.to) {
                return Err(GraphError::MissingNode(edge.to.clone()));
            }

            let inserted = successors
                .entry(edge.from.clone())
                .or_default()
                .insert(edge.to.clone());
            if inserted {
                *indegree
                    .get_mut(&edge.to)
                    .expect("edge destination validated above") += 1;
            }
        }

        let mut ready = indegree
            .iter()
            .filter_map(|(node_id, degree)| (*degree == 0).then_some(node_id.clone()))
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(self.graph.nodes.len());

        while let Some(node_id) = ready.pop_first() {
            order.push(node_id.clone());
            if let Some(children) = successors.get(&node_id) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("edge destination validated above");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }

        if order.len() != self.graph.nodes.len() {
            let blocked = indegree
                .into_iter()
                .filter_map(|(node_id, degree)| (degree > 0).then_some(node_id))
                .collect();
            return Err(GraphError::Cycle { nodes: blocked });
        }

        Ok(order)
    }
}

#[cfg(feature = "supervisor-integration")]
fn supervisor_snapshot() -> SupervisorSnapshot {
    // ponytail: read the real plan but do not spawn child processes. Upgrade to
    // supervised tasks when Node::run becomes async.
    let supervisor = PidOneSupervisor::new();
    SupervisorSnapshot {
        plan_version: supervisor.plan_version,
        child_count: supervisor.total_children(),
        mocked: true,
    }
}

#[cfg(not(feature = "supervisor-integration"))]
fn supervisor_snapshot() -> SupervisorSnapshot {
    // ponytail: a zero-process mock keeps the P0 graph isolated from the current
    // supervisor→verify dependency chain. Enable `supervisor-integration` to
    // read the real supervisor plan without starting it.
    SupervisorSnapshot {
        plan_version: 0,
        child_count: 0,
        mocked: true,
    }
}

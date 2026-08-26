//! # Task DAG 拓扑
//!
//! 1:1 翻译 v0.9.21 `taskTools.js` 任务依赖图 + 估补 `apeireth-workflow` 7 NodeType DAG 集成.
//!
//! **核心 invariant**:
//! - DAG 无环 (`detect_cycle`)
//! - 拓扑序存在 (`topological_sort`)
//! - DAG 深度 ≤ `MAX_DAG_DEPTH` (32, 防栈溢出)
//!
//! **设计选择** (per RIVAL §2.5.1 + 主人 2026-08-05 拍板):
//! - skeleton 阶段用 `HashMap<TaskId, Vec<TaskId>>` 存邻接表
//! - 阶段 3 续接 `apeireth-graph::Dag` (dagre 布局 + 已有 topological_sort)
//! - 估补 `apeireth-workflow` 的 7 NodeType (Input / LLM / Tool / Branch / Loop / Parallel / Output) 通过 node_type 字段占位

use crate::{TaskError, TaskId, MAX_DAG_DEPTH};
use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// §1 编译期守门
// ============================================================================

/// DAG 节点类型 (per apeireth-workflow 7 NodeType 估补).
///
/// skeleton 阶段只占位 4 关键类型; 阶段 3 跟 apeireth-workflow 7 NodeType 1:1 对齐.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// 输入节点 (per workflow NodeType::Input)
    Input,
    /// LLM 调用节点
    Llm,
    /// 工具调用节点 (eg. Task submit / cancel / retry)
    Tool,
    /// 输出节点
    Output,
}

impl NodeKind {
    /// 4 节点类型 (skeleton 阶段估补, 阶段 3 续到 7 跟 apeireth-workflow 对齐)
    pub const SKELETON_KINDS: usize = 4;
}

// ============================================================================
// §2 TaskNode — DAG 单节点
// ============================================================================

/// DAG 单节点 (1 个 Task).
#[derive(Debug, Clone)]
pub struct TaskNode {
    /// Task ID
    pub id: TaskId,
    /// 节点类型 (per apeireth-workflow 7 NodeType 估补)
    pub kind: NodeKind,
    /// 显示名 (per v0.9.21 taskTools `taskName`)
    pub name: String,
    /// DAG 深度 (从根算起, 0 = 根)
    pub depth: usize,
}

// ============================================================================
// §3 TaskDag — 邻接表 + 拓扑排序
// ============================================================================

/// Task DAG 邻接表 (skeleton 阶段用 HashMap, 阶段 3 续 apeireth-graph::Dag).
#[derive(Debug, Clone, Default)]
pub struct TaskDag {
    /// 节点表: id → TaskNode
    pub nodes: HashMap<TaskId, TaskNode>,
    /// 出边表: id → 依赖此节点的下游 id 列表
    pub edges: HashMap<TaskId, Vec<TaskId>>,
    /// 反向表: id → 此节点依赖的上游 id 列表 (入边)
    pub deps: HashMap<TaskId, Vec<TaskId>>,
}

impl TaskDag {
    /// 新建空 DAG.
    pub fn new() -> Self {
        Self::default()
    }

    /// 加节点.
    pub fn add_node(&mut self, id: TaskId, kind: NodeKind, name: impl Into<String>) {
        let depth = 0; // 实际深度在 add_edge 后 recalculate
        self.nodes.insert(
            id.clone(),
            TaskNode {
                id: id.clone(),
                kind,
                name: name.into(),
                depth,
            },
        );
        self.edges.entry(id.clone()).or_default();
        self.deps.entry(id).or_default();
    }

    /// 加边: `from` 是 `to` 的依赖 (即 `to` 依赖 `from` 完成).
    pub fn add_edge(&mut self, from: TaskId, to: TaskId) -> Result<(), TaskError> {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return Err(TaskError::DagNodeNotFound { missing: format!("{from} -> {to}") });
        }
        self.edges.entry(from.clone()).or_default().push(to.clone());
        self.deps.entry(to).or_default().push(from);
        Ok(())
    }

    /// 校验 DAG 深度 ≤ MAX_DAG_DEPTH.
    pub fn validate_depth(&self) -> Result<(), TaskError> {
        let max_depth = self.compute_max_depth();
        if max_depth > MAX_DAG_DEPTH {
            return Err(TaskError::DagTooDeep {
                depth: max_depth,
                max: MAX_DAG_DEPTH,
            });
        }
        Ok(())
    }

    /// 算最大深度 (BFS from roots).
    fn compute_max_depth(&self) -> usize {
        let mut max_depth = 0;
        for (id, node) in &self.nodes {
            if self.deps.get(id).map_or(true, |d| d.is_empty()) {
                // 根节点, BFS 算下游深度
                let mut queue = VecDeque::new();
                queue.push_back((id.clone(), 0_usize));
                let mut visited = HashSet::new();
                while let Some((cur, depth)) = queue.pop_front() {
                    if !visited.insert(cur.clone()) {
                        continue;
                    }
                    max_depth = max_depth.max(depth);
                    if let Some(children) = self.edges.get(&cur) {
                        for c in children {
                            queue.push_back((c.clone(), depth + 1));
                        }
                    }
                }
            }
            let _ = node; // 沉默 unused
        }
        max_depth
    }

    /// 检测环 (BFS + 入度表, Kahn's algorithm).
    pub fn detect_cycle(&self) -> bool {
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(id.clone(), self.deps.get(id).map_or(0, |d| d.len()));
        }
        let mut queue: VecDeque<TaskId> = in_degree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut visited = 0_usize;
        while let Some(cur) = queue.pop_front() {
            visited += 1;
            if let Some(children) = self.edges.get(&cur) {
                for c in children {
                    if let Some(d) = in_degree.get_mut(c) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            queue.push_back(c.clone());
                        }
                    }
                }
            }
        }
        visited != self.nodes.len()
    }

    /// 拓扑排序 (Kahn's algorithm). 返回拓扑序 TaskId 列表.
    pub fn topological_sort(&self) -> Result<Vec<TaskId>, TaskError> {
        if self.detect_cycle() {
            return Err(TaskError::DagCycle);
        }
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(id.clone(), self.deps.get(id).map_or(0, |d| d.len()));
        }
        let mut queue: VecDeque<TaskId> = in_degree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut result = Vec::with_capacity(self.nodes.len());
        while let Some(cur) = queue.pop_front() {
            result.push(cur.clone());
            if let Some(children) = self.edges.get(&cur) {
                for c in children {
                    if let Some(d) = in_degree.get_mut(c) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            queue.push_back(c.clone());
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    /// 节点数.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_topological_sort_simple() {
        let mut dag = TaskDag::new();
        let a = TaskId::new();
        let b = TaskId::new();
        let c = TaskId::new();
        dag.add_node(a.clone(), NodeKind::Input, "a");
        dag.add_node(b.clone(), NodeKind::Tool, "b");
        dag.add_node(c.clone(), NodeKind::Output, "c");
        dag.add_edge(a.clone(), b.clone()).unwrap();
        dag.add_edge(b.clone(), c.clone()).unwrap();
        let topo = dag.topological_sort().unwrap();
        assert_eq!(topo.len(), 3);
        // a 必在 b 前, b 必在 c 前
        let pos_a = topo.iter().position(|x| x == &a).unwrap();
        let pos_b = topo.iter().position(|x| x == &b).unwrap();
        let pos_c = topo.iter().position(|x| x == &c).unwrap();
        assert!(pos_a < pos_b && pos_b < pos_c);
    }

    #[test]
    fn test_dag_detect_cycle() {
        let mut dag = TaskDag::new();
        let a = TaskId::new();
        let b = TaskId::new();
        dag.add_node(a.clone(), NodeKind::Input, "a");
        dag.add_node(b.clone(), NodeKind::Output, "b");
        dag.add_edge(a.clone(), b.clone()).unwrap();
        dag.add_edge(b.clone(), a.clone()).unwrap();
        assert!(dag.detect_cycle());
        assert!(dag.topological_sort().is_err());
    }
}

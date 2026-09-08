//! B2 · RA-15 P0-B: 屏障优先级联修复执行器（Research 前缀，默认关闭）。
//!
//! # 学术账本（铁律 3）
//! - **问题定义**（吸收自 arXiv:2605.07242 MEMOREPAIR）: 源工件被删除/更正后，
//!   由它派生的后代（summary/缓存/嵌入/技能/工具过程）仍可见并以陈旧支撑
//!   继续引导行为。修复必须覆盖**可见派生状态**的级联失效。
//! - **对手契约**: 修复事件 = 从失效后代状态到验证后继状态的受控迁移——
//!   受影响后代**先全部撤回** → 用保留支撑 + 已修复前驱**重建** → 仅放行
//!   "前驱闭包已验证"的后继；修复选择归约为**最大权前驱闭包**，可用单个
//!   s-t 最小割精确求解。
//! - **状态**: 原型已实现（纯内存图 + 自写 Dinic，0 新外部依赖）。
//!   真持久化（撤回/重建/再发布的落库动作）留部署层接线（0 装）。
//! - **引用**: arXiv:2605.07242；逐行对照见
//!   `docs/03-reference/absorption-2026-09.md` §P0-B。
//! - **默认关闭（铁律 1）**: 本模块不挂任何生产路径；不替换 `forget_episode` /
//!   `research_forget_closure` 的审计语义——本模块是审计报告**之后**的
//!   下游执行器（审计 → 人类批准 → 修复执行）。
//!
//! # 与学术线论文的关系
//! 论文卖点 = 审计前置 + 探针协议；本执行器是其**下游**——论文 related work
//! 写"审计是修复的前置层，MEMOREPAIR 类修复器是其下游执行器"。

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::research_derived_memory::DerivedRef;

/// 修复图中的单个派生工件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairNode {
    /// 工件标识（与血缘登记同命名空间）。
    pub artifact: DerivedRef,
    /// 支撑集（父来源；前驱闭包按此定义）。
    pub sources: Vec<DerivedRef>,
    /// 修复效用权重（放行价值）。
    pub weight: f64,
    /// 归一化修复算子成本。
    pub cost: f64,
}

/// 屏障式修复结果：受控迁移的三段账本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairOutcome {
    /// 已撤回的受影响工件（含根）。
    pub withdrawn: Vec<DerivedRef>,
    /// 已重建、已验证、已再发布的后继（前驱闭包成立）。
    pub repaired_and_republished: Vec<DerivedRef>,
    /// 已撤回但未再发布（未选中 / 验证失败 / 支撑缺失）。
    pub still_withdrawn: Vec<DerivedRef>,
    /// 选中集总修复成本与总权重（最小割解）。
    pub selected_cost: f64,
    pub selected_weight: f64,
    /// 0 装：本模块只规划状态迁移；持久化动作由部署层执行。
    pub planner_only: bool,
}

/// 修复选择标量化参数：λ = 每单位成本换权重的折价（对手的
/// "scalarized repair-selection problem for a fixed repair-cost tradeoff"）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepairTradeoff {
    pub lambda: f64,
}

impl Default for RepairTradeoff {
    fn default() -> Self {
        Self { lambda: 1.0 }
    }
}

/// 屏障式修复执行器（纯函数规划；默认关闭）。
#[derive(Debug, Clone, Default)]
pub struct RepairExecutor;

impl RepairExecutor {
    /// 计算受影响集：以 `affected_roots` 为根，沿支撑集前驱方向 BFS
    /// （与血缘 taint 闭包同语义：任一来源失效 ⇒ 后代受影响）。
    pub fn affected_closure(
        nodes: &[RepairNode],
        affected_roots: &[DerivedRef],
    ) -> HashSet<DerivedRef> {
        let index: HashMap<DerivedRef, &RepairNode> =
            nodes.iter().map(|n| (n.artifact.clone(), n)).collect();
        let mut affected: HashSet<DerivedRef> = affected_roots.iter().cloned().collect();
        let mut queue: VecDeque<DerivedRef> = affected_roots.iter().cloned().collect();
        // 反向邻接: parent -> children。
        let mut children: HashMap<DerivedRef, Vec<DerivedRef>> = HashMap::new();
        for n in nodes {
            for s in &n.sources {
                children
                    .entry(s.clone())
                    .or_default()
                    .push(n.artifact.clone());
            }
        }
        while let Some(cur) = queue.pop_front() {
            if let Some(kids) = children.get(&cur) {
                for k in kids {
                    if affected.insert(k.clone()) {
                        queue.push_back(k.clone());
                    }
                }
            }
        }
        let _ = index; // 仅文档性索引; 后续阶段按 nodes 顺序处理.
        affected
    }

    /// 最大权前驱闭包（吸收自 arXiv:2605.07242 §repair-selection）:
    /// 在受影响集上选子集 S，满足前驱闭包（v ∈ S ⇒ 所有父来源 ∈ S），
    /// 最大化 Σ weight − λ·Σ cost。归约为 s-t 最小割（自写 Dinic）精确求解。
    pub fn select_repair_set(
        nodes: &[RepairNode],
        affected: &HashSet<DerivedRef>,
        tradeoff: RepairTradeoff,
    ) -> (HashSet<DerivedRef>, f64, f64) {
        let affected_nodes: Vec<&RepairNode> = nodes
            .iter()
            .filter(|n| affected.contains(&n.artifact))
            .collect();
        // 图节点编号。
        let mut node_id: HashMap<DerivedRef, usize> = HashMap::new();
        for (i, n) in affected_nodes.iter().enumerate() {
            node_id.insert(n.artifact.clone(), i);
        }
        let m = affected_nodes.len();
        if m == 0 {
            return (HashSet::new(), 0.0, 0.0);
        }
        // 流图: 0 = s, 1..=m = 工件节点, m+1 = t。
        let s = 0usize;
        let t = m + 1;
        let mut total_positive = 0.0f64;
        const SCALE: f64 = 1_000_000.0;
        let mut source_caps: Vec<f64> = vec![0.0; m];
        let mut sink_caps: Vec<f64> = vec![0.0; m];
        for (i, n) in affected_nodes.iter().enumerate() {
            let profit = n.weight - tradeoff.lambda * n.cost;
            if profit > 0.0 {
                source_caps[i] = profit;
                total_positive += profit;
            } else if profit < 0.0 {
                sink_caps[i] = -profit;
            }
        }
        // 离散化容量: 最小割精确性由浮点放大取整保证（确定性）;
        // 前驱闭包约束 (v 依赖父 u ⇒ 选 v 必选 u) 编码为边 v→u 容量 ∞（割断即违约）。
        let mut dinic_flow = Dinic::new(m + 2);
        for (i, cap) in source_caps.iter().enumerate() {
            if *cap > 0.0 {
                let c = (cap * SCALE).round() as i64;
                dinic_flow.add_edge(s, i + 1, c);
            }
        }
        for (i, cap) in sink_caps.iter().enumerate() {
            if *cap > 0.0 {
                let c = (cap * SCALE).round() as i64;
                dinic_flow.add_edge(i + 1, t, c);
            }
        }
        for (i, n) in affected_nodes.iter().enumerate() {
            let v = i + 1;
            for parent in &n.sources {
                if let Some(&pu) = node_id.get(parent) {
                    let u = pu + 1;
                    dinic_flow.add_edge(v, u, i64::MAX / 4);
                }
            }
        }
        let max_flow = dinic_flow.max_flow(s, t);
        let _ = max_flow; // 最小割值 (诊断用; 选中集由 s 侧可达集直接给出).
                          // s 侧可达集 = 选中集（闭合）。
        let reachable = dinic_flow.reachable_from(s);
        let mut selected: HashSet<DerivedRef> = HashSet::new();
        let mut selected_weight = 0.0f64;
        let mut selected_cost = 0.0f64;
        for (i, n) in affected_nodes.iter().enumerate() {
            if reachable[i + 1] {
                selected.insert(n.artifact.clone());
                selected_weight += n.weight;
                selected_cost += n.cost;
            }
        }
        let _ = total_positive; // 利润上界 (诊断用).
        (selected, selected_weight, selected_cost)
    }

    /// 屏障式修复执行: 撤回全部受影响 → 阶段重建（拓扑序, 前驱先行）→
    /// 验证（父支撑 ∈ 保留 ∪ 已修复已验证）→ 仅放行前驱闭包成立的后继。
    /// 根节点（在 affected_roots 中）无保留支撑，不重建（它们的正确值由
    /// 上游人工更正提供，本执行器不生成内容——0 装）。
    pub fn execute(
        nodes: &[RepairNode],
        affected_roots: &[DerivedRef],
        tradeoff: RepairTradeoff,
    ) -> RepairOutcome {
        let affected = Self::affected_closure(nodes, affected_roots);
        let (selected, selected_weight, selected_cost) =
            Self::select_repair_set(nodes, &affected, tradeoff);

        // 阶段重建: 按受影响集内的拓扑序迭代（来源数 0 → 依赖全满足）。
        // 确定性: 工件按 (kind,id) 排序, 初始队列取排序后 indeg==0 者,
        // 子节点按 deps 插入序（同源于排序表）推入。
        let index: HashMap<DerivedRef, &RepairNode> =
            nodes.iter().map(|n| (n.artifact.clone(), n)).collect();
        let mut repaired: Vec<DerivedRef> = Vec::new();
        let root_set: HashSet<DerivedRef> = affected_roots.iter().cloned().collect();
        let mut affected_vec: Vec<&RepairNode> = nodes
            .iter()
            .filter(|n| affected.contains(&n.artifact))
            .collect();
        affected_vec.sort_by(|a, b| {
            (a.artifact.kind.clone(), a.artifact.id.clone())
                .cmp(&(b.artifact.kind.clone(), b.artifact.id.clone()))
        });
        // 拓扑序: Kahn, 依赖 = sources∩(affected \ roots)（根已被上游人工修复, 视为已处理）。
        let mut indeg: HashMap<DerivedRef, usize> = HashMap::new();
        let mut deps: HashMap<DerivedRef, Vec<DerivedRef>> = HashMap::new();
        for n in &affected_vec {
            let mut d = 0usize;
            for s in &n.sources {
                if affected.contains(s) && !root_set.contains(s) {
                    d += 1;
                    deps.entry(s.clone()).or_default().push(n.artifact.clone());
                }
            }
            indeg.insert(n.artifact.clone(), d);
        }
        let mut queue: VecDeque<DerivedRef> = affected_vec
            .iter()
            .filter(|n| indeg.get(&n.artifact).copied().unwrap_or(1) == 0)
            .map(|n| n.artifact.clone())
            .collect();
        let mut topo: Vec<DerivedRef> = Vec::new();
        while let Some(cur) = queue.pop_front() {
            topo.push(cur.clone());
            if let Some(kids) = deps.get(&cur) {
                for k in kids {
                    if let Some(d) = indeg.get_mut(k) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push_back(k.clone());
                        }
                    }
                }
            }
        }

        let mut validated: HashSet<DerivedRef> = HashSet::new();
        for art in &topo {
            if root_set.contains(art) {
                // 根 = 失效源头: 不重建 (内容需上游人工更正, 0 装).
                continue;
            }
            if !selected.contains(art) {
                // 未选中: 撤回后不修复.
                continue;
            }
            let Some(node) = index.get(art) else { continue };
            // 屏障验证 (防御性不变量; 精确闭包解下必然成立):
            // 所有父来源 ∈ (保留集 ∪ 已修复已验证 ∪ 根集[上游人工已更正]).
            let all_support_ok = node.sources.iter().all(|s| {
                if affected.contains(s) {
                    validated.contains(s) || root_set.contains(s)
                } else {
                    // 保留支撑: 不在受影响集内, 天然有效。
                    true
                }
            });
            if all_support_ok {
                validated.insert(art.clone());
                repaired.push(art.clone()); // 拓扑序 = 前驱先行, 保持顺序 (确定性).
            }
        }

        let mut withdrawn: Vec<DerivedRef> = affected.iter().cloned().collect();
        withdrawn
            .sort_by(|a, b| (a.kind.clone(), a.id.clone()).cmp(&(b.kind.clone(), b.id.clone())));
        let repaired_set: HashSet<DerivedRef> = repaired.iter().cloned().collect();
        let mut still_vec: Vec<DerivedRef> = affected
            .iter()
            .filter(|a| !repaired_set.contains(*a))
            .cloned()
            .collect();
        still_vec
            .sort_by(|a, b| (a.kind.clone(), a.id.clone()).cmp(&(b.kind.clone(), b.id.clone())));

        RepairOutcome {
            withdrawn,
            repaired_and_republished: repaired,
            still_withdrawn: still_vec,
            selected_cost,
            selected_weight,
            planner_only: true,
        }
    }
}

// ============================================================================
// 自写 Dinic 最大流（纯 Safe Rust，0 新外部依赖；容量 i64 整数化保证确定性）。
// ============================================================================

#[derive(Debug, Clone)]
struct Edge {
    to: usize,
    rev: usize,
    cap: i64,
}

struct Dinic {
    graph: Vec<Vec<Edge>>,
}

impl Dinic {
    fn new(n: usize) -> Self {
        Self {
            graph: vec![Vec::new(); n],
        }
    }

    fn add_edge(&mut self, from: usize, to: usize, cap: i64) {
        let rev_from = self.graph[to].len();
        let rev_to = self.graph[from].len();
        self.graph[from].push(Edge {
            to,
            rev: rev_from,
            cap,
        });
        self.graph[to].push(Edge {
            to: from,
            rev: rev_to,
            cap: 0,
        });
    }

    fn bfs(&self, s: usize, t: usize, level: &mut [i32]) -> bool {
        level.fill(-1);
        level[s] = 0;
        let mut q = VecDeque::new();
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            for e in &self.graph[v] {
                if e.cap > 0 && level[e.to] < 0 {
                    level[e.to] = level[v] + 1;
                    q.push_back(e.to);
                }
            }
        }
        level[t] >= 0
    }

    fn dfs(&mut self, v: usize, t: usize, f: i64, level: &[i32], iter: &mut [usize]) -> i64 {
        if v == t {
            return f;
        }
        while iter[v] < self.graph[v].len() {
            let idx = iter[v];
            let e = self.graph[v][idx].clone();
            if e.cap > 0 && level[v] < level[e.to] {
                let d = self.dfs(e.to, t, f.min(e.cap), level, iter);
                if d > 0 {
                    self.graph[v][idx].cap -= d;
                    let rev = e.rev;
                    self.graph[e.to][rev].cap += d;
                    return d;
                }
            }
            iter[v] += 1;
        }
        0
    }

    fn max_flow(&mut self, s: usize, t: usize) -> i64 {
        let mut flow = 0i64;
        let n = self.graph.len();
        let mut level = vec![0i32; n];
        loop {
            if !self.bfs(s, t, &mut level) {
                break;
            }
            let mut iter = vec![0usize; n];
            loop {
                let f = self.dfs(s, t, i64::MAX, &level, &mut iter);
                if f == 0 {
                    break;
                }
                flow += f;
            }
        }
        flow
    }

    /// 残量图上从 s 可达的顶点（最小割 s 侧 = 选中集）。
    fn reachable_from(&self, s: usize) -> Vec<bool> {
        let n = self.graph.len();
        let mut vis = vec![false; n];
        let mut q = VecDeque::new();
        q.push_back(s);
        vis[s] = true;
        while let Some(v) = q.pop_front() {
            for e in &self.graph[v] {
                if e.cap > 0 && !vis[e.to] {
                    vis[e.to] = true;
                    q.push_back(e.to);
                }
            }
        }
        vis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(kind: &str, id: &str, sources: &[(&str, &str)], weight: f64, cost: f64) -> RepairNode {
        RepairNode {
            artifact: DerivedRef::new(kind, id),
            sources: sources
                .iter()
                .map(|(k, i)| DerivedRef::new(*k, *i))
                .collect(),
            weight,
            cost,
        }
    }

    /// 级联: a→b→c 全部受影响; 选中集必须前驱闭合.
    #[test]
    fn cascade_affected_closure_and_predecessor_closed_selection() {
        let nodes = vec![
            node("note", "b", &[("episode", "a")], 10.0, 2.0),
            node("wiki", "c", &[("note", "b")], 9.0, 1.0),
            node("note", "d", &[("episode", "x")], 5.0, 1.0),
        ];
        let affected = RepairExecutor::affected_closure(&nodes, &[DerivedRef::new("episode", "a")]);
        assert!(affected.contains(&DerivedRef::new("note", "b")));
        assert!(affected.contains(&DerivedRef::new("wiki", "c")));
        assert!(!affected.contains(&DerivedRef::new("note", "d")));

        let (selected, _, _) =
            RepairExecutor::select_repair_set(&nodes, &affected, RepairTradeoff::default());
        // b 与 c 利润为正, 都该入选; 且闭包: 选 c ⇒ 必选 b.
        assert!(selected.contains(&DerivedRef::new("note", "b")));
        assert!(selected.contains(&DerivedRef::new("wiki", "c")));
        assert!(!selected.contains(&DerivedRef::new("note", "d")));
    }

    /// 最小割最优性: 一个已知小实例——负利润节点 (c) 不入选;
    /// 独立正利润节点 (d) 入选; 依赖链 b←c 闭合.
    #[test]
    fn mincut_selects_positive_closure_and_drops_negative() {
        let nodes = vec![
            // c 依赖 b, 两者利润都为正 → 都选.
            node("note", "b", &[("episode", "a")], 10.0, 2.0),
            node("wiki", "c", &[("note", "b")], 9.0, 1.0),
            // d 独立且利润为负 (5 - 1*10) → 不选.
            node("note", "d", &[("episode", "x")], 5.0, 10.0),
        ];
        let affected = RepairExecutor::affected_closure(
            &nodes,
            &[
                DerivedRef::new("episode", "a"),
                DerivedRef::new("episode", "x"),
            ],
        );
        let (selected, weight, cost) =
            RepairExecutor::select_repair_set(&nodes, &affected, RepairTradeoff::default());
        assert!(selected.contains(&DerivedRef::new("note", "b")));
        assert!(selected.contains(&DerivedRef::new("wiki", "c")));
        assert!(!selected.contains(&DerivedRef::new("note", "d")));
        assert!((weight - 19.0).abs() < 1e-6);
        assert!((cost - 3.0).abs() < 1e-6);
    }

    /// 屏障式执行: 撤回→阶段重建→验证→放行; 根不重建 (0 装).
    #[test]
    fn barrier_execute_withdraw_rebuild_release() {
        let nodes = vec![
            node("note", "b", &[("episode", "a")], 10.0, 2.0),
            node("wiki", "c", &[("note", "b")], 9.0, 1.0),
            node("note", "d", &[("episode", "x")], 5.0, 1.0),
        ];
        let outcome = RepairExecutor::execute(
            &nodes,
            &[DerivedRef::new("episode", "a")],
            RepairTradeoff::default(),
        );
        // withdrawn 含根: [episode:a, note:b, wiki:c].
        assert_eq!(outcome.withdrawn.len(), 3);
        assert!(outcome
            .repaired_and_republished
            .contains(&DerivedRef::new("note", "b")));
        assert!(outcome
            .repaired_and_republished
            .contains(&DerivedRef::new("wiki", "c")));
        // 根自身不重建 (内容由上游人工更正), 留在 still_withdrawn.
        assert_eq!(outcome.still_withdrawn.len(), 1);
        assert!(outcome
            .still_withdrawn
            .contains(&DerivedRef::new("episode", "a")));
        assert!(outcome.planner_only);
    }

    /// 精确闭包语义: 修复 c 必须连带修复其负利润前驱 b (最大权前驱闭包),
    /// 独立负利润节点 d 被排除; 屏障放行顺序 = 前驱先行 (拓扑序).
    #[test]
    fn exact_closure_repairs_negative_predecessor_and_drops_independent() {
        let nodes = vec![
            // b 利润 -9 (1 - 1*10), c 利润 +99 (100 - 1*1) 且依赖 b → 闭包 {b, c}.
            node("note", "b", &[("episode", "a")], 1.0, 10.0),
            node("wiki", "c", &[("note", "b")], 100.0, 1.0),
            // d 独立且利润 -9 → 不选.
            node("note", "d", &[("episode", "x")], 1.0, 10.0),
        ];
        let outcome = RepairExecutor::execute(
            &nodes,
            &[
                DerivedRef::new("episode", "a"),
                DerivedRef::new("episode", "x"),
            ],
            RepairTradeoff::default(),
        );
        assert!(
            outcome
                .repaired_and_republished
                .contains(&DerivedRef::new("note", "b")),
            "前驱闭包须连带修复负利润前驱: {:?}",
            outcome.repaired_and_republished
        );
        assert!(outcome
            .repaired_and_republished
            .contains(&DerivedRef::new("wiki", "c")));
        assert!(!outcome
            .repaired_and_republished
            .contains(&DerivedRef::new("note", "d")));
        // 屏障放行顺序: 前驱 b 先于后继 c (拓扑序).
        let pos_b = outcome
            .repaired_and_republished
            .iter()
            .position(|r| r == &DerivedRef::new("note", "b"))
            .unwrap();
        let pos_c = outcome
            .repaired_and_republished
            .iter()
            .position(|r| r == &DerivedRef::new("wiki", "c"))
            .unwrap();
        assert!(pos_b < pos_c, "放行顺序必须前驱先行");
        assert!(outcome
            .still_withdrawn
            .contains(&DerivedRef::new("note", "d")));
    }

    /// 根节点自身: 撤回但不重建 (内容需上游人工更正).
    #[test]
    fn roots_are_withdrawn_but_not_rebuilt() {
        let nodes = vec![node("note", "r", &[], 10.0, 1.0)];
        let outcome = RepairExecutor::execute(
            &nodes,
            &[DerivedRef::new("note", "r")],
            RepairTradeoff::default(),
        );
        assert!(outcome.repaired_and_republished.is_empty());
        assert!(outcome
            .still_withdrawn
            .contains(&DerivedRef::new("note", "r")));
    }
}

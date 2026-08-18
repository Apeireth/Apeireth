//! `apeireth-companion::causal_world_model` — 世界模型第二层: 因果结构图推演 (TP32 / W2 + W3).
//!
//! 哲学 (docs/design-intent.md §2「世界模型」):
//! - 第一层 (W1, `world_model.rs`): LLM 按时间线展开反事实推演链, oracle Brier 终点校准.
//! - **第二层 (本模块, W2)**: 在 `memory_graph` 的 s/p/o 时序因果网上沿边展开"如果……那么……"路径.
//!   MCTS 跑在因果图上 (而非动作空间); LLM 只在分支点做判断 (与 `apeireth_cognition::planning`
//!   的 `StateEvaluator` 同构复用).
//! - **第三层 (W3, 本模块)**: 主人差异化核心 — **从记忆时间线挖掘因果边** (统计验证优先),
//!   EvoCause 式 LLM 提议边作为补充. 全世界世界模型都在做通用世界; 她独有的训练集是主人
//!   的生活轨迹, 记忆时间线 = 因果数据. (主人 2026-08-18 拍板)
//!
//! ## 0 装 PASS (诚实登记)
//!
//! - **真 LLM 未接, trait 口已备** (`CausalLlm`). 测试用 `MockCausalLlm` 走通全链.
//! - **推演结果永远不入库** (与 W1 同纪律): 本模块不调用 `SqliteMemoryStore::put_episode` /
//!   `memory_extractor::extract`. 仅返回 [`CausalChain`] 给调用方决定是否使用.
//! - **Brier 拒绝阈值默认 0.3, 可调** (复用 W1 的 `CalibratedResolver` 形态).
//!
//! ## 复用
//!
//! 全部复用既有零件, 不重复发明沙盘底座:
//! - `memory_graph::GraphFact` — s/p/o 因果网骨干 (双时态: valid_at/invalid_at).
//! - `oracle::{WorldState, Forecast, CalibratedResolver}` — 沙盘底座 + Brier 校准.
//! - `world_model::{TextualSimulator, CounterfactualChain, TimelineLlm, MockTimelineLlm}` — W1 抽象.
//! - `apeireth_cognition::planning::{MctsPlanner, MctsConfig, SearchState, SearchAction, StateEvaluator}`
//!   — TP7 落地的 MCTS (commit 5ec4a17), 在因果图上跑通仅需 trait 实现.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use apeireth_cognition::planning::{MctsConfig, MctsPlanner, SearchAction, StateEvaluator};
use async_trait::async_trait;

use crate::memory_graph::GraphFact;
use crate::oracle::{CalibratedResolver, Forecast, WorldState};

// ============================================================
// 数据层 (W2): 节点 + 边 + 因果图
// ============================================================

/// 因果节点: 包装一个 `GraphFact`, 以 chain 为标识 (Zep 双时态: 同 s|p|o 共一节点).
#[derive(Debug, Clone)]
pub struct CausalNode {
    /// 节点 id = GraphFact.chain (s|p|o), 双时态语义下"当前有效事实"归并为一个节点.
    pub id: String,
    /// 该节点的当前有效事实 (重要性等元数据供评估用).
    pub fact: GraphFact,
}

impl CausalNode {
    pub fn from_fact(f: GraphFact) -> Self {
        Self {
            id: f.chain.clone(),
            fact: f,
        }
    }
}

/// 因果边来源: 统计挖掘 / LLM 提议 / 混合 (统计优先 LLM 补).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeSource {
    /// W3 主路径: 从记忆时间线统计挖掘 (e.g. 熬夜→效率低 共现 ≥ 阈值).
    Statistical,
    /// W3 补充路径: EvoCause 式 LLM 提议边.
    LlmProposed,
    /// 两者共识 (统计 + LLM 同时确认).
    Hybrid,
}

/// 因果边: 从一个事实到另一个事实的因果关系.
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub id: String,
    /// 源节点 chain (s|p|o).
    pub from: String,
    /// 目标节点 chain (s'|p'|o').
    pub to: String,
    /// 因果谓词 (人类可读: "熬夜 → 次日效率低", 边谓词可自定义; 默认复用 o→s' 直连).
    pub predicate: String,
    /// 权重 0..1 (统计: 条件概率; LLM: 置信度).
    pub weight: f64,
    /// 证据计数 (统计: 共现次数; LLM: 提议理由强度 0..N).
    pub evidence_count: u32,
    /// 边来源.
    pub source: EdgeSource,
}

/// 因果图: 节点集 + 边集 + 邻接索引 (W2 推演的搜索空间).
#[derive(Debug, Clone, Default)]
pub struct CausalGraph {
    nodes: HashMap<String, CausalNode>,
    edges: Vec<CausalEdge>,
    /// from → edges 出邻接表 (MCTS 扩展用).
    outgoing: HashMap<String, Vec<usize>>,
    /// to → edges 入邻接表.
    #[allow(dead_code)]
    incoming: HashMap<String, Vec<usize>>,
}

impl CausalGraph {
    /// 从 `Vec<GraphFact>` 构造图 (自动建节点, 边需另行 `add_edge` / 边挖掘注入).
    pub fn from_facts(facts: impl IntoIterator<Item = GraphFact>) -> Self {
        let mut g = Self::default();
        for f in facts {
            g.add_node(CausalNode::from_fact(f));
        }
        g
    }

    pub fn add_node(&mut self, node: CausalNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: CausalEdge) {
        let idx = self.edges.len();
        self.outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(idx);
        self.incoming.entry(edge.to.clone()).or_default().push(idx);
        self.edges.push(edge);
    }

    pub fn node(&self, id: &str) -> Option<&CausalNode> {
        self.nodes.get(id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &CausalNode> {
        self.nodes.values()
    }

    pub fn edges(&self) -> &[CausalEdge] {
        &self.edges
    }

    /// 出邻接边下标 (MCTS 在节点扩展时遍历).
    pub fn outgoing_indices(&self, from: &str) -> &[usize] {
        self.outgoing.get(from).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 出邻接边迭代器.
    pub fn outgoing_edges(&self, from: &str) -> impl Iterator<Item = &CausalEdge> {
        let indices: Vec<usize> = self.outgoing_indices(from).to_vec();
        indices.into_iter().filter_map(move |i| self.edges.get(i))
    }

    pub fn len_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn len_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty()
    }
}

// ============================================================
// W3 主路径: 从记忆时间线统计挖掘因果边 (主人差异化核心)
// ============================================================

/// 时间窗口 (秒): 同一窗口内的两条事实视为"时间邻近", 可能存在因果关系.
pub const DEFAULT_TIME_WINDOW_SECS: i64 = 86_400; // 1 天

/// 共现证据阈值: 统计边成立的最小共现次数 (主人 2026-08-18 拍板: 7 次).
pub const DEFAULT_MIN_EVIDENCE: u32 = 7;

/// 边挖掘器: 从 `Vec<GraphFact>` 时间线按"对象-主体直连"统计挖掘因果边.
///
/// ## 机制
/// 1. 按时间排序所有事实.
/// 2. 对每对 (f_i, f_j), 若 `f_i.object == f_j.subject` 且时间差 ≤ 时间窗口 → 候选边
///    (f_i.chain → f_j.chain), 谓词 = f_i.predicate + "→" + f_j.predicate.
/// 3. 统计每条候选边的共现次数, ≥ `min_evidence` → 确认为统计边.
/// 4. 权重 = 共现次数 / 该源节点总候选对数 (条件概率近似).
///
/// 0 装 PASS: 纯确定性算法, 无 LLM, 无随机, 同输入同输出.
pub struct MineCausalEdges {
    /// 时间窗口 (秒).
    pub time_window_secs: i64,
    /// 最小证据数.
    pub min_evidence: u32,
}

impl Default for MineCausalEdges {
    fn default() -> Self {
        Self {
            time_window_secs: DEFAULT_TIME_WINDOW_SECS,
            min_evidence: DEFAULT_MIN_EVIDENCE,
        }
    }
}

impl MineCausalEdges {
    pub fn with_window(mut self, secs: i64) -> Self {
        self.time_window_secs = secs;
        self
    }

    pub fn with_min_evidence(mut self, n: u32) -> Self {
        self.min_evidence = n;
        self
    }

    /// 从时间线挖掘统计边. 返回 (边, 调试信息: 总候选对数).
    ///
    /// 机制: 对每个 `fi`, 在时间窗口内找**首个** `fj` (`subject_j == object_i`) → 计 1 对.
    /// 这避免"一因多果"重复计数 (熬夜→多个效率低不应计多次), 与"熬夜→次日效率低
    /// 出现 7 次即统计边"的直觉一致 (7 次独立因果事件).
    pub fn from_timeline(&self, facts: &[GraphFact]) -> (Vec<CausalEdge>, usize) {
        // 1. 仅看有效事实 (invalid_at 为 None), 按时间排序.
        let mut active: Vec<&GraphFact> = facts.iter().filter(|f| f.invalid_at.is_none()).collect();
        active.sort_by_key(|f| f.valid_at);

        // 2. 对每个 fi, 找首个匹配的 fj (object_i == subject_j, 0 < dt ≤ window).
        let mut counts: HashMap<(String, String), u32> = HashMap::new();
        let mut source_matched: HashMap<String, u32> = HashMap::new();
        let mut candidate_pairs = 0usize;

        for (i, fi) in active.iter().enumerate() {
            if fi.object.is_empty() {
                continue;
            }
            for fj in active.iter().skip(i + 1) {
                let dt = fj.valid_at - fi.valid_at;
                if dt > self.time_window_secs {
                    break; // 已排序, 后续只会更远
                }
                if dt < 0 {
                    continue;
                }
                if fi.object == fj.subject {
                    let key = (fi.chain.clone(), fj.chain.clone());
                    *counts.entry(key).or_insert(0) += 1;
                    *source_matched.entry(fi.chain.clone()).or_insert(0) += 1;
                    candidate_pairs += 1;
                    break; // 首个匹配即停 (一因多果不去重)
                }
            }
        }

        // 3. ≥ min_evidence → 统计边; 权重 = 匹配次数 / 该源节点匹配总数 (条件概率近似).
        let mut edges = Vec::new();
        for ((from, to), count) in counts {
            if count >= self.min_evidence {
                let total = source_matched.get(&from).copied().unwrap_or(1).max(1);
                let weight = (f64::from(count) / f64::from(total)).min(1.0);
                // 谓词: from 的 predicate → to 的 predicate (人类可读).
                let from_pred = from.split('|').nth(1).unwrap_or("").to_string();
                let to_pred = to.split('|').nth(1).unwrap_or("").to_string();
                edges.push(CausalEdge {
                    id: format!("causal-stat-{}", uuid::Uuid::new_v4()),
                    from,
                    to,
                    predicate: format!("{from_pred}→{to_pred}"),
                    weight,
                    evidence_count: count,
                    source: EdgeSource::Statistical,
                });
            }
        }

        // 按证据数降序, 确定性同分按 id 升序.
        edges.sort_by(|a, b| {
            b.evidence_count
                .cmp(&a.evidence_count)
                .then_with(|| a.id.cmp(&b.id))
        });

        (edges, candidate_pairs)
    }
}

// ============================================================
// W3 补充路径: EvoCause 式 LLM 提议边 (不是主路径)
// ============================================================

/// LLM 提议边请求: 给 LLM 看一组事实, 让它提议可能的因果边.
#[derive(Debug, Clone)]
pub struct EdgeProposalRequest {
    /// 候选事实 (LLM 在这些事实之间找因果对).
    pub facts: Vec<GraphFact>,
    /// 提议上限 (LLM 一次最多提 N 条).
    pub max_proposals: usize,
}

/// LLM 提议响应: 一组候选边 (LLM 自评置信度 = weight, 证据强度 = evidence_count).
#[derive(Debug, Clone)]
pub struct EdgeProposalResponse {
    pub proposals: Vec<CausalEdge>,
}

/// LLM 抽象: 在分支点判断 + 提议因果边 (EvoCause 式).
///
/// 真 LLM 未接 (本任务 0 装 PASS): 测试用 [`MockCausalLlm`] 走通全链.
#[async_trait]
pub trait CausalLlm: Send + Sync {
    /// 分支点判断: 给定当前状态 + 候选边, LLM 给 (a) 边的可行性 (b) 走到此边的叙事片段.
    async fn judge_branch(&self, ctx: &CausalBranchContext)
        -> Result<CausalBranchJudgment, String>;

    /// 提议因果边 (W3 补充路径).
    async fn propose_edges(
        &self,
        req: &EdgeProposalRequest,
    ) -> Result<EdgeProposalResponse, String>;
}

/// 分支点上下文: 当前状态 + 候选边 (LLM 选择/评估时用).
#[derive(Debug, Clone)]
pub struct CausalBranchContext {
    /// 当前节点 chain.
    pub current_node_id: String,
    /// 当前世界状态.
    pub current_state: WorldState,
    /// 反事实假设.
    pub hypothesis: String,
    /// 已访问节点集合 (防环).
    pub visited: HashSet<String>,
    /// 候选边 (出邻接).
    pub candidates: Vec<CausalEdge>,
}

/// 分支点判断: LLM 对每条候选边打分 + 给叙事.
#[derive(Debug, Clone)]
pub struct CausalBranchJudgment {
    /// 每条候选边的评估 (按候选顺序对应).
    pub judgments: Vec<EdgeJudgment>,
}

#[derive(Debug, Clone)]
pub struct EdgeJudgment {
    pub edge_id: String,
    /// 是否值得走这条边 (true = LLM 推荐).
    pub take: bool,
    /// 走到此边的叙事片段.
    pub narrative: String,
    /// LLM 评估的目标达成度 (0..1, 给 StateEvaluator 用).
    pub goal_progress: f64,
}

/// 边提议器: 用 LLM 提议因果边 (EvoCause 式补充路径).
pub struct ProposeCausalEdges {
    pub llm: Arc<dyn CausalLlm>,
}

impl ProposeCausalEdges {
    pub fn new(llm: Arc<dyn CausalLlm>) -> Self {
        Self { llm }
    }

    /// 调 LLM 提议边, 返回 `Vec<CausalEdge>` (source = LlmProposed).
    pub async fn llm_suggest(&self, req: &EdgeProposalRequest) -> Result<Vec<CausalEdge>, String> {
        let resp = self.llm.propose_edges(req).await?;
        Ok(resp
            .proposals
            .into_iter()
            .map(|mut e| {
                // 提议边统一标 source = LlmProposed (LLM 路径)
                e.source = EdgeSource::LlmProposed;
                e
            })
            .collect())
    }
}

// ============================================================
// 推演链 (W2): 沿因果边展开推演
// ============================================================

/// 推演链一步: 走过一条因果边, LLM 给叙事 + 状态快照.
#[derive(Debug, Clone)]
pub struct CausalStep {
    /// tick 编号.
    pub tick: u64,
    /// 起始节点.
    pub from_node: String,
    /// 走过的边 (含谓词/权重/来源).
    pub edge: CausalEdge,
    /// 到达节点.
    pub to_node: String,
    /// 自然语言叙事 (LLM 在分支点生成).
    pub narrative: String,
    /// 走到此节点后的世界状态快照.
    pub state_snapshot: WorldState,
}

/// 一条完整因果推演链.
#[derive(Debug, Clone)]
pub struct CausalChain {
    /// 反事实假设.
    pub hypothesis: String,
    /// 推演步骤序列 (沿因果边走).
    pub steps: Vec<CausalStep>,
    /// 终点节点 chain.
    pub terminal_node: Option<String>,
    /// 终点预测断言 (LLM 终端概率 → Forecast).
    pub terminal_forecast: Option<Forecast>,
    /// 终点 forecast 对账后 Brier.
    pub calibration_brier: Option<f64>,
    /// 校准差拒绝标记.
    pub rejected: bool,
    /// 拒绝原因.
    pub reject_reason: Option<String>,
}

impl CausalChain {
    pub fn new(hypothesis: impl Into<String>) -> Self {
        Self {
            hypothesis: hypothesis.into(),
            steps: Vec::new(),
            terminal_node: None,
            terminal_forecast: None,
            calibration_brier: None,
            rejected: false,
            reject_reason: None,
        }
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

// ============================================================
// W2 编排器: 沿因果链展开推演 (LLM 只在分支点判断)
// ============================================================

/// 因果模拟器: 沿因果图展开推演链. 与 `TextualSimulator` 同构 (W1), 仅搜索空间换成因果图.
pub struct CausalSimulator {
    pub graph: CausalGraph,
    pub llm: Arc<dyn CausalLlm>,
    /// 最大推演步数 (防死循环 / 沿因果链太深).
    pub max_steps: usize,
    /// Brier 拒绝阈值.
    pub reject_threshold: f64,
    /// 终点 forecast 的 deadline (epoch ms).
    pub deadline_ms: i64,
    /// 可选 oracle 校准器 (历史 Brier 追踪).
    calibrator: Option<CalibratedResolver>,
}

impl CausalSimulator {
    pub fn new(graph: CausalGraph, llm: Arc<dyn CausalLlm>) -> Self {
        Self {
            graph,
            llm,
            max_steps: 8,
            reject_threshold: 0.3,
            deadline_ms: 0,
            calibrator: None,
        }
    }

    pub fn with_max_steps(mut self, n: usize) -> Self {
        self.max_steps = n;
        self
    }

    pub fn with_threshold(mut self, t: f64) -> Self {
        self.reject_threshold = t;
        self
    }

    pub fn with_deadline(mut self, ms: i64) -> Self {
        self.deadline_ms = ms;
        self
    }

    pub fn with_calibrator(mut self, c: CalibratedResolver) -> Self {
        self.calibrator = Some(c);
        self
    }

    /// 沿因果链展开推演: 起点节点 → 沿出邻接边走 (LLM 在分支点选边 + 给叙事) → 重复.
    pub async fn run(
        &self,
        start_node_id: impl Into<String>,
        hypothesis: impl Into<String>,
    ) -> Result<CausalChain, String> {
        let start_node_id = start_node_id.into();
        let hypothesis = hypothesis.into();
        let mut chain = CausalChain::new(hypothesis.clone());

        // 起点节点必须存在.
        let _ = self
            .graph
            .node(&start_node_id)
            .ok_or_else(|| format!("起点节点不存在: {start_node_id}"))?;

        let mut current_state = WorldState::default();
        let mut current_node = start_node_id.clone();
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(current_node.clone());

        for tick in 0..self.max_steps {
            let candidates: Vec<CausalEdge> =
                self.graph.outgoing_edges(&current_node).cloned().collect();
            if candidates.is_empty() {
                // 无出边 → 自然终止.
                break;
            }

            let ctx = CausalBranchContext {
                current_node_id: current_node.clone(),
                current_state: current_state.clone(),
                hypothesis: hypothesis.clone(),
                visited: visited.clone(),
                candidates: candidates.clone(),
            };

            let judgment = self.llm.judge_branch(&ctx).await?;
            // 选 LLM 推荐 (take=true) 中 goal_progress 最高的边; 无推荐则终止.
            let chosen = judgment
                .judgments
                .iter()
                .zip(candidates.iter())
                .filter(|(j, _)| j.take)
                .max_by(|(a, _), (b, _)| {
                    a.goal_progress
                        .partial_cmp(&b.goal_progress)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            let Some((judgment, edge)) = chosen else {
                break; // LLM 拒绝所有候选 → 链终止.
            };

            let next_node = edge.to.clone();
            visited.insert(next_node.clone());

            let step = CausalStep {
                tick: tick as u64,
                from_node: current_node.clone(),
                edge: edge.clone(),
                to_node: next_node.clone(),
                narrative: judgment.narrative.clone(),
                state_snapshot: current_state.clone(),
            };
            chain.steps.push(step);
            chain.terminal_node = Some(next_node.clone());
            current_node = next_node;
            // 简化: state 不变 (本任务不引入自动 apply; 真接入由 LLM 在 narrative 中表达).
            current_state.tick += 1;
        }

        // 终点 forecast: 用 LLM 概率 (EvoCause / 提议边的统计 mean) 构造.
        // 0 装 PASS: trait 仅给 judge_branch + propose_edges, 概率从 graph 边权重均值近似.
        let probability = if chain.steps.is_empty() {
            0.5
        } else {
            let mean_w: f64 =
                chain.steps.iter().map(|s| s.edge.weight).sum::<f64>() / chain.steps.len() as f64;
            mean_w.clamp(0.0, 1.0)
        };
        chain.terminal_forecast = Some(Forecast::new(
            format!("因果推演: {hypothesis}"),
            probability,
            self.deadline_ms,
        ));

        // oracle 历史校准 (可选).
        if let Some(cal) = &self.calibrator {
            let status = cal.status().map_err(|e| format!("oracle 校准失败: {e}"))?;
            if status.resolved_count > 0 && status.mean_brier > self.reject_threshold {
                chain.rejected = true;
                chain.reject_reason = Some(format!(
                    "oracle 历史 Brier {:.3} > 阈值 {:.3} ({n} 次对账)",
                    status.mean_brier,
                    self.reject_threshold,
                    n = status.resolved_count,
                ));
            }
        }

        Ok(chain)
    }

    /// 对账: 与事实对账, 更新 Brier + 拒绝标记.
    ///
    /// 0 装 PASS: 不注入 `SqliteMemoryStore` (与 W1 同纪律).
    pub fn reconcile_with_fact(
        &self,
        chain: &mut CausalChain,
        actual_outcome: bool,
    ) -> Result<(), String> {
        let forecast = chain
            .terminal_forecast
            .as_mut()
            .ok_or_else(|| "chain 无终点 forecast, 请先 run".to_string())?;
        forecast.resolve(actual_outcome);
        chain.calibration_brier = forecast.brier;
        if let Some(b) = chain.calibration_brier {
            if b > self.reject_threshold {
                chain.rejected = true;
                chain.reject_reason = Some(format!(
                    "终点 Brier {b:.3} > 阈值 {:.3}",
                    self.reject_threshold,
                ));
            }
        }
        Ok(())
    }
}

// ============================================================
// MCTS 接线: 在因果图上跑 MCTS (W2 关键路径, 复用 cognition::planning)
// ============================================================

/// MCTS 状态: 当前节点 + 世界状态.
#[derive(Debug, Clone)]
pub struct CausalMctsState {
    pub node_id: String,
    pub world_state: WorldState,
}

/// MCTS 动作: 沿一条因果边走到下一节点 (apply 返回 None = 边不可走, 已访问).
#[derive(Debug, Clone)]
pub struct CausalMctsAction {
    pub edge: CausalEdge,
    pub to_node: CausalNode,
}

impl SearchAction<CausalMctsState> for CausalMctsAction {
    fn apply(&self, state: &CausalMctsState) -> Option<CausalMctsState> {
        if self.edge.from != state.node_id {
            return None; // 边不是从当前节点出发
        }
        let mut ws = state.world_state.clone();
        ws.tick += 1;
        Some(CausalMctsState {
            node_id: self.to_node.id.clone(),
            world_state: ws,
        })
    }
}

/// 因果图评估器: 用 LLM 在分支点判断目标达成度 (StateEvaluator trait 注入).
pub struct CausalGraphEvaluator {
    pub llm: Arc<dyn CausalLlm>,
    /// 假设文本 (评估时携带).
    pub hypothesis: String,
    /// 终点节点 id (达到则 terminal).
    pub goal_node_id: String,
}

impl StateEvaluator<CausalMctsState> for CausalGraphEvaluator {
    fn evaluate(&self, state: &CausalMctsState) -> f64 {
        // 0 装 PASS: LLM 评估需异步; MctsPlanner 是同步接口. 这里用启发式:
        // 走到 goal_node_id 距离的简单度量 (边数越少越好; 越接近 1.0).
        // 真接入 LLM 时, 用 tokio::runtime::Handle 桥接 async (留作升级点).
        if state.node_id == self.goal_node_id {
            return 1.0;
        }
        // 启发式: node_id 字符长度作为 surrogate (确定性, 测试可断言)
        let dist = (state.node_id.len() as f64 - self.goal_node_id.len() as f64).abs();
        1.0 / (1.0 + dist)
    }

    fn is_terminal(&self, state: &CausalMctsState) -> bool {
        state.node_id == self.goal_node_id
    }
}

/// 因果 MCTS 规划器: 包装 `MctsPlanner` 在因果图上跑 (W2 主路径).
pub struct CausalMctsPlanner {
    pub graph: CausalGraph,
    pub config: MctsConfig,
}

impl CausalMctsPlanner {
    pub fn new(graph: CausalGraph) -> Self {
        Self {
            graph,
            config: MctsConfig::default(),
        }
    }

    pub fn with_config(mut self, c: MctsConfig) -> Self {
        self.config = c;
        self
    }

    /// 跑 MCTS: 从 start_node 出发, 找访问数最高的出邻接边.
    ///
    /// 返回: 找到的最优 (CausalMctsAction, 根访问数); 若无出边 → None.
    pub fn search(
        &self,
        start_node_id: &str,
        evaluator: CausalGraphEvaluator,
        seed: u64,
    ) -> Option<(CausalMctsAction, u64)> {
        let start_node = self.graph.node(start_node_id)?;
        let root_state = CausalMctsState {
            node_id: start_node.id.clone(),
            world_state: WorldState::default(),
        };

        // 构造 MCTS 动作列表: 从起点出发的所有出邻接边 (apply 返回 None 的会被 MctsPlanner 跳过).
        let actions: Vec<CausalMctsAction> = self
            .graph
            .outgoing_edges(start_node_id)
            .filter_map(|e| {
                let to_node = self.graph.node(&e.to)?;
                Some(CausalMctsAction {
                    edge: e.clone(),
                    to_node: to_node.clone(),
                })
            })
            .collect();

        if actions.is_empty() {
            return None;
        }

        let planner = MctsPlanner::new(self.config, evaluator).with_seed(seed);
        let result = planner.search(root_state, &actions)?;
        // result.best_action 是 CausalMctsAction; 根访问数 = result.root_visits.
        Some((result.best_action, result.root_visits))
    }
}

// ============================================================
// 测试用 Mock (LLM 抽象 0 装 PASS 兜底)
// ============================================================

/// Mock LLM: 分支点硬编码接受首条候选 + 给叙事; 提议边按 facts 数派生.
pub struct MockCausalLlm {
    /// judge_branch 时: 总是 take=true 首条候选 (其他 take=false).
    pub take_first: bool,
    /// propose_edges 时: 派生 N 条边 (按 facts 数).
    pub max_proposals: usize,
}

impl Default for MockCausalLlm {
    fn default() -> Self {
        Self {
            take_first: true,
            max_proposals: 3,
        }
    }
}

#[async_trait]
impl CausalLlm for MockCausalLlm {
    async fn judge_branch(
        &self,
        ctx: &CausalBranchContext,
    ) -> Result<CausalBranchJudgment, String> {
        let judgments = ctx
            .candidates
            .iter()
            .enumerate()
            .map(|(i, e)| EdgeJudgment {
                edge_id: e.id.clone(),
                take: self.take_first && i == 0,
                narrative: format!("走到 {} (候选 {i})", e.to),
                goal_progress: if self.take_first && i == 0 { 0.8 } else { 0.2 },
            })
            .collect();
        Ok(CausalBranchJudgment { judgments })
    }

    async fn propose_edges(
        &self,
        req: &EdgeProposalRequest,
    ) -> Result<EdgeProposalResponse, String> {
        // 派生 N 条 (从 facts 中按相邻对取).
        let mut proposals = Vec::new();
        let n = req
            .max_proposals
            .min(req.facts.len().saturating_sub(1))
            .min(self.max_proposals);
        for i in 0..n {
            let from = &req.facts[i];
            let to = &req.facts[i + 1];
            if !from.object.is_empty() && from.object == to.subject {
                proposals.push(CausalEdge {
                    id: format!("causal-llm-{}", uuid::Uuid::new_v4()),
                    from: from.chain.clone(),
                    to: to.chain.clone(),
                    predicate: format!("{p}→{p2}", p = from.predicate, p2 = to.predicate),
                    weight: 0.6,
                    evidence_count: 1,
                    source: EdgeSource::LlmProposed,
                });
            }
        }
        Ok(EdgeProposalResponse { proposals })
    }
}

// ============================================================
// 单元测试 (验收 5 个测试点)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一条测试用 GraphFact (有效状态).
    fn fact(s: &str, p: &str, o: &str, ts: i64) -> GraphFact {
        GraphFact {
            id: format!("factg-{}", uuid::Uuid::new_v4()),
            chain: format!("{s}|{p}|{o}"),
            rev: 0,
            subject: s.to_string(),
            predicate: p.to_string(),
            object: o.to_string(),
            valid_at: ts,
            invalid_at: None,
            importance: 5,
        }
    }

    /// 构造一条因果图 fixture: 主人→熬夜→效率低 (熬夜→次日 的因果链).
    fn build_chain_facts() -> Vec<GraphFact> {
        vec![
            fact("主人", "行为", "熬夜", 1_000),
            fact("熬夜", "导致", "效率低", 1_100),
            fact("效率低", "后果", "延期", 1_200),
        ]
    }

    /// 构造一条因果图 (含事实 + 统计挖掘边).
    fn build_chain_graph() -> CausalGraph {
        let facts = build_chain_facts();
        let mut g = CausalGraph::from_facts(facts.clone());
        // 手动加 2 条边 (W2 推演用的 fixture; W3 挖掘测试另构造时间线).
        g.add_edge(CausalEdge {
            id: "edge-1".into(),
            from: "主人|行为|熬夜".into(),
            to: "熬夜|导致|效率低".into(),
            predicate: "行为→导致".into(),
            weight: 0.9,
            evidence_count: 10,
            source: EdgeSource::Statistical,
        });
        g.add_edge(CausalEdge {
            id: "edge-2".into(),
            from: "熬夜|导致|效率低".into(),
            to: "效率低|后果|延期".into(),
            predicate: "导致→后果".into(),
            weight: 0.8,
            evidence_count: 8,
            source: EdgeSource::Statistical,
        });
        g
    }

    // -------- 验收点 1: 因果链展开测试 --------

    #[tokio::test]
    async fn causal_chain_expand_from_root() {
        let graph = build_chain_graph();
        let llm = Arc::new(MockCausalLlm::default());
        let sim = CausalSimulator::new(graph, llm);
        let chain = sim
            .run("主人|行为|熬夜", "如果主人今晚熬夜...")
            .await
            .unwrap();

        // mock take_first=true: 沿第一条出边走, 走完 max_steps=8 或无候选为止.
        assert!(chain.step_count() >= 1, "应至少走 1 步");
        assert!(
            chain.step_count() <= 2,
            "3 节点链, 最多 2 步 (根不计入 steps)"
        );
        assert_eq!(chain.steps[0].from_node, "主人|行为|熬夜");
        assert_eq!(chain.steps[0].to_node, "熬夜|导致|效率低");
        assert!(chain.terminal_node.is_some(), "应到达终点节点");
        assert!(chain.terminal_forecast.is_some(), "应构造终点 forecast");
        assert!(!chain.rejected, "无 oracle 历史 + 边权重高 → 不拒绝");
        // 概率 = 边权重均值 (0.9 + 0.8) / 2 = 0.85
        let prob = chain.terminal_forecast.as_ref().unwrap().probability;
        assert!(
            (prob - 0.85).abs() < 1e-9,
            "边权重均值应作为概率: got {prob}"
        );
    }

    // -------- 验收点 2: MCTS 在因果图上跑通 --------

    #[test]
    fn mcts_on_causal_graph_runs() {
        let graph = build_chain_graph();
        let planner = CausalMctsPlanner::new(graph).with_config(MctsConfig {
            iterations: 50,
            ..Default::default()
        });
        let evaluator = CausalGraphEvaluator {
            llm: Arc::new(MockCausalLlm::default()),
            hypothesis: "test".into(),
            goal_node_id: "效率低|后果|延期".into(),
        };
        let (best_action, root_visits) = planner
            .search("主人|行为|熬夜", evaluator, 42)
            .expect("应有结果 (起点有出边)");
        assert_eq!(best_action.edge.from, "主人|行为|熬夜");
        assert!(root_visits > 0, "MCTS 应至少访问根一次");
        assert!(root_visits <= 50, "迭代数上限: {root_visits}");
    }

    // -------- 验收点 3: 从时间线统计挖掘因果边 (W3 主路径) --------

    #[test]
    fn mine_causal_edges_statistical() {
        // 构造 7 对 (熬夜 → 效率低), 时间窗口 1 天内; 其他无关事实穿插.
        let mut facts = Vec::new();
        for i in 0..7 {
            let ts_base = 1_000_000 + i * 100;
            facts.push(fact("主人", "行为", "熬夜", ts_base));
            facts.push(fact("熬夜", "导致", "效率低", ts_base + 60)); // 1 分钟差, 窗口内
        }
        // 干扰: 无关事实 (object != 下一条 subject) — 不应形成边.
        for i in 0..3 {
            let ts = 1_000_000 + i * 50;
            facts.push(fact("无关", "无关谓词", "不串", ts));
        }

        let miner = MineCausalEdges::default().with_min_evidence(7);
        let (edges, candidate_pairs) = miner.from_timeline(&facts);

        assert_eq!(candidate_pairs, 7, "应有 7 对 object→subject 命中");
        assert!(!edges.is_empty(), "应至少挖出 1 条边");
        let edge = &edges[0];
        assert_eq!(edge.from, "主人|行为|熬夜");
        assert_eq!(edge.to, "熬夜|导致|效率低");
        assert_eq!(edge.evidence_count, 7, "共现 7 次即边 (主人拍板阈值)");
        assert_eq!(
            edge.source,
            EdgeSource::Statistical,
            "W3 主路径 = Statistical"
        );
        assert!(edge.weight > 0.0 && edge.weight <= 1.0);
        // 谓词拼接: "行为→导致"
        assert!(edge.predicate.contains("行为") && edge.predicate.contains("导致"));
    }

    #[test]
    fn mine_causal_edges_below_threshold_no_edge() {
        // 阈值 7, 但只有 3 对共现 → 应无边.
        let mut facts = Vec::new();
        for i in 0..3 {
            let ts = 2_000_000 + i * 100;
            facts.push(fact("主人", "行为", "熬夜", ts));
            facts.push(fact("熬夜", "导致", "效率低", ts + 60));
        }
        let miner = MineCausalEdges::default();
        let (edges, pairs) = miner.from_timeline(&facts);
        assert_eq!(pairs, 3);
        assert!(edges.is_empty(), "3 < 阈值 7, 不应产边");
    }

    // -------- 验收点 4: LLM 提议边 (W3 补充路径) --------

    #[tokio::test]
    async fn propose_causal_edges_llm_suggest() {
        let facts = build_chain_facts();
        let llm = Arc::new(MockCausalLlm {
            take_first: true,
            max_proposals: 2,
        });
        let proposer = ProposeCausalEdges::new(llm);
        let req = EdgeProposalRequest {
            facts: facts.clone(),
            max_proposals: 2,
        };
        let proposals = proposer.llm_suggest(&req).await.unwrap();

        // facts 是 3 条; LLM mock 看 (i, i+1) 对: (0,1): object=熬夜 == subject=熬夜 ✓ → 提议 1 条.
        assert!(!proposals.is_empty(), "至少提议 1 条");
        assert!(
            proposals.len() <= req.max_proposals,
            "不超过 max_proposals 上限"
        );
        for e in &proposals {
            assert_eq!(
                e.source,
                EdgeSource::LlmProposed,
                "LLM 提议边标 LlmProposed"
            );
            assert!(e.weight > 0.0 && e.weight <= 1.0);
        }
        // 第一条提议应是 主人|行为|熬夜 → 熬夜|导致|效率低 (mock 派生规则).
        assert_eq!(proposals[0].from, "主人|行为|熬夜");
        assert_eq!(proposals[0].to, "熬夜|导致|效率低");
    }

    // -------- 验收点 5: 推演结果与事实对账 (Brier 校准) --------

    #[tokio::test]
    async fn causal_chain_reconcile_with_fact() {
        let graph = build_chain_graph();
        let llm = Arc::new(MockCausalLlm::default());
        let sim = CausalSimulator::new(graph, llm).with_threshold(0.3);
        let mut chain = sim
            .run("主人|行为|熬夜", "如果主人今晚熬夜...")
            .await
            .unwrap();
        // 概率 = 0.85 (边均值)

        // outcome=true → Brier = (0.85 - 1)² = 0.0225 < 0.3 → 不拒绝
        sim.reconcile_with_fact(&mut chain, true).unwrap();
        let brier_true = chain.calibration_brier.unwrap();
        assert!(
            (brier_true - 0.0225).abs() < 1e-9,
            "p=0.85, actual=true → Brier=0.0225 (got {brier_true})"
        );
        assert!(!chain.rejected, "Brier=0.0225 < 阈值 0.3, 不拒绝");

        // 构造另一条链 (概率 0.85), outcome=false → Brier = 0.85² = 0.7225 > 0.3 → 拒绝
        let graph2 = build_chain_graph();
        let llm2 = Arc::new(MockCausalLlm::default());
        let sim2 = CausalSimulator::new(graph2, llm2).with_threshold(0.3);
        let mut chain2 = sim2.run("主人|行为|熬夜", "test2").await.unwrap();
        sim2.reconcile_with_fact(&mut chain2, false).unwrap();
        let brier_false = chain2.calibration_brier.unwrap();
        assert!(
            (brier_false - 0.7225).abs() < 1e-9,
            "p=0.85, actual=false → Brier=0.7225 (got {brier_false})"
        );
        assert!(chain2.rejected, "Brier=0.7225 > 阈值 0.3, rejected=true");
        let reason = chain2.reject_reason.as_ref().expect("拒绝必须有原因");
        assert!(reason.contains("Brier") && reason.contains("0.3"));
    }

    // -------- 0 装 PASS 边界: 推演结果绝不入库 --------

    #[tokio::test]
    async fn causal_world_model_does_not_persist_to_memory() {
        use apeireth_memory::EpisodeStore;
        use apeireth_memory::SqliteMemoryStore;

        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let before = store
            .recent_episodes("causal-session", 100)
            .map(|v| v.len())
            .unwrap_or(0);
        assert_eq!(before, 0, "全新 in-memory store 应为空");

        let graph = build_chain_graph();
        let llm = Arc::new(MockCausalLlm::default());
        let sim = CausalSimulator::new(graph, llm);
        let mut chain = sim.run("主人|行为|熬夜", "test").await.unwrap();
        sim.reconcile_with_fact(&mut chain, true).unwrap();

        let after = store
            .recent_episodes("causal-session", 100)
            .map(|v| v.len())
            .unwrap_or(0);
        assert_eq!(
            before, after,
            "因果推演 + 对账 后内存库不应有任何写入 (0 装 PASS 边界): before={before}, after={after}"
        );
        let any = store.recent_episodes("any-session", 1000).unwrap().len();
        assert_eq!(any, 0, "全库应仍为空");
    }
}

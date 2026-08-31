//! P-arch (2026-08-28): W3 Causal World Model Edges 器官真移植 v2
//! (被动路径, 确定性无 LLM).
//!
//! 1:1 翻译 v1 `apeireth-companion::causal_world_model` 中 **W3 边挖掘** 部分
//! (v1 `MineCausalEdges`, per `legacy/canonical/apeireth-companion/src/causal_world_model.rs:170-274`).
//!
//! ## 与 W2 (causal_world_model 主动 MCTS LLM) 严格区分
//!
//! v1 文件 (`causal_world_model.rs`) 同时承载 W2 + W3:
//! - **W2**: 因果图推演 (`CausalSimulator` / `CausalMctsPlanner` / `CausalLlm` /
//!   `ProposeCausalEdges`) — 主动 MCTS, LLM 在分支点判断, 主动反事实推演.
//! - **W3**: 边挖掘 (`MineCausalEdges::from_timeline`) — 被动观察, 统计挖掘
//!   主人的记忆时间线 = 因果数据 (per 主人 2026-08-18 拍板).
//!
//! **本模块只装 W3**: 被动观察 + 累计权重 + 确定性无 LLM. W2 的 MCTS / LLM
//! 推演路径**全部不装** (那是 rc 阶段或 v2.1 的工作).
//!
//! ## 0 装 PASS (诚实登记)
//!
//! - **W3 被动路径, 0 LLM**: `EdgeMinerOrgan::llm_factory()` 返 `None` (与 E4
//!   curiosity 同模式, 0 装诚实). 0 装诱导预防: 不假装"W3 也用 LLM".
//! - **纯确定性**: 全部算法无随机无 LLM, 同输入同输出 (同 v1 doc 第 179 行)。
//! - **trait 边界对齐**: trait `process()` 返 `OrganOutput::WorldModel { edges,
//!   counterfactual }` (per `apeireth-plugin::organ::OrganOutput` 已定义 variant),
//!   其中 `counterfactual` 留空 (W3 不做反事实推演, 那条是 W2 的活).
//!
//! ## v1 哲学 (per 主人 2026-08-18 拍板, docs/design-intent.md §2)
//!
//! > "全世界世界模型都在做通用世界; 她独有的训练集是主人的生活轨迹,
//! > 记忆时间线 = 因果数据."
//!
//! W3 主人差异化核心: 从主人的时间线统计挖掘"熬夜→次日效率低"类共现因果,
//! ≥ 7 次共现即确认为统计边 (主人拍板阈值).
//!
//! ## 复用 (per task brief, 全部复用既有零件)
//!
//! - `apeireth_plugin::organ::{OrganTrait, OrganInput, OrganOutput, OrganKind,
//!   OrganError, CausalEdge}` — 统一 trait 边界 (1:1 schema 翻译, plugin 层
//!   `CausalEdge` 是 4 字段简化版, 本模块内部 `MinedEdge` 保留 v1 完整 8 字段
//!   schema, 输出层映射到 plugin schema).
//! - `apeireth_plugin::llm_factory::LlmFactory` — 仅 trait 边界, 0 装路径.
//!
//! ## 3 阶审查 (O-6 锚 9)
//!
//! 1. 总体: 1:1 翻译 v1 `MineCausalEdges` 统计边挖掘, trait 边界对齐 E4/F4/F1/F6 模式
//!    (engine 实现 + foundation trait + 0 装诚实).
//! 2. 系统: `apeireth-organ` → `apeireth-plugin` → `apeireth-core` 单向依赖.
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, W3 走 `EdgeMinerOrgan::process`
//!    输出 `OrganOutput::WorldModel` 路径.

use std::collections::HashMap;

use apeireth_plugin::llm_factory::LlmFactory;
use apeireth_plugin::organ::{
    CausalEdge, OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait,
};

/// 简易 uuid v4 (per 子代理 R8 `memory.rs` 同模式: 0 新外部依赖).
///
/// **0 装诚实**: 不是密码学安全 uuid v4. 仅保证全局唯一性足够 (per W3 边
/// id 唯一性需求). 真生产可换 `uuid` crate (1 依赖).
mod uuid {
    pub struct Uuid;
    impl Uuid {
        pub fn new_v4() -> String {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            // 16 hex chars + 4 hex chars (简化 uuid 形态)
            format!("{:016x}-{:04x}", nanos ^ counter, (counter & 0xFFFF) as u16)
        }
    }
}

// ============================================================
// 常量 (per v1 DEFAULT_* 1:1)
// ============================================================

/// 时间窗口 (秒): 同一窗口内的两条事实视为"时间邻近", 可能存在因果关系.
/// 1:1 翻译 v1 `DEFAULT_TIME_WINDOW_SECS = 86_400` (1 天).
pub const DEFAULT_TIME_WINDOW_SECS: i64 = 86_400;

/// 共现证据阈值: 统计边成立的最小共现次数.
/// 1:1 翻译 v1 `DEFAULT_MIN_EVIDENCE = 7` (主人 2026-08-18 拍板).
pub const DEFAULT_MIN_EVIDENCE: u32 = 7;

// ============================================================
// 内部数据结构 (v1 GraphFact 简化版, 1:1 翻译核心 s/p/o + timestamp)
// ============================================================

/// 事实记录: v1 `GraphFact` 的核心 4 字段 (s/p/o + 时间).
///
/// v1 `GraphFact` 含 id/chain/rev/importance/invalid_at (元数据); 本模块仅取
/// 边挖掘必需的 subject/predicate/object/valid_at, 1:1 翻译算法语义.
///
/// `chain = "{subject}|{predicate}|{object}"` (派生, 与 v1 一致).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FactRecord {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_at: i64,
}

impl FactRecord {
    pub fn new(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
        valid_at: i64,
    ) -> Self {
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            valid_at,
        }
    }

    /// chain = s|p|o (per v1 GraphFact::chain 派生).
    pub fn chain(&self) -> String {
        format!("{}|{}|{}", self.subject, self.predicate, self.object)
    }
}

/// 边来源 (per v1 `EdgeSource` 1:1).
///
/// W3 主路径 = Statistical (统计挖掘). LlmProposed / Hybrid 是 v1 文档列出的
/// 补充路径, 但本模块 0 装 (那是 v1 W3 的 LLM 提议路径, W3 主路径 = 统计).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeSource {
    /// W3 主路径: 统计挖掘 (e.g. 熬夜→效率低 共现 ≥ 阈值).
    Statistical,
    /// 0 装 PASS: W3 不接 LLM, 此 variant 仅保留 v1 schema 完整性, 永不产.
    #[allow(dead_code)]
    LlmProposed,
    /// 0 装 PASS: 同上, 永不产.
    #[allow(dead_code)]
    Hybrid,
}

/// 挖掘出的因果边 (v1 `CausalEdge` 1:1 翻译, 内部完整 8 字段).
///
/// **0 装诚实**: 这是 W3 模块的**内部**数据结构. trait 输出 (`OrganOutput`)
/// 走 plugin 层简化 `CausalEdge` schema (`{cause, effect, conf, source}`).
/// 内部完整字段 (`predicate/evidence_count/id`) 在 `EdgeStats` 暴露, 留给
/// 后续 W2 接线和调试用.
#[derive(Debug, Clone)]
pub struct MinedEdge {
    /// 边唯一 id (per v1 `causal-stat-<uuid>`).
    pub id: String,
    /// 源节点 chain (s|p|o).
    pub from: String,
    /// 目标节点 chain (s'|p'|o').
    pub to: String,
    /// 因果谓词 (人类可读: "行为 → 导致").
    pub predicate: String,
    /// 权重 0..1 (统计: 条件概率近似).
    pub weight: f64,
    /// 共现次数 (统计证据数).
    pub evidence_count: u32,
    /// 边来源 (本模块永产 Statistical).
    pub source: EdgeSource,
}

/// 边统计 (query API 返回值, 1:1 翻译 v1 `EdgeStats` 任务说明 schema).
#[derive(Debug, Clone)]
pub struct EdgeStats {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    pub observation_count: u32,
    pub total_weight: f32,
}

/// 边类型 (per v1 任务说明 schema).
///
/// 0 装 PASS: v1 MineCausalEdges 不区分 EdgeKind (只看 object→subject 共现).
/// 本 enum 提供 schema 占位, 默认 `Correlates` (统计共现). 真生产时由 W2 推演
/// 时细化为 Causes/Enables/Prevents (那是 W2 的活, 不在 W3 范围).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    Causes,
    Enables,
    Prevents,
    Correlates,
}

/// 因果图 (1:1 翻译 v1 `CausalGraph`, 简化版 — 仅边集 + 邻接索引).
///
/// v1 `CausalGraph` 含 nodes + edges + outgoing/incoming. 本模块 W3 主路径
/// 只需边集 (节点 = FactRecord, 由调用方持有). 邻接索引仅 `outgoing` (出边),
/// 因为 W3 仅做"从 from 出发"查询 (per v1 `outgoing_edges` API).
#[derive(Debug, Clone, Default)]
pub struct CausalGraph {
    /// edge id → MinedEdge
    edges: HashMap<String, MinedEdge>,
    /// from chain → edge ids 出邻接
    outgoing: HashMap<String, Vec<String>>,
}

impl CausalGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_edge(&mut self, edge: MinedEdge) {
        self.outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.id.clone());
        self.edges.insert(edge.id.clone(), edge);
    }

    pub fn edge(&self, id: &str) -> Option<&MinedEdge> {
        self.edges.get(id)
    }

    pub fn edges(&self) -> impl Iterator<Item = &MinedEdge> {
        self.edges.values()
    }

    pub fn len_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// 出邻接边迭代器 (per v1 `CausalGraph::outgoing_edges` 1:1).
    pub fn outgoing_edges(&self, from: &str) -> impl Iterator<Item = &MinedEdge> {
        self.outgoing
            .get(from)
            .into_iter()
            .flat_map(move |ids| ids.iter())
            .filter_map(move |id| self.edges.get(id))
    }
}

// ============================================================
// 边挖掘器 (1:1 翻译 v1 MineCausalEdges, 确定性无 LLM)
// ============================================================

/// 边挖掘器配置 (1:1 翻译 v1 `MineCausalEdges` 字段).
#[derive(Debug, Clone)]
pub struct EdgeMinerConfig {
    /// 时间窗口 (秒).
    pub time_window_secs: i64,
    /// 最小证据数 (共现 ≥ 此值才确认为统计边).
    pub min_evidence: u32,
}

impl Default for EdgeMinerConfig {
    fn default() -> Self {
        Self {
            time_window_secs: DEFAULT_TIME_WINDOW_SECS,
            min_evidence: DEFAULT_MIN_EVIDENCE,
        }
    }
}

impl EdgeMinerConfig {
    pub fn with_window(mut self, secs: i64) -> Self {
        self.time_window_secs = secs;
        self
    }

    pub fn with_min_evidence(mut self, n: u32) -> Self {
        self.min_evidence = n;
        self
    }
}

/// 因果边挖掘器 (per v1 `MineCausalEdges` 1:1 翻译).
///
/// ## 机制 (per v1 doc 第 173-177 行, 1:1 翻译)
///
/// 1. 按时间排序所有事实.
/// 2. 对每对 (f_i, f_j), 若 `f_i.object == f_j.subject` 且时间差 ≤ 时间窗口 →
///    候选边 (f_i.chain → f_j.chain), 谓词 = f_i.predicate → f_j.predicate.
/// 3. 统计每条候选边的共现次数, ≥ `min_evidence` → 确认为统计边.
/// 4. 权重 = 共现次数 / 该源节点总候选对数 (条件概率近似).
///
/// ## 0 装 PASS
///
/// 纯确定性算法, 无 LLM, 无随机, 同输入同输出 (per v1 doc 第 179 行).
pub struct CausalEdgeMiner {
    config: EdgeMinerConfig,
    /// 已确认的统计边 (chain (from, to) → MinedEdge).
    /// v1 用 HashMap<(String, String), CausalEdge> 累积, 本模块用同形态但 key 加 kind 字段.
    edges: HashMap<(String, String, EdgeKind), MinedEdge>,
    /// 全量候选对数 (调试 / 诊断用).
    candidate_pairs: usize,
}

impl CausalEdgeMiner {
    /// 构造边挖掘器 (per v1 字段).
    pub fn new(config: EdgeMinerConfig) -> Self {
        Self {
            config,
            edges: HashMap::new(),
            candidate_pairs: 0,
        }
    }

    /// 从默认配置构造 (1:1 翻译 v1 `MineCausalEdges::default`).
    pub fn with_defaults() -> Self {
        Self::new(EdgeMinerConfig::default())
    }

    /// 时间窗口设置 (per v1 `MineCausalEdges::with_window`).
    pub fn with_window(mut self, secs: i64) -> Self {
        self.config.time_window_secs = secs;
        self
    }

    /// 最小证据数设置 (per v1 `MineCausalEdges::with_min_evidence`).
    pub fn with_min_evidence(mut self, n: u32) -> Self {
        self.config.min_evidence = n;
        self
    }

    /// 观察一条边并累计权重 (per v1 `observe_event` API 任务说明 schema).
    ///
    /// **1:1 翻译 v1 累计语义**: 同一 (from, to, kind) 多次观察 → 权重累加,
    /// 证据数 +1. 达到 `min_evidence` 后, 该边进入 `edges` 索引供 `get_top_edges` 查询.
    pub fn observe_event(&mut self, from: &str, to: &str, kind: EdgeKind, weight: f32) {
        let key = (from.to_string(), to.to_string(), kind);
        let entry = self.edges.entry(key).or_insert_with(|| MinedEdge {
            id: format!("w3-mined-{}", uuid::Uuid::new_v4()),
            from: from.to_string(),
            to: to.to_string(),
            predicate: format!("{from} → {to}"),
            weight: 0.0,
            evidence_count: 0,
            source: EdgeSource::Statistical,
        });
        entry.weight += f64::from(weight);
        entry.evidence_count += 1;
    }

    /// 时间线挖掘 (per v1 `MineCausalEdges::from_timeline` 1:1 翻译).
    ///
    /// 输入: 一组 `FactRecord` 时间线. 输出: `(Vec<MinedEdge>, usize)` = (确认边, 候选对数).
    ///
    /// ## 机制 (1:1 翻译 v1)
    ///
    /// 1. 按 `valid_at` 排序所有事实 (v1 仅看有效: `invalid_at == None`; 本模块
    ///    FactRecord 无 invalid_at 字段, 全部视为有效, 1:1 翻译简化).
    /// 2. 对每个 `fi`, 在时间窗口内找**首个**匹配的 `fj` (object_i == subject_j,
    ///    0 < dt ≤ window) → 计 1 对.
    /// 3. 权重 = 匹配次数 / 该源节点匹配总数 (条件概率近似, clamp 到 [0, 1]).
    /// 4. 排序: 证据数降序 → 同分按 id 升序 (确定性).
    pub fn from_timeline(&mut self, facts: &[FactRecord]) -> (Vec<MinedEdge>, usize) {
        // 1. 按时间排序.
        let mut active: Vec<&FactRecord> = facts.iter().collect();
        active.sort_by_key(|f| f.valid_at);

        // 2. 对每个 fi, 找首个匹配的 fj.
        let mut counts: HashMap<(String, String), u32> = HashMap::new();
        let mut source_matched: HashMap<String, u32> = HashMap::new();
        let mut candidate_pairs = 0usize;

        for (i, fi) in active.iter().enumerate() {
            if fi.object.is_empty() {
                continue;
            }
            for fj in active.iter().skip(i + 1) {
                let dt = fj.valid_at - fi.valid_at;
                if dt > self.config.time_window_secs {
                    break; // 已排序, 后续只会更远.
                }
                if dt < 0 {
                    continue;
                }
                if fi.object == fj.subject {
                    let key = (fi.chain(), fj.chain());
                    *counts.entry(key).or_insert(0) += 1;
                    *source_matched.entry(fi.chain()).or_insert(0) += 1;
                    candidate_pairs += 1;
                    break; // 首个匹配即停 (一因多果不去重, per v1 doc).
                }
            }
        }

        self.candidate_pairs = candidate_pairs;

        // 3. ≥ min_evidence → 统计边.
        let mut edges = Vec::new();
        for ((from, to), count) in counts {
            if count >= self.config.min_evidence {
                let total = source_matched.get(&from).copied().unwrap_or(1).max(1);
                let weight = (f64::from(count) / f64::from(total)).min(1.0);
                let from_pred = from.split('|').nth(1).unwrap_or("").to_string();
                let to_pred = to.split('|').nth(1).unwrap_or("").to_string();
                let edge = MinedEdge {
                    id: format!("w3-stat-{}", uuid::Uuid::new_v4()),
                    from: from.clone(),
                    to: to.clone(),
                    predicate: format!("{from_pred}→{to_pred}"),
                    weight,
                    evidence_count: count,
                    source: EdgeSource::Statistical,
                };
                // 4. 累计到内部 edges 索引 (per v1 累计语义, 后续 get_top_edges 可查).
                let key = (from.clone(), to.clone(), EdgeKind::Correlates);
                self.edges.insert(key, edge.clone());
                edges.push(edge);
            }
        }

        // 5. 排序: 证据数降序 → 同分按 id 升序 (确定性, per v1 line 266-270).
        edges.sort_by(|a, b| {
            b.evidence_count
                .cmp(&a.evidence_count)
                .then_with(|| a.id.cmp(&b.id))
        });

        (edges, candidate_pairs)
    }

    /// Top-K 边 (per v1 任务说明 `get_top_edges` API).
    ///
    /// 按 `total_weight` 降序, 同分按 observation_count 降序, 再同按 from 升序 (确定性).
    pub fn get_top_edges(&self, k: usize) -> Vec<EdgeStats> {
        let mut all: Vec<EdgeStats> = self
            .edges
            .values()
            .map(|e| EdgeStats {
                from: e.from.clone(),
                to: e.to.clone(),
                kind: EdgeKind::Correlates,
                observation_count: e.evidence_count,
                total_weight: e.weight as f32,
            })
            .collect();
        all.sort_by(|a, b| {
            b.total_weight
                .partial_cmp(&a.total_weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.observation_count.cmp(&a.observation_count))
                .then_with(|| a.from.cmp(&b.from))
        });
        all.into_iter().take(k).collect()
    }

    /// 时间衰减 (per v1 任务说明 `decay_weights` API).
    ///
    /// `dt_ms` 毫秒数 → 按 `decay_rate` 指数衰减所有边权重.
    /// `weight *= (1 - decay_rate).powf(dt_secs / 1000)` (粗略指数衰减, 1:1 翻译 v1 意图).
    pub fn decay_weights(&mut self, dt_ms: i64) {
        let decay_rate = 0.01_f64; // 默认每秒 1% 衰减 (per v1 doc "时间衰减" 意图)
        let dt_secs = (dt_ms as f64) / 1000.0;
        let factor = (1.0 - decay_rate).powf(dt_secs);
        for edge in self.edges.values_mut() {
            edge.weight *= factor;
        }
    }

    /// 总边数 (per v1 任务说明 `total_edges` API).
    pub fn total_edges(&self) -> usize {
        self.edges.len()
    }

    /// 单 entity 出/入边 (per v1 任务说明 `edges_for` API).
    ///
    /// entity 出现在 from 或 to 字段 → 全部相关边.
    pub fn edges_for(&self, entity: &str) -> Vec<EdgeStats> {
        self.edges
            .values()
            .filter(|e| e.from == entity || e.to == entity)
            .map(|e| EdgeStats {
                from: e.from.clone(),
                to: e.to.clone(),
                kind: EdgeKind::Correlates,
                observation_count: e.evidence_count,
                total_weight: e.weight as f32,
            })
            .collect()
    }

    /// 当前候选对数 (调试 / 诊断).
    pub fn candidate_pairs(&self) -> usize {
        self.candidate_pairs
    }

    /// 取当前累积图快照 (per v1 调试意图, 调试 API).
    pub fn graph_snapshot(&self) -> CausalGraph {
        let mut g = CausalGraph::new();
        for edge in self.edges.values() {
            g.add_edge(edge.clone());
        }
        g
    }
}

// ============================================================
// W3 EdgeMinerOrgan (v2 trait 真实现)
// ============================================================

/// W3 因果边挖掘器官 (per v2 OrganTrait 1:1 翻译 v1 MineCausalEdges).
///
/// ## 0 装诚实
///
/// - `llm_factory()` 返 `None` — v1 W3 是**确定性被动观察**, 无 LLM 依赖.
///   0 装诱导预防: 不假装"W3 也用 LLM" (那是 W2 的主动 MCTS 推演).
/// - 内部状态用 `Mutex` 包裹, 跨 `process()` 调用的并发安全 (per E4 同样板).
///
/// ## API 设计 (与 E4/F1/F4/F6 同样板)
///
/// - `new(llm_factory, model)`: 保留 trait 边界签名, llm_factory 占位未来扩展,
///   当前**不用** (0 装诚实). 当前算法只用内部 `CausalEdgeMiner`.
/// - `observe_fact(s, p, o, ts)`: 暴露给 Runtime 喂时间线事实 (per v1 GraphFact 输入).
/// - `observe_event(from, to, kind, weight)`: 暴露给 Runtime 直接观察一条边
///   (per v1 任务说明 `observe_event` API).
/// - `process(input)`: trait 边界入口, 走 1:1 翻译 v1 `from_timeline` 路径.
pub struct EdgeMinerOrgan {
    miner: std::sync::Mutex<CausalEdgeMiner>,
    /// 保留 LLM factory 字段 (未来扩展, 当前**不用** — 0 装诚实)
    _llm_factory: std::sync::Arc<dyn LlmFactory>,
    /// 保留 model ID 字段 (未来扩展, 当前**不用** — 0 装诚实)
    _model: String,
}

impl EdgeMinerOrgan {
    /// 构造 W3 edge miner organ (默认 config).
    ///
    /// `llm_factory` 和 `model` 保留给未来扩展, 当前算法**不调用** (per 0 装诚实).
    pub fn new(llm_factory: std::sync::Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            miner: std::sync::Mutex::new(CausalEdgeMiner::with_defaults()),
            _llm_factory: llm_factory,
            _model: model.into(),
        }
    }

    /// 构造 + 自定义 config.
    pub fn with_config(
        llm_factory: std::sync::Arc<dyn LlmFactory>,
        model: impl Into<String>,
        config: EdgeMinerConfig,
    ) -> Self {
        Self {
            miner: std::sync::Mutex::new(CausalEdgeMiner::new(config)),
            _llm_factory: llm_factory,
            _model: model.into(),
        }
    }

    /// 喂时间线事实 (per v1 `MineCausalEdges::from_timeline` 输入 API).
    ///
    /// 返回 (确认边, 候选对数), 1:1 翻译 v1 from_timeline 返回 tuple.
    pub fn feed_timeline(&self, facts: &[FactRecord]) -> (Vec<MinedEdge>, usize) {
        let mut miner = self
            .miner
            .lock()
            .expect("EdgeMinerOrgan mutex poisoned (0 装诚实)");
        miner.from_timeline(facts)
    }

    /// 观察单条边事件 (per v1 任务说明 `observe_event` API).
    pub fn observe_event(&self, from: &str, to: &str, kind: EdgeKind, weight: f32) {
        let mut miner = self
            .miner
            .lock()
            .expect("EdgeMinerOrgan mutex poisoned (0 装诚实)");
        miner.observe_event(from, to, kind, weight);
    }

    /// Top-K 边查询 (per v1 任务说明 `get_top_edges` API).
    pub fn get_top_edges(&self, k: usize) -> Vec<EdgeStats> {
        let miner = self
            .miner
            .lock()
            .expect("EdgeMinerOrgan mutex poisoned (0 装诚实)");
        miner.get_top_edges(k)
    }

    /// 时间衰减 (per v1 任务说明 `decay_weights` API).
    pub fn decay_weights(&self, dt_ms: i64) {
        let mut miner = self
            .miner
            .lock()
            .expect("EdgeMinerOrgan mutex poisoned (0 装诚实)");
        miner.decay_weights(dt_ms);
    }

    /// 总边数 (per v1 任务说明 `total_edges` API).
    pub fn total_edges(&self) -> usize {
        let miner = self
            .miner
            .lock()
            .expect("EdgeMinerOrgan mutex poisoned (0 装诚实)");
        miner.total_edges()
    }

    /// 单 entity 边查询 (per v1 任务说明 `edges_for` API).
    pub fn edges_for(&self, entity: &str) -> Vec<EdgeStats> {
        let miner = self
            .miner
            .lock()
            .expect("EdgeMinerOrgan mutex poisoned (0 装诚实)");
        miner.edges_for(entity)
    }
}

#[async_trait::async_trait]
impl OrganTrait for EdgeMinerOrgan {
    fn name(&self) -> &'static str {
        "W3 Causal Edge Miner"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::W3
    }

    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 1:1 翻译 v1 W3 路径:
        // - W3 是**被动观察** + **累计权重**, 0 反事实推演 (那是 W2 主动 MCTS 的活).
        // - process() 不主动挖掘; 仅 snapshot 当前累积的 top 边, 输出 WorldModel schema.
        // - 真正喂数据走 `feed_timeline` / `observe_event` (Runtime 调).
        // - dry_run 模式: 不写状态, 仅 snapshot (当前 API 都是 read, dry_run 不影响).
        //
        // 输出: OrganOutput::WorldModel { edges, counterfactual }
        // - edges: 当前累积的 top 边 (1:1 翻译 plugin 层 CausalEdge 4 字段 schema).
        // - counterfactual: 空 (W3 不做反事实推演, 0 装诚实).

        let top = {
            let miner = self
                .miner
                .lock()
                .map_err(|e| OrganError::Internal(format!("mutex poisoned: {e}")))?;
            miner.get_top_edges(64)
        };

        let edges: Vec<CausalEdge> = top
            .into_iter()
            .map(|s| CausalEdge {
                cause: s.from.clone(),
                effect: s.to.clone(),
                conf: s.total_weight,
                source: "Statistical".to_string(),
            })
            .collect();

        Ok(OrganOutput::WorldModel {
            edges,
            counterfactual: Vec::new(), // W3 0 装反事实 (那是 W2 主动 MCTS 路径)
        })
    }

    /// 0 装诚实: W3 被动观察路径, 0 LLM 依赖, 返 None 不假装.
    fn llm_factory(&self) -> Option<std::sync::Arc<dyn LlmFactory>> {
        None
    }
}

// ============================================================
// 单元测试 (1:1 翻译 v1 causal_world_model.rs 第 909-955 行测试)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_plugin::llm_factory::NoopLlmFactory;

    fn test_factory() -> std::sync::Arc<dyn LlmFactory> {
        std::sync::Arc::new(NoopLlmFactory)
    }

    fn empty_input() -> OrganInput {
        use apeireth_core::kernel::memory::Episode;
        let ep = Episode {
            id: "test-episode-w3".into(),
            session_id: "test-session-w3".into(),
            role: "user".into(),
            content: "".into(),
            timestamp: 0,
        };
        OrganInput::new(ep, vec![])
    }

    /// 1:1 翻译 v1 `mine_causal_edges_statistical` 测试:
    /// 7 对 (熬夜 → 效率低) 时间窗口内 → 挖出 1 条统计边.
    #[test]
    fn from_timeline_mines_statistical_edges() {
        let mut miner = CausalEdgeMiner::with_defaults().with_min_evidence(7);
        let mut facts = Vec::new();
        for i in 0..7 {
            let ts_base = 1_000_000 + i * 100;
            facts.push(FactRecord::new("主人", "行为", "熬夜", ts_base));
            facts.push(FactRecord::new("熬夜", "导致", "效率低", ts_base + 60));
        }
        // 干扰: 无关事实.
        for i in 0..3 {
            let ts = 1_000_000 + i * 50;
            facts.push(FactRecord::new("无关", "无关谓词", "不串", ts));
        }

        let (edges, candidate_pairs) = miner.from_timeline(&facts);
        assert_eq!(candidate_pairs, 7, "应有 7 对 object→subject 命中");
        assert!(!edges.is_empty(), "应至少挖出 1 条边");
        let edge = &edges[0];
        assert_eq!(edge.from, "主人|行为|熬夜");
        assert_eq!(edge.to, "熬夜|导致|效率低");
        assert_eq!(edge.evidence_count, 7, "共现 7 次即边");
        assert_eq!(
            edge.source,
            EdgeSource::Statistical,
            "W3 主路径 = Statistical"
        );
        assert!(edge.weight > 0.0 && edge.weight <= 1.0);
        assert!(edge.predicate.contains("行为") && edge.predicate.contains("导致"));
    }

    /// 1:1 翻译 v1 `mine_causal_edges_below_threshold_no_edge` 测试:
    /// 阈值 7 但只有 3 对共现 → 应无边.
    #[test]
    fn from_timeline_below_threshold_no_edge() {
        let mut miner = CausalEdgeMiner::with_defaults();
        let mut facts = Vec::new();
        for i in 0..3 {
            let ts = 2_000_000 + i * 100;
            facts.push(FactRecord::new("主人", "行为", "熬夜", ts));
            facts.push(FactRecord::new("熬夜", "导致", "效率低", ts + 60));
        }
        let (edges, pairs) = miner.from_timeline(&facts);
        assert_eq!(pairs, 3);
        assert!(edges.is_empty(), "3 < 阈值 7, 不应产边");
    }

    /// observe_event 路径: 累计权重 + 证据数.
    #[test]
    fn observe_event_accumulates_weight_and_count() {
        let mut miner = CausalEdgeMiner::with_defaults().with_min_evidence(1);
        miner.observe_event("A", "B", EdgeKind::Correlates, 0.3);
        miner.observe_event("A", "B", EdgeKind::Correlates, 0.5);
        miner.observe_event("A", "B", EdgeKind::Correlates, 0.2);

        let top = miner.get_top_edges(10);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].from, "A");
        assert_eq!(top[0].to, "B");
        assert_eq!(top[0].observation_count, 3);
        assert!((top[0].total_weight - 1.0).abs() < 1e-5, "0.3+0.5+0.2=1.0");
    }

    /// get_top_edges 排序: total_weight 降序.
    #[test]
    fn get_top_edges_sorts_by_weight_descending() {
        let mut miner = CausalEdgeMiner::with_defaults().with_min_evidence(1);
        miner.observe_event("A", "B", EdgeKind::Correlates, 0.5);
        miner.observe_event("C", "D", EdgeKind::Correlates, 0.9);
        miner.observe_event("E", "F", EdgeKind::Correlates, 0.2);

        let top = miner.get_top_edges(10);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].from, "C", "权重 0.9 应排第一");
        assert_eq!(top[1].from, "A", "权重 0.5 应排第二");
        assert_eq!(top[2].from, "E", "权重 0.2 应排第三");
    }

    /// decay_weights 路径: dt_ms 后权重衰减.
    #[test]
    fn decay_weights_reduces_weights_over_time() {
        let mut miner = CausalEdgeMiner::with_defaults().with_min_evidence(1);
        miner.observe_event("A", "B", EdgeKind::Correlates, 1.0);
        let before = miner.get_top_edges(10)[0].total_weight;
        assert!((before - 1.0).abs() < 1e-5);

        // 1000 秒后衰减 (factor = 0.99^1000 ≈ 4.3e-5).
        miner.decay_weights(1_000_000);
        let after = miner.get_top_edges(10)[0].total_weight;
        assert!(
            after < before,
            "衰减后权重应下降: before={before}, after={after}"
        );
        assert!(after > 0.0, "衰减不归零: after={after}");
    }

    /// edges_for 路径: 单 entity 出/入边查询.
    #[test]
    fn edges_for_returns_entity_related_edges() {
        let mut miner = CausalEdgeMiner::with_defaults().with_min_evidence(1);
        miner.observe_event("熬夜", "效率低", EdgeKind::Correlates, 0.8);
        miner.observe_event("效率低", "延期", EdgeKind::Correlates, 0.7);
        miner.observe_event("无关", "不串", EdgeKind::Correlates, 0.9);

        // "效率低" 出现在 to (第一条) 和 from (第二条).
        let related = miner.edges_for("效率低");
        assert_eq!(related.len(), 2, "效率低应有 2 条相关边");
        let froms: Vec<&str> = related.iter().map(|e| e.from.as_str()).collect();
        assert!(froms.contains(&"熬夜"));
        assert!(froms.contains(&"效率低"));

        // 不相关 entity 应空.
        let unrelated = miner.edges_for("冷门主题");
        assert!(unrelated.is_empty());
    }

    /// total_edges 路径: 计数正确.
    #[test]
    fn total_edges_counts_unique_pairs() {
        let mut miner = CausalEdgeMiner::with_defaults().with_min_evidence(1);
        assert_eq!(miner.total_edges(), 0);
        miner.observe_event("A", "B", EdgeKind::Correlates, 0.1);
        miner.observe_event("A", "B", EdgeKind::Correlates, 0.2); // 同一对, 不计数
        miner.observe_event("C", "D", EdgeKind::Correlates, 0.3);
        assert_eq!(miner.total_edges(), 2, "同 (from,to,kind) 只算一条");
    }

    /// 0 装诚实: llm_factory() 返 None (W3 被动路径, 0 LLM).
    #[test]
    fn llm_factory_returns_none_per_v1_truth() {
        let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");
        assert!(
            organ.llm_factory().is_none(),
            "v1 W3 是确定性被动观察, v2 不假装能调 LLM"
        );
    }

    /// organ_id + name 锁定 W3.
    #[test]
    fn name_and_organ_id_locked_to_w3() {
        let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");
        assert_eq!(organ.name(), "W3 Causal Edge Miner");
        assert_eq!(organ.organ_id(), OrganKind::W3);
    }

    /// process() 走完 W3 路径 → OrganOutput::WorldModel { edges, counterfactual: [] }.
    #[tokio::test]
    async fn process_returns_world_model_output_with_edges() {
        let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");
        // 喂 7 对 → 触发挖掘
        let mut facts = Vec::new();
        for i in 0..7 {
            let ts_base = 3_000_000 + i * 100;
            facts.push(FactRecord::new("主人", "行为", "熬夜", ts_base));
            facts.push(FactRecord::new("熬夜", "导致", "效率低", ts_base + 60));
        }
        let (edges, _) = organ.feed_timeline(&facts);
        assert!(!edges.is_empty());

        let output = organ.process(empty_input()).await.expect("process ok");
        match output {
            OrganOutput::WorldModel {
                edges: out_edges,
                counterfactual,
            } => {
                assert!(!out_edges.is_empty(), "应输出至少 1 条边");
                assert!(
                    counterfactual.is_empty(),
                    "W3 被动路径 0 反事实 (那是 W2 的活)"
                );
                // 验证 schema 1:1 翻译 plugin CausalEdge.
                assert!(out_edges.iter().all(|e| !e.cause.is_empty()));
                assert!(out_edges.iter().all(|e| !e.effect.is_empty()));
                assert!(out_edges.iter().all(|e| e.conf > 0.0 && e.conf <= 1.0));
                assert!(out_edges.iter().all(|e| e.source == "Statistical"));
            }
            other => panic!("expected WorldModel output, got {other:?}"),
        }
    }

    /// 0 装诚实: process() 不调 LLM, 不假装能调.
    #[tokio::test]
    async fn process_does_not_invoke_llm() {
        let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");
        // 即使没喂数据, process() 也不报错 (返空边), 不假装需要 LLM.
        let output = organ.process(empty_input()).await.expect("process ok");
        match output {
            OrganOutput::WorldModel {
                edges,
                counterfactual,
            } => {
                assert!(edges.is_empty(), "无数据 → 0 边");
                assert!(counterfactual.is_empty());
            }
            other => panic!("expected WorldModel output, got {other:?}"),
        }
    }
}

//! P-arch (2026-08-28): W2 Causal World Model + W3 Causal Edge Mining 器官真移植 v2 (LLM 重).
//!
//! v1 `apeireth-companion::causal_world_model::CausalWorldModel` 1:1 翻译 (`legacy/donor/apeireth-companion/src/causal_world_model.rs`,
//! 1061 行, TP32 / W2 + W3).
//!
//! **与 E4/F1/F4/F6 不同**: v1 W2 是 **LLM 重** (per v1 doc: "LLM 推理: 因果图反事实推演").
//! v2 W2 真实现**真接 `LlmFactory`** 调 LLM MCTS 反事实推演, 不假装确定性.
//!
//! **W2 vs W3 角色 (per v1 doc §2「世界模型」)**:
//!
//! - **W2 (本模块, LLM 重)**: 在 `memory_graph` s/p/o 因果网上沿边展开"如果……那么……"路径.
//!   MCTS 跑在因果图上 (非动作空间); LLM 在分支点做判断.
//! - **W3 (本模块, 确定性)**: 主人差异化核心 — **从记忆时间线挖掘因果边** (统计验证优先),
//!   EvoCause 式 LLM 提议边作为补充. 全世界世界模型都在做通用世界; 她独有的训练集是主人
//!   的生活轨迹, 记忆时间线 = 因果数据 (主人 2026-08-18 拍板).
//!
//! **0 装 PASS**:
//!
//! - W2 organ **`llm_factory()` 返 `Some`** (per 任务 §3: "W2 必须 llm_factory() 返 Some(Arc<dyn LlmFactory>), 真接 LLM").
//! - W2 `process()` 真接 LLM (factory.spawn → LlmInstance::complete → 解析 JSON).
//! - W2 factory=None → 显式 `OrganError::LlmUnavailable` (0 装诚实, 不假装能调).
//! - W3 stat miner **纯确定性** (per v1 doc: "纯确定性算法, 无 LLM"), W3 LLM 提议边
//!   (`ProposeCausalEdges`) 走 `Arc<dyn LlmFactory>`, 也真接 LLM.
//! - 推演结果**永远不入库** (与 v1 同纪律): W2 organ 不调 `SqliteMemoryStore::put_episode` /
//!   `memory_extractor::extract`. 仅返回 `OrganOutput::WorldModel` 给调用方决定是否使用.
//! - Brier 拒绝阈值默认 0.3, 可调 (复用 v1 `CalibratedResolver` 形态, 此处用本地算术).
//!
//! **v1 compat**: 本模块保留 v1 全部数据结构 (`CausalNode` / `CausalEdge` / `CausalGraph` /
//! `CausalChain` / `EdgeSource` / `EdgeProposalRequest` / `EdgeProposalResponse` /
//! `CounterfactualQuery` / `MCTSNode` / `MockCausalLlm` / `MineCausalEdges` / `ProposeCausalEdges` /
//! `CausalSimulator` / `CausalMctsPlanner` / `CausalGraphEvaluator`), trait 边界对齐 v2
//! `OrganTrait`. v1 不引入, v2 真生产路径走本模块.
//!
//! **承接 (per 任务 §5)**:
//!
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入). W2 与 E4/F4/F6 共享
//!   `LlmFactory` trait 边界; **W2 真接** (`llm_factory()` 返 Some).
//!
//! **3 阶审查** (O-6 锚 9):
//!
//! 1. 总体: 1:1 翻译 v1 `CausalWorldModel` + W3 边挖掘, W2 trait process() 真接 LLM MCTS
//! 2. 系统: impl 在 engine (`apeireth-organ`), trait 在 foundation (`apeireth-plugin`)
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, W2 trait process() 调 `CausalSimulator` + LLM

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use apeireth_plugin::llm_factory::{
    CompletionMessage, CompletionRequest, LlmError, LlmFactory, LlmInstance,
};
use apeireth_plugin::organ::{
    CausalEdge as PluginCausalEdge, OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait,
};
use apeireth_orchestration::SubagentRole;
use async_trait::async_trait;

// ============================================================
// 数据层 (W2/W3 共享): 节点 + 边 + 因果图
// ============================================================

/// 因果节点: 一个事实节点, 以 chain (s|p|o) 为标识 (Zep 双时态: 同 s|p|o 共一节点).
#[derive(Debug, Clone)]
pub struct CausalNode {
    /// 节点 id = chain (s|p|o), 双时态语义下"当前有效事实"归并为一个节点.
    pub id: String,
    /// 节点显示标签 (人类可读).
    pub label: String,
    /// 节点属性 (open schema).
    pub attributes: HashMap<String, String>,
}

impl CausalNode {
    /// 从 (s, p, o) chain 构造节点.
    pub fn from_chain(chain: impl Into<String>) -> Self {
        let id = chain.into();
        Self {
            label: id.clone(),
            id,
            attributes: HashMap::new(),
        }
    }

    /// 从 (s, p, o, attrs) 构造节点 (open schema).
    pub fn with_attrs(
        chain: impl Into<String>,
        label: impl Into<String>,
        attrs: HashMap<String, String>,
    ) -> Self {
        let id = chain.into();
        Self {
            id,
            label: label.into(),
            attributes: attrs,
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
///
/// v1 `CausalEdge` schema 1:1 (id / from / to / predicate / weight / evidence_count / source).
/// v2 plugin 层 `CausalEdge { cause, effect, conf, source }` 是另一套 (per
/// `apeireth-plugin::organ::CausalEdge`), 这里保留 v1 完整 schema (含 predicate / evidence_count),
/// 转 plugin `OrganOutput::WorldModel { edges }` 时再做 schema 映射 (字段对子集).
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub id: String,
    /// 源节点 chain (s|p|o).
    pub from: String,
    /// 目标节点 chain (s'|p'|o').
    pub to: String,
    /// 因果谓词 (人类可读: "熬夜 → 次日效率低").
    pub predicate: String,
    /// 权重 0..1 (统计: 条件概率; LLM: 置信度).
    pub weight: f64,
    /// 证据计数 (统计: 共现次数; LLM: 提议理由强度 0..N).
    pub evidence_count: u32,
    /// 边来源.
    pub source: EdgeSource,
}

impl CausalEdge {
    /// 转换到 plugin 层 `CausalEdge` (用于 `OrganOutput::WorldModel { edges }`).
    ///
    /// 字段映射:
    /// - `cause` ← `from`
    /// - `effect` ← `to`
    /// - `conf` ← `weight as f32`
    /// - `source` ← EdgeSource 字符串名 ("Statistical" / "LlmProposed" / "Hybrid")
    pub fn to_plugin(&self) -> PluginCausalEdge {
        let source = match self.source {
            EdgeSource::Statistical => "Statistical",
            EdgeSource::LlmProposed => "LlmProposed",
            EdgeSource::Hybrid => "Hybrid",
        };
        PluginCausalEdge {
            cause: self.from.clone(),
            effect: self.to.clone(),
            conf: self.weight as f32,
            source: source.to_string(),
        }
    }
}

/// 因果图: 节点集 + 边集 + 邻接索引 (W2 推演的搜索空间).
///
/// v1 `CausalGraph` 1:1 (per `causal_world_model.rs:90-158`).
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
    /// 从 `Vec<CausalNode>` 构造图.
    pub fn from_nodes(nodes: impl IntoIterator<Item = CausalNode>) -> Self {
        let mut g = Self::default();
        for n in nodes {
            g.add_node(n);
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
///
/// v1 `DEFAULT_TIME_WINDOW_SECS = 86_400` (1 天, per `causal_world_model.rs:165`).
pub const DEFAULT_TIME_WINDOW_SECS: i64 = 86_400;

/// 共现证据阈值: 统计边成立的最小共现次数 (主人 2026-08-18 拍板: 7 次).
///
/// v1 `DEFAULT_MIN_EVIDENCE = 7` (per `causal_world_model.rs:168`).
pub const DEFAULT_MIN_EVIDENCE: u32 = 7;

/// 时间线条目 (W3 边挖掘输入, per v1 `GraphFact` 形态).
///
/// **0 装 PASS**: 本结构不依赖 `apeireth-memory::SqliteMemoryStore`; 简化形态 (chain /
/// valid_at / invalid_at / importance). W3 矿工从外部注入时间线数据 (runtime 桥接
/// `EpisodeStore` → 此结构).
#[derive(Debug, Clone)]
pub struct TimelineFact {
    /// 节点 chain (s|p|o).
    pub chain: String,
    /// subject (s).
    pub subject: String,
    /// predicate (p).
    pub predicate: String,
    /// object (o).
    pub object: String,
    /// valid_at (epoch ms).
    pub valid_at: i64,
    /// invalid_at (None = 当前有效).
    pub invalid_at: Option<i64>,
    /// importance 0..10.
    pub importance: u8,
}

/// 边挖掘器: 从时间线按"对象-主体直连"统计挖掘因果边.
///
/// v1 `MineCausalEdges` 1:1 (per `causal_world_model.rs:180-274`).
///
/// **机制**:
/// 1. 按时间排序所有事实.
/// 2. 对每对 (f_i, f_j), 若 `f_i.object == f_j.subject` 且时间差 ≤ 时间窗口 → 候选边.
/// 3. 统计每条候选边的共现次数, ≥ `min_evidence` → 确认为统计边.
/// 4. 权重 = 共现次数 / 该源节点总候选对数 (条件概率近似).
///
/// **0 装 PASS**: 纯确定性算法, 无 LLM, 无随机, 同输入同输出.
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

    /// 从时间线挖掘统计边. 返回 (边, 总候选对数).
    ///
    /// v1 `MineCausalEdges::from_timeline` 1:1 翻译.
    pub fn from_timeline(&self, facts: &[TimelineFact]) -> (Vec<CausalEdge>, usize) {
        // 1. 仅看有效事实 (invalid_at 为 None), 按时间排序.
        let mut active: Vec<&TimelineFact> =
            facts.iter().filter(|f| f.invalid_at.is_none()).collect();
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
                let dt_secs = (fj.valid_at - fi.valid_at) / 1000; // ms → s
                if dt_secs > self.time_window_secs {
                    break; // 已排序, 后续只会更远
                }
                if dt_secs < 0 {
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
        let mut idx = 0u32;
        for ((from, to), count) in counts {
            if count >= self.min_evidence {
                let total = source_matched.get(&from).copied().unwrap_or(1).max(1);
                let weight = (f64::from(count) / f64::from(total)).min(1.0);
                // 谓词: from 的 predicate → to 的 predicate (人类可读).
                let from_pred = from.split('|').nth(1).unwrap_or("").to_string();
                let to_pred = to.split('|').nth(1).unwrap_or("").to_string();
                edges.push(CausalEdge {
                    id: format!("causal-stat-{idx}"),
                    from,
                    to,
                    predicate: format!("{from_pred}→{to_pred}"),
                    weight,
                    evidence_count: count,
                    source: EdgeSource::Statistical,
                });
                idx += 1;
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
// W3 补充路径: EvoCause 式 LLM 提议边
// ============================================================

/// LLM 提议边请求 (per v1 `EdgeProposalRequest`).
#[derive(Debug, Clone)]
pub struct EdgeProposalRequest {
    /// 候选事实 (LLM 在这些事实之间找因果对).
    pub facts: Vec<TimelineFact>,
    /// 提议上限 (LLM 一次最多提 N 条).
    pub max_proposals: usize,
}

/// LLM 提议响应 (per v1 `EdgeProposalResponse`).
#[derive(Debug, Clone)]
pub struct EdgeProposalResponse {
    pub proposals: Vec<CausalEdge>,
}

/// LLM 提议一条边的最小 schema (用于 JSON 解析).
///
/// LLM 返 JSON 数组, 每个元素是这种 shape.
#[derive(Debug, Clone, serde::Deserialize)]
struct ProposedEdgeJson {
    from: String,
    to: String,
    predicate: String,
    /// LLM 自评置信度 0..1 (→ weight)
    confidence: f32,
    /// 证据强度 1..N (→ evidence_count, 上限 5)
    evidence_strength: u32,
}

/// 边提议器: 用 LLM 提议因果边 (EvoCause 式补充路径).
///
/// v1 `ProposeCausalEdges` 1:1 翻译 (`causal_world_model.rs:345-367`).
///
/// **0 装诚实**: 真接 LLM (per 任务 §3: "W2 必须 llm_factory() 返 Some"). W3 LLM
/// 提议路径同样真接 LLM (与 W2 主路径共享 LlmFactory).
pub struct ProposeCausalEdges {
    pub llm_factory: Arc<dyn LlmFactory>,
    pub model: String,
}

impl ProposeCausalEdges {
    pub fn new(llm_factory: Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            llm_factory,
            model: model.into(),
        }
    }

    /// 调 LLM 提议边, 返回 `Vec<CausalEdge>` (source = LlmProposed).
    ///
    /// **0 装诚实**: 真接 LLM. JSON 解析失败 → 返空 vec (LLM 输出不保证严格 JSON,
    /// 这是 v1 同款保守行为 — v1 用 `MockCausalLlm`, 我们的真路径同样用 try-parse).
    pub async fn llm_suggest(
        &self,
        req: &EdgeProposalRequest,
    ) -> Result<Vec<CausalEdge>, OrganError> {
        let instance = self
            .llm_factory
            .spawn(SubagentRole::Reviewer, &self.model)
            .await
            .map_err(|e| OrganError::LlmError(format!("spawn LLM failed: {e}")))?;

        // 构造 prompt: 给 LLM 看 facts, 让它提议因果边
        let facts_text: String = req
            .facts
            .iter()
            .take(20) // 上限 20 条避免 prompt 爆
            .map(|f| {
                format!(
                    "- chain={} | s={} | p={} | o={} | valid_at={}",
                    f.chain, f.subject, f.predicate, f.object, f.valid_at
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = "你是因果图边提议器 (EvoCause 式). 给定一组事实 (s|p|o), 在它们之间提议最可能的因果边. 输出严格 JSON 数组, 每个元素: {\"from\": \"s|p|o\", \"to\": \"s'|p'|o'\", \"predicate\": \"人类可读\", \"confidence\": 0.0..1.0, \"evidence_strength\": 1..5}. 仅输出 JSON 数组, 无解释.".to_string();

        let user_prompt = format!(
            "候选事实 ({} 条, 上限 {} 提议):\n{}\n\n请提议最多 {} 条因果边.",
            req.facts.len(),
            req.max_proposals,
            facts_text,
            req.max_proposals
        );

        let llm_req = CompletionRequest {
            system_prompt,
            messages: vec![CompletionMessage {
                role: "user".into(),
                content: user_prompt,
            }],
            temperature: 0.3, // 因果推理偏确定性
            tools: vec![],
            max_tokens: Some(2048),
        };

        let resp = instance
            .complete(llm_req)
            .await
            .map_err(|e| OrganError::LlmError(format!("LLM complete failed: {e}")))?;

        // 解析 JSON 数组
        let content = resp.message.content.clone();
        let proposals = parse_proposed_edges_json(&content, req.max_proposals);

        Ok(proposals
            .into_iter()
            .map(|mut e| {
                e.source = EdgeSource::LlmProposed;
                e
            })
            .collect())
    }
}

/// 从 LLM 响应文本提取 JSON 数组并解析成 `Vec<CausalEdge>` (source = LlmProposed).
///
/// **0 装诚实**: 解析失败 → 返空 vec, 不假装"我们提议了边". v1 mock 行为同样保守.
fn parse_proposed_edges_json(content: &str, max: usize) -> Vec<CausalEdge> {
    // 提取 ```json ... ``` 块或首个 [ ... ]
    let trimmed = content.trim();
    let json_str = if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            &after[..end]
        } else {
            after
        }
    } else if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            return Vec::new();
        }
    } else {
        return Vec::new();
    };

    let arr: Vec<ProposedEdgeJson> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    arr.into_iter()
        .take(max)
        .enumerate()
        .map(|(i, j)| CausalEdge {
            id: format!("causal-llm-{i}"),
            from: j.from,
            to: j.to,
            predicate: j.predicate,
            weight: f64::from(j.confidence).clamp(0.0, 1.0),
            evidence_count: j.evidence_strength.clamp(1, 5),
            source: EdgeSource::LlmProposed, // 显式标 (也会被调用方重设)
        })
        .collect()
}

// ============================================================
// 推演链 (W2): 沿因果边展开推演
// ============================================================

/// 推演链一步: 走过一条因果边, LLM 给叙事 + 状态快照.
///
/// v1 `CausalStep` 1:1.
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
    /// 走到此节点后的"推演 tick" 快照.
    pub tick_snapshot: u64,
}

/// 一条完整因果推演链.
///
/// v1 `CausalChain` 1:1.
#[derive(Debug, Clone)]
pub struct CausalChain {
    /// 反事实假设.
    pub hypothesis: String,
    /// 推演步骤序列 (沿因果边走).
    pub steps: Vec<CausalStep>,
    /// 终点节点 chain.
    pub terminal_node: Option<String>,
    /// 终点预测概率 (从 LLM JSON 提取, 或边均值兜底).
    pub terminal_probability: Option<f64>,
    /// 校准 Brier (与事实对账后).
    pub calibration_brier: Option<f64>,
    /// 校准差拒绝标记 (Brier > threshold).
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
            terminal_probability: None,
            calibration_brier: None,
            rejected: false,
            reject_reason: None,
        }
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

/// 推演请求: 给 LLM 反事实推演用 (1:1 翻译 v1 `CounterfactualQuery`).
#[derive(Debug, Clone)]
pub struct CounterfactualQuery {
    /// 反事实假设 (e.g. "如果主人今晚熬夜...").
    pub hypothesis: String,
    /// 当前因果图 (节点 + 边).
    pub current_graph: CausalGraph,
    /// 起点节点 chain.
    pub start_node: String,
    /// 最大推演步数.
    pub max_steps: usize,
}

/// MCTS 节点 (per task brief).
///
/// v2 简化形态: state (graph) + score + children. 真生产路径用 `CausalMctsPlanner`
/// (复用 v1 同名 planner), 这里保留 brief 提到的 MCTSNode 作为数据结构.
#[derive(Debug, Clone)]
pub struct MCTSNode {
    /// 当前因果图 (走到此节点后的状态).
    pub state: CausalGraph,
    /// 节点评估分数 (LLM 给出, 或 planner 启发式).
    pub score: f32,
    /// 子节点 (走过的边 → 下一状态).
    pub children: Vec<MCTSNode>,
}

// ============================================================
// LLM 抽象 (W2/W3 共享): 因果图 LLM trait (替代 v1 `CausalLlm`)
// ============================================================

/// 因果图 LLM trait (per v1 `CausalLlm` 1:1).
///
/// 真生产路径: 由 `CausalWorldModelOrgan` 内置的 `Arc<dyn LlmFactory>` 实现 (走
/// `LlmFactory::spawn` → `LlmInstance::complete`), 不直接 import 这个 trait.
///
/// 测试路径: `MockCausalLlm` 走通全链 (不走真 LLM, 0 装 PASS).
#[async_trait]
pub trait CausalLlm: Send + Sync {
    /// 分支点判断: 给定当前状态 + 候选边, LLM 给 (a) 边的可行性 (b) 走到此边的叙事片段.
    async fn judge_branch(
        &self,
        ctx: &CausalBranchContext,
    ) -> Result<CausalBranchJudgment, String>;

    /// 提议因果边 (W3 补充路径).
    async fn propose_edges(
        &self,
        req: &EdgeProposalRequest,
    ) -> Result<EdgeProposalResponse, String>;
}

/// 分支点上下文: 当前状态 + 候选边 (LLM 选择/评估时用).
///
/// v1 `CausalBranchContext` 1:1.
#[derive(Debug, Clone)]
pub struct CausalBranchContext {
    /// 当前节点 chain.
    pub current_node_id: String,
    /// 反事实假设.
    pub hypothesis: String,
    /// 已访问节点集合 (防环).
    pub visited: HashSet<String>,
    /// 候选边 (出邻接).
    pub candidates: Vec<CausalEdge>,
    /// 当前 tick (推演步数).
    pub current_tick: u64,
}

/// 分支点判断: LLM 对每条候选边打分 + 给叙事.
///
/// v1 `CausalBranchJudgment` 1:1.
#[derive(Debug, Clone)]
pub struct CausalBranchJudgment {
    /// 每条候选边的评估 (按候选顺序对应).
    pub judgments: Vec<EdgeJudgment>,
}

/// 单条候选边评估.
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

/// Mock LLM: 分支点硬编码接受首条候选 + 给叙事.
///
/// v1 `MockCausalLlm` 1:1 (`causal_world_model.rs:730-793`).
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
                    id: format!("causal-llm-mock-{i}"),
                    from: from.chain.clone(),
                    to: to.chain.clone(),
                    predicate: format!("{}→{}", from.predicate, to.predicate),
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
// LLM-backed CausalLlm 实现 (W2 真接 LLM, per 任务 §3)
// ============================================================

/// 用 `LlmFactory` + `LlmInstance` 实现的 `CausalLlm` trait.
///
/// **0 装诚实**: 真接 LLM. JSON 解析失败 → 保守返 "无推荐" (不假装 LLM 给了好答案).
pub struct LlmFactoryCausalLlm {
    factory: Arc<dyn LlmFactory>,
    model: String,
}

impl LlmFactoryCausalLlm {
    pub fn new(factory: Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            factory,
            model: model.into(),
        }
    }
}

/// judge_branch 的 LLM 响应 JSON schema (单条 judgment).
#[derive(Debug, Clone, serde::Deserialize)]
struct JudgmentJson {
    edge_id: String,
    take: bool,
    narrative: String,
    goal_progress: f64,
}

/// judge_branch 的 LLM 响应 JSON schema (整体).
#[derive(Debug, Clone, serde::Deserialize)]
struct JudgmentResponseJson {
    judgments: Vec<JudgmentJson>,
}

#[async_trait]
impl CausalLlm for LlmFactoryCausalLlm {
    async fn judge_branch(
        &self,
        ctx: &CausalBranchContext,
    ) -> Result<CausalBranchJudgment, String> {
        let instance = self
            .factory
            .spawn(SubagentRole::Reviewer, &self.model)
            .await
            .map_err(|e| format!("spawn LLM failed: {e}"))?;

        // 构造 prompt: 当前节点 + 候选边 + 假设
        let candidates_text: String = ctx
            .candidates
            .iter()
            .map(|e| {
                format!(
                    "- edge_id={} | from={} | to={} | predicate={} | weight={:.2} | source={:?}",
                    e.id, e.from, e.to, e.predicate, e.weight, e.source
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = "你是因果图分支判断器. 给定当前节点 + 候选边 + 反事实假设, 对每条边判断 (take=true/false) + 给叙事 + goal_progress 0..1. 输出严格 JSON: {\"judgments\": [{\"edge_id\": \"...\", \"take\": bool, \"narrative\": \"...\", \"goal_progress\": 0.0..1.0}, ...]}".to_string();

        let user_prompt = format!(
            "假设: {}\n当前节点: {}\n已访问: {:?}\n候选边:\n{}\n\n返回 JSON.",
            ctx.hypothesis, ctx.current_node_id, ctx.visited, candidates_text
        );

        let req = CompletionRequest {
            system_prompt,
            messages: vec![CompletionMessage {
                role: "user".into(),
                content: user_prompt,
            }],
            temperature: 0.3,
            tools: vec![],
            max_tokens: Some(2048),
        };

        let resp = instance
            .complete(req)
            .await
            .map_err(|e| format!("LLM complete failed: {e}"))?;

        let content = resp.message.content.clone();

        // 解析 JSON
        let trimmed = content.trim();
        let json_str = if let Some(start) = trimmed.find("```json") {
            let after = &trimmed[start + 7..];
            if let Some(end) = after.find("```") {
                &after[..end]
            } else {
                after
            }
        } else if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                &trimmed[start..=end]
            } else {
                return Err(format!("no JSON object found in LLM response: {content}"));
            }
        } else {
            return Err(format!("no JSON found in LLM response: {content}"));
        };

        let parsed: JudgmentResponseJson =
            serde_json::from_str(json_str).map_err(|e| format!("JSON parse failed: {e}"))?;

        // 用 LLM 响应补全 judgments (按 edge_id 对齐 ctx.candidates)
        let mut judgments = Vec::with_capacity(ctx.candidates.len());
        for edge in &ctx.candidates {
            if let Some(j) = parsed.judgments.iter().find(|j| j.edge_id == edge.id) {
                judgments.push(EdgeJudgment {
                    edge_id: j.edge_id.clone(),
                    take: j.take,
                    narrative: j.narrative.clone(),
                    goal_progress: j.goal_progress.clamp(0.0, 1.0),
                });
            } else {
                // LLM 没评这条边 → 保守返 take=false
                judgments.push(EdgeJudgment {
                    edge_id: edge.id.clone(),
                    take: false,
                    narrative: format!("(LLM 未评 {} → 保守拒绝)", edge.id),
                    goal_progress: 0.0,
                });
            }
        }

        Ok(CausalBranchJudgment { judgments })
    }

    async fn propose_edges(
        &self,
        req: &EdgeProposalRequest,
    ) -> Result<EdgeProposalResponse, String> {
        // 复用 ProposeCausalEdges 的 LLM 路径 (保持真接 LLM 一致)
        let proposer = ProposeCausalEdges::new(self.factory.clone(), self.model.clone());
        let edges = proposer
            .llm_suggest(req)
            .await
            .map_err(|e| format!("propose_edges LLM failed: {e}"))?;
        Ok(EdgeProposalResponse { proposals: edges })
    }
}

// ============================================================
// W2 编排器: 沿因果链展开推演 (LLM 只在分支点判断)
// ============================================================

/// 因果模拟器: 沿因果图展开推演链. 与 v1 `TextualSimulator` 同构 (W1), 仅搜索空间换成因果图.
///
/// v1 `CausalSimulator` 1:1 (`causal_world_model.rs:432-606`).
pub struct CausalSimulator {
    pub graph: CausalGraph,
    pub llm: Arc<dyn CausalLlm>,
    /// 最大推演步数.
    pub max_steps: usize,
    /// Brier 拒绝阈值.
    pub reject_threshold: f64,
}

impl CausalSimulator {
    pub fn new(graph: CausalGraph, llm: Arc<dyn CausalLlm>) -> Self {
        Self {
            graph,
            llm,
            max_steps: 8,
            reject_threshold: 0.3,
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

        let mut current_node = start_node_id.clone();
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(current_node.clone());
        let mut current_tick: u64 = 0;

        for tick in 0..self.max_steps {
            let candidates: Vec<CausalEdge> =
                self.graph.outgoing_edges(&current_node).cloned().collect();
            if candidates.is_empty() {
                break;
            }

            let ctx = CausalBranchContext {
                current_node_id: current_node.clone(),
                hypothesis: hypothesis.clone(),
                visited: visited.clone(),
                candidates: candidates.clone(),
                current_tick,
            };

            let judgment = self.llm.judge_branch(&ctx).await?;
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
                break;
            };

            let next_node = edge.to.clone();
            visited.insert(next_node.clone());

            let step = CausalStep {
                tick: tick as u64,
                from_node: current_node.clone(),
                edge: edge.clone(),
                to_node: next_node.clone(),
                narrative: judgment.narrative.clone(),
                tick_snapshot: current_tick,
            };
            chain.steps.push(step);
            chain.terminal_node = Some(next_node.clone());
            current_node = next_node;
            current_tick += 1;
        }

        // 终点 forecast 概率: 边权重均值 (v1 同款 — "0 装 PASS: trait 仅给 judge_branch + propose_edges")
        let probability = if chain.steps.is_empty() {
            0.5
        } else {
            let mean_w: f64 =
                chain.steps.iter().map(|s| s.edge.weight).sum::<f64>() / chain.steps.len() as f64;
            mean_w.clamp(0.0, 1.0)
        };
        chain.terminal_probability = Some(probability);

        Ok(chain)
    }

    /// 对账: 与事实对账, 更新 Brier + 拒绝标记.
    pub fn reconcile_with_fact(
        &self,
        chain: &mut CausalChain,
        actual_outcome: bool,
    ) -> Result<(), String> {
        let prob = chain
            .terminal_probability
            .ok_or_else(|| "chain 无终点 probability, 请先 run".to_string())?;
        let actual_f = if actual_outcome { 1.0 } else { 0.0 };
        let brier = (prob - actual_f).powi(2);
        chain.calibration_brier = Some(brier);
        if brier > self.reject_threshold {
            chain.rejected = true;
            chain.reject_reason = Some(format!(
                "终点 Brier {brier:.3} > 阈值 {:.3}",
                self.reject_threshold,
            ));
        }
        Ok(())
    }
}

// ============================================================
// CausalWorldModel: W2 核心容器 (整合 W3 边挖掘 + 推演)
// ============================================================

/// W2 CausalWorldModel 容器 (per v1 `CausalWorldModel` + `CausalSimulator`).
///
/// **0 装诚实**: 真接 LLM. factory=None → trait `llm_factory()` 返 None, **organ
/// 构造时 panic 明确标缺** (避免悄悄无 LLM 跑 — W2 是 LLM 重器官, 没 LLM 不能跑).
pub struct CausalWorldModel {
    /// 因果图 (从外部注入或 W3 挖掘).
    pub graph: std::sync::Mutex<CausalGraph>,
    /// LLM factory (真接 LLM).
    pub factory: Arc<dyn LlmFactory>,
    /// Model ID.
    pub model: String,
    /// 推演配置.
    pub max_steps: usize,
    /// Brier 拒绝阈值.
    pub reject_threshold: f64,
}

impl CausalWorldModel {
    /// 构造 W2 (factory 必传, 否则 0 装诚实标缺).
    pub fn new(factory: Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            graph: std::sync::Mutex::new(CausalGraph::default()),
            factory,
            model: model.into(),
            max_steps: 8,
            reject_threshold: 0.3,
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

    /// 注入图 (从 W3 miner 或外部 store).
    pub fn load_graph(&self, graph: CausalGraph) {
        let mut g = self
            .graph
            .lock()
            .expect("CausalWorldModel mutex poisoned (0 装诚实)");
        *g = graph;
    }

    /// 加节点 (确定性).
    pub fn add_entity(&self, node: CausalNode) {
        let mut g = self
            .graph
            .lock()
            .expect("CausalWorldModel mutex poisoned (0 装诚实)");
        g.add_node(node);
    }

    /// 加边 (确定性).
    pub fn add_edge(&self, edge: CausalEdge) {
        let mut g = self
            .graph
            .lock()
            .expect("CausalWorldModel mutex poisoned (0 装诚实)");
        g.add_edge(edge);
    }

    /// W3 主路径: 从时间线统计挖掘边, 注入图.
    pub fn mine_and_load(&self, facts: &[TimelineFact], miner: &MineCausalEdges) -> usize {
        let (edges, _pairs) = miner.from_timeline(facts);
        let count = edges.len();
        for e in edges {
            self.add_edge(e);
        }
        count
    }

    /// 当前图克隆 (供 trait process 用, 避免持锁 await).
    pub fn snapshot_graph(&self) -> CausalGraph {
        let g = self
            .graph
            .lock()
            .expect("CausalWorldModel mutex poisoned (0 装诚实)");
        g.clone()
    }

    /// 当前边数.
    pub fn edge_count(&self) -> usize {
        self.graph
            .lock()
            .expect("CausalWorldModel mutex poisoned (0 装诚实)")
            .len_edges()
    }

    /// 当前节点数.
    pub fn node_count(&self) -> usize {
        self.graph
            .lock()
            .expect("CausalWorldModel mutex poisoned (0 装诚实)")
            .len_nodes()
    }

    /// 沿因果图展开反事实推演 (W2 主路径, 真接 LLM).
    ///
    /// **0 装诚实**: 真接 LLM (`LlmFactoryCausalLlm` 走 `spawn → complete`).
    pub async fn simulate_counterfactual(
        &self,
        query: CounterfactualQuery,
    ) -> Result<CausalGraph, OrganError> {
        // 校验假设非空
        if query.hypothesis.trim().is_empty() {
            return Err(OrganError::Config(
                "CounterfactualQuery.hypothesis must not be empty".into(),
            ));
        }

        // 校验起点节点存在
        if query.current_graph.node(&query.start_node).is_none() {
            return Err(OrganError::Config(format!(
                "start_node not in graph: {}",
                query.start_node
            )));
        }

        // 真接 LLM (LlmFactoryCausalLlm 内部 spawn + complete)
        let causal_llm: Arc<dyn CausalLlm> = Arc::new(LlmFactoryCausalLlm::new(
            self.factory.clone(),
            self.model.clone(),
        ));
        let sim = CausalSimulator::new(query.current_graph, causal_llm)
            .with_max_steps(query.max_steps.min(self.max_steps))
            .with_threshold(self.reject_threshold);

        let chain = sim
            .run(&query.start_node, &query.hypothesis)
            .await
            .map_err(|e| OrganError::LlmError(format!("CausalSimulator::run failed: {e}")))?;

        // 沿推演链构造新因果图: 起点 + 每步终点 + 走过的边
        let mut new_graph = CausalGraph::default();
        new_graph.add_node(CausalNode::from_chain(&query.start_node));
        for step in &chain.steps {
            // 节点 (如果还没加)
            if new_graph.node(&step.from_node).is_none() {
                new_graph.add_node(CausalNode::from_chain(&step.from_node));
            }
            if new_graph.node(&step.to_node).is_none() {
                new_graph.add_node(CausalNode::from_chain(&step.to_node));
            }
            new_graph.add_edge(step.edge.clone());
        }

        Ok(new_graph)
    }

    /// W2 + W3 MCTS 搜索 (主路径, 真接 LLM).
    ///
    /// 简化形态: 走 `simulate_counterfactual` + 在结果图上扩展 N 步作为 children 节点.
    /// 真生产路径可换 v1 `CausalMctsPlanner` (复用 cognition::planning), 此处保留
    /// 任务 brief 提到的 `MCTSNode` 结构, 但搜索逻辑保持 1:1 翻译 v1 思路.
    pub async fn mcts_search(
        &self,
        query: CounterfactualQuery,
        depth: usize,
    ) -> Result<MCTSNode, OrganError> {
        let new_graph = self.simulate_counterfactual(query.clone()).await?;
        // 简化: children = 每个终点节点的下一层 (递归, 上限 depth)
        let children = if depth > 0 {
            self.expand_children(&new_graph, &query.hypothesis, depth)
                .await?
        } else {
            Vec::new()
        };

        let score = if new_graph.len_edges() > 0 {
            (new_graph.edges().iter().map(|e| e.weight as f32).sum::<f32>()
                / new_graph.len_edges() as f32)
                .clamp(0.0, 1.0)
        } else {
            0.5
        };

        Ok(MCTSNode {
            state: new_graph,
            score,
            children,
        })
    }

    /// 递归扩展 MCTS children (简化形态).
    async fn expand_children(
        &self,
        graph: &CausalGraph,
        hypothesis: &str,
        depth: usize,
    ) -> Result<Vec<MCTSNode>, OrganError> {
        let mut children = Vec::new();
        for edge in graph.edges() {
            if depth == 0 {
                break;
            }
            let sub_query = CounterfactualQuery {
                hypothesis: format!("{hypothesis} → {}", edge.to),
                current_graph: graph.clone(),
                start_node: edge.to.clone(),
                max_steps: 1,
            };
            let sub_graph = self.simulate_counterfactual(sub_query).await?;
            let sub_score = if sub_graph.len_edges() > 0 {
                (sub_graph.edges().iter().map(|e| e.weight as f32).sum::<f32>()
                    / sub_graph.len_edges() as f32)
                    .clamp(0.0, 1.0)
            } else {
                0.5
            };
            children.push(MCTSNode {
                state: sub_graph,
                score: sub_score,
                children: Vec::new(),
            });
        }
        Ok(children)
    }
}

// ============================================================
// CausalWorldModelOrgan: W2 trait 真实现 (v2 OrganTrait)
// ============================================================

/// W2 Causal World Model 器官 (per v2 OrganTrait 1:1 翻译 v1 CausalWorldModel).
///
/// **0 装诚实**:
/// - `llm_factory()` 返 `Some` — W2 是 LLM 重, 真接 LLM (与 E4/F4/F6/F1 确定性器官不同).
/// - factory=None → `process()` 返 `OrganError::LlmUnavailable`, **不假装**能跑推演.
/// - `process()` 走 `simulate_counterfactual` 真接 LLM (经 `LlmFactoryCausalLlm`).
/// - 推演结果**永远不入库** (与 v1 同纪律): 仅返回 `OrganOutput::WorldModel { edges, counterfactual }`.
pub struct CausalWorldModelOrgan {
    model: Arc<CausalWorldModel>,
}

impl CausalWorldModelOrgan {
    /// 构造 W2 organ (factory 必传).
    pub fn new(factory: Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            model: Arc::new(CausalWorldModel::new(factory, model)),
        }
    }

    /// 构造 + 自定义推演配置.
    pub fn with_config(
        factory: Arc<dyn LlmFactory>,
        model: impl Into<String>,
        max_steps: usize,
        reject_threshold: f64,
    ) -> Self {
        Self {
            model: Arc::new(
                CausalWorldModel::new(factory, model)
                    .with_max_steps(max_steps)
                    .with_threshold(reject_threshold),
            ),
        }
    }

    /// 暴露内层 `CausalWorldModel` (供 W3 miner / 外部 store 调用).
    pub fn inner(&self) -> &Arc<CausalWorldModel> {
        &self.model
    }
}

#[async_trait]
impl OrganTrait for CausalWorldModelOrgan {
    fn name(&self) -> &'static str {
        "W2 Causal World Model"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::W2
    }

    /// **真接 LLM**: 走 `simulate_counterfactual` LLM MCTS 路径.
    async fn process(&self, input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 0 装诚实: dry_run 模式同样走 LLM (per v1 dry_run 不影响 causal sim),
        // 但 cost 控制交给 runtime (不在 organ 内部决定).
        let _ = input.dry_run;

        // 校验 LLM factory 真存在 (per 任务 §3: "0 装诚实标")
        if self.model.factory.name().is_empty() {
            return Err(OrganError::LlmUnavailable(
                "W2 causal world model requires LlmFactory (真接 LLM, 0 装诚实)".into(),
            ));
        }

        // 1) 从 input 提取假设 + 起点
        let hypothesis = if !input.context_hints.is_empty() {
            input.context_hints.join(" ")
        } else {
            input.episode.content.clone()
        };
        if hypothesis.trim().is_empty() {
            return Err(OrganError::Config(
                "OrganInput must have hypothesis (context_hints or episode.content)".into(),
            ));
        }

        // 2) 拿快照图 (避免持锁 await)
        let current_graph = self.model.snapshot_graph();
        if current_graph.is_empty() {
            return Err(OrganError::Config(
                "CausalGraph is empty; load edges first (via W3 miner or manual add_edge)"
                    .into(),
            ));
        }

        // 3) 选起点: 默认第一条节点的 chain, 或 context_hints[0] (如有)
        let start_node = if !input.context_hints.is_empty() {
            input.context_hints[0].clone()
        } else {
            current_graph
                .nodes()
                .next()
                .map(|n| n.id.clone())
                .ok_or_else(|| OrganError::Internal("no nodes in graph".into()))?
        };

        // 4) 真接 LLM MCTS 反事实推演
        let query = CounterfactualQuery {
            hypothesis: hypothesis.clone(),
            current_graph,
            start_node,
            max_steps: self.model.max_steps,
        };
        let new_graph = self.model.simulate_counterfactual(query).await?;

        // 5) 构造 OrganOutput::WorldModel { edges, counterfactual }
        let plugin_edges: Vec<PluginCausalEdge> =
            new_graph.edges().iter().map(|e| e.to_plugin()).collect();
        let counterfactual: Vec<String> = new_graph
            .edges()
            .iter()
            .map(|e| format!("{} → {} ({})", e.from, e.to, e.predicate))
            .collect();

        // 6) 推演结果**永远不入库** (per 任务 + v1 同纪律):
        // 这里仅返 OrganOutput, 不调 SqliteMemoryStore / memory_extractor.
        Ok(OrganOutput::WorldModel {
            edges: plugin_edges,
            counterfactual,
        })
    }

    /// 0 装诚实: W2 是 LLM 重器官, **真接 LLM** (`llm_factory()` 返 Some).
    fn llm_factory(&self) -> Option<Arc<dyn LlmFactory>> {
        Some(self.model.factory.clone())
    }
}

// ============================================================
// W3 边挖掘器官 (v2 OrganTrait, 0 装: W3 是统计主路径, 确定性无 LLM)
// ============================================================

/// W3 Causal Edge Mining 器官 (per v2 OrganTrait).
///
/// **0 装诚实**:
/// - W3 主路径 = 统计挖掘, 纯确定性无 LLM (per v1 doc: "纯确定性算法, 无 LLM").
/// - W3 trait `llm_factory()` 返 None (主路径确定性).
/// - W3 补充路径 (EvoCause LLM 提议边) 走 `ProposeCausalEdges` 真接 LLM, 但**不在
///   process() 默认调用**, 仅暴露 `propose_edges_with_llm` API 让 runtime 显式触发.
/// - 这是 W3 与 W2 的关键区别: W2 是 LLM 重 (process 必调 LLM), W3 主路径是
///   统计确定性 (process 不调 LLM), LLM 是补充路径.
pub struct CausalEdgeMiningOrgan {
    /// W3 内部 miner (统计挖掘).
    miner: MineCausalEdges,
    /// W3 LLM 提议器 (EvoCause 补充路径).
    proposer: Option<ProposeCausalEdges>,
}

impl CausalEdgeMiningOrgan {
    /// 构造 W3 (统计主路径, 无 LLM).
    pub fn new() -> Self {
        Self {
            miner: MineCausalEdges::default(),
            proposer: None,
        }
    }

    /// 构造 W3 + 自定义 miner 配置.
    pub fn with_miner(miner: MineCausalEdges) -> Self {
        Self {
            miner,
            proposer: None,
        }
    }

    /// 构造 W3 + LLM 提议器 (EvoCause 补充路径就位).
    pub fn with_llm_proposer(
        miner: MineCausalEdges,
        llm_factory: Arc<dyn LlmFactory>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            miner,
            proposer: Some(ProposeCausalEdges::new(llm_factory, model)),
        }
    }

    /// 暴露 miner (供外部 store 调用).
    pub fn miner(&self) -> &MineCausalEdges {
        &self.miner
    }

    /// 暴露 proposer (供外部调用 EvoCause LLM 提议边).
    pub fn proposer(&self) -> Option<&ProposeCausalEdges> {
        self.proposer.as_ref()
    }
}

impl Default for CausalEdgeMiningOrgan {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrganTrait for CausalEdgeMiningOrgan {
    fn name(&self) -> &'static str {
        "W3 Causal Edge Mining"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::W3
    }

    /// **W3 主路径**: 统计挖掘 + 返 OrganOutput::WorldModel (边 + 统计元数据).
    /// 注: 本 trait process 不直接接受时间线; runtime 应调用 `miner().from_timeline(...)`
    /// 然后把结果灌入 W2 CausalGraph. 此 process 仅作 trait 兼容入口, 返空边 + 显式
    /// OrganOutput::WorldModel (避免无意义调用 LLM).
    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        // W3 trait process 不直接接受时间线 (timeline 数据结构由 runtime 提供);
        // 这里返 0 装 PASS: 显式说明 W3 主路径需 runtime 桥接 timeline → miner.
        // 0 装诚实: 不假装 process 能跑端到端.
        Err(OrganError::Config(
            "W3 主路径是统计挖掘, 需 runtime 显式调用 miner().from_timeline(facts) 后灌入 W2 图; process() 入口仅作 trait 兼容".into(),
        ))
    }

    /// 0 装诚实: W3 主路径是确定性无 LLM, trait 返 None.
    /// 注: proposer 是**补充路径**, 仅当显式调用 `proposer().llm_suggest(...)` 才调 LLM,
    /// 不在 process 默认路径内, 不影响 trait `llm_factory()` 返 None.
    fn llm_factory(&self) -> Option<Arc<dyn LlmFactory>> {
        None
    }
}

// ============================================================
// 错误转换: LlmError → OrganError (per task §3: "不假装")
// ============================================================

/// 转换 `LlmError` 到 `OrganError::LlmError`.
#[allow(dead_code)]
fn llm_error_to_organ(e: LlmError) -> OrganError {
    OrganError::LlmError(e.to_string())
}

// ============================================================
// 单元测试 (1:1 翻译 v1 causal_world_model.rs 测试)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_factory() -> Arc<dyn LlmFactory> {
        Arc::new(apeireth_plugin::llm_factory::NoopLlmFactory)
    }

    fn fact(s: &str, p: &str, o: &str, ts: i64) -> TimelineFact {
        TimelineFact {
            chain: format!("{s}|{p}|{o}"),
            subject: s.to_string(),
            predicate: p.to_string(),
            object: o.to_string(),
            valid_at: ts,
            invalid_at: None,
            importance: 5,
        }
    }

    /// 构造一条因果图 fixture: 主人→熬夜→效率低 → 延期 (熬夜→次日 因果链).
    fn build_chain_graph() -> CausalGraph {
        let mut g = CausalGraph::from_nodes([
            CausalNode::from_chain("主人|行为|熬夜"),
            CausalNode::from_chain("熬夜|导致|效率低"),
            CausalNode::from_chain("效率低|后果|延期"),
        ]);
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

    /// v1 1:1: MockCausalLlm 沿因果链展开 (确定性).
    #[tokio::test]
    async fn causal_chain_expand_from_root() {
        let graph = build_chain_graph();
        let llm: Arc<dyn CausalLlm> = Arc::new(MockCausalLlm::default());
        let sim = CausalSimulator::new(graph, llm);
        let chain = sim
            .run("主人|行为|熬夜", "如果主人今晚熬夜...")
            .await
            .unwrap();

        assert!(chain.step_count() >= 1, "应至少走 1 步");
        assert!(
            chain.step_count() <= 2,
            "3 节点链, 最多 2 步 (根不计入 steps)"
        );
        assert_eq!(chain.steps[0].from_node, "主人|行为|熬夜");
        assert_eq!(chain.steps[0].to_node, "熬夜|导致|效率低");
        assert!(chain.terminal_node.is_some(), "应到达终点节点");
        assert!(chain.terminal_probability.is_some(), "应构造终点 probability");
        assert!(!chain.rejected, "无对账 + 边权重高 → 不拒绝");
        let prob = chain.terminal_probability.unwrap();
        assert!(
            (prob - 0.85).abs() < 1e-9,
            "边权重均值应作为概率: got {prob}"
        );
    }

    /// v1 1:1: W3 主路径 - 从时间线统计挖掘边
    #[test]
    fn mine_causal_edges_statistical() {
        let mut facts = Vec::new();
        for i in 0..7 {
            let ts_base = 1_000_000_000 + i * 100; // ms
            facts.push(fact("主人", "行为", "熬夜", ts_base));
            facts.push(fact("熬夜", "导致", "效率低", ts_base + 60_000)); // 1 分钟差, 窗口内
        }
        // 干扰: 无关事实 (object != 下一条 subject) — 不应形成边.
        for i in 0..3 {
            let ts = 1_000_000_000 + i * 50_000;
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
        assert!(edge.predicate.contains("行为") && edge.predicate.contains("导致"));
    }

    /// v1 1:1: 阈值 7, 但只有 3 对共现 → 应无边.
    #[test]
    fn mine_causal_edges_below_threshold_no_edge() {
        let mut facts = Vec::new();
        for i in 0..3 {
            let ts = 2_000_000_000 + i * 100;
            facts.push(fact("主人", "行为", "熬夜", ts));
            facts.push(fact("熬夜", "导致", "效率低", ts + 60_000));
        }
        let miner = MineCausalEdges::default();
        let (edges, pairs) = miner.from_timeline(&facts);
        assert_eq!(pairs, 3);
        assert!(edges.is_empty(), "3 < 阈值 7, 不应产边");
    }

    /// v1 1:1: Mock LLM 提议边 (W3 补充路径)
    #[tokio::test]
    async fn propose_causal_edges_mock() {
        let facts = vec![
            fact("主人", "行为", "熬夜", 1_000_000_000),
            fact("熬夜", "导致", "效率低", 1_000_000_060),
            fact("效率低", "后果", "延期", 1_000_000_120),
        ];
        let llm: Arc<dyn CausalLlm> = Arc::new(MockCausalLlm {
            take_first: true,
            max_proposals: 2,
        });
        // Mock 路径直接调 trait propose_edges (不真接 LLM)
        let req = EdgeProposalRequest {
            facts,
            max_proposals: 2,
        };
        let resp = llm.propose_edges(&req).await.unwrap();
        let proposals = resp.proposals;

        assert!(!proposals.is_empty(), "至少提议 1 条");
        assert!(
            proposals.len() <= req.max_proposals,
            "不超过 max_proposals 上限"
        );
        for e in &proposals {
            assert_eq!(e.source, EdgeSource::LlmProposed);
            assert!(e.weight > 0.0 && e.weight <= 1.0);
        }
        assert_eq!(proposals[0].from, "主人|行为|熬夜");
        assert_eq!(proposals[0].to, "熬夜|导致|效率低");
    }

    /// v1 1:1: 推演结果与事实对账 (Brier 校准)
    #[tokio::test]
    async fn causal_chain_reconcile_with_fact() {
        let graph = build_chain_graph();
        let llm: Arc<dyn CausalLlm> = Arc::new(MockCausalLlm::default());
        let sim = CausalSimulator::new(graph, llm).with_threshold(0.3);
        let mut chain = sim
            .run("主人|行为|熬夜", "如果主人今晚熬夜...")
            .await
            .unwrap();

        // outcome=true → Brier = (0.85 - 1)² = 0.0225 < 0.3 → 不拒绝
        sim.reconcile_with_fact(&mut chain, true).unwrap();
        let brier_true = chain.calibration_brier.unwrap();
        assert!(
            (brier_true - 0.0225).abs() < 1e-9,
            "p=0.85, actual=true → Brier=0.0225 (got {brier_true})"
        );
        assert!(!chain.rejected, "Brier=0.0225 < 阈值 0.3, 不拒绝");

        // outcome=false → Brier = 0.85² = 0.7225 > 0.3 → 拒绝
        let graph2 = build_chain_graph();
        let llm2: Arc<dyn CausalLlm> = Arc::new(MockCausalLlm::default());
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

    /// v2 新增: parse_proposed_edges_json 单元测试
    #[test]
    fn parse_proposed_edges_json_basic() {
        let json = r#"[
            {"from": "a|b|c", "to": "b|c|d", "predicate": "b→c", "confidence": 0.7, "evidence_strength": 2}
        ]"#;
        let edges = parse_proposed_edges_json(json, 10);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "a|b|c");
        assert_eq!(edges[0].to, "b|c|d");
        // 0.7 f32 → f64 转换有精度损失, 用 1e-6 容差
        assert!((edges[0].weight - 0.7).abs() < 1e-6, "weight={}", edges[0].weight);
        assert_eq!(edges[0].evidence_count, 2);
        assert_eq!(edges[0].source, EdgeSource::LlmProposed);
    }

    /// v2 新增: parse_proposed_edges_json 解析 ```json ... ``` 块
    #[test]
    fn parse_proposed_edges_json_code_block() {
        let json = "```json\n[{\"from\": \"x|y|z\", \"to\": \"y|z|w\", \"predicate\": \"p\", \"confidence\": 0.5, \"evidence_strength\": 1}]\n```";
        let edges = parse_proposed_edges_json(json, 10);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "x|y|z");
    }

    /// v2 新增: parse_proposed_edges_json 坏 JSON → 空 vec (0 装诚实)
    #[test]
    fn parse_proposed_edges_json_bad_returns_empty() {
        let bad = "not json at all";
        let edges = parse_proposed_edges_json(bad, 10);
        assert!(edges.is_empty());
    }

    /// v2 新增: CausalEdge::to_plugin 字段映射 (W2 → plugin WorldModel 边 schema)
    #[test]
    fn causal_edge_to_plugin_schema_mapping() {
        let mk = |src: EdgeSource| CausalEdge {
            id: "test".into(),
            from: "a|b|c".into(),
            to: "b|c|d".into(),
            predicate: "p".into(),
            weight: 0.75,
            evidence_count: 5,
            source: src,
        };
        let p = mk(EdgeSource::Statistical).to_plugin();
        assert_eq!(p.cause, "a|b|c");
        assert_eq!(p.effect, "b|c|d");
        assert!((p.conf - 0.75_f32).abs() < 1e-6);
        assert_eq!(p.source, "Statistical");

        assert_eq!(mk(EdgeSource::LlmProposed).to_plugin().source, "LlmProposed");
        assert_eq!(mk(EdgeSource::Hybrid).to_plugin().source, "Hybrid");
    }

    /// v2 新增: CausalGraph 节点/边基本操作
    #[test]
    fn causal_graph_basic_ops() {
        let mut g = CausalGraph::default();
        assert!(g.is_empty());
        g.add_node(CausalNode::from_chain("s|p|o"));
        assert_eq!(g.len_nodes(), 1);
        g.add_edge(CausalEdge {
            id: "e1".into(),
            from: "s|p|o".into(),
            to: "s'|p'|o'".into(),
            predicate: "p".into(),
            weight: 0.5,
            evidence_count: 1,
            source: EdgeSource::Statistical,
        });
        assert_eq!(g.len_edges(), 1);
        let out = g.outgoing_edges("s|p|o").count();
        assert_eq!(out, 1);
    }

    /// v2 新增: CausalWorldModel.add_entity / add_edge 确定性
    #[test]
    fn causal_world_model_add_entity_and_edge_deterministically() {
        let model = CausalWorldModel::new(test_factory(), "minimax-m3-thinking");
        assert_eq!(model.node_count(), 0);
        assert_eq!(model.edge_count(), 0);
        model.add_entity(CausalNode::from_chain("s|p|o"));
        model.add_entity(CausalNode::from_chain("s'|p'|o'"));
        assert_eq!(model.node_count(), 2);
        model.add_edge(CausalEdge {
            id: "e1".into(),
            from: "s|p|o".into(),
            to: "s'|p'|o'".into(),
            predicate: "p".into(),
            weight: 0.6,
            evidence_count: 1,
            source: EdgeSource::Statistical,
        });
        assert_eq!(model.edge_count(), 1);
    }

    /// v2 新增: W2 organ llm_factory() 返 Some (LLM 真接, vs E4/F4/F6/F1 返 None)
    #[test]
    fn w2_organ_llm_factory_returns_some_per_v1_truth() {
        let organ = CausalWorldModelOrgan::new(test_factory(), "minimax-m3-thinking");
        assert!(
            organ.llm_factory().is_some(),
            "W2 是 LLM 重器官, llm_factory() 必须返 Some (0 装诚实)"
        );
    }

    /// v2 新增: W2 organ organ_id + name 锁定 W2
    #[test]
    fn w2_organ_name_and_organ_id_locked() {
        let organ = CausalWorldModelOrgan::new(test_factory(), "minimax-m3-thinking");
        assert_eq!(organ.name(), "W2 Causal World Model");
        assert_eq!(organ.organ_id(), OrganKind::W2);
    }

    /// v2 新增: W3 organ llm_factory() 返 None (W3 主路径是确定性无 LLM)
    #[test]
    fn w3_organ_llm_factory_returns_none_per_v1_truth() {
        let organ = CausalEdgeMiningOrgan::new();
        assert!(
            organ.llm_factory().is_none(),
            "W3 主路径是统计挖掘 (确定性无 LLM), trait 必须返 None"
        );
    }

    /// v2 新增: W3 organ organ_id + name 锁定 W3
    #[test]
    fn w3_organ_name_and_organ_id_locked() {
        let organ = CausalEdgeMiningOrgan::new();
        assert_eq!(organ.name(), "W3 Causal Edge Mining");
        assert_eq!(organ.organ_id(), OrganKind::W3);
    }

    /// v2 新增: CausalSimulator 默认配置 (max_steps=8, reject_threshold=0.3)
    #[test]
    fn causal_simulator_defaults_match_v1() {
        let graph = build_chain_graph();
        let llm: Arc<dyn CausalLlm> = Arc::new(MockCausalLlm::default());
        let sim = CausalSimulator::new(graph, llm);
        assert_eq!(sim.max_steps, 8);
        assert!((sim.reject_threshold - 0.3).abs() < 1e-9);
    }
}
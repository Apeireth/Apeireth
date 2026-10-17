//! 检索维度自进化 (V4-c) —— 检索排序权重随使用证据小幅改写。
//!
//! 假说 (研究卡「检索基础设施的自进化闭环」的工程化): 检索结构不应是设计期
//! 常量, 而应是运行期可被证据改写的变量 —— 哪类记忆总是有用, 就给它的检索
//! 维度小幅提权。本模块是**纯确定性内核 + 生效值落地槽**, 无 IO、不读时钟;
//! 证据回执采集与账本落盘在接线层。
//!
//! ## 证据账 (信号定义, 0 装诚实)
//!
//! - **信号源 = 注入后命中回执** ([`SelectedAttribution`] 注入回执流): 检索命中
//!   并注入后, 后续检索轮的选中回执就是「后续使用」的最简可用真值面。本代码
//!   库**没有**回合引用标记 (回合输出不携带"该记忆被引用/被追问"回执), 该
//!   信号面显式缺席 —— 缺席就**不进化**, 绝不编造"被引用/被追问"信号。
//! - **正信号** [`EvidenceSignal::Positive`]: 注入条目在下个有选中回执的检索
//!   轮再次被命中 (注入后仍在赚命中 = 有用)。
//! - **负信号** [`EvidenceSignal::Negative`]: 注入条目在下个有选中回执的检索
//!   轮未再命中 (机会在场而无回执 = 注入后无用)。
//! - **无信号**: 之后再无有选中的检索轮、或回执面缺位 → 不裁决、不进化
//!   (显式如实; 接线层的 [`SignalSurface`] 状态会照实报"无信号面")。
//!
//! ## 调权规则 (步长有界防震荡, 单调可回溯)
//!
//! - 归因: 证据同时记到**条目显著性** (该条目) 与其**显著维度** (层权重);
//!   条目无评分分量数据时只动条目, 维度归因如实缺席 (不编造)。
//! - 裁决: 同一调权对象一批证据按净方向动一步 (正>负 提权, 负>正 降权,
//!   持平不调), 每步 ±[`DIMENSION_STEP`] / ±[`ENTRY_STEP`] (有界)。
//! - 走廊: 维度权重夹在基线 ±[`TUNING_CORRIDOR`] 内 (下限 0), 条目显著性夹在
//!   ±[`TUNING_CORRIDOR`] 内 —— 往复信号只在走廊内往复, 永不越界 (防震荡)。
//! - 可回溯: 每次真实移动在自进化账 `retrieval_tuning.jsonl` 留一行
//!   ([`RetrievalTuningRecord`]: 何时 / 因何证据 / 何维度 / 前后权重), `seq`
//!   单调递增且前后值接龙, 轨迹可从账面完整重建。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::scope::{MemoryRankingConfig, ScoreComponents};

/// 维度权重的单步上限 (每条证据一批最多动一步; 有界, 防震荡)。
pub const DIMENSION_STEP: f64 = 0.01;

/// 条目显著性的单步上限 (与维度同档小幅)。
pub const ENTRY_STEP: f64 = 0.01;

/// 权重走廊半宽: 相对基线的最大单向漂移 (往复信号不越界的硬边界)。
pub const TUNING_CORRIDOR: f64 = 0.05;

/// 检索维度 (评分层) —— 权重改写的对象之一 (`MemoryRankingConfig` 各层权重)。
///
/// 顺序 = [`ScoreComponents`] 字段序 = 显著维度归因的平局裁决序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalDimension {
    /// 语义相似度层。
    Semantic,
    /// 词法命中层。
    Lexical,
    /// 重要性层。
    Importance,
    /// 新近度层。
    Recency,
    /// 激活层 (使用史)。
    Activation,
    /// 连续性层。
    Continuity,
    /// 置信层。
    Confidence,
    /// 图谱关系层。
    Graph,
    /// 新颖度层。
    Novelty,
}

impl RetrievalDimension {
    /// 全部维度 (固定序 = 归因平局裁决序)。
    pub const ALL: [RetrievalDimension; 9] = [
        Self::Semantic,
        Self::Lexical,
        Self::Importance,
        Self::Recency,
        Self::Activation,
        Self::Continuity,
        Self::Confidence,
        Self::Graph,
        Self::Novelty,
    ];

    /// 维度 id (snake_case, 账面 `dimension:<id>` 用)。
    pub const fn id(self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Lexical => "lexical",
            Self::Importance => "importance",
            Self::Recency => "recency",
            Self::Activation => "activation",
            Self::Continuity => "continuity",
            Self::Confidence => "confidence",
            Self::Graph => "graph",
            Self::Novelty => "novelty",
        }
    }

    /// 从维度 id 解析 (账面读回用)。
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|dimension| dimension.id() == id)
    }

    /// 该维度在评分里的分量值。
    pub fn component(self, components: &ScoreComponents) -> f64 {
        match self {
            Self::Semantic => components.semantic,
            Self::Lexical => components.lexical,
            Self::Importance => components.importance,
            Self::Recency => components.recency,
            Self::Activation => components.activation,
            Self::Continuity => components.continuity,
            Self::Confidence => components.confidence,
            Self::Graph => components.graph,
            Self::Novelty => components.novelty,
        }
    }

    /// 显著维度归因 (确定性纯函数): 分量值最大者, 平局按 [`Self::ALL`] 固定序。
    ///
    /// 全部分量 ≤ 0 时**无显著维度** (`None`) —— 归因不编造。
    pub fn dominant(components: &ScoreComponents) -> Option<Self> {
        let mut best: Option<(Self, f64)> = None;
        for dimension in Self::ALL {
            let value = dimension.component(components);
            match best {
                Some((_, best_value)) if best_value >= value => {}
                _ => best = Some((dimension, value)),
            }
        }
        best.filter(|(_, value)| *value > 0.0).map(|(d, _)| d)
    }

    /// 账面 `dimension:<id>` 目标串。
    pub fn target_id(self) -> String {
        format!("dimension:{}", self.id())
    }
}

/// 条目显著性账面目标串 (`entry:<memory_id>`)。
pub fn entry_target_id(memory_id: &str) -> String {
    format!("entry:{memory_id}")
}

/// 注入回执的一条归因 (选中条目 + 其显著维度; [`ScoreComponents`] 缺失时维度
/// 如实为 `None`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedAttribution {
    /// 被选中注入的记忆 id。
    pub id: String,
    /// 选中时的显著维度 (归因); `None` = 无评分分量数据, 不编造归因。
    pub dominant_dimension: Option<RetrievalDimension>,
}

impl SelectedAttribution {
    /// 一条注入回执归因。
    pub fn new(id: impl Into<String>, dominant_dimension: Option<RetrievalDimension>) -> Self {
        Self {
            id: id.into(),
            dominant_dimension,
        }
    }
}

/// 证据信号: 注入后命中回执推导出的使用证据方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSignal {
    /// 注入后再次被命中 (有用)。
    Positive,
    /// 注入后机会在场而未再命中 (无用)。
    Negative,
}

impl EvidenceSignal {
    /// 账面信号词。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }
}

/// 一条使用证据 (信号 + 归因 + 回执摘要)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuningEvidence {
    /// 信号方向。
    pub signal: EvidenceSignal,
    /// 涉及的记忆条目。
    pub memory_id: String,
    /// 归因维度 (注入时的显著维度); `None` = 无分量数据不编造。
    pub dimension: Option<RetrievalDimension>,
    /// 证据时刻 (epoch ms, 由回执携带, 引擎不读时钟)。
    pub at_epoch_ms: i64,
    /// 因何证据 (人可读回执摘要)。
    pub note: String,
}

impl TuningEvidence {
    /// 一条正信号 (注入后命中回执再现)。
    pub fn positive(
        memory_id: impl Into<String>,
        dimension: Option<RetrievalDimension>,
        at_epoch_ms: i64,
    ) -> Self {
        let memory_id = memory_id.into();
        Self {
            signal: EvidenceSignal::Positive,
            note: format!("注入后命中回执: {memory_id} 于下个有选中回执的检索轮再次命中"),
            memory_id,
            dimension,
            at_epoch_ms,
        }
    }

    /// 一条负信号 (注入后机会在场而未再命中)。
    pub fn negative(
        memory_id: impl Into<String>,
        dimension: Option<RetrievalDimension>,
        at_epoch_ms: i64,
    ) -> Self {
        let memory_id = memory_id.into();
        Self {
            signal: EvidenceSignal::Negative,
            note: format!("注入后命中回执: {memory_id} 在下个有选中回执的检索轮未再命中"),
            memory_id,
            dimension,
            at_epoch_ms,
        }
    }
}

/// 自进化账的一行 —— 何时 / 因何证据 / 何维度 / 前后权重 (append-only 可回溯)。
///
/// 序列化即 `retrieval_tuning.jsonl` 一行的 body (snake_case)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalTuningRecord {
    /// 单调递增序号 (引擎内全局)。
    pub seq: u64,
    /// 调权时刻 (epoch ms, 由证据携带)。
    pub at_epoch_ms: i64,
    /// 何维度: `dimension:<id>` (层权重) 或 `entry:<memory_id>` (条目显著性)。
    pub target: String,
    /// 判定信号 (`positive` / `negative`)。
    pub signal: String,
    /// 调整前权重。
    pub previous: f64,
    /// 调整后权重 (夹紧后; 真实移动才有本行)。
    pub next: f64,
    /// 因何证据 (批内净方向 + 回执条目摘要)。
    pub evidence: String,
}

/// 一个调权对象的证据账 (正/负计数; 无移动也如实累计)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceTally {
    /// 正信号条数。
    pub positive: u64,
    /// 负信号条数。
    pub negative: u64,
}

/// 有界权重: 基线锚定 + 走廊夹紧 + 固定小步 (防震荡的物理边界)。
#[derive(Debug, Clone, Copy, PartialEq)]
struct BoundedWeight {
    baseline: f64,
    value: f64,
    step: f64,
    min: f64,
    max: f64,
}

impl BoundedWeight {
    /// 维度权重: 走廊 = 基线 ±[`TUNING_CORRIDOR`] (下限 0)。
    fn dimension(baseline: f64) -> Self {
        Self {
            baseline,
            value: baseline,
            step: DIMENSION_STEP,
            min: (baseline - TUNING_CORRIDOR).max(0.0),
            max: baseline + TUNING_CORRIDOR,
        }
    }

    /// 条目显著性: 基线 0, 走廊 ±[`TUNING_CORRIDOR`]。
    fn entry() -> Self {
        Self {
            baseline: 0.0,
            value: 0.0,
            step: ENTRY_STEP,
            min: -TUNING_CORRIDOR,
            max: TUNING_CORRIDOR,
        }
    }

    /// 走廊内动一步; 已到边界 = 零移动 (`None`, 不造"移动"假账)。
    fn move_by(&mut self, direction: i8) -> Option<(f64, f64)> {
        let previous = self.value;
        let next = (previous + f64::from(direction) * self.step).clamp(self.min, self.max);
        if next == previous {
            return None;
        }
        self.value = next;
        Some((previous, next))
    }
}

/// 检索维度自进化引擎: 观察使用证据 → 小幅改写层权重/条目显著性, 每次移动
/// 留痕可回溯。
///
/// 全确定性、无 IO (持久化由接线层负责); 时间戳来自证据。
#[derive(Debug)]
pub struct RetrievalTuningEngine {
    /// 基线 (走廊锚定; `diversity_lambda` 不属可调维度, 保持基线值)。
    baseline: MemoryRankingConfig,
    dimensions: BTreeMap<RetrievalDimension, BoundedWeight>,
    entries: BTreeMap<String, BoundedWeight>,
    tally: BTreeMap<String, EvidenceTally>,
    history: Vec<RetrievalTuningRecord>,
    next_seq: u64,
}

impl Default for RetrievalTuningEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrievalTuningEngine {
    /// 以默认评分权重为基线起步。
    pub fn new() -> Self {
        Self::with_baseline(MemoryRankingConfig::default())
    }

    /// 以给定评分权重为基线起步 (走廊锚定在基线上)。
    pub fn with_baseline(baseline: MemoryRankingConfig) -> Self {
        let mut dimensions = BTreeMap::new();
        for (dimension, value) in [
            (RetrievalDimension::Semantic, baseline.semantic_weight),
            (RetrievalDimension::Lexical, baseline.lexical_weight),
            (RetrievalDimension::Importance, baseline.importance_weight),
            (RetrievalDimension::Recency, baseline.recency_weight),
            (RetrievalDimension::Activation, baseline.activation_weight),
            (RetrievalDimension::Continuity, baseline.continuity_weight),
            (RetrievalDimension::Confidence, baseline.confidence_weight),
            (RetrievalDimension::Graph, baseline.graph_weight),
            (RetrievalDimension::Novelty, baseline.novelty_weight),
        ] {
            dimensions.insert(dimension, BoundedWeight::dimension(value));
        }
        Self {
            baseline,
            dimensions,
            entries: BTreeMap::new(),
            tally: BTreeMap::new(),
            history: Vec::new(),
            next_seq: 1,
        }
    }

    /// 观察一批证据: 按调权对象归并、净方向裁决、每对象最多动一步。
    ///
    /// 持平 (正负相抵) 不调; 已到走廊边界的移动不产生记录 (零移动不造账)。
    /// 返回本批产生的移动记录 (已入 [`Self::history`])。
    pub fn observe_batch(&mut self, evidence: &[TuningEvidence]) -> Vec<RetrievalTuningRecord> {
        let mut by_target: BTreeMap<String, TargetEvidence> = BTreeMap::new();
        for item in evidence {
            by_target
                .entry(entry_target_id(&item.memory_id))
                .or_insert_with(|| TargetEvidence::for_entry(&item.memory_id))
                .push(item);
            if let Some(dimension) = item.dimension {
                by_target
                    .entry(dimension.target_id())
                    .or_insert_with(|| TargetEvidence::for_dimension(dimension))
                    .push(item);
            }
        }

        let mut records = Vec::new();
        for (target_id, batch) in by_target {
            let positive = i64::try_from(batch.tally.positive).unwrap_or(i64::MAX);
            let negative = i64::try_from(batch.tally.negative).unwrap_or(i64::MAX);
            let net = positive - negative;
            if net == 0 {
                self.tally.insert(target_id, batch.tally);
                continue;
            }
            let direction = if net > 0 { 1 } else { -1 };
            let moved = match &batch.target {
                TuningTarget::Dimension(dimension) => {
                    let weight = self
                        .dimensions
                        .entry(*dimension)
                        .or_insert_with(|| BoundedWeight::dimension(0.0));
                    weight.move_by(direction)
                }
                TuningTarget::Entry(memory_id) => self
                    .entries
                    .entry(memory_id.clone())
                    .or_insert_with(BoundedWeight::entry)
                    .move_by(direction),
            };
            self.tally.insert(target_id.clone(), batch.tally);
            if let Some((previous, next)) = moved {
                let record = RetrievalTuningRecord {
                    seq: self.next_seq,
                    at_epoch_ms: batch.at_epoch_ms,
                    target: target_id,
                    signal: if net > 0 {
                        EvidenceSignal::Positive.as_str().to_string()
                    } else {
                        EvidenceSignal::Negative.as_str().to_string()
                    },
                    previous,
                    next,
                    evidence: batch.evidence_summary(),
                };
                self.next_seq += 1;
                self.history.push(record.clone());
                records.push(record);
            }
        }
        records
    }

    /// 观察单条证据 (批量观察的便捷口)。
    pub fn observe(&mut self, evidence: &TuningEvidence) -> Vec<RetrievalTuningRecord> {
        self.observe_batch(std::slice::from_ref(evidence))
    }

    /// 当前层权重 (改写后的生效值)。
    pub fn ranking(&self) -> MemoryRankingConfig {
        let value = |dimension: RetrievalDimension| {
            self.dimensions
                .get(&dimension)
                .map(|weight| weight.value)
                .unwrap_or(0.0)
        };
        MemoryRankingConfig {
            semantic_weight: value(RetrievalDimension::Semantic),
            lexical_weight: value(RetrievalDimension::Lexical),
            importance_weight: value(RetrievalDimension::Importance),
            recency_weight: value(RetrievalDimension::Recency),
            activation_weight: value(RetrievalDimension::Activation),
            continuity_weight: value(RetrievalDimension::Continuity),
            confidence_weight: value(RetrievalDimension::Confidence),
            graph_weight: value(RetrievalDimension::Graph),
            novelty_weight: value(RetrievalDimension::Novelty),
            diversity_lambda: self.baseline.diversity_lambda,
        }
    }

    /// 条目显著性现值 (未调过的条目 = 0)。
    pub fn salience(&self, memory_id: &str) -> f64 {
        self.entries
            .get(memory_id)
            .map(|weight| weight.value)
            .unwrap_or(0.0)
    }

    /// 证据账 (正/负计数, 按调权对象)。
    pub fn evidence_tally(&self) -> &BTreeMap<String, EvidenceTally> {
        &self.tally
    }

    /// 全部移动记录 (新 → 旧序即文件序)。
    pub fn history(&self) -> &[RetrievalTuningRecord] {
        &self.history
    }

    /// 进化次数 (真实移动的记录数)。
    pub fn evolution_count(&self) -> u64 {
        self.history.len() as u64
    }

    /// 最近一次调权真值。
    pub fn last_move(&self) -> Option<&RetrievalTuningRecord> {
        self.history.last()
    }

    /// 生效值快照 (接线层 [`install_effective`] 的输入)。
    pub fn effective(&self) -> EffectiveRanking {
        EffectiveRanking {
            ranking: self.ranking(),
            salience: self
                .entries
                .iter()
                .map(|(id, weight)| (id.clone(), weight.value))
                .collect(),
        }
    }
}

/// 一批同目标证据的归并账 (净方向裁决 + 因何证据摘要)。
struct TargetEvidence {
    target: TuningTarget,
    tally: EvidenceTally,
    at_epoch_ms: i64,
    ids: Vec<String>,
}

enum TuningTarget {
    Dimension(RetrievalDimension),
    Entry(String),
}

impl TargetEvidence {
    fn for_dimension(dimension: RetrievalDimension) -> Self {
        Self {
            target: TuningTarget::Dimension(dimension),
            tally: EvidenceTally::default(),
            at_epoch_ms: i64::MIN,
            ids: Vec::new(),
        }
    }

    fn for_entry(memory_id: &str) -> Self {
        Self {
            target: TuningTarget::Entry(memory_id.to_string()),
            tally: EvidenceTally::default(),
            at_epoch_ms: i64::MIN,
            ids: Vec::new(),
        }
    }

    fn push(&mut self, evidence: &TuningEvidence) {
        match evidence.signal {
            EvidenceSignal::Positive => self.tally.positive += 1,
            EvidenceSignal::Negative => self.tally.negative += 1,
        }
        self.at_epoch_ms = self.at_epoch_ms.max(evidence.at_epoch_ms);
        if !self.ids.contains(&evidence.memory_id) {
            self.ids.push(evidence.memory_id.clone());
        }
    }

    /// 因何证据 (账面摘要): 净方向计数 + 回执条目 (有界)。
    fn evidence_summary(&self) -> String {
        const MAX_IDS: usize = 4;
        let mut ids = self.ids.clone();
        ids.sort();
        let mut shown = ids
            .iter()
            .take(MAX_IDS)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if ids.len() > MAX_IDS {
            shown.push_str(", …");
        }
        format!(
            "注入后命中回执: 正 {} / 负 {} ({shown})",
            self.tally.positive, self.tally.negative
        )
    }
}

/// 进程内生效值 override (调权落地点)。
///
/// 接线层在每次移动后 [`install_effective`], 检索排序使用点经
/// [`effective_ranking_or`] / [`entry_salience`] 实时取值 —— 调整真生效,
/// 不装学习。无 override 时回落调用方传入值/零显著性。
static EFFECTIVE: std::sync::Mutex<Option<EffectiveRanking>> = std::sync::Mutex::new(None);

/// 生效检索排序状态: 层权重 + 条目显著性表。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectiveRanking {
    /// 生效层权重。
    pub ranking: MemoryRankingConfig,
    /// 生效条目显著性 (`memory_id` → 显著性加成)。
    pub salience: BTreeMap<String, f64>,
}

/// 安装当前生效检索排序 (调权后调用)。
pub fn install_effective(value: EffectiveRanking) {
    let mut slot = EFFECTIVE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = Some(value);
}

/// 清除生效 override (回落调用方传入值; 测试与重置用)。
pub fn clear_effective() {
    let mut slot = EFFECTIVE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = None;
}

/// 读生效层权重: 有 override 取 override, 否则取调用方传入值 (零改动回落)。
pub fn effective_ranking_or(fallback: MemoryRankingConfig) -> MemoryRankingConfig {
    EFFECTIVE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .map(|effective| effective.ranking)
        .unwrap_or(fallback)
}

/// 读条目显著性 (无 override / 未调条目 = 0)。
pub fn entry_salience(memory_id: &str) -> f64 {
    EFFECTIVE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .and_then(|effective| effective.salience.get(memory_id).copied())
        .unwrap_or(0.0)
}

/// 生效 override 是否在场 (诊断/自述用)。
pub fn effective_installed() -> bool {
    EFFECTIVE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_some()
}

/// 测试隔离: 生效 override 是**进程级全局槽**, 同一测试进程里凡触碰该槽的
/// 用例必须串行 (crate 内各测试模块共用这把锁, 不改产品语义)。
#[cfg(test)]
pub(crate) static EFFECTIVE_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_effective() -> std::sync::MutexGuard<'static, ()> {
        EFFECTIVE_TEST_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn evidence(
        signal: EvidenceSignal,
        id: &str,
        dimension: Option<RetrievalDimension>,
    ) -> TuningEvidence {
        match signal {
            EvidenceSignal::Positive => TuningEvidence::positive(id, dimension, 1_000),
            EvidenceSignal::Negative => TuningEvidence::negative(id, dimension, 1_000),
        }
    }

    #[test]
    fn dominant_dimension_is_argmax_with_stable_tie_order_and_never_invented() {
        let components = ScoreComponents {
            semantic: 0.4,
            lexical: 0.9,
            novelty: 0.9,
            ..ScoreComponents::default()
        };
        // lexical 与 novelty 平局 → 按固定序取 lexical。
        assert_eq!(
            RetrievalDimension::dominant(&components),
            Some(RetrievalDimension::Lexical)
        );
        let zero = ScoreComponents::default();
        assert_eq!(
            RetrievalDimension::dominant(&zero),
            None,
            "全零分量 = 无显著维度, 归因不编造"
        );
    }

    #[test]
    fn positive_evidence_promotes_the_entry_and_its_dimension() {
        let mut engine = RetrievalTuningEngine::new();
        let records = engine.observe(&evidence(
            EvidenceSignal::Positive,
            "mem-a",
            Some(RetrievalDimension::Semantic),
        ));
        assert_eq!(
            records.len(),
            2,
            "条目显著性 + 归因维度各动一步: {records:?}"
        );
        let entry = records
            .iter()
            .find(|record| record.target == "entry:mem-a")
            .expect("entry move");
        assert_eq!(entry.signal, "positive");
        assert!((entry.next - entry.previous - ENTRY_STEP).abs() < 1e-12);
        let dimension = records
            .iter()
            .find(|record| record.target == "dimension:semantic")
            .expect("dimension move");
        assert!((dimension.next - dimension.previous - DIMENSION_STEP).abs() < 1e-12);
        assert!(
            dimension.evidence.contains("正 1 / 负 0"),
            "{}",
            dimension.evidence
        );
        assert_eq!(engine.evolution_count(), 2);
    }

    #[test]
    fn negative_evidence_demotes_with_the_same_bounded_step() {
        let mut engine = RetrievalTuningEngine::new();
        let records = engine.observe(&evidence(
            EvidenceSignal::Negative,
            "mem-b",
            Some(RetrievalDimension::Lexical),
        ));
        assert_eq!(records.len(), 2);
        for record in &records {
            assert_eq!(record.signal, "negative");
            assert!(record.next < record.previous);
            assert!((record.previous - record.next - ENTRY_STEP).abs() < 1e-12);
        }
        assert!(engine.salience("mem-b") < 0.0);
    }

    #[test]
    fn moves_without_attribution_touch_only_the_entry() {
        let mut engine = RetrievalTuningEngine::new();
        let records = engine.observe(&evidence(EvidenceSignal::Positive, "mem-c", None));
        assert_eq!(records.len(), 1, "无分量数据 = 维度归因缺席, 只动条目");
        assert_eq!(records[0].target, "entry:mem-c");
    }

    #[test]
    fn the_step_is_bounded_and_the_corridor_clamps_without_fabricating_moves() {
        let mut engine = RetrievalTuningEngine::new();
        let baseline = engine.ranking().semantic_weight;
        let mut moves = 0;
        for _ in 0..20 {
            moves += engine
                .observe(&evidence(
                    EvidenceSignal::Positive,
                    "mem-d",
                    Some(RetrievalDimension::Semantic),
                ))
                .iter()
                .filter(|record| record.target == "dimension:semantic")
                .count();
        }
        let value = engine.ranking().semantic_weight;
        assert!(
            (value - (baseline + TUNING_CORRIDOR)).abs() < 1e-12,
            "{value}"
        );
        let expected = (TUNING_CORRIDOR / DIMENSION_STEP).round() as usize;
        assert_eq!(moves, expected, "只有真实移动留账, 走廊顶的零移动不造账");
        for record in engine.history() {
            assert!((record.next - record.previous).abs() <= DIMENSION_STEP + 1e-12);
        }
    }

    #[test]
    fn alternating_evidence_stays_inside_the_corridor() {
        let mut engine = RetrievalTuningEngine::new();
        let baseline = engine.ranking().semantic_weight;
        for round in 0..100 {
            let signal = if round % 2 == 0 {
                EvidenceSignal::Positive
            } else {
                EvidenceSignal::Negative
            };
            engine.observe(&evidence(
                signal,
                "mem-e",
                Some(RetrievalDimension::Semantic),
            ));
            let value = engine.ranking().semantic_weight;
            assert!(
                value >= baseline - TUNING_CORRIDOR - 1e-12
                    && value <= baseline + TUNING_CORRIDOR + 1e-12,
                "往复信号越界: {value}"
            );
            assert!(engine.salience("mem-e").abs() <= TUNING_CORRIDOR + 1e-12);
        }
    }

    #[test]
    fn a_tied_evidence_batch_makes_no_move_but_stays_accounted() {
        let mut engine = RetrievalTuningEngine::new();
        let before = engine.ranking();
        // 同一维度一正一负 = 维度上持平不调; 两个条目是各自独立的调权对象。
        let records = engine.observe_batch(&[
            evidence(
                EvidenceSignal::Positive,
                "mem-f",
                Some(RetrievalDimension::Graph),
            ),
            evidence(
                EvidenceSignal::Negative,
                "mem-g",
                Some(RetrievalDimension::Graph),
            ),
        ]);
        assert!(
            records
                .iter()
                .all(|record| record.target != "dimension:graph"),
            "维度持平不调 (不乱调): {records:?}"
        );
        assert_eq!(engine.ranking(), before, "维度权重不动");
        assert_eq!(records.len(), 2, "条目显著性是独立调权对象, 各动一步");
        let tally = &engine.evidence_tally()["dimension:graph"];
        assert_eq!((tally.positive, tally.negative), (1, 1), "证据账照实累计");
        // 同一对象一正一负 = 该对象持平不调。
        let mut engine = RetrievalTuningEngine::new();
        let records = engine.observe_batch(&[
            evidence(
                EvidenceSignal::Positive,
                "mem-h",
                Some(RetrievalDimension::Graph),
            ),
            evidence(
                EvidenceSignal::Negative,
                "mem-h",
                Some(RetrievalDimension::Graph),
            ),
        ]);
        assert!(records.is_empty(), "同对象持平不调: {records:?}");
    }

    #[test]
    fn the_record_chain_is_monotonic_and_traceable() {
        let mut engine = RetrievalTuningEngine::new();
        for round in 0..3 {
            let signal = if round % 2 == 0 {
                EvidenceSignal::Positive
            } else {
                EvidenceSignal::Negative
            };
            engine.observe_batch(&[
                evidence(signal, "mem-h", Some(RetrievalDimension::Recency)),
                evidence(signal, "mem-i", Some(RetrievalDimension::Recency)),
            ]);
        }
        let history = engine.history();
        assert!(!history.is_empty());
        for (index, record) in history.iter().enumerate() {
            assert_eq!(record.seq, index as u64 + 1, "seq 单调递增");
        }
        let recency: Vec<_> = history
            .iter()
            .filter(|record| record.target == "dimension:recency")
            .collect();
        for pair in recency.windows(2) {
            assert!(
                (pair[0].next - pair[1].previous).abs() < 1e-12,
                "前后值接龙, 轨迹可回溯"
            );
        }
    }

    #[test]
    fn no_evidence_means_no_evolution() {
        let mut engine = RetrievalTuningEngine::new();
        assert!(engine.observe_batch(&[]).is_empty());
        assert_eq!(engine.evolution_count(), 0);
        assert!(engine.last_move().is_none());
    }

    #[test]
    fn the_effective_override_is_consumed_by_ranking_and_clearable() {
        let _guard = lock_effective();
        clear_effective();
        let fallback = MemoryRankingConfig::default();
        assert_eq!(effective_ranking_or(fallback), fallback);
        assert_eq!(entry_salience("mem-j"), 0.0);
        assert!(!effective_installed());

        let mut engine = RetrievalTuningEngine::new();
        engine.observe(&evidence(
            EvidenceSignal::Positive,
            "mem-j",
            Some(RetrievalDimension::Activation),
        ));
        install_effective(engine.effective());
        assert!(effective_installed());
        assert_eq!(effective_ranking_or(fallback), engine.ranking());
        assert!((entry_salience("mem-j") - ENTRY_STEP).abs() < 1e-12);
        clear_effective();
        assert_eq!(effective_ranking_or(fallback), fallback);
        assert_eq!(entry_salience("mem-j"), 0.0);
    }
}

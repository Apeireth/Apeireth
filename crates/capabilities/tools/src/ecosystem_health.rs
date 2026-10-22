//! ecosystem_health —— 工具生态健康度内核（生物机制族 W2 批，2026-10-19；IMPLEMENTED，未接线）。
//!
//! # 生物原型
//!
//! 共生菌群（symbiotic microbiota）给宿主提供能力：消化、合成维生素、占位抵抗病原。
//! 生态学上，**多样性本身就是健康**：功能冗余让群落能吸收扰动，关键功能有备份；
//! 但「功能重叠」是双刃剑——适度重叠是韧性，过度重叠意味着多个成员抢同一份生态位，
//! 群落失去分工，表现为菌群失调（dysbiosis）。失调不是某一个成员的错，而是
//! **结构与分工**的病；临床上先调整生态（补益生菌、调饮食），而不是见一个菌就杀一个。
//!
//! 本模块把工具生态类比成菌群：
//!
//! - [`ToolProfile`] 是一个「菌株」：名字是主键，能力集合是它占据的功能生态位，
//!   [`ToolSource`] 是它的来源（内置 / MCP / 本地），`last_used_ms` 是观察到的
//!   最近活动记录（**只作输入**，本模块不读系统时钟）；
//! - **多样性**（[`EcosystemReport::diversity_score`]）看能力覆盖相对工具数是否够宽；
//! - **冗余**（`redundancy_score`）看是否有一批工具完全重复同一组能力；
//! - **重叠**（[`OverlapPair`]）用能力集合的 Jaccard 相似度量化「两个工具抢同一份
//!   生态位」的程度，达到阈值就记为疑似竞争；
//! - **失调**（`dysbiosis_score`）把三者加权成一个 0..1 的健康度反向指标；
//! - **建议**（`recommendations`）是给上层/人看的调理意见，**不是剪枝指令**。
//!
//! # 工程边界（0 装诚实口径）
//!
//! - **IMPLEMENTED，未接线**：本模块是纯函数内核，**未进工具注册表**
//!   （`ToolRegistry` / 内置工具清单不认识本模块）、**未产出产品面报告**
//!   （没有会话摘要 / 面板字段消费它）、**无 CLI / 工具命令入口**。
//!   接线与产品面披露留给 W3。
//! - **健康度只给建议，不自动剪枝**：本模块不删除、不注销、不改写任何工具定义，
//!   也不返回「该删谁」的排序；`recommendations` 只是文本建议，是否合并、如何分工
//!   由上层策略或人决定。这是纯只读分析：入参 `&[ToolProfile]`，无 `&mut`、0 I/O、
//!   0 系统时钟、0 全局状态。
//! - **不新增依赖**：只用 `std` + `serde`；序列化只为报告/配置可存档，本模块不落盘。
//! - **不做动态追踪**：`last_used_ms` 完全由调用方注入，本模块不采样、不推导
//!   使用频率；同一输入两次调用必得同一报告。
//!
//! # 指标公式（全部落在 0..=1）
//!
//! 记 `n = tool_count`，`|U|` = 所有工具能力集合的并集大小
//! （`distinct_capabilities`），`C(n,2)` = 可能的两两组合数。
//!
//! - `diversity_score = min(1, |U| / n)`：平均每个工具贡献多少「新能力」；一个工具
//!   声明多项能力可以补偿另一个工具的能力面窄。空工具集无证据，记 0。
//! - **完全重复组**：能力集合（去首尾空白、去重、排序后）完全相同的**非空**组；
//!   组内 `k` 个工具产生 `k - 1` 个「多余副本」。空能力集工具信息不足，不计入
//!   重复（避免把「未声明」误判成「重复」）。
//! - `redundancy_score = max(0, 多余副本数 - redundancy_tolerance) / n`：容忍额度
//!   内的副本（默认 1 个，例如热备/降级副本）不扣分，超出的副本按工具数占比计分。
//! - `overlap_pairs`：对每一对工具算 Jaccard `|A∩B| / |A∪B|`，保留
//!   `similarity >= overlap_threshold`（**含等号**）的对；两集合任一为空时相似度
//!   无定义，按不重叠处理。展示前按「相似度降序 → 左名 → 右名」排序并截断到
//!   `max_pairs`。
//! - `overlap_pressure = Σ similarity / C(n,2)`（对**所有**达到阈值的对求和，
//!   **不受 `max_pairs` 截断影响**）：既反映重叠对的数量，也反映重叠的深度。
//! - `dysbiosis_score = w_d·(1 - diversity) + w_r·redundancy + w_o·overlap_pressure`，
//!   权重按 [`EcosystemWeights::normalized`] 归一（默认 0.4 / 0.2 / 0.4）。
//!
//! # 确定性与不变量
//!
//! - 工具按名字排序后分析；能力 token 去首尾空白、去重、排序；同名工具按**首次
//!   出现**计一次（生态以名字为主键）。因此除同名冲突外，输入顺序不影响结果。
//! - `overlap_pairs` 的左右名按字典序规范化为 `left < right`，**不含「工具与自身」**
//!   的对；排序是全序（相似度降序 → 左名升序 → 右名升序），`max_pairs` 只截断
//!   展示列表，不改任何分数。
//! - 所有权重/阈值在入口消毒：非有限或越界值回退默认，分数最终 `clamp(0,1)`，
//!   永不为 NaN / 越界。空工具集返回全 0 报告，不 panic。
//! - 复杂度 `O(n²)`：面向注册表尺度（几十到几百个工具），不是大规模图分析。

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// 默认重叠阈值：Jaccard `>= 0.8` 视为「疑似抢同一生态位」。
///
/// 0.8 的含义是「两个工具的能力集合至少八成重合」：低于此值的工具通常还能靠
/// 差异化能力共存，达到或超过此值才值得提示合并或明确分工。
pub const DEFAULT_OVERLAP_THRESHOLD: f32 = 0.8;

/// 默认冗余容忍额度：1 个完全重复副本不扣分（例如热备 / 降级副本）。
pub const DEFAULT_REDUNDANCY_TOLERANCE: usize = 1;

/// 默认明细上限：`overlap_pairs` 最多列出 32 对（防止报告随工具数平方膨胀）。
pub const DEFAULT_MAX_PAIRS: usize = 32;

/// 默认多样性权重：0.4（与重叠权重并列最高，「有没有分工」是健康度的主轴）。
pub const DEFAULT_DIVERSITY_WEIGHT: f32 = 0.4;

/// 默认冗余权重：0.2（冗余也提供韧性，只有「完全重复」才温和扣分）。
pub const DEFAULT_REDUNDANCY_WEIGHT: f32 = 0.2;

/// 默认重叠权重：0.4（与多样性权重并列最高，「抢不抢生态位」是健康度的主轴）。
pub const DEFAULT_OVERLAP_WEIGHT: f32 = 0.4;

/// 失调告警线：`dysbiosis_score >= 0.6` 时给出整体调理建议。
pub const DYSBIOSIS_ALERT_THRESHOLD: f32 = 0.6;

/// 逐对重叠建议最多列出的条数（报告文本有界，避免随工具数平方膨胀）。
const MAX_PAIR_RECOMMENDATIONS: usize = 3;

/// 完全重复组建议最多列出的组数（其余以一行汇总提示）。
const MAX_DUPLICATE_RECOMMENDATIONS: usize = 3;

/// 工具来源：内置 / MCP / 本地。
///
/// 来源不参与分数计算（分数只看能力结构），只用于给出依赖面建议：全部来自 MCP 的
/// 生态缺少本地降级路径，而「小而稳的本地工具」是菌群里的益生菌——依赖面小、
/// 故障半径小，能给关键能力兜底。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    /// 内置：随二进制分发、默认可用（依赖面最小）。
    Builtin,
    /// MCP：经 MCP 桥接的外部工具服务器提供（依赖网络与第三方进程）。
    Mcp,
    /// 本地：在用户机器上运行的小型本地工具（益生菌位）。
    Local,
}

impl ToolSource {
    /// 稳定的 snake_case 名称（用于解释文本与序列化键，与 serde 重命名一致）。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Mcp => "mcp",
            Self::Local => "local",
        }
    }
}

/// 单个工具的生态画像：名字是主键，能力集合决定重叠，来源决定依赖面。
///
/// `capabilities` 在分析入口会被去首尾空白、去重、排序；比较是**区分大小写**的
/// 精确字符串比较（调用方负责统一命名风格）。`last_used_ms` 只是调用方注入的
/// 最近活动记录（毫秒），本模块不读时钟；它只用于在完全重复组里判定「以谁为主」。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolProfile {
    /// 工具名（生态内的主键；同名条目按首次出现计一次）。
    pub name: String,
    /// 能力标签列表（自由文本，去空白、去重后参与 Jaccard 计算）。
    pub capabilities: Vec<String>,
    /// 工具来源（内置 / MCP / 本地）。
    pub source: ToolSource,
    /// 调用方观察到的最近使用时刻（毫秒，仅作输入；`None` 表示无记录）。
    pub last_used_ms: Option<u64>,
}

impl ToolProfile {
    /// 构造一个没有最近使用记录的工具画像（`last_used_ms = None`）。
    pub fn new(name: impl Into<String>, capabilities: Vec<String>, source: ToolSource) -> Self {
        Self {
            name: name.into(),
            capabilities,
            source,
            last_used_ms: None,
        }
    }
}

/// 失调评分的三项权重（详见模块文档的公式）。
///
/// 默认 [`DEFAULT_DIVERSITY_WEIGHT`] / [`DEFAULT_REDUNDANCY_WEIGHT`] /
/// [`DEFAULT_OVERLAP_WEIGHT`] = `0.4 / 0.2 / 0.4`：多样性与重叠各占四成，
/// 因为它们分别回答「生态有没有分工」和「成员是否抢同一份生态位」；冗余占两成，
/// 因为适度冗余是韧性而不是病，只有完全重复才应被温和惩罚。
///
/// 权重不必手工归一：分析入口调用 [`EcosystemWeights::normalized`]，按总和缩放；
/// 非有限 / 负数按 0 处理，总和为 0（或不可用）时整体回退默认权重。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EcosystemWeights {
    /// 多样性权重（作用在 `1 - diversity_score` 上：越不多样越失调）。
    pub diversity: f32,
    /// 冗余权重（作用在 `redundancy_score` 上）。
    pub redundancy: f32,
    /// 重叠权重（作用在 `overlap_pressure` 上）。
    pub overlap: f32,
}

impl Default for EcosystemWeights {
    fn default() -> Self {
        Self {
            diversity: DEFAULT_DIVERSITY_WEIGHT,
            redundancy: DEFAULT_REDUNDANCY_WEIGHT,
            overlap: DEFAULT_OVERLAP_WEIGHT,
        }
    }
}

impl EcosystemWeights {
    /// 用显式三项权重构造（是否归一由分析入口负责）。
    pub const fn new(diversity: f32, redundancy: f32, overlap: f32) -> Self {
        Self {
            diversity,
            redundancy,
            overlap,
        }
    }

    /// 消毒并归一：非有限 / 负数权重按 0 处理，再按总和缩放到和为 1。
    ///
    /// 总和为 0 或不可用（例如全 0、全非法、求和溢出到无穷）时回退
    /// [`EcosystemWeights::default`]；因此返回值的每一项都有限、非负，且总和为 1。
    pub fn normalized(self) -> Self {
        let diversity = sanitize_weight(self.diversity);
        let redundancy = sanitize_weight(self.redundancy);
        let overlap = sanitize_weight(self.overlap);
        let total = diversity + redundancy + overlap;
        if !total.is_finite() || total <= 0.0 {
            return Self::default();
        }
        Self {
            diversity: diversity / total,
            redundancy: redundancy / total,
            overlap: overlap / total,
        }
    }
}

/// 分析配置（全部字段公开，可由配置文件直接反序列化）。
///
/// [`EcosystemConfig::sanitized`] 只消毒 `overlap_threshold`：必须有限且落在
/// `0.0..=1.0`，否则回退 [`DEFAULT_OVERLAP_THRESHOLD`]（`0.0` 合法，表示把所有
/// 非空能力对都记为重叠）。`redundancy_tolerance` 与 `max_pairs` 是 `usize`，
/// 任意值都合法：前者越大越宽容，后者为 0 表示不列明细。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// 重叠阈值：Jaccard `>=` 该值记为疑似重叠（**含等号**）。
    pub overlap_threshold: f32,
    /// 冗余容忍额度：完全重复组里允许多少个「多余副本」不扣分（默认 1）。
    pub redundancy_tolerance: usize,
    /// `overlap_pairs` 明细上限（默认 32）；只截断展示列表，不影响分数。
    pub max_pairs: usize,
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            overlap_threshold: DEFAULT_OVERLAP_THRESHOLD,
            redundancy_tolerance: DEFAULT_REDUNDANCY_TOLERANCE,
            max_pairs: DEFAULT_MAX_PAIRS,
        }
    }
}

impl EcosystemConfig {
    /// 返回消毒后的配置：阈值非有限或越界时回退默认值，其余字段原样保留。
    pub fn sanitized(self) -> Self {
        let overlap_threshold = if self.overlap_threshold.is_finite()
            && (0.0..=1.0).contains(&self.overlap_threshold)
        {
            self.overlap_threshold
        } else {
            DEFAULT_OVERLAP_THRESHOLD
        };
        Self {
            overlap_threshold,
            redundancy_tolerance: self.redundancy_tolerance,
            max_pairs: self.max_pairs,
        }
    }
}

/// 一对疑似重叠的工具：左右名字按字典序规范化（`left < right`），
/// `similarity` 是能力集合的 Jaccard 相似度（`0..=1`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlapPair {
    /// 字典序较小的一侧工具名。
    pub left: String,
    /// 字典序较大的一侧工具名。
    pub right: String,
    /// 能力集合的 Jaccard 相似度 `|A∩B| / |A∪B|`，落在 `0..=1`。
    pub similarity: f32,
}

/// 一次工具生态体检的报告（纯只读快照，可序列化存档）。
///
/// 所有分数都是 0..=1，且为「[`ToolProfile`] 输入 + [`EcosystemConfig`] +
/// [`EcosystemWeights`]」的纯函数结果；报告不携带任何可执行动作，
/// `recommendations` 只是建议文本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcosystemReport {
    /// 参与分析的唯一工具数（同名按首次出现计一次）。
    pub tool_count: usize,
    /// 所有工具能力集合的并集大小（去空白、去重后）。
    pub distinct_capabilities: usize,
    /// 多样性分 `min(1, distinct_capabilities / tool_count)`，越高越健康。
    pub diversity_score: f32,
    /// 冗余分 `max(0, 多余副本数 - redundancy_tolerance) / tool_count`，越高越重复。
    pub redundancy_score: f32,
    /// 达到阈值的重叠对明细（相似度降序 → 左名 → 右名，截断到 `max_pairs`）。
    pub overlap_pairs: Vec<OverlapPair>,
    /// 失调分（0 健康，1 最失调），由多样性 / 冗余 / 重叠加权得到。
    pub dysbiosis_score: f32,
    /// 调理建议（非空：健康时给一条维持建议，空生态给一条状态说明）。
    pub recommendations: Vec<String>,
}

/// 分析工具生态并返回健康度报告（使用默认权重 [`EcosystemWeights::default`]）。
///
/// 这是纯函数：不读系统时钟、不做 I/O、不修改入参、不持有全局状态；同样的输入
/// （除同名冲突外，与顺序无关）必得同样的报告。空切片返回全 0 报告，不 panic。
///
/// **健康度只给建议，不自动剪枝**：返回值里的 `recommendations` 是给人/上层看的
/// 文本，不包含任何删除、注销或改写工具的动作。
pub fn analyze(profiles: &[ToolProfile], config: &EcosystemConfig) -> EcosystemReport {
    analyze_with_weights(profiles, config, &EcosystemWeights::default())
}

/// 与 [`analyze`] 相同，但使用显式权重（权重会先经
/// [`EcosystemWeights::normalized`] 消毒归一）。
pub fn analyze_with_weights(
    profiles: &[ToolProfile],
    config: &EcosystemConfig,
    weights: &EcosystemWeights,
) -> EcosystemReport {
    let config = config.sanitized();
    if profiles.is_empty() {
        return empty_report();
    }

    let tools = unique_tools(profiles);
    let tool_count = tools.len();
    let distinct_capabilities = capabilities_union_len(&tools);
    let diversity_score = (distinct_capabilities as f32 / tool_count as f32).clamp(0.0, 1.0);

    let duplicate_groups = collect_duplicate_groups(&tools);
    let surplus_copies: usize = duplicate_groups.iter().map(|group| group.len() - 1).sum();
    let penalized_copies = surplus_copies.saturating_sub(config.redundancy_tolerance);
    let redundancy_score = (penalized_copies as f32 / tool_count as f32).clamp(0.0, 1.0);

    let mut qualifying_pairs = collect_overlap_pairs(&tools, config.overlap_threshold);
    let overlap_pressure = compute_overlap_pressure(tool_count, &qualifying_pairs);
    let dysbiosis_score =
        compute_dysbiosis_score(weights, diversity_score, redundancy_score, overlap_pressure);

    let recommendations = build_recommendations(
        &tools,
        &duplicate_groups,
        &qualifying_pairs,
        config.overlap_threshold,
        AdvisoryMetrics {
            tool_count,
            distinct_capabilities,
            diversity_score,
            dysbiosis_score,
        },
    );

    qualifying_pairs.truncate(config.max_pairs);
    EcosystemReport {
        tool_count,
        distinct_capabilities,
        diversity_score,
        redundancy_score,
        overlap_pairs: qualifying_pairs,
        dysbiosis_score,
        recommendations,
    }
}

/// 空工具集的缺省报告：所有分数 0，只带一条状态说明（无工具可评估，不 panic）。
fn empty_report() -> EcosystemReport {
    EcosystemReport {
        tool_count: 0,
        distinct_capabilities: 0,
        diversity_score: 0.0,
        redundancy_score: 0.0,
        overlap_pairs: Vec::new(),
        dysbiosis_score: 0.0,
        recommendations: vec!["空工具集：无工具可评估，各分数记 0；等待工具注册后再评估".to_owned()],
    }
}

/// 归一化后的工具视图：能力去空白、去重、排序；工具按名字排序，同名只留首次出现。
struct ToolView {
    name: String,
    capabilities: Vec<String>,
    source: ToolSource,
    last_used_ms: Option<u64>,
}

/// 供建议生成使用的指标子集（避免把整份报告借来借去）。
struct AdvisoryMetrics {
    tool_count: usize,
    distinct_capabilities: usize,
    diversity_score: f32,
    dysbiosis_score: f32,
}

/// 构造唯一的工具视图：同名按首次出现计一次，随后按名字排序保证确定性。
fn unique_tools(profiles: &[ToolProfile]) -> Vec<ToolView> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut tools = Vec::with_capacity(profiles.len());
    for profile in profiles {
        if !seen.insert(profile.name.as_str()) {
            continue;
        }
        tools.push(ToolView {
            name: profile.name.clone(),
            capabilities: canonical_capabilities(&profile.capabilities),
            source: profile.source,
            last_used_ms: profile.last_used_ms,
        });
    }
    tools.sort_by(|left, right| left.name.cmp(&right.name));
    tools
}

/// 能力 token 归一：去首尾空白、丢弃空串、排序、去重（区分大小写）。
fn canonical_capabilities(capabilities: &[String]) -> Vec<String> {
    let mut canonical: Vec<String> = capabilities
        .iter()
        .map(|capability| capability.trim().to_owned())
        .filter(|capability| !capability.is_empty())
        .collect();
    canonical.sort_unstable();
    canonical.dedup();
    canonical
}

/// 所有工具能力集合的并集大小。
fn capabilities_union_len(tools: &[ToolView]) -> usize {
    let mut union: BTreeSet<&str> = BTreeSet::new();
    for tool in tools {
        for capability in &tool.capabilities {
            union.insert(capability.as_str());
        }
    }
    union.len()
}

/// 收集完全重复组：能力集合（非空）完全相同的工具索引组，按组首工具名排序。
fn collect_duplicate_groups(tools: &[ToolView]) -> Vec<Vec<usize>> {
    let mut by_signature: BTreeMap<&[String], Vec<usize>> = BTreeMap::new();
    for (index, tool) in tools.iter().enumerate() {
        if tool.capabilities.is_empty() {
            continue;
        }
        by_signature
            .entry(tool.capabilities.as_slice())
            .or_default()
            .push(index);
    }
    let mut groups: Vec<Vec<usize>> = by_signature
        .into_values()
        .filter(|group| group.len() >= 2)
        .collect();
    // 工具已按名字排序，组内索引天然升序；组间按首个成员（即字典序最小名字）排序。
    groups.sort_by_key(|group| group[0]);
    groups
}

/// 收集达到阈值的重叠对（左右名按字典序规范化，相似度降序 → 左名 → 右名）。
fn collect_overlap_pairs(tools: &[ToolView], threshold: f32) -> Vec<OverlapPair> {
    let mut pairs = Vec::new();
    for (index, tool) in tools.iter().enumerate() {
        for other in &tools[index + 1..] {
            if tool.capabilities.is_empty() || other.capabilities.is_empty() {
                // 0/0 无定义：把「未声明能力」误读成「完全重叠」会制造假阳性。
                continue;
            }
            let similarity = jaccard_similarity(&tool.capabilities, &other.capabilities);
            if similarity >= threshold {
                pairs.push(OverlapPair {
                    left: tool.name.clone(),
                    right: other.name.clone(),
                    similarity,
                });
            }
        }
    }
    pairs.sort_by(|left, right| {
        right
            .similarity
            .partial_cmp(&left.similarity)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.left.cmp(&right.left))
            .then_with(|| left.right.cmp(&right.right))
    });
    pairs
}

/// 两个已排序能力集合的 Jaccard 相似度；任一为空时按 0（无定义，不制造重叠）。
fn jaccard_similarity(left: &[String], right: &[String]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let mut intersection = 0usize;
    let mut left_index = 0usize;
    let mut right_index = 0usize;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => left_index += 1,
            Ordering::Greater => right_index += 1,
            Ordering::Equal => {
                intersection += 1;
                left_index += 1;
                right_index += 1;
            }
        }
    }
    let union = left.len() + right.len() - intersection;
    intersection as f32 / union as f32
}

/// 重叠压力：所有达到阈值的对的相似度之和 / 可能的两两组合数（不受 `max_pairs` 影响）。
fn compute_overlap_pressure(tool_count: usize, pairs: &[OverlapPair]) -> f32 {
    let pair_slots = tool_count.saturating_mul(tool_count.saturating_sub(1)) / 2;
    if pair_slots == 0 {
        return 0.0;
    }
    let similarity_sum: f64 = pairs.iter().map(|pair| f64::from(pair.similarity)).sum();
    (similarity_sum / pair_slots as f64).clamp(0.0, 1.0) as f32
}

/// 加权失调分：`w_d·(1-div) + w_r·red + w_o·pressure`，权重先归一，结果 clamp 到 0..=1。
fn compute_dysbiosis_score(
    weights: &EcosystemWeights,
    diversity_score: f32,
    redundancy_score: f32,
    overlap_pressure: f32,
) -> f32 {
    let normalized = weights.normalized();
    let weighted = f64::from(normalized.diversity) * f64::from(1.0 - diversity_score)
        + f64::from(normalized.redundancy) * f64::from(redundancy_score)
        + f64::from(normalized.overlap) * f64::from(overlap_pressure);
    (weighted as f32).clamp(0.0, 1.0)
}

/// 生成调理建议（只读文本，不产生任何剪枝动作）。
fn build_recommendations(
    tools: &[ToolView],
    duplicate_groups: &[Vec<usize>],
    pairs: &[OverlapPair],
    threshold: f32,
    metrics: AdvisoryMetrics,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    if metrics.dysbiosis_score >= DYSBIOSIS_ALERT_THRESHOLD {
        recommendations.push(format!(
            "疑似菌群失调（dysbiosis {:.2} ≥ {:.2}）：职责重叠/冗余偏高，建议先梳理能力分工，再合并同类工具或补足能力多样性",
            metrics.dysbiosis_score, DYSBIOSIS_ALERT_THRESHOLD
        ));
    }

    for group in duplicate_groups.iter().take(MAX_DUPLICATE_RECOMMENDATIONS) {
        recommendations.push(duplicate_group_advice(tools, group));
    }
    if duplicate_groups.len() > MAX_DUPLICATE_RECOMMENDATIONS {
        let extra = duplicate_groups.len() - MAX_DUPLICATE_RECOMMENDATIONS;
        recommendations.push(format!(
            "另有 {extra} 组工具能力集合完全相同，建议一并复核主备分工"
        ));
    }

    let mut pair_advice_count = 0usize;
    let mut skipped_pairs = 0usize;
    for pair in pairs {
        if pair.similarity >= 1.0 {
            // 能力集合完全相同：已由上面的重复组建议覆盖，避免同一对说两遍。
            continue;
        }
        if pair_advice_count < MAX_PAIR_RECOMMENDATIONS {
            recommendations.push(format!(
                "{}/{} 能力重叠 {:.2}，建议合并或明确分工",
                pair.left, pair.right, pair.similarity
            ));
            pair_advice_count += 1;
        } else {
            skipped_pairs += 1;
        }
    }
    if skipped_pairs > 0 {
        recommendations.push(format!(
            "另有 {skipped_pairs} 对工具能力重叠 ≥ {threshold:.2}，建议按需复核（明细见 overlap_pairs）"
        ));
    }

    let undeclared = tools
        .iter()
        .filter(|tool| tool.capabilities.is_empty())
        .count();
    if undeclared > 0 {
        recommendations.push(format!(
            "{undeclared} 个工具未声明任何能力，无法参与重叠评估；建议补齐能力声明后再评估"
        ));
    }

    if metrics.tool_count >= 2 && metrics.diversity_score < 0.5 {
        recommendations.push(format!(
            "{} 个工具只覆盖 {} 项能力（diversity {:.2}），能力面偏窄：建议补充缺口能力或合并职责重叠的工具",
            metrics.tool_count, metrics.distinct_capabilities, metrics.diversity_score
        ));
    }

    if metrics.tool_count >= 2 && tools.iter().all(|tool| tool.source == ToolSource::Mcp) {
        recommendations.push(format!(
            "全部 {} 个工具都来自外部 MCP，缺少本地内置降级路径：建议保留一个本地小工具作为离线/故障备选",
            metrics.tool_count
        ));
    }

    if recommendations.is_empty() {
        recommendations.push(format!(
            "未发现明显菌群失调（diversity {:.2} / dysbiosis {:.2}）：保持现有分工即可",
            metrics.diversity_score, metrics.dysbiosis_score
        ));
    }
    recommendations
}

/// 为单个完全重复组生成建议：按最近使用记录判定主备（只建议，不剪枝）。
fn duplicate_group_advice(tools: &[ToolView], group: &[usize]) -> String {
    let names = group
        .iter()
        .map(|index| tools[*index].name.as_str())
        .collect::<Vec<_>>()
        .join("/");
    let capability_count = tools[group[0]].capabilities.len();
    let primary = group
        .iter()
        .map(|index| &tools[*index])
        .max_by(|left, right| {
            left.last_used_ms
                .cmp(&right.last_used_ms)
                .then_with(|| right.name.cmp(&left.name))
        });
    let Some(primary) = primary else {
        return format!(
            "完全重复：{names} 的能力集合相同（{capability_count} 项能力），建议合并或明确主备分工"
        );
    };
    match primary.last_used_ms {
        Some(last_used_ms) => format!(
            "完全重复：{names} 的能力集合相同（{capability_count} 项能力），建议合并或明确主备：以最近使用的 {}（last_used_ms={last_used_ms}）为主，其余转备份",
            primary.name
        ),
        None => format!(
            "完全重复：{names} 的能力集合相同（{capability_count} 项能力），建议合并为单一工具或明确主备分工（无最近使用记录可判定主备）"
        ),
    }
}

/// 权重消毒：非有限或非正数按 0 处理。
fn sanitize_weight(weight: f32) -> f32 {
    if weight.is_finite() && weight > 0.0 {
        weight
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str, capabilities: &[&str], source: ToolSource) -> ToolProfile {
        ToolProfile {
            name: name.to_owned(),
            capabilities: capabilities
                .iter()
                .map(|capability| (*capability).to_owned())
                .collect(),
            source,
            last_used_ms: None,
        }
    }

    fn profile_at(
        name: &str,
        capabilities: &[&str],
        source: ToolSource,
        last_used_ms: u64,
    ) -> ToolProfile {
        ToolProfile {
            last_used_ms: Some(last_used_ms),
            ..profile(name, capabilities, source)
        }
    }

    fn config(
        overlap_threshold: f32,
        redundancy_tolerance: usize,
        max_pairs: usize,
    ) -> EcosystemConfig {
        EcosystemConfig {
            overlap_threshold,
            redundancy_tolerance,
            max_pairs,
        }
    }

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn empty_ecosystem_reports_all_zeros_without_panicking() {
        let report = analyze(&[], &EcosystemConfig::default());
        assert_eq!(report.tool_count, 0);
        assert_eq!(report.distinct_capabilities, 0);
        close(report.diversity_score, 0.0);
        close(report.redundancy_score, 0.0);
        close(report.dysbiosis_score, 0.0);
        assert!(report.overlap_pairs.is_empty());
        assert_eq!(report.recommendations.len(), 1);
        assert!(report.recommendations[0].contains("空工具集"));
    }

    #[test]
    fn default_weights_document_the_mix_and_normalize_to_one() {
        let weights = EcosystemWeights::default();
        close(weights.diversity, 0.4);
        close(weights.redundancy, 0.2);
        close(weights.overlap, 0.4);
        let normalized = weights.normalized();
        close(
            normalized.diversity + normalized.redundancy + normalized.overlap,
            1.0,
        );

        let scaled = EcosystemWeights::new(2.0, 1.0, 1.0).normalized();
        close(scaled.diversity, 0.5);
        close(scaled.redundancy, 0.25);
        close(scaled.overlap, 0.25);

        let degenerate = EcosystemWeights::new(0.0, 0.0, 0.0).normalized();
        close(degenerate.diversity, 0.4);
        close(degenerate.redundancy, 0.2);
        close(degenerate.overlap, 0.4);

        let poisoned = EcosystemWeights::new(-1.0, f32::NAN, f32::INFINITY).normalized();
        close(poisoned.diversity, 0.4);
        close(poisoned.redundancy, 0.2);
        close(poisoned.overlap, 0.4);
    }

    #[test]
    fn config_defaults_are_documented_and_out_of_range_thresholds_fall_back() {
        let default = EcosystemConfig::default();
        close(default.overlap_threshold, 0.8);
        assert_eq!(default.redundancy_tolerance, 1);
        assert_eq!(default.max_pairs, 32);

        let nan = config(f32::NAN, 0, 0).sanitized();
        close(nan.overlap_threshold, 0.8);
        let too_high = config(1.5, 0, 0).sanitized();
        close(too_high.overlap_threshold, 0.8);
        let negative = config(-0.01, 0, 0).sanitized();
        close(negative.overlap_threshold, 0.8);
        let zero = config(0.0, 0, 0).sanitized();
        close(zero.overlap_threshold, 0.0);
        assert_eq!(zero.max_pairs, 0);
    }

    #[test]
    fn single_tool_never_overlaps_with_itself() {
        let profiles = [profile("solo", &["a", "b"], ToolSource::Local)];
        let report = analyze(&profiles, &EcosystemConfig::default());
        assert_eq!(report.tool_count, 1);
        assert_eq!(report.distinct_capabilities, 2);
        close(report.diversity_score, 1.0);
        close(report.redundancy_score, 0.0);
        close(report.dysbiosis_score, 0.0);
        assert!(
            report.overlap_pairs.is_empty(),
            "a tool must not be paired with itself"
        );
    }

    #[test]
    fn diversity_is_distinct_capability_coverage_per_tool_and_clamps() {
        let disjoint = [
            profile("alpha", &["a"], ToolSource::Builtin),
            profile("beta", &["b"], ToolSource::Builtin),
        ];
        close(
            analyze(&disjoint, &EcosystemConfig::default()).diversity_score,
            1.0,
        );

        let shared = [
            profile("alpha", &["a"], ToolSource::Builtin),
            profile("beta", &["a"], ToolSource::Builtin),
        ];
        close(
            analyze(&shared, &EcosystemConfig::default()).diversity_score,
            0.5,
        );

        let wide = [
            profile("alpha", &["a", "b", "c"], ToolSource::Builtin),
            profile("beta", &[], ToolSource::Builtin),
        ];
        let report = analyze(&wide, &EcosystemConfig::default());
        close(report.diversity_score, 1.0);
        assert_eq!(report.distinct_capabilities, 3);
    }

    #[test]
    fn jaccard_overlap_pairs_use_canonical_left_right_order() {
        let profiles = [
            profile("zebra", &["a", "b"], ToolSource::Mcp),
            profile("alpha", &["b", "c"], ToolSource::Mcp),
        ];
        let report = analyze(&profiles, &config(0.3, 0, 8));
        assert_eq!(report.overlap_pairs.len(), 1);
        let pair = &report.overlap_pairs[0];
        assert_eq!(pair.left, "alpha");
        assert_eq!(pair.right, "zebra");
        close(pair.similarity, 1.0 / 3.0);
    }

    #[test]
    fn overlap_threshold_is_inclusive_at_exactly_0_8() {
        let at_threshold = [
            profile("alpha", &["a", "b", "c", "d"], ToolSource::Builtin),
            profile("beta", &["a", "b", "c", "d", "e"], ToolSource::Builtin),
        ];
        let report = analyze(&at_threshold, &config(0.8, 1, 8));
        assert_eq!(report.overlap_pairs.len(), 1);
        close(report.overlap_pairs[0].similarity, 0.8);

        let below_threshold = [
            profile("alpha", &["a", "b", "c"], ToolSource::Builtin),
            profile("beta", &["a", "b", "d"], ToolSource::Builtin),
        ];
        let report = analyze(&below_threshold, &config(0.8, 1, 8));
        assert!(
            report.overlap_pairs.is_empty(),
            "0.75 is below the 0.8 threshold"
        );
    }

    #[test]
    fn overlap_pairs_sort_by_similarity_then_names() {
        let profiles = [
            profile("c", &["x"], ToolSource::Builtin),
            profile("a", &["x"], ToolSource::Builtin),
            profile("b", &["x"], ToolSource::Builtin),
            profile("d", &["x", "y", "z"], ToolSource::Builtin),
        ];
        let report = analyze(&profiles, &config(0.3, 0, 16));
        let labels: Vec<(&str, &str)> = report
            .overlap_pairs
            .iter()
            .map(|pair| (pair.left.as_str(), pair.right.as_str()))
            .collect();
        assert_eq!(
            labels,
            vec![
                ("a", "b"),
                ("a", "c"),
                ("b", "c"),
                ("a", "d"),
                ("b", "d"),
                ("c", "d"),
            ]
        );
    }

    #[test]
    fn max_pairs_truncates_the_display_but_not_the_scores() {
        let profiles = [
            profile("a", &["x"], ToolSource::Builtin),
            profile("b", &["x"], ToolSource::Builtin),
            profile("c", &["x"], ToolSource::Builtin),
            profile("d", &["x"], ToolSource::Builtin),
        ];
        let wide = analyze(&profiles, &config(0.8, 0, 100));
        let narrow = analyze(&profiles, &config(0.8, 0, 2));
        assert_eq!(wide.overlap_pairs.len(), 6);
        assert_eq!(narrow.overlap_pairs.len(), 2);
        close(narrow.redundancy_score, wide.redundancy_score);
        close(narrow.dysbiosis_score, wide.dysbiosis_score);
        assert_eq!(narrow.overlap_pairs[0].left, "a");
        assert_eq!(narrow.overlap_pairs[0].right, "b");
    }

    #[test]
    fn redundancy_counts_surplus_copies_and_respects_tolerance() {
        let profiles = [
            profile("a", &["x"], ToolSource::Builtin),
            profile("b", &["x"], ToolSource::Builtin),
            profile("c", &["y"], ToolSource::Builtin),
        ];
        close(
            analyze(&profiles, &config(0.8, 0, 8)).redundancy_score,
            1.0 / 3.0,
        );
        close(analyze(&profiles, &config(0.8, 1, 8)).redundancy_score, 0.0);

        let triple = [
            profile("a", &["x"], ToolSource::Builtin),
            profile("b", &["x"], ToolSource::Builtin),
            profile("c", &["x"], ToolSource::Builtin),
            profile("d", &["y"], ToolSource::Builtin),
        ];
        close(
            analyze(&triple, &config(0.8, 0, 8)).redundancy_score,
            2.0 / 4.0,
        );

        let empty_capabilities = [
            profile("a", &[], ToolSource::Builtin),
            profile("b", &["  ", ""], ToolSource::Builtin),
        ];
        close(
            analyze(&empty_capabilities, &config(0.8, 0, 8)).redundancy_score,
            0.0,
        );
    }

    #[test]
    fn dysbiosis_is_weighted_sum_and_stays_in_unit_interval() {
        let duplicate = [
            profile("a", &["a"], ToolSource::Builtin),
            profile("b", &["a"], ToolSource::Builtin),
        ];
        let cfg = config(0.8, 0, 8);
        let report = analyze(&duplicate, &cfg);
        close(report.diversity_score, 0.5);
        close(report.redundancy_score, 0.5);
        // 0.4 * (1 - 0.5) + 0.2 * 0.5 + 0.4 * 1.0 = 0.7
        close(report.dysbiosis_score, 0.7);

        let overlap_only =
            analyze_with_weights(&duplicate, &cfg, &EcosystemWeights::new(0.0, 0.0, 1.0));
        close(overlap_only.dysbiosis_score, 1.0);

        let diversity_only =
            analyze_with_weights(&duplicate, &cfg, &EcosystemWeights::new(1.0, 0.0, 0.0));
        close(diversity_only.dysbiosis_score, 0.5);
    }

    #[test]
    fn every_score_is_finite_and_within_unit_interval() {
        let profiles = [
            profile("read-file", &["fs.read", "fs.stat"], ToolSource::Builtin),
            profile("read-file-alt", &["fs.read", "fs.stat"], ToolSource::Local),
            profile(
                "search",
                &["search.grep", "search.glob"],
                ToolSource::Builtin,
            ),
            profile("mcp-remote", &["search.grep"], ToolSource::Mcp),
            profile("no-capabilities", &[], ToolSource::Mcp),
            profile("overlap", &["fs.read", "search.grep"], ToolSource::Local),
        ];
        let configs = [
            config(0.0, 0, 0),
            config(0.8, 1, 4),
            config(1.0, 3, 100),
            config(f32::NAN, 0, 4),
        ];
        for cfg in configs {
            let report = analyze(&profiles, &cfg);
            for score in [
                report.diversity_score,
                report.redundancy_score,
                report.dysbiosis_score,
            ] {
                assert!(score.is_finite(), "score must be finite, got {score}");
                assert!(
                    (0.0..=1.0).contains(&score),
                    "score must stay in 0..=1, got {score}"
                );
            }
        }
    }

    #[test]
    fn analysis_is_deterministic_under_input_permutation() {
        let profiles = vec![
            profile("alpha", &["a", "b"], ToolSource::Builtin),
            profile("beta", &["b", "c"], ToolSource::Mcp),
            profile("gamma", &["a", "b"], ToolSource::Local),
            profile("delta", &["d"], ToolSource::Builtin),
        ];
        let cfg = config(0.5, 1, 16);
        let forward = analyze(&profiles, &cfg);
        let mut reversed = profiles.clone();
        reversed.reverse();
        let backward = analyze(&reversed, &cfg);
        assert_eq!(forward, backward);
        assert_eq!(forward, analyze(&profiles, &cfg));
    }

    #[test]
    fn recommendations_are_advisory_and_leave_inputs_untouched() {
        let profiles = [
            profile("a", &["x"], ToolSource::Mcp),
            profile("b", &["x"], ToolSource::Mcp),
            profile("c", &["y"], ToolSource::Mcp),
        ];
        let snapshot = profiles.clone();
        let report = analyze(&profiles, &config(0.8, 0, 8));
        assert_eq!(profiles, snapshot, "analysis must not mutate its input");
        assert!(
            report
                .recommendations
                .iter()
                .any(|advice| advice.contains("建议")),
            "a dysbiotic ecosystem should carry advice"
        );
        for advice in &report.recommendations {
            for forbidden in ["删除", "移除", "注销", "卸载"] {
                assert!(
                    !advice.contains(forbidden),
                    "recommendations must never suggest pruning: {advice}"
                );
            }
        }
    }

    #[test]
    fn duplicate_advice_prefers_the_most_recently_used_tool() {
        let mut profiles = [
            profile_at("alpha", &["x"], ToolSource::Builtin, 100),
            profile_at("beta", &["x"], ToolSource::Builtin, 500),
        ];
        let report = analyze(&profiles, &config(0.8, 0, 8));
        let advice = report
            .recommendations
            .iter()
            .find(|advice| advice.contains("完全重复"))
            .expect("duplicate advice");
        assert!(advice.contains("以最近使用的 beta"));
        assert!(advice.contains("last_used_ms=500"));

        profiles[0].last_used_ms = None;
        profiles[1].last_used_ms = None;
        let report = analyze(&profiles, &config(0.8, 0, 8));
        let advice = report
            .recommendations
            .iter()
            .find(|advice| advice.contains("完全重复"))
            .expect("duplicate advice");
        assert!(advice.contains("无最近使用记录"));
    }

    #[test]
    fn all_mcp_ecosystem_suggests_a_local_fallback() {
        let profiles = [
            profile("remote-a", &["a"], ToolSource::Mcp),
            profile("remote-b", &["b"], ToolSource::Mcp),
        ];
        let report = analyze(&profiles, &EcosystemConfig::default());
        assert!(
            report
                .recommendations
                .iter()
                .any(|advice| advice.contains("本地")),
            "an MCP-only ecosystem should suggest a local fallback"
        );
    }

    #[test]
    fn capability_tokens_are_trimmed_deduplicated_and_case_sensitive() {
        let profiles = [
            profile("a", &[" read ", "read", "write"], ToolSource::Builtin),
            profile("b", &["read", "write"], ToolSource::Builtin),
        ];
        let report = analyze(&profiles, &config(0.9, 0, 8));
        assert_eq!(report.distinct_capabilities, 2);
        assert_eq!(report.overlap_pairs.len(), 1);
        close(report.overlap_pairs[0].similarity, 1.0);
        close(report.redundancy_score, 0.5);

        let case_sensitive = [
            profile("a", &["Read"], ToolSource::Builtin),
            profile("b", &["read"], ToolSource::Builtin),
        ];
        let report = analyze(&case_sensitive, &EcosystemConfig::default());
        assert_eq!(report.distinct_capabilities, 2);
        assert!(report.overlap_pairs.is_empty());
    }

    #[test]
    fn report_and_profile_round_trip_through_json() {
        let profiles = [
            profile("remote", &["a", "b"], ToolSource::Mcp),
            profile("local", &["b", "c"], ToolSource::Local),
        ];
        let report = analyze(&profiles, &EcosystemConfig::default());
        let json = serde_json::to_string(&report).expect("serialize report");
        let decoded: EcosystemReport = serde_json::from_str(&json).expect("deserialize report");
        assert_eq!(decoded, report);

        let profile_json = serde_json::to_string(&profiles[0]).expect("serialize profile");
        assert!(profile_json.contains("\"source\":\"mcp\""));
        let decoded_profile: ToolProfile =
            serde_json::from_str(&profile_json).expect("deserialize profile");
        assert_eq!(decoded_profile, profiles[0]);
    }
}

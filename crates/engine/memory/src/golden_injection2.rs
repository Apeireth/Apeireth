//! `apeireth-memory::golden_injection2` — 注入形态对照的**第二轮**（消偏置版）。
//!
//! # 这一轮要消解什么
//!
//! 第一轮（[`crate::golden_injection`]）在《紅樓夢》上跑出了第一张注入形态对照表
//! （`head-120` / `head-400` / `tail-120` / `best-window-160` / `whole`）。
//! 那张表**方向上可信**（同预算下 `head-120` 覆盖最高），
//! 但**自带两个致命偏置** ⇒ 结论**口径上不牢**，必须消解：
//!
//! ## 偏置 ①：期望片段的位置**由挖题器挑的** ⇒ 天然偏前
//!
//! 挖题器的 `fragment_for`（[`crate::golden_mining`]）取的是**第一句**含主体的句子
//! ⇒ 期望片段**系统性地落在正文开头**
//! （第一轮实测：719 个片段 p50 = 第 346 字，只有 38.1% 落在**前 120 字**内）
//! ⇒ **「取开头」的注入形态被构造性占优**
//! ⇒ 「BestWindow 比 Head 差」这个结论**可能只是偏置造成的**。
//!
//! ⇒ 本模块提供 [`resample_fragments_uniformly`]：
//! 把期望片段改成**在正文里均匀采样**（跨整篇正文，不只看开头）
//! ⇒ 片段位置分布**不再偏前** ⇒ 头/尾/中间窗口**公平竞争**。
//!
//! ## 偏置 ②：没固定**总注入预算**
//!
//! 第一轮固定的是「**每条**窗口大小」⇒ 「少条大窗 vs 多条小窗」这个
//! **真正与生产相关的取舍**根本没测（生产关心的是**上下文总占用**）。
//!
//! ⇒ 本模块按**总预算固定**跑对照（[`BudgetPlan`]）：
//! 每条窗口大小 × 条数上限，**总量封顶**。
//!
//! # 口径（别自创）
//!
//! * **检索结果固定**（调用方传入）⇒ 变的只有注入形态与预算分配
//!   ⇒ 否则测的是「检索差异」而不是「注入差异」（**混淆变量**）。
//! * **装入顺序**：`relevance` **降序**（同分按 id 升序 ⇒ 确定性）。
//!   `relevance` 由**检索名次**派生（`1/(rank+1)`，RRF 式）⇒ **输入顺序就是相关度顺序**。
//! * **总量封顶**：装到 `total_chars` 就**停**（不跳过后面的条目去凑）
//!   ⇒ **硬不变式**：`injected_chars ≤ total_chars`（`total_chars = 0` 除外）。
//! * **条数封顶** `max_items`（`max_items = 0` ⇒ 不限条数，与
//!   [`crate::injection_renderer::Budget`] 同口径）。
//! * **预算只计「证据正文」**：出处（「 —— 」+ 记录 id）与换行分隔符**不计入**预算
//!   ⇒ 「10 条小窗」不会被 10 份出处开销吃掉预算 ⇒ 测的是**形态**不是**开销**。
//!   （代价：两轮的「注入字符数」**不是同一把尺子**，覆盖率可比、字符数不可直接相减。）
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、预算对照台、均匀重采样、报告与测试齐备（**库级**实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 **0** 调用点；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：已在**真实语料**（《紅樓夢》120 回）上跑过对照，
//!   但**只有一种语料**、**只有一种检索器** ⇒ 结论的适用范围见交付说明。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::collections::{BTreeMap, BTreeSet};

use crate::golden_injection::{render_record, InjectionStrategy, RetrievedRecord};
use crate::golden_set::{
    covers_fragment, evaluate, EvalBatch, GoldenCase, GoldenSet, Metrics, Retrieved,
};
use crate::injection_renderer::{Budget, BulletedRenderer, Evidence, Injection, InjectionRenderer};

/// **总预算**固定的注入方案（每条窗口大小 × 条数上限，**总量封顶**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetPlan {
    /// 每条证据的窗口大小（字）。
    ///
    /// ⚠️ 这是**窗口正文**的字符数（不含出处与分隔符，见模块文档「口径」）。
    pub per_item_chars: usize,
    /// 最多注入几条。
    ///
    /// `0` = **不限条数**（与 [`crate::injection_renderer::Budget::max_items`] 同口径）。
    pub max_items: usize,
    /// 总字符预算（**硬上限**）。
    ///
    /// `0` = **不限总量**，只受 `per_item_chars` × `max_items` 约束。
    pub total_chars: usize,
}

impl BudgetPlan {
    /// 构造（`total_chars = 0` ⇒ 不限总量，只受 `per_item_chars` × `max_items` 约束）。
    pub fn new(per_item_chars: usize, max_items: usize, total_chars: usize) -> Self {
        Self {
            per_item_chars,
            max_items,
            total_chars,
        }
    }
}

/// 一轮「**总预算固定**」的对照结果。
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetOutcome {
    /// 方案（每条字数 × 条数上限 × 总预算）。
    pub plan: BudgetPlan,
    /// 策略 id（`head` / `tail` / `best-window`）。
    pub strategy_id: String,
    /// 卷子指标（recall / nDCG / 证据族）。
    ///
    /// ⚠️ **recall 与 nDCG 与注入预算无关**（检索结果固定）⇒
    /// 它们在对照表里应当**逐字相同**；若不同 ⇒ 说明有人偷偷换了检索结果。
    pub metrics: Metrics,
    /// 逐片段覆盖率（同 W-12 的口径：分母 = 全部期望片段数）。
    pub fragment_coverage: f64,
    /// **实际**注入总字符数（**必须 ≤ `plan.total_chars`** —— 除非 `total_chars = 0`）。
    ///
    /// # 口径（逐条钉死）
    ///
    /// * **每题独立预算**：生产里每道题的注入各占一份上下文 ⇒
    ///   本字段是**逐题注入字符数的最大值**（= 最坏情况）⇒ 恒 ≤ `plan.total_chars`；
    /// * 只计**证据正文**（窗口文本）⇒ 不含出处与换行分隔符
    ///   ⇒ 「10 条」方案在生产里还要另付约 `10 × 15` 字的出处开销
    ///   （见交付说明：这条开销**偏向多条方案**，读表时必须一起算）。
    pub injected_chars: usize,
    /// 实际注入条数。
    ///
    /// 同 `injected_chars`：**逐题口径的最大值** ⇒ 恒 ≤ `plan.max_items`。
    pub injected_items: usize,
}

impl BudgetOutcome {
    /// 方案标签（人话 + **可排序** ⇒ 并列时按它定序）。
    ///
    /// 形如 `head-120x10@1200`（策略 · 每条字数 x 条数上限 @ 总预算）。
    pub fn label(&self) -> String {
        format!(
            "{}-{}x{}@{}",
            self.strategy_id, self.plan.per_item_chars, self.plan.max_items, self.plan.total_chars
        )
    }
}

/// 按**总预算固定**跑一轮对照。
///
/// # 口径
///
/// * **检索结果固定**（调用方传入）⇒ 变的只有注入形态与预算分配；
/// * 装入顺序：**按 `relevance` 降序**（同分按 id 升序 ⇒ 确定性）；
/// * **总量封顶**：装到 `total_chars` 就停（**硬不变式**：`injected_chars <= total_chars`）；
/// * 条数封顶 `max_items`；
/// * `total_chars = 0` ⇒ 只受「每条 × 条数」约束。
///
/// # 每个方案跑三种窗口形态
///
/// `head` / `tail` / `best-window`（**同一批检索结果**、**同一个渲染器**）
/// ⇒ 同一方案下三行只差「窗口从哪儿取」⇒ 变的是**形态**，不是预算。
///
/// # 返回顺序
///
/// **方案优先、形态次之**（`plans` 的顺序 × `head`→`tail`→`best-window`）⇒ 确定性。
///
/// # 空输入的口径
///
/// * **空卷**（`golden.cases` 为空）、**空检索结果**（`retrieved` 为空）
///   或**空方案表** ⇒ 返回**空 Vec**（**没有可比较的对象**）⇒ [`best_plan`] 因此返回 `None`；
/// * `retrieved` 比 `golden.cases` **短** ⇒ 缺的题按「什么也没检索到」算（分数 0），
///   **不截断卷子**（截断会让两批分数的分母不同 ⇒ 不可比）。
pub fn compare_under_budget(
    golden: &GoldenSet,
    retrieved: &[Vec<RetrievedRecord>],
    plans: &[BudgetPlan],
) -> Vec<BudgetOutcome> {
    if golden.cases.is_empty() || retrieved.is_empty() || plans.is_empty() {
        return Vec::new();
    }
    let mut outcomes = Vec::with_capacity(plans.len() * WindowStrategy::ALL.len());
    for plan in plans {
        for strategy in WindowStrategy::ALL {
            outcomes.push(run_plan(golden, retrieved, *plan, strategy));
        }
    }
    outcomes
}

/// **均匀采样**的期望片段（**消解偏置 ①**）。
///
/// # 为什么（这是本批最重要的东西）
///
/// 挖题器的 `fragment_for` 取**第一句**含主体的句子 ⇒ 期望片段**系统性偏前**
/// ⇒ 「取开头」的注入策略被**构造性占优** ⇒ 对照结论不可信。
///
/// ⇒ 本函数从**正文的均匀位置**采样（**跨整篇正文**，不只看开头）
/// ⇒ 期望片段的位置分布**不再偏前** ⇒ 头/尾/中间窗口**公平竞争**。
///
/// # 口径
///
/// * 对每条记录：把正文按字符数切成 N 段（N = [`UNIFORM_BUCKETS`]），
///   在**每段里**取第一句含主体的句子（若该段没有 ⇒ 取该段的**首句**）；
/// * **确定性**：按段序 ⇒ 同输入同输出；
/// * 若正文太短（放不下 N 段）⇒ 退化为**按句均匀取 N 句**。
///
/// # 「主体」怎么定
///
/// 题面里的**实词**（[`crate::golden_mining::tokenize`] 的 token）
/// ∩ **原期望片段**（原片段就是围绕主体裁的 ⇒ 交集里最长的那个就是主体）
/// ⇒ 与挖题器「主体来自正文里的实体」同口径；
/// 原片段缺失时退化为「题面 token ∩ 正文」。
///
/// # 片段长度
///
/// 沿用挖题器的 [`crate::golden_mining::FRAGMENT_MAX_CHARS`]（60 字）
/// ⇒ 两轮的片段**长度分布**一致 ⇒ 覆盖率可比。
pub const UNIFORM_BUCKETS: usize = 8;

/// 用**均匀采样**重建一份考卷的期望片段（返回新卷子；**不改原卷**）。
///
/// ⚠️ **只改 `expect_fragments`**（`expect_ids` / `query` / `tags` 原样保留）。
///
/// # 口径
///
/// * **一条记录 ⇒ 多条片段**（每个位置桶一条）⇒ 片段数量变多
///   ⇒ **覆盖率的分子分母同时变大** ⇒ 仍是可比的比率；
/// * **找不到对应记录**（记录不在 `records` 里）⇒ **保留原片段**
///   （不丢标注；该题仍按原样参与评测）；
/// * **版本号**加 `-uniform` 后缀（改题必须换版本 ⇒ 机械保证）。
pub fn resample_fragments_uniformly(golden: &GoldenSet, records: &[RetrievedRecord]) -> GoldenSet {
    let mut bodies: BTreeMap<&str, &str> = BTreeMap::new();
    for record in records {
        bodies.insert(record.id.as_str(), record.content.as_str());
    }
    let cases = golden
        .cases
        .iter()
        .map(|case| {
            let mut case = case.clone();
            let mut fragments: Vec<String> = Vec::new();
            for (position, id) in case.expect_ids.iter().enumerate() {
                let anchor = case.expect_fragments.get(position).map(String::as_str);
                match bodies.get(id.as_str()) {
                    Some(body) => {
                        let sampled = sample_uniform_fragments(body, &case.query, anchor);
                        if sampled.is_empty() {
                            // 采样不出来（正文是空的）⇒ 保留原片段（不丢标注）。
                            if let Some(anchor) = anchor {
                                fragments.push(anchor.to_string());
                            }
                        } else {
                            fragments.extend(sampled);
                        }
                    }
                    None => {
                        if let Some(anchor) = anchor {
                            fragments.push(anchor.to_string());
                        }
                    }
                }
            }
            case.expect_fragments = fragments;
            case
        })
        .collect();
    GoldenSet {
        version: format!("{}-uniform", golden.version),
        cases,
    }
}

/// 对照表（人话）。
///
/// # 口径
///
/// * 第一行是**表头**（含列名）；
/// * 数值固定 **4 位小数**（可逐位比对，不因平台而变）；
/// * **空输入 ⇒ 空串**（没有可比的行 ⇒ 不画表头）。
pub fn render_budget_table(outcomes: &[BudgetOutcome]) -> String {
    if outcomes.is_empty() {
        return String::new();
    }
    let width = outcomes
        .iter()
        .map(|outcome| outcome.label().chars().count())
        .max()
        .unwrap_or(8)
        .max(8);
    let mut lines = Vec::with_capacity(outcomes.len() + 1);
    lines.push(format!(
        "{:<width$}  {:>10}  {:>10}  {:>12}  {:>8}",
        "方案", "recall@k", "coverage", "注入字符数", "注入条数"
    ));
    for outcome in outcomes {
        lines.push(format!(
            "{:<width$}  {:>10.4}  {:>10.4}  {:>12}  {:>8}",
            outcome.label(),
            outcome.metrics.recall_at_k,
            outcome.fragment_coverage,
            outcome.injected_chars,
            outcome.injected_items
        ));
    }
    lines.join("\n")
}

/// 汇总：**哪套「每条 × 条数」在同总预算下覆盖最好**。
///
/// # 口径
///
/// * 比的是 [`BudgetOutcome::fragment_coverage`]（**逐片段**口径 ⇒ 跨方案可比）；
/// * **并列**（覆盖率**逐字相等**）⇒ 取**标签序最小**的（确定性，不看输入顺序）；
/// * 返回的是**胜出方案的标签**（`策略-每条字数x条数@总预算`），
///   因为同一套预算下三种形态可能各赢各的 ⇒ 标签能唯一定位；
/// * 空输入 ⇒ `None`。
pub fn best_plan(outcomes: &[BudgetOutcome]) -> Option<String> {
    let mut best: Option<&BudgetOutcome> = None;
    for outcome in outcomes {
        best = Some(match best {
            None => outcome,
            Some(current) => {
                if outcome.fragment_coverage > current.fragment_coverage
                    || (outcome.fragment_coverage == current.fragment_coverage
                        && outcome.label() < current.label())
                {
                    outcome
                } else {
                    current
                }
            }
        });
    }
    best.map(|outcome| outcome.label())
}

// ============================================
// 内部：窗口形态
// ============================================

/// 窗口形态（**窗口从正文的哪儿取**）。
///
/// # 为什么自己定一个枚举而不直接用 [`InjectionStrategy`]
///
/// 第一轮的 [`InjectionStrategy`] 把「形态」和「窗口大小」**焊在一起**
/// （`Head { chars }`）⇒ 本轮要把**窗口大小**提到方案层（[`BudgetPlan`]）
/// ⇒ 这里只留「从哪儿取」，大小由方案给。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowStrategy {
    /// 取正文**前 N 字**（生产口径）。
    Head,
    /// 取正文**后 N 字**（对照）。
    Tail,
    /// 取**与题面词重叠最多**的那一段（对照口径）。
    BestWindow,
}

impl WindowStrategy {
    /// 全部形态（**顺序固定** ⇒ 输出顺序确定）。
    const ALL: [WindowStrategy; 3] = [Self::Head, Self::Tail, Self::BestWindow];

    /// 策略 id（`head` / `tail` / `best-window`）。
    fn id(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Tail => "tail",
            Self::BestWindow => "best-window",
        }
    }

    /// 换成第一轮的窗口实现（**逐字同构** ⇒ 两轮的窗口可比）。
    fn injection(self, chars: usize) -> InjectionStrategy {
        match self {
            Self::Head => InjectionStrategy::Head { chars },
            Self::Tail => InjectionStrategy::Tail { chars },
            Self::BestWindow => InjectionStrategy::BestWindow { chars },
        }
    }
}

/// 名次派生的相关度（**单调递减** ⇒ 装配顺序 = 检索顺序）。
fn rank_relevance(rank: usize) -> f64 {
    1.0 / (rank as f64 + 1.0)
}

/// 装入顺序：**`relevance` 降序**（同分按 **id 升序**）⇒ 确定性。
///
/// # 为什么同分按 id 升序
///
/// 派生口径下 `relevance` 随名次**严格递减**（不会同分）⇒ 这条规则当前是**兜底**；
/// 但保留它 ⇒ 排序是**全序** ⇒ 万一将来 `relevance` 变成外部给定，也不会出现
/// 「同分条目顺序随输入抖动」的不确定性。
fn order_records(records: &[RetrievedRecord]) -> Vec<&RetrievedRecord> {
    let mut ranked: Vec<(f64, &str, &RetrievedRecord)> = records
        .iter()
        .enumerate()
        .map(|(rank, record)| (rank_relevance(rank), record.id.as_str(), record))
        .collect();
    ranked.sort_by(|left, right| right.0.total_cmp(&left.0).then_with(|| left.1.cmp(right.1)));
    ranked.into_iter().map(|(_, _, record)| record).collect()
}

// ============================================
// 内部：跑一个方案
// ============================================

/// 跑一个方案（一种形态 + 一套预算）⇒ 一行结果。
fn run_plan(
    golden: &GoldenSet,
    retrieved: &[Vec<RetrievedRecord>],
    plan: BudgetPlan,
    strategy: WindowStrategy,
) -> BudgetOutcome {
    let mut batch = EvalBatch::default();
    let mut fragments_per_case: Vec<Vec<String>> = Vec::with_capacity(golden.cases.len());
    let mut max_chars = 0usize;
    let mut max_items = 0usize;

    for (index, case) in golden.cases.iter().enumerate() {
        // 检索结果**短于卷子** ⇒ 缺的题按「什么也没检索到」算（不截断卷子）。
        let records: &[RetrievedRecord] = retrieved.get(index).map(Vec::as_slice).unwrap_or(&[]);
        let (fragments, chars, items) = render_case(strategy, plan, case, records);
        max_chars = max_chars.max(chars);
        max_items = max_items.max(items);
        fragments_per_case.push(fragments.clone());
        batch.cases.push(case.clone());
        // ⚠️ **ids 必须是检索顺序**（recall / nDCG 按位置折损）
        //    ⇒ 它们**不随注入预算变** ⇒ recall 在对照表里逐字相同（检索固定的证据）。
        //    `fragments` 是**真正被注入**的条目（预算不够时可能少于 ids）。
        batch.results.push(Retrieved {
            ids: records.iter().map(|record| record.id.clone()).collect(),
            fragments,
        });
    }

    // `evaluate` 的错误分支（cases 与 results 长度不等）**不可达**
    // （本函数保证两者等长）⇒ 退回默认指标（**不 panic**，也不假装成功）。
    let metrics = evaluate(&batch, eval_k(retrieved)).unwrap_or_default();
    let fragment_coverage = fragment_coverage(&golden.cases, &fragments_per_case);
    BudgetOutcome {
        plan,
        strategy_id: strategy.id().to_string(),
        metrics,
        fragment_coverage,
        injected_chars: max_chars,
        injected_items: max_items,
    }
}

/// 一题的注入：按方案选窗口 ⇒ 交给渲染器装配 ⇒ (注入条目, 证据正文字数, 条数)。
///
/// # 为什么走渲染器
///
/// 渲染器是**注入契约**的实现 ⇒ 本模块**不重造**一套；
/// 且所有形态用**同一个**渲染器 ⇒ 变的只有**窗口**，不是形态标签。
///
/// # 预算怎么算
///
/// * **只计证据正文**（窗口文本的字符数）⇒ 硬不变式
///   `injected_chars ≤ total_chars` 在**选条目**那一步就成立；
/// * 渲染时给渲染器的预算是**装配后的确切长度** ⇒ **不会触发截断**
///   ⇒ 每条注入的窗口**就是**方案给的那个大小（最后一条也不例外）。
fn render_case(
    strategy: WindowStrategy,
    plan: BudgetPlan,
    case: &GoldenCase,
    records: &[RetrievedRecord],
) -> (Vec<String>, usize, usize) {
    let ordered = order_records(records);
    let mut selected: Vec<(&RetrievedRecord, String)> = Vec::new();
    let mut used = 0usize;
    for record in ordered {
        // 条数封顶（`max_items = 0` ⇒ 不限条数）。
        if plan.max_items > 0 && selected.len() >= plan.max_items {
            break;
        }
        let text = render_record(strategy.injection(plan.per_item_chars), record, &case.query);
        if text.is_empty() {
            // 空窗口不入产物（也不占预算）。
            continue;
        }
        let cost = text.chars().count();
        if plan.total_chars > 0 && used + cost > plan.total_chars {
            // 总量封顶 ⇒ **停**（不跳过后面的条目去凑预算 ⇒ 装入顺序 = 相关度顺序）。
            break;
        }
        used += cost;
        selected.push((record, text));
    }

    let evidence: Vec<Evidence> = selected
        .iter()
        .enumerate()
        .map(|(rank, (record, text))| {
            Evidence::new(
                record.id.clone(),
                text.clone(),
                record.id.clone(),
                rank_relevance(rank),
            )
        })
        .collect();
    // 渲染器要的是**装配后的确切长度** ⇒ 预算给足 ⇒ 不会截断任何一条。
    let assembled = assemble_cost(&evidence);
    let injection = BulletedRenderer.render(&evidence, Budget::new(assembled, 0));
    (fragments_of(&injection), used, selected.len())
}

/// 渲染器把一条证据写成 `内容 —— 出处` ⇒ 行长 = 正文 + 分隔符 + 出处。
const ATTRIBUTION_SEPARATOR: &str = " —— ";

/// 装配后产物的**确切长度**（字）：各行长度之和 + 行间换行分隔符。
///
/// # 为什么自己算而不是让渲染器截断
///
/// 渲染器在「装不下」时会**截断最后一条**并标注「…（已截断）」
/// ⇒ 那样注入的窗口就不是方案给的大小了 ⇒ 测的就不再是「每条 × 条数」了。
/// ⇒ 自己算好确切长度喂给它 ⇒ **一条都不截断**（要么整条装，要么不装）。
fn assemble_cost(evidence: &[Evidence]) -> usize {
    let lines: usize = evidence
        .iter()
        .map(|item| {
            item.text.chars().count()
                + ATTRIBUTION_SEPARATOR.chars().count()
                + item.attribution.chars().count()
        })
        .sum();
    lines + evidence.len().saturating_sub(1)
}

/// 注入产物 ⇒ 证据条目列表（喂给评测的**片段**）。
fn fragments_of(injection: &Injection) -> Vec<String> {
    match injection {
        Injection::Plain { text } => text.lines().map(str::to_string).collect(),
        Injection::Bulleted { lines } => lines.clone(),
        Injection::Conclusions { items } => items.clone(),
        Injection::Empty { .. } => Vec::new(),
    }
}

/// 评测用的 top-k：**按传入的检索结果条数自适应**（各题最大条数，至少 1）。
fn eval_k(retrieved: &[Vec<RetrievedRecord>]) -> usize {
    retrieved
        .iter()
        .map(|records| records.len())
        .max()
        .unwrap_or(0)
        .max(1)
}

/// **逐片段覆盖率**：分母 = 全部期望片段数，分子 = 被覆盖的期望片段数。
///
/// # 口径（与 W-12 逐字同构 ⇒ 两轮可比）
///
/// * 期望片段**先 trim 再去空**（与卷子的证据族口径一致）⇒ 空片段不占分母；
/// * 覆盖判定 = [`covers_fragment`]（**同一把尺子**）；
/// * **跨题累加**（不是逐题平均后再平均）⇒ 分母与方案**无关** ⇒ 跨方案可比。
fn fragment_coverage(cases: &[GoldenCase], fragments_per_case: &[Vec<String>]) -> f64 {
    let mut expected_total = 0usize;
    let mut covered_total = 0usize;
    for (case, fragments) in cases.iter().zip(fragments_per_case.iter()) {
        for expected in case
            .expect_fragments
            .iter()
            .map(|fragment| fragment.trim())
            .filter(|fragment| !fragment.is_empty())
        {
            expected_total += 1;
            if fragments
                .iter()
                .any(|injected| covers_fragment(injected, expected))
            {
                covered_total += 1;
            }
        }
    }
    if expected_total == 0 {
        0.0
    } else {
        covered_total as f64 / expected_total as f64
    }
}

// ============================================
// 内部：均匀采样
// ============================================

/// 一句话（带**字符下标**，用来定位它在正文的位置）。
struct Sentence {
    /// 句子正文（去首尾空白 ⇒ 仍是正文的**子串**）。
    text: String,
    /// 句子在正文里的起始**字符**下标。
    start: usize,
}

/// 切句：按句号/问号/叹号/分号/换行切（**与挖题器同口径** ⇒ 采样句与挖题句对齐）。
fn split_sentences(content: &str) -> Vec<Sentence> {
    let mut out: Vec<Sentence> = Vec::new();
    let mut buffer = String::new();
    let mut pending_start = 0usize;
    let mut index = 0usize;
    for ch in content.chars() {
        if is_sentence_break(ch) {
            push_sentence(&mut out, &buffer, pending_start);
            buffer.clear();
            index += 1;
            pending_start = index;
        } else {
            buffer.push(ch);
            index += 1;
        }
    }
    push_sentence(&mut out, &buffer, pending_start);
    out
}

/// 把一段缓冲收成一句（记下它在正文里的起始字符下标）。
fn push_sentence(out: &mut Vec<Sentence>, buffer: &str, pending_start: usize) {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return;
    }
    let lead = buffer.chars().take_while(|ch| ch.is_whitespace()).count();
    let start = pending_start + lead;
    out.push(Sentence {
        text: trimmed.to_string(),
        start,
    });
}

/// 句子边界字符（**与挖题器同口径**）。
fn is_sentence_break(ch: char) -> bool {
    matches!(
        ch,
        '。' | '！' | '？' | '!' | '?' | '；' | ';' | '\n' | '\r'
    )
}

/// 从正文里**均匀采样**出若干片段（**消解偏置 ①** 的核心）。
///
/// # 口径
///
/// * 正文够长（≥ [`UNIFORM_BUCKETS`] 段且句子数 ≥ 段数）⇒
///   **按字符位置分段**，每段取「第一句含主体的句子」（没有 ⇒ 该段首句）；
/// * 正文太短 ⇒ **退化为按句均匀取**（等距下标，最多 [`UNIFORM_BUCKETS`] 句）；
/// * **片段去重**（同一句被多段选中 ⇒ 只留一次 ⇒ 分母不重复计数）。
fn sample_uniform_fragments(body: &str, query: &str, anchor: Option<&str>) -> Vec<String> {
    let sentences = split_sentences(body);
    if sentences.is_empty() {
        return Vec::new();
    }
    let terms = subject_terms(query, body, anchor);
    let mut fragments: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    let total = body.chars().count();
    if total < UNIFORM_BUCKETS || sentences.len() < UNIFORM_BUCKETS {
        // 退化路径：按句均匀取（等距下标；句数 ≤ 段数时就是全部句子）。
        let take = sentences.len().min(UNIFORM_BUCKETS);
        for step in 0..take {
            let index = if take <= 1 {
                0
            } else {
                step * (sentences.len() - 1) / (take - 1)
            };
            let sentence = &sentences[index];
            push_fragment(&mut fragments, &mut seen, &sentence.text, &terms);
        }
        return fragments;
    }

    // 正常路径：按字符位置切段 ⇒ 每段取一句 ⇒ 片段位置**跨遍正文**。
    for bucket in 0..UNIFORM_BUCKETS {
        let start = total * bucket / UNIFORM_BUCKETS;
        let end = total * (bucket + 1) / UNIFORM_BUCKETS;
        let mut chosen: Option<&Sentence> = None;
        let mut first: Option<&Sentence> = None;
        for sentence in &sentences {
            let sentence_end = sentence.start + sentence.text.chars().count();
            if sentence.start < end && sentence_end > start {
                if first.is_none() {
                    first = Some(sentence);
                }
                if chosen.is_none() && contains_any(&sentence.text, &terms) {
                    chosen = Some(sentence);
                }
            }
        }
        let Some(sentence) = chosen.or(first) else {
            continue;
        };
        push_fragment(&mut fragments, &mut seen, &sentence.text, &terms);
    }
    fragments
}

/// 采到一句 ⇒ 裁成片段（**去重**后入列）。
fn push_fragment(
    fragments: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    sentence: &str,
    terms: &[String],
) {
    let fragment = clip_around_any(sentence, terms, crate::golden_mining::FRAGMENT_MAX_CHARS);
    if fragment.is_empty() {
        return;
    }
    if seen.insert(fragment.clone()) {
        fragments.push(fragment);
    }
}

/// 「主体」候选：题面实词 ∩ **原期望片段**（原片段就是围绕主体裁的）。
///
/// 原片段缺失时退化为「题面实词 ∩ 正文」。
fn subject_terms(query: &str, body: &str, anchor: Option<&str>) -> Vec<String> {
    let tokens: Vec<String> = crate::golden_mining::tokenize(query)
        .into_iter()
        .filter(|token| token.chars().count() >= 2)
        .collect();
    let mut out: Vec<String> = Vec::new();
    if let Some(anchor) = anchor {
        for token in &tokens {
            if anchor.contains(token.as_str()) {
                out.push(token.clone());
            }
        }
    }
    if out.is_empty() {
        for token in &tokens {
            if body.contains(token.as_str()) {
                out.push(token.clone());
            }
        }
    }
    out
}

/// 句子里有没有任一主体词。
fn contains_any(sentence: &str, terms: &[String]) -> bool {
    terms.iter().any(|term| sentence.contains(term.as_str()))
}

/// 围绕主体裁一段（≤ `max_chars` 字；**与挖题器的 `clip_around` 同口径**）。
///
/// 主体取**最长**的那个（并列按字典序）⇒ 裁到人名/术语上，而不是碎片 bigram。
fn clip_around_any(sentence: &str, terms: &[String], max_chars: usize) -> String {
    let mut best: Option<&str> = None;
    for term in terms {
        if !sentence.contains(term.as_str()) {
            continue;
        }
        match best {
            Some(current) => {
                if term.chars().count() > current.chars().count()
                    || (term.chars().count() == current.chars().count() && term.as_str() < current)
                {
                    best = Some(term.as_str());
                }
            }
            None => best = Some(term.as_str()),
        }
    }
    clip_around(sentence, best, max_chars)
}

/// 截断到 ≤ `max_chars` 字，并尽量让 `term` 落在窗口里。
fn clip_around(sentence: &str, term: Option<&str>, max_chars: usize) -> String {
    let chars: Vec<char> = sentence.chars().collect();
    if chars.len() <= max_chars {
        return sentence.to_string();
    }
    let term_pos = term.and_then(|term| find_char_pos(sentence, term));
    let start = match term_pos {
        Some(pos) => pos
            .saturating_sub(15)
            .min(chars.len().saturating_sub(max_chars)),
        None => 0,
    };
    let mut end = (start + max_chars).min(chars.len());
    if let (Some(pos), Some(term)) = (term_pos, term) {
        let term_end = pos + term.chars().count();
        if end < term_end {
            end = term_end.min(chars.len());
        }
    }
    if end - start > max_chars {
        // 词本身超长 ⇒ 退化为前缀截断（仍 ≤ max_chars）。
        return chars[..max_chars].iter().collect();
    }
    chars[start..end].iter().collect()
}

/// 词在句子里首次出现的**字符下标**。
fn find_char_pos(sentence: &str, term: &str) -> Option<usize> {
    let byte_pos = sentence.find(term)?;
    Some(sentence[..byte_pos].chars().count())
}

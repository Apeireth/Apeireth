//! `apeireth-memory::golden_injection` — 注入策略 × 评测的**实验台**。
//!
//! # 为什么要有这个模块
//!
//! 内部卷已经能出分数，但**证据族指标测不出注入策略的差异**：
//! 期望片段在正文的位置**偏后**（《紅樓夢》实测：719 个片段 p50 = 第 346 字，
//! 只有 38.1% 落在**前 120 字**内）⇒ 生产注入的**前 120 字**够不着。
//!
//! ⇒ 本模块把「**注入策略**」变成**可枚举、可比较的变量**，
//! 并用**同一个卷子 + 同一个检索结果**跑一遍 ⇒ 得到一张**对照表**。
//!
//! # 关键纪律
//!
//! * **检索结果必须固定**（同一批 `Retrieved`）⇒ 变的只有注入形态
//!   ⇒ 否则测的是「检索差异」而不是「注入差异」（**混淆变量**）。
//! * 结论要**如实**：若某个形态的分数**不高于**基线 ⇒ 就说它不更好。
//!
//! # 三个口径（别自创）
//!
//! * [`StrategyOutcome::fragment_coverage`]：**逐片段**覆盖率
//!   （分母 = 期望片段总数）⇒ **跨策略可比**（不受「题级 / 片段级」分母差异影响）
//!   ⇒ 这是本模块的**主指标**；
//! * [`StrategyOutcome::metrics`]：卷子自带的 [`Metrics`]（recall / nDCG / 证据族）
//!   ⇒ 其中 recall / nDCG **与注入形态无关**（检索固定）⇒ 只有证据族两项会动；
//! * [`StrategyOutcome::injected_chars`]：**实际注入**的字符数（装配后产物的长度）
//!   ⇒ 用来看「覆盖是靠**更多字**换来的，还是靠**更准的窗口**换来的」。
//!
//! # 注入形态怎么变成「可比较的变量」
//!
//! * 每条命中记录按策略变成一段**注入文本**（窗口）；
//! * 这些窗口交给**同一个渲染器**（[`crate::injection_renderer::BulletedRenderer`]）
//!   装配成注入产物 ⇒ **形态标签固定**（所有策略一样）⇒ 变的只有**窗口**；
//! * 预算按「证据字数 + 出处字数 + 装配余量」给 ⇒ **装得下**（不因预算被截断）
//!   ⇒ 变的不是预算，是窗口；
//! * 装配后的条目就是喂给评测的**证据片段** ⇒ **测的就是真正被注入的东西**。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、策略枚举、对照台、报告与测试齐备（**库级**实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 **0** 调用点；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：已在**真实语料**（《紅樓夢》120 回）上跑过对照，
//!   但**只有一种语料**、**只有一种检索器** ⇒ 结论的适用范围见交付说明。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::collections::BTreeSet;

use crate::golden_set::{
    covers_fragment, evaluate, EvalBatch, GoldenCase, GoldenSet, Metrics, Retrieved,
};
use crate::injection_renderer::{Budget, BulletedRenderer, Evidence, Injection, InjectionRenderer};

/// 一种**注入策略**（怎么把一条记录变成「注入给模型的证据」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionStrategy {
    /// 生产口径：取正文**前 N 字**（与 `memory_injection::EVIDENCE_MAX_CHARS` 同构）。
    Head { chars: usize },
    /// **尾部**：取正文**后 N 字**（对照：证明「前 N 字」不是唯一选择）。
    Tail { chars: usize },
    /// **最相关窗口**：按查询词的重叠选窗口（对照口径）。
    BestWindow { chars: usize },
    /// **整条**：不截断（上限 = 整条正文）⇒ 检验「窗口大小是不是主因」。
    Whole,
}

impl InjectionStrategy {
    /// 策略 id（用于报告与确定性排序）。
    ///
    /// # 口径
    ///
    /// * 纯小写 + 短横线 ⇒ **字典序 = 报告里的行序**（人读得懂、机器也能排）；
    /// * 数字**不补零**（`head-120` 而不是 `head-0120`）⇒ 与人写的一致。
    pub fn id(&self) -> String {
        match self {
            Self::Head { chars } => format!("head-{chars}"),
            Self::Tail { chars } => format!("tail-{chars}"),
            Self::BestWindow { chars } => format!("best-window-{chars}"),
            Self::Whole => "whole".to_string(),
        }
    }
}

/// 一条待注入的记录（检索命中的）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievedRecord {
    /// 记录 id（与考卷里的 `expect_ids` 对得上）。
    pub id: String,
    /// 记录正文（窗口从它里面切）。
    pub content: String,
}

/// 一次策略对照的**一行结果**。
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyOutcome {
    /// 策略 id（= [`InjectionStrategy::id`]）。
    pub strategy_id: String,
    /// 卷子指标（recall / nDCG / 证据族）。
    ///
    /// ⚠️ **recall 与 nDCG 与注入形态无关**（检索结果固定）⇒
    /// 它们在对照表里应当**逐字相同**；若不同 ⇒ 说明有人偷偷换了检索结果。
    pub metrics: Metrics,
    /// **期望片段被覆盖的比例**（0.0 ~ 1.0）⇒ 「注入的证据里有多少能命中期望片段」。
    ///
    /// ⚠️ **这是本模块的核心指标**：它是**逐片段**的（不受「题级/片段级」分母差异影响）
    /// ⇒ 可跨策略直接比较。
    ///
    /// # 口径
    ///
    /// 分母 = **全部期望片段数**（跨题累加）；分子 = **被至少一条注入证据覆盖**的期望片段数。
    /// 覆盖判定用 [`crate::golden_set::covers_fragment`]（与卷子证据族**同一把尺子**）。
    pub fragment_coverage: f64,
    /// 实际注入的总字符数（装配后产物的长度之和）。
    pub injected_chars: usize,
}

/// 跑一次策略对照。
///
/// # 口径
///
/// * **检索结果固定**：调用方传入 `retrieved`（每题的命中记录）⇒ 本函数**不检索**；
/// * 对每种策略：① 按策略把每条记录变成「注入文本」；
///   ② 算 `fragment_coverage`（每条期望片段是否被**覆盖**⇒ 覆盖口径用
///   [`crate::golden_set::covers_fragment`]）；③ 组装 `EvalBatch` 跑
///   [`crate::golden_set::evaluate`] 得 [`Metrics`]；
/// * **预算**：`Budget::new(total_chars, 0)`（不限条数）⇒ 变的是**形态**不是预算。
///
/// # 必须保证
///
/// * **确定性**：同一输入两次 ⇒ 逐字相等；
/// * **不 panic**：空卷、空检索结果 ⇒ 返回空结果（不炸）。
///
/// # 空输入的口径
///
/// * **空卷**（`golden.cases` 为空）或**空检索结果**（`retrieved` 为空）⇒ 返回**空 Vec**
///   （**没有可比较的对象**）⇒ [`best_strategy`] 因此返回 `None`；
/// * `retrieved` 比 `golden.cases` **短** ⇒ 缺的题按「什么也没检索到」算（分数 0），
///   **不截断卷子**（截断会让两批分数的分母不同 ⇒ 不可比）。
pub fn compare_strategies(
    golden: &GoldenSet,
    retrieved: &[Vec<RetrievedRecord>],
    strategies: &[InjectionStrategy],
) -> Vec<StrategyOutcome> {
    if golden.cases.is_empty() || retrieved.is_empty() {
        return Vec::new();
    }
    strategies
        .iter()
        .map(|strategy| {
            let (metrics, fragment_coverage, injected_chars) =
                run_strategy(golden, retrieved, *strategy);
            StrategyOutcome {
                strategy_id: strategy.id(),
                metrics,
                fragment_coverage,
                injected_chars,
            }
        })
        .collect()
}

/// 把一条记录按策略变成「注入文本」。
///
/// # 口径
///
/// * [`InjectionStrategy::Head`]：正文**前 N 字**（与生产逐字同构：按字符切，不切坏 UTF-8）；
/// * [`InjectionStrategy::Tail`]：正文**后 N 字**；
/// * [`InjectionStrategy::BestWindow`]：**重叠滑窗**里与**查询词重叠最多**的那一段
///   （步长 = 窗口的一半；并列取**靠前**的 ⇒ 确定性）；
/// * [`InjectionStrategy::Whole`]：**整条正文**。
///
/// # 为什么「最相关窗口」按 **bigram 重叠**算
///
/// 分词用 [`crate::golden_mining::tokenize`]（CJK bigram + ASCII 词）——
/// 与**挖题器**同一个分词器 ⇒ 查询里的**实词**与正文里的词**同口径**
/// ⇒ 窗口选择才看得到「查询说的是哪一段」。
///
/// ⚠️ 若改用「单字」口径（`unicode_tokens` 对中文只切单字）⇒ 窗口里几乎总有
/// 「的 / 了 / 我」这类字 ⇒ 重叠数被模板词淹没 ⇒ **窗口选择退化成随机**。
pub fn render_record(strategy: InjectionStrategy, record: &RetrievedRecord, query: &str) -> String {
    match strategy {
        InjectionStrategy::Head { chars } => head_window(&record.content, chars),
        InjectionStrategy::Tail { chars } => tail_window(&record.content, chars),
        InjectionStrategy::BestWindow { chars } => best_window(&record.content, query, chars),
        InjectionStrategy::Whole => record.content.clone(),
    }
}

/// 人话对照表（每行：策略 · recall · coverage · 注入字符数）。
///
/// # 口径
///
/// * 第一行是**表头**（含列名）；
/// * 数值固定 **4 位小数**（可逐位比对，不因平台而变）；
/// * **空输入 ⇒ 空串**（没有可比的行 ⇒ 不画表头）。
pub fn render_table(outcomes: &[StrategyOutcome]) -> String {
    if outcomes.is_empty() {
        return String::new();
    }
    let width = outcomes
        .iter()
        .map(|outcome| outcome.strategy_id.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let mut lines = Vec::with_capacity(outcomes.len() + 1);
    lines.push(format!(
        "{:<width$}  {:>10}  {:>10}  {:>12}",
        "策略", "recall@k", "coverage", "注入字符数"
    ));
    for outcome in outcomes {
        lines.push(format!(
            "{:<width$}  {:>10.4}  {:>10.4}  {:>12}",
            outcome.strategy_id,
            outcome.metrics.recall_at_k,
            outcome.fragment_coverage,
            outcome.injected_chars
        ));
    }
    lines.join("\n")
}

/// **结论**：哪种策略的 `fragment_coverage` 最高（并列 ⇒ 名字序）。
/// 返回 `None` 表示**没有可比较的策略**（空输入）。
///
/// # 口径
///
/// * 比的是 [`StrategyOutcome::fragment_coverage`]（**逐片段**口径 ⇒ 跨策略可比）；
/// * **并列**（覆盖率**逐字相等**）⇒ 取**名字序最小**的（确定性，不看输入顺序）；
/// * 空输入 ⇒ `None`。
pub fn best_strategy(outcomes: &[StrategyOutcome]) -> Option<String> {
    let mut best: Option<&StrategyOutcome> = None;
    for outcome in outcomes {
        best = Some(match best {
            None => outcome,
            Some(current) => {
                if outcome.fragment_coverage > current.fragment_coverage
                    || (outcome.fragment_coverage == current.fragment_coverage
                        && outcome.strategy_id < current.strategy_id)
                {
                    outcome
                } else {
                    current
                }
            }
        });
    }
    best.map(|outcome| outcome.strategy_id.clone())
}

// ============================================
// 内部：跑一个策略
// ============================================

/// 跑一个策略 ⇒ (指标, 逐片段覆盖率, 注入字符数)。
fn run_strategy(
    golden: &GoldenSet,
    retrieved: &[Vec<RetrievedRecord>],
    strategy: InjectionStrategy,
) -> (Metrics, f64, usize) {
    let mut batch = EvalBatch::default();
    let mut fragments_per_case: Vec<Vec<String>> = Vec::with_capacity(golden.cases.len());
    let mut injected_chars = 0usize;

    for (index, case) in golden.cases.iter().enumerate() {
        // 检索结果**短于卷子** ⇒ 缺的题按「什么也没检索到」算（不截断卷子）。
        let records: &[RetrievedRecord] = retrieved.get(index).map(Vec::as_slice).unwrap_or(&[]);
        let (fragments, chars) = render_case(strategy, case, records);
        injected_chars += chars;
        fragments_per_case.push(fragments.clone());
        batch.cases.push(case.clone());
        // ⚠️ **ids 必须是检索顺序**（recall / nDCG 按位置折损）；
        //    `fragments` 是**真正被注入**的条目（顺序与 ids 对应）。
        batch.results.push(Retrieved {
            ids: records.iter().map(|record| record.id.clone()).collect(),
            fragments,
        });
    }

    // `evaluate` 的错误分支（cases 与 results 长度不等）**不可达**
    // （本函数保证两者等长）⇒ 退回默认指标（**不 panic**，也不假装成功）。
    let metrics = evaluate(&batch, eval_k(retrieved)).unwrap_or_default();
    let fragment_coverage = fragment_coverage(&golden.cases, &fragments_per_case);
    (metrics, fragment_coverage, injected_chars)
}

/// 每条证据的**装配余量**（字）。
///
/// # 为什么要有
///
/// 渲染器给每条证据加「出处」（`内容 —— 出处`）⇒ 条目比证据正文长几个字
/// ⇒ 预算只按正文算 ⇒ 产物装不下 ⇒ **覆盖全 0**（假结论）。
/// ⇒ 每条留 8 字余量（出处 + 分隔符）⇒ **装得下** ⇒ 变的只是窗口。
const ASSEMBLY_HEADROOM_PER_ITEM: usize = 8;

/// 一题的注入：按策略切窗口 ⇒ 交给渲染器装配 ⇒ (证据条目, 注入字符数)。
///
/// # 为什么走渲染器
///
/// 渲染器是**注入契约**的实现（预算纪律、截断标注都在那儿）⇒ 本模块**不重造**一套。
/// 且所有策略用**同一个**渲染器 ⇒ 变的只有**窗口**，不是形态标签。
fn render_case(
    strategy: InjectionStrategy,
    case: &GoldenCase,
    records: &[RetrievedRecord],
) -> (Vec<String>, usize) {
    let evidence: Vec<Evidence> = records
        .iter()
        .enumerate()
        .map(|(rank, record)| {
            Evidence::new(
                record.id.clone(),
                render_record(strategy, record, &case.query),
                record.id.clone(),
                rank_relevance(rank),
            )
        })
        .collect();
    // 预算 = 证据字数 + 出处字数 + 每条的**装配余量** ⇒ **装得下** ⇒ 变的不是预算。
    //
    // ⚠️ **余量为什么必须有**（踩过）：渲染器给每条证据加「出处」（`内容 —— 出处`）
    // ⇒ 条目实际比证据正文**长** ⇒ 只按正文算预算 ⇒ 第一条就「装不下」
    // ⇒ 产物是空的 ⇒ **整表覆盖全是 0**（看起来像「哪种策略都不行」）。
    let total_chars = evidence
        .iter()
        .map(|item| item.text.chars().count() + item.attribution.chars().count())
        .sum::<usize>()
        + evidence.len() * ASSEMBLY_HEADROOM_PER_ITEM;
    let budget = Budget::new(total_chars, 0);
    let injection = BulletedRenderer.render(&evidence, budget);
    let chars = injection.char_len();
    (fragments_of(&injection), chars)
}

/// 评测用的 top-k：**按传入的检索结果条数自适应**（各题最大条数，至少 1）。
///
/// # 为什么自适应
///
/// `retrieved` 已经是检索器的 top-k 结果 ⇒ 评测的 k 必须**装得下它**
/// （否则 recall 被 k 截断 ⇒ 测的是 k 而不是检索）。
fn eval_k(retrieved: &[Vec<RetrievedRecord>]) -> usize {
    retrieved
        .iter()
        .map(|records| records.len())
        .max()
        .unwrap_or(0)
        .max(1)
}

/// 名次派生的相关度（**单调递减** ⇒ 装配顺序 = 检索顺序）。
///
/// # 为什么要造一个相关度
///
/// 渲染器按相关度降序装入 ⇒ 若都给同一个相关度，装配顺序就变成**按 id 排序**
/// ⇒ 与检索顺序脱节 ⇒ 注入产物和 `ids` 对不上。
/// ⇒ 用名次派生的单调递减值（RRF 式）⇒ 装配顺序 = 检索顺序。
fn rank_relevance(rank: usize) -> f64 {
    1.0 / (rank as f64 + 1.0)
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

/// **逐片段覆盖率**：分母 = 全部期望片段数，分子 = 被覆盖的期望片段数。
///
/// # 口径
///
/// * 期望片段**先 trim 再去空**（与卷子的证据族口径一致）⇒ 空片段不占分母；
/// * 覆盖判定 = [`covers_fragment`]（**同一把尺子**）⇒ 与 `evidence_precision`
///   的口径不打架；
/// * **跨题累加**（不是逐题平均后再平均）⇒ 分母与策略**无关** ⇒ 跨策略可比。
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
// 内部：窗口
// ============================================

/// 取正文**前 N 字**（按字符切 ⇒ 不切坏 UTF-8）。
fn head_window(content: &str, chars: usize) -> String {
    content.chars().take(chars).collect()
}

/// 取正文**后 N 字**。
fn tail_window(content: &str, chars: usize) -> String {
    let length = content.chars().count();
    if chars >= length {
        return content.to_string();
    }
    content.chars().skip(length - chars).collect()
}

/// 取**与查询词重叠最多**的那一段（重叠滑窗；并列取靠前的）。
///
/// # 口径
///
/// * 窗口 = `chars` 字，步长 = 窗口的一半（**重叠**滑窗 ⇒ 不错过片段边界）；
/// * 评分 = 窗口里出现的**查询词个数**（去重后的查询词，逐个 `contains`）；
/// * 查询词用 [`crate::golden_mining::tokenize`]（CJK bigram + ASCII 词）；
/// * **并列取靠前的窗口** ⇒ 确定性；
/// * 查询**没有词**（空 / 全标点）⇒ 退回「取开头」（与生产口径同构，不猜）。
fn best_window(content: &str, query: &str, chars: usize) -> String {
    if chars == 0 {
        return String::new();
    }
    let all: Vec<char> = content.chars().collect();
    if all.len() <= chars {
        return content.to_string();
    }
    let query_terms: BTreeSet<String> = crate::golden_mining::tokenize(query).into_iter().collect();
    if query_terms.is_empty() {
        return head_window(content, chars);
    }
    let step = (chars / 2).max(1);
    let mut best_score = 0usize;
    let mut best_start = 0usize;
    let mut start = 0usize;
    while start < all.len() {
        let end = (start + chars).min(all.len());
        let window: String = all[start..end].iter().collect();
        let score = query_terms
            .iter()
            .filter(|term| window.contains(term.as_str()))
            .count();
        if score > best_score {
            best_score = score;
            best_start = start;
        }
        if end == all.len() {
            break;
        }
        start += step;
    }
    all[best_start..(best_start + chars).min(all.len())]
        .iter()
        .collect()
}

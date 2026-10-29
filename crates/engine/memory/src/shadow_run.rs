//! `apeireth-memory::shadow_run` — **影子双跑**（换算法可零风险上线/回退）。
//!
//! # 什么是影子双跑
//!
//! 生产上**同时**跑两套检索：现役（champion）与候选（challenger）。
//! **影子侧的结果不返回给用户**，只被记录与比较 ⇒
//! ⇒ 可以在**零风险**下观察 challenger 的真实表现 ⇒ 上线/回退都有**证据**。
//!
//! # 三条纪律
//!
//! 1. **影子侧绝不影响生产**（结果不外泄、不改状态）；
//! 2. **比「稳定分」不比「单次分」**（§6.4）⇒ 允许**多次运行取中位数**；
//! 3. **唯一的上线许可**：准确度**不变**的前提下，允许以**更少 token** 完成（纯赚）
//!    ⇒ 超过 1% 的劣化 ⇒ **不许换**（这就是 `Swap cost` 闸）。
//!
//! # 口径（逐条钉死，别自创）
//!
//! ## 准确度 = `recall@k`（`k` = [`ACCURACY_TOP_K`] = 10）
//!
//! * 复用 [`crate::golden_set::evaluate`] 这把**已有的尺子** ⇒ 口径**只有一处**
//!   （另写一份 recall 公式 ⇒ 迟早两个数对不上 ⇒ 纪律失效）；
//! * **无标准答案的题（`expect_ids` 为空）跳过**（不进分母、**也不记 0 分**）——
//!   与 `evaluate` 的口径一致：没有标准答案的题不该给系统送分，也不该罚系统；
//! * 一题多目标时按 `evaluate` 的口径算「命中了几个标准答案 / 标准答案总数」。
//!
//! ## Token 效率 = 两侧注入 token 的**中位数**
//!
//! * 单次跑的口径 = **全题注入 token 之和**（每题一次检索的 `injected_tokens` 相加）；
//! * ⚠️ **token 口径 = 字符数代理**（仓里没有分词器）⇒ **绝对值不代表真实 token 数**，
//!   只用于**两侧相对比较**（「谁更省」），**不许**拿去和外部系统的真实 token 数对齐。
//!
//! ## 中位数 = 升序排序后取**下中位数**（第 `⌈n/2⌉` 位，1-based）
//!
//! * 偶数个时取**较小**的那个 ⇒ 结果永远是**整数**（token 侧不需要四舍五入）
//!   且**确定性**、可手算复核；
//! * 为什么不用平均值：一次坏运会把均值拖走 ⇒ 也就把「稳定分」又变回「带噪分」。
//!
//! ## 判定一览（[`judge_swap`] 的输出，**先看这个**）
//!
//! | 情形 | 判定 |
//! |---|---|
//! | 空题集 / `runs = 0` / 卷子没有带标准答案的题 | **Rejected**（没证据 ≠ 没变差） |
//! | 准确度劣化 **> 1%** | **Rejected**（理由带数字） |
//! | 注入 token **多于**现役 | **Rejected**（不许更贵；**即便准确度提升也拒**） |
//! | 准确度**提升**且 token 未增 | **ApprovedWithGain**（理由写明提升多少） |
//! | 准确度持平或微跌（≤ 1%）且 token 未增 | **Approved**（实测通过，不是默认通过） |
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、执行、度量、裁决、测试齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 0 调用点
//!   （「影子双跑」要在生产里同时跑两套检索 ⇒ 那是**下批**的接线活）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：尚未在真实卷子 + 真实检索器上跑过
//!   （公开基准数据未到位 ⇒ 只有单测里的假检索器跑过）。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use crate::golden_set::{evaluate, EvalBatch, GoldenCase, Metrics, Retrieved};

/// 准确度口径的 `k`（`recall@k` 的 cutoff）。
///
/// # 为什么固定 10 而不是「用检索器返回多少条」
///
/// `recall@k` 的 `k` 是**统一的取数窗口** ⇒ 两侧必须同一个 `k` 才可比。
/// 检索器自己返回不足 `k` 条时按实际条数算（**不许**为每侧各定一个 `k`）。
///
/// ⚠️ **固有性质**：检索器多返回条目 ⇒ 召回窗口更大 ⇒ 分数更高。
/// 所以**检索器实现方应当自己按 `k` 截断**（否则就是在比「谁塞得多」，
/// 而不是比「谁排得准」）。
///
/// # 为什么是 10
///
/// 与仓内内部卷基线**同口径**（`scripts/golden-baseline.json` 的 `"k": 10`）
/// ⇒ 影子分与基线分**可比**（换一个 `k` ⇒ 两把尺子量出来的数不能对着看）。
pub const ACCURACY_TOP_K: usize = 10;

/// 默认运行次数（§6.4「比稳定分不比单次分」⇒ 默认跑 3 次取中位数）。
pub const DEFAULT_RUNS: usize = 3;

/// 相对劣化容忍度（1%）—— **与 [`crate::golden_set::check_no_regression`] 同口径**。
///
/// 为什么复用同一个数：判「掉了」的标准只应有一处 ⇒ 两处各写各的 ⇒ 迟早一边放行一边拦下。
const TOLERANCE_RATIO: f64 = 0.01;

/// 绝对下限（吸收浮点表示误差）—— 同 [`crate::golden_set::check_no_regression`]。
const TOLERANCE_FLOOR: f64 = 1e-9;

/// 一次检索（**可替换的算法**）的产物。
#[derive(Debug, Clone, PartialEq)]
pub struct RunOutcome {
    /// 返回的记录 id（**有序** ⇒ 第一条是 top-1）。
    pub ids: Vec<String>,
    /// 注入的**总 token 估算**（用字符数代理：仓里没有分词器 ⇒ 口径要**写明**）。
    pub injected_tokens: usize,
}

/// 一套检索算法（**可替换** ⇒ 这是「换算法」的接口）。
pub trait Retriever {
    /// 算法 id（如 `bm25` / `hybrid` / `challenger-v2`）。
    fn id(&self) -> &str;
    /// 对一道题检索。
    fn retrieve(&self, case: &GoldenCase) -> RunOutcome;
}

/// 影子双跑的**一对结果**。
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowPair {
    pub case_id: String,
    /// 现役侧。
    pub champion: RunOutcome,
    /// 影子侧。
    pub challenger: RunOutcome,
}

/// 影子双跑的结果（**一次跑完**）。
///
/// # 为什么这里**没有**「最终答案」字段
///
/// 影子侧的结果**只用于比较** ⇒ 一旦这里出现「给用户看的东西」，
/// 就有可能被人/程序拿去当生产结果返回 ⇒ **影子纪律当场破功**。
/// 所以本结构只装 **id 与指标**（`ids` / `injected_tokens`），**别加答案字段**。
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowReport {
    /// 现役算法 id。
    pub champion_id: String,
    /// 候选算法 id。
    pub challenger_id: String,
    /// 逐题结果。
    pub pairs: Vec<ShadowPair>,
}

/// 跑一次影子双跑。
///
/// # 口径
///
/// * **逐题成对**：`pairs[i]` 对应 `cases[i]` ⇒ 调用方能逐题对齐比较；
/// * **影子侧不外泄**：只返回 **id + 指标**，**没有**「最终答案」字段；
/// * **不改状态**：两侧检索都是 `&self` 只读调用 ⇒ 影子侧**没有**写生产的口子；
/// * **空题集** ⇒ 返回空的 `pairs`（**不 panic**）。
pub fn run_shadow(
    cases: &[GoldenCase],
    champion: &dyn Retriever,
    challenger: &dyn Retriever,
) -> ShadowReport {
    let mut pairs = Vec::with_capacity(cases.len());
    for case in cases {
        // 顺序固定：现役先、影子后 ⇒ 两侧调用序可复现（便于排查「谁先动的手」）。
        let champion_outcome = champion.retrieve(case);
        let challenger_outcome = challenger.retrieve(case);
        pairs.push(ShadowPair {
            case_id: case.id.clone(),
            champion: champion_outcome,
            challenger: challenger_outcome,
        });
    }
    ShadowReport {
        champion_id: champion.id().to_string(),
        challenger_id: challenger.id().to_string(),
        pairs,
    }
}

/// 换算法的**裁决**。
#[derive(Debug, Clone, PartialEq)]
pub struct SwapVerdict {
    pub decision: SwapDecision,
    /// 现役的中位分数（**没度量到** ⇒ `None`）。
    pub champion_score: Option<f64>,
    /// 候选的中位分数（**没度量到** ⇒ `None`）。
    pub challenger_score: Option<f64>,
    /// 现役的中位 token（**没度量到** ⇒ `None`）。
    pub champion_tokens: Option<usize>,
    /// 候选的中位 token（**没度量到** ⇒ `None`）。
    pub challenger_tokens: Option<usize>,
    /// **人话理由**（拒绝时**必须**说明差在哪，且**带数字**）。
    pub reason: String,
}

/// 裁决的三种结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapDecision {
    /// 准确度不变 + token 不多 ⇒ 允许换（纯赚）。
    Approved,
    /// 准确度提升 ⇒ 允许换。
    ApprovedWithGain,
    /// 不许换。
    Rejected,
}

/// **换算法的闸**（`Swap cost` 的判据）。
///
/// # 口径（照设计稿 §6.4）
///
/// * 现役 vs 候选，各跑 **N 次**（默认 [`DEFAULT_RUNS`] = 3）⇒ **比中位数**（不比单次）；
/// * **准确度**：候选的 `recall_at_k` **不得低于**现役的 99%（**最差情况不劣化 > 1%**）；
/// * **Token 效率**：候选的注入 token **不得多于**现役（不许更贵）；
/// * ⇒ **判定**：
///   * `Approved` —— 准确度不掉**且** token 相同或更少 ⇒ **允许换**（纯赚）；
///   * `ApprovedWithGain` —— 准确度**提升** ⇒ 允许换（**明确说明提升多少**）；
///   * `Rejected` —— 劣化超 1% 或 token 变多 ⇒ **不许换**（**给出原因**）。
///
/// # 判定顺序（为什么是这个顺序）
///
/// 1. **没度量到 ⇒ 不许换**：空题集 / `runs = 0` / 卷子里没有带标准答案的题
///    ⇒ 一律 `Rejected`（**不是默认通过** —— 「没证据」不等于「没变差」）。
/// 2. **准确度闸**：劣化超过 1% ⇒ `Rejected`（§6.4「准确度一分不许掉」）。
/// 3. **token 闸**：候选注入 token 多于现役 ⇒ `Rejected`（**不许更贵**）。
///    ⚠️ **口径分歧点**：即便准确度**提升**，token 变多也判拒。
///    理由：§6.4 钦定的交换许可只有「准确度不变 + 更少 token（纯赚）」这一条
///    ⇒ 「拿 token 换准确度」属于**政策裁量**，不是机器闸能自动放行的事
///    ⇒ 宁可拦下并**在理由里说清差在哪**，让人来裁。
/// 4. **有提升** ⇒ `ApprovedWithGain`（理由里**写明提升了多少**）。
/// 5. **持平（含 ≤ 1% 的微跌）** ⇒ `Approved`。
///
/// # 必须保证
///
/// * **确定性**：同一输入多次 ⇒ 逐字相等；
/// * **不许 panic**：空题集 ⇒ 返回一个**明确**的判定（不是默认通过）。
pub fn judge_swap(
    champion: &dyn Retriever,
    challenger: &dyn Retriever,
    cases: &[GoldenCase],
    runs: usize,
) -> SwapVerdict {
    // --- 0. 没度量到 ⇒ 不许换（不是默认通过） ---
    if runs == 0 {
        return SwapVerdict {
            decision: SwapDecision::Rejected,
            champion_score: None,
            challenger_score: None,
            champion_tokens: None,
            challenger_tokens: None,
            reason: "运行次数为 0 ⇒ **没有度量** ⇒ 不许换（没证据 ≠ 没变差）".to_string(),
        };
    }
    if cases.is_empty() {
        return SwapVerdict {
            decision: SwapDecision::Rejected,
            champion_score: None,
            challenger_score: None,
            champion_tokens: None,
            challenger_tokens: None,
            reason: "空题集 ⇒ **没有度量** ⇒ 不许换（没证据 ≠ 没变差）".to_string(),
        };
    }

    // --- 1. 各跑 N 次，取中位数（§6.4「比稳定分不比单次分」） ---
    let mut champion_scores = Vec::with_capacity(runs);
    let mut challenger_scores = Vec::with_capacity(runs);
    let mut champion_tokens = Vec::with_capacity(runs);
    let mut challenger_tokens = Vec::with_capacity(runs);
    let mut scorable_cases = 0usize;

    for _ in 0..runs {
        let champion_measurement = measure(champion, cases);
        let challenger_measurement = measure(challenger, cases);
        champion_scores.push(champion_measurement.score);
        champion_tokens.push(champion_measurement.tokens);
        challenger_scores.push(challenger_measurement.score);
        challenger_tokens.push(challenger_measurement.tokens);
        // 「能打分的题数」只取决于卷子（标注），与跑几次无关 ⇒ 取最后一次即可。
        scorable_cases = champion_measurement.scorable_cases;
    }

    let champion_score = median_f64(&champion_scores);
    let challenger_score = median_f64(&challenger_scores);
    let champion_token_median = median_usize(&champion_tokens);
    let challenger_token_median = median_usize(&challenger_tokens);

    // --- 2. 准确度不可度量 ⇒ 不许换 ---
    if scorable_cases == 0 {
        return SwapVerdict {
            decision: SwapDecision::Rejected,
            champion_score: None,
            challenger_score: None,
            champion_tokens: Some(champion_token_median),
            challenger_tokens: Some(challenger_token_median),
            reason: format!(
                "卷子里**没有可打分的题**（{cases_count} 题里 0 题带有效 `expect_ids`）⇒ \
                 准确度**不可度量** ⇒ 不许换（没证据 ≠ 没变差）",
                cases_count = cases.len()
            ),
        };
    }

    // --- 3. 准确度闸：劣化 > 1% ⇒ 不许换 ---
    let allowed_drop = champion_score * TOLERANCE_RATIO + TOLERANCE_FLOOR;
    let drop = champion_score - challenger_score;
    if drop > allowed_drop {
        return SwapVerdict {
            decision: SwapDecision::Rejected,
            champion_score: Some(champion_score),
            challenger_score: Some(challenger_score),
            champion_tokens: Some(champion_token_median),
            challenger_tokens: Some(challenger_token_median),
            reason: format!(
                "准确度劣化：recall@{k} 中位数 {challenger_score:.6} vs 现役 {champion_score:.6} \
                 （掉 {drop:.6}，超过允许上限 {allowed_drop:.6}）⇒ **不许换**",
                k = ACCURACY_TOP_K
            ),
        };
    }

    // --- 4. token 闸：候选不许更贵 ---
    if challenger_token_median > champion_token_median {
        return SwapVerdict {
            decision: SwapDecision::Rejected,
            champion_score: Some(champion_score),
            challenger_score: Some(challenger_score),
            champion_tokens: Some(champion_token_median),
            challenger_tokens: Some(challenger_token_median),
            reason: format!(
                "注入 token 变多：候选中位 {challenger_token_median} vs 现役 {champion_token_median} \
                 （多 {extra}）⇒ **不许换**（不许更贵）",
                extra = challenger_token_median - champion_token_median
            ),
        };
    }

    // --- 5. 有提升 ⇒ 放行并说明提升多少 ---
    let gain = challenger_score - champion_score;
    if gain > TOLERANCE_FLOOR {
        return SwapVerdict {
            decision: SwapDecision::ApprovedWithGain,
            champion_score: Some(champion_score),
            challenger_score: Some(challenger_score),
            champion_tokens: Some(champion_token_median),
            challenger_tokens: Some(challenger_token_median),
            reason: format!(
                "准确度提升：recall@{k} 中位数 {challenger_score:.6} vs 现役 {champion_score:.6} \
                 （提升 {gain:.6}）且注入 token 未增加（中位 {challenger_token_median} ≤ \
                 {champion_token_median}）⇒ **允许换**",
                k = ACCURACY_TOP_K
            ),
        };
    }

    // --- 6. 持平（微跌在 1% 容忍内）⇒ 放行 ---
    SwapVerdict {
        decision: SwapDecision::Approved,
        champion_score: Some(champion_score),
        challenger_score: Some(challenger_score),
        champion_tokens: Some(champion_token_median),
        challenger_tokens: Some(challenger_token_median),
        reason: format!(
            "准确度未劣化：recall@{k} 中位数 {challenger_score:.6} vs 现役 {champion_score:.6} \
             （劣化 {}，在 1% 容忍上限 {allowed_drop:.6} 内）且注入 token 未增加\
             （中位 {challenger_token_median} ≤ {champion_token_median}）⇒ **允许换**",
            drop.max(0.0),
            k = ACCURACY_TOP_K
        ),
    }
}

// ============================================
// 内部工具
// ============================================

/// 一次跑批的度量结果（**单次**，尚未取中位数）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Measurement {
    /// 准确度 = `recall@k`（单次值）。
    score: f64,
    /// 全题注入 token 之和。
    tokens: usize,
    /// 有标准答案、能参与准确度打分的题数。
    scorable_cases: usize,
}

/// 跑一遍检索并度量（**单次**）。
///
/// 准确度用 [`evaluate`] 这把**已有的尺子**算 ⇒ 与考卷模块**同一个口径**
/// （自己再写一份 recall 公式 ⇒ 两个数迟早对不上 ⇒ 纪律失效）。
fn measure(retriever: &dyn Retriever, cases: &[GoldenCase]) -> Measurement {
    let mut batch = EvalBatch::default();
    let mut tokens = 0usize;
    for case in cases {
        let outcome = retriever.retrieve(case);
        tokens = tokens.saturating_add(outcome.injected_tokens);
        batch.cases.push(case.clone());
        // ⚠️ **片段留空**：本模块只比「检索准不准 + 花多少 token」，
        // 证据族指标（evidence precision / groundedness）是**注入形态**的事，
        // 与「换检索算法」**不是一件事** ⇒ 混进来会把改算法误判成改注入。
        batch.results.push(Retrieved {
            ids: outcome.ids,
            fragments: Vec::new(),
        });
    }
    match evaluate(&batch, ACCURACY_TOP_K) {
        Ok(metrics) => Measurement {
            score: metrics.recall_at_k,
            tokens,
            scorable_cases: scorable_cases(&metrics),
        },
        // `evaluate` 只在「题数与结果数不等」时失败 ⇒ 这里两边等长 ⇒ 理论上不可达；
        // 万一不可达：**不 panic**，按「不可度量」处理（⇒ 裁决会判不许换）。
        Err(_) => Measurement {
            score: 0.0,
            tokens,
            scorable_cases: 0,
        },
    }
}

/// 有标准答案、能参与准确度打分的题数（`evaluate` 的 `skipped_cases` 记的是「整题无法评」）。
fn scorable_cases(metrics: &Metrics) -> usize {
    metrics.total_cases.saturating_sub(metrics.skipped_cases)
}

/// 中位数（`f64`）：升序排序后取**下中位数**（第 `⌈n/2⌉` 位，1-based）。
fn median_f64(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    // NaN 不可比较 ⇒ 视为相等（排序稳定 ⇒ 结果仍确定）；`judge_swap` 的分数不会是 NaN。
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    sorted[(sorted.len() - 1) / 2]
}

/// 中位数（`usize`）：升序排序后取**下中位数**（第 `⌈n/2⌉` 位，1-based）。
fn median_usize(values: &[usize]) -> usize {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[(sorted.len() - 1) / 2]
}

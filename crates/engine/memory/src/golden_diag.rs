//! `apeireth-memory::golden_diag` — 召回缺口的**诊断**（回答「为什么 recall 这么低」）。
//!
//! # 这个模块要回答什么
//!
//! 内部卷（《紅樓夢》120 回 / 476 题）跑出 `recall@10 = 0.2983` ⇒
//! **73% 的题，目标记录根本没进检索结果**（W-12 实测：天花板 coverage 0.2712 ≈ 0.91 × recall@10）
//! ⇒ **瓶颈在召回，不在注入**。
//!
//! 但「召回低」有**五种**完全不同的成因，混在一起看只会互相甩锅。
//! 本模块负责把它们**分开**：
//!
//! | 分类 | 含义 | 该找谁 |
//! |---|---|---|
//! | [`GapClass::Hit`] | 目标在 k 以内 | — |
//! | [`GapClass::RankedOut`] | 目标**进了候选池**但排在 k 外 | **排序** |
//! | [`GapClass::NotInPool`] | 目标**压根没进候选池** | **索引/分词**（或题挖错了） |
//! | [`GapClass::NoExpectation`] | 这题**没有期望记录** | **卷子本身** |
//!
//! # 口径（钉死，别自创）
//!
//! * **候选池** = 检索返回的**完整**（未按 k 截断）有序结果。
//!   ⚠️ 调用方**必须**给一个**比 k 大**的池子 ⇒ 否则「没进池」与「排在 k 外」
//!   **不可区分**（那时 `NotInPool` 只能读成「不在 top-k 里」）。
//! * **名次** = 目标记录在池子里的**位次**（0 基）。
//! * **多名次**：期望多条记录时取**最好**的那个（最乐观口径）⇒
//!   本模块给的是**上界**；真实 recall（`golden_set::evaluate`）按**命中比例**算，会更低。
//! * **零分结果不算检索结果** ⇒ 池子只保留**分数 > 0** 的候选（与 `golden_run::run` 同口径：
//!   BM25 源会按 id 顺序填满 limit ⇒ 分数为 0 的文档也会被返回 ⇒ 不滤就会「假命中」）。
//! * **无期望的题不进召回分母**（与 `golden_set::evaluate` 的 Recall 口径一致）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：诊断类型 + 计算 + 报告齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 **0 调用点**（只有 example 与测试在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✔（部分）：已在《紅樓夢》**真实语料**上跑过（2026-10-26），
//!   但**未在第二台机器复现** ⇒ 不宣称跨机器可复现。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::collections::BTreeSet;

use crate::golden_run::IndexRecord;
use crate::golden_set::GoldenCase;

// ⚠️ 契约草案里写的是 `use crate::golden_run::Retrieved;`，但 `golden_run` **没有** re-export
// 这个类型（它自己也是从 `golden_set` 导入的）⇒ 本批**不许改** `golden_run.rs`
// ⇒ 这里从**定义处** `golden_set` 导入并**再导出**。
// 两者是**同一个类型** ⇒ 公开签名与契约**逐字一致**（`golden_diag::Retrieved` 亦可用）。
pub use crate::golden_set::Retrieved;

/// 一道题的**召回诊断**。
#[derive(Debug, Clone, PartialEq)]
pub struct RecallDiag {
    /// 题 id。
    pub case_id: String,
    /// 期望命中的记录 id 列表。
    ///
    /// ⚠️ **已去掉空白 id**（与 `golden_set` 的 `expect_set` 同口径：空串 id 视为无效标注）
    /// ⇒ 列表为空 ⇒ 这题**没有期望记录**（[`GapClass::NoExpectation`]）。
    pub expect_ids: Vec<String>,
    /// 目标记录在检索结果里的**名次**（0 基；未命中 ⇒ `None`）。
    ///
    /// ⚠️ **多名次**：期望命中多条时 ⇒ 取**最好**的那个名次（最乐观口径）。
    pub best_rank: Option<usize>,
    /// 目标记录是否**进了候选池**（= 出现在比 k 更多的结果里）。
    pub in_candidate_pool: bool,
    /// 这次检索实际返回了几条。
    pub returned: usize,
    /// 该题的**主体词**（用于人工看）。
    pub subject: String,
}

impl RecallDiag {
    /// 缺口分类（**这是本模块的核心产出**）。
    ///
    /// * [`GapClass::Hit`]：名次 < k ⇒ 目标在 top-k 里（这一题**召回成功**）。
    /// * [`GapClass::RankedOut`]：名次 ≥ k ⇒ **进了候选池但排在 k 之外**
    ///   ⇒ **排序问题**（召回本身没问题）。
    /// * [`GapClass::NotInPool`]：压根没进候选池 ⇒ **索引/分词问题**（或题挖错了）。
    /// * [`GapClass::NoExpectation`]：这题**没有期望记录**（题坏了）
    ///   ⇒ 单独归类，**不计入召回分母**。
    pub fn classify(&self, k: usize) -> GapClass {
        if self.expect_ids.is_empty() {
            return GapClass::NoExpectation;
        }
        match self.best_rank {
            Some(rank) if rank < k => GapClass::Hit,
            Some(_) => GapClass::RankedOut,
            None => GapClass::NotInPool,
        }
    }

    /// 该题的**有效期望 id**（去空白；与 `golden_set` 同口径）。
    fn expected(&self) -> BTreeSet<&str> {
        self.expect_ids
            .iter()
            .map(|id| id.trim())
            .filter(|id| !id.is_empty())
            .collect()
    }
}

/// 召回缺口的**分类**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapClass {
    /// **命中了**（在 k 以内）。
    Hit,
    /// **进了候选池但排在 k 之外** ⇒ **排序问题**（召回不是问题）。
    RankedOut,
    /// **压根没进候选池** ⇒ **索引/分词问题**（或题挖错了）。
    NotInPool,
    /// **这题没有期望记录**（题坏了）⇒ 单独归类，**不计入召回分母**。
    NoExpectation,
}

impl GapClass {
    /// 分类的**中文名**（报告与日志用；不引第三方枚举名 ⇒ 零依赖）。
    pub fn label(self) -> &'static str {
        match self {
            GapClass::Hit => "命中",
            GapClass::RankedOut => "排在 k 外",
            GapClass::NotInPool => "没进候选池",
            GapClass::NoExpectation => "无期望",
        }
    }
}

/// 汇总诊断结果。
#[derive(Debug, Clone, PartialEq)]
pub struct DiagSummary {
    /// 总题数。
    pub total: usize,
    /// `Hit` 的题数。
    pub hits: usize,
    /// `RankedOut` 的题数。
    pub ranked_out: usize,
    /// `NotInPool` 的题数。
    pub not_in_pool: usize,
    /// `NoExpectation` 的题数（**题坏了**）。
    pub no_expectation: usize,
    /// **目标记录在候选池里的名次分布**（中位数 / p25 / p75 / p95 / 最大）。
    ///
    /// ⚠️ **只统计「进过候选池」的目标**（`NotInPool` 的题**没有名次**，不是「名次 = ∞」）
    /// ⇒ 分母 = `hits + ranked_out`。
    pub rank_percentiles: Option<RankStats>,
}

/// 名次分布。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RankStats {
    /// 25 分位。
    pub p25: usize,
    /// 中位数（50 分位）。
    pub p50: usize,
    /// 75 分位。
    pub p75: usize,
    /// 95 分位。
    pub p95: usize,
    /// 最大名次。
    pub max: usize,
}

/// 诊断一批题。
///
/// # 口径
///
/// * `cases[i]` 与 `retrieved[i]` **一一对应**（`retrieved[i]` = 该题的**候选池**，
///   即**未按 k 截断**的完整有序结果）；
/// * `retrieved` 比 `cases` **短** ⇒ 缺结果的题按「空结果」处理（`returned = 0`
///   ⇒ 归 [`GapClass::NotInPool`]）；`retrieved` 比 `cases` **长** ⇒ 多出来的忽略；
/// * `k` 只在 [`RecallDiag::classify`] 里用得到（逐题诊断本身与 k 无关）
///   ⇒ 这里保留它是为了让调用方的口径**一眼可见**（别把 k 记错）。
///
/// # 前提（不满足就会误判）
///
/// **候选池必须比 k 大** ⇒ 否则「没进池」与「排在 k 外」**不可区分**
/// （`NotInPool` 会被高估，`RankedOut` 恒为 0）。
pub fn diagnose(cases: &[GoldenCase], retrieved: &[Vec<Retrieved>], k: usize) -> Vec<RecallDiag> {
    // `k` 在逐题诊断里用不到（分类在 `classify` 里做）⇒ 显式说明，别让 lint 猜。
    let _ = k;
    let mut out = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let expect_ids: Vec<String> = case
            .expect_ids
            .iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect();
        let pool: &[String] = retrieved
            .get(index)
            .and_then(|entries| entries.first())
            .map(|entry| entry.ids.as_slice())
            .unwrap_or(&[]);
        // 名次 = 目标 id 在池子里的**首次**位次；多条期望 ⇒ 取**最好**的那个。
        let mut best_rank: Option<usize> = None;
        for expected in &expect_ids {
            if let Some(position) = pool.iter().position(|id| id == expected) {
                best_rank =
                    Some(best_rank.map_or(position, |current: usize| current.min(position)));
            }
        }
        out.push(RecallDiag {
            case_id: case.id.clone(),
            expect_ids,
            in_candidate_pool: best_rank.is_some(),
            best_rank,
            returned: pool.len(),
            subject: subject_of(&case.query),
        });
    }
    out
}

/// 汇总。
///
/// # 口径
///
/// * 计数按 [`RecallDiag::classify`] 的四类分；
/// * `rank_percentiles` 只统计**进过候选池**的目标（`NotInPool` 没有名次）
///   ⇒ 全都不在池里时是 `None`（**不是** `Some(0)`）。
pub fn summarize(diags: &[RecallDiag], k: usize) -> DiagSummary {
    let mut hits = 0usize;
    let mut ranked_out = 0usize;
    let mut not_in_pool = 0usize;
    let mut no_expectation = 0usize;
    let mut ranks: Vec<usize> = Vec::new();

    for diag in diags {
        match diag.classify(k) {
            GapClass::Hit => hits += 1,
            GapClass::RankedOut => ranked_out += 1,
            GapClass::NotInPool => not_in_pool += 1,
            GapClass::NoExpectation => no_expectation += 1,
        }
        if let Some(rank) = diag.best_rank {
            ranks.push(rank);
        }
    }

    let rank_percentiles = rank_stats(&ranks);
    DiagSummary {
        total: diags.len(),
        hits,
        ranked_out,
        not_in_pool,
        no_expectation,
        rank_percentiles,
    }
}

/// 名次分布（**空输入 ⇒ `None`**，不 panic）。
///
/// # 分位口径
///
/// **最近秩法**（nearest-rank）：`p` 分位 = 升序序列里第 `ceil(p × n)` 个元素（1 基）。
/// 例：`[0,1,2,3,4]` ⇒ `p25 = 1`、`p50 = 2`、`p75 = 3`、`p95 = 4`、`max = 4`。
/// 选它的理由：**确定性**、**无插值**（名次是整数 ⇒ 插值出来的分位数没有物理意义）。
fn rank_stats(ranks: &[usize]) -> Option<RankStats> {
    if ranks.is_empty() {
        return None;
    }
    let mut sorted = ranks.to_vec();
    sorted.sort_unstable();
    Some(RankStats {
        p25: percentile(&sorted, 0.25),
        p50: percentile(&sorted, 0.50),
        p75: percentile(&sorted, 0.75),
        p95: percentile(&sorted, 0.95),
        max: *sorted.last().unwrap_or(&0),
    })
}

/// 最近秩分位数（`sorted` 必须**已升序**且非空）。
fn percentile(sorted: &[usize], p: f64) -> usize {
    if sorted.is_empty() {
        return 0;
    }
    let n = sorted.len();
    let index = ((p * n as f64).ceil() as usize)
        .saturating_sub(1)
        .min(n - 1);
    sorted[index]
}

/// 人话报告（**逐类计数 + 结论句**）。
///
/// # 结论句必须**如实**（不许粉饰）
///
/// * 若 `ranked_out` 占多数 ⇒ 「**排序**是瓶颈，不是召回」；
/// * 若 `not_in_pool` 占多数 ⇒ 「**索引/分词**是瓶颈」（或题挖错了）；
/// * 若 `no_expectation` 占比高 ⇒ 「**卷子本身有问题**」。
///
/// # 判定口径（钉死）
///
/// * 「占多数」= 该类占**有效题数**（`total - no_expectation`）的 **> 50%**；
/// * 「占比高」= `no_expectation` 占**总题数**的 **≥ 10%**；
/// * 「命中良好」只在命中率 **> 90%** 时才说 ⇒ 其余一律说「远未达标」。
pub fn render_report(summary: &DiagSummary, k: usize) -> String {
    let total = summary.total;
    let scored = total.saturating_sub(summary.no_expectation);
    let mut lines = Vec::new();

    lines.push(format!("召回缺口诊断（k={k}）"));
    lines.push(format!(
        "  总题数 {total} · 有效题数（进召回分母）{scored} · 无期望（题坏了）{}",
        summary.no_expectation
    ));
    lines.push(format!(
        "  · 命中 Hit：{} 题（{:.1}%）",
        summary.hits,
        percent(summary.hits, scored)
    ));
    lines.push(format!(
        "  · 排在 k 外 RankedOut：{} 题（{:.1}%）",
        summary.ranked_out,
        percent(summary.ranked_out, scored)
    ));
    lines.push(format!(
        "  · 没进候选池 NotInPool：{} 题（{:.1}%）",
        summary.not_in_pool,
        percent(summary.not_in_pool, scored)
    ));
    lines.push(format!(
        "  · 无期望 NoExpectation：{} 题（{:.1}%）",
        summary.no_expectation,
        percent(summary.no_expectation, total)
    ));
    match summary.rank_percentiles {
        Some(stats) => lines.push(format!(
            "  · 名次分布（只统计进过候选池的目标）：p25={} p50={} p75={} p95={} max={}",
            stats.p25, stats.p50, stats.p75, stats.p95, stats.max
        )),
        None => lines.push(
            "  · 名次分布：**空**（没有任何目标进过候选池 ⇒ 全部是索引/分词问题）".to_string(),
        ),
    }
    lines.push(format!("  · k 敏感性：{}", k_sensitivity(summary)));
    lines.push(format!("结论：{}", conclusion(summary, k)));
    lines.join("\n")
}

/// 「k 敏感性」一句话（**如实**说明加大 k 能救回多少）。
fn k_sensitivity(summary: &DiagSummary) -> String {
    let mut out = String::new();
    if let Some(stats) = summary.rank_percentiles {
        out.push_str(&format!(
            "把 k 提到 {} 才能覆盖**全部**进过候选池的目标",
            stats.max + 1
        ));
    } else {
        out.push_str("没有任何目标进过候选池 ⇒ **加大 k 完全无用**");
    }
    if summary.not_in_pool > 0 {
        out.push_str(&format!(
            "；NotInPool 的 {} 题**任何 k 都救不回来**（⇒ 加大 k 解决不了它们）",
            summary.not_in_pool
        ));
    }
    out
}

/// 结论句（**如实**，不许粉饰）。
fn conclusion(summary: &DiagSummary, k: usize) -> String {
    let total = summary.total;
    let scored = total.saturating_sub(summary.no_expectation);
    if scored == 0 {
        return format!("全部 {total} 题都没有期望记录 ⇒ **卷子本身有问题**（这不是召回的问题）");
    }

    let mut parts: Vec<String> = Vec::new();
    if summary.hits >= scored {
        // ⚠️ **没有缺口** ⇒ 报告**不许**甩锅（说「索引/分词」或「排序」都是假的）。
        parts.push(format!("全部 {scored} 道有效题都命中 ⇒ **没有召回缺口**"));
    } else if summary.not_in_pool > summary.ranked_out {
        parts.push(format!(
            "**索引/分词**是瓶颈（目标记录**根本没进候选池**）：NotInPool {} 题（{:.1}%）> RankedOut {} 题（{:.1}%）⇒ **瓶颈在召回，不在排序**",
            summary.not_in_pool,
            percent(summary.not_in_pool, scored),
            summary.ranked_out,
            percent(summary.ranked_out, scored)
        ));
    } else if summary.ranked_out > summary.not_in_pool {
        parts.push(format!(
            "**排序**是瓶颈（目标记录**进了候选池**但排在 k={k} 之外）：RankedOut {} 题（{:.1}%）> NotInPool {} 题（{:.1}%）⇒ **瓶颈是排序，不是召回**",
            summary.ranked_out,
            percent(summary.ranked_out, scored),
            summary.not_in_pool,
            percent(summary.not_in_pool, scored)
        ));
    } else {
        // ⚠️ 走到这里必有缺口（`hits < scored`）⇒ 两类都 > 0 ⇒ 真的是「持平」。
        parts.push(format!(
            "NotInPool（{} 题）与 RankedOut（{} 题）**持平** ⇒ 缺口是**混合**的：索引/分词与排序各占一半 ⇒ 两条线都要动",
            summary.not_in_pool, summary.ranked_out
        ));
    }

    let hit_ratio = summary.hits as f64 / scored as f64;
    if hit_ratio > 0.9 {
        parts.push(format!("命中良好（{:.1}%）", hit_ratio * 100.0));
    } else {
        parts.push(format!(
            "命中只有 {} / {}（{:.1}%）⇒ 召回**远未达标**",
            summary.hits,
            scored,
            hit_ratio * 100.0
        ));
    }

    if total > 0 && summary.no_expectation as f64 / total as f64 >= NO_EXPECTATION_HIGH_RATIO {
        parts.push(format!(
            "无期望的题占 {:.1}% ⇒ **卷子本身有问题**（这几题本就不该进召回分母）",
            summary.no_expectation as f64 / total as f64 * 100.0
        ));
    }

    parts.join("；")
}

/// 「无期望占比高」的阈值（占总题数）。
const NO_EXPECTATION_HIGH_RATIO: f64 = 0.10;

/// 百分比（分母为 0 ⇒ 0.0）。
fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64 * 100.0
    }
}

/// 从题面里取「主体词」（**人工看**用）。
///
/// # 口径
///
/// 按挖题器（`golden_mining`）的**五种题型模板**反解主体：
///
/// | 题型 | 题面模板 | 反解出的主体 |
/// |---|---|---|
/// | `keyword` | `关于{主体}，我们当时记了什么？` | `{主体}` |
/// | `decision` | `{主体}这件事最后是怎么定的？` | `{主体}` |
/// | `multi_record` | `{主体A}和{主体B}这两件事是什么关系？` | `{主体A}和{主体B}` |
/// | `temporal` | `{主体}是什么时候定下来的？` | `{主体}` |
/// | `conflict` | `{主体}后来是怎么改的？` | `{主体}` |
///
/// **都不匹配** ⇒ 退回整条题面（**如实**：不编一个主体出来）。
pub fn subject_of(query: &str) -> String {
    const TEMPLATES: [(&str, &str); 5] = [
        ("关于", "，我们当时记了什么？"),
        ("", "这件事最后是怎么定的？"),
        ("", "这两件事是什么关系？"),
        ("", "是什么时候定下来的？"),
        ("", "后来是怎么改的？"),
    ];
    for (prefix, suffix) in TEMPLATES {
        let Some(rest) = query.strip_suffix(suffix) else {
            continue;
        };
        let rest = rest.strip_prefix(prefix).unwrap_or(rest);
        let trimmed = rest.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    query.trim().to_string()
}

/// 检索**候选池**（**未按 k 截断**的完整有序结果）⇒ 供 [`diagnose`] 诊断。
///
/// # 口径（与 `golden_run::run` **逐字同源**）
///
/// * 检索器 = **生产用的 BM25 源**（`retrieval_pipeline::Bm25LexicalCandidateSource`）；
/// * 可检索文本 = `IndexRecord::text`（**不含题面来源** ⇒ 不泄漏答案）；
/// * **零分结果不算检索结果** ⇒ 只保留 `score > 0` 的候选
///   （BM25 源会**按 id 顺序填满** limit ⇒ 分数为 0 的文档也会被返回
///   ⇒ 不滤就会「目标恰好排在第 1」的**假命中**）。
///
/// ⚠️ **与 `golden_run::build_source` 的关系**：那个函数是**私有**的，而本批**不许改**
/// `golden_run.rs` ⇒ 这里**复制**了一份逐字同口径的实现
/// ⇒ **改一处必须改另一处**（否则诊断与基线分数会不同口径）。
pub fn retrieve_pool(
    records: &[IndexRecord],
    cases: &[GoldenCase],
    pool_size: usize,
) -> Result<Vec<Vec<Retrieved>>, crate::MemoryError> {
    use crate::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};

    let source = build_source(records);
    let mut pools = Vec::with_capacity(cases.len());
    for case in cases {
        let candidates = source.candidates(&case.query, pool_size)?;
        let ids: Vec<String> = candidates
            .into_iter()
            .filter(|candidate| candidate.score > 0.0)
            .map(|candidate| candidate.id)
            .collect();
        pools.push(vec![Retrieved {
            ids,
            fragments: Vec::new(),
        }]);
    }
    Ok(pools)
}

/// 按「最好名次」口径估算 recall@k（**乐观口径**：任一期望记录命中即算命中）。
///
/// # 口径
///
/// * 分母 = **有期望的题数**（与 `golden_set::evaluate` 的 Recall 分母一致）；
/// * 分子 = `best_rank < k` 的题数。
/// * ⚠️ 这是**上界**：官方 recall 按「命中比例」算（多目标题要求**全部**命中才满分）
///   ⇒ 本值 ≥ 官方 recall。
pub fn recall_estimate_at_k(diags: &[RecallDiag], k: usize) -> f64 {
    let mut hits = 0usize;
    let mut scored = 0usize;
    for diag in diags {
        if diag.expected().is_empty() {
            continue;
        }
        scored += 1;
        if diag.best_rank.is_some_and(|rank| rank < k) {
            hits += 1;
        }
    }
    if scored == 0 {
        0.0
    } else {
        hits as f64 / scored as f64
    }
}

/// 建索引（把记录变成可检索的候选源）—— 与 `golden_run::build_source` **同口径**。
fn build_source(records: &[IndexRecord]) -> crate::retrieval_pipeline::Bm25LexicalCandidateSource {
    use crate::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};

    let documents = records
        .iter()
        .map(|record| MemoryCandidate {
            id: record.id.clone(),
            layer: "golden".to_string(),
            scope: MemoryScope::Global,
            content: record.text.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance {
                source: "golden_corpus".to_string(),
                source_session: None,
                source_trace: None,
                source_request: None,
            },
        })
        .collect();
    crate::retrieval_pipeline::Bm25LexicalCandidateSource::new(documents)
}

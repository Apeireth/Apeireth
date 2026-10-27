//! `apeireth-memory::quality_metrics` — 设计稿 §6.1 的四个剩余指标。
//!
//! # L1 定位
//!
//! * **时间正确率**：检索返回的记录里，有多少是**在有效期内**的
//!   （`valid_from_ms <= as_of < valid_until_ms`；`valid_until_ms = None` ⇒ 永久有效）。
//! * **矛盾检出率**：**互相矛盾**的记忆，检索时**能不能一起被检出**。
//!
//! # L4 可塑性
//!
//! * **影子胜率**：影子里的候选算法**赢**现役的比例；
//! * **回退时间**：从「决定回退」到「回退完成」的**实际耗时**（毫秒）。
//!
//! # 同文件里的 L2 两项（**保留，不拆走**）
//!
//! [`measure_token_efficiency`]（Token 效率）与 [`measure_citation_coverage`]
//! （Citation coverage）也是 §6.1 的项，与本模块同属「度量台账」这条线
//! ⇒ 留在同一文件里，`scripts/check_quality_metrics.py` 的**代码锚点**不破。
//! 渲染时用 [`render_quality_report_extended`]（六项版）；
//! [`render_quality_report`] 只渲染**上面四项**（契约钦定的签名）。
//!
//! # 三条纪律
//!
//! 1. **测不到就是测不到** ⇒ 拿不到分母时返回 `None`，
//!    **不许**用 `0.0` 或 `1.0` 冒充（`0.0` = 全错、`1.0` = 全对 ⇒ 都是**假话**）；
//! 2. **口径写在数字旁边** ⇒ 例如矛盾检出率**受 k 限制**
//!    （k 小 ⇒ 检出率天然低 ⇒ **不是**检索不行的证据）⇒ 报告里**必须同时给出 k**；
//! 3. **墙钟只用于看趋势** ⇒ 回退时间是**实测**的，但**不做断言依据**
//!    （断言看「回退是否比切换快」这个**比值关系**）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径对照（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、度量、报告、测试齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：未接入 canonical 运行时主路径，生产 **0 调用点**
//!   （只有 example 与测试在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✔（部分）：已在《紅樓夢》**真实语料**上跑过对照
//!   （2026-10-27，见 `examples/quality_metrics_report.rs`），
//!   但**未在第二台机器复现** ⇒ 不宣称跨机器可复现。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::collections::{BTreeMap, BTreeSet};

use crate::golden_set::GoldenCase;

/// 准确度比较的**浮点容差**（吸收表示误差）。
///
/// 与 `shadow_run` 的 `TOLERACY_FLOOR` **同口径** ⇒ 两处对「算出来一样」的判断一致
/// （否则一处说「持平」、另一处说「更优」⇒ 影子裁决与影子胜率会互相打架）。
const SCORE_EPSILON: f64 = 1e-9;

// ============================================================
// L1 定位：时间正确率
// ============================================================

/// 一条记录的**有效期**（时间正确率用；双时间：生效起点 / 失效时间）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Validity {
    /// 生效起点（epoch 毫秒；`None` = 未记录 ⇒ 视为一直有效）。
    pub valid_from_ms: Option<i64>,
    /// 失效时间（epoch 毫秒；`None` = 永久有效）。
    pub valid_until_ms: Option<i64>,
}

impl Validity {
    /// 该记录在 `as_of_ms` 时刻是否**有效**。
    ///
    /// # 口径
    ///
    /// * `valid_from_ms` 有值且 `as_of_ms < valid_from_ms` ⇒ **尚未生效**（不算错）；
    /// * `valid_until_ms` 有值且 `as_of_ms >= valid_until_ms` ⇒ **已失效**（**这是错**）；
    /// * 其余 ⇒ 有效。
    ///
    /// # 为什么是**半开区间** `[from, until)`
    ///
    /// 与仓内 `canonical::domain` 的 `valid_until` 语义**逐字一致**
    /// （`as_of == valid_until` ⇒ **已失效**）⇒ 两处对「到期那一刻算不算有效」
    /// 不会打架。边界测试见 `tests/quality_metrics.rs`。
    pub fn is_valid_at(&self, as_of_ms: i64) -> bool {
        if let Some(from) = self.valid_from_ms {
            if as_of_ms < from {
                return false;
            }
        }
        match self.valid_until_ms {
            Some(until) => as_of_ms < until,
            None => true,
        }
    }

    /// 该记录在 `as_of_ms` 时刻是否**已失效**（**这才是「时间正确率」要抓的错**）。
    ///
    /// # 为什么要有这个单独的判定
    ///
    /// 「尚未生效」（`as_of < valid_from`）与「已失效」（`as_of >= valid_until`）
    /// 在 [`Validity::is_valid_at`] 里**都**返回 `false`，但两者**性质不同**：
    /// 前者是「**还没到时候**」（不算错），后者是「**过期了还被返回**」（**是错**）。
    /// ⇒ 计错时**只**认「已失效」⇒ 不把「尚未生效」误判成时间错误。
    pub fn is_expired_at(&self, as_of_ms: i64) -> bool {
        self.valid_until_ms.is_some_and(|until| as_of_ms >= until)
    }
}

/// **时间正确率**的一次判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemporalVerdict {
    /// 被检查的记录数。
    pub checked: usize,
    /// 其中**已失效**却仍被返回的（**错**）。
    pub stale_returned: usize,
    /// **时间正确率** = 1 - 错/总（`0.0 ~ 1.0`；总为 0 ⇒ `None` 口径见 `Option`）。
    pub correct_rate: Option<f64>,
}

/// 算**时间正确率**。
///
/// # 口径
///
/// * 对每条**被检索返回**的记录（`returned_ids`）⇒ 查它的有效期 ⇒ 判 `as_of_ms` 时是否有效；
/// * **分母 = 被返回的记录数**（不是题数）；
/// * ⚠️ **查不到有效期的记录** ⇒ **不计入分母**（**如实**：无信息不算错）
///   ⇒ 但要在 `checked` 里体现（`checked` 只数**查到了**的）。
///
/// # 三条补充口径（钉死，别自创）
///
/// 1. **同一条记录被重复返回 ⇒ 只算一条**（分母是「**记录数**」；
///    否则同一个错误会被重复计数，把率压低成假的）。
/// 2. **「尚未生效」不算错**（见 [`Validity::is_expired_at`]）⇒ 它进分母、不进错数。
/// 3. **分母为 0 ⇒ `correct_rate = None`**（**不许**当 1.0 冒充完美，
///    也不许当 0 冒充「全错」——那是「没东西可测」）。
///
/// # ⚠️ 这项指标当前的**结构性问题**（诚实交代，2026-10-27 实测）
///
/// 全仓**没有任何写入路径**会给 `episodes` 写 `valid_until_ms`
/// （`episode::put_episode` 一律写 `None` = 永久有效）⇒
/// ⇒ 在真实库上跑出来的时间正确率**恒为 1.0 且无区分力**
/// ⇒ 这个 1.0 **不是**「检索很棒」的证据，只是「**没有记录会过期**」的事实。
pub fn measure_temporal(
    returned_ids: &[String],
    validity: &BTreeMap<String, Validity>,
    as_of_ms: i64,
) -> TemporalVerdict {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut checked = 0usize;
    let mut stale_returned = 0usize;

    for id in returned_ids {
        // 重复返回同一条 ⇒ 只算一条（记录数口径）。
        if !seen.insert(id.as_str()) {
            continue;
        }
        // ⚠️ **查不到有效期 ⇒ 不计入分母**（无信息不算错）⇒ 连 `checked` 都不进。
        let Some(entry) = validity.get(id) else {
            continue;
        };
        checked += 1;
        // 「已失效」才算错；「尚未生效」不算错（口径见函数说明）。
        if entry.is_expired_at(as_of_ms) {
            stale_returned += 1;
        }
    }

    let correct_rate = if checked == 0 {
        None
    } else {
        Some(1.0 - stale_returned as f64 / checked as f64)
    };
    TemporalVerdict {
        checked,
        stale_returned,
        correct_rate,
    }
}

// ============================================================
// L1 定位：矛盾检出率
// ============================================================

/// **矛盾检出率**的一次判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContradictionVerdict {
    /// **矛盾对**总数（语料里挖出的 conflict 题型数）。
    pub total_pairs: usize,
    /// 其中**被检出**的（两条记录**同时**出现在同一次检索结果里）。
    pub detected_pairs: usize,
    /// **矛盾检出率** = 检出/总（`None` = 没有矛盾对可测 ⇒ **不许**当 0）。
    pub detection_rate: Option<f64>,
}

/// 算**矛盾检出率**。
///
/// # 口径
///
/// * `contradiction_pairs` = 矛盾对的两个 id（**每对一组**）；
/// * 「检出」= 同一道题的检索结果里**同时**含这两个 id；
/// * ⚠️ **k 限制了检出上限** ⇒ 若 `k` 小 ⇒ 检出率天然低 ⇒ **不是**检索不行的证据
///   ⇒ 报告里要**同时给出 k**（**口径诚实**）。
///
/// # 「检出」为什么是「**同一道题**里同时出现」
///
/// * 矛盾的价值在于**同时呈现**（人/模型看到两个互相打架的说法才能发现矛盾）
///   ⇒ 「在不同题里分别出现」说明不了「这一次检索能不能把矛盾摆到台面上」；
/// * 不要求**相邻**（中间可以夹别的）⇒ 只要求**同时在结果里**。
///
/// # 对齐口径（钉死）
///
/// * `contradiction_pairs[i]` 对应 `retrieved_ids_per_case[i]`
///   （**逐题对齐** ⇒ 「这道题的检索结果有没有同时含这两个 id」）；
/// * `retrieved_ids_per_case` 比 `contradiction_pairs` **短** ⇒ 缺结果的题
///   **按未检出**算（**不许**默认检出）；
/// * `retrieved_ids_per_case` 比 `contradiction_pairs` **长** ⇒ 多出来的结果忽略
///   （没有对应的矛盾对 ⇒ 无从判定）。
pub fn measure_contradiction(
    contradiction_pairs: &[(String, String)],
    retrieved_ids_per_case: &[Vec<String>],
) -> ContradictionVerdict {
    let total_pairs = contradiction_pairs.len();
    let mut detected_pairs = 0usize;

    for (index, (first, second)) in contradiction_pairs.iter().enumerate() {
        // 缺这道题的结果 ⇒ 未检出（**不许**默认检出）。
        let Some(retrieved) = retrieved_ids_per_case.get(index) else {
            continue;
        };
        if retrieved.contains(first) && retrieved.contains(second) {
            detected_pairs += 1;
        }
    }

    let detection_rate = if total_pairs == 0 {
        None
    } else {
        Some(detected_pairs as f64 / total_pairs as f64)
    };
    ContradictionVerdict {
        total_pairs,
        detected_pairs,
        detection_rate,
    }
}

/// 从考卷题目里取出**矛盾对**（每题一对）。
///
/// # 口径
///
/// * 只有**期望记录 ≥ 2 条**的题才算矛盾对 ⇒ 单记录题**跳过**
///   （「一条记录和谁矛盾」不是矛盾）；
/// * 每题只取**前两条**（矛盾是「两条记录互斥」的形态）。
///
/// # 为什么要这个转换器
///
/// 挖题器（`golden_mining`）的 `conflict` 题型产出的就是「两条互相矛盾的记录」
/// ⇒ 转换后**直接**就是 [`measure_contradiction`] 要的形状
/// ⇒ 调用方不必在两个模块之间手抄一遍对齐逻辑（抄一遍 = 迟早漂移）。
pub fn contradiction_pairs_from_cases(cases: &[GoldenCase]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for case in cases {
        if case.expect_ids.len() < 2 {
            continue;
        }
        pairs.push((case.expect_ids[0].clone(), case.expect_ids[1].clone()));
    }
    pairs
}

// ============================================================
// L2 注入：Token 效率
// ============================================================

/// **Token 效率**的一次判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenEfficiency {
    /// 注入的**总字符数**（**字符是 token 的代理** ⇒ 仓里没有分词器）。
    pub injected_chars: usize,
    /// **被证据支撑**的注入字符数（命中期望片段的那部分）。
    pub supported_chars: usize,
    /// **效率** = 支撑/总（`None` = **没注入任何字符** ⇒ 无从判断）。
    ///
    /// ⚠️ **口径提醒**：字符数**不是**真实 token 数 ⇒ 效率是**相对**指标
    /// （同一次测量内可比），**不与**别家系统的 token 效率直接比较。
    pub efficiency: Option<f64>,
}

/// 算 **Token 效率**（注入里有多少字符是**被证据支撑**的）。
///
/// # 口径
///
/// * `injected_text` = 实际注入给模型的文本；
/// * `supported_fragments` = 期望片段（**证据本体**）⇒ 注入文本里**含有**多少；
/// * **支撑字符数** = 被至少一个期望片段**覆盖**的字符数
///   （按片段在注入文本里的**位置区间**并集算 ⇒ 不重复计数）。
///
/// # 空输入
///
/// ⇒ `efficiency = None`（**不许**当 0 或 1 冒充）。
pub fn measure_token_efficiency(
    injected_text: &str,
    supported_fragments: &[String],
) -> TokenEfficiency {
    let injected_chars = injected_text.chars().count();
    // 覆盖区间（字符下标）⇒ 求并集 ⇒ 不重复计数。
    let mut intervals: Vec<(usize, usize)> = Vec::new();
    for fragment in supported_fragments {
        if fragment.is_empty() {
            continue;
        }
        // ⚠️ **按字符**找（不是字节）⇒ 与 `chars().count()` 同一把尺子。
        let fragment_chars: Vec<char> = fragment.chars().collect();
        let text_chars: Vec<char> = injected_text.chars().collect();
        if fragment_chars.len() > text_chars.len() {
            continue;
        }
        for start in 0..=(text_chars.len() - fragment_chars.len()) {
            if text_chars[start..start + fragment_chars.len()] == fragment_chars[..] {
                intervals.push((start, start + fragment_chars.len()));
                break;
            }
        }
    }
    intervals.sort_unstable();
    let mut supported_chars = 0usize;
    let mut cursor: Option<usize> = None;
    for (start, end) in intervals {
        match cursor {
            Some(current_end) if start <= current_end => {
                // 与上一段重叠 ⇒ 合并。
                if end > current_end {
                    cursor = Some(end);
                    supported_chars += end - current_end;
                }
            }
            _ => {
                supported_chars += end - start;
                cursor = Some(end);
            }
        }
    }
    let efficiency = if injected_chars == 0 {
        None
    } else {
        Some(supported_chars as f64 / injected_chars as f64)
    };
    TokenEfficiency {
        injected_chars,
        supported_chars,
        efficiency,
    }
}

// ============================================================
// L2 注入：Citation coverage
// ============================================================

/// **Citation coverage** 的一次判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CitationCoverage {
    /// 引用总数（**注入里标了出处**的条数）。
    pub total_citations: usize,
    /// 其中**出处真能追回原文**的（= 出处 id 在语料里**存在**）。
    pub resolvable_citations: usize,
    /// **覆盖率** = 可追回/总（`None` = **没有任何引用** ⇒ 无从判断）。
    pub coverage: Option<f64>,
}

/// 算 **Citation coverage**（引用能不能追回原文）。
///
/// # 口径
///
/// * `cited_ids` = 注入里引用到的记录 id；
/// * `known_ids` = 语料里真实存在的 id；
/// * 「可追回」= 引用 id ∈ 已知 id。
pub fn measure_citation_coverage(cited_ids: &[String], known_ids: &[String]) -> CitationCoverage {
    let total_citations = cited_ids.len();
    let mut resolvable_citations = 0usize;
    for id in cited_ids {
        if known_ids.iter().any(|known| known == id) {
            resolvable_citations += 1;
        }
    }
    let coverage = if total_citations == 0 {
        None
    } else {
        Some(resolvable_citations as f64 / total_citations as f64)
    };
    CitationCoverage {
        total_citations,
        resolvable_citations,
        coverage,
    }
}

// ============================================================
// L4 可塑性：影子胜率
// ============================================================

/// **影子胜率**的一次判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowWinVerdict {
    /// 对照次数。
    pub comparisons: usize,
    /// 候选**赢**的次数（准确度更高 **或** 准确度相同但 token 更少）。
    pub challenger_wins: usize,
    /// **影子胜率** = 赢/对照（`None` = 没有对照 ⇒ **不许**当 0）。
    pub win_rate: Option<f64>,
}

/// 算**影子胜率**。
///
/// # 口径（照 §6.4「唯一交换许可」）
///
/// * 候选赢 = 「准确度更高」**或**（「准确度相同」且「token **不更多**」）；
/// * 平局（准确度相同、token 也相同）⇒ **不算赢**（不算输）。
///
/// # 三条补充口径（钉死）
///
/// 1. **平局不算赢**：契约里「token 不更多」与「平局不算赢」两句在
///    「准确度相同 + token 相同」上冲突 ⇒ 按**更具体**的那条落口径
///    ⇒ 实际判定 = 「准确度**严格**更高」**或**「准确度相同（浮点容差内）且 token **严格更少**」。
/// 2. **对照次数 = 四组数组里最短的那组长度**（数组不齐时**不许**多算对照）。
/// 3. **没有对照 ⇒ `win_rate = None`**（**不许**当 0：那不是「候选一场没赢」，
///    是「**没跑过对照**」）。
pub fn measure_shadow_win(
    champion_scores: &[f64],
    challenger_scores: &[f64],
    champion_tokens: &[usize],
    challenger_tokens: &[usize],
) -> ShadowWinVerdict {
    let comparisons = champion_scores
        .len()
        .min(challenger_scores.len())
        .min(champion_tokens.len())
        .min(challenger_tokens.len());

    let mut challenger_wins = 0usize;
    for index in 0..comparisons {
        let champion_score = champion_scores[index];
        let challenger_score = challenger_scores[index];
        let champion_token = champion_tokens[index];
        let challenger_token = challenger_tokens[index];

        let score_delta = challenger_score - champion_score;
        if score_delta > SCORE_EPSILON {
            // 准确度更高 ⇒ 赢（§6.4：准确度是硬指标）。
            challenger_wins += 1;
        } else if score_delta.abs() <= SCORE_EPSILON && challenger_token < champion_token {
            // 准确度持平且 token **更少** ⇒ 赢（§6.4 钦定的**唯一交换许可**）。
            challenger_wins += 1;
        }
        // 其余（准确度更低 / 完全平局）⇒ **不算赢**（也不算输）。
    }

    let win_rate = if comparisons == 0 {
        None
    } else {
        Some(challenger_wins as f64 / comparisons as f64)
    };
    ShadowWinVerdict {
        comparisons,
        challenger_wins,
        win_rate,
    }
}

// ============================================================
// L4 可塑性：回退时间
// ============================================================

/// **回退时间**的一次判定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RollbackTiming {
    /// 回退耗时（毫秒，**墙钟实测**）。
    pub rollback_ms: u64,
    /// 对照的切换耗时（毫秒；**有则比** ⇒ 回退应更快）。
    pub switch_ms: Option<u64>,
    /// 回退是否**比切换快**（`switch_ms = None` ⇒ `None` = 无从比较）。
    pub faster_than_switch: Option<bool>,
}

/// 算**回退时间**。
///
/// # 口径
///
/// * 实测耗时（**墙钟** ⇒ 只用于看趋势，不是断言依据）；
/// * 与切换耗时比较 ⇒ **回退必须更快**（否则「可回退」是假的）。
///
/// # 为什么 `switch_ms = None` 时是 `None` 而不是 `Some(true)`
///
/// 「没有对照的切换耗时」⇒ **无从比较** ⇒ 报 `None`（报告里写「没测到」）。
/// 若默认成 `Some(true)` ⇒ 报告会说「回退更快」——**那是假的**（没比过）。
pub fn measure_rollback_time(rollback_ms: u64, switch_ms: Option<u64>) -> RollbackTiming {
    let faster_than_switch = switch_ms.map(|switch| rollback_ms < switch);
    RollbackTiming {
        rollback_ms,
        switch_ms,
        faster_than_switch,
    }
}

// ============================================================
// 报告
// ============================================================

/// 人话报告（四项）。
///
/// # 口径
///
/// * 每一项缺测 ⇒ 渲染成「**没测到**」（**不许**渲染成 0 或 1.0）；
/// * 报告**必须带 `k`** ⇒ 因为矛盾检出率受 k 限制（不给出 k ⇒ 读的人会被误导）。
pub fn render_quality_report(
    temporal: Option<&TemporalVerdict>,
    contradiction: Option<&ContradictionVerdict>,
    shadow_win: Option<&ShadowWinVerdict>,
    rollback_timing: Option<&RollbackTiming>,
    k: usize,
) -> String {
    render_report(
        temporal,
        contradiction,
        None,
        None,
        shadow_win,
        rollback_timing,
        k,
    )
}

/// 人话报告（**六项版**：多了 L2 的 Token 效率 / Citation coverage）。
///
/// # 与 [`render_quality_report`] 的关系
///
/// * [`render_quality_report`] = 契约钦定的**四项**版（L1 两项 + L4 两项）；
/// * 本函数 = **加挂** L2 两项的版本（同一套渲染逻辑 ⇒ 口径只有一处）。
///
/// ⚠️ **Token 效率那一行必须带「字符是 token 的代理」的提醒**
/// ⇒ 否则读的人会把字符数当真实 token 数 ⇒ 拿它跟别家系统的真实 token 数对齐
/// ⇒ 那是**错的对比**（设计稿 §6.3 的诚实要求同源）。
pub fn render_quality_report_extended(
    temporal: Option<&TemporalVerdict>,
    contradiction: Option<&ContradictionVerdict>,
    token_efficiency: Option<&TokenEfficiency>,
    citation: Option<&CitationCoverage>,
    shadow_win: Option<&ShadowWinVerdict>,
    rollback_timing: Option<&RollbackTiming>,
    k: usize,
) -> String {
    render_report(
        temporal,
        contradiction,
        token_efficiency,
        citation,
        shadow_win,
        rollback_timing,
        k,
    )
}

/// 渲染的**共同实现**（口径只有一处 ⇒ 两版报告不会互相漂移）。
fn render_report(
    temporal: Option<&TemporalVerdict>,
    contradiction: Option<&ContradictionVerdict>,
    token_efficiency: Option<&TokenEfficiency>,
    citation: Option<&CitationCoverage>,
    shadow_win: Option<&ShadowWinVerdict>,
    rollback_timing: Option<&RollbackTiming>,
    k: usize,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("质量度量（设计稿 §6.1 的剩余项）".to_string());
    // ⚠️ **k 放在头部**（不只贴在矛盾检出率那一行）⇒
    // 因为 k 会影响**所有**基于 top-k 的指标（矛盾检出率只是最明显的一个）。
    lines.push(format!("  （检索 k = {k}）"));
    lines.push(String::new());

    lines.push(format!("  L1 时间正确率：{}", temporal_line(temporal)));
    lines.push(format!(
        "  L1 矛盾检出率：{}",
        contradiction_line(contradiction, k)
    ));
    if token_efficiency.is_some() || citation.is_some() {
        lines.push(format!(
            "  L2 Token 效率：{}",
            token_efficiency_line(token_efficiency)
        ));
        lines.push(format!(
            "  L2 Citation coverage：{}",
            citation_line(citation)
        ));
    }
    lines.push(format!("  L4 影子胜率：{}", shadow_line(shadow_win)));
    lines.push(format!("  L4 回退时间：{}", rollback_line(rollback_timing)));

    lines.push(String::new());
    lines.push(
        "  口径：本模块只算「**能不能算**」；生产接线状态见各模块的四级口径标注。".to_string(),
    );
    lines.join("\n")
}

/// 时间正确率那一行。
fn temporal_line(temporal: Option<&TemporalVerdict>) -> String {
    let Some(verdict) = temporal else {
        return "**没测到**".to_string();
    };
    match verdict.correct_rate {
        Some(rate) => format!(
            "{rate:.4}（检查 {} 条 · 失效仍返回 {} 条）",
            verdict.checked, verdict.stale_returned
        ),
        None => "**没测到**（没有查得到有效期的记录 ⇒ 分母为 0）".to_string(),
    }
}

/// 矛盾检出率那一行。
fn contradiction_line(contradiction: Option<&ContradictionVerdict>, k: usize) -> String {
    let Some(verdict) = contradiction else {
        return "**没测到**".to_string();
    };
    if verdict.total_pairs == 0 {
        return "**没测到**（没有矛盾对可测）".to_string();
    }
    match verdict.detection_rate {
        Some(rate) => format!(
            "{rate:.4}（矛盾对 {} · 检出 {} · ⚠️ **k = {k} 限制了检出上限**，\
             k 小时检出率天然低 ⇒ 不是检索不行的证据）",
            verdict.total_pairs, verdict.detected_pairs
        ),
        None => format!("**没测到**（矛盾对 {} · 分母为 0）", verdict.total_pairs),
    }
}

/// Token 效率那一行。
fn token_efficiency_line(token_efficiency: Option<&TokenEfficiency>) -> String {
    let Some(verdict) = token_efficiency else {
        return "**没测到**".to_string();
    };
    match verdict.efficiency {
        Some(rate) => format!(
            "{rate:.4}（注入 {} 字 · 被证据支撑 {} 字 · ⚠️ **字符是 token 的代理** ⇒ 只做相对比较）",
            verdict.injected_chars, verdict.supported_chars
        ),
        None => "**没测到**（没注入字符）".to_string(),
    }
}

/// Citation coverage 那一行。
fn citation_line(citation: Option<&CitationCoverage>) -> String {
    let Some(verdict) = citation else {
        return "**没测到**".to_string();
    };
    match verdict.coverage {
        Some(rate) => format!(
            "{rate:.4}（引用 {} · 可追回 {}）",
            verdict.total_citations, verdict.resolvable_citations
        ),
        None => "**没测到**（没有引用）".to_string(),
    }
}

/// 影子胜率那一行。
fn shadow_line(shadow_win: Option<&ShadowWinVerdict>) -> String {
    let Some(verdict) = shadow_win else {
        return "**没测到**".to_string();
    };
    if verdict.comparisons == 0 {
        return "**没测到**（没有对照）".to_string();
    };
    match verdict.win_rate {
        Some(rate) => format!(
            "{rate:.4}（对照 {} 次 · 候选赢 {} 次）",
            verdict.comparisons, verdict.challenger_wins
        ),
        None => format!("**没测到**（对照 {} 次 · 分母为 0）", verdict.comparisons),
    }
}

/// 回退时间那一行。
fn rollback_line(rollback_timing: Option<&RollbackTiming>) -> String {
    let Some(verdict) = rollback_timing else {
        return "**没测到**".to_string();
    };
    let mut line = format!("{} ms", verdict.rollback_ms);
    match (verdict.switch_ms, verdict.faster_than_switch) {
        (Some(switch_ms), Some(true)) => {
            line.push_str(&format!(
                "（切换 {switch_ms} ms · 回退更快：是 · ⚠️ 墙钟只用于看趋势）"
            ));
        }
        (Some(switch_ms), Some(false)) => {
            line.push_str(&format!(
                "（切换 {switch_ms} ms · 回退更快：**否** ⇒「可回退」存疑 · ⚠️ 墙钟只用于看趋势）"
            ));
        }
        (Some(switch_ms), None) => {
            line.push_str(&format!("（切换 {switch_ms} ms · 无从比较）"));
        }
        (None, _) => {
            line.push_str("（没有对照的切换耗时 ⇒ 无从比较）");
        }
    }
    line
}

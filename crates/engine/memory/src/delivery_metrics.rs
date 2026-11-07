//! `apeireth-memory::delivery_metrics` — **送达率**（检索到了 ⇒ 送到了吗？）。
//!
//! # 为什么要有这个指标
//!
//! 2026-10-27 的实测（`docs/status/injection-oracle-bound-2026-10-27.md`）暴露了一件事：
//! **L2（注入）的瓶颈比 L1（定位）还严重**，而它**一直不被测量**。
//!
//! * 《紅樓夢》`recall@10 = 0.2983` ⇒ 看起来「找到了 30% 的答案」；
//! * 但 **`groundedness = 0.0924`** ⇒ **只有 9.2% 的题**在注入里拿到了有据证据
//!   ⇒ ⇒ **已送达比例 = 31.0%** ⇒ ⇒ **69% 的正确答案「检索到了却没送到模型面前」**。
//!
//! ⇒ 而这个「**漏送**」在现有指标体系里**看不见**
//! （`recall` 只管找没找到，`evidence_precision` 只管片段级比例，
//! `groundedness` 只管题级「有没有一条有据」）
//! ⇒ ⇒ **没有一个数直接说「找到的东西有多少真的送到了」**。
//!
//! # 三条纪律
//!
//! 1. **不混口径**：`recall` 与 `groundedness` 的**分母不同**
//!    （前者按**题**、后者按**题**但判据是「有据」）
//!    ⇒ 本模块**显式**给出两者，并**如实**标出「这是两个口径相除」；
//! 2. **测不到就是测不到** ⇒ 分母为 0 ⇒ `None`（**不许**当 0 或 1 冒充）；
//! 3. **不是产品指标**：「送达率」是**两个口径相除**的**诊断量**
//!    ⇒ **不许**拿它去对外许诺。

use crate::golden_set::Metrics;

/// **送达率**的一次测量。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeliveryVerdict {
    /// `recall@k`（**题级**：标准答案被检索到的比例）。
    pub recall_at_k: f64,
    /// `groundedness`（**题级**：至少拿到一条**有据证据**的比例）。
    pub groundedness: f64,
    /// **送达率** = `groundedness / recall_at_k`
    /// （**`None`** = `recall_at_k = 0` ⇒ 无从判断 ⇒ **不许**当 0）。
    ///
    /// # 口径（**必须**读懂再用）
    ///
    /// * 它衡量的是「**检索到的东西里，有多少真的送到了模型面前**」；
    /// * ⚠️ **它可以 > 1.0** ⇒ 因为 `groundedness` 的判据是「**有据**」
    ///   （片段里含期望片段）而 `recall` 的判据是「**命中标准答案记录**」
    ///   ⇒ 两者**不是**同一个事件 ⇒ **比值只是诊断量，不是概率**。
    pub delivery_rate: Option<f64>,
    /// **漏送比例** = 1 − 送达率（**`None`** = 无从判断）。
    pub missed_ratio: Option<f64>,
}

/// 算**送达率**。
///
/// # 口径
///
/// * 输入 = 一次跑批的 [`Metrics`]（**同一个 k、同一次测量** ⇒ 口径一致）；
/// * `recall_at_k = 0` ⇒ **无法判断** ⇒ 返回 `None`（**不许**当 0）。
///
/// # ⚠️ 这个数**不是**「模型答对率」
///
/// 它衡量的是**证据有没有送到**，**不衡量**模型有没有用好证据。
pub fn measure_delivery(metrics: &Metrics) -> DeliveryVerdict {
    let recall = metrics.recall_at_k;
    let groundedness = metrics.groundedness;
    let delivery_rate = if recall > 0.0 {
        Some(groundedness / recall)
    } else {
        None
    };
    let missed_ratio = delivery_rate.map(|rate| 1.0 - rate);
    DeliveryVerdict {
        recall_at_k: recall,
        groundedness,
        delivery_rate,
        missed_ratio,
    }
}

/// 人话报告（**送达率** + **口径提醒**）。
pub fn render_delivery_report(verdict: &DeliveryVerdict, k: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("证据送达率（k = {k}）"));
    lines.push(String::new());
    lines.push(format!(
        "  recall@{} = {:.4}（标准答案**被检索到**的比例）",
        k, verdict.recall_at_k
    ));
    lines.push(format!(
        "  groundedness = {:.4}（**至少拿到一条有据证据**的比例）",
        verdict.groundedness
    ));
    match verdict.delivery_rate {
        Some(rate) => {
            lines.push(format!(
                "  **送达率 = {:.4}**（{:.1}%）",
                rate,
                rate * 100.0
            ));
            lines.push(format!(
                "  ⇒ **漏送 {:.1}%**（检索到了却没送到模型面前）",
                (1.0 - rate) * 100.0
            ));
        }
        None => {
            lines.push("  送达率：**没测到**（recall = 0 ⇒ 无从判断；**不许**当成 0）".to_string())
        }
    }
    lines.push(String::new());
    lines.push("  ⚠️ 口径提醒：".to_string());
    lines.push("    · 送达率是**两个口径相除**（recall 的判据是「命中标准答案记录」，".to_string());
    lines.push("      groundedness 的判据是「片段里含期望片段」）⇒ **它可以 > 1.0**".to_string());
    lines.push("      ⇒ ⇒ 它是**诊断量**，**不是**概率，**更不是**「模型答对率」。".to_string());
    lines.join("\n")
}

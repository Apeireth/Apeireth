//! `delivery_metrics` 的测试 —— 「**检索到了 ⇒ 送到了吗**」。
//!
//! **核心纪律**：
//! * **测不到就是测不到**（`recall = 0` ⇒ `None`，**不许**当 0）；
//! * **两个口径相除** ⇒ **可以 > 1.0** ⇒ **不是**概率；
//! * **不是**「模型答对率」。

use apeireth_memory::delivery_metrics::{measure_delivery, render_delivery_report};
use apeireth_memory::golden_set::Metrics;

fn metrics(recall: f64, groundedness: f64) -> Metrics {
    Metrics {
        recall_at_k: recall,
        ndcg_at_k: recall,
        evidence_precision: groundedness,
        groundedness,
        skipped_cases: 0,
        total_cases: 100,
    }
}

#[test]
fn delivery_rate_is_groundedness_over_recall() {
    // 真实语料实测：0.0924 ÷ 0.2983 = 0.310 ⇒ **31% 已送达** ⇒ 69% 漏送。
    let verdict = measure_delivery(&metrics(0.2983, 0.0924));
    let rate = verdict.delivery_rate.expect("有分母");
    assert!((rate - 0.0924 / 0.2983).abs() < 1e-9, "实际 {rate}");
    assert!((verdict.missed_ratio.expect("有值") - 0.6903).abs() < 1e-3);
}

#[test]
fn delivery_rate_is_none_when_recall_is_zero() {
    // ⚠️ **不许**当 0（那会说「一个都没送到」，而真相是「无从判断」）。
    let verdict = measure_delivery(&metrics(0.0, 0.0));
    assert_eq!(verdict.delivery_rate, None);
    assert_eq!(verdict.missed_ratio, None);
}

#[test]
fn delivery_rate_can_exceed_one() {
    // ⚠️ 两个口径**不是**同一个事件 ⇒ 比值**可以 > 1.0**
    // （groundedness 的判据是「有据」，recall 的判据是「命中标准答案记录」）。
    let verdict = measure_delivery(&metrics(0.2983, 0.3761));
    let rate = verdict.delivery_rate.expect("有分母");
    assert!(rate > 1.0, "实际 {rate}");
    assert!(
        verdict.missed_ratio.expect("有值") < 0.0,
        "漏送比例会是负数 ⇒ 如实"
    );
}

#[test]
fn delivery_rate_is_one_when_everything_retrieved_is_grounded() {
    let verdict = measure_delivery(&metrics(0.5, 0.5));
    assert_eq!(verdict.delivery_rate, Some(1.0));
    assert_eq!(verdict.missed_ratio, Some(0.0));
}

#[test]
fn delivery_rate_keeps_both_source_numbers() {
    let verdict = measure_delivery(&metrics(0.6875, 0.4758));
    assert_eq!(verdict.recall_at_k, 0.6875);
    assert_eq!(verdict.groundedness, 0.4758);
}

#[test]
fn report_states_the_numbers_and_the_caliber_warning() {
    let verdict = measure_delivery(&metrics(0.2983, 0.0924));
    let text = render_delivery_report(&verdict, 10);
    for needle in [
        "recall@10",
        "0.2983",
        "0.0924",
        "送达率",
        "漏送",
        "可以 > 1.0",
    ] {
        assert!(text.contains(needle), "报告里应含 {needle}：\n{text}");
    }
}

#[test]
fn report_marks_unmeasurable_delivery_as_not_measured() {
    let verdict = measure_delivery(&metrics(0.0, 0.0));
    let text = render_delivery_report(&verdict, 10);
    assert!(text.contains("没测到"), "报告里应标明「没测到」：\n{text}");
}

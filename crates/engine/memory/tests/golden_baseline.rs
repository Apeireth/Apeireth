//! `golden_baseline` 的测试：**基线**能不能真的把「分数掉了」抓出来。
//!
//! # 这个模块在四级口径里的位置（能力断言的口径）
//!
//! - **IMPLEMENTED** ✔：类型、读写、校验、比较齐备（本文件逐条钉住）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有门禁脚本与本测试在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✔（部分）：已在本机**真实语料**上跑过一遍
//!   （2026-10-26），**未在第二台机器复现** ⇒ 不宣称跨机器可复现。
//!
//! # 为什么这些测试重要
//!
//! 「准确度不许掉」这条纪律，**全靠比较函数真的能认出掉分**。
//! ⇒ 比较函数一旦放宽（把掉了当没掉）⇒ 门禁**永远绿** ⇒ 纪律变成空话。
//! ⇒ 每条「掉了 / 没掉 / 不可比」的判定都要有测试钉住。

use apeireth_memory::golden_baseline::{
    compare_to_baseline, corpus_baseline, parse_baseline, to_json, CorpusBaseline, GoldenBaseline,
};
use apeireth_memory::golden_set::{check_no_regression, Metrics};

/// 契约里的基线文件形状（**与 `scripts/golden-baseline.json` 同形**）。
const CANONICAL_JSON: &str = r#"{
  "version": "internal-2026-10-26",
  "k": 10,
  "corpora": [
    { "name": "redchamber", "recall_at_k": 0.2983, "ndcg_at_k": 0.1615, "cases": 476 },
    { "name": "watershed", "recall_at_k": 0.6794, "ndcg_at_k": 0.4796, "cases": 248 }
  ]
}"#;

/// 造一份「只含两项检索指标」的当前分数。
fn metrics(recall_at_k: f64, ndcg_at_k: f64) -> Metrics {
    Metrics {
        recall_at_k,
        ndcg_at_k,
        ..Metrics::default()
    }
}

/// 一份只有一条语料的基线。
fn baseline(recall_at_k: f64, ndcg_at_k: f64, cases: usize) -> GoldenBaseline {
    GoldenBaseline::new(
        "internal-test-v1",
        10,
        vec![CorpusBaseline::new(
            "redchamber",
            recall_at_k,
            ndcg_at_k,
            cases,
        )],
    )
}

#[test]
fn parse_baseline_reads_the_canonical_shape() {
    let parsed = parse_baseline(CANONICAL_JSON).expect("契约里的形状必须能解析");
    assert_eq!(parsed.version, "internal-2026-10-26");
    assert_eq!(parsed.k, 10);
    assert_eq!(parsed.corpora.len(), 2);
    let red = corpus_baseline(&parsed, "redchamber").expect("红楼市基线在");
    assert_eq!(red.cases, 476);
    assert!((red.metrics.recall_at_k - 0.2983).abs() < 1e-12);
    assert!((red.metrics.ndcg_at_k - 0.1615).abs() < 1e-12);
    let water = corpus_baseline(&parsed, "watershed").expect("水浒传基线在");
    assert_eq!(water.cases, 248);
}

#[test]
fn parse_baseline_rejects_broken_json_without_panicking() {
    // ⚠️ 格式错误 ⇒ **如实报错**（不 panic、不静默当空基线）。
    let error = parse_baseline("{ 这不是 JSON").expect_err("坏 JSON 必须报错");
    assert!(!error.is_empty(), "错误说明不能是空串");
    let error = parse_baseline("[]").expect_err("顶层不是对象必须报错");
    assert!(error.contains("对象"), "错误要说明期望什么：{error}");
}

#[test]
fn parse_baseline_rejects_missing_required_fields() {
    // 缺顶层字段。
    assert!(
        parse_baseline(r#"{"k": 10, "corpora": []}"#).is_err(),
        "缺 version"
    );
    assert!(
        parse_baseline(r#"{"version": "v1", "corpora": []}"#).is_err(),
        "缺 k"
    );
    assert!(
        parse_baseline(r#"{"version": "v1", "k": 10}"#).is_err(),
        "缺 corpora"
    );
    // 缺语料字段（尤其 **cases** —— 没有题数就分不清「分数变了」与「题变了」）。
    assert!(
        parse_baseline(
            r#"{"version":"v1","k":10,"corpora":[{"name":"x","ndcg_at_k":0.1,"cases":3}]}"#
        )
        .is_err(),
        "缺 recall_at_k 必须报错"
    );
    assert!(
        parse_baseline(
            r#"{"version":"v1","k":10,"corpora":[{"name":"x","recall_at_k":0.1,"ndcg_at_k":0.1}]}"#
        )
        .is_err(),
        "缺 cases 必须报错"
    );
}

#[test]
fn parse_baseline_rejects_duplicate_corpus_names() {
    // 重名 ⇒ 不知道该跟哪条比 ⇒ 报错（不许默默取第一条）。
    let text = r#"{"version":"v1","k":10,"corpora":[
        {"name":"x","recall_at_k":0.1,"ndcg_at_k":0.1,"cases":3},
        {"name":"x","recall_at_k":0.2,"ndcg_at_k":0.2,"cases":4}]}"#;
    let error = parse_baseline(text).expect_err("重名必须报错");
    assert!(error.contains("重名"), "错误要说明是重名：{error}");
}

#[test]
fn corpus_baseline_finds_or_reports_absence() {
    let parsed = parse_baseline(CANONICAL_JSON).expect("解析");
    assert!(corpus_baseline(&parsed, "redchamber").is_some());
    assert!(corpus_baseline(&parsed, "not-there").is_none());
}

#[test]
fn compare_to_baseline_reports_nothing_when_scores_improve() {
    // **涨了 ⇒ 掉了的列表必须为空**（闸门只管「不许掉」，不管「涨没涨」）。
    let base = baseline(0.2983, 0.1615, 476);
    let (dropped, incomparable) =
        compare_to_baseline(&base, "redchamber", &metrics(0.35, 0.20), 476);
    assert!(dropped.is_empty(), "涨了不该报掉：{dropped:?}");
    assert!(
        incomparable.is_empty(),
        "题数相同就该可比：{incomparable:?}"
    );
}

#[test]
fn compare_to_baseline_reports_the_drop_and_how_much() {
    // 相对劣化 > 1% ⇒ 算「掉了」，且**必须说清跌了多少**。
    let base = baseline(0.2983, 0.1615, 476);
    let (dropped, incomparable) =
        compare_to_baseline(&base, "redchamber", &metrics(0.2500, 0.1500), 476);
    assert_eq!(dropped.len(), 2, "两项都掉了：{dropped:?}");
    assert!(
        incomparable.is_empty(),
        "题数相同就该可比：{incomparable:?}"
    );
    let recall = dropped
        .iter()
        .find(|text| text.starts_with("recall_at_k"))
        .expect("有 recall 的说明");
    assert!(recall.contains("0.048300"), "要写出跌了多少：{recall}");
    assert!(recall.contains("相对劣化"), "要写出相对劣化：{recall}");
    let ndcg = dropped
        .iter()
        .find(|text| text.starts_with("ndcg_at_k"))
        .expect("有 nDCG 的说明");
    assert!(ndcg.contains("相对劣化"), "要写出相对劣化：{ndcg}");
}

#[test]
fn compare_to_baseline_tolerates_a_drop_within_one_percent() {
    // **相对劣化 ≤ 1% ⇒ 不算掉**（与 `check_no_regression` 的容忍度一致）。
    let base = baseline(0.5000, 0.5000, 100);
    let (dropped, incomparable) =
        compare_to_baseline(&base, "redchamber", &metrics(0.4950, 0.4950), 100);
    assert!(dropped.is_empty(), "1% 以内的波动不算掉：{dropped:?}");
    assert!(incomparable.is_empty(), "{incomparable:?}");
    // 边界：恰好 1%（0.5 → 0.495）放行；1.1% 拦下。
    let (dropped, _) = compare_to_baseline(&base, "redchamber", &metrics(0.4940, 0.5000), 100);
    assert_eq!(dropped.len(), 1, "超过 1% 必须报掉：{dropped:?}");
}

#[test]
fn compare_to_baseline_marks_different_case_counts_as_incomparable() {
    // **题数不同 ⇒ 不可比 ⇒ 不判掉**（题变了分数就不可比）。
    let base = baseline(0.2983, 0.1615, 476);
    let (dropped, incomparable) =
        compare_to_baseline(&base, "redchamber", &metrics(0.1000, 0.1000), 400);
    assert!(dropped.is_empty(), "题数不同不该判掉：{dropped:?}");
    assert_eq!(
        incomparable.len(),
        1,
        "要有一条不可比说明：{incomparable:?}"
    );
    let note = &incomparable[0];
    assert!(note.contains("不可比"), "{note}");
    assert!(
        note.contains("476") && note.contains("400"),
        "要说清两个题数：{note}"
    );
}

#[test]
fn compare_to_baseline_marks_unknown_corpus_as_incomparable() {
    // 基线里没有这份语料 ⇒ **不可比**（未被覆盖 ≠ 没掉）。
    let base = baseline(0.2983, 0.1615, 476);
    let (dropped, incomparable) = compare_to_baseline(&base, "other", &metrics(0.1, 0.1), 10);
    assert!(dropped.is_empty(), "{dropped:?}");
    assert_eq!(incomparable.len(), 1, "{incomparable:?}");
    assert!(incomparable[0].contains("不可比"), "{:?}", incomparable);
}

#[test]
fn to_json_round_trips_through_parse_baseline() {
    let parsed = parse_baseline(CANONICAL_JSON).expect("解析");
    let rendered = to_json(&parsed);
    let again = parse_baseline(&rendered).expect("自己写出来的必须能读回去");
    assert_eq!(parsed, again, "round-trip 必须相等");
    // 再往返一次仍相等（幂等）。
    assert_eq!(again, parse_baseline(&to_json(&again)).expect("再解析"));
}

#[test]
fn to_json_is_deterministic() {
    let parsed = parse_baseline(CANONICAL_JSON).expect("解析");
    let first = to_json(&parsed);
    let second = to_json(&parsed);
    assert_eq!(first, second, "同一份基线两次渲染必须逐字相等");
}

#[test]
fn to_json_drops_non_baseline_metrics_on_purpose() {
    // 基线**只固化两项检索指标** ⇒ 证据族指标**有意不落盘**。
    // （把注入族的分数塞进检索基线 ⇒ 改注入策略会被误判成「检索掉了」。）
    let mut corpus = CorpusBaseline::new("redchamber", 0.2983, 0.1615, 476);
    corpus.metrics.groundedness = 0.5;
    corpus.metrics.evidence_precision = 0.4;
    let base = GoldenBaseline::new("internal-test-v1", 10, vec![corpus]);
    let rendered = to_json(&base);
    assert!(!rendered.contains("groundedness"), "不该落盘：{rendered}");
    assert!(
        !rendered.contains("evidence_precision"),
        "不该落盘：{rendered}"
    );
    let again = parse_baseline(&rendered).expect("解析");
    assert_eq!(again.corpora[0].metrics.groundedness, 0.0);
    assert_eq!(again.corpora[0].metrics.recall_at_k, 0.2983);
}

#[test]
fn compare_to_baseline_agrees_with_check_no_regression() {
    // ⚠️ **同一个判据必须有两处实现**（库里的比较 + owner 裁定的闸门）⇒
    // 两处对同一组分数的判断必须**一致**（否则一边放行一边拦下 ⇒ 纪律失效）。
    let cases = [
        (0.5000, 0.5000, 0.4950, 0.4950), // 1% 以内 ⇒ 两处都放行
        (0.5000, 0.5000, 0.4900, 0.5000), // 2% ⇒ 两处都拦下
        (0.5000, 0.5000, 0.6000, 0.6000), // 变好 ⇒ 两处都放行
    ];
    for (base_recall, base_ndcg, cur_recall, cur_ndcg) in cases {
        let base = baseline(base_recall, base_ndcg, 10);
        let current = metrics(cur_recall, cur_ndcg);
        let (dropped, _) = compare_to_baseline(&base, "redchamber", &current, 10);
        let gate = check_no_regression(&base.corpora[0].metrics, &current);
        assert_eq!(
            dropped.is_empty(),
            gate.is_ok(),
            "两处判断必须一致：baseline=({base_recall}, {base_ndcg}) current=({cur_recall}, {cur_ndcg})"
        );
    }
}

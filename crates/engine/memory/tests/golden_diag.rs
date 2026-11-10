//! `golden_diag` 的测试：把「召回缺口**分类**」这件事钉死。
//!
//! 重点不在「算得出数」，而在**分类的口径不许漂**：
//! * 四种分类各自**互斥**（同一道题只能落一类）；
//! * **多名次取最好**（最乐观口径 ⇒ 报告不许悲观）；
//! * **名次分位**用**已知序列**算（不靠「看起来对」）；
//! * ⚠️ **报告不许粉饰**：多数题没进候选池时，报告必须说「索引/分词」，
//!   **不许**只说「命中良好」。

use apeireth_memory::golden_diag::{
    diagnose, recall_estimate_at_k, render_report, retrieve_pool, subject_of, summarize, GapClass,
    RecallDiag,
};
use apeireth_memory::golden_set::{GoldenCase, Retrieved};

/// 造一道题。
fn case(id: &str, query: &str, expect_ids: &[&str]) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: query.to_string(),
        expect_ids: expect_ids.iter().map(|item| item.to_string()).collect(),
        expect_fragments: Vec::new(),
        tags: vec!["internal".to_string()],
    }
}

/// 造一个**候选池**（未按 k 截断的有序结果）。
fn pool(ids: &[&str]) -> Vec<Retrieved> {
    vec![Retrieved {
        ids: ids.iter().map(|item| item.to_string()).collect(),
        fragments: Vec::new(),
    }]
}

/// 造一条诊断（直接给名次 ⇒ 用来测汇总与分位）。
fn diag(case_id: &str, expect_ids: &[&str], best_rank: Option<usize>) -> RecallDiag {
    RecallDiag {
        case_id: case_id.to_string(),
        expect_ids: expect_ids.iter().map(|item| item.to_string()).collect(),
        best_rank,
        in_candidate_pool: best_rank.is_some(),
        returned: 12,
        subject: "寶玉".to_string(),
    }
}

// ============================================================
// classify：四种分类
// ============================================================

#[test]
fn classify_marks_a_hit_when_the_rank_is_inside_k() {
    let item = diag("c1", &["r1"], Some(3));
    assert_eq!(item.classify(10), GapClass::Hit);
    // 边界：名次 = k - 1 仍是命中。
    assert_eq!(diag("c2", &["r1"], Some(9)).classify(10), GapClass::Hit);
}

#[test]
fn classify_marks_ranked_out_when_the_rank_is_beyond_k() {
    let item = diag("c1", &["r1"], Some(10));
    assert_eq!(item.classify(10), GapClass::RankedOut);
    // 边界：名次 = k 就不算命中（0 基 ⇒ 第 11 条）。
    assert_eq!(
        diag("c2", &["r1"], Some(11)).classify(10),
        GapClass::RankedOut
    );
}

#[test]
fn classify_marks_not_in_pool_when_the_target_never_showed_up() {
    let item = diag("c1", &["r1"], None);
    assert_eq!(item.classify(10), GapClass::NotInPool);
    // k 再大也救不回来（压根没进池）。
    assert_eq!(item.classify(1000), GapClass::NotInPool);
}

#[test]
fn classify_marks_no_expectation_when_there_is_no_standard_answer() {
    let item = diag("c1", &[], None);
    assert_eq!(item.classify(10), GapClass::NoExpectation);
    // 即使有名次也没标准答案 ⇒ 仍是「题坏了」。
    assert_eq!(
        diag("c2", &[], Some(0)).classify(10),
        GapClass::NoExpectation
    );
}

#[test]
fn the_four_classes_are_mutually_exclusive() {
    let samples = [
        (diag("a", &["r1"], Some(0)), GapClass::Hit),
        (diag("b", &["r1"], Some(50)), GapClass::RankedOut),
        (diag("c", &["r1"], None), GapClass::NotInPool),
        (diag("d", &[], None), GapClass::NoExpectation),
    ];
    for (item, expected) in samples {
        assert_eq!(item.classify(10), expected);
    }
}

// ============================================================
// diagnose：逐题诊断
// ============================================================

#[test]
fn diagnose_finds_the_rank_of_the_expected_record() {
    let cases = vec![case("c1", "关于寶玉，我们当时记了什么？", &["r2"])];
    let pools = vec![pool(&["r1", "r2", "r3"])];
    let diags = diagnose(&cases, &pools, 10);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].best_rank, Some(1));
    assert!(diags[0].in_candidate_pool);
    assert_eq!(diags[0].returned, 3);
    assert_eq!(diags[0].expect_ids, vec!["r2".to_string()]);
}

#[test]
fn diagnose_takes_the_best_rank_when_several_records_are_expected() {
    // 期望两条：一条在第 3 位、一条在第 1 位 ⇒ 取**最好**的（第 1 位 ⇒ 0 基名次 0）。
    let cases = vec![case("c1", "寶玉和黛玉这两件事是什么关系？", &["r3", "r1"])];
    let pools = vec![pool(&["r1", "r2", "r3"])];
    let diags = diagnose(&cases, &pools, 10);
    assert_eq!(
        diags[0].best_rank,
        Some(0),
        "多名次必须取最好的：{:?}",
        diags[0]
    );
    assert_eq!(diags[0].classify(10), GapClass::Hit);
}

#[test]
fn diagnose_marks_a_case_without_expectations_as_broken() {
    let cases = vec![case("c1", "空题", &[])];
    let pools = vec![pool(&["r1"])];
    let diags = diagnose(&cases, &pools, 10);
    assert_eq!(diags[0].expect_ids.len(), 0);
    assert_eq!(diags[0].classify(10), GapClass::NoExpectation);
    assert!(!diags[0].in_candidate_pool);
}

#[test]
fn diagnose_reports_a_missing_result_row_as_an_empty_pool() {
    // `retrieved` 比 `cases` 短 ⇒ 缺的题按空结果处理（不许 panic、不许悄悄丢掉）。
    let cases = vec![case("c1", "q1", &["r1"]), case("c2", "q2", &["r2"])];
    let pools = vec![pool(&["r1"])];
    let diags = diagnose(&cases, &pools, 10);
    assert_eq!(diags.len(), 2);
    assert_eq!(diags[1].returned, 0);
    assert_eq!(diags[1].classify(10), GapClass::NotInPool);
}

#[test]
fn diagnose_ignores_extra_result_rows() {
    let cases = vec![case("c1", "q1", &["r1"])];
    let pools = vec![pool(&["r1"]), pool(&["r2"])];
    let diags = diagnose(&cases, &pools, 10);
    assert_eq!(diags.len(), 1);
}

#[test]
fn diagnose_is_deterministic() {
    let cases = vec![case("c1", "q1", &["r2"]), case("c2", "q2", &["r9"])];
    let pools = vec![pool(&["r1", "r2"]), pool(&["r9", "r8"])];
    let first = diagnose(&cases, &pools, 10);
    let second = diagnose(&cases, &pools, 10);
    assert_eq!(first, second, "同一输入两次诊断必须逐字相等");
}

// ============================================================
// summarize：计数 + 分位
// ============================================================

#[test]
fn summarize_counts_every_class() {
    let diags = vec![
        diag("c1", &["r1"], Some(0)),  // Hit
        diag("c2", &["r2"], Some(1)),  // Hit
        diag("c3", &["r3"], Some(2)),  // Hit
        diag("c4", &["r4"], Some(40)), // RankedOut
        diag("c5", &["r5"], Some(41)), // RankedOut
        diag("c6", &["r6"], None),     // NotInPool
        diag("c7", &[], None),         // NoExpectation
    ];
    let summary = summarize(&diags, 10);
    assert_eq!(summary.total, 7);
    assert_eq!(summary.hits, 3);
    assert_eq!(summary.ranked_out, 2);
    assert_eq!(summary.not_in_pool, 1);
    assert_eq!(summary.no_expectation, 1);
}

#[test]
fn rank_percentiles_match_the_known_sequence() {
    // 已知序列 [0,1,2,3,4] ⇒ 最近秩法：p25=1、p50=2、p75=3、p95=4、max=4。
    let diags = vec![
        diag("c1", &["r1"], Some(0)),
        diag("c2", &["r2"], Some(1)),
        diag("c3", &["r3"], Some(2)),
        diag("c4", &["r4"], Some(3)),
        diag("c5", &["r5"], Some(4)),
    ];
    let summary = summarize(&diags, 10);
    let stats = summary.rank_percentiles.expect("有名次分布");
    assert_eq!(stats.p50, 2, "中位数必须是中间那个：{stats:?}");
    assert_eq!(stats.p25, 1);
    assert_eq!(stats.p75, 3);
    assert_eq!(stats.p95, 4);
    assert_eq!(stats.max, 4);
}

#[test]
fn rank_percentiles_are_none_for_empty_input() {
    let summary = summarize(&[], 10);
    assert_eq!(summary.total, 0);
    assert!(
        summary.rank_percentiles.is_none(),
        "空输入不许 panic、不许假装有分布"
    );

    // 全都不在池里 ⇒ 也没有名次分布（不是「名次 = 0」）。
    let only_out_of_pool = vec![diag("c1", &["r1"], None)];
    let summary = summarize(&only_out_of_pool, 10);
    assert!(summary.rank_percentiles.is_none());
}

#[test]
fn rank_percentiles_only_count_records_that_entered_the_pool() {
    // 10 道 NotInPool + 2 道在池 ⇒ 分布**只**统计那 2 道（不是把没进池的当「名次 = ∞」）。
    let mut diags: Vec<RecallDiag> = (0..10)
        .map(|i| diag(&format!("out-{i}"), &["r"], None))
        .collect();
    diags.push(diag("in-1", &["r"], Some(5)));
    diags.push(diag("in-2", &["r"], Some(7)));
    let summary = summarize(&diags, 10);
    let stats = summary.rank_percentiles.expect("有名次分布");
    assert_eq!(stats.p50, 5, "分布只统计在池目标：{stats:?}");
    assert_eq!(stats.max, 7);
}

// ============================================================
// render_report：逐类计数 + 如实的结论句
// ============================================================

#[test]
fn render_report_contains_every_class_count() {
    let diags = vec![
        diag("c1", &["r1"], Some(0)),
        diag("c2", &["r2"], Some(1)),
        diag("c3", &["r3"], Some(2)),
        diag("c4", &["r4"], Some(40)),
        diag("c5", &["r5"], None),
        diag("c6", &[], None),
    ];
    let summary = summarize(&diags, 10);
    let text = render_report(&summary, 10);
    assert!(text.contains("命中 Hit：3 题"), "{text}");
    assert!(text.contains("排在 k 外 RankedOut：1 题"), "{text}");
    assert!(text.contains("没进候选池 NotInPool：1 题"), "{text}");
    assert!(text.contains("无期望 NoExpectation：1 题"), "{text}");
}

#[test]
fn render_report_does_not_whitewash_a_pool_miss_dominated_batch() {
    // ⚠️ **不许粉饰**：多数题压根没进候选池 ⇒ 报告必须点名「索引/分词」。
    let mut diags: Vec<RecallDiag> = (0..8)
        .map(|i| diag(&format!("out-{i}"), &["r"], None))
        .collect();
    diags.push(diag("in-1", &["r"], Some(0)));
    diags.push(diag("in-2", &["r"], Some(30)));
    let summary = summarize(&diags, 10);
    let text = render_report(&summary, 10);
    assert!(text.contains("索引/分词"), "报告必须点名索引/分词：{text}");
    assert!(
        !text.contains("命中良好"),
        "命中只有 1/10 ⇒ 不许说「命中良好」：{text}"
    );
    assert!(text.contains("远未达标"), "{text}");
}

#[test]
fn render_report_says_ranking_is_the_bottleneck_when_most_cases_ranked_out() {
    let mut diags: Vec<RecallDiag> = (0..8)
        .map(|i| diag(&format!("out-{i}"), &["r"], Some(20 + i)))
        .collect();
    diags.push(diag("in-1", &["r"], Some(0)));
    diags.push(diag("in-2", &["r"], None));
    let summary = summarize(&diags, 10);
    let text = render_report(&summary, 10);
    assert!(text.contains("排序"), "报告必须点名排序：{text}");
    assert!(text.contains("瓶颈是排序，不是召回"), "{text}");
    assert!(!text.contains("命中良好"), "{text}");
}

#[test]
fn render_report_flags_a_broken_exam_sheet() {
    // 一半的题没有标准答案 ⇒ 「卷子本身有问题」。
    let mut diags: Vec<RecallDiag> = (0..5)
        .map(|i| diag(&format!("bad-{i}"), &[], None))
        .collect();
    diags.push(diag("ok-1", &["r"], Some(0)));
    let summary = summarize(&diags, 10);
    let text = render_report(&summary, 10);
    assert!(text.contains("卷子本身有问题"), "{text}");
}

#[test]
fn render_report_says_nothing_is_broken_when_the_batch_is_healthy() {
    let diags: Vec<RecallDiag> = (0..10)
        .map(|i| diag(&format!("c{i}"), &["r"], Some(i)))
        .collect();
    let summary = summarize(&diags, 10);
    let text = render_report(&summary, 10);
    assert!(text.contains("命中良好"), "{text}");
    assert!(
        !text.contains("索引/分词"),
        "全命中 ⇒ 不该甩锅给索引/分词：{text}"
    );
}

#[test]
fn render_report_survives_an_empty_batch() {
    let summary = summarize(&[], 10);
    let text = render_report(&summary, 10);
    assert!(!text.is_empty());
    assert!(text.contains("结论"), "{text}");
}

// ============================================================
// 辅助件
// ============================================================

#[test]
fn subject_of_extracts_the_subject_from_every_question_template() {
    assert_eq!(subject_of("关于寶玉，我们当时记了什么？"), "寶玉");
    assert_eq!(subject_of("寶玉这件事最后是怎么定的？"), "寶玉");
    assert_eq!(subject_of("寶玉和黛玉这两件事是什么关系？"), "寶玉和黛玉");
    assert_eq!(subject_of("寶玉是什么时候定下来的？"), "寶玉");
    assert_eq!(subject_of("寶玉后来是怎么改的？"), "寶玉");
    // 不认识的题面 ⇒ **如实**退回整条题面（不编一个主体出来）。
    assert_eq!(subject_of("随便一句问话"), "随便一句问话");
}

#[test]
fn recall_estimate_uses_the_best_rank_and_skips_broken_cases() {
    let diags = vec![
        diag("c1", &["r1"], Some(0)),  // 命中
        diag("c2", &["r2"], Some(5)),  // 命中
        diag("c3", &["r3"], Some(50)), // k=10 时不命中
        diag("c4", &[], None),         // 题坏了 ⇒ 不进分母
    ];
    // 分母 = 3（有期望的题）⇒ 2/3。
    let estimate = recall_estimate_at_k(&diags, 10);
    assert!((estimate - 2.0 / 3.0).abs() < 1e-9, "{estimate}");
    // k 提到 60 ⇒ 3/3（但这不是官方 recall：官方按「命中比例」算，多目标题更严）。
    let wide = recall_estimate_at_k(&diags, 60);
    assert!((wide - 1.0).abs() < 1e-9, "{wide}");
}

#[test]
fn retrieve_pool_returns_only_positive_scored_candidates() {
    use apeireth_memory::golden_run::IndexRecord;

    let records = vec![
        IndexRecord {
            id: "r1".to_string(),
            text: "限流阈值今天定在每秒两千次".to_string(),
        },
        IndexRecord {
            id: "r2".to_string(),
            text: "今天天气不错适合出门散步".to_string(),
        },
    ];
    let cases = vec![case("c1", "限流阈值 每秒两千次", &["r1"])];
    let pools = retrieve_pool(&records, &cases, 10).expect("检索");
    assert_eq!(pools.len(), 1);
    let ids = &pools[0][0].ids;
    assert!(ids.contains(&"r1".to_string()), "目标必须在池里：{ids:?}");
    // 零分结果**不算**检索结果（否则「目标恰好排第 1」会假命中）。
    assert!(
        !ids.contains(&"r2".to_string()),
        "零分文档不该进池：{ids:?}"
    );
}

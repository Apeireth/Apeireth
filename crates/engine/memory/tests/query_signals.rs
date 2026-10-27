//! `query_signals` 的测试：把「题面信号增强」的三条纪律**钉死**。
//!
//! 重点不在「算得出来」，而在**口径不许漂**：
//!
//! * **解析**：题面实词去停用词；**主体认不出 ⇒ `None`**（不硬凑）；
//!   `answer_terms` **不含**主体；
//! * **判别力**：主体到处都是 ⇒ ≈0；主体独一份 ⇒ ≈1；**分母为 0** ⇒ 不 panic；
//! * **对照**：**基线**也在结果里；**确定性**（同输入两次逐字相等）；**空卷** ⇒ 空结果；
//! * **报告**：含各方案的 recall 与判别力，且含**两条口径提醒**
//!   （「题面被改写 ⇒ 分数不可与基线直接比」+「判别力是信息量不是效果」）。

use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::query_signals::{
    compare_schemes, measure_discriminability, parse_signals, render_signal_report, scheme_signals,
    scheme_signals_in_batch, tokenize, SchemeOutcome, BASELINE_SCHEME, SCHEMES,
};

// ============================================================
// 造数据
// ============================================================

/// 造一道题。
fn case(id: &str, query: &str, expect_ids: &[&str], fragments: &[&str]) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: query.to_string(),
        expect_ids: expect_ids.iter().map(|s| s.to_string()).collect(),
        expect_fragments: fragments.iter().map(|s| s.to_string()).collect(),
        tags: vec!["internal".to_string()],
    }
}

/// 造一批语料记录（`(id, 正文)`）。
fn records(count: usize, filler: &str) -> Vec<(String, String)> {
    (0..count)
        .map(|index| {
            (
                format!("r{index}"),
                format!("{filler}第{index}條記錄的正文內容。"),
            )
        })
        .collect()
}

// ============================================================
// parse_signals：解析口径
// ============================================================

#[test]
fn parse_signals_drops_stopwords_and_template_words() {
    // 「后来是怎么改的」全是**模板词** ⇒ 不该进实词表。
    let parsed = parse_signals(&case(
        "c1",
        "寶玉后来是怎么改的？",
        &["r1"],
        &["寶玉在園中。"],
    ));
    assert_eq!(parsed.case_id, "c1");
    assert!(
        parsed.terms.contains(&"寶玉".to_string()),
        "主体词必须在实词表里：{:?}",
        parsed.terms
    );
    for banned in ["后来", "来后", "怎么", "怎麼", "改的"] {
        assert!(
            !parsed.terms.contains(&banned.to_string()),
            "模板/停用词不该进实词表（{banned}）：{:?}",
            parsed.terms
        );
    }
}

#[test]
fn parse_signals_returns_none_when_subject_is_unrecognisable() {
    // 题面里**只有模板词** ⇒ 认不出主体 ⇒ **返回 None，不硬凑**。
    let parsed = parse_signals(&case("c1", "后来是怎么改的？", &["r1"], &["任意片段。"]));
    assert!(
        parsed.subject.is_none(),
        "认不出主体时必须返回 None（不硬凑）：{:?}",
        parsed.subject
    );
}

#[test]
fn parse_signals_subject_is_the_longest_content_word() {
    // ASCII 词可以很长 ⇒ 主体应当取**最长**那个。
    let parsed = parse_signals(&case(
        "c1",
        "threshold 寶玉后来是怎么改的？",
        &["r1"],
        &["threshold 與寶玉。"],
    ));
    assert_eq!(
        parsed.subject.as_deref(),
        Some("threshold"),
        "主体必须是题面里最长的实词：{:?}",
        parsed.subject
    );
}

#[test]
fn parse_signals_prefers_the_subject_that_appears_in_the_answer_fragment() {
    // 「关于寶玉」在 bigram 口径下会切出「于寶」这个**跨词碎片**
    // ⇒ 纯按位置会挑错 ⇒ 必须用「答案片段里有没有它」来判。
    let parsed = parse_signals(&case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1"],
        &["寶玉在園中。"],
    ));
    assert_eq!(
        parsed.subject.as_deref(),
        Some("寶玉"),
        "主体必须是答案片段里出现的那个：{:?}",
        parsed.subject
    );
}

#[test]
fn parse_signals_answer_terms_exclude_the_subject() {
    let parsed = parse_signals(&case(
        "c1",
        "寶玉后来是怎么改的？",
        &["r1"],
        &["寶玉帶著限流閾值來的。"],
    ));
    assert_eq!(parsed.subject.as_deref(), Some("寶玉"));
    assert!(
        !parsed.answer_terms.contains(&"寶玉".to_string()),
        "answer_terms 不该含主体：{:?}",
        parsed.answer_terms
    );
    assert!(
        !parsed.answer_terms.is_empty(),
        "答案片段里应该有别的实词可带：{:?}",
        parsed.answer_terms
    );
}

// ============================================================
// measure_discriminability：判别力口径
// ============================================================

#[test]
fn discriminability_is_near_zero_when_subject_is_everywhere() {
    let corpus = records(10, "寶玉出現。");
    let cases = vec![case("c1", "寶玉后来是怎么改的？", &["r1"], &["寶玉出現。"])];
    let measured = measure_discriminability(&cases, &corpus, "subject");
    assert_eq!(measured.total_records, 10);
    assert_eq!(measured.matching_records, 10, "主体到处都是 ⇒ 每条都匹配");
    assert!(
        measured.distinctiveness.abs() < 1e-9,
        "主体到处都是 ⇒ 判别力应当 ≈0（实际 {}）",
        measured.distinctiveness
    );
}

#[test]
fn discriminability_is_near_one_when_subject_is_unique() {
    let mut corpus = records(10, "與正文無關的內容。");
    corpus[3] = ("r3".to_string(), "只有這條提到限流閾值。".to_string());
    let cases = vec![case(
        "c1",
        "限流閾值后来是怎么改的？",
        &["r3"],
        &["限流閾值在這裡。"],
    )];
    let measured = measure_discriminability(&cases, &corpus, "subject");
    assert_eq!(measured.matching_records, 1, "主体独一份 ⇒ 只匹配一条");
    assert!(
        measured.distinctiveness > 0.85,
        "主体独一份 ⇒ 判别力应当 ≈1（实际 {}）",
        measured.distinctiveness
    );
}

#[test]
fn discriminability_does_not_panic_when_denominator_is_zero() {
    let cases = vec![case("c1", "寶玉后来是怎么改的？", &["r1"], &["寶玉。"])];
    let empty: Vec<(String, String)> = Vec::new();
    let measured = measure_discriminability(&cases, &empty, "subject");
    assert_eq!(measured.total_records, 0);
    assert_eq!(measured.distinctiveness, 0.0, "分母为 0 ⇒ 记 0（不 panic）");
}

#[test]
fn discriminability_does_not_panic_on_empty_cases() {
    let corpus = records(3, "正文。");
    let measured = measure_discriminability(&[], &corpus, "subject");
    assert_eq!(measured.signal_count, 0);
    assert_eq!(measured.matching_records, 0);
    assert_eq!(measured.distinctiveness, 0.0);
}

#[test]
fn discriminability_counts_extra_signals_as_a_conjunction() {
    // 加了第二个信号 ⇒ 候选集应当**更小**（合取口径）。
    let corpus = records(10, "寶玉出現。");
    let cases = vec![case(
        "c1",
        "寶玉后来是怎么改的？",
        &["r1"],
        &["寶玉與限流閾值。"],
    )];
    let subject_only = measure_discriminability(&cases, &corpus, "subject");
    let with_anchor = measure_discriminability(&cases, &corpus, "subject+answer-term");
    assert_eq!(subject_only.matching_records, 10);
    assert!(
        with_anchor.matching_records < subject_only.matching_records,
        "加信号后候选集应当更小（{} vs {}）",
        with_anchor.matching_records,
        subject_only.matching_records
    );
    assert!(
        with_anchor.distinctiveness > subject_only.distinctiveness,
        "加信号后判别力应当更高"
    );
}

// ============================================================
// scheme_signals：方案写进题面的词
// ============================================================

#[test]
fn scheme_signals_subject_only_writes_the_subject() {
    let one = case("c1", "寶玉后来是怎么改的？", &["r1"], &["寶玉與限流閾值。"]);
    let signals = scheme_signals(&one, "subject");
    assert_eq!(signals.len(), 1, "仅主体 ⇒ 只写一个信号");
    assert_eq!(signals[0].kind, "主体");
    assert_eq!(signals[0].term, "寶玉");
}

#[test]
fn scheme_signals_adds_one_more_term_for_the_enhanced_schemes() {
    let one = case("c1", "寶玉后来是怎么改的？", &["r1"], &["寶玉與限流閾值。"]);
    for scheme in ["subject+predicate", "subject+answer-term"] {
        let signals = scheme_signals(&one, scheme);
        assert_eq!(signals.len(), 2, "{scheme} ⇒ 应当有两个信号");
        assert_eq!(signals[0].kind, "主体");
    }
}

#[test]
fn scheme_signals_two_subjects_picks_the_second_entity_from_the_query() {
    let one = case(
        "c1",
        "寶玉和黛玉这两件事是什么关系？",
        &["r1", "r2"],
        &["寶玉與黛玉。"],
    );
    let signals = scheme_signals(&one, "two-subjects");
    assert_eq!(signals.len(), 2, "题面里有「A和B」⇒ 应当有两个信号");
    assert_eq!(signals[0].term, "寶玉");
    assert_eq!(signals[1].kind, "第二主体");
    assert_eq!(signals[1].term, "黛玉");
}

#[test]
fn scheme_signals_predicate_is_the_word_right_after_the_subject() {
    // 「谓词/对象」= 主体**之后**的第一个实词（位置口径 ⇒「这件事是什么」）。
    let one = case("c1", "寶玉后来是怎么改的？", &["r1"], &["寶玉限流閾值。"]);
    let signals = scheme_signals(&one, "subject+predicate");
    assert_eq!(signals.len(), 2);
    assert_eq!(signals[1].kind, "谓词");
    assert_eq!(signals[1].term, "限流");
}

#[test]
fn scheme_signals_in_batch_picks_another_question_subject() {
    // 「多主体对照」= 答案片段里那个**别的题也拿它当主体**的词（题集口径）。
    let first = case("c1", "寶玉后来是怎么改的？", &["r1"], &["寶玉與黛玉敘話。"]);
    let second = case("c2", "黛玉后来是怎么改的？", &["r2"], &["黛玉在房中。"]);
    let cases = vec![first.clone(), second];
    let batch = scheme_signals_in_batch(&cases, &first, "two-subjects");
    assert_eq!(batch.len(), 2);
    assert_eq!(batch[1].kind, "第二主体");
    assert_eq!(batch[1].term, "黛玉", "批口径下应当挑中「别的题的主体」");
    // 单题口径拿不到题集 ⇒ 退回兜底（**如实**降级，不是硬凑）。
    let solo = scheme_signals(&first, "two-subjects");
    assert_eq!(solo.len(), 2);
    assert_ne!(solo[1].term, "黛玉", "单题口径不应挑出题集身份");
}

#[test]
fn scheme_signals_in_batch_matches_single_case_for_non_two_subject_schemes() {
    let first = case("c1", "寶玉后来是怎么改的？", &["r1"], &["寶玉限流閾值。"]);
    let second = case("c2", "黛玉后来是怎么改的？", &["r2"], &["黛玉在房中。"]);
    let cases = vec![first.clone(), second];
    for scheme in ["subject", "subject+predicate", "subject+answer-term"] {
        assert_eq!(
            scheme_signals_in_batch(&cases, &first, scheme),
            scheme_signals(&first, scheme),
            "{scheme} 的选词规则与题集无关 ⇒ 两口径必须一致"
        );
    }
}

#[test]
fn schemes_constant_is_the_four_contract_candidates() {
    assert_eq!(
        SCHEMES,
        &[
            "subject",
            "subject+predicate",
            "subject+answer-term",
            "two-subjects"
        ]
    );
}

// ============================================================
// compare_schemes：对照实验口径
// ============================================================

/// 一份小语料 + 两道题（够跑对照，又不至于慢）。
fn small_setup() -> (GoldenSet, Vec<(String, String)>) {
    let corpus = vec![
        ("r1".to_string(), "寶玉與黛玉在園中說話。".to_string()),
        ("r2".to_string(), "限流閾值定在每秒兩千次。".to_string()),
        ("r3".to_string(), "賈母在堂上說了許多話。".to_string()),
        ("r4".to_string(), "夜間雨下得很大。".to_string()),
    ];
    let cases = vec![
        case(
            "c1",
            "寶玉后来是怎么改的？",
            &["r1"],
            &["寶玉與黛玉在園中說話。"],
        ),
        case(
            "c2",
            "限流閾值后来是怎么改的？",
            &["r2"],
            &["限流閾值定在每秒兩千次。"],
        ),
    ];
    (GoldenSet::new("test-v1", cases), corpus)
}

#[test]
fn compare_schemes_contains_the_baseline() {
    let (golden, corpus) = small_setup();
    let outcomes = compare_schemes(&golden, &corpus, 10);
    assert_eq!(
        outcomes.first().map(|outcome| outcome.scheme.as_str()),
        Some(BASELINE_SCHEME),
        "基线（原题面）必须也在结果里：{:?}",
        outcomes
            .iter()
            .map(|o| o.scheme.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(outcomes.len(), SCHEMES.len() + 1, "结果 = 基线 + 四个方案");
}

#[test]
fn compare_schemes_is_deterministic() {
    let (golden, corpus) = small_setup();
    let first = compare_schemes(&golden, &corpus, 10);
    let second = compare_schemes(&golden, &corpus, 10);
    assert_eq!(first, second, "同输入两次 ⇒ 逐字相等");
    assert_eq!(
        render_signal_report(&first, first[0].recall_at_k),
        render_signal_report(&second, second[0].recall_at_k),
        "同输入两次 ⇒ 报告逐字相等"
    );
}

#[test]
fn compare_schemes_on_empty_set_returns_empty_without_panic() {
    let golden = GoldenSet::new("empty-v1", Vec::new());
    let corpus = records(3, "正文。");
    let outcomes = compare_schemes(&golden, &corpus, 10);
    assert!(outcomes.is_empty(), "空卷 ⇒ 空结果");
}

#[test]
fn compare_schemes_subject_only_matches_the_baseline() {
    // 「仅主体」方案不追加任何词 ⇒ 题面**逐字未改** ⇒ 分数必须与基线**完全相同**
    // （这是实验台的**自检**：若不相等 ⇒ 改写逻辑坏了）。
    let (golden, corpus) = small_setup();
    let outcomes = compare_schemes(&golden, &corpus, 10);
    let baseline = outcomes
        .iter()
        .find(|outcome| outcome.scheme == BASELINE_SCHEME)
        .expect("基线行");
    let subject = outcomes
        .iter()
        .find(|outcome| outcome.scheme == "subject")
        .expect("仅主体行");
    assert_eq!(
        baseline.recall_at_k, subject.recall_at_k,
        "仅主体方案不改题面 ⇒ 分数必须与基线相同"
    );
}

#[test]
fn compare_schemes_reports_recall_and_distinctiveness_in_range() {
    let (golden, corpus) = small_setup();
    for outcome in compare_schemes(&golden, &corpus, 10) {
        assert!(
            (0.0..=1.0).contains(&outcome.recall_at_k),
            "recall 必须在 [0,1]：{}",
            outcome.recall_at_k
        );
        assert!(
            (0.0..=1.0).contains(&outcome.distinctiveness),
            "判别力必须在 [0,1]：{}",
            outcome.distinctiveness
        );
    }
}

// ============================================================
// render_signal_report：报告口径
// ============================================================

#[test]
fn report_contains_each_scheme_recall_and_distinctiveness() {
    let outcomes = vec![
        SchemeOutcome {
            scheme: BASELINE_SCHEME.to_string(),
            recall_at_k: 0.2983,
            distinctiveness: 0.2667,
        },
        SchemeOutcome {
            scheme: "subject".to_string(),
            recall_at_k: 0.2983,
            distinctiveness: 0.2667,
        },
        SchemeOutcome {
            scheme: "subject+predicate".to_string(),
            recall_at_k: 0.4120,
            distinctiveness: 0.8000,
        },
    ];
    let report = render_signal_report(&outcomes, 0.2983);
    for outcome in &outcomes {
        assert!(
            report.contains(&outcome.scheme),
            "报告必须含方案名（{}）",
            outcome.scheme
        );
    }
    assert!(
        report.contains("0.2983") && report.contains("0.4120"),
        "报告必须含各方案的 recall：\n{report}"
    );
    assert!(
        report.contains("0.2667") && report.contains("0.8000"),
        "报告必须含各方案的判别力：\n{report}"
    );
}

#[test]
fn report_contains_the_two_caveats() {
    let outcomes = vec![SchemeOutcome {
        scheme: BASELINE_SCHEME.to_string(),
        recall_at_k: 0.1,
        distinctiveness: 0.2,
    }];
    let report = render_signal_report(&outcomes, 0.1);
    assert!(
        report.contains("题面被改写 ⇒ 分数不可与基线直接比"),
        "报告必须含口径提醒①：\n{report}"
    );
    assert!(
        report.contains("判别力是信息量不是效果"),
        "报告必须含口径提醒②：\n{report}"
    );
}

// ============================================================
// 分词口径：与 golden_mining::tokenize 同口径
// ============================================================

#[test]
fn tokenizer_matches_golden_mining_verbatim() {
    let samples = [
        "寶玉后来是怎么改的？",
        "关于寶玉，我们当时记了什么？",
        "限流阈值定在每秒两千次",
        "The quick brown fox jumps over the lazy dog.",
    ];
    for sample in samples {
        assert_eq!(
            tokenize(sample),
            apeireth_memory::golden_mining::tokenize(sample),
            "分词口径必须与 golden_mining::tokenize 逐字一致（样本：{sample}）"
        );
    }
}

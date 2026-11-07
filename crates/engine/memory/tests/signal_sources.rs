//! `signal_sources` 的测试：把「非答案侧信号来源」的纪律**钉死**。
//!
//! 重点不在「算得出来」，而在**口径不许漂**、**不许泄漏答案**：
//!
//! * **不许泄漏**：题面里加了答案字段（`expect_ids` / `expect_fragments`）之后，
//!   各来源**产出的词必须逐字不变**（信号链路**只**吃「语料 + 题面」）；
//! * **如实**：取不到词的来源**照样列出来**（`questions_with_terms = 0` ⇒ 分数 = 基线）；
//! * **确定性**：同输入两次**逐字相等**；
//! * **统一**：每个来源加的词**同样多**（不许为某个来源调权重凑分数）。

use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::signal_sources::{
    compare_sources, cooccurrence_terms, paragraph_head_terms, record_head_terms,
    render_source_report, same_subject_history_terms, signal_for_case, subject_of,
    time_anchor_terms, SourceKind, SourceOutcome, MAX_TERMS_PER_SOURCE, SOURCE_KINDS,
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

/// 一份**可控**的语料（共现口径用）。
///
/// * `寶玉` 出现在 **10** 条里；
/// * `限流` 出现在 **4** 条里，**且这 4 条都提到寶玉** ⇒ 共现强度 1.0、`df=4`（**入选**）；
/// * `府邸` 出现在 **6** 条里，**也都提到寶玉** ⇒ 强度 1.0，但 `df=6` **太宽** ⇒ **被 df 上限筛掉**；
/// * `園中` 出现在 **4** 条里，**都不提寶玉** ⇒ 共现强度 0 ⇒ **被强度筛掉**；
/// * `天氣` 只出现在 **2** 条里 ⇒ `df < 3` ⇒ **被最小 df 筛掉**。
fn cooccurrence_corpus() -> Vec<(String, String)> {
    let mut records: Vec<(String, String)> = Vec::new();
    for index in 0..6 {
        records.push((
            format!("r{index}"),
            format!("正文內容說到寶玉與府邸。第{index}條記錄的正文。"),
        ));
    }
    for index in 6..10 {
        records.push((
            format!("r{index}"),
            format!("正文內容說到寶玉與限流。第{index}條記錄的正文。"),
        ));
    }
    for index in 10..14 {
        records.push((
            format!("r{index}"),
            format!("正文內容說到寶玉。第{index}條記錄的正文。"),
        ));
    }
    for index in 14..18 {
        records.push((
            format!("r{index}"),
            format!("正文內容說到園中。第{index}條記錄的正文。"),
        ));
    }
    for index in 18..20 {
        records.push((
            format!("r{index}"),
            format!("正文內容說到天氣。第{index}條記錄的正文。"),
        ));
    }
    records
}

/// 一份**带明显开头**的小语料（同主题历史 / 开头句用）。
fn head_corpus() -> Vec<(String, String)> {
    vec![
        ("h0".to_string(), "園中黛玉與寶玉敘話了很久。".to_string()),
        ("h1".to_string(), "園中黛玉又來找寶玉說話。".to_string()),
        ("h2".to_string(), "別處的事情與寶玉無關。".to_string()),
        ("h3".to_string(), "夜裡下起雨來。".to_string()),
    ]
}

/// 跑对照实验用的小语料 + 两道题。
fn small_setup() -> (GoldenSet, Vec<(String, String)>) {
    let mut corpus: Vec<(String, String)> = Vec::new();
    // 6 条提到「寶玉」的记录；其中 3 条还提到「限流」
    // ⇒ 共现强度 1.0、`df=3` ⇒ **入选**（语料 12 条 ⇒ df 上限 = 3）。
    for index in 0..6 {
        let extra = if index < 3 {
            "，限流閾值定在每秒兩千次"
        } else {
            ""
        };
        corpus.push((
            format!("r{index}"),
            format!("第{index}條正文說到寶玉{extra}。"),
        ));
    }
    for index in 6..12 {
        corpus.push((
            format!("r{index}"),
            format!("第{index}條正文說到夜裡下起雨來。"),
        ));
    }
    let cases = vec![
        case(
            "c1",
            "寶玉后来是怎么改的？",
            &["r0"],
            &["第0條正文說到寶玉，限流閾值定在每秒兩千次。"],
        ),
        case(
            "c2",
            "限流閾值后来是怎么改的？",
            &["r1"],
            &["第1條正文說到寶玉，限流閾值定在每秒兩千次。"],
        ),
    ];
    (GoldenSet::new("source-v1", cases), corpus)
}

// ============================================================
// cooccurrence_terms：共现口径
// ============================================================

#[test]
fn cooccurrence_terms_returns_the_strongly_associated_word() {
    let corpus = cooccurrence_corpus();
    let terms = cooccurrence_terms("寶玉", &corpus, 3);
    assert!(
        terms.contains(&"限流".to_string()),
        "与主体强共现的词必须在结果里：{terms:?}"
    );
}

#[test]
fn cooccurrence_terms_drops_too_broad_words() {
    // `府邸` 与主体强共现（强度 1.0）但**自身太泛**（df=6 > 语料×0.25）⇒ 必须被筛掉。
    let corpus = cooccurrence_corpus();
    let terms = cooccurrence_terms("寶玉", &corpus, 3);
    assert!(
        !terms.contains(&"府邸".to_string()),
        "自身太泛的词不该入选：{terms:?}"
    );
}

#[test]
fn cooccurrence_terms_drops_words_without_the_subject() {
    // `園中` 与主体**不同现**（强度 0）⇒ 必须被筛掉。
    let corpus = cooccurrence_corpus();
    let terms = cooccurrence_terms("寶玉", &corpus, 3);
    assert!(
        !terms.contains(&"園中".to_string()),
        "不同现的词不该入选：{terms:?}"
    );
}

#[test]
fn cooccurrence_terms_is_empty_when_nothing_co_occurs() {
    let corpus = vec![(
        "only".to_string(),
        "正文內容說到寶玉。第0條記錄的正文。".to_string(),
    )];
    let terms = cooccurrence_terms("寶玉", &corpus, 3);
    assert!(
        terms.is_empty(),
        "找不到共现词 ⇒ 返回空（**不硬凑**）：{terms:?}"
    );
}

#[test]
fn cooccurrence_terms_is_deterministic() {
    let corpus = cooccurrence_corpus();
    let first = cooccurrence_terms("寶玉", &corpus, 3);
    let second = cooccurrence_terms("寶玉", &corpus, 3);
    assert_eq!(first, second, "同输入两次 ⇒ 逐字相等");
}

#[test]
fn cooccurrence_terms_respects_the_max_terms_cap() {
    let corpus = cooccurrence_corpus();
    let terms = cooccurrence_terms("寶玉", &corpus, 1);
    assert!(terms.len() <= 1, "上限必须生效：{terms:?}");
}

#[test]
fn cooccurrence_terms_drops_cross_word_fragments() {
    // 「寶玉敘話」会切出「玉敘」这种**根本不是词**的碎片
    // ⇒ 它只出现在主体旁边 ⇒ 不该被当成「强共现词」。
    let mut corpus: Vec<(String, String)> = Vec::new();
    for index in 0..4 {
        corpus.push((format!("r{index}"), format!("寶玉敘話第{index}次。")));
    }
    for index in 4..20 {
        corpus.push((format!("r{index}"), format!("夜裡下起雨來第{index}次。")));
    }
    let terms = cooccurrence_terms("寶玉", &corpus, 3);
    assert!(
        !terms.contains(&"玉敘".to_string()),
        "跨词边界的碎片不该入选：{terms:?}"
    );
    assert!(
        !terms.contains(&"寶玉".to_string()),
        "主体自己不该入选：{terms:?}"
    );
}

// ============================================================
// same_subject_history_terms：同主题历史口径
// ============================================================

#[test]
fn same_subject_history_terms_takes_words_from_the_record_heads() {
    let corpus = head_corpus();
    let terms = same_subject_history_terms("寶玉", &corpus, 8);
    assert!(
        terms.contains(&"園中".to_string()),
        "同主体记录开头里的词必须在结果里：{terms:?}"
    );
    assert!(
        terms.contains(&"黛玉".to_string()),
        "同主体记录开头里的词必须在结果里：{terms:?}"
    );
    assert!(
        terms.contains(&"敘話".to_string()),
        "同主体记录开头里的词必须在结果里：{terms:?}"
    );
}

#[test]
fn same_subject_history_terms_excludes_the_subject_itself() {
    let corpus = head_corpus();
    let terms = same_subject_history_terms("寶玉", &corpus, 3);
    assert!(
        !terms.contains(&"寶玉".to_string()),
        "主体已在题面里 ⇒ 不该重复加：{terms:?}"
    );
}

#[test]
fn same_subject_history_terms_is_empty_when_the_subject_is_absent() {
    let corpus = head_corpus();
    let terms = same_subject_history_terms("薛蟠", &corpus, 3);
    assert!(
        terms.is_empty(),
        "语料里没有该主体 ⇒ 返回空（**不编造**）：{terms:?}"
    );
}

#[test]
fn same_subject_history_terms_is_deterministic() {
    let corpus = head_corpus();
    assert_eq!(
        same_subject_history_terms("寶玉", &corpus, 3),
        same_subject_history_terms("寶玉", &corpus, 3),
        "同输入两次 ⇒ 逐字相等"
    );
}

// ============================================================
// time_anchor_terms：时间锚口径
// ============================================================

#[test]
fn time_anchor_terms_finds_the_chapter_ordinal() {
    let terms = time_anchor_terms("第三十二回　訴肺腑心迷活寶玉后来是怎么改的？");
    assert!(
        terms.contains(&"第三十二回".to_string()),
        "题面里的「第 N 回」必须被取到：{terms:?}"
    );
}

#[test]
fn time_anchor_terms_finds_relative_time_words() {
    let terms = time_anchor_terms("寶玉三年前是怎么改的？");
    assert!(
        terms.contains(&"三年前".to_string()),
        "题面里的「N 年前」必须被取到：{terms:?}"
    );
    let next = time_anchor_terms("寶玉次日是怎么改的？");
    assert!(
        next.contains(&"次日".to_string()),
        "题面里的「次日」必须被取到：{next:?}"
    );
}

#[test]
fn time_anchor_terms_is_empty_without_time_words() {
    // ⚠️ **不许编造时间词** ⇒ 题面里没有时间信号 ⇒ 必须返回空。
    let terms = time_anchor_terms("寶玉后来是怎么改的？");
    assert!(
        terms.is_empty(),
        "题面里没有时间锚 ⇒ 必须返回空（**不编造**）：{terms:?}"
    );
}

#[test]
fn time_anchor_terms_is_deterministic() {
    let query = "第三十二回　訴肺腑心迷活寶玉后来是怎么改的？";
    assert_eq!(
        time_anchor_terms(query),
        time_anchor_terms(query),
        "同输入两次 ⇒ 逐字相等"
    );
}

// ============================================================
// record_head_terms / paragraph_head_terms：开头句口径
// ============================================================

#[test]
fn record_head_terms_takes_only_the_first_sentence() {
    let content = "寶玉走進園子。第二句裡有別的詞。第三句也有。";
    let terms = record_head_terms(content, 5);
    assert!(
        terms.contains(&"寶玉".to_string()),
        "第一句的实词必须在结果里：{terms:?}"
    );
    assert!(
        !terms.contains(&"第二句".to_string()) && !terms.contains(&"第三句".to_string()),
        "只取第一句 ⇒ 后面的句子不该进来：{terms:?}"
    );
}

#[test]
fn record_head_terms_is_empty_for_empty_content() {
    assert!(record_head_terms("", 3).is_empty(), "空内容 ⇒ 空结果");
    assert!(
        record_head_terms("   \n  ", 3).is_empty(),
        "空白内容 ⇒ 空结果"
    );
}

#[test]
fn record_head_terms_respects_the_max_terms_cap() {
    let content = "寶玉走進園子裡看見一株老梅樹。";
    let terms = record_head_terms(content, 1);
    assert_eq!(terms.len(), 1, "上限必须生效：{terms:?}");
}

#[test]
fn paragraph_head_terms_takes_the_subject_paragraph_head() {
    let content = "第一段與主體無關。\n第二段提到寶玉並且講了園子裡的事。第三段別的內容。";
    let terms = paragraph_head_terms(content, "寶玉", 12);
    assert!(
        terms.contains(&"園子".to_string()),
        "含主体那一段的首句实词必须在结果里：{terms:?}"
    );
    assert!(
        !terms.contains(&"第三段".to_string()),
        "只取目标段落的开头句：{terms:?}"
    );
}

// ============================================================
// 主体口径 + 不泄漏答案
// ============================================================

#[test]
fn subject_of_matches_the_query_only_notion() {
    // 主体 = 题面里**最长**的实词；**期望片段不参与**（接口上就没有）。
    let one = case(
        "c1",
        "寶玉后来是怎么改的？",
        &["r1"],
        &["與主體毫無關係的答案片段。"],
    );
    assert_eq!(subject_of(&one).as_deref(), Some("寶玉"));
}

#[test]
fn source_signals_do_not_depend_on_the_answer_fields() {
    // ⚠️ **不许泄漏答案**：把答案字段（`expect_ids` / `expect_fragments`）换成别的内容
    // ⇒ 各来源**产出的词必须逐字不变**。
    let corpus = head_corpus();
    let honest = case(
        "c1",
        "寶玉后来是怎么改的？",
        &["h0"],
        &["園中黛玉與寶玉敘話了很久。"],
    );
    let tampered = case(
        "c1",
        "寶玉后来是怎么改的？",
        &["h3"],
        &["夜裡下起雨來。完全不同的答案片段。"],
    );
    for kind in SOURCE_KINDS {
        let first = signal_for_case(&honest, &corpus, *kind);
        let second = signal_for_case(&tampered, &corpus, *kind);
        assert_eq!(
            first.terms,
            second.terms,
            "{} 的信号不许受答案字段影响：{:?} vs {:?}",
            kind.label(),
            first.terms,
            second.terms
        );
    }
}

#[test]
fn source_signals_are_deterministic() {
    let corpus = head_corpus();
    let one = case("c1", "寶玉后来是怎么改的？", &["h0"], &["園中。"]);
    for kind in SOURCE_KINDS {
        let first = signal_for_case(&one, &corpus, *kind);
        let second = signal_for_case(&one, &corpus, *kind);
        assert_eq!(first.terms, second.terms, "{} 必须确定", kind.label());
        assert!(first.terms.len() <= MAX_TERMS_PER_SOURCE);
    }
}

// ============================================================
// compare_sources：对照实验口径
// ============================================================

#[test]
fn compare_sources_contains_the_baseline() {
    let (golden, corpus) = small_setup();
    let outcomes = compare_sources(&golden, &corpus, 10);
    assert_eq!(
        outcomes.first().map(|outcome| outcome.kind),
        Some(SourceKind::Baseline),
        "基线必须在结果里（第一行）：{:?}",
        outcomes.iter().map(|o| o.kind).collect::<Vec<_>>()
    );
    assert_eq!(
        outcomes.len(),
        SOURCE_KINDS.len() + 1,
        "结果 = 基线 + 五个来源"
    );
}

#[test]
fn compare_sources_is_deterministic() {
    let (golden, corpus) = small_setup();
    let first = compare_sources(&golden, &corpus, 10);
    let second = compare_sources(&golden, &corpus, 10);
    assert_eq!(first, second, "同输入两次 ⇒ 逐字相等");
    assert_eq!(
        render_source_report(&first, first[0].recall_at_k),
        render_source_report(&second, second[0].recall_at_k),
        "同输入两次 ⇒ 报告逐字相等"
    );
}

#[test]
fn compare_sources_reports_reasonable_coverage() {
    let (golden, corpus) = small_setup();
    let outcomes = compare_sources(&golden, &corpus, 10);
    for outcome in &outcomes {
        if outcome.kind == SourceKind::Baseline {
            continue;
        }
        assert!(
            outcome.questions_covered <= golden.len(),
            "{} 的覆盖题数不该超过卷子题数",
            outcome.kind.label()
        );
        assert!(
            outcome.questions_with_terms <= outcome.questions_covered,
            "{} 的出词题数必须是覆盖题数的子集",
            outcome.kind.label()
        );
    }
    // 语料里确实有「寶玉」⇒ 同主题历史与实体共现**必须**出手（不许全是 0）。
    for kind in [SourceKind::SameSubjectHistory, SourceKind::Cooccurrence] {
        let outcome = outcomes
            .iter()
            .find(|outcome| outcome.kind == kind)
            .expect("来源行");
        assert!(
            outcome.questions_with_terms > 0,
            "{} 在这份语料上必须能取到词（实际 {:?}）",
            kind.label(),
            outcome
        );
    }
}

#[test]
fn compare_sources_delta_is_relative_to_the_baseline() {
    let (golden, corpus) = small_setup();
    let outcomes = compare_sources(&golden, &corpus, 10);
    let baseline = outcomes[0].recall_at_k;
    for outcome in &outcomes[1..] {
        let expected = outcome.recall_at_k - baseline;
        assert!(
            (outcome.delta - expected).abs() < 1e-9,
            "{} 的 delta 必须是相对基线的差",
            outcome.kind.label()
        );
    }
}

#[test]
fn compare_sources_on_empty_set_returns_empty() {
    let golden = GoldenSet::new("empty-v1", Vec::new());
    let corpus = vec![("r0".to_string(), "正文內容說到寶玉。".to_string())];
    let outcomes = compare_sources(&golden, &corpus, 10);
    assert!(outcomes.is_empty(), "空卷 ⇒ 空结果（不 panic）");
}

// ============================================================
// render_source_report：报告口径
// ============================================================

#[test]
fn report_contains_every_source_even_the_ineffective_one() {
    let outcomes = vec![
        SourceOutcome {
            kind: SourceKind::Baseline,
            recall_at_k: 0.2983,
            delta: 0.0,
            questions_covered: 476,
            questions_with_terms: 0,
        },
        SourceOutcome {
            kind: SourceKind::Cooccurrence,
            recall_at_k: 0.3200,
            delta: 0.0217,
            questions_covered: 476,
            questions_with_terms: 210,
        },
        SourceOutcome {
            kind: SourceKind::TimeAnchor,
            recall_at_k: 0.2983,
            delta: 0.0,
            questions_covered: 476,
            questions_with_terms: 0,
        },
    ];
    let report = render_source_report(&outcomes, 0.2983);
    for outcome in &outcomes {
        assert!(
            report.contains(outcome.kind.label()),
            "报告必须含每个来源（含**实测无效的**）：{}",
            outcome.kind.label()
        );
    }
    assert!(
        report.contains("0.2983") && report.contains("0.3200"),
        "报告必须含各来源的 recall：\n{report}"
    );
    assert!(
        report.contains("实测无效"),
        "没取到词的来源必须标明「实测无效」：\n{report}"
    );
}

#[test]
fn report_contains_the_caliber_caveats() {
    let outcomes = vec![SourceOutcome {
        kind: SourceKind::Baseline,
        recall_at_k: 0.2983,
        delta: 0.0,
        questions_covered: 476,
        questions_with_terms: 0,
    }];
    let report = render_source_report(&outcomes, 0.2983);
    assert!(
        report.contains("不含答案信息"),
        "报告必须含口径提醒「不含答案信息」：\n{report}"
    );
    assert!(
        report.contains("不可直接比"),
        "报告必须含口径提醒「与 W-30 的 oracle 上界不可直接比」：\n{report}"
    );
    assert!(
        report.contains("上界"),
        "报告必须说明 W-30 的数字是上界：\n{report}"
    );
}

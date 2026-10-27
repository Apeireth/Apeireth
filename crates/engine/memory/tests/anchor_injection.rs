//! `anchor_injection` 的测试 —— **结构化锚点** + **主体段定位**。
//!
//! **核心纪律**：
//! * **解析不出来就是解析不出来**（空 `Vec` ⇒ **不是**「解析成 0」）；
//! * **定位不到就退回开头**（**不返回空** ⇒ 宁可弱也不空）⇒ 且**必须能看见**这个事实；
//! * **主体段定位取第一段** ⇒ 已知局限 ⇒ **不许**假装它是精确的。

use apeireth_memory::anchor_injection::{
    build_anchored_evidence, chapter_text, find_subject_span, parse_time_anchors,
    render_anchor_report, TimeAnchor, TimeUnit, FIXED_TIME_WORDS,
};

#[test]
fn parses_chapter_anchor_from_query() {
    let anchors = parse_time_anchors("第12回裡，賈寶玉記了什麼？");
    assert_eq!(anchors, vec![TimeAnchor::Chapter(12)]);
}

#[test]
fn parses_chapter_anchor_with_chapter_and_section_units() {
    assert_eq!(
        parse_time_anchors("第3章的內容"),
        vec![TimeAnchor::Chapter(3)]
    );
    assert_eq!(
        parse_time_anchors("第5節說了什麼"),
        vec![TimeAnchor::Chapter(5)]
    );
}

#[test]
fn does_not_parse_chinese_numerals() {
    // ⚠️ **刻意不支持中文数字**（口径写死）⇒ ⇒ **解析不出来就是空**。
    let anchors = parse_time_anchors("第十二回裡記了什麼？");
    assert!(anchors.is_empty(), "中文数字不该被当成锚点：{anchors:?}");
}

#[test]
fn parses_relative_time() {
    let anchors = parse_time_anchors("3年前我們記了什麼？");
    assert_eq!(
        anchors,
        vec![TimeAnchor::Relative {
            amount: 3,
            unit: TimeUnit::Year
        }]
    );
}

#[test]
fn parses_fixed_time_words() {
    let anchors = parse_time_anchors("次日的時候記了什麼？");
    assert_eq!(anchors, vec![TimeAnchor::Word("次日".to_string())]);
}

#[test]
fn returns_empty_when_no_anchor_is_present() {
    // ⚠️ **空 `Vec`** ⇒ 不是「解析成 0」。
    assert!(parse_time_anchors("關於賈寶玉，我們當時記了什麼？").is_empty());
}

#[test]
fn does_not_hang_on_digits_without_a_unit_suffix() {
    // ⚠️ **死循环回归测试**（今天踩过：数字后不接后缀 ⇒ 游标不前进 ⇒ 无限循环）。
    let anchors = parse_time_anchors("3個字和5年後的事");
    assert_eq!(
        anchors,
        vec![TimeAnchor::Relative {
            amount: 5,
            unit: TimeUnit::Year
        }]
    );
}

#[test]
fn deduplicates_repeated_anchors() {
    let anchors = parse_time_anchors("第12回…第12回…第7回…");
    assert_eq!(
        anchors,
        vec![TimeAnchor::Chapter(12), TimeAnchor::Chapter(7)]
    );
}

#[test]
fn finds_the_paragraph_containing_the_subject() {
    let text = "第一段沒有它。\n這一段有賈寶玉在說話。\n第三段也不對。";
    let span = find_subject_span(text, "賈寶玉", 20).expect("应能定位");
    assert!(
        span.text.contains("賈寶玉"),
        "取到的段应含主体：{}",
        span.text
    );
    assert!(
        span.text.starts_with("這一段"),
        "取到的段应从段首开始：{}",
        span.text
    );
}

#[test]
fn subject_span_start_is_a_correct_offset() {
    // ⚠️ **偏移 bug 回归测试**（今天踩过：偏移写成段长 ⇒ 注入落在错位置）。
    let text = "第一段沒有它。\n這一段有賈寶玉在說話。\n第三段也不對。";
    let span = find_subject_span(text, "賈寶玉", 20).expect("应能定位");
    let chars: Vec<char> = text.chars().collect();
    assert_eq!(span.start, "第一段沒有它。\n".chars().count());
    assert_eq!(
        span.text,
        chars[span.start..span.start + span.text.chars().count()]
            .iter()
            .collect::<String>()
    );
}

#[test]
fn returns_none_when_the_subject_is_absent() {
    assert!(find_subject_span("完全沒有主体的正文", "賈寶玉", 100).is_none());
}

#[test]
fn returns_none_for_empty_inputs() {
    assert!(find_subject_span("", "賈寶玉", 100).is_none());
    assert!(find_subject_span("正文", "", 100).is_none());
    assert!(find_subject_span("正文", "賈寶玉", 0).is_none());
}

#[test]
fn anchored_evidence_uses_the_subject_span_when_found() {
    let text = "第一段沒有它。\n這一段有賈寶玉在說話。\n第三段也不對。";
    let evidence = build_anchored_evidence(text, 12, "賈寶玉", 40);
    assert_eq!(evidence.chapter, 12);
    assert!(evidence.subject_span_found);
    assert!(evidence.evidence.contains("賈寶玉"));
}

#[test]
fn anchored_evidence_falls_back_to_the_head_and_says_so() {
    // ⚠️ **不许返回空** ⇒ 退回开头；且**必须能看见**这是兜底。
    let evidence = build_anchored_evidence("正文开头的内容", 3, "不存在的主体", 10);
    assert!(!evidence.subject_span_found);
    assert_eq!(evidence.evidence, "正文开头的内容");
}

#[test]
fn chapter_text_uses_the_callers_mapping() {
    // ⚠️ **口径不漂** ⇒ 编号规则由调用方给 ⇒ 本模块**不猜**。
    let texts = vec![
        "rec-1".to_string(),
        "rec-2".to_string(),
        "rec-3".to_string(),
    ];
    let found = chapter_text(&texts, 2, |text| {
        text.chars()
            .last()
            .and_then(|ch| ch.to_digit(10))
            .map(|digit| digit as usize)
    });
    assert_eq!(found, Some("rec-2".to_string()));
    assert_eq!(chapter_text(&texts, 9, |_| None), None);
}

#[test]
fn report_states_the_anchors_and_the_span_outcome() {
    let anchors = parse_time_anchors("第12回裡，賈寶玉記了什麼？");
    let evidence = build_anchored_evidence("第一段。\n賈寶玉在這裡。", 12, "賈寶玉", 40);
    let text = render_anchor_report("第12回裡，賈寶玉記了什麼？", &anchors, &evidence, "賈寶玉");
    for needle in ["回数/章数 = 12", "命中", "独立字段", "未必是答案段"] {
        assert!(text.contains(needle), "报告里应含 {needle}：\n{text}");
    }
}

#[test]
fn report_marks_a_missing_anchor_as_not_parsed() {
    let anchors = parse_time_anchors("沒有時間錨點的問句");
    let evidence = build_anchored_evidence("正文", 1, "主體", 10);
    let text = render_anchor_report("沒有時間錨點的問句", &anchors, &evidence, "主體");
    assert!(
        text.contains("没解析出来"),
        "报告里应标明「没解析出来」：\n{text}"
    );
}

#[test]
fn fixed_time_words_are_all_parseable() {
    // ⇒ 词表里的每个词**自己都能被认出来**（⇒ 词表与解析器**不脱节**）。
    for word in FIXED_TIME_WORDS {
        let anchors = parse_time_anchors(word);
        assert_eq!(
            anchors,
            vec![TimeAnchor::Word((*word).to_string())],
            "词表里的 {word} 应能被解析出来"
        );
    }
}

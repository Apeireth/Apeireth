//! **LoCoMo 加载器**的测试 —— 形状、口径、以及**必须如实计数的东西**。
//!
//! # 这个测试守护什么
//!
//! `locomo_corpus` 把「真实对话基准」变成「记忆库 + 考卷」
//! ⇒ ⇒ 若它**静默丢题 / 静默改正文**，后面的送达率数字就**全不可信**
//! ⇒ ⇒ 所以这里逐条钉住：
//!
//! 1. **一条记忆 = 一个对话轮次**，正文逐字带说话人前缀；
//! 2. **证据片段必须是正文真子串**（否则永远 ground 不上 ⇒ **沉默偏差**）；
//! 3. **对抗类（答案 `None`）不进考卷**，且**题数如实单独计数**；
//! 4. **缺证据 / 轮次不存在的题** ⇒ 计入 `skipped_cases`（**不静默丢**）；
//! 5. **会话序号按数值排**（`session_2` 在 `session_10` 前）。

use apeireth_memory::locomo_corpus::{self, ADVERSARIAL_CATEGORY, FRAGMENT_MAX_CHARS};

/// 构造一个最小但形状完整的 LoCoMo JSON。
fn fixture() -> String {
    serde_json::json!([
        {
            "sample_id": "conv-test",
            "conversation": {
                "speaker_a": "Alice",
                "speaker_b": "Bob",
                "session_1_date_time": "1:00 pm on 1 May, 2023",
                "session_1": [
                    { "speaker": "Alice", "dia_id": "D1:1", "text": "I adopted a greyhound named Comet." },
                    { "speaker": "Bob", "dia_id": "D1:2", "text": "That is lovely, how old is he?" }
                ],
                "session_2_date_time": "1:00 pm on 2 May, 2023",
                "session_2": [
                    { "speaker": "Alice", "dia_id": "D2:1", "text": "Comet chewed my running shoes today." }
                ],
                "session_10_date_time": "1:00 pm on 10 May, 2023",
                "session_10": [
                    { "speaker": "Bob", "dia_id": "D10:1", "text": "Did you ever find those shoes?" }
                ]
            },
            "qa": [
                { "question": "What is the dog named?", "answer": "Comet", "evidence": ["D1:1"], "category": 4 },
                { "question": "What did the dog chew?", "answer": "running shoes", "evidence": ["D2:1"], "category": 4 },
                { "question": "Who is the dog's owner?", "answer": "Alice", "evidence": ["D1:1", "D1:2"], "category": 1 },
                { "question": "What did Alice eat?", "answer": "None", "evidence": ["D2:1"], "category": 5 },
                { "question": "Question without evidence", "answer": "x", "evidence": [], "category": 4 },
                { "question": "Question pointing nowhere", "answer": "x", "evidence": ["D99:9"], "category": 4 }
            ]
        }
    ])
    .to_string()
}

#[test]
fn one_memory_per_dialogue_turn_with_speaker_prefix() {
    let samples = locomo_corpus::parse(&fixture()).expect("应能解析");
    assert_eq!(samples.len(), 1);
    let sample = &samples[0];
    assert_eq!(sample.sample_id, "conv-test");
    assert_eq!(sample.turn_count(), 4, "四个轮次 ⇒ 四条记忆");
    assert_eq!(
        sample.records[0].text, "Alice: I adopted a greyhound named Comet.",
        "正文必须逐字带说话人前缀"
    );
    assert_eq!(
        sample.records[0].id, "conv-test::D1:1",
        "id 要能反查回 dia_id"
    );
}

#[test]
fn sessions_are_ordered_numerically_not_lexically() {
    // ⚠️ `session_10` 字典序在 `session_2` **之前** ⇒ ⇒ 必须按**数值**排。
    let samples = locomo_corpus::parse(&fixture()).expect("应能解析");
    let ids: Vec<&str> = samples[0]
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect();
    let session_2 = ids
        .iter()
        .position(|id| id.ends_with("D2:1"))
        .expect("D2:1");
    let session_10 = ids
        .iter()
        .position(|id| id.ends_with("D10:1"))
        .expect("D10:1");
    assert!(
        session_2 < session_10,
        "session_2 应排在 session_10 之前（实际 {ids:?}）"
    );
}

#[test]
fn evidence_fragments_are_real_substrings_of_the_record_text() {
    // ⇒ 🎯 若片段不是正文子串 ⇒ **永远 ground 不上** ⇒ ⇒ 无声的 0 分。
    let samples = locomo_corpus::parse(&fixture()).expect("应能解析");
    let sample = &samples[0];
    for case in &sample.cases {
        for fragment in &case.expect_fragments {
            let hit = sample
                .records
                .iter()
                .any(|record| record.text.contains(fragment));
            assert!(hit, "片段 {fragment:?} 不是任何记忆正文的子串");
        }
    }
}

#[test]
fn fragments_are_capped_so_they_can_fit_the_injection_window() {
    // ⇒ 生产注入每条只给 120 字 ⇒ ⇒ 片段超过它就**永远送不到** ⇒ 上限必须真的生效。
    let long_text = "甲".repeat(FRAGMENT_MAX_CHARS * 3);
    let json = serde_json::json!([
        {
            "sample_id": "conv-long",
            "conversation": {
                "speaker_a": "A", "speaker_b": "B",
                "session_1_date_time": "t",
                "session_1": [
                    { "speaker": "A", "dia_id": "D1:1", "text": long_text }
                ]
            },
            "qa": [
                { "question": "问", "answer": "答", "evidence": ["D1:1"], "category": 4 }
            ]
        }
    ])
    .to_string();
    let samples = locomo_corpus::parse(&json).expect("应能解析");
    let fragment = &samples[0].cases[0].expect_fragments[0];
    assert!(
        fragment.chars().count() <= FRAGMENT_MAX_CHARS,
        "片段长度 {} 超过上限 {}",
        fragment.chars().count(),
        FRAGMENT_MAX_CHARS
    );
}

#[test]
fn adversarial_questions_are_excluded_and_counted() {
    // ⚠️ 答案 `None` 的陷阱题 ⇒ 「召回」不是正确指标 ⇒ **不进考卷**，但**必须计数**。
    let samples = locomo_corpus::parse(&fixture()).expect("应能解析");
    let sample = &samples[0];
    assert_eq!(sample.adversarial_cases, 1, "应如实计出 1 道对抗题");
    assert!(
        sample.cases.iter().all(
            |case| !case.id.contains(&format!("cat{ADVERSARIAL_CATEGORY}"))
                || ADVERSARIAL_CATEGORY == 5
        ),
        "考卷里不该混进对抗类"
    );
    // 更直接：对抗题的问句不该出现在考卷里。
    assert!(
        sample
            .cases
            .iter()
            .all(|case| case.query != "What did Alice eat?"),
        "对抗题被误收进考卷了"
    );
}

#[test]
fn unusable_questions_are_counted_not_silently_dropped() {
    // ⇒ 缺证据 1 题 + 证据指向不存在的轮次 1 题 ⇒ ⇒ 都应计入 `skipped_cases`。
    let samples = locomo_corpus::parse(&fixture()).expect("应能解析");
    let sample = &samples[0];
    assert_eq!(
        sample.skipped_cases, 2,
        "缺证据 + 轮次不存在 ⇒ 应如实计 2 题（实际 {}）",
        sample.skipped_cases
    );
    // 能评的题：3 道（四道非对抗题里有两道不可用）。
    assert_eq!(sample.cases.len(), 3, "应剩 3 道可评题");
}

#[test]
fn multi_evidence_question_keeps_all_expected_ids() {
    let samples = locomo_corpus::parse(&fixture()).expect("应能解析");
    let sample = &samples[0];
    let multi = sample
        .cases
        .iter()
        .find(|case| case.expect_ids.len() == 2)
        .expect("应有一道双证据题");
    assert_eq!(multi.expect_ids.len(), 2, "两条证据都要记住");
    assert_eq!(multi.expect_fragments.len(), 2, "两个片段都要记住");
    assert!(multi.tags.iter().any(|tag| tag == "cat:1"), "类别要进 tags");
}

#[test]
fn categories_are_counted() {
    let samples = locomo_corpus::parse(&fixture()).expect("应能解析");
    let per = &samples[0].cases_per_category;
    assert_eq!(per.get(&4), Some(&2), "两道单跳题（{per:?}）");
    assert_eq!(per.get(&1), Some(&1), "一道多跳题（{per:?}）");
}

#[test]
fn malformed_json_is_an_error_not_an_empty_success() {
    // ⇒ 静默返回空 ⇒ 后面会得出「0 分」的假结论 ⇒ ⇒ **必须报错**。
    assert!(locomo_corpus::parse("not json").is_err());
    assert!(locomo_corpus::parse("{}").is_err(), "顶层不是数组 ⇒ 报错");
    assert!(locomo_corpus::parse("[]").is_err(), "空样本 ⇒ 报错");
}

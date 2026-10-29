//! `golden_mining` 的契约测试（挖掘工序的「像人话」防线）。
//!
//! 分四层断言：
//! 1. 分词 / 关键词口径（确定性、可复现）；
//! 2. 五种题型各自**能挖出**与**挖不出**；
//! 3. 全局契约（确定性、出题数上限、短记录不挖、挖不出返回空 Vec）；
//! 4. 「答案片段必在原文里」这条最硬的契约（否则模型答对了也没法判「有据」）。

use std::collections::{BTreeMap, BTreeSet};

use apeireth_memory::golden_mining::{
    key_terms, mine, tokenize, MineRecord, MiningConfig, DECISION_MARKERS, FRAGMENT_MAX_CHARS,
};

/// 造一条记录（测试用简写）。
fn rec(id: &str, content: &str, ts: i64, thread: Option<&str>, title: Option<&str>) -> MineRecord {
    MineRecord {
        id: id.to_string(),
        content: content.to_string(),
        title: title.map(|t| t.to_string()),
        timestamp_ms: ts,
        thread_id: thread.map(|t| t.to_string()),
    }
}

/// 一组「有话题重叠」的语料（跨记录题用）。
fn corpus() -> Vec<MineRecord> {
    vec![
        rec(
            "m1",
            "记忆检索的召回延迟今天压到了十毫秒。压测结果显示召回没有掉点。",
            1_000,
            None,
            Some("召回延迟治理"),
        ),
        rec(
            "m2",
            "召回延迟的压测结果今天也确认了。召回路径没有明显抖动。",
            1_000 + 30 * 24 * 3_600_000,
            None,
            Some("召回延迟复盘"),
        ),
        rec(
            "t1",
            "接入层的限流阈值今天定在每秒两千次。压测跑完没有明显抖动。",
            10_000,
            Some("th-1"),
            Some("限流阈值"),
        ),
        rec(
            "t2",
            "限流阈值最终定在每秒两千次。这个数字是压测跑出来的结论。",
            3_610_000,
            Some("th-1"),
            Some("限流阈值定稿"),
        ),
        rec(
            "d1",
            "今天决定把记忆索引换成 sqlite 全文检索。原因是我们需要更稳的召回。",
            20_000,
            None,
            Some("索引存储选型"),
        ),
    ]
}

// ============================================================
// 一、分词 / 关键词口径
// ============================================================

#[test]
fn tokenize_zh_sentence_yields_bigrams() {
    // 连续 CJK 段切成两两组合。
    assert_eq!(tokenize("记忆检索"), vec!["记忆", "忆检", "检索"]);
    // 标点断段 ⇒ bigram 不跨标点。
    assert_eq!(tokenize("记忆,检索。"), vec!["记忆", "检索"]);
}

#[test]
fn tokenize_en_splits_into_words() {
    assert_eq!(
        tokenize("memory retrieval is fast"),
        vec!["memory", "retrieval", "is", "fast"]
    );
    // 单字符 ASCII 不成词。
    assert_eq!(tokenize("a memory"), vec!["memory"]);
}

#[test]
fn tokenize_drops_punctuation() {
    let tokens = tokenize("记忆,检索。is fast!");
    assert_eq!(tokens, vec!["记忆", "检索", "is", "fast"]);
    assert!(tokens.iter().all(|t| !t.contains(',') && !t.contains('。')));
}

#[test]
fn tokenize_dedups_and_keeps_order() {
    assert_eq!(tokenize("检索 检索 召回 检索"), vec!["检索", "召回"]);
}

#[test]
fn key_terms_orders_by_frequency_then_lexicographic() {
    // 频次降序：记忆/召回 各 2 次、索引 1 次；同频次按字典序（召回 < 记忆）。
    let terms = key_terms("记忆 记忆 召回 召回 索引", 10);
    assert_eq!(terms, vec!["召回", "记忆", "索引"]);
    // max_terms 截断。
    assert_eq!(
        key_terms("记忆 记忆 召回 召回 索引", 2),
        vec!["召回", "记忆"]
    );
}

#[test]
fn key_terms_filters_stopwords_and_short_tokens() {
    let terms = key_terms("the and of memory retrieval", 10);
    assert!(!terms.iter().any(|t| t == "the" || t == "and" || t == "of"));
    assert!(terms.contains(&"memory".to_string()));
    assert!(terms.contains(&"retrieval".to_string()));
    // 含虚词字的 CJK bigram = 跨词边界的碎片 ⇒ 丢掉（否则题目不像人话）。
    assert!(key_terms("们需要更多", 10).is_empty());
    // 纯数字不当问题关键词。
    assert!(!key_terms("2024 2025 索引", 10).contains(&"2024".to_string()));
}

// ============================================================
// 二、五种题型
// ============================================================

#[test]
fn mine_keyword_case_has_fragment_inside_content() {
    let r = rec(
        "r1",
        "今天把记忆检索的索引重构了一遍。新的索引用倒排表做召回，延迟降到十毫秒以下。",
        1_700_000_000_000,
        None,
        Some("索引重构"),
    );
    let cases = mine(&[r.clone()], MiningConfig::default());
    let keyword_case = cases
        .iter()
        .find(|c| c.kind == "keyword")
        .expect("单条记录必须挖出 keyword 题");
    assert_eq!(keyword_case.expect_ids, vec!["r1".to_string()]);
    assert_eq!(keyword_case.expect_fragments.len(), 1);
    let fragment = &keyword_case.expect_fragments[0];
    assert!(
        r.content.contains(fragment.as_str()),
        "答案片段必须真在原文里: {fragment}"
    );
    assert!(fragment.chars().count() <= FRAGMENT_MAX_CHARS);
    assert!(keyword_case.query.contains("索引"));
    assert!(keyword_case.query.starts_with("关于"));
}

#[test]
fn mine_multi_record_case_links_both_records() {
    let a = rec(
        "m1",
        "记忆检索的召回延迟今天压到了十毫秒。压测结果显示召回没有掉点。",
        1_000,
        None,
        Some("召回延迟治理"),
    );
    let b = rec(
        "m2",
        "召回延迟的压测结果今天也确认了。召回路径没有明显抖动。",
        1_000 + 30 * 24 * 3_600_000,
        None,
        Some("召回延迟复盘"),
    );
    let cases = mine(&[a.clone(), b.clone()], MiningConfig::default());
    let multi = cases
        .iter()
        .find(|c| c.kind == "multi_record")
        .expect("共享词足够的两条必须挖出 multi_record 题");
    assert_eq!(multi.expect_ids.len(), 2);
    assert!(multi.expect_ids.contains(&"m1".to_string()));
    assert!(multi.expect_ids.contains(&"m2".to_string()));
    assert_eq!(multi.expect_fragments.len(), 2);
    for (id, fragment) in multi.expect_ids.iter().zip(multi.expect_fragments.iter()) {
        let source = if id == "m1" { &a } else { &b };
        assert!(
            source.content.contains(fragment.as_str()),
            "片段必须真在对应记录里: {fragment}"
        );
    }
    assert!(multi.query.contains("这两件事是什么关系？"));
    // 这两条相隔 30 天（不在时间窗内）⇒ 不该被当成时序题。
    assert!(!cases.iter().any(|c| c.kind == "temporal"));
}

#[test]
fn mine_temporal_case_includes_both_records() {
    let a = rec(
        "t1",
        "接入层的限流阈值今天定在每秒两千次。压测跑完没有明显抖动。",
        0,
        Some("th-1"),
        Some("限流阈值"),
    );
    let b = rec(
        "t2",
        "限流阈值最终定在每秒两千次。这个数字是压测跑出来的结论。",
        3_600_000,
        Some("th-1"),
        Some("限流阈值定稿"),
    );
    // 默认配置（每记录 2 题）就该挖出 temporal 题：这条比「调大预算」更能守住出厂行为。
    let cases = mine(&[a.clone(), b.clone()], MiningConfig::default());
    let temporal = cases
        .iter()
        .find(|c| c.kind == "temporal")
        .expect("时间相邻且同主题必须挖出 temporal 题");
    assert_eq!(temporal.expect_ids.len(), 2);
    assert!(temporal.expect_ids.contains(&"t1".to_string()));
    assert!(temporal.expect_ids.contains(&"t2".to_string()));
    // 较晚那条必须在答案里（答案含时间）。
    assert_eq!(temporal.expect_ids.last().unwrap(), "t2");
    assert!(temporal.query.contains("是什么时候定下来的？"));
    for (id, fragment) in temporal
        .expect_ids
        .iter()
        .zip(temporal.expect_fragments.iter())
    {
        let source = if id == "t1" { &a } else { &b };
        assert!(source.content.contains(fragment.as_str()));
    }
}

#[test]
fn mine_temporal_absent_when_outside_window() {
    let a = rec(
        "t1",
        "接入层的限流阈值今天定在每秒两千次。压测跑完没有明显抖动。",
        0,
        Some("th-1"),
        Some("限流阈值"),
    );
    let b = rec(
        "t2",
        "限流阈值最终定在每秒两千次。这个数字是压测跑出来的结论。",
        30 * 24 * 3_600_000,
        Some("th-1"),
        Some("限流阈值定稿"),
    );
    let config = MiningConfig {
        max_cases_per_record: 4,
        ..MiningConfig::default()
    };
    let cases = mine(&[a.clone(), b.clone()], config);
    assert!(
        !cases.iter().any(|c| c.kind == "temporal"),
        "超出时间窗不得出 temporal 题"
    );
    // 同主题的 multi_record 题仍在 ⇒ 说明是时间窗起作用，不是压根没匹配上。
    assert!(cases.iter().any(|c| c.kind == "multi_record"));
}

#[test]
fn mine_conflict_case_when_contents_differ() {
    let a = rec(
        "c1",
        "记忆索引今天换了新的召回实现。压测脚本跑了两轮。",
        0,
        Some("th-2"),
        Some("索引重构方案"),
    );
    let b = rec(
        "c2",
        "配色方案今天换成了另一套字号规范。文档也换了版本。",
        7_200_000,
        Some("th-2"),
        Some("配色规范定稿"),
    );
    // 默认配置（每记录 2 题）就该挖出 conflict 题。
    let cases = mine(&[a.clone(), b.clone()], MiningConfig::default());
    let conflict = cases
        .iter()
        .find(|c| c.kind == "conflict")
        .expect("同主题但内容不同必须挖出 conflict 题");
    assert_eq!(conflict.expect_ids.len(), 2);
    assert!(conflict.expect_ids.contains(&"c1".to_string()));
    assert!(conflict.expect_ids.contains(&"c2".to_string()));
    assert!(conflict.query.contains("后来是怎么改的？"));
}

#[test]
fn mine_no_conflict_when_contents_almost_identical() {
    let a = rec(
        "d1",
        "记忆索引今天换了新的召回实现。压测脚本跑了两轮。",
        0,
        Some("th-3"),
        Some("索引重构方案"),
    );
    let b = rec(
        "d2",
        "记忆索引今天换了新的召回实现。压测脚本跑了三轮。",
        7_200_000,
        Some("th-3"),
        Some("索引重构方案"),
    );
    let config = MiningConfig {
        max_cases_per_record: 4,
        ..MiningConfig::default()
    };
    let cases = mine(&[a.clone(), b.clone()], config);
    assert!(
        !cases.iter().any(|c| c.kind == "conflict"),
        "内容几乎相同不算冲突（那是重复记录）"
    );
    // 同主题、多记录题照常挖得出来 ⇒ 说明不是压根没匹配上。
    assert!(cases.iter().any(|c| c.kind == "multi_record"));
}

#[test]
fn mine_decision_case_only_with_marker() {
    let with_marker = rec(
        "k1",
        "今天决定把记忆索引换成 sqlite 全文检索。原因是我们需要更稳的召回。",
        0,
        None,
        Some("索引存储选型"),
    );
    let without_marker = rec(
        "k2",
        "今天把记忆索引换成 sqlite 全文检索。新的检索链路已经上线。",
        0,
        None,
        Some("索引存储选型"),
    );

    let cases = mine(&[with_marker.clone()], MiningConfig::default());
    let decision = cases
        .iter()
        .find(|c| c.kind == "decision")
        .expect("含决策词必须挖出 decision 题");
    assert_eq!(decision.expect_ids, vec!["k1".to_string()]);
    assert!(decision.query.contains("这件事最后是怎么定的？"));
    let fragment = &decision.expect_fragments[0];
    assert!(with_marker.content.contains(fragment.as_str()));
    assert!(
        DECISION_MARKERS.iter().any(|m| fragment.contains(m)),
        "答案片段必须含决策词: {fragment}"
    );

    let cases2 = mine(&[without_marker.clone()], MiningConfig::default());
    assert!(
        !cases2.iter().any(|c| c.kind == "decision"),
        "不含决策词不得出 decision 题"
    );
}

// ============================================================
// 三、全局契约
// ============================================================

#[test]
fn mine_is_deterministic_across_calls() {
    let records = corpus();
    let first = mine(&records, MiningConfig::default());
    let second = mine(&records, MiningConfig::default());
    assert_eq!(first, second, "同输入两次必须逐字相等");
    assert!(!first.is_empty(), "语料必须能挖出题");

    // 输入顺序不得影响产出（同一批记忆换个顺序挖，卷子一样）。
    let mut reversed = records.clone();
    reversed.reverse();
    assert_eq!(first, mine(&reversed, MiningConfig::default()));
}

#[test]
fn mine_respects_max_cases_per_record() {
    let records = corpus();
    let config = MiningConfig {
        max_cases_per_record: 1,
        ..MiningConfig::default()
    };
    let cases = mine(&records, config);
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for case in &cases {
        for id in &case.expect_ids {
            *counts.entry(id.as_str()).or_insert(0) += 1;
        }
    }
    assert!(!counts.is_empty(), "语料必须能挖出题");
    for (id, n) in counts {
        assert!(n <= 1, "记录 {id} 出了 {n} 道题，超出每记录上限");
    }
}

#[test]
fn mine_respects_max_records_per_case() {
    let records = corpus();
    let config = MiningConfig {
        max_records_per_case: 1,
        ..MiningConfig::default()
    };
    let cases = mine(&records, config);
    assert!(!cases.is_empty());
    for case in &cases {
        assert!(
            case.expect_ids.len() <= 1,
            "单题最多关联 1 条记录（配的是 1）: {case:?}"
        );
    }
}

#[test]
fn mine_skips_short_records() {
    let short = rec("s1", "太短", 0, None, None);
    let long = rec(
        "s2",
        "记忆索引今天换了新的召回实现。压测脚本跑了两轮。",
        0,
        None,
        Some("索引重构"),
    );
    let cases = mine(&[short.clone(), long.clone()], MiningConfig::default());
    assert!(
        cases
            .iter()
            .all(|c| !c.expect_ids.contains(&"s1".to_string())),
        "正文太短的记录不挖"
    );
    assert!(cases
        .iter()
        .any(|c| c.expect_ids.contains(&"s2".to_string())));
}

#[test]
fn mine_returns_empty_for_unminable_input() {
    // 空输入。
    assert!(mine(&[], MiningConfig::default()).is_empty());
    // 只有一条太短的记录。
    let short = rec("s1", "太短", 0, None, None);
    assert!(mine(&[short], MiningConfig::default()).is_empty());
}

#[test]
fn mine_case_ids_unique_and_sorted() {
    let cases = mine(&corpus(), MiningConfig::default());
    let ids: BTreeSet<&str> = cases.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), cases.len(), "候选 id 必须唯一");
    let mut sorted = cases.clone();
    sorted.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.id.cmp(&b.id)));
    assert_eq!(cases, sorted, "产出必须按 (kind, id) 排序");
}

#[test]
fn mine_fragments_are_always_substrings() {
    let records = corpus();
    let cases = mine(&records, MiningConfig::default());
    assert!(!cases.is_empty());
    for case in &cases {
        assert_eq!(
            case.expect_ids.len(),
            case.expect_fragments.len(),
            "片段必须与标准答案一一对应"
        );
        assert!(!case.expect_ids.is_empty(), "每题至少 1 条标准答案");
        for (id, fragment) in case.expect_ids.iter().zip(case.expect_fragments.iter()) {
            let source = records
                .iter()
                .find(|r| r.id == *id)
                .expect("expect_ids 必须指向输入记录");
            assert!(
                source.content.contains(fragment.as_str()),
                "片段必须是原文子串: {fragment}"
            );
            assert!(fragment.chars().count() <= FRAGMENT_MAX_CHARS);
            assert!(!fragment.trim().is_empty());
        }
    }
}

// ============================================================
// 题面质量：数量短语、主张词分离、词边界截断
// ============================================================

#[test]
fn chinese_quantity_phrases_are_not_question_subjects() {
    // 「数量不是话题」⇒ 「关于两千，我们当时记了什么？」不像人话。
    let terms = key_terms("接入层的限流阈值今天定在每秒两千次。", 20);
    assert!(
        !terms.iter().any(|term| term.contains("两千")),
        "中文数量短语不该当题面关键词：{terms:?}"
    );
    assert!(
        !terms.iter().any(|term| term.contains("三次")),
        "数词+量词也不该：{terms:?}"
    );
}

#[test]
fn numbered_labels_still_work_as_question_subjects() {
    // ⚠️ **本仓的领域词汇大量是编号标签**（「五层」「六层」）⇒
    // 不能把「含数词」一刀切掉，否则会误伤真问题主体。
    let terms = key_terms("六层纵深体系里，D0 到 D5 各层职责不同。", 20);
    assert!(
        terms.iter().any(|term| term.contains("六层")),
        "编号标签应保留：{terms:?}"
    );
}

#[test]
fn same_claim_in_two_records_is_temporal_not_conflict() {
    // 两条记录讲**同一件事**（阈值都是每秒两千次）⇒ 不该被判成「冲突」，
    // 而该挖出「时序题」（较晚那条是定稿）。
    // ⚠️ 这条钉住的是「题面过滤」与「相似度判断」**必须用两套词池**：
    // 数量短语从题面丢，但它是「是不是同一主张」的**关键证据**。
    let a = rec(
        "t1",
        "接入层的限流阈值今天定在每秒两千次。压测跑完没有明显抖动。",
        0,
        Some("th-1"),
        Some("限流阈值"),
    );
    let b = rec(
        "t2",
        "限流阈值最终定在每秒两千次。这个数字是压测跑出来的结论。",
        3_600_000,
        Some("th-1"),
        Some("限流阈值定稿"),
    );
    let cases = mine(&[a, b], MiningConfig::default());
    let temporal = cases
        .iter()
        .find(|case| case.kind == "temporal")
        .expect("同主张 + 时间相邻 ⇒ 该挖出 temporal 题");
    assert_eq!(temporal.expect_ids.len(), 2);
    assert!(!cases.iter().any(|case| case.kind == "conflict"));
}

#[test]
fn conflicting_quantity_values_are_still_detected_as_conflict() {
    // 数量短语对「是否冲突」是**关键证据**：
    // 「两千次」vs「五千次」⇒ 真的改了 ⇒ 该判成冲突。
    let a = rec(
        "c1",
        "接入层的限流阈值今天定在每秒两千次。",
        0,
        Some("th-1"),
        Some("限流阈值"),
    );
    let b = rec(
        "c2",
        "限流阈值最终定在每秒五千次，比原计划放宽很多。",
        3_600_000,
        Some("th-1"),
        Some("限流阈值定稿"),
    );
    let cases = mine(&[a, b], MiningConfig::default());
    assert!(
        cases.iter().any(|case| case.kind == "conflict"),
        "数量不同 ⇒ 该判成冲突：{:?}",
        cases
            .iter()
            .map(|c| (&c.kind, &c.query))
            .collect::<Vec<_>>()
    );
}

#[test]
fn subject_is_clipped_at_a_word_boundary_not_mid_word() {
    // 硬切 16 字会留下半截词（「15-proactive-sch」）⇒ 不像人话。
    let cases = mine(
        &[rec(
            "w1",
            "提交 15-proactive-schedule 与 12-mcp-extension 的说明正文，内容足够长以便挖题。",
            0,
            Some("th-1"),
            Some("15-proactive-schedule 与 12-mcp-extension 的说明"),
        )],
        MiningConfig::default(),
    );
    for case in &cases {
        assert!(
            !case.query.contains("sch是") && !case.query.contains("pro-sch"),
            "不该切出半截词：{}",
            case.query
        );
    }
}

// ============================================================
// 问题主体：来自**正文里的实体**，不是标题
// ============================================================

#[test]
fn question_subject_prefers_entities_from_the_body_over_the_title() {
    // ⚠️ 这条钉住 2026-10-26 的口径修正：
    // 原先用**标题**（提交主题 / 小说回目联）当主体 ⇒ 题目长成
    // 「第三十二回　訴肺腑心迷活寶玉后来是怎么改的？」
    // ⇒ 查询词**不在正文里** ⇒ 检索靠偶然命中、注入按查询选窗口**必然选错**
    // ⇒ 主体改成**正文里真实出现的实体**（人话：「寶玉后来是怎么改的？」）。
    let record = rec(
        "n1",
        "寶玉正在黛玉房中玩耍。賈母進來看見。寶玉笑道：今日天氣正好。",
        0,
        None,
        Some("第三十二回　訴肺腑心迷活寶玉　含恥辱情烈死金釧"),
    );
    let cases = mine(&[record], MiningConfig::default());
    assert!(!cases.is_empty());
    for case in &cases {
        assert!(
            !case.query.contains("第三十二回"),
            "题面不该用回目联当主体：{}",
            case.query
        );
        assert!(
            !case.query.contains("訴肺腑"),
            "题面不该用回目联当主体：{}",
            case.query
        );
    }
}

#[test]
fn generic_terms_are_never_used_as_question_subjects() {
    // ⚠️ 实测：古本的**数量泛称**（「一個」「兩個」）是**全篇最高频**的 bigram
    // （《水滸傳》「兩個」1240 次、「一個」1014 次）⇒ 不滤掉必然被选中
    // ⇒ 题面会变成「一個后来是怎么改的？」——**不像人话**。
    let record = rec(
        "n1",
        "這是一個很長的故事。一個男子走進門來。兩個女子站在那裏。先生問道。夫人答道。",
        0,
        None,
        None,
    );
    let cases = mine(&[record], MiningConfig::default());
    for case in &cases {
        for generic in ["一個", "兩個", "先生", "夫人"] {
            assert!(
                !case.query.contains(generic),
                "泛称不该当主体（{generic}）：{}",
                case.query
            );
        }
    }
}

#[test]
fn separator_runs_are_never_used_as_question_subjects() {
    // ⚠️ 实测：古本的**分隔线**会被 OCR 成 `____` ⇒ 它是高频词
    // ⇒ 不滤掉就会挖出「____后来是怎么改的？」这种**假主体**。
    let record = rec(
        "n1",
        "____________________\n寶玉笑道：今日天氣正好。賈母聽了點頭。\n____________________",
        0,
        None,
        None,
    );
    let cases = mine(&[record], MiningConfig::default());
    for case in &cases {
        assert!(
            !case.query.contains('_'),
            "分隔线不该当主体：{}",
            case.query
        );
    }
}

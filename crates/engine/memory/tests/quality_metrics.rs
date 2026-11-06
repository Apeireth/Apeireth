//! `quality_metrics` 的测试 —— 设计稿 §6.1 的四个剩余指标
//! （L1 时间正确率 / 矛盾检出率，L4 影子胜率 / 回退时间）。
//!
//! **核心纪律**：测不到就是测不到 ⇒ 拿不到分母时返回 `None`
//! ⇒ **不许**用 `0.0` / `1.0` 冒充（**假话**）。
//!
//! 另含同文件里 L2 两项（Token 效率 / Citation coverage）的测试。

use std::collections::BTreeMap;

use apeireth_memory::golden_set::GoldenCase;
use apeireth_memory::quality_metrics::{
    contradiction_pairs_from_cases, measure_citation_coverage, measure_contradiction,
    measure_rollback_time, measure_shadow_win, measure_temporal, measure_token_efficiency,
    render_quality_report, render_quality_report_extended, CitationCoverage, ContradictionVerdict,
    RollbackTiming, ShadowWinVerdict, TemporalVerdict, TokenEfficiency, Validity,
};

// ============================================================
// L1 时间正确率：有效期判定
// ============================================================

#[test]
fn validity_permanent_record_is_always_valid() {
    // `None` / `None` ⇒ **永久有效**（未记录有效期 ⇒ 视为一直有效）。
    let forever = Validity {
        valid_from_ms: None,
        valid_until_ms: None,
    };
    assert!(forever.is_valid_at(0));
    assert!(forever.is_valid_at(1_700_000_000_000));
    assert!(!forever.is_expired_at(1_700_000_000_000));
}

#[test]
fn validity_not_yet_effective_is_not_valid() {
    let window = Validity {
        valid_from_ms: Some(500),
        valid_until_ms: None,
    };
    assert!(!window.is_valid_at(499), "尚未生效");
    assert!(window.is_valid_at(500), "生效起点当刻有效");
    // ⚠️ **尚未生效 ≠ 已失效**：不算错（口径见 `is_expired_at`）。
    assert!(!window.is_expired_at(499));
    assert!(!window.is_expired_at(500));
}

#[test]
fn validity_expired_record_is_not_valid() {
    let window = Validity {
        valid_from_ms: Some(100),
        valid_until_ms: Some(200),
    };
    assert!(window.is_valid_at(150), "有效期内");
    assert!(!window.is_valid_at(200), "失效当刻即已失效");
    assert!(window.is_expired_at(200), "已失效 ⇒ 算错");
    assert!(window.is_expired_at(201));
    assert!(!window.is_expired_at(199), "还没到期 ⇒ 不算错");
}

#[test]
fn validity_boundary_is_half_open_interval() {
    // ⚠️ **半开区间** `[from, until)`：与 `canonical::domain` 同口径。
    let window = Validity {
        valid_from_ms: Some(100),
        valid_until_ms: Some(200),
    };
    assert!(!window.is_valid_at(99), "生效前");
    assert!(window.is_valid_at(100), "生效当刻");
    assert!(window.is_valid_at(199), "到期前一刻");
    assert!(!window.is_valid_at(200), "到期当刻 ⇒ 已失效");
}

// ============================================================
// L1 时间正确率：度量
// ============================================================

#[test]
fn temporal_rate_is_one_when_everything_returned_is_valid() {
    let mut validity = BTreeMap::new();
    validity.insert(
        "a".to_string(),
        Validity {
            valid_from_ms: Some(0),
            valid_until_ms: None,
        },
    );
    validity.insert(
        "b".to_string(),
        Validity {
            valid_from_ms: Some(0),
            valid_until_ms: Some(1_000),
        },
    );
    let verdict = measure_temporal(&["a".to_string(), "b".to_string()], &validity, 500);
    assert_eq!(verdict.checked, 2);
    assert_eq!(verdict.stale_returned, 0);
    assert_eq!(verdict.correct_rate, Some(1.0));
}

#[test]
fn temporal_counts_stale_returns_as_errors() {
    let mut validity = BTreeMap::new();
    validity.insert(
        "a".to_string(),
        Validity {
            valid_from_ms: Some(0),
            valid_until_ms: Some(50),
        },
    );
    validity.insert(
        "b".to_string(),
        Validity {
            valid_from_ms: Some(0),
            valid_until_ms: None,
        },
    );
    let verdict = measure_temporal(&["a".to_string(), "b".to_string()], &validity, 100);
    assert_eq!(verdict.checked, 2);
    assert_eq!(verdict.stale_returned, 1);
    let rate = verdict.correct_rate.expect("有分母");
    assert!((rate - 0.5).abs() < 1e-9, "一半错 ⇒ 0.5，实际 {rate}");
}

#[test]
fn temporal_excludes_records_without_known_validity_from_denominator() {
    // ⚠️ **查不到有效期 ⇒ 不计入分母**（无信息不算错）⇒ `checked` 只数查到的。
    let mut validity = BTreeMap::new();
    validity.insert(
        "known".to_string(),
        Validity {
            valid_from_ms: Some(0),
            valid_until_ms: None,
        },
    );
    let verdict = measure_temporal(&["known".to_string(), "unknown".to_string()], &validity, 10);
    assert_eq!(verdict.checked, 1, "只数查到的");
    assert_eq!(verdict.stale_returned, 0);
    assert_eq!(verdict.correct_rate, Some(1.0));
}

#[test]
fn temporal_returns_none_when_nothing_is_known() {
    // ⚠️ **不许**当 1.0 冒充完美。
    let verdict = measure_temporal(&["a".to_string()], &BTreeMap::new(), 10);
    assert_eq!(verdict.checked, 0);
    assert_eq!(verdict.stale_returned, 0);
    assert_eq!(verdict.correct_rate, None);
}

#[test]
fn temporal_with_empty_returns_is_none() {
    // ⚠️ **不许**当 1.0 冒充完美（空返回 = 没有可测的记录）。
    let verdict = measure_temporal(&[], &BTreeMap::new(), 10);
    assert_eq!(verdict.checked, 0);
    assert_eq!(verdict.stale_returned, 0);
    assert_eq!(verdict.correct_rate, None);
}

#[test]
fn temporal_not_yet_effective_is_not_counted_as_an_error() {
    // 「还没到有效期」**不是**「返回了失效的记忆」⇒ 不进错数。
    let mut validity = BTreeMap::new();
    validity.insert(
        "future".to_string(),
        Validity {
            valid_from_ms: Some(10_000),
            valid_until_ms: None,
        },
    );
    let verdict = measure_temporal(&["future".to_string()], &validity, 10);
    assert_eq!(verdict.checked, 1, "查到了 ⇒ 进分母");
    assert_eq!(verdict.stale_returned, 0, "尚未生效 ≠ 已失效");
    assert_eq!(verdict.correct_rate, Some(1.0));
}

#[test]
fn temporal_counts_a_repeated_record_once() {
    // 同一条记录被返回两次 ⇒ **一条记录**（否则同一处错误被重复计数）。
    let mut validity = BTreeMap::new();
    validity.insert(
        "a".to_string(),
        Validity {
            valid_from_ms: Some(0),
            valid_until_ms: Some(10),
        },
    );
    let verdict = measure_temporal(
        &["a".to_string(), "a".to_string(), "a".to_string()],
        &validity,
        100,
    );
    assert_eq!(verdict.checked, 1);
    assert_eq!(verdict.stale_returned, 1);
    assert_eq!(verdict.correct_rate, Some(0.0));
}

// ============================================================
// L1 矛盾检出率
// ============================================================

#[test]
fn contradiction_is_detected_when_both_appear_in_the_same_result() {
    let pairs = vec![("a".to_string(), "b".to_string())];
    let retrieved = vec![vec!["x".to_string(), "a".to_string(), "b".to_string()]];
    let verdict = measure_contradiction(&pairs, &retrieved);
    assert_eq!(verdict.total_pairs, 1);
    assert_eq!(verdict.detected_pairs, 1);
    assert_eq!(verdict.detection_rate, Some(1.0));
}

#[test]
fn contradiction_is_not_detected_when_only_one_appears() {
    let pairs = vec![("a".to_string(), "b".to_string())];
    let retrieved = vec![vec!["a".to_string()]];
    let verdict = measure_contradiction(&pairs, &retrieved);
    assert_eq!(verdict.detected_pairs, 0);
    assert_eq!(verdict.detection_rate, Some(0.0));
}

#[test]
fn contradiction_rate_is_none_without_pairs() {
    // ⚠️ **不许**当 0（那会说「一条都没检出」，而真相是「没矛盾可测」）。
    let verdict = measure_contradiction(&[], &[vec!["a".to_string()]]);
    assert_eq!(verdict.total_pairs, 0);
    assert_eq!(verdict.detected_pairs, 0);
    assert_eq!(verdict.detection_rate, None);
}

#[test]
fn contradiction_uses_the_aligned_case_result() {
    // 「检出」= **同一道题**的结果里同时含两个 id
    // ⇒ 第 2 对的 id 出现在**别的题**里 ⇒ 不算检出。
    let pairs = vec![
        ("a".to_string(), "b".to_string()),
        ("c".to_string(), "d".to_string()),
    ];
    let retrieved = vec![
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
    ];
    let verdict = measure_contradiction(&pairs, &retrieved);
    assert_eq!(verdict.detected_pairs, 1, "第 2 对只出了一条 ⇒ 未检出");
    assert_eq!(verdict.detection_rate, Some(0.5));
}

#[test]
fn contradiction_missing_result_counts_as_not_detected() {
    // 缺这道题的结果 ⇒ **不许**默认检出。
    let pairs = vec![("a".to_string(), "b".to_string())];
    let verdict = measure_contradiction(&pairs, &[]);
    assert_eq!(verdict.total_pairs, 1);
    assert_eq!(verdict.detected_pairs, 0);
    assert_eq!(verdict.detection_rate, Some(0.0));
}

#[test]
fn contradiction_pairs_from_cases_skips_single_record_cases() {
    let cases = vec![
        GoldenCase {
            id: "c1".to_string(),
            query: "问".to_string(),
            expect_ids: vec!["a".to_string(), "b".to_string()],
            expect_fragments: Vec::new(),
            tags: vec!["kind:conflict".to_string()],
        },
        GoldenCase {
            id: "c2".to_string(),
            query: "问".to_string(),
            expect_ids: vec!["only".to_string()],
            expect_fragments: Vec::new(),
            tags: Vec::new(),
        },
    ];
    let pairs = contradiction_pairs_from_cases(&cases);
    assert_eq!(pairs.len(), 1, "单记录题不是矛盾对");
    assert_eq!(pairs[0], ("a".to_string(), "b".to_string()));
}

// ============================================================
// L4 影子胜率
// ============================================================

#[test]
fn shadow_win_counts_higher_accuracy_as_a_win() {
    let verdict = measure_shadow_win(&[0.5], &[0.8], &[100], &[100]);
    assert_eq!(verdict.comparisons, 1);
    assert_eq!(verdict.challenger_wins, 1);
    assert_eq!(verdict.win_rate, Some(1.0));
}

#[test]
fn shadow_win_counts_fewer_tokens_as_a_win_when_accuracy_ties() {
    // §6.4「唯一交换许可」：准确度不变 + 更少 token ⇒ **纯赚** ⇒ 算赢。
    let verdict = measure_shadow_win(&[0.5], &[0.5], &[200], &[100]);
    assert_eq!(verdict.challenger_wins, 1);
    assert_eq!(verdict.win_rate, Some(1.0));
}

#[test]
fn shadow_win_ignores_a_complete_tie() {
    // 平局 ⇒ **既不算赢也不算输**。
    let verdict = measure_shadow_win(&[0.5], &[0.5], &[100], &[100]);
    assert_eq!(verdict.challenger_wins, 0);
    assert_eq!(verdict.win_rate, Some(0.0));
}

#[test]
fn shadow_win_does_not_count_lower_accuracy_as_a_win() {
    // 准确度更低 ⇒ 不许靠 token 省回来算赢（§6.4：准确度是硬指标）。
    let verdict = measure_shadow_win(&[0.8], &[0.5], &[100], &[10]);
    assert_eq!(verdict.challenger_wins, 0);
    assert_eq!(verdict.win_rate, Some(0.0));
}

#[test]
fn shadow_win_rate_is_none_without_comparisons() {
    let verdict = measure_shadow_win(&[], &[], &[], &[]);
    assert_eq!(verdict.comparisons, 0);
    assert_eq!(verdict.challenger_wins, 0);
    assert_eq!(verdict.win_rate, None);
}

#[test]
fn shadow_win_uses_the_shortest_input_array() {
    // 数组不齐 ⇒ 以**最短**的一组为准（不许多算对照）。
    let verdict = measure_shadow_win(&[0.5, 0.5], &[0.9], &[100, 100], &[100]);
    assert_eq!(verdict.comparisons, 1);
    assert_eq!(verdict.challenger_wins, 1);
}

// ============================================================
// L4 回退时间
// ============================================================

#[test]
fn rollback_faster_than_switch_is_required() {
    let verdict = measure_rollback_time(30, Some(100));
    assert_eq!(verdict.rollback_ms, 30);
    assert_eq!(verdict.switch_ms, Some(100));
    assert_eq!(verdict.faster_than_switch, Some(true));
    let slower = measure_rollback_time(150, Some(100));
    assert_eq!(slower.faster_than_switch, Some(false));
}

#[test]
fn rollback_without_switch_time_cannot_be_compared() {
    let verdict = measure_rollback_time(30, None);
    assert_eq!(verdict.switch_ms, None);
    assert_eq!(
        verdict.faster_than_switch, None,
        "没比过 ⇒ 无从比较（不许默认「更快」）"
    );
}

// ============================================================
// 报告
// ============================================================

#[test]
fn report_marks_missing_measurements_as_not_measured() {
    let report = render_quality_report(None, None, None, None, 10);
    assert!(
        report.contains("没测到"),
        "缺项必须写「没测到」：\n{report}"
    );
    assert!(report.contains("k = 10"), "报告必须带 k：\n{report}");
    assert!(report.contains("时间正确率"));
    assert!(report.contains("矛盾检出率"));
    assert!(report.contains("影子胜率"));
    assert!(report.contains("回退时间"));
}

#[test]
fn report_includes_every_measured_number() {
    let temporal = TemporalVerdict {
        checked: 3,
        stale_returned: 1,
        correct_rate: Some(2.0 / 3.0),
    };
    let contradiction = ContradictionVerdict {
        total_pairs: 4,
        detected_pairs: 2,
        detection_rate: Some(0.5),
    };
    let shadow = ShadowWinVerdict {
        comparisons: 6,
        challenger_wins: 3,
        win_rate: Some(0.5),
    };
    let timing = RollbackTiming {
        rollback_ms: 30,
        switch_ms: Some(100),
        faster_than_switch: Some(true),
    };
    let report = render_quality_report(
        Some(&temporal),
        Some(&contradiction),
        Some(&shadow),
        Some(&timing),
        10,
    );
    for needle in ["0.6667", "0.5000", "30 ms", "k = 10"] {
        assert!(report.contains(needle), "报告里应含 {needle}：\n{report}");
    }
}

#[test]
fn report_states_the_k_caveat_next_to_contradiction_rate() {
    // ⚠️ k 小 ⇒ 检出率天然低 ⇒ **不是**检索不行的证据 ⇒ 报告里必须写出来。
    let contradiction = ContradictionVerdict {
        total_pairs: 4,
        detected_pairs: 1,
        detection_rate: Some(0.25),
    };
    let report = render_quality_report(None, Some(&contradiction), None, None, 3);
    assert!(report.contains("k = 3"), "报告必须带 k：\n{report}");
    assert!(
        report.contains("限制了检出上限"),
        "矛盾检出率旁必须写 k 的限制：\n{report}"
    );
}

#[test]
fn report_renders_not_measured_when_rate_is_none() {
    // 分母为 0 ⇒ 「没测到」，**不许**渲染成 0 或 1。
    let temporal = TemporalVerdict {
        checked: 0,
        stale_returned: 0,
        correct_rate: None,
    };
    let report = render_quality_report(Some(&temporal), None, None, None, 10);
    assert!(
        report.contains("没测到"),
        "分母为 0 必须写「没测到」：\n{report}"
    );
    assert!(
        !report.contains("1.0000"),
        "不许把没测到渲染成满分：\n{report}"
    );
}

#[test]
fn extended_report_also_renders_l2_items() {
    let token = TokenEfficiency {
        injected_chars: 100,
        supported_chars: 40,
        efficiency: Some(0.4),
    };
    let citation = CitationCoverage {
        total_citations: 2,
        resolvable_citations: 1,
        coverage: Some(0.5),
    };
    let report =
        render_quality_report_extended(None, None, Some(&token), Some(&citation), None, None, 10);
    assert!(report.contains("0.4000"), "应含 Token 效率：\n{report}");
    assert!(
        report.contains("0.5000"),
        "应含 Citation coverage：\n{report}"
    );
    assert!(
        report.contains("字符是 token 的代理"),
        "Token 效率必须带口径提醒：\n{report}"
    );
}

// ============================================================
// L2 Token 效率（与本模块同属度量台账这条线）
// ============================================================

#[test]
fn token_efficiency_counts_supported_characters() {
    let verdict = measure_token_efficiency("abcdefghij", &["abcd".to_string()]);
    assert_eq!(verdict.injected_chars, 10);
    assert_eq!(verdict.supported_chars, 4);
    let rate = verdict.efficiency.expect("有分母");
    assert!((rate - 0.4).abs() < 1e-9, "实际 {rate}");
}

#[test]
fn token_efficiency_merges_overlapping_intervals_without_double_counting() {
    // 两个片段**重叠** ⇒ 并集长度不是两者之和。
    let verdict = measure_token_efficiency("abcdefghij", &["abcd".to_string(), "cdef".to_string()]);
    assert_eq!(verdict.supported_chars, 6, "abcd ∪ cdef = 6 字");
}

#[test]
fn token_efficiency_is_zero_when_nothing_is_supported() {
    let verdict = measure_token_efficiency("abcdefghij", &["zzzz".to_string()]);
    assert_eq!(verdict.supported_chars, 0);
    assert_eq!(verdict.efficiency, Some(0.0));
}

#[test]
fn token_efficiency_is_none_without_injected_text() {
    let verdict = measure_token_efficiency("", &["abcd".to_string()]);
    assert_eq!(verdict.injected_chars, 0);
    assert_eq!(verdict.efficiency, None);
}

#[test]
fn token_efficiency_handles_multibyte_text() {
    // ⚠️ **按字符**算（不是字节）⇒ 中文的效率与英文同尺。
    let verdict = measure_token_efficiency("宝玉黛玉", &["宝玉".to_string()]);
    assert_eq!(verdict.injected_chars, 4);
    assert_eq!(verdict.supported_chars, 2);
}

// ============================================================
// L2 Citation coverage（与本模块同属度量台账这条线）
// ============================================================

#[test]
fn citation_coverage_counts_resolvable_citations() {
    let verdict = measure_citation_coverage(
        &["a".to_string(), "ghost".to_string()],
        &["a".to_string(), "b".to_string()],
    );
    assert_eq!(verdict.total_citations, 2);
    assert_eq!(verdict.resolvable_citations, 1);
    assert_eq!(verdict.coverage, Some(0.5));
}

#[test]
fn citation_coverage_is_none_without_citations() {
    let verdict = measure_citation_coverage(&[], &["a".to_string()]);
    assert_eq!(verdict.coverage, None);
}

//! `forgetting_metrics` 的测试 —— 设计稿 §6.2 的四个「遗忘」指标。
//!
//! **核心纪律**：
//! * **测不到就是测不到** ⇒ 拿不到分母时返回 `None`（**不许**用 0.0 / 1.0 冒充）；
//! * **不许把越权销毁当「扣分项」** ⇒ 越权数**必须恒 0** ⇒ 非 0 即事故；
//! * **不许假设「还能检索到」** ⇒ 可达性必须由**真实检索**测出。

use apeireth_memory::forgetting_metrics::{
    judge_retrievable, measure_destruction, measure_downweight, measure_reachable, render_report,
    summarize, ArchivedItem, DestructionRecord, DownweightedItem,
};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};

// ============================================================
// 降权正确率 + 秩位位移
// ============================================================

fn item(id: &str, before: usize, after: usize) -> DownweightedItem {
    DownweightedItem {
        id: id.to_string(),
        rank_before: before,
        rank_after: after,
    }
}

#[test]
fn downweight_rate_counts_items_that_moved_back() {
    // 三条里两条真往后排 ⇒ 2/3。
    let items = vec![item("a", 0, 5), item("b", 1, 3), item("c", 2, 1)];
    let (rate, _median) = measure_downweight(&items).expect("有可算的");
    assert!((rate - 2.0 / 3.0).abs() < 1e-9, "实际 {rate}");
}

#[test]
fn downweight_rate_is_none_on_empty_input() {
    // ⚠️ **不许**返回 0.0（那会说「一条都没做对」）**也不许**返回 1.0（冒充完美）。
    assert!(measure_downweight(&[]).is_none());
}

#[test]
fn rank_shift_uses_median_not_mean() {
    // ⚠️ **中位数**（一次异常会把均值拖走）。
    // 位移 = [1, 2, 3, 100] ⇒ 均值 26.5、中位数 2.5 → 3。
    let items = vec![
        item("a", 0, 1),
        item("b", 0, 2),
        item("c", 0, 3),
        item("d", 0, 100),
    ];
    let (_rate, median) = measure_downweight(&items).expect("有可算的");
    assert_eq!(median, 3, "中位数 2.5 → 四舍五入 3（**不是**均值 26）");
}

#[test]
fn rank_shift_of_odd_length_is_the_middle_value() {
    let items = vec![item("a", 0, 5), item("b", 0, 1), item("c", 0, 9)];
    let (_rate, median) = measure_downweight(&items).expect("有可算的");
    assert_eq!(median, 5);
}

#[test]
fn items_that_moved_forward_are_not_counted_as_correct() {
    // 「该往后排的**真往后排**」⇒ 反而变好的**不算做对**。
    let items = vec![item("a", 5, 0)];
    let (rate, median) = measure_downweight(&items).expect("有可算的");
    assert_eq!(rate, 0.0);
    assert!(median < 0, "位移是负的（变好了）");
}

// ============================================================
// 保真可达率
// ============================================================

fn archived(id: &str, archived: bool, destroyed: bool, retrievable: bool) -> ArchivedItem {
    ArchivedItem {
        id: id.to_string(),
        archived,
        destroyed,
        retrievable_when_needed: retrievable,
    }
}

#[test]
fn reachable_rate_excludes_destroyed_items_from_the_denominator() {
    // ⚠️ **核心语义**：销毁的不要求可达 ⇒ 不进分母。
    // 否则等于要求「删掉的还能找到」⇒ 口径自相矛盾。
    let items = vec![
        archived("alive", true, false, true),
        archived("gone", true, true, false),
    ];
    let rate = measure_reachable(&items).expect("有可算的");
    assert_eq!(rate, 1.0, "只有 alive 进分母，且它可达");
}

#[test]
fn reachable_rate_drops_when_archived_item_cannot_be_found() {
    let items = vec![
        archived("a", true, false, true),
        archived("b", true, false, false),
    ];
    let rate = measure_reachable(&items).expect("有可算的");
    assert!((rate - 0.5).abs() < 1e-9, "实际 {rate}");
}

#[test]
fn reachable_rate_is_none_when_nothing_is_downweighted_or_archived() {
    assert!(measure_reachable(&[]).is_none());
}

#[test]
fn reachable_rate_is_none_when_everything_was_destroyed() {
    // 全部销毁 ⇒ **分母为空** ⇒ **没测到**（**不是** 0%）。
    let items = vec![archived("a", true, true, false)];
    assert!(measure_reachable(&items).is_none());
}

// ============================================================
// 销毁合规率 + 越权销毁数
// ============================================================

fn destruction(id: &str, authorized: bool, tombstone: bool, protected: bool) -> DestructionRecord {
    DestructionRecord {
        id: id.to_string(),
        authorized,
        has_tombstone: tombstone,
        protected,
    }
}

#[test]
fn compliant_destruction_needs_authorization_tombstone_and_unprotected() {
    let records = vec![destruction("a", true, true, false)];
    let (rate, unauthorized) = measure_destruction(&records);
    assert_eq!(rate, 1.0);
    assert_eq!(unauthorized, 0);
}

#[test]
fn destroying_a_protected_item_counts_as_unauthorized() {
    // ⚠️ **越权销毁** ⇒ 计数 +1 ⇒ **恒 0** 的红线被破。
    let records = vec![
        destruction("ok", true, true, false),
        destruction("protected", true, true, true),
    ];
    let (rate, unauthorized) = measure_destruction(&records);
    assert_eq!(unauthorized, 1, "受保护却被销毁 ⇒ 越权");
    assert!((rate - 0.5).abs() < 1e-9, "实际 {rate}");
}

#[test]
fn destroying_without_authorization_counts_as_unauthorized() {
    let records = vec![destruction("a", false, true, false)];
    let (_rate, unauthorized) = measure_destruction(&records);
    assert_eq!(unauthorized, 1);
}

#[test]
fn destroying_without_tombstone_is_not_compliant() {
    let records = vec![destruction("a", true, false, false)];
    let (rate, unauthorized) = measure_destruction(&records);
    assert_eq!(rate, 0.0, "没墓碑 ⇒ 不合规");
    assert_eq!(
        unauthorized, 0,
        "**没墓碑不是「越权」**（越权 = 动了不该动的东西；没墓碑 = 账不全）"
    );
}

#[test]
fn empty_destruction_log_yields_no_unauthorized_and_zero_rate() {
    let (rate, unauthorized) = measure_destruction(&[]);
    assert_eq!(rate, 0.0, "**0.0 的含义是「没测到」**，不是「0% 合规」");
    assert_eq!(unauthorized, 0);
}

// ============================================================
// 汇总 + 报告
// ============================================================

#[test]
fn summary_marks_unmeasurable_groups_as_not_measured() {
    let report = summarize(&[], &[], &[]);
    assert!(report.metrics.is_none(), "三组都空 ⇒ 整组「没测到」");
    assert!(!report.notes.is_empty(), "**必须说明为什么没测到**");
}

#[test]
fn summary_produces_metrics_when_data_is_present() {
    let report = summarize(
        &[item("a", 0, 5)],
        &[archived("a", true, false, true)],
        &[destruction("a", true, true, false)],
    );
    let metrics = report.metrics.expect("有数据 ⇒ 有指标");
    assert_eq!(metrics.unauthorized_destructions, 0);
    assert_eq!(metrics.reachable_rate, 1.0);
}

#[test]
fn report_contains_the_four_numbers_and_the_caliber_warning() {
    let report = summarize(
        &[item("a", 0, 5)],
        &[archived("a", true, false, true)],
        &[destruction("a", true, true, false)],
    );
    let text = render_report(&report);
    for needle in ["降权正确率", "保真可达率", "销毁合规率", "秩位位移"] {
        assert!(text.contains(needle), "报告里应含 {needle}：\n{text}");
    }
    // ⚠️ §6.2 的诚实要求：**必须**带口径提醒（不许与「删除式遗忘」的分数混用）。
    assert!(
        text.contains("删除式遗忘") || text.contains("不得"),
        "报告里必须有口径提醒：\n{text}"
    );
}

#[test]
fn report_flags_unauthorized_destructions() {
    let report = summarize(
        &[item("a", 0, 5)],
        &[archived("a", true, false, true)],
        &[destruction("protected", true, true, true)],
    );
    let text = render_report(&report);
    assert!(
        text.contains("越权") && text.contains("1"),
        "越权销毁数必须在报告里出现：\n{text}"
    );
}

// ============================================================
// 可达性判定（必须用真实检索）
// ============================================================

fn case(id: &str, expect: &[&str]) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: format!("查 {id}"),
        expect_ids: expect.iter().map(|value| value.to_string()).collect(),
        expect_fragments: Vec::new(),
        tags: Vec::new(),
    }
}

fn golden_with(cases: Vec<GoldenCase>) -> GoldenSet {
    GoldenSet::new("test".to_string(), cases)
}

#[test]
fn retrievable_requires_real_retrieval_to_find_it() {
    let golden = golden_with(vec![case("c1", &["target"])]);
    // 真实检索能找到 ⇒ 可达。
    let found = judge_retrievable("target", &golden, |_case| vec!["target".to_string()]);
    assert!(found);
    // 真实检索**找不到** ⇒ 不可达（**不许**假设）。
    let missing = judge_retrievable("target", &golden, |_case| vec!["other".to_string()]);
    assert!(!missing);
}

#[test]
fn retrievable_without_any_question_about_it_is_false() {
    // **没有判定依据** ⇒ 保守判不可达（**不许**凭空说「可达」）。
    let golden = golden_with(vec![case("c1", &["something-else"])]);
    let verdict = judge_retrievable("target", &golden, |_case| vec!["target".to_string()]);
    assert!(!verdict);
}

#[test]
fn retrievable_requires_every_relevant_question_to_find_it() {
    // ⚠️ **保守方向**：两道相关题里只要有一道找不到 ⇒ 就算不可达。
    let golden = golden_with(vec![case("c1", &["target"]), case("c2", &["target"])]);
    let verdict = judge_retrievable("target", &golden, |case| {
        if case.id == "c1" {
            vec!["target".to_string()]
        } else {
            vec!["other".to_string()]
        }
    });
    assert!(!verdict);
    // 两道都检索得到 ⇒ 可达。
    let all = judge_retrievable("target", &golden, |_case| vec!["target".to_string()]);
    assert!(all);
}

// ============================================================
// 以下为 W-27 补的用例（口径同上；补的是**边界**与**报告层**）
// ============================================================

#[test]
fn rank_shift_of_even_length_averages_the_two_middles() {
    // 位移 = [0, 2] ⇒ 中位数 = (0 + 2) / 2 = 1（**不是**均值以外的任何数）。
    let items = vec![item("a", 0, 0), item("b", 0, 2)];
    let (_rate, median) = measure_downweight(&items).expect("有可算的");
    assert_eq!(median, 1, "偶数条 ⇒ 两个中位数的平均");
}

#[test]
fn items_that_did_not_move_count_as_violations() {
    // 名次**没动** ⇒ 降权**没生效** ⇒ 算没做对（但仍留在分母里：不许靠剔除拉高分）。
    let items = vec![item("a", 0, 0), item("b", 1, 2)];
    let (rate, _median) = measure_downweight(&items).expect("有可算的");
    assert!(
        (rate - 0.5).abs() < 1e-9,
        "没动的那条不算做对（分母里留着）"
    );
}

#[test]
fn reachable_rate_is_one_when_everything_is_retrievable() {
    let items = vec![
        archived("a", true, false, true),
        archived("b", false, false, true),
    ];
    let rate = measure_reachable(&items).expect("有可算的");
    assert!((rate - 1.0).abs() < 1e-9, "实际 {rate}");
}

#[test]
fn unauthorized_counts_each_record_once_even_with_multiple_violations() {
    // 一条记录同时「受保护」+「无授权」⇒ **只计 1 次**（数的是「多少次事故」）。
    let records = vec![destruction("a", false, true, true)];
    let (_rate, unauthorized) = measure_destruction(&records);
    assert_eq!(unauthorized, 1, "一条记录最多计一次");
}

#[test]
fn summary_names_the_unauthorized_records() {
    // 越权必须**点名**是哪几条（不许只报一个数）。
    let report = summarize(
        &[item("a", 0, 5)],
        &[archived("a", true, false, true)],
        &[
            destruction("ok", true, true, false),
            destruction("bad", true, false, false),
        ],
    );
    let metrics = report.metrics.expect("有数据 ⇒ 有指标");
    assert_eq!(
        metrics.unauthorized_destructions, 0,
        "没墓碑 = 账不全 ⇒ 不算越权"
    );
    let joined = report.notes.join("\n");
    assert!(joined.contains("销毁合规率"), "notes 要写合规率：{joined}");
}

#[test]
fn summary_names_the_records_that_broke_the_red_line() {
    let report = summarize(
        &[item("a", 0, 5)],
        &[archived("a", true, false, true)],
        &[
            destruction("ok", true, true, false),
            destruction("protected", true, true, true),
        ],
    );
    let metrics = report.metrics.expect("有数据 ⇒ 有指标");
    assert_eq!(
        metrics.unauthorized_destructions, 1,
        "受保护却被销毁 ⇒ 越权 +1"
    );
    let joined = report.notes.join("\n");
    assert!(
        joined.contains("protected"),
        "notes 要点名越权的那条：{joined}"
    );
    assert!(joined.contains("事故"), "越权必须写成事故：{joined}");
}

#[test]
fn summary_without_destruction_records_is_not_measured() {
    // 销毁子项没数据 ⇒ 整组「没测到」（不许拿 0.0 冒充合规率）。
    let report = summarize(&[item("a", 0, 5)], &[archived("a", true, false, true)], &[]);
    assert!(
        report.metrics.is_none(),
        "销毁子项没数据 ⇒ 整组 None（不许拿 0.0 冒充合规率）"
    );
}

#[test]
fn report_contains_the_unauthorized_count_label() {
    let report = summarize(
        &[item("a", 0, 5)],
        &[archived("a", true, false, true)],
        &[destruction("a", true, true, false)],
    );
    let text = render_report(&report);
    assert!(
        text.contains("越权销毁数"),
        "报告里必须有越权销毁数：\n{text}"
    );
    assert!(
        text.contains("降权 + 仍可检索") && text.contains("selective forgetting"),
        "报告里必须有口径提醒：\n{text}"
    );
}

#[test]
fn report_says_not_measured_when_metrics_are_missing() {
    let report = summarize(&[], &[], &[]);
    let text = render_report(&report);
    assert!(text.contains("没测到"), "{text}");
    assert!(
        !text.contains("降权正确率：0.0000"),
        "没测到不许打成 0.0000：{text}"
    );
    assert!(
        text.contains("selective forgetting"),
        "口径提醒即使没测到也要带：{text}"
    );
}

#[test]
fn metrics_struct_carries_the_four_numbers() {
    // 编译期口径：字段不许改名/漏掉（五个字段 = 四个指标 + 越权数）。
    let metrics = apeireth_memory::forgetting_metrics::ForgettingMetrics {
        downweight_correct_rate: 1.0,
        reachable_rate: 0.5,
        destruction_compliance_rate: 0.25,
        unauthorized_destructions: 0,
        median_rank_shift: -1,
    };
    assert!((metrics.downweight_correct_rate - 1.0).abs() < 1e-12);
    assert!((metrics.reachable_rate - 0.5).abs() < 1e-12);
    assert!((metrics.destruction_compliance_rate - 0.25).abs() < 1e-12);
    assert_eq!(metrics.unauthorized_destructions, 0);
    assert_eq!(metrics.median_rank_shift, -1);
}

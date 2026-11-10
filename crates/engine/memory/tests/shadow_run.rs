//! `shadow_run` 的测试：**影子双跑**逐题对应 + **换算法闸**的判定口径。
//!
//! 重点不在「跑得动」，而在**判定口径不许被悄悄改掉**：
//! * 影子侧**动不了**生产（现役侧结果逐字不变）；
//! * 判定**比中位数**不比单次分（§6.4「比稳定分不比单次分」）；
//! * **没证据**（空题集 / 跑 0 次 / 卷子没标准答案）⇒ **不许换**（不是默认通过）。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use apeireth_memory::golden_set::GoldenCase;
use apeireth_memory::shadow_run::{
    judge_swap, run_shadow, Retriever, RunOutcome, ShadowReport, SwapDecision, DEFAULT_RUNS,
};

/// 一道题：标准答案 = `["hit"]`（假检索器命中时返回 `hit`，否则返回 `miss`）。
fn case(id: &str) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: format!("关于 {id} 的问题"),
        expect_ids: vec!["hit".to_string()],
        expect_fragments: Vec::new(),
        tags: vec!["internal".to_string()],
    }
}

/// 一道**没有标准答案**的题（整题无法评 ⇒ 不进准确度分母）。
fn case_without_answer(id: &str) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: format!("关于 {id} 的问题"),
        expect_ids: Vec::new(),
        expect_fragments: Vec::new(),
        tags: vec!["internal".to_string()],
    }
}

/// 假检索器：命中 `hit_cases` 里的题 ⇒ 返回 `hit`，否则返回 `miss`。
struct Fake {
    id: String,
    hit_cases: BTreeSet<String>,
    tokens: usize,
}

impl Fake {
    /// 命中哪些题（字面量写法）。
    fn new(id: &str, hit_cases: &[&str], tokens: usize) -> Self {
        Fake::with_ids(
            id,
            hit_cases.iter().map(|value| value.to_string()).collect(),
            tokens,
        )
    }

    /// 命中哪些题（程序生成写法）。
    fn with_ids(id: &str, hit_cases: Vec<String>, tokens: usize) -> Self {
        Fake {
            id: id.to_string(),
            hit_cases: hit_cases.into_iter().collect(),
            tokens,
        }
    }
}

impl Retriever for Fake {
    fn id(&self) -> &str {
        &self.id
    }

    fn retrieve(&self, case: &GoldenCase) -> RunOutcome {
        let ids = if self.hit_cases.contains(&case.id) {
            vec!["hit".to_string()]
        } else {
            vec!["miss".to_string()]
        };
        RunOutcome {
            ids,
            injected_tokens: self.tokens,
        }
    }
}

/// 噪声检索器：同一题的**第 n 次调用** ⇒ 按 `pattern[n % pattern.len()]` 决定命中与否。
///
/// 用来模拟「单次运行波动大」的真实检索器 ⇒ 检验判定到底用的是
/// **中位数**（稳定分）还是**单次分**（§6.4）。
struct Noisy {
    id: String,
    pattern: Vec<bool>,
    calls: RefCell<BTreeMap<String, usize>>,
    tokens: usize,
}

impl Noisy {
    fn new(id: &str, pattern: Vec<bool>, tokens: usize) -> Self {
        Noisy {
            id: id.to_string(),
            pattern,
            calls: RefCell::new(BTreeMap::new()),
            tokens,
        }
    }
}

impl Retriever for Noisy {
    fn id(&self) -> &str {
        &self.id
    }

    fn retrieve(&self, case: &GoldenCase) -> RunOutcome {
        let index = {
            let mut calls = self.calls.borrow_mut();
            let counter = calls.entry(case.id.clone()).or_insert(0);
            let current = *counter;
            *counter += 1;
            current
        };
        let hit = self.pattern[index % self.pattern.len()];
        let ids = if hit {
            vec!["hit".to_string()]
        } else {
            vec!["miss".to_string()]
        };
        RunOutcome {
            ids,
            injected_tokens: self.tokens,
        }
    }
}

/// 造 n 道题（题 id = `c0` …）。
fn cases(count: usize) -> Vec<GoldenCase> {
    (0..count).map(|index| case(&format!("c{index}"))).collect()
}

/// 前 n 道题的题 id（用来造「命中前 n 题」的检索器）。
fn first_ids(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("c{index}")).collect()
}

#[test]
fn run_shadow_pairs_every_case_one_to_one() {
    let cases = cases(3);
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let challenger = Fake::new("hybrid", &["c0", "c2"], 80);
    let report = run_shadow(&cases, &champion, &challenger);

    assert_eq!(report.pairs.len(), cases.len(), "逐题必须成对");
    assert_eq!(report.champion_id, "bm25");
    assert_eq!(report.challenger_id, "hybrid");
    for (index, pair) in report.pairs.iter().enumerate() {
        assert_eq!(pair.case_id, cases[index].id, "第 {index} 对必须对得上题");
    }
    // 两侧内容与「各自单独跑」一致 ⇒ 影子侧没有串味。
    // 现役命中 c0/c1；影子命中 c0/c2 ⇒ 逐题一真一假。
    assert_eq!(report.pairs[0].champion.ids, vec!["hit".to_string()]);
    assert_eq!(report.pairs[0].challenger.ids, vec!["hit".to_string()]);
    assert_eq!(report.pairs[1].champion.ids, vec!["hit".to_string()]);
    assert_eq!(report.pairs[1].challenger.ids, vec!["miss".to_string()]);
    assert_eq!(report.pairs[2].champion.ids, vec!["miss".to_string()]);
    assert_eq!(report.pairs[2].challenger.ids, vec!["hit".to_string()]);
    assert_eq!(report.pairs[0].challenger.injected_tokens, 80);
    assert_eq!(report.pairs[0].champion.injected_tokens, 100);
}

#[test]
fn run_shadow_on_an_empty_case_set_is_empty_and_does_not_panic() {
    let champion = Fake::new("bm25", &[], 100);
    let challenger = Fake::new("hybrid", &[], 100);
    let report = run_shadow(&[], &champion, &challenger);
    assert!(report.pairs.is_empty());
    assert_eq!(report.champion_id, "bm25");
    assert_eq!(report.challenger_id, "hybrid");
}

#[test]
fn the_shadow_side_cannot_influence_the_champion_side() {
    // 影子侧故意返回一堆垃圾 + 高 token ⇒ 现役侧必须**逐字不变**。
    let cases = cases(2);
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let junk = Fake::new("junk", &[], 999);
    let report = run_shadow(&cases, &champion, &junk);

    let standalone = champion.retrieve(&cases[0]);
    assert_eq!(
        report.pairs[0].champion, standalone,
        "现役侧不许被影子侧影响"
    );
    assert_eq!(report.pairs[0].champion.injected_tokens, 100);
    assert_eq!(report.pairs[1].champion.injected_tokens, 100);
}

#[test]
fn the_shadow_report_carries_only_ids_and_metrics_no_user_facing_answer() {
    // 影子纪律的**结构性**保证：报告里**没有**「最终答案」这类给用户看的东西。
    // ① 编译期：字段必须**恰好**是这三个（多一个字段就编不过）。
    let report = ShadowReport {
        champion_id: "bm25".to_string(),
        challenger_id: "hybrid".to_string(),
        pairs: Vec::new(),
    };
    let ShadowReport {
        champion_id,
        challenger_id,
        pairs,
    } = report;
    assert_eq!(champion_id, "bm25");
    assert_eq!(challenger_id, "hybrid");
    assert!(pairs.is_empty());

    // ② 运行期：调试输出里**不许**出现「答案/正文/回复」这类字段名。
    let cases = cases(1);
    let champion = Fake::new("bm25", &["c0"], 100);
    let challenger = Fake::new("hybrid", &["c0"], 100);
    let debug = format!("{:?}", run_shadow(&cases, &champion, &challenger)).to_lowercase();
    for forbidden in [
        "answer", "final", "response", "reply", "content", "text", "fragment", "prompt", "output",
    ] {
        assert!(
            !debug.contains(forbidden),
            "影子报告里不该有「{forbidden}」这类字段：{debug}"
        );
    }
}

#[test]
fn identical_algorithms_are_approved_by_measurement_not_by_default() {
    let cases = cases(2);
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let same = Fake::new("bm25-copy", &["c0", "c1"], 100);
    let verdict = judge_swap(&champion, &same, &cases, DEFAULT_RUNS);

    assert_eq!(verdict.decision, SwapDecision::Approved);
    // 「实测通过」⇒ 分数与 token 都**量得到**（不是默认值、不是 None）。
    assert_eq!(verdict.champion_score, Some(1.0));
    assert_eq!(verdict.challenger_score, Some(1.0));
    assert_eq!(verdict.champion_tokens, Some(200));
    assert_eq!(verdict.challenger_tokens, Some(200));
    assert!(!verdict.reason.is_empty());
}

#[test]
fn a_candidate_with_higher_accuracy_is_approved_with_gain() {
    let cases = cases(3);
    let champion = Fake::new("bm25", &["c0"], 100);
    let challenger = Fake::new("hybrid", &["c0", "c1", "c2"], 100);
    let verdict = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);

    assert_eq!(verdict.decision, SwapDecision::ApprovedWithGain);
    // 理由里必须**说明提升了多少** ⇒ 带数字。
    assert!(
        verdict.reason.contains("0.666667"),
        "理由要写明提升量：{}",
        verdict.reason
    );
    assert!(verdict.reason.contains("提升"), "{}", verdict.reason);
}

#[test]
fn a_candidate_that_loses_more_than_one_percent_is_rejected_with_numbers() {
    let cases = cases(100);
    let champion = Fake::with_ids("bm25", first_ids(100), 100);
    let challenger = Fake::with_ids("bad", first_ids(50), 100);
    let verdict = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);

    assert_eq!(verdict.decision, SwapDecision::Rejected);
    // 理由里必须有数字（现役 1.000000 / 候选 0.500000）⇒ 不许写「变差了」这种空话。
    assert!(
        verdict.reason.contains("1.000000") && verdict.reason.contains("0.500000"),
        "理由要带数字：{}",
        verdict.reason
    );
    assert!(verdict.reason.contains("0.010000"), "{}", verdict.reason);
}

#[test]
fn a_micro_drop_within_one_percent_is_not_rejected() {
    // 200 题：现役全中（1.0），候选只丢 1 题（0.995）⇒ 掉 0.5% < 1% ⇒ **不判拒**。
    let cases = cases(200);
    let champion = Fake::with_ids("bm25", first_ids(200), 100);
    let challenger = Fake::with_ids("slightly-worse", first_ids(199), 100);
    let verdict = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);

    assert_ne!(
        verdict.decision,
        SwapDecision::Rejected,
        "≤1% 的微跌不该判拒：{}",
        verdict.reason
    );
    assert_eq!(verdict.decision, SwapDecision::Approved);
}

#[test]
fn exactly_one_percent_drop_is_tolerated() {
    // 100 题丢 1 题 ⇒ 恰好掉 1% ⇒ 边界**放行**（与 `check_no_regression` 同口径）。
    let cases = cases(100);
    let champion = Fake::with_ids("bm25", first_ids(100), 100);
    let challenger = Fake::with_ids("edge", first_ids(99), 100);
    let verdict = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);

    assert_eq!(
        verdict.decision,
        SwapDecision::Approved,
        "恰好 1% 应放行：{}",
        verdict.reason
    );
}

#[test]
fn a_candidate_that_costs_more_tokens_is_rejected() {
    // 准确度**一模一样**，只是更贵 ⇒ **不许换**（不许更贵）。
    let cases = cases(2);
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let pricier = Fake::new("hybrid", &["c0", "c1"], 120);
    let verdict = judge_swap(&champion, &pricier, &cases, DEFAULT_RUNS);

    assert_eq!(verdict.decision, SwapDecision::Rejected);
    // 中位 token 是**全题之和**（2 题 × 每题 100 / 120）⇒ 200 vs 240。
    assert_eq!(verdict.champion_tokens, Some(200));
    assert_eq!(verdict.challenger_tokens, Some(240));
    assert!(
        verdict.reason.contains("240") && verdict.reason.contains("200"),
        "理由要写清 token 差在哪：{}",
        verdict.reason
    );
}

#[test]
fn a_candidate_that_costs_less_with_the_same_accuracy_is_a_pure_gain() {
    let cases = cases(2);
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let cheaper = Fake::new("hybrid", &["c0", "c1"], 60);
    let verdict = judge_swap(&champion, &cheaper, &cases, DEFAULT_RUNS);

    assert_eq!(verdict.decision, SwapDecision::Approved);
    assert_eq!(verdict.champion_tokens, Some(200));
    assert_eq!(verdict.challenger_tokens, Some(120));
}

#[test]
fn an_empty_case_set_is_rejected_not_default_approved() {
    // 「没证据」≠「没变差」⇒ 空题集必须**明确判不许换**。
    let champion = Fake::new("bm25", &[], 100);
    let challenger = Fake::new("hybrid", &[], 100);
    let verdict = judge_swap(&champion, &challenger, &[], DEFAULT_RUNS);

    assert_ne!(
        verdict.decision,
        SwapDecision::Approved,
        "空题集不许默认通过"
    );
    assert_eq!(verdict.decision, SwapDecision::Rejected);
    assert!(verdict.champion_score.is_none());
    assert!(verdict.challenger_score.is_none());
    assert!(!verdict.reason.is_empty());
}

#[test]
fn zero_runs_is_rejected_not_default_approved() {
    let cases = cases(2);
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let challenger = Fake::new("hybrid", &["c0", "c1"], 100);
    let verdict = judge_swap(&champion, &challenger, &cases, 0);

    assert_eq!(verdict.decision, SwapDecision::Rejected);
    assert!(verdict.champion_score.is_none());
}

#[test]
fn a_golden_set_without_standard_answers_cannot_be_measured() {
    // 卷子里**没有带标准答案的题** ⇒ 准确度不可度量 ⇒ 不许换。
    let cases = vec![case_without_answer("c0"), case_without_answer("c1")];
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let challenger = Fake::new("hybrid", &["c0", "c1"], 100);
    let verdict = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);

    assert_eq!(verdict.decision, SwapDecision::Rejected);
    assert!(verdict.champion_score.is_none());
    assert!(!verdict.reason.is_empty());
}

#[test]
fn the_verdict_uses_the_median_not_the_first_run() {
    // 影子侧：第 1 次跑满分、第 2/3 次跑 0 分 ⇒ **单次分**是满分，**中位数**是 0。
    let cases = cases(1);
    let champion = Fake::new("bm25", &["c0"], 100);
    let challenger = Noisy::new("noisy", vec![true, false, false], 100);
    let verdict = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);

    // 单看第一次跑 ⇒ 影子侧是满分（会被误判通过）。
    assert_eq!(
        challenger.retrieve(&cases[0]).ids,
        vec!["hit".to_string()],
        "影子侧第一次跑确实是满分（所以单次分不可信）"
    );
    // 但判定必须按**中位数** ⇒ 影子侧稳定分是 0 ⇒ **不许换**。
    assert_eq!(
        verdict.decision,
        SwapDecision::Rejected,
        "必须比中位数（稳定分），不许比单次分：{}",
        verdict.reason
    );
    assert_eq!(verdict.champion_score, Some(1.0));
    assert_eq!(verdict.challenger_score, Some(0.0));
}

#[test]
fn the_verdict_uses_the_median_not_the_mean() {
    // 影子侧：第 3 次跑挂了 ⇒ 均值会被拖到 0.667（⇒ 误判拒），但**中位数**仍是 1.0。
    let cases = cases(1);
    let champion = Fake::new("bm25", &["c0"], 100);
    let challenger = Noisy::new("noisy", vec![true, true, false], 100);
    let verdict = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);

    assert_eq!(
        verdict.decision,
        SwapDecision::Approved,
        "一次坏运不该拖垮判定：{}",
        verdict.reason
    );
    assert_eq!(
        verdict.challenger_score,
        Some(1.0),
        "中位数 = 1.0（均值会是 0.666667）"
    );
}

#[test]
fn judge_swap_is_deterministic() {
    let cases = cases(3);
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let challenger = Fake::new("hybrid", &["c0", "c1", "c2"], 90);

    let first = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);
    let second = judge_swap(&champion, &challenger, &cases, DEFAULT_RUNS);
    assert_eq!(first, second, "同一输入两次 ⇒ 逐字相等");
}

#[test]
fn run_shadow_is_deterministic() {
    let cases = cases(3);
    let champion = Fake::new("bm25", &["c0", "c1"], 100);
    let challenger = Fake::new("hybrid", &["c0"], 90);

    let first = run_shadow(&cases, &champion, &challenger);
    let second = run_shadow(&cases, &champion, &challenger);
    assert_eq!(first, second, "同一输入两次 ⇒ 逐字相等");
}

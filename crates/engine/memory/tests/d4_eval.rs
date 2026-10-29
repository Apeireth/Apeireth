//! D4 离线评估器（`apeireth_memory::d4_eval`）集成测试。
//!
//! 钉住几件事（每条都是「不许悄悄变」的判据）：
//! 1. **零增量 ⇒ 不值得开**：基线与开启后完全相同 ⇒ `delta_primary == 0`
//!    ⇒ 过闸但**不值得开**（没有收益 ⇒ 白付成本）。
//! 2. **有提升 ⇒ 值得开**：recall 涨了 ⇒ `delta_primary > 0` 且 `worth_enabling`。
//! 3. **掉分 ⇒ 不准开**：recall 掉了 ⇒ **过不了「不许变差」闸**
//!    （**即使其它指标涨了**也不许开）。
//! 4. **如实**：[`rationale`](apeireth_memory::d4_eval::AlgoVerdict::rationale)
//!    必须写出真实数值（涨/跌多少）+ 成本提示，不许含糊。
//! 5. **确定性**：同一输入两次评估**逐字相等**；批量评估按 `id` 排序；
//!    空输入不炸。

use apeireth_memory::d4_eval::{
    evaluate, evaluate_all, evaluate_with_k, summarize, AlgoInput, AlgoProfile, AnnotationKind,
    EvalInput, PRIMARY_METRIC,
};
use apeireth_memory::golden_set::{EvalBatch, GoldenCase, GoldenSet, Metrics, Retrieved};

/// 构造一条考卷题目（`expect_fragments` 非空 ⇒ 该题参与 groundedness 统计）。
fn case(id: &str, expect_ids: &[&str], expect_fragments: &[&str]) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: format!("问题 {id}"),
        expect_ids: expect_ids.iter().map(|s| s.to_string()).collect(),
        expect_fragments: expect_fragments.iter().map(|s| s.to_string()).collect(),
        tags: Vec::new(),
    }
}

/// 构造一次检索返回（候选 id + 注入片段）。
fn hit(ids: &[&str], fragments: &[&str]) -> Retrieved {
    Retrieved {
        ids: ids.iter().map(|s| s.to_string()).collect(),
        fragments: fragments.iter().map(|s| s.to_string()).collect(),
    }
}

/// 构造一批（`cases` 与 `results` 一一对应）。
fn batch(pairs: Vec<(GoldenCase, Retrieved)>) -> EvalBatch {
    let mut cases = Vec::new();
    let mut results = Vec::new();
    for (c, r) in pairs {
        cases.push(c);
        results.push(r);
    }
    EvalBatch { cases, results }
}

/// 构造考卷（版本号固定，便于断言 rationale 里的卷子说明）。
fn golden(cases: &[GoldenCase]) -> GoldenSet {
    GoldenSet::new("v-d4-eval-test", cases.to_vec())
}

/// 默认画像（带成本提示）。
fn profile(id: &str) -> AlgoProfile {
    AlgoProfile {
        id: id.to_string(),
        title: format!("算法 {id}"),
        consumes: AlgoInput::Text,
        produces: AnnotationKind::Pattern,
        cost_hint: Some("每轮额外 1 次全量重算".to_string()),
    }
}

// ============================================
// 三个可复用的实验场景
// ============================================

/// 场景 A：**完全没变**（基线 = 开启后）⇒ 零增量。
fn scenario_unchanged() -> EvalInput {
    let q1 = case("q1", &["a"], &[]);
    let same = batch(vec![(q1.clone(), hit(&["a"], &[]))]);
    EvalInput {
        profile: profile("unchanged_algo"),
        golden: golden(&[q1]),
        baseline: same.clone(),
        with_algo: same,
    }
}

/// 场景 B：**recall 提升**（q1 从漏召回到命中）⇒ 有增量。
fn scenario_improved() -> EvalInput {
    let q1 = case("q1", &["a"], &[]);
    let q2 = case("q2", &["b"], &[]);
    EvalInput {
        profile: profile("improved_algo"),
        golden: golden(&[q1.clone(), q2.clone()]),
        baseline: batch(vec![
            (q1.clone(), hit(&["x"], &[])),
            (q2.clone(), hit(&["b"], &[])),
        ]),
        with_algo: batch(vec![(q1, hit(&["a"], &[])), (q2, hit(&["b"], &[]))]),
    }
}

/// 场景 C：**recall 掉了**，但 groundedness / evidence_precision **涨了**。
///
/// ⚠️ 为什么 nDCG 在这个场景里必然也掉：nDCG@k 的分子是「命中项按位置的折损和」，
/// 少命中一个期望 id ⇒ 该题 DCG 严格变小 ⇒ nDCG 必然不升（这是指标定义的数学性质，
/// 不是实现缺陷）。所以「其它指标涨了」用 **groundedness / evidence_precision**
/// （它们只看注入片段，与排序无关）来演示。
fn scenario_regressed() -> EvalInput {
    let q1 = case("q1", &["a"], &["frag-a"]);
    let q2 = case("q2", &["b", "c"], &["frag-b"]);
    EvalInput {
        profile: profile("regressed_algo"),
        golden: golden(&[q1.clone(), q2.clone()]),
        // 基线：q1 命中且注入垃圾片段；q2 只命中 1/2。
        baseline: batch(vec![
            (q1.clone(), hit(&["a"], &["junk-1"])),
            (q2.clone(), hit(&["b"], &["junk-2"])),
        ]),
        // 开启后：q1 排序变差（漏召）但注入片段命中标准答案；q2 全命中。
        with_algo: batch(vec![
            (q1, hit(&["z"], &["frag-a"])),
            (q2, hit(&["b", "c"], &["frag-b"])),
        ]),
    }
}

// ============================================
// 1. 零增量 ⇒ 不值得开
// ============================================

#[test]
fn unchanged_run_has_zero_delta_and_is_not_worth_enabling() {
    let verdict = evaluate(scenario_unchanged());

    assert_eq!(verdict.delta_primary, 0.0, "完全没变 ⇒ 增量必须是 0");
    assert!(verdict.passes_regression_gate, "没变 ⇒ 绝不该被判成掉分");
    assert!(
        !verdict.worth_enabling,
        "零增量 ⇒ 没有收益 ⇒ 白付成本 ⇒ 不值得开"
    );
    assert_eq!(verdict.algo_id, "unchanged_algo");
    assert_eq!(verdict.primary_metric, PRIMARY_METRIC);
}

#[test]
fn zero_delta_rationale_says_no_benefit() {
    let verdict = evaluate(scenario_unchanged());

    assert!(
        verdict.rationale.contains("0.000000"),
        "rationale 必须写出零增量这个数：{}",
        verdict.rationale
    );
    assert!(
        verdict.rationale.contains("无变化"),
        "rationale 必须说清是「无变化」：{}",
        verdict.rationale
    );
    assert!(
        verdict.rationale.contains("不值得开"),
        "rationale 必须说清不值得开：{}",
        verdict.rationale
    );
}

// ============================================
// 2. 有提升 ⇒ 值得开
// ============================================

#[test]
fn recall_improvement_is_worth_enabling() {
    let verdict = evaluate(scenario_improved());

    assert!(
        verdict.delta_primary > 0.0,
        "recall 从 0.5 涨到 1.0 ⇒ 增量必须为正，实际 {}",
        verdict.delta_primary
    );
    assert!(verdict.passes_regression_gate, "只涨不跌 ⇒ 必须过闸");
    assert!(
        verdict.worth_enabling,
        "不倒退且有提升 ⇒ 值得开：{}",
        verdict.rationale
    );
}

#[test]
fn verdict_carries_metrics_unchanged() {
    let verdict = evaluate(scenario_improved());

    // 基线 recall = (0 + 1) / 2 = 0.5；开启后 = (1 + 1) / 2 = 1.0。
    assert!((verdict.baseline.recall_at_k - 0.5).abs() < 1e-12);
    assert!((verdict.with_algo.recall_at_k - 1.0).abs() < 1e-12);
    assert!((verdict.delta_primary - 0.5).abs() < 1e-12);
    assert_eq!(verdict.baseline.total_cases, 2);
    assert_eq!(verdict.with_algo.total_cases, 2);
}

// ============================================
// 3. 掉分 ⇒ 不准开（其它指标涨了也不许开）
// ============================================

#[test]
fn recall_regression_blocks_enabling_even_when_other_metrics_rise() {
    let verdict = evaluate(scenario_regressed());

    // 其它指标确实涨了（groundedness / evidence_precision：0.0 → 1.0）。
    assert!(
        verdict.with_algo.groundedness > verdict.baseline.groundedness,
        "本场景的注入片段质量是涨的"
    );
    assert!(
        verdict.with_algo.evidence_precision > verdict.baseline.evidence_precision,
        "本场景的证据精准率是涨的"
    );
    // 但 recall 掉了 ⇒ 不许开。
    assert!(
        verdict.delta_primary < 0.0,
        "recall 从 0.75 掉到 0.5 ⇒ 增量必须为负，实际 {}",
        verdict.delta_primary
    );
    assert!(
        !verdict.passes_regression_gate,
        "掉分就是不准开（哪怕其它指标涨了）"
    );
    assert!(
        !verdict.worth_enabling,
        "掉分 ⇒ 绝不值得开：{}",
        verdict.rationale
    );
}

#[test]
fn regression_rationale_reports_the_drop_and_refuses_to_open() {
    let verdict = evaluate(scenario_regressed());

    assert!(
        verdict.rationale.contains("-0.250000"),
        "rationale 必须如实写出退步的数值：{}",
        verdict.rationale
    );
    assert!(
        verdict.rationale.contains("退步"),
        "rationale 必须说清是退步：{}",
        verdict.rationale
    );
    assert!(
        verdict.rationale.contains("不准开"),
        "rationale 必须说清不准开：{}",
        verdict.rationale
    );
    assert!(
        verdict
            .rationale
            .contains("（基线 0.750000 → 当前 0.500000）"),
        "rationale 必须写出基线与当前的具体数值：{}",
        verdict.rationale
    );
}

// ============================================
// 4. rationale 如实（数值 + 成本提示）
// ============================================

#[test]
fn rationale_states_the_numbers_and_the_cost_hint() {
    let verdict = evaluate(scenario_improved());

    assert!(
        verdict.rationale.contains("+0.500000"),
        "rationale 必须写出提升的数值：{}",
        verdict.rationale
    );
    assert!(
        verdict
            .rationale
            .contains("（基线 0.500000 → 当前 1.000000）"),
        "rationale 必须写出基线与当前的具体数值：{}",
        verdict.rationale
    );
    assert!(
        verdict.rationale.contains("提升"),
        "rationale 必须说清方向：{}",
        verdict.rationale
    );
    assert!(
        verdict.rationale.contains("每轮额外 1 次全量重算"),
        "rationale 必须带上成本提示：{}",
        verdict.rationale
    );
    assert!(
        verdict.rationale.contains("improved_algo"),
        "rationale 必须能定位到是哪个算法：{}",
        verdict.rationale
    );
}

#[test]
fn rationale_says_cost_is_unknown_when_profile_has_no_hint() {
    let mut input = scenario_improved();
    input.profile.cost_hint = None;

    let verdict = evaluate(input);

    assert!(
        verdict.rationale.contains("未标注"),
        "画像没写成本时必须如实说「未标注」：{}",
        verdict.rationale
    );
    assert!(
        !verdict.rationale.contains("每轮额外 1 次全量重算"),
        "没有成本提示就不许编一个出来：{}",
        verdict.rationale
    );
}

// ============================================
// 5. 批量评估：确定性排序
// ============================================

#[test]
fn evaluate_all_sorts_by_algo_id() {
    let mut zeta = scenario_improved();
    zeta.profile.id = "zeta_algo".to_string();
    let mut alpha = scenario_improved();
    alpha.profile.id = "alpha_algo".to_string();
    let mut mid = scenario_regressed();
    mid.profile.id = "mid_algo".to_string();

    // 故意乱序喂进去。
    let verdicts = evaluate_all(vec![zeta, alpha, mid]);

    let ids: Vec<&str> = verdicts.iter().map(|v| v.algo_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["alpha_algo", "mid_algo", "zeta_algo"],
        "批量评估必须按 id 排序（确定性）"
    );
}

#[test]
fn evaluate_all_on_empty_input_returns_empty_vec() {
    assert!(evaluate_all(Vec::new()).is_empty());
}

// ============================================
// 6. summarize
// ============================================

#[test]
fn summarize_on_empty_input_says_zero_candidates() {
    let text = summarize(&[]);

    assert!(
        text.contains("0 个候选"),
        "空输入必须如实说「0 个候选」：{text}"
    );
}

#[test]
fn summarize_reports_counts() {
    let verdicts = evaluate_all(vec![
        scenario_improved(),
        scenario_improved(),
        scenario_regressed(),
    ]);

    let text = summarize(&verdicts);

    assert!(text.contains("3 个候选"), "必须报总数：{text}");
    assert!(text.contains("2 个值得开"), "必须报值得开的数量：{text}");
    assert!(
        text.contains("1 个不通过不变差闸"),
        "必须报过不了闸的数量：{text}"
    );
}

#[test]
fn summarize_separates_zero_gain_from_gate_failure() {
    // 零增量（过闸但没收益）与掉分（不过闸）是两件事 ⇒ 必须分开报。
    let verdicts = evaluate_all(vec![scenario_unchanged(), scenario_regressed()]);

    let text = summarize(&verdicts);

    assert!(text.contains("2 个候选"), "{text}");
    assert!(text.contains("0 个值得开"), "{text}");
    assert!(text.contains("1 个不通过不变差闸"), "{text}");
    assert!(text.contains("1 个无增量"), "{text}");
}

// ============================================
// 7. 关键指标口径
// ============================================

#[test]
fn primary_metric_constant_matches_metrics_field_name() {
    assert_eq!(
        PRIMARY_METRIC, "recall_at_k",
        "关键指标必须与 Metrics 的字段名一致"
    );

    let verdict = evaluate(scenario_improved());
    assert_eq!(verdict.primary_metric, PRIMARY_METRIC);
    assert_eq!(verdict.primary_metric, "recall_at_k");
}

#[test]
fn delta_follows_recall_not_ndcg() {
    // 只改**顺序**（nDCG 变）但**命中集合不变**（recall 不变）⇒ 增量必须不动。
    let q1 = case("q1", &["a", "b"], &[]);
    let q2 = case("q2", &["c"], &[]);
    let input = EvalInput {
        profile: profile("ndcg_only_algo"),
        golden: golden(&[q1.clone(), q2.clone()]),
        // 基线：q1 两个期望都命中，但顺序反了（nDCG < 1）。
        baseline: batch(vec![
            (q1.clone(), hit(&["b", "a"], &[])),
            (q2.clone(), hit(&["c"], &[])),
        ]),
        // 开启后：命中集合一模一样，只是顺序好了（nDCG = 1）。
        with_algo: batch(vec![(q1, hit(&["a", "b"], &[])), (q2, hit(&["c"], &[]))]),
    };

    let verdict = evaluate(input);

    assert!(
        verdict.delta_primary.abs() < 1e-12,
        "增量只跟关键指标（recall）走，nDCG 变化不该影响它，实际 {}",
        verdict.delta_primary
    );
}

// ============================================
// 8. 画像不影响判定
// ============================================

#[test]
fn consumes_level_does_not_change_the_verdict() {
    let mut texts = scenario_improved();
    texts.profile.consumes = AlgoInput::Text;
    let mut rankings = scenario_improved();
    rankings.profile.consumes = AlgoInput::TextAndRanking;
    let mut full = scenario_improved();
    full.profile.consumes = AlgoInput::FullPipeline;

    let a = evaluate(texts);
    let b = evaluate(rankings);
    let c = evaluate(full);

    // 画像不同 ⇒ 该跑不同的实验，但**量出来的尺子必须是同一把**。
    assert_eq!(a.delta_primary, b.delta_primary);
    assert_eq!(b.delta_primary, c.delta_primary);
    assert_eq!(a.passes_regression_gate, b.passes_regression_gate);
    assert_eq!(b.passes_regression_gate, c.passes_regression_gate);
    assert_eq!(a.worth_enabling, b.worth_enabling);
    assert_eq!(b.worth_enabling, c.worth_enabling);
}

#[test]
fn produces_kind_does_not_change_the_verdict() {
    let mut gap = scenario_improved();
    gap.profile.produces = AnnotationKind::Gap;
    let mut association = scenario_improved();
    association.profile.produces = AnnotationKind::Association;
    let mut score = scenario_improved();
    score.profile.produces = AnnotationKind::Score;

    let a = evaluate(gap);
    let b = evaluate(association);
    let c = evaluate(score);

    assert_eq!(a.delta_primary, b.delta_primary);
    assert_eq!(b.delta_primary, c.delta_primary);
    assert_eq!(a.worth_enabling, c.worth_enabling);
}

// ============================================
// 9. 确定性
// ============================================

#[test]
fn evaluate_is_deterministic() {
    let input = scenario_improved();

    let first = evaluate(input.clone());
    let second = evaluate(input);

    assert_eq!(first, second, "同一输入两次评估必须逐字相等");
    assert_eq!(first.rationale, second.rationale);
}

#[test]
fn evaluate_all_is_deterministic_under_input_permutation() {
    let a = scenario_improved();
    let b = scenario_regressed();

    let forward = evaluate_all(vec![a.clone(), b.clone()]);
    let backward = evaluate_all(vec![b, a]);

    assert_eq!(forward, backward, "换个输入顺序，批量评估结果必须逐字相等");
}

// ============================================
// 10. k 的口径（1% 容忍度边界）
// ============================================

#[test]
fn evaluate_uses_default_k() {
    let input = scenario_improved();

    let via_default = evaluate(input.clone());
    let via_explicit = evaluate_with_k(input, apeireth_memory::d4_eval::DEFAULT_K);

    assert_eq!(via_default, via_explicit, "evaluate 应当等价于用 DEFAULT_K");
}

#[test]
fn tiny_regression_within_tolerance_still_is_not_worth_enabling() {
    // 造一批 100 题（k=100）：基线 recall = 1.0，开启后 recall = 0.99。
    // recall 的相对劣化 = 1%（正好落在「不许变差」闸的容忍度内 ⇒ 过闸），
    // 但增量仍为负 ⇒ **不值得开**（在容忍度里 ≠ 有增量价值）。
    let expect_ids: Vec<String> = (0..100).map(|i| format!("e{i}")).collect();
    let expect_refs: Vec<&str> = expect_ids.iter().map(|s| s.as_str()).collect();
    let q1 = case("q1", &expect_refs, &[]);
    let all: Vec<&str> = expect_refs.clone();
    let mut all_with_last = all.clone();
    all_with_last.pop();

    let input = EvalInput {
        profile: profile("tiny_regression_algo"),
        golden: golden(&[q1.clone()]),
        baseline: batch(vec![(q1.clone(), hit(&all, &[]))]),
        with_algo: batch(vec![(q1, hit(&all_with_last, &[]))]),
    };

    let verdict = evaluate_with_k(input, 100);

    assert!(
        (verdict.baseline.recall_at_k - 1.0).abs() < 1e-12,
        "基线应满分"
    );
    assert!(
        (verdict.with_algo.recall_at_k - 0.99).abs() < 1e-12,
        "开启后应为 0.99，实际 {}",
        verdict.with_algo.recall_at_k
    );
    assert!(
        verdict.passes_regression_gate,
        "1% 的相对劣化在容忍度内 ⇒ 过闸"
    );
    assert!(
        !verdict.worth_enabling,
        "过闸但没增量 ⇒ 仍然不值得开：{}",
        verdict.rationale
    );
    assert!(
        verdict.rationale.contains("无增量"),
        "rationale 必须说清是「无增量」：{}",
        verdict.rationale
    );
}

// ============================================
// 11. 批次坏了 ⇒ 不许静默当 0 分
// ============================================

#[test]
fn broken_batch_is_reported_not_silently_zero() {
    let q1 = case("q1", &["a"], &[]);
    let mut broken = batch(vec![(q1.clone(), hit(&["a"], &[]))]);
    // 制造长度不一致（cases 2 条 / results 1 条）⇒ 指标算不出来。
    broken.cases.push(case("q2", &["b"], &[]));

    let input = EvalInput {
        profile: profile("broken_algo"),
        golden: golden(&[q1.clone()]),
        baseline: batch(vec![(q1.clone(), hit(&["a"], &[]))]),
        with_algo: broken,
    };

    let verdict = evaluate(input);

    assert!(
        verdict.rationale.contains("无法评分"),
        "批次坏了必须如实说「无法评分」：{}",
        verdict.rationale
    );
    assert!(
        !verdict.worth_enabling,
        "算不出分数的算法绝不该被判成「值得开」"
    );
}

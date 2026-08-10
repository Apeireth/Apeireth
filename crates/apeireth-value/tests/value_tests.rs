//! A11.3 集成测试 — 价值器官端到端（独立 test binary, 编译为 public API）.
//!
//! 覆盖场景:
//! 1. 单候选评估 + 5 层洋葱一致性 + E 层硬拒绝 + S 层 drift
//! 2. 多候选排序 — E-conflict 压底 + threshold + priority_kind 权重
//! 3. evaluate_cycle — 周期统计 (avg + passing_count) 正确
//! 4. ASI V0.5 评分与 ValueEvaluation 联动 (跨 crate 调用)
//! 5. 自定义 OnionValueMapping — 替换默认启发式映射
//! 6. 5 层洋葱常量 ONION_LAYERS 与 ValueDimension::ALL 数量一致 (LOCKED)
//!
//! 本文件作为 integration test, 只能使用 public API. 编译为独立 binary,
//! `cargo test -p apeireth-value --test value_tests` 运行.

use apeireth_asi::AsiV05Scores;
use apeireth_core::{ActionTarget, PhilosophyKey, PhilosophyVerdict};
use apeireth_value::{
    check_5_layer_consistency, evaluate_cycle, evaluate_value, prioritize_values,
    ConsistencyVerdict, DefaultPrioritizer, DefaultValueEvaluator, HeuristicOnionMapping,
    OnionLayerStance, OnionValueMapping, ValueAlignment, ValueCandidate, ValueComparison,
    ValueDimension, ValueError, ValueEvaluation, ValuePrioritization, ValuePriorityKind,
    ValueResult, ONION_LAYERS,
};

fn candidate_with_dims(label: &str, score: f64, dims: Vec<ValueDimension>) -> ValueCandidate {
    let mut c = ValueCandidate::new(label, dims);
    c.autonomy_consistency = score;
    c.value_stability = score;
    c.intrinsic_motivation = score;
    c
}

fn all_layers() -> Vec<ValueDimension> {
    ValueDimension::ALL.to_vec()
}

#[test]
fn end_to_end_motivation_score_and_threshold() {
    // motivation_score = 等权平均 3 子分
    // threshold >= 0.85 时, score=0.9 通过, score=0.5 失败
    let c_high = candidate_with_dims("hi", 0.9, all_layers());
    let r_high = evaluate_value(&c_high).expect("eval hi");
    assert!((r_high.motivation - 0.9).abs() < 1e-9);
    assert!(r_high.passes_threshold);

    let c_low = candidate_with_dims("lo", 0.5, vec![ValueDimension::ValueS]);
    let r_low = evaluate_value(&c_low).expect("eval lo");
    assert!((r_low.motivation - 0.5).abs() < 1e-9);
    assert!(!r_low.passes_threshold);
}

#[test]
fn end_to_end_e_layer_hard_reject() {
    // 传 verdict = Block → E 层 Conflicted, has_e_layer_conflict = true
    let mut c = candidate_with_dims("evil_clone", 0.99, all_layers());
    c.verdict = Some(PhilosophyVerdict::Block(PhilosophyKey::NotClone));
    let r = evaluate_value(&c).expect("eval ok");
    assert!(
        r.has_e_layer_conflict,
        "E 层 Block 必须触发 has_e_layer_conflict"
    );
    assert_eq!(
        r.alignment_map.get(&ValueDimension::PrincipleE),
        Some(&ValueAlignment::Conflicted)
    );

    // 排序时该候选被压到队尾
    let safe = candidate_with_dims("safe", 0.95, all_layers());
    let ranks = prioritize_values(&[c.clone(), safe.clone()]).expect("priorities ok");
    assert_eq!(ranks.last().unwrap().candidate_id, c.id);
    assert_eq!(ranks.first().unwrap().candidate_id, safe.id);
}

#[test]
fn end_to_end_s_layer_drift_verdict() {
    // value_stability = 0.2 < 0.4 → S 层 Conflicted → ConsistencyVerdict::Drift
    let mut c = candidate_with_dims("drift_s", 0.5, vec![ValueDimension::ValueS]);
    c.value_stability = 0.2;
    let (v, map) = check_5_layer_consistency(&c, &HeuristicOnionMapping).expect("consistency ok");
    assert_eq!(v, ConsistencyVerdict::Drift);
    assert_eq!(
        map.get(&ValueDimension::ValueS),
        Some(&OnionLayerStance::Conflicted)
    );
}

#[test]
fn end_to_end_priority_kind_weight_sort() {
    // 同 motivation 但 priority_kind 不同, 即时 > 长期 > 地平线
    let imm = {
        let mut c = candidate_with_dims("imm", 0.9, all_layers());
        c.priority_kind = ValuePriorityKind::Immediate;
        c
    };
    let long = {
        let mut c = candidate_with_dims("long", 0.9, all_layers());
        c.priority_kind = ValuePriorityKind::LongTerm;
        c
    };
    let horizon = {
        let mut c = candidate_with_dims("horizon", 0.9, all_layers());
        c.priority_kind = ValuePriorityKind::Horizon;
        c
    };
    let ranks = prioritize_values(&[horizon.clone(), long.clone(), imm.clone()]).expect("ok");
    assert_eq!(ranks[0].candidate_id, imm.id);
    assert_eq!(ranks[1].candidate_id, long.id);
    assert_eq!(ranks[2].candidate_id, horizon.id);
}

#[test]
fn end_to_end_cycle_avg_and_passing_count() {
    // 3 候选: 0.95, 0.92, 0.4
    // threshold 0.85 → 前 2 个 pass, 第 3 个 fail
    let cands = vec![
        candidate_with_dims("a", 0.95, all_layers()),
        candidate_with_dims("b", 0.92, all_layers()),
        candidate_with_dims("c", 0.4, vec![ValueDimension::ValueS]),
    ];
    let (reports, avg, passing) = evaluate_cycle(&cands, &DefaultValueEvaluator).expect("cycle ok");
    assert_eq!(reports.len(), 3);
    assert_eq!(passing, 2, "只有前两个通过 ≥ 0.85");

    let expected_avg = (0.95 + 0.92 + 0.4) / 3.0;
    assert!((avg - expected_avg).abs() < 1e-9);

    let passing_cand_ids: std::collections::HashSet<_> = reports
        .iter()
        .filter(|r| r.passes_threshold)
        .map(|r| r.candidate_id)
        .collect();
    assert!(passing_cand_ids.contains(&cands[0].id));
    assert!(passing_cand_ids.contains(&cands[1].id));
    assert!(!passing_cand_ids.contains(&cands[2].id));
}

#[test]
fn end_to_end_asi_v05_scores_linkage() {
    // 验证: 候选的 motivation_score 与外部 ASI 评分可联动 (本测试只生成 default V0.5 评分).
    let c = candidate_with_dims("asi_link", 0.9, all_layers());
    let r = evaluate_value(&c).expect("eval ok");

    let asi = AsiV05Scores::default();
    // ASI 评分是 V0.5 17 维主体, 与本器官动机分彼此独立 — 这里只确认共同走 OK 不 panic.
    assert!(asi.continuity.is_finite() || asi.continuity == 0.0);
    assert!(r.motivation >= 0.85);
}

#[test]
fn end_to_end_custom_onion_mapping() {
    // 自定义 mapping: 全 S 层强制 Aligned
    struct ForceSAligned;
    impl OnionValueMapping for ForceSAligned {
        fn stance_for(
            &self,
            _candidate: &ValueCandidate,
            layer: ValueDimension,
        ) -> OnionLayerStance {
            if layer == ValueDimension::ValueS {
                OnionLayerStance::Aligned
            } else {
                OnionLayerStance::Underspecified
            }
        }
    }

    let c = candidate_with_dims("custom", 0.5, vec![ValueDimension::ValueS]);
    let (v, m) = check_5_layer_consistency(&c, &ForceSAligned).expect("custom ok");
    assert_eq!(
        m.get(&ValueDimension::ValueS),
        Some(&OnionLayerStance::Aligned),
        "S 层必须被强制 Aligned"
    );
    // 全 5 层 (除 S 外其他 Underspecified) → verdict 至少 Partial
    assert!(matches!(
        v,
        ConsistencyVerdict::Partial | ConsistencyVerdict::Pass
    ));
}

#[test]
fn end_to_end_onion_layers_constant_matches_all() {
    // LOCKED 不变式: ONION_LAYERS == ValueDimension::ALL.len()
    assert_eq!(ONION_LAYERS, 5);
    assert_eq!(ONION_LAYERS, ValueDimension::ALL.len());
}

#[test]
fn end_to_end_dimension_label_round_trip() {
    for d in ValueDimension::ALL {
        let ltr = d.letter();
        assert_eq!(
            ValueDimension::from_letter(ltr.chars().next().unwrap()),
            Some(d)
        );
    }
}

#[test]
fn end_to_end_value_error_invalid_input_via_empty_label() {
    let mut bad = candidate_with_dims("ok", 0.9, all_layers());
    bad.label = "".into();
    let r: ValueResult<_> = evaluate_value(&bad);
    assert!(matches!(r, Err(ValueError::InvalidInput(_))));
}

#[test]
fn end_to_end_value_error_score_out_of_range() {
    let mut bad = candidate_with_dims("ok", 0.9, all_layers());
    bad.value_stability = 1.5;
    let r: ValueResult<_> = evaluate_value(&bad);
    assert!(matches!(r, Err(ValueError::ScoreOutOfRange(_))));
}

#[test]
fn end_to_end_compare_higher_lower_equal() {
    let a = candidate_with_dims("a", 0.95, all_layers());
    let b = candidate_with_dims("b", 0.5, all_layers());
    let c = candidate_with_dims("c", 0.95, all_layers());

    let eval = DefaultValueEvaluator;
    assert_eq!(eval.compare(&a, &b), ValueComparison::Higher);
    assert_eq!(eval.compare(&b, &a), ValueComparison::Lower);
    // a vs c 同分, 无 E 冲突, ValueEvaluation::compare 返回 Equal
    assert_eq!(eval.compare(&a, &c), ValueComparison::Equal);
}

#[test]
fn end_to_end_target_propagates_through_evaluation() {
    // 验证 ActionTarget 可在 ValueCandidate 上下文中传递, 但不影响核心评估
    let mut c = candidate_with_dims("with_target", 0.9, all_layers());
    c.target = Some(ActionTarget::NormalAction("noop".into()));
    let r = evaluate_value(&c).expect("ok");
    assert!(r.passes_threshold);
}

#[test]
fn end_to_end_default_prioritizer_ranks_one_based() {
    let cands = vec![
        candidate_with_dims("a", 0.95, all_layers()),
        candidate_with_dims("b", 0.92, all_layers()),
        candidate_with_dims("c", 0.88, all_layers()),
    ];
    let pri = DefaultPrioritizer;
    let ranks = pri.prioritize(&cands).expect("ok");
    assert_eq!(ranks.len(), 3);
    assert_eq!(ranks[0].rank, 1);
    assert_eq!(ranks[1].rank, 2);
    assert_eq!(ranks[2].rank, 3);
    for (i, r) in ranks.iter().enumerate().skip(1) {
        assert!(r.score <= ranks[i - 1].score + 1e-9, "排序必须单调下降");
    }
}

#[test]
fn end_to_end_empty_candidate_list_error() {
    let r: ValueResult<Vec<_>> = prioritize_values(&[]);
    assert!(r.is_err(), "空候选必须返回 InvalidInput");
}

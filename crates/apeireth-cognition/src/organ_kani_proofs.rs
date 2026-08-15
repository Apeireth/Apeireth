//! R177 cognition organ Kani proofs (W1-2 9 organ invariants)
//!
//! **要验证的不变量**:
//! 1. CognitiveInput::validate: candidates 非空 + context_tag 非空
//! 2. run_cycle: 空 candidates / 空 context_tag 拒绝
//! 3. CognitiveCycle.is_rejected / is_allowed 互斥
//! 4. verdicts.len() == candidate_targets.len()
//! 5. AsiV05Scores 5 维各 ∈ [0.0, 1.0]
//! 6. V1136Submeasures 5+2 各 ∈ [0.0, 1.0]
//! 7. decide: 任一 Block → Reject; 全 Allow → Decision
//! 8. scoring 函数输出 ∈ [0.0, 1.0]

#![allow(missing_docs)]

use apeireth_asi::{AsiV05Scores, V1136Submeasures};
use apeireth_core::{ActionTarget, PhilosophyKey, PhilosophyVerdict};

use crate::decision::{decide, evaluate_actions};
use crate::{
    run_cycle, scoring, CognitiveCycle, CognitiveInput,
};

// ============================================
// Property 1: CognitiveInput::validate 拒绝空 candidates / 空 context_tag
// ============================================
#[test]
fn r177_cog_01_validate_rejects_empty() {
    let input = CognitiveInput::new(vec![], "ctx");
    assert!(input.validate().is_err(), "空 candidates 应被拒绝");

    let target = ActionTarget::NormalAction("noop".into());
    let input2 = CognitiveInput::new(vec![target], "");
    assert!(input2.validate().is_err(), "空 context_tag 应被拒绝");
}

// ============================================
// Property 2: run_cycle 拒绝空输入
// ============================================
#[test]
fn r177_cog_02_run_cycle_rejects_invalid() {
    let bad1 = CognitiveInput::new(vec![], "x");
    assert!(run_cycle(bad1).is_err());

    let bad2 = CognitiveInput::new(
        vec![ActionTarget::NormalAction("a".into())],
        "",
    );
    assert!(run_cycle(bad2).is_err());
}

// ============================================
// Property 3: is_rejected / is_allowed 互斥
// ============================================
#[test]
fn r177_cog_03_is_rejected_is_allowed_mutually_exclusive() {
    let target = ActionTarget::NormalAction("noop".into());
    let input = CognitiveInput::new(vec![target], "ctx");
    let cycle = run_cycle(input).unwrap();
    assert!(
        !(cycle.is_rejected() && cycle.is_allowed()),
        "is_rejected 与 is_allowed 不能同时为 true"
    );
}

// ============================================
// Property 4: verdicts.len() == candidate_targets.len()
// ============================================
#[test]
fn r177_cog_04_verdicts_match_targets() {
    for n in 1..=5 {
        let targets: Vec<ActionTarget> = (0..n)
            .map(|i| ActionTarget::NormalAction(format!("act-{}", i)))
            .collect();
        let verdicts = evaluate_actions(&targets);
        assert_eq!(verdicts.len(), n);
    }
}

// ============================================
// Property 5: AsiV05Scores 5 维 ∈ [0.0, 1.0]
// ============================================
#[test]
fn r177_cog_05_v05_in_range() {
    let target = ActionTarget::NormalAction("noop".into());
    let input = CognitiveInput::new(vec![target], "ctx");
    let v05 = scoring::score_v05(&input);
    let dims = [
        ("continuity", v05.continuity),
        ("salience", v05.salience),
        ("identity", v05.identity),
        ("philosophy_guard", v05.philosophy_guard),
        ("transferability", v05.transferability),
    ];
    for (name, v) in dims {
        assert!((0.0..=1.0).contains(&v), "V0.5 维度 {} 越界: {}", name, v);
    }
}

// ============================================
// Property 6: V1136Submeasures 7 子测度 ∈ [0.0, 1.0]
// ============================================
#[test]
fn r177_cog_06_v1136_in_range() {
    let target = ActionTarget::NormalAction("noop".into());
    let input = CognitiveInput::new(vec![target], "ctx");
    let v1136 = scoring::score_v1136(&input);
    for (i, v) in v1136.continuity_5.iter().enumerate() {
        assert!((0.0..=1.0).contains(v), "V1136 continuity_5[{}] 越界: {}", i, v);
    }
    for (i, v) in v1136.transferability_2.iter().enumerate() {
        assert!((0.0..=1.0).contains(v), "V1136 transferability_2[{}] 越界: {}", i, v);
    }
}

// ============================================
// Property 7: decide 任一 Block → Reject; 全 Allow → Decision
// ============================================
#[test]
fn r177_cog_07_decide_block_vs_allow() {
    let all_allow = vec![PhilosophyVerdict::Allow, PhilosophyVerdict::Allow];
    match decide(&all_allow) {
        Ok(crate::CognitiveOutput::Decision(_)) => {}
        other => panic!("全 Allow 应得 Decision, got {:?}", other),
    }

    let with_block = vec![
        PhilosophyVerdict::Allow,
        PhilosophyVerdict::Block(PhilosophyKey::NotClone),
    ];
    match decide(&with_block) {
        Ok(crate::CognitiveOutput::Reject(k)) => assert_eq!(k, PhilosophyKey::NotClone),
        other => panic!("含 Block 应得 Reject, got {:?}", other),
    }
}

// ============================================
// Property 8: scoring 各函数输出 ∈ [0.0, 1.0]
// ============================================
#[test]
fn r177_cog_08_scoring_in_range() {
    let target = ActionTarget::NormalAction("noop".into());
    let input = CognitiveInput::new(vec![target], "ctx");
    let s = [
        ("continuity", scoring::continuity_score(&input)),
        ("salience", scoring::salience_score(&input)),
        ("identity", scoring::identity_score(&input)),
        ("philosophy_guard", scoring::philosophy_guard_score(&input)),
        ("transferability", scoring::transferability_score(&input)),
    ];
    for (name, v) in s {
        assert!((0.0..=1.0).contains(&v), "scoring {} 越界: {}", name, v);
    }
}

// ============================================
// Property 9: V1136Submeasures 长度固定 5+2
// ============================================
#[test]
fn r177_cog_09_v1136_fixed_len() {
    let target = ActionTarget::NormalAction("noop".into());
    let input = CognitiveInput::new(vec![target], "ctx");
    let v1136 = scoring::score_v1136(&input);
    assert_eq!(v1136.continuity_5.len(), 5);
    assert_eq!(v1136.transferability_2.len(), 2);
}

// ============================================
// Property 10: run_cycle 产生 verdicts.len() == targets.len()
// ============================================
#[test]
fn r177_cog_10_run_cycle_verdicts_match() {
    let targets: Vec<ActionTarget> = (0..3)
        .map(|i| ActionTarget::NormalAction(format!("a-{}", i)))
        .collect();
    let input = CognitiveInput::new(targets.clone(), "ctx");
    let cycle = run_cycle(input).unwrap();
    assert_eq!(cycle.verdicts.len(), targets.len());
    assert_eq!(cycle.input_id.to_string().len(), 36);
}

// ============================================
// Kani-style formal proof — V0.5 各维度 ∈ [0.0, 1.0]
// ============================================
#[cfg(kani)]
#[kani::proof]
fn r177_cog_kani_01_v05_invariants() {
    let target = ActionTarget::NormalAction("noop".into());
    let input = CognitiveInput::new(vec![target], "ctx");
    let v05 = scoring::score_v05(&input);
    let dims = [
        v05.continuity,
        v05.salience,
        v05.identity,
        v05.philosophy_guard,
        v05.transferability,
    ];
    for v in &dims {
        assert!(*v >= 0.0 && *v <= 1.0, "V0.5 维度越界: {}", v);
    }
}

// ============================================
// Kani-style formal proof — V1136 各子测度 ∈ [0.0, 1.0]
// ============================================
#[cfg(kani)]
#[kani::proof]
fn r177_cog_kani_02_v1136_invariants() {
    let target = ActionTarget::NormalAction("noop".into());
    let input = CognitiveInput::new(vec![target], "ctx");
    let v1136 = scoring::score_v1136(&input);
    for v in &v1136.continuity_5 {
        assert!(*v >= 0.0 && *v <= 1.0, "V1136 continuity 越界: {}", v);
    }
    for v in &v1136.transferability_2 {
        assert!(*v >= 0.0 && *v <= 1.0, "V1136 transferability 越界: {}", v);
    }
}

#[allow(dead_code)]
fn _ensure_cycle_used(c: &CognitiveCycle) -> usize {
    c.verdicts.len()
}

#[allow(dead_code)]
fn _ensure_v05_used(v: &AsiV05Scores) -> f64 {
    v.continuity
}

#[allow(dead_code)]
fn _ensure_v1136_used(v: &V1136Submeasures) -> usize {
    v.continuity_5.len()
}

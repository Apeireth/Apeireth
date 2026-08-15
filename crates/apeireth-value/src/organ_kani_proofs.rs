//! R177 value organ Kani proofs (W2)

#![allow(missing_docs)]

use crate::{
    ValueAlignment, ValueCandidate, ValueComparison, ValueDimension, ValuePriorityKind,
};

// ============================================
// Property 1: ValueDimension 5 层 ALL
// ============================================
#[test]
fn r177_val_01_dim_all_5() {
    assert_eq!(ValueDimension::ALL.len(), 5);
}

#[test]
fn r177_val_02_dim_letters() {
    assert_eq!(ValueDimension::PrincipleE.letter(), "E");
    assert_eq!(ValueDimension::ValueS.letter(), "S");
    assert_eq!(ValueDimension::ExperienceA.letter(), "A");
    assert_eq!(ValueDimension::MethodologyM.letter(), "M");
    assert_eq!(ValueDimension::OperationO.letter(), "O");
}

#[test]
fn r177_val_03_from_letter_roundtrip() {
    for &d in &ValueDimension::ALL {
        let l = d.letter().chars().next().unwrap();
        assert_eq!(ValueDimension::from_letter(l), Some(d));
        // 大小写
        let l_lower = l.to_ascii_lowercase();
        assert_eq!(ValueDimension::from_letter(l_lower), Some(d));
    }
    assert_eq!(ValueDimension::from_letter('X'), None);
    assert_eq!(ValueDimension::from_letter(' '), None);
}

#[test]
fn r177_val_04_priority_kind_weights() {
    assert!(ValuePriorityKind::Immediate.weight() > ValuePriorityKind::ShortTerm.weight());
    assert!(ValuePriorityKind::ShortTerm.weight() > ValuePriorityKind::LongTerm.weight());
    assert!(ValuePriorityKind::LongTerm.weight() > ValuePriorityKind::Horizon.weight());
}

#[test]
fn r177_val_05_candidate_validate() {
    let mut c = ValueCandidate::new("test", vec![ValueDimension::ValueS]);
    assert!(c.validate().is_ok(), "正常 candidate 应通过");

    c.label = "".into();
    assert!(c.validate().is_err(), "空 label 应拒绝");

    c.label = "ok".into();
    c.dimensions.clear();
    assert!(c.validate().is_err(), "空 dimensions 应拒绝");
}

#[test]
fn r177_val_06_candidate_score_in_range() {
    let mut c = ValueCandidate::new("test", vec![ValueDimension::ValueS]);
    c.autonomy_consistency = 1.0;
    c.value_stability = 1.0;
    c.intrinsic_motivation = 1.0;
    let s = c.motivation_score();
    assert!(s >= 0.0 && s <= 1.0);
    assert!((s - 1.0).abs() < 1e-9);

    c.autonomy_consistency = 0.0;
    c.value_stability = 0.0;
    c.intrinsic_motivation = 0.0;
    assert!((c.motivation_score() - 0.0).abs() < 1e-9);
}

#[test]
fn r177_val_07_candidate_passes_threshold() {
    let mut c = ValueCandidate::new("test", vec![ValueDimension::ValueS]);
    c.autonomy_consistency = 0.9;
    c.value_stability = 0.9;
    c.intrinsic_motivation = 0.9;
    assert!(c.passes_threshold(0.85), "0.9 ≥ 0.85 应通过");
    assert!(!c.passes_threshold(0.95), "0.9 < 0.95 应不通过");
}

#[test]
fn r177_val_08_value_comparison_variants() {
    let variants = [
        ValueComparison::Higher,
        ValueComparison::Lower,
        ValueComparison::Equal,
        ValueComparison::Incomparable,
    ];
    assert_eq!(variants.len(), 4);
    assert_eq!(ValueComparison::Higher, ValueComparison::Higher);
    assert_ne!(ValueComparison::Higher, ValueComparison::Lower);
}

#[test]
fn r177_val_09_alignment_variants() {
    let a1 = ValueAlignment::Aligned;
    let a2 = ValueAlignment::Conflicted;
    assert_ne!(a1, a2);
}

#[test]
fn r177_val_10_dim_ai_self_modifiable() {
    // PrincipleE 不可自改, 其余可自改 (按 §13.2 文档)
    assert!(!ValueDimension::PrincipleE.is_ai_self_modifiable());
    assert!(ValueDimension::ExperienceA.is_ai_self_modifiable());
    assert!(ValueDimension::MethodologyM.is_ai_self_modifiable());
    assert!(ValueDimension::OperationO.is_ai_self_modifiable());
}

#[cfg(kani)]
#[kani::proof]
fn r177_val_kani_01_dim_count() {
    assert_eq!(ValueDimension::ALL.len(), 5);
}

#[cfg(kani)]
#[kani::proof]
fn r177_val_kani_02_priority_weights_descending() {
    assert!(ValuePriorityKind::Immediate.weight() > ValuePriorityKind::ShortTerm.weight());
    assert!(ValuePriorityKind::ShortTerm.weight() > ValuePriorityKind::LongTerm.weight());
    assert!(ValuePriorityKind::LongTerm.weight() > ValuePriorityKind::Horizon.weight());
}

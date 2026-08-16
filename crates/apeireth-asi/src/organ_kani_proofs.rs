//! R177 asi organ Kani proofs (W5)

#![allow(missing_docs)]

use crate::{
    AsiV05Scores, V1136Submeasures, V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT,
    V1136_SUBMEASURE_NAMES,
};

#[test]
fn r177_asi_01_v05_24_dims() {
    assert_eq!(V05_DIM_COUNT, 24);
    assert_eq!(V05_DIMENSION_NAMES.len(), 24);
}

#[test]
fn r177_asi_02_v1136_9_subs() {
    assert_eq!(V1136_SUBMEASURE_COUNT, 9);
    assert_eq!(V1136_SUBMEASURE_NAMES.len(), 9);
}

#[test]
fn r177_asi_03_v05_dim_names_distinct() {
    let mut seen = std::collections::HashSet::new();
    for n in &V05_DIMENSION_NAMES {
        assert!(seen.insert(*n), "V0.5 dim name 重复: {}", n);
    }
    assert_eq!(seen.len(), 24);
}

#[test]
fn r177_asi_04_v1136_sub_names_distinct() {
    let mut seen = std::collections::HashSet::new();
    for n in &V1136_SUBMEASURE_NAMES {
        assert!(seen.insert(*n), "V1136 sub name 重复: {}", n);
    }
    assert_eq!(seen.len(), 9);
}

#[test]
fn r177_asi_05_v05_categories() {
    // 5 类 × 5 维 = 25 但 transferability 4 维 = 24
    let names = V05_DIMENSION_NAMES;
    let continuity_count = names
        .iter()
        .filter(|n| {
            n.contains("continuity")
                || n.contains("recall")
                || n.contains("context_window")
                || n.contains("session_recovery")
                || n.contains("identity_persistence")
        })
        .count();
    let salience_count = names
        .iter()
        .filter(|n| {
            n.contains("importance")
                || n.contains("novelty")
                || n.contains("actionability")
                || n.contains("confidence")
                || n.contains("temporal_relevance")
        })
        .count();
    let identity_count = names
        .iter()
        .filter(|n| {
            n.contains("core_values")
                || n.contains("voice_")
                || n.contains("behavioral")
                || n.contains("role_")
                || n.contains("philosophy_alignment")
        })
        .count();
    let philosophy_count = names
        .iter()
        .filter(|n| {
            n.starts_with("v1_")
                || n.starts_with("v2_")
                || n.starts_with("v3_")
                || n.contains("cone_of_truth")
                || n.contains("action_guard")
        })
        .count();
    assert!(continuity_count >= 1);
    assert!(salience_count >= 1);
    assert!(identity_count >= 1);
    assert!(philosophy_count >= 1);
}

#[test]
fn r177_asi_06_v05_scores_struct() {
    let s = AsiV05Scores {
        continuity: 0.5,
        salience: 0.5,
        identity: 0.5,
        philosophy_guard: 0.5,
        transferability: 0.5,
    };
    assert_eq!(s.continuity, 0.5);
    assert_eq!(s.transferability, 0.5);
}

#[test]
fn r177_asi_07_v1136_struct() {
    let s = V1136Submeasures {
        continuity_5: [0.5; 5],
        transferability_2: [0.5; 2],
    };
    assert_eq!(s.continuity_5.len(), 5);
    assert_eq!(s.transferability_2.len(), 2);
}

#[test]
fn r177_asi_08_v05_scores_default() {
    let s = AsiV05Scores::default();
    assert_eq!(s.continuity, 0.0);
    assert_eq!(s.transferability, 0.0);
}

#[test]
fn r177_asi_09_v1136_default() {
    let s = V1136Submeasures::default();
    for v in &s.continuity_5 {
        assert_eq!(*v, 0.0);
    }
    for v in &s.transferability_2 {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn r177_asi_10_placeholder() {
    assert!(crate::placeholder().contains("asi") || crate::placeholder().contains("R14"));
}

#[cfg(kani)]
#[kani::proof]
fn r177_asi_kani_01_v05_count() {
    assert_eq!(V05_DIM_COUNT, 24);
    assert_eq!(V05_DIMENSION_NAMES.len(), 24);
}

#[cfg(kani)]
#[kani::proof]
fn r177_asi_kani_02_v1136_count() {
    assert_eq!(V1136_SUBMEASURE_COUNT, 9);
    assert_eq!(V1136_SUBMEASURE_NAMES.len(), 9);
}

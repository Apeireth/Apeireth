//! P27 public API acceptance: every stage-4 abstraction has two observable scenarios.

use apeireth_cognition::{
    Abstraction, BasicCognitiveEngine, Cognition, Consolidation, Forgetting, Intuition, Learning,
    MetaCognition, Reasoning, Recall,
};

fn engine() -> BasicCognitiveEngine {
    BasicCognitiveEngine
}

#[test]
fn cognition_combines_normalized_observations() {
    assert_eq!(
        engine().cognize(&["  alpha ", "beta"]),
        Some("alpha | beta".into())
    );
}

#[test]
fn cognition_rejects_empty_observations() {
    assert_eq!(engine().cognize(&["", "   "]), None);
}

#[test]
fn intuition_selects_first_non_empty_candidate() {
    assert_eq!(engine().intuit(&["", "safe", "later"]), Some("safe"));
}

#[test]
fn intuition_returns_none_without_candidate() {
    assert_eq!(engine().intuit(&[]), None);
}

#[test]
fn reasoning_accepts_sufficient_true_premises() {
    assert!(engine().reason(&[true, true]));
}

#[test]
fn reasoning_rejects_false_or_empty_premises() {
    assert!(!engine().reason(&[true, false]));
    assert!(!engine().reason(&[]));
}

#[test]
fn meta_cognition_preserves_in_range_confidence() {
    assert_eq!(engine().assess_confidence(0.75), 0.75);
}

#[test]
fn meta_cognition_clamps_or_sanitizes_confidence() {
    assert_eq!(engine().assess_confidence(2.0), 1.0);
    assert_eq!(engine().assess_confidence(f64::NAN), 0.0);
}

#[test]
fn recall_finds_first_matching_memory() {
    assert_eq!(
        engine().recall("rust", &["old", "safe rust", "rust later"]),
        Some("safe rust")
    );
}

#[test]
fn recall_rejects_empty_or_missing_query() {
    assert_eq!(engine().recall("", &["anything"]), None);
    assert_eq!(engine().recall("absent", &["anything"]), None);
}

#[test]
fn consolidation_removes_blanks_and_adjacent_duplicates() {
    assert_eq!(
        engine().consolidate(&[" one ", "one", "", "two"]),
        vec!["one".to_string(), "two".to_string()]
    );
}

#[test]
fn consolidation_preserves_distinct_order() {
    assert_eq!(
        engine().consolidate(&["a", "b", "a"]),
        vec!["a".to_string(), "b".to_string(), "a".to_string()]
    );
}

#[test]
fn forgetting_retains_memories_selected_by_policy() {
    assert_eq!(
        engine().forget(&["keep-a", "drop", "keep-b"], &|item| item
            .starts_with("keep")),
        vec!["keep-a".to_string(), "keep-b".to_string()]
    );
}

#[test]
fn forgetting_can_remove_every_memory() {
    assert!(engine().forget(&["a", "b"], &|_| false).is_empty());
}

#[test]
fn learning_applies_positive_and_negative_feedback() {
    assert_eq!(engine().learn(0.5, 0.2), 0.7);
    assert_eq!(engine().learn(0.5, -0.2), 0.3);
}

#[test]
fn learning_clamps_and_sanitizes_values() {
    assert_eq!(engine().learn(0.9, 0.5), 1.0);
    assert_eq!(engine().learn(f64::NAN, 0.2), 0.2);
}

#[test]
fn abstraction_finds_ascii_and_unicode_commonality() {
    assert_eq!(
        engine().abstract_commonality(&["reason", "reader"]),
        Some("rea".into())
    );
    assert_eq!(
        engine().abstract_commonality(&["认知层", "认知器官"]),
        Some("认知".into())
    );
}

#[test]
fn abstraction_handles_empty_or_unrelated_samples() {
    assert_eq!(engine().abstract_commonality(&[]), None);
    assert_eq!(engine().abstract_commonality(&["alpha", "beta"]), None);
}

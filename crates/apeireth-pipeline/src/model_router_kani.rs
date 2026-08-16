//! R177 pipeline model_router + provider_registry invariants (W5)

#![allow(missing_docs)]

use crate::model_router::{RoutingCondition, RoutingRule};
use crate::provider_registry::{
    ProviderCapability, SelectionStrategy, ALL_PROVIDER_CAPABILITIES, ALL_SELECTION_STRATEGIES,
};
use apeireth_protocol::MessageRole;

#[test]
fn r177_mr_01_routing_condition_5_variants() {
    let r1 = RoutingCondition::KeywordMatch(vec!["foo".to_string()]);
    let r2 = RoutingCondition::TokenCountRange(0, 100);
    let r3 = RoutingCondition::RoleBased(MessageRole::User);
    let r4 = RoutingCondition::Complexity(0.5);
    let _r5 = RoutingCondition::Custom(std::sync::Arc::new(|_s: &str| true));
    // 仅断言 5 种 variant 都可构造 (不依赖 PartialEq)
    match r1 {
        RoutingCondition::KeywordMatch(_) => {}
        _ => panic!(),
    }
    match r2 {
        RoutingCondition::TokenCountRange(_, _) => {}
        _ => panic!(),
    }
    match r3 {
        RoutingCondition::RoleBased(_) => {}
        _ => panic!(),
    }
    match r4 {
        RoutingCondition::Complexity(_) => {}
        _ => panic!(),
    }
}

#[test]
fn r177_mr_02_routing_rule_keyword() {
    let r = RoutingRule::keyword("rule1", vec!["foo".to_string()], "gpt-4o");
    assert_eq!(r.name, "rule1");
    assert_eq!(r.target_model, "gpt-4o");
}

#[test]
fn r177_mr_03_routing_rule_token_range() {
    let r = RoutingRule::token_range("tr1", 0, 1000, "gpt-3.5");
    assert_eq!(r.name, "tr1");
    assert_eq!(r.target_model, "gpt-3.5");
}

#[test]
fn r177_mr_04_routing_rule_role() {
    let r = RoutingRule::role("r1", MessageRole::User, "claude-3");
    assert_eq!(r.name, "r1");
}

#[test]
fn r177_mr_05_routing_rule_complexity() {
    let r = RoutingRule::complexity("c1", 0.5, "gpt-4");
    assert_eq!(r.name, "c1");
}

#[test]
fn r177_mr_06_provider_capability_6() {
    assert_eq!(ALL_PROVIDER_CAPABILITIES.len(), 6);
    assert!(ALL_PROVIDER_CAPABILITIES.contains(&ProviderCapability::Chat));
    assert!(ALL_PROVIDER_CAPABILITIES.contains(&ProviderCapability::Completion));
    assert!(ALL_PROVIDER_CAPABILITIES.contains(&ProviderCapability::Embedding));
    assert!(ALL_PROVIDER_CAPABILITIES.contains(&ProviderCapability::Tool));
    assert!(ALL_PROVIDER_CAPABILITIES.contains(&ProviderCapability::Vision));
    assert!(ALL_PROVIDER_CAPABILITIES.contains(&ProviderCapability::Audio));
}

#[test]
fn r177_mr_07_provider_capability_distinct() {
    let mut seen = std::collections::HashSet::new();
    for c in &ALL_PROVIDER_CAPABILITIES {
        assert!(seen.insert(*c), "capability 重复: {:?}", c);
    }
}

#[test]
fn r177_mr_08_selection_strategy_5() {
    assert_eq!(ALL_SELECTION_STRATEGIES.len(), 5);
    assert!(ALL_SELECTION_STRATEGIES.contains(&SelectionStrategy::RoundRobin));
    assert!(ALL_SELECTION_STRATEGIES.contains(&SelectionStrategy::LowestLatency));
    assert!(ALL_SELECTION_STRATEGIES.contains(&SelectionStrategy::LowestCost));
    assert!(ALL_SELECTION_STRATEGIES.contains(&SelectionStrategy::Capability));
    assert!(ALL_SELECTION_STRATEGIES.contains(&SelectionStrategy::Custom));
}

#[test]
fn r177_mr_09_selection_strategy_distinct() {
    let mut seen = std::collections::HashSet::new();
    for s in &ALL_SELECTION_STRATEGIES {
        assert!(seen.insert(*s), "strategy 重复: {:?}", s);
    }
}

#[test]
fn r177_mr_10_capability_display() {
    assert_eq!(format!("{}", ProviderCapability::Chat), "chat");
    assert_eq!(format!("{}", ProviderCapability::Vision), "vision");
}

#[cfg(kani)]
#[kani::proof]
fn r177_mr_kani_01_capability_count() {
    assert_eq!(ALL_PROVIDER_CAPABILITIES.len(), 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_mr_kani_02_strategy_count() {
    assert_eq!(ALL_SELECTION_STRATEGIES.len(), 5);
}

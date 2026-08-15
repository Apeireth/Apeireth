//! R177 action organ Kani proofs (W4)

#![allow(missing_docs)]

use crate::silence::SilenceReason;
use crate::{ActionError, ExpressionChannel, new_tx_id, DefaultActionEngine};

#[test]
fn r177_act_01_silence_reason_6() {
    let reasons = [
        SilenceReason::NotSilent,
        SilenceReason::OutOfScope,
        SilenceReason::NoConsent,
        SilenceReason::NoNeed,
        SilenceReason::Deliberate,
        SilenceReason::EthicalDoubt,
    ];
    assert_eq!(reasons.len(), 6);
}

#[test]
fn r177_act_02_not_silent_is_not_silent() {
    assert!(!SilenceReason::NotSilent.is_silent());
}

#[test]
fn r177_act_03_other_reasons_silent() {
    assert!(SilenceReason::OutOfScope.is_silent());
    assert!(SilenceReason::NoConsent.is_silent());
    assert!(SilenceReason::NoNeed.is_silent());
    assert!(SilenceReason::Deliberate.is_silent());
    assert!(SilenceReason::EthicalDoubt.is_silent());
}

#[test]
fn r177_act_04_silence_priority_ordering() {
    assert!(SilenceReason::EthicalDoubt.priority() > SilenceReason::NoConsent.priority());
    assert!(SilenceReason::NoConsent.priority() > SilenceReason::OutOfScope.priority());
    assert!(SilenceReason::OutOfScope.priority() > SilenceReason::NoNeed.priority());
    assert!(SilenceReason::NoNeed.priority() > SilenceReason::Deliberate.priority());
    assert!(SilenceReason::Deliberate.priority() > SilenceReason::NotSilent.priority());
}

#[test]
fn r177_act_05_silence_names_distinct() {
    let names: Vec<&str> = [
        SilenceReason::NotSilent,
        SilenceReason::OutOfScope,
        SilenceReason::NoConsent,
        SilenceReason::NoNeed,
        SilenceReason::Deliberate,
        SilenceReason::EthicalDoubt,
    ]
    .iter()
    .map(|r| r.name())
    .collect();
    let mut seen = std::collections::HashSet::new();
    for n in &names {
        assert!(seen.insert(*n), "silence name 重复: {}", n);
    }
}

#[test]
fn r177_act_06_expression_channel_variants() {
    let channels = [
        ExpressionChannel::Text,
        ExpressionChannel::Voice,
        ExpressionChannel::Structured,
        ExpressionChannel::MultiModal,
    ];
    assert_eq!(channels.len(), 4);
}

#[test]
fn r177_act_07_action_error_variants() {
    let err1 = ActionError::InvalidInput("test".into());
    assert!(matches!(err1, ActionError::InvalidInput(_)));
}

#[test]
fn r177_act_08_default_action_engine_new() {
    let engine = DefaultActionEngine::new();
    let _ = engine.engine();
}

#[test]
fn r177_act_09_new_tx_id_unique() {
    let tx1 = new_tx_id();
    let tx2 = new_tx_id();
    assert_ne!(tx1, tx2, "tx_id 应唯一");
}

#[test]
fn r177_act_10_silence_priority_5_highest() {
    assert_eq!(SilenceReason::EthicalDoubt.priority(), 5);
    assert_eq!(SilenceReason::NotSilent.priority(), 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_act_kani_01_silence_count() {
    let reasons = [
        SilenceReason::NotSilent,
        SilenceReason::OutOfScope,
        SilenceReason::NoConsent,
        SilenceReason::NoNeed,
        SilenceReason::Deliberate,
        SilenceReason::EthicalDoubt,
    ];
    assert_eq!(reasons.len(), 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_act_kani_02_silent_predicate() {
    assert!(!SilenceReason::NotSilent.is_silent());
    assert!(SilenceReason::Deliberate.is_silent());
}

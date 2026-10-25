//! D-2 映射口径的测试（`council_ballots.rs` 的子模块）。
//!
//! 覆盖：4 个 `AdvisorDecision` 变体 → `StanceKind`、7 个 `AdvisorKind` 变体 →
//! `rubric::AdvisorDomain`、显式置信度 vs 兜底置信度、权重取域默认权重、票上带域、
//! 落盘记录与票一致。

use super::{
    ballot_from_evaluation, domain_for_kind, kind_slug, record_from_evaluation,
    stance_for_decision, stance_slug,
};
use apeireth_governance::rubric;
use apeireth_orchestration::{AdvisorDecision, AdvisorEvaluation, AdvisorKind, AdvisorVerdict};

fn evaluation(
    advisor: &str,
    kind: AdvisorKind,
    score: f64,
    decision: AdvisorDecision,
    confidence: Option<f64>,
) -> AdvisorEvaluation {
    AdvisorEvaluation {
        advisor: advisor.to_string(),
        kind,
        verdict: AdvisorVerdict {
            score,
            verdict: decision,
            critique: String::new(),
            confidence,
        },
    }
}

#[test]
fn allow_maps_to_approve() {
    assert_eq!(
        stance_for_decision(AdvisorDecision::Allow),
        rubric::StanceKind::Approve
    );
}

#[test]
fn stop_maps_to_disapprove() {
    assert_eq!(
        stance_for_decision(AdvisorDecision::Stop),
        rubric::StanceKind::Disapprove
    );
}

#[test]
fn retry_maps_to_neither_approve_nor_disapprove() {
    // 要重做 ⇒ 既非赞成也非反对（rubric 现有枚举里对应 Neutral，score 0.0）。
    let stance = stance_for_decision(AdvisorDecision::Retry);
    assert_eq!(stance, rubric::StanceKind::Neutral);
    assert!(!stance.is_disapprove());
    assert_eq!(stance.score(), 0.0);
}

#[test]
fn abstain_maps_to_abstain() {
    assert_eq!(
        stance_for_decision(AdvisorDecision::Abstain),
        rubric::StanceKind::Abstain
    );
}

#[test]
fn all_seven_kinds_map_to_same_named_domain() {
    assert_eq!(
        domain_for_kind(AdvisorKind::Safety),
        rubric::AdvisorDomain::Safety
    );
    assert_eq!(
        domain_for_kind(AdvisorKind::Performance),
        rubric::AdvisorDomain::Performance
    );
    assert_eq!(
        domain_for_kind(AdvisorKind::Philosophy),
        rubric::AdvisorDomain::Philosophy
    );
    assert_eq!(
        domain_for_kind(AdvisorKind::History),
        rubric::AdvisorDomain::History
    );
    assert_eq!(
        domain_for_kind(AdvisorKind::Strategy),
        rubric::AdvisorDomain::Strategy
    );
    assert_eq!(
        domain_for_kind(AdvisorKind::Ethics),
        rubric::AdvisorDomain::Ethics
    );
    assert_eq!(
        domain_for_kind(AdvisorKind::Legal),
        rubric::AdvisorDomain::Legal
    );
    // 一一对应 ⇒ 7 个域互不塌缩。
    let mapped = [
        domain_for_kind(AdvisorKind::Safety),
        domain_for_kind(AdvisorKind::Performance),
        domain_for_kind(AdvisorKind::Philosophy),
        domain_for_kind(AdvisorKind::History),
        domain_for_kind(AdvisorKind::Strategy),
        domain_for_kind(AdvisorKind::Ethics),
        domain_for_kind(AdvisorKind::Legal),
    ];
    assert_eq!(mapped.len(), rubric::AdvisorDomain::ALL.len());
}

#[test]
fn explicit_confidence_is_used_and_marked_as_given() {
    let evaluation = evaluation(
        "advisor-safety",
        AdvisorKind::Safety,
        0.42,
        AdvisorDecision::Allow,
        Some(0.8),
    );
    let ballot = ballot_from_evaluation(&evaluation);
    assert_eq!(ballot.confidence, 0.8);
    assert!(!ballot.confidence_defaulted);
    // 显式置信度 ≠ 顾问自评质量分：取的是 0.8，不是 0.42。
    assert_ne!(ballot.confidence, evaluation.verdict.score);
}

#[test]
fn missing_confidence_falls_back_to_advisor_score_and_is_marked_defaulted() {
    let evaluation = evaluation(
        "advisor-history",
        AdvisorKind::History,
        0.37,
        AdvisorDecision::Stop,
        None,
    );
    let ballot = ballot_from_evaluation(&evaluation);
    assert_eq!(ballot.confidence, 0.37);
    assert!(ballot.confidence_defaulted);
}

#[test]
fn missing_confidence_never_becomes_one() {
    // 1.0 兜底 = 凭空宣称满置信 = 编造；顾问只给了 0.0 自评分就只能是 0.0。
    let evaluation = evaluation(
        "advisor-legal",
        AdvisorKind::Legal,
        0.0,
        AdvisorDecision::Abstain,
        None,
    );
    let ballot = ballot_from_evaluation(&evaluation);
    assert_eq!(ballot.confidence, 0.0);
    assert!(ballot.confidence_defaulted);
}

#[test]
fn weight_uses_registered_domain_default_weight() {
    let safety = ballot_from_evaluation(&evaluation(
        "advisor-safety",
        AdvisorKind::Safety,
        0.5,
        AdvisorDecision::Allow,
        Some(0.5),
    ));
    assert_eq!(
        safety.weight,
        rubric::AdvisorDomain::Safety.default_weight()
    );
    assert_eq!(safety.weight, 1.00);

    let history = ballot_from_evaluation(&evaluation(
        "advisor-history",
        AdvisorKind::History,
        0.5,
        AdvisorDecision::Allow,
        Some(0.5),
    ));
    assert_eq!(
        history.weight,
        rubric::AdvisorDomain::History.default_weight()
    );
    assert_eq!(history.weight, 0.55);
}

#[test]
fn ballot_carries_its_domain() {
    let ballot = ballot_from_evaluation(&evaluation(
        "advisor-philosophy",
        AdvisorKind::Philosophy,
        0.6,
        AdvisorDecision::Allow,
        Some(0.7),
    ));
    assert_eq!(
        ballot.domain,
        Some(rubric::AdvisorDomain::Philosophy),
        "domain 必须写上票"
    );
    assert_eq!(ballot.advisor_id, "advisor-philosophy");
}

#[test]
fn stance_and_kind_slugs_are_snake_case() {
    assert_eq!(stance_slug(AdvisorDecision::Allow), "approve");
    assert_eq!(stance_slug(AdvisorDecision::Stop), "disapprove");
    assert_eq!(stance_slug(AdvisorDecision::Retry), "hold");
    assert_eq!(stance_slug(AdvisorDecision::Abstain), "abstain");

    assert_eq!(kind_slug(AdvisorKind::Safety), "safety");
    assert_eq!(kind_slug(AdvisorKind::Performance), "performance");
    assert_eq!(kind_slug(AdvisorKind::Philosophy), "philosophy");
    assert_eq!(kind_slug(AdvisorKind::History), "history");
    assert_eq!(kind_slug(AdvisorKind::Strategy), "strategy");
    assert_eq!(kind_slug(AdvisorKind::Ethics), "ethics");
    assert_eq!(kind_slug(AdvisorKind::Legal), "legal");
}

#[test]
fn record_mirrors_the_ballot() {
    let evaluation = evaluation(
        "advisor-strategy",
        AdvisorKind::Strategy,
        0.55,
        AdvisorDecision::Retry,
        None,
    );
    let ballot = ballot_from_evaluation(&evaluation);
    let record = record_from_evaluation(&evaluation, 1_700_000_000_123);

    assert_eq!(record.advisor, "advisor-strategy");
    assert_eq!(record.stance, "hold");
    assert_eq!(record.confidence, ballot.confidence);
    assert_eq!(record.confidence_defaulted, ballot.confidence_defaulted);
    assert!(record.confidence_defaulted);
    assert_eq!(record.weight, ballot.weight);
    assert_eq!(
        record.weight,
        rubric::AdvisorDomain::Strategy.default_weight()
    );
    assert_eq!(record.domain, Some("strategy".to_string()));
    assert_eq!(record.advisor_score, 0.55);
    assert_eq!(record.at_ms, 1_700_000_000_123);
}

#[test]
fn record_keeps_explicit_confidence_and_score_separately() {
    let evaluation = evaluation(
        "advisor-ethics",
        AdvisorKind::Ethics,
        0.9,
        AdvisorDecision::Allow,
        Some(0.3),
    );
    let record = record_from_evaluation(&evaluation, 42);
    assert_eq!(record.confidence, 0.3);
    assert!(!record.confidence_defaulted);
    assert_eq!(record.advisor_score, 0.9);
    assert_eq!(record.stance, "approve");
    assert_eq!(record.domain, Some("ethics".to_string()));
}

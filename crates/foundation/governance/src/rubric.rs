//! Council scoring / hold rubric recovered as a **library helper**.
//!
//! v2 `apeireth-orchestration::Council` remains the only Council owner. This
//! module does not run advisors, does not own a loop, and is not wired into
//! the canonical pipeline. It scores already-collected stances:
//!
//! * weighted synthesis (`score × confidence × weight`, clamped to [-1, 1])
//! * stance mapping at ±0.6 / ±0.2
//! * hold triggers: ≥30% strong-disapprove, unanimous non-abstain disapprove,
//!   optional 60s timeout
//! * voting strategies: weighted majority / top scoring / supermajority (2/3)
//!
//! Default advisor-domain weights match the donor Safety=1.00 … History=0.55
//! table.

use serde::{Deserialize, Serialize};

/// Strong-disapprove share that triggers a hold (integer percent).
pub const HOLD_STRONG_DISAPPROVE_PERCENT: u8 = 30;

/// Deliberation timeout used by the optional timeout helper (60s).
pub const HOLD_DELIBERATION_TIMEOUT_MS: u64 = 60_000;

/// Supermajority fraction (2/3 of non-empty ballots).
pub const SUPERMAJORITY_FRACTION: f64 = 2.0 / 3.0;

/// Stance used by the donor synthesis table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StanceKind {
    StrongApprove,
    Approve,
    Neutral,
    Disapprove,
    StrongDisapprove,
    Abstain,
}

impl StanceKind {
    pub const fn score(self) -> f64 {
        match self {
            Self::StrongApprove => 1.00,
            Self::Approve => 0.60,
            Self::Neutral => 0.00,
            Self::Disapprove => -0.60,
            Self::StrongDisapprove => -1.00,
            Self::Abstain => 0.00,
        }
    }

    pub const fn is_strong_disapprove(self) -> bool {
        matches!(self, Self::StrongDisapprove)
    }

    pub const fn is_abstain(self) -> bool {
        matches!(self, Self::Abstain)
    }

    pub const fn is_disapprove(self) -> bool {
        matches!(self, Self::Disapprove | Self::StrongDisapprove)
    }

    /// Map a normalized weighted score in [-1, 1] onto a stance.
    pub fn from_weighted_score(score: f64) -> Self {
        if score >= 0.6 {
            Self::StrongApprove
        } else if score >= 0.2 {
            Self::Approve
        } else if score >= -0.2 {
            Self::Neutral
        } else if score >= -0.6 {
            Self::Disapprove
        } else {
            Self::StrongDisapprove
        }
    }
}

/// Seven donor advisor domains. Weights are used only when a ballot leaves
/// `weight` at 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdvisorDomain {
    Safety,
    Performance,
    Philosophy,
    History,
    Strategy,
    Ethics,
    Legal,
}

impl AdvisorDomain {
    pub const ALL: [AdvisorDomain; 7] = [
        Self::Safety,
        Self::Performance,
        Self::Philosophy,
        Self::History,
        Self::Strategy,
        Self::Ethics,
        Self::Legal,
    ];

    pub const fn default_weight(self) -> f64 {
        match self {
            Self::Safety => 1.00,
            Self::Philosophy => 0.95,
            Self::Ethics => 0.90,
            Self::Legal => 0.85,
            Self::Strategy => 0.75,
            Self::Performance => 0.65,
            Self::History => 0.55,
        }
    }
}

/// One already-collected ballot. No LLM, no I/O.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ballot {
    pub advisor_id: String,
    pub stance: StanceKind,
    pub confidence: f64,
    pub weight: f64,
    pub domain: Option<AdvisorDomain>,
}

impl Ballot {
    pub fn new(
        advisor_id: impl Into<String>,
        stance: StanceKind,
        confidence: f64,
        weight: f64,
    ) -> Self {
        Self {
            advisor_id: advisor_id.into(),
            stance,
            confidence: confidence.clamp(0.0, 1.0),
            weight,
            domain: None,
        }
    }

    fn effective_weight(&self) -> f64 {
        if self.weight > 0.0 {
            self.weight
        } else if let Some(domain) = self.domain {
            domain.default_weight()
        } else {
            1.0
        }
    }
}

/// Weighted synthesis of non-abstaining ballots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisReport {
    pub weighted_score: f64,
    pub aggregated_stance: StanceKind,
    pub confidence: f64,
    pub dissenting: Vec<String>,
    pub hold: HoldDecision,
    pub opinion_count: usize,
}

/// Synthesize ballots. Abstentions are dropped from the score; hold is
/// evaluated against the original slice (including abstentions, matching the
/// donor: strong-disapprove percent uses total count).
pub fn synthesize(ballots: &[Ballot]) -> SynthesisReport {
    let mut weighted = Vec::new();
    let mut sum_weighted_score = 0.0;
    let mut sum_weight = 0.0;

    for ballot in ballots {
        if ballot.stance.is_abstain() {
            continue;
        }
        let weight = ballot.effective_weight();
        sum_weighted_score += ballot.stance.score() * ballot.confidence * weight;
        sum_weight += weight;
        weighted.push(ballot);
    }

    let opinion_count = weighted.len();
    let weighted_score = if sum_weight > 0.0 {
        (sum_weighted_score / sum_weight).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let aggregated_stance = StanceKind::from_weighted_score(weighted_score);
    let confidence = if opinion_count > 0 {
        weighted.iter().map(|ballot| ballot.confidence).sum::<f64>() / opinion_count as f64
    } else {
        0.0
    };
    let dissenting = weighted
        .iter()
        .filter(|ballot| opposite_to(ballot.stance, aggregated_stance))
        .map(|ballot| ballot.advisor_id.clone())
        .collect();
    let hold = match HoldTrigger::evaluate(ballots) {
        Some(trigger) => HoldDecision::held(trigger),
        None => HoldDecision::released(),
    };

    SynthesisReport {
        weighted_score,
        aggregated_stance,
        confidence,
        dissenting,
        hold,
        opinion_count,
    }
}

fn opposite_to(a: StanceKind, b: StanceKind) -> bool {
    let a_sign = a.score().signum() as i32;
    let b_sign = b.score().signum() as i32;
    a_sign != 0 && b_sign != 0 && a_sign != b_sign
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoldThreshold {
    StrongDisapprovePercent { actual_percent: u8, threshold: u8 },
    UnanimousDisapprove { opposing_count: usize },
    DeliberationTimeout { actual_ms: u64, threshold_ms: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldTrigger {
    pub threshold: HoldThreshold,
    pub dissenting_opinions: Vec<String>,
}

impl HoldTrigger {
    pub fn evaluate(ballots: &[Ballot]) -> Option<Self> {
        let total = ballots.len();
        if total == 0 {
            return None;
        }
        let non_abstain: Vec<&Ballot> = ballots
            .iter()
            .filter(|ballot| !ballot.stance.is_abstain())
            .collect();
        if non_abstain.is_empty() {
            return None;
        }
        let strong: Vec<&Ballot> = non_abstain
            .iter()
            .copied()
            .filter(|ballot| ballot.stance.is_strong_disapprove())
            .collect();
        let disapprove: Vec<&Ballot> = non_abstain
            .iter()
            .copied()
            .filter(|ballot| ballot.stance.is_disapprove())
            .collect();

        let strong_pct = ((strong.len() * 100) / total) as u8;
        if strong_pct >= HOLD_STRONG_DISAPPROVE_PERCENT {
            return Some(Self {
                threshold: HoldThreshold::StrongDisapprovePercent {
                    actual_percent: strong_pct,
                    threshold: HOLD_STRONG_DISAPPROVE_PERCENT,
                },
                dissenting_opinions: strong
                    .iter()
                    .map(|ballot| ballot.advisor_id.clone())
                    .collect(),
            });
        }
        if disapprove.len() == non_abstain.len() {
            return Some(Self {
                threshold: HoldThreshold::UnanimousDisapprove {
                    opposing_count: disapprove.len(),
                },
                dissenting_opinions: disapprove
                    .iter()
                    .map(|ballot| ballot.advisor_id.clone())
                    .collect(),
            });
        }
        None
    }

    pub fn evaluate_timeout(actual_ms: u64) -> Option<Self> {
        if actual_ms >= HOLD_DELIBERATION_TIMEOUT_MS {
            Some(Self {
                threshold: HoldThreshold::DeliberationTimeout {
                    actual_ms,
                    threshold_ms: HOLD_DELIBERATION_TIMEOUT_MS,
                },
                dissenting_opinions: Vec::new(),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldDecision {
    pub held: bool,
    pub trigger: Option<HoldTrigger>,
}

impl HoldDecision {
    pub fn released() -> Self {
        Self {
            held: false,
            trigger: None,
        }
    }

    pub fn held(trigger: HoldTrigger) -> Self {
        Self {
            held: true,
            trigger: Some(trigger),
        }
    }

    pub fn is_held(&self) -> bool {
        self.held
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VotingStrategy {
    WeightedMajority,
    TopScoring,
    Supermajority,
}

/// Evaluate a voting strategy against already-synthesized ballots.
pub fn passes_strategy(
    strategy: VotingStrategy,
    ballots: &[Ballot],
    report: &SynthesisReport,
) -> bool {
    match strategy {
        VotingStrategy::WeightedMajority => report.weighted_score > 0.0,
        VotingStrategy::TopScoring => ballots
            .iter()
            .filter(|ballot| !ballot.stance.is_abstain())
            .max_by(|a, b| {
                a.stance
                    .score()
                    .partial_cmp(&b.stance.score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|ballot| ballot.stance.score() > 0.0)
            .unwrap_or(false),
        VotingStrategy::Supermajority => {
            if ballots.is_empty() {
                return false;
            }
            let approve = ballots
                .iter()
                .filter(|ballot| ballot.stance.score() > 0.0)
                .count();
            (approve as f64) >= (ballots.len() as f64) * SUPERMAJORITY_FRACTION
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ballot(id: &str, stance: StanceKind, confidence: f64) -> Ballot {
        Ballot::new(id, stance, confidence, 1.0)
    }

    #[test]
    fn synthesis_maps_unanimous_approve() {
        let ballots = vec![
            ballot("a", StanceKind::Approve, 0.8),
            ballot("b", StanceKind::StrongApprove, 0.9),
        ];
        let report = synthesize(&ballots);
        assert!(report.weighted_score > 0.5);
        assert!(matches!(
            report.aggregated_stance,
            StanceKind::Approve | StanceKind::StrongApprove
        ));
        assert!(!report.hold.is_held());
    }

    #[test]
    fn abstain_is_dropped_from_score() {
        let ballots = vec![
            ballot("a", StanceKind::Approve, 1.0),
            ballot("b", StanceKind::Abstain, 1.0),
        ];
        let report = synthesize(&ballots);
        assert_eq!(report.opinion_count, 1);
        assert!((report.weighted_score - 0.60).abs() < 1e-9);
    }

    #[test]
    fn hold_triggers_on_thirty_percent_strong_disapprove() {
        let ballots = vec![
            ballot("s", StanceKind::StrongDisapprove, 1.0),
            ballot("a", StanceKind::Approve, 1.0),
            ballot("b", StanceKind::Approve, 1.0),
        ];
        // 1/3 ≈ 33% ≥ 30
        let trigger = HoldTrigger::evaluate(&ballots).unwrap();
        assert!(matches!(
            trigger.threshold,
            HoldThreshold::StrongDisapprovePercent {
                actual_percent: 33,
                ..
            }
        ));
    }

    #[test]
    fn hold_triggers_on_unanimous_disapprove() {
        let ballots = vec![
            ballot("a", StanceKind::Disapprove, 1.0),
            ballot("b", StanceKind::Disapprove, 1.0),
        ];
        assert!(matches!(
            HoldTrigger::evaluate(&ballots).unwrap().threshold,
            HoldThreshold::UnanimousDisapprove { opposing_count: 2 }
        ));
    }

    #[test]
    fn hold_timeout() {
        assert!(HoldTrigger::evaluate_timeout(60_000).is_some());
        assert!(HoldTrigger::evaluate_timeout(59_999).is_none());
    }

    #[test]
    fn default_domain_weights_match_donor() {
        assert_eq!(AdvisorDomain::Safety.default_weight(), 1.00);
        assert_eq!(AdvisorDomain::History.default_weight(), 0.55);
        assert_eq!(AdvisorDomain::ALL.len(), 7);
    }

    #[test]
    fn supermajority_requires_two_thirds() {
        let ballots = vec![
            ballot("a", StanceKind::Approve, 1.0),
            ballot("b", StanceKind::Approve, 1.0),
            ballot("c", StanceKind::Disapprove, 1.0),
        ];
        let report = synthesize(&ballots);
        assert!(passes_strategy(
            VotingStrategy::Supermajority,
            &ballots,
            &report
        ));
        let mixed = vec![
            ballot("a", StanceKind::Approve, 1.0),
            ballot("b", StanceKind::Disapprove, 1.0),
            ballot("c", StanceKind::Disapprove, 1.0),
        ];
        let mixed_report = synthesize(&mixed);
        assert!(!passes_strategy(
            VotingStrategy::Supermajority,
            &mixed,
            &mixed_report
        ));
    }

    #[test]
    fn weighted_majority_uses_score_sign() {
        let ballots = vec![ballot("a", StanceKind::Approve, 1.0)];
        let report = synthesize(&ballots);
        assert!(passes_strategy(
            VotingStrategy::WeightedMajority,
            &ballots,
            &report
        ));
    }
}

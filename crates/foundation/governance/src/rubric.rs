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
//! Default advisor-domain weights match the baseline Safety=1.00 … History=0.55
//! table.

use serde::{Deserialize, Serialize};

/// Strong-disapprove share that triggers a hold (integer percent).
pub const HOLD_STRONG_DISAPPROVE_PERCENT: u8 = 30;

/// Deliberation timeout used by the optional timeout helper (60s).
pub const HOLD_DELIBERATION_TIMEOUT_MS: u64 = 60_000;

/// Supermajority fraction (2/3 of non-empty ballots).
pub const SUPERMAJORITY_FRACTION: f64 = 2.0 / 3.0;

/// Stance used by the baseline synthesis table.
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

/// Seven baseline advisor domains. Weights are used only when a ballot leaves
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
    /// D-2：置信度是否为「兜底值」而非顾问显式给出。
    ///
    /// `true` = 顾问没给置信度，这个值是映射层兜底/从质量分折算来的
    /// （消费者据此如实标注「此置信度非显式」）；`false` = 顾问显式给出。
    ///
    /// `#[serde(default)]` ⇒ 旧 JSON（没有这个键）读出来是 `false`，
    /// 与「显式给出」的旧语义保持一致。
    #[serde(default)]
    pub confidence_defaulted: bool,
    pub weight: f64,
    pub domain: Option<AdvisorDomain>,
}

/// Clamp a confidence-like value into `0.0..=1.0`.
///
/// `f64::clamp` keeps NaN as NaN (NaN 比较恒为 false)，这里显式把 NaN 挡掉
/// ⇒ 落到 `0.0`，避免 NaN 置信度污染下游加权分。
fn clamp_confidence(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
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
            confidence_defaulted: false,
            weight,
            domain: None,
        }
    }

    /// D-2 映射口径：从「顾问给的一条评价」折算成一张票。
    ///
    /// * `confidence`：`Some(c)` = 顾问**显式**给的置信度（原样取用，clamp 0..=1）；
    ///   `None` = 顾问没给 ⇒ 用 `fallback_confidence`（映射层折算出的质量分），
    ///   并把 `confidence_defaulted` 置 `true`。
    /// * `weight`：调用方给的权重（映射层会传「现有顾问注册权重」）。
    /// * `domain`：记下这张票来自哪个顾问域（`domain` 字段同时可用于
    ///   `effective_weight()` 的兜底）。
    pub fn from_advice(
        advisor_id: impl Into<String>,
        stance: StanceKind,
        confidence: Option<f64>,
        fallback_confidence: f64,
        weight: f64,
        domain: AdvisorDomain,
    ) -> Self {
        let confidence_defaulted = confidence.is_none();
        let confidence = match confidence {
            Some(explicit) => clamp_confidence(explicit),
            None => clamp_confidence(fallback_confidence),
        };
        Self {
            advisor_id: advisor_id.into(),
            stance,
            confidence,
            confidence_defaulted,
            weight,
            domain: Some(domain),
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
/// baseline: strong-disapprove percent uses total count).
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
    fn default_domain_weights_match_baseline() {
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

    // ---- D-2: council ballot 接线（from_advice 映射口径） ----

    #[test]
    fn from_advice_keeps_explicit_confidence_and_is_not_defaulted() {
        let b = Ballot::from_advice(
            "advisor-safety",
            StanceKind::StrongApprove,
            Some(0.82),
            0.10,
            1.0,
            AdvisorDomain::Safety,
        );
        assert!(!b.confidence_defaulted, "显式给的置信度不能标成兜底值");
        assert!((b.confidence - 0.82).abs() < 1e-9, "显式置信度原样取用");
        assert_eq!(b.advisor_id, "advisor-safety");
        assert_eq!(b.stance, StanceKind::StrongApprove);
    }

    #[test]
    fn from_advice_falls_back_and_flags_defaulted() {
        let b = Ballot::from_advice(
            "advisor-history",
            StanceKind::Approve,
            None,
            0.42,
            0.55,
            AdvisorDomain::History,
        );
        assert!(b.confidence_defaulted, "缺省置信度必须标成兜底值");
        assert!(
            (b.confidence - 0.42).abs() < 1e-9,
            "缺省时取 fallback_confidence"
        );
    }

    #[test]
    fn from_advice_clamps_out_of_range_and_nan() {
        let low = Ballot::from_advice(
            "a",
            StanceKind::Approve,
            Some(-0.5),
            0.5,
            1.0,
            AdvisorDomain::Safety,
        );
        assert!(
            (low.confidence - 0.0).abs() < 1e-9,
            "显式 -0.5 应 clamp 到 0.0"
        );

        let high = Ballot::from_advice(
            "b",
            StanceKind::Approve,
            Some(1.7),
            0.5,
            1.0,
            AdvisorDomain::Safety,
        );
        assert!(
            (high.confidence - 1.0).abs() < 1e-9,
            "显式 1.7 应 clamp 到 1.0"
        );

        let nan = Ballot::from_advice(
            "c",
            StanceKind::Approve,
            Some(f64::NAN),
            0.5,
            1.0,
            AdvisorDomain::Safety,
        );
        assert!((nan.confidence - 0.0).abs() < 1e-9, "NaN 置信度应落到 0.0");

        // fallback 同样要 clamp / 挡 NaN
        let low_fallback = Ballot::from_advice(
            "d",
            StanceKind::Approve,
            None,
            -0.5,
            1.0,
            AdvisorDomain::Safety,
        );
        assert!((low_fallback.confidence - 0.0).abs() < 1e-9);
        assert!(low_fallback.confidence_defaulted);

        let high_fallback = Ballot::from_advice(
            "e",
            StanceKind::Approve,
            None,
            1.7,
            1.0,
            AdvisorDomain::Safety,
        );
        assert!((high_fallback.confidence - 1.0).abs() < 1e-9);

        let nan_fallback = Ballot::from_advice(
            "f",
            StanceKind::Approve,
            None,
            f64::NAN,
            1.0,
            AdvisorDomain::Safety,
        );
        assert!(
            (nan_fallback.confidence - 0.0).abs() < 1e-9,
            "NaN fallback 应落到 0.0"
        );
        assert!(nan_fallback.confidence_defaulted);
    }

    #[test]
    fn from_advice_records_domain_and_effective_weight_uses_domain_default() {
        let b = Ballot::from_advice(
            "advisor-history",
            StanceKind::Approve,
            Some(0.5),
            0.5,
            0.0,
            AdvisorDomain::History,
        );
        assert_eq!(b.domain, Some(AdvisorDomain::History), "domain 必须被记下");
        assert!(
            (b.effective_weight() - AdvisorDomain::History.default_weight()).abs() < 1e-9,
            "weight = 0 时 effective_weight 应回落到域默认权重"
        );

        let explicit = Ballot::from_advice(
            "advisor-safety",
            StanceKind::Approve,
            Some(0.5),
            0.5,
            2.0,
            AdvisorDomain::Safety,
        );
        assert!(
            (explicit.effective_weight() - 2.0).abs() < 1e-9,
            "weight > 0 时 effective_weight 用调用方给的权重"
        );
    }

    #[test]
    fn new_keeps_explicit_confidence_and_is_not_defaulted() {
        let b = Ballot::new("advisor-safety", StanceKind::Approve, 0.7, 1.0);
        assert!(!b.confidence_defaulted, "new 的置信度是调用方显式给的");
        assert!((b.confidence - 0.7).abs() < 1e-9);
        assert_eq!(b.domain, None);
    }

    #[test]
    fn weighted_score_denominator_excludes_confidence() {
        // 现有口径：weighted_score = Σ(stance.score() × confidence × weight) / Σ(weight)
        // ⇒ **分母不含 confidence**。
        // 这是**当前**口径；改口径属产品决策（D-8），不要顺手改。
        let ballots = vec![
            Ballot::new("zero", StanceKind::StrongApprove, 0.0, 1.0),
            Ballot::new("one", StanceKind::StrongApprove, 1.0, 1.0),
        ];
        let report = synthesize(&ballots);
        // 分子 = 1.00×0 + 1.00×1 = 1.00；分母 = 1 + 1 = 2 ⇒ 0.5
        assert!(
            (report.weighted_score - 0.5).abs() < 1e-9,
            "分母不含 confidence ⇒ 一张 confidence=0 + 一张 confidence=1 的赞成票得到 0.5，而不是 1.0"
        );
        // 等权同立场：分子 = 0.60×0.25 + 0.60×0.75 = 0.60；分母 = 2 ⇒ 0.30
        // （= stance.score() × 平均 confidence 0.50；若分母含 confidence 则会是 0.60）
        let equal = vec![
            Ballot::new("a", StanceKind::Approve, 0.25, 1.0),
            Ballot::new("b", StanceKind::Approve, 0.75, 1.0),
        ];
        let equal_report = synthesize(&equal);
        assert!(
            (equal_report.weighted_score - 0.30).abs() < 1e-9,
            "分母是纯 Σ(weight)，不是 Σ(weight × confidence)"
        );

        // 不等权：分子 = 0.60×1.00×3 + 0.60×0.00×1 = 1.80；分母 = 3 + 1 = 4 ⇒ 0.45
        // （若分母含 confidence 则是 1.80/3 = 0.60）
        let uneven = vec![
            Ballot::new("heavy", StanceKind::Approve, 1.0, 3.0),
            Ballot::new("light", StanceKind::Approve, 0.0, 1.0),
        ];
        let uneven_report = synthesize(&uneven);
        assert!(
            (uneven_report.weighted_score - 0.45).abs() < 1e-9,
            "分母不含 confidence：低置信度票仍然全额占权重"
        );
    }
}

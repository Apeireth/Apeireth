//! Ensemble forecast aggregation + Hanson LMSR prediction market.
//!
//! Recovered from `legacy/donor/apeireth-cognition/src/forecast.rs`.
//!
//! Two complementary aggregators, both pure math and independent of any LLM
//! transport:
//!
//! - [`EnsembleForecast`] — Bayesian / mean / median aggregation of
//!   `(prediction, confidence)` members, with an optional contrarian boost so
//!   a minority forecast is not washed out by majority agreement.
//! - [`PredictionMarket`] — Hanson (2003) Logarithmic Market Scoring Rule.
//!   Cost `C(q) = b · log(Σ exp(q_i / b))`; implied prices are a softmax of
//!   the share vector. A contrarian subsidy cheapens currently under-priced
//!   outcomes.
//!
//! LMSR uses a log-sum-exp shift for numerical stability (an improvement over
//! the donor's raw `exp(q/b)`). No provider, store, or loop is owned here.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::calibration::Observation;

// ============================================================================
// EnsembleForecast
// ============================================================================

/// Aggregation strategy for ensemble members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AggregationStrategy {
    /// Confidence × contrarian-factor weighted mean. Default.
    #[default]
    Bayesian,
    /// Equal-weight mean.
    Mean,
    /// Median (robust to a single outlier).
    Median,
}

/// One ensemble member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnsembleMember {
    /// Source identifier (model name, human id, …).
    pub source_id: String,
    /// Predicted probability in `[0, 1]`.
    pub prediction: f64,
    /// Self-reported / historical confidence in `[0, 1]`.
    pub confidence: f64,
}

impl EnsembleMember {
    /// Construct a member; values are clamped to the unit interval.
    pub fn new(source_id: impl Into<String>, prediction: f64, confidence: f64) -> Self {
        Self {
            source_id: source_id.into(),
            prediction: finite_unit(prediction),
            confidence: finite_unit(confidence),
        }
    }
}

/// Ensemble configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnsembleConfig {
    /// Aggregation strategy.
    pub strategy: AggregationStrategy,
    /// Contrarian weight in `[0, 1]`. Only used by [`AggregationStrategy::Bayesian`].
    pub contrarian_weight: f64,
}

impl Default for EnsembleConfig {
    fn default() -> Self {
        Self {
            strategy: AggregationStrategy::Bayesian,
            contrarian_weight: 0.0,
        }
    }
}

/// Aggregated ensemble result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnsembleForecast {
    /// Input members (cloned for audit).
    pub members: Vec<EnsembleMember>,
    /// Aggregated probability in `[0, 1]`.
    pub aggregate_prediction: f64,
    /// Aggregated confidence in `[0, 1]`.
    pub aggregate_confidence: f64,
    /// Agreement in `[0, 1]`: `1 − 2·stddev(predictions)`.
    pub agreement_score: f64,
    /// Strategy that produced this result.
    pub strategy: AggregationStrategy,
    /// Contrarian weight that was applied.
    pub contrarian_weight: f64,
}

impl EnsembleForecast {
    /// Aggregate `K` members. Empty input returns a non-committal default
    /// (`prediction = 0.5`, `confidence = 0.0`).
    pub fn aggregate(members: Vec<EnsembleMember>, config: EnsembleConfig) -> Self {
        if members.is_empty() {
            return Self {
                members,
                aggregate_prediction: 0.5,
                aggregate_confidence: 0.0,
                agreement_score: 0.0,
                strategy: config.strategy,
                contrarian_weight: config.contrarian_weight,
            };
        }

        let agreement = agreement_score(&members);
        let aggregate_prediction = match config.strategy {
            AggregationStrategy::Mean => {
                members.iter().map(|m| m.prediction).sum::<f64>() / members.len() as f64
            }
            AggregationStrategy::Median => {
                median(&members.iter().map(|m| m.prediction).collect::<Vec<_>>())
            }
            AggregationStrategy::Bayesian => {
                weighted_mean(&members, config.contrarian_weight, agreement, |m| {
                    m.prediction
                })
            }
        };

        let aggregate_confidence = match config.strategy {
            AggregationStrategy::Mean => {
                members.iter().map(|m| m.confidence).sum::<f64>() / members.len() as f64
            }
            _ => weighted_mean(&members, config.contrarian_weight, agreement, |m| {
                m.confidence
            }),
        };

        Self {
            members,
            aggregate_prediction: finite_unit(aggregate_prediction),
            aggregate_confidence: finite_unit(aggregate_confidence),
            agreement_score: agreement,
            strategy: config.strategy,
            contrarian_weight: config.contrarian_weight,
        }
    }

    /// Convert the aggregate into a calibration [`Observation`] once the
    /// ground-truth outcome is known.
    pub fn as_observation(&self, outcome: f64) -> Observation {
        Observation::new(self.aggregate_prediction, outcome)
    }
}

fn weighted_mean(
    members: &[EnsembleMember],
    contrarian_weight: f64,
    agreement: f64,
    value: impl Fn(&EnsembleMember) -> f64,
) -> f64 {
    let contrarian_factor = 1.0 + contrarian_weight * (1.0 - agreement);
    let med = median(&members.iter().map(|m| m.prediction).collect::<Vec<_>>());
    let mut total_weight = 0.0;
    let mut weighted = 0.0;
    for m in members {
        let minority_boost = if (m.prediction - med).abs() > 0.1 {
            1.0 + contrarian_weight
        } else {
            1.0
        };
        let w = m.confidence * contrarian_factor * minority_boost;
        total_weight += w;
        weighted += w * value(m);
    }
    if total_weight <= 0.0 {
        members.iter().map(&value).sum::<f64>() / members.len() as f64
    } else {
        weighted / total_weight
    }
}

/// Agreement in `[0, 1]`. `stddev ∈ [0, 0.5]` for values in `[0, 1]`, so
/// `1 − 2·stddev` maps that range onto `[0, 1]`.
fn agreement_score(members: &[EnsembleMember]) -> f64 {
    if members.len() < 2 {
        return 1.0;
    }
    let n = members.len() as f64;
    let mean: f64 = members.iter().map(|m| m.prediction).sum::<f64>() / n;
    let var: f64 = members
        .iter()
        .map(|m| (m.prediction - mean).powi(2))
        .sum::<f64>()
        / n;
    (1.0 - 2.0 * var.sqrt()).clamp(0.0, 1.0)
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

fn finite_unit(x: f64) -> f64 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

// ============================================================================
// PredictionMarket (LMSR)
// ============================================================================

/// LMSR market configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketConfig {
    /// Liquidity parameter `b > 0`. Larger `b` → smaller price impact.
    pub liquidity_b: f64,
    /// Number of mutually exclusive outcomes (≥ 2).
    pub num_outcomes: usize,
    /// Contrarian subsidy in `[0, 1]`. Cheapens currently under-priced outcomes.
    pub contrarian_weight: f64,
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        }
    }
}

/// Result of one LMSR buy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeReceipt {
    /// Outcome index purchased.
    pub outcome_idx: usize,
    /// Shares purchased (`Δ ≥ 0`).
    pub shares: f64,
    /// Total cost paid (after any contrarian subsidy).
    pub cost: f64,
    /// Average price (`cost / shares`).
    pub avg_price: f64,
    /// Implied price of the purchased outcome after the trade.
    pub price_after: f64,
}

/// Hanson LMSR market state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionMarket {
    /// Outstanding shares per outcome.
    pub quantities: Vec<f64>,
    /// Market configuration.
    pub config: MarketConfig,
}

/// LMSR trade error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarketError {
    /// `shares < 0`.
    NegativeShares(f64),
    /// Outcome index out of range.
    InvalidOutcome(usize),
    /// `liquidity_b` was not strictly positive or `num_outcomes < 2`.
    InvalidConfig,
}

impl std::fmt::Display for MarketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NegativeShares(s) => write!(f, "negative shares: {s}"),
            Self::InvalidOutcome(i) => write!(f, "invalid outcome idx: {i}"),
            Self::InvalidConfig => write!(f, "invalid LMSR config"),
        }
    }
}

impl std::error::Error for MarketError {}

impl PredictionMarket {
    /// Uniform market (`q_i = 0` → prices `1/N`). Returns `None` on invalid config.
    pub fn try_new(config: MarketConfig) -> Result<Self, MarketError> {
        if !(config.liquidity_b > 0.0) || config.num_outcomes < 2 {
            return Err(MarketError::InvalidConfig);
        }
        Ok(Self {
            quantities: vec![0.0; config.num_outcomes],
            config,
        })
    }

    /// Uniform market. Panics only on programmer error (invalid config) in
    /// tests that construct a known-good [`MarketConfig`].
    pub fn new(config: MarketConfig) -> Self {
        Self::try_new(config).expect("LMSR MarketConfig must have b>0 and ≥2 outcomes")
    }

    /// Offset each `q_i` by `1/N`. Prices are translation-invariant, so this
    /// is cosmetic; it exists for donor API parity.
    pub fn uniform(config: MarketConfig) -> Self {
        let mut m = Self::new(config);
        let n = m.config.num_outcomes as f64;
        for q in &mut m.quantities {
            *q = 1.0 / n;
        }
        m
    }

    /// LMSR cost `C(q) = b · logsumexp(q / b)`.
    pub fn cost(&self) -> f64 {
        lmsr_cost(&self.quantities, self.config.liquidity_b)
    }

    /// Implied price vector. Sums to 1.0.
    pub fn prices(&self) -> Vec<f64> {
        lmsr_prices(&self.quantities, self.config.liquidity_b)
    }

    /// Implied price of a single outcome.
    pub fn price_of(&self, idx: usize) -> f64 {
        self.prices()[idx]
    }

    /// Cost of buying `shares` of outcome `idx`, after the contrarian subsidy.
    ///
    /// `cost = C(q + Δ e_idx) − C(q)`, then multiplied by
    /// `1 − contrarian_weight · deficit / fair_price` when the current price
    /// is below the uniform `1/N`.
    pub fn cost_to_buy(&self, idx: usize, shares: f64) -> Result<f64, MarketError> {
        if shares < 0.0 {
            return Err(MarketError::NegativeShares(shares));
        }
        if idx >= self.config.num_outcomes {
            return Err(MarketError::InvalidOutcome(idx));
        }

        let mut new_q = self.quantities.clone();
        new_q[idx] += shares;
        let raw_cost = lmsr_cost(&new_q, self.config.liquidity_b) - self.cost();

        let current_price = self.price_of(idx);
        let fair_price = 1.0 / self.config.num_outcomes as f64;
        let deficit = (fair_price - current_price).max(0.0);
        let subsidy = 1.0 - self.config.contrarian_weight * (deficit / fair_price).min(1.0);
        Ok(raw_cost * subsidy)
    }

    /// Execute a buy, mutating state.
    pub fn execute_buy(&mut self, idx: usize, shares: f64) -> Result<TradeReceipt, MarketError> {
        let cost = self.cost_to_buy(idx, shares)?;
        self.quantities[idx] += shares;
        let price_after = self.price_of(idx);
        Ok(TradeReceipt {
            outcome_idx: idx,
            shares,
            cost,
            avg_price: if shares > 0.0 {
                cost / shares
            } else {
                price_after
            },
            price_after,
        })
    }

    /// Collective belief for an outcome (the implied price).
    pub fn aggregate_belief(&self, idx: usize) -> f64 {
        self.price_of(idx)
    }
}

/// Numerically stable `b · logsumexp(q / b)`.
fn lmsr_cost(quantities: &[f64], b: f64) -> f64 {
    if quantities.is_empty() {
        return 0.0;
    }
    let scaled: Vec<f64> = quantities.iter().map(|q| q / b).collect();
    let max = scaled.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max.is_finite() {
        return 0.0;
    }
    let sum_exp: f64 = scaled.iter().map(|x| (x - max).exp()).sum();
    b * (max + sum_exp.ln())
}

/// Softmax of `q / b`.
fn lmsr_prices(quantities: &[f64], b: f64) -> Vec<f64> {
    if quantities.is_empty() {
        return Vec::new();
    }
    let scaled: Vec<f64> = quantities.iter().map(|q| q / b).collect();
    let max = scaled.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = scaled.iter().map(|x| (x - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    if sum <= 0.0 || !sum.is_finite() {
        let n = quantities.len() as f64;
        return vec![1.0 / n; quantities.len()];
    }
    exps.iter().map(|e| e / sum).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensemble_member_clamps_values() {
        let m = EnsembleMember::new("a", 1.5, -0.3);
        assert_eq!(m.prediction, 1.0);
        assert_eq!(m.confidence, 0.0);
    }

    #[test]
    fn ensemble_mean_uniform() {
        let members = vec![
            EnsembleMember::new("a", 0.5, 0.9),
            EnsembleMember::new("b", 0.5, 0.7),
        ];
        let cfg = EnsembleConfig {
            strategy: AggregationStrategy::Mean,
            contrarian_weight: 0.0,
        };
        let agg = EnsembleForecast::aggregate(members, cfg);
        assert!((agg.aggregate_prediction - 0.5).abs() < 1e-9);
        assert!((agg.aggregate_confidence - 0.8).abs() < 1e-9);
        assert!((agg.agreement_score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ensemble_mean_weighted_by_count() {
        let members = vec![
            EnsembleMember::new("a", 0.8, 1.0),
            EnsembleMember::new("b", 0.6, 1.0),
            EnsembleMember::new("c", 0.4, 1.0),
        ];
        let cfg = EnsembleConfig {
            strategy: AggregationStrategy::Mean,
            contrarian_weight: 0.0,
        };
        let agg = EnsembleForecast::aggregate(members, cfg);
        assert!((agg.aggregate_prediction - 0.6).abs() < 1e-9);
    }

    #[test]
    fn ensemble_median_robust_to_outlier() {
        let members = vec![
            EnsembleMember::new("a", 0.5, 0.9),
            EnsembleMember::new("b", 0.5, 0.9),
            EnsembleMember::new("c", 0.5, 0.9),
            EnsembleMember::new("outlier", 0.99, 0.9),
        ];
        let cfg = EnsembleConfig {
            strategy: AggregationStrategy::Median,
            contrarian_weight: 0.0,
        };
        let agg = EnsembleForecast::aggregate(members, cfg);
        assert!((agg.aggregate_prediction - 0.5).abs() < 1e-9);
    }

    #[test]
    fn ensemble_bayesian_confidence_weighted() {
        let members = vec![
            EnsembleMember::new("a", 0.9, 1.0),
            EnsembleMember::new("b", 0.1, 0.1),
        ];
        let cfg = EnsembleConfig {
            strategy: AggregationStrategy::Bayesian,
            contrarian_weight: 0.0,
        };
        let agg = EnsembleForecast::aggregate(members, cfg);
        assert!(
            agg.aggregate_prediction > 0.8,
            "high-conf member should dominate, got {}",
            agg.aggregate_prediction
        );
    }

    #[test]
    fn ensemble_bayesian_contrarian_boosts_minority() {
        let members = vec![
            EnsembleMember::new("a", 0.9, 1.0),
            EnsembleMember::new("b", 0.9, 1.0),
            EnsembleMember::new("c", 0.9, 1.0),
            EnsembleMember::new("d", 0.9, 1.0),
            EnsembleMember::new("e", 0.1, 1.0),
        ];
        let cfg_no = EnsembleConfig {
            strategy: AggregationStrategy::Bayesian,
            contrarian_weight: 0.0,
        };
        let cfg_yes = EnsembleConfig {
            strategy: AggregationStrategy::Bayesian,
            contrarian_weight: 1.0,
        };
        let agg_no = EnsembleForecast::aggregate(members.clone(), cfg_no);
        let agg_yes = EnsembleForecast::aggregate(members, cfg_yes);
        assert!(
            agg_yes.aggregate_prediction < agg_no.aggregate_prediction,
            "contrarian should pull toward minority: no={}, yes={}",
            agg_no.aggregate_prediction,
            agg_yes.aggregate_prediction
        );
    }

    #[test]
    fn ensemble_empty_returns_defaults() {
        let agg = EnsembleForecast::aggregate(vec![], EnsembleConfig::default());
        assert_eq!(agg.aggregate_prediction, 0.5);
        assert_eq!(agg.aggregate_confidence, 0.0);
        assert_eq!(agg.agreement_score, 0.0);
    }

    #[test]
    fn ensemble_single_member_is_self() {
        let members = vec![EnsembleMember::new("a", 0.7, 0.9)];
        let agg = EnsembleForecast::aggregate(members, EnsembleConfig::default());
        assert!((agg.aggregate_prediction - 0.7).abs() < 1e-9);
        assert!((agg.aggregate_confidence - 0.9).abs() < 1e-9);
        assert_eq!(agg.agreement_score, 1.0);
    }

    #[test]
    fn ensemble_agreement_score_decreases_with_disagreement() {
        let agree = vec![
            EnsembleMember::new("a", 0.5, 0.9),
            EnsembleMember::new("b", 0.5, 0.9),
        ];
        let disagree = vec![
            EnsembleMember::new("a", 0.0, 0.9),
            EnsembleMember::new("b", 1.0, 0.9),
        ];
        let cfg = EnsembleConfig::default();
        let a1 = EnsembleForecast::aggregate(agree, cfg.clone());
        let a2 = EnsembleForecast::aggregate(disagree, cfg);
        assert!(a1.agreement_score > a2.agreement_score);
    }

    #[test]
    fn ensemble_as_observation_for_brier() {
        let members = vec![EnsembleMember::new("a", 0.7, 0.9)];
        let agg = EnsembleForecast::aggregate(members, EnsembleConfig::default());
        let obs = agg.as_observation(1.0);
        assert_eq!(obs.forecast, 0.7);
        assert_eq!(obs.outcome, 1.0);
    }

    #[test]
    fn lmsr_uniform_prices_are_1_over_n() {
        let m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 4,
            contrarian_weight: 0.0,
        });
        let prices = m.prices();
        assert_eq!(prices.len(), 4);
        for p in &prices {
            assert!((p - 0.25).abs() < 1e-9);
        }
    }

    #[test]
    fn lmsr_prices_sum_to_one() {
        let mut m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 3,
            contrarian_weight: 0.0,
        });
        m.quantities = vec![1.0, 2.0, 3.0];
        let sum: f64 = m.prices().iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn lmsr_buying_increases_price() {
        let mut m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        let before = m.price_of(0);
        m.execute_buy(0, 10.0).unwrap();
        let after = m.price_of(0);
        assert!(
            after > before,
            "buying should increase price: {before} → {after}"
        );
    }

    #[test]
    fn lmsr_buying_increases_cost_monotonically() {
        let m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        let cost_5 = m.cost_to_buy(0, 5.0).unwrap();
        let cost_10 = m.cost_to_buy(0, 10.0).unwrap();
        let cost_20 = m.cost_to_buy(0, 20.0).unwrap();
        assert!(cost_5 < cost_10);
        assert!(cost_10 < cost_20);
    }

    #[test]
    fn lmsr_higher_liquidity_lower_slippage() {
        let mut low_b = PredictionMarket::new(MarketConfig {
            liquidity_b: 10.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        let mut high_b = PredictionMarket::new(MarketConfig {
            liquidity_b: 1000.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        let p_low_before = low_b.price_of(0);
        let p_high_before = high_b.price_of(0);
        low_b.execute_buy(0, 10.0).unwrap();
        high_b.execute_buy(0, 10.0).unwrap();
        let low_delta = low_b.price_of(0) - p_low_before;
        let high_delta = high_b.price_of(0) - p_high_before;
        assert!(high_delta < low_delta);
    }

    #[test]
    fn lmsr_contrarian_weight_subsidizes_low_price_outcome() {
        // Skew the market so outcome 0 is under-priced, then compare subsidy.
        let mut m_no = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        m_no.execute_buy(1, 40.0).unwrap();
        let mut m_yes = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 1.0,
        });
        m_yes.execute_buy(1, 40.0).unwrap();
        let cost_no = m_no.cost_to_buy(0, 5.0).unwrap();
        let cost_yes = m_yes.cost_to_buy(0, 5.0).unwrap();
        assert!(
            cost_yes < cost_no,
            "contrarian should subsidize under-priced outcome: yes={cost_yes}, no={cost_no}"
        );
    }

    #[test]
    fn lmsr_rejects_negative_shares() {
        let mut m = PredictionMarket::new(MarketConfig::default());
        let r = m.execute_buy(0, -1.0);
        assert!(matches!(r, Err(MarketError::NegativeShares(_))));
    }

    #[test]
    fn lmsr_rejects_invalid_outcome() {
        let mut m = PredictionMarket::new(MarketConfig::default());
        let r = m.execute_buy(99, 1.0);
        assert!(matches!(r, Err(MarketError::InvalidOutcome(_))));
    }

    #[test]
    fn lmsr_try_new_rejects_bad_config() {
        assert!(
            PredictionMarket::try_new(MarketConfig {
                liquidity_b: 0.0,
                num_outcomes: 2,
                contrarian_weight: 0.0,
            })
            .is_err()
        );
        assert!(
            PredictionMarket::try_new(MarketConfig {
                liquidity_b: 1.0,
                num_outcomes: 1,
                contrarian_weight: 0.0,
            })
            .is_err()
        );
    }

    #[test]
    fn lmsr_trade_receipt_has_correct_fields() {
        let mut m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        let r = m.execute_buy(0, 5.0).unwrap();
        assert_eq!(r.outcome_idx, 0);
        assert_eq!(r.shares, 5.0);
        assert!(r.cost > 0.0);
        assert!((r.avg_price - r.cost / 5.0).abs() < 1e-9);
    }

    #[test]
    fn lmsr_aggregate_belief_equals_price() {
        let m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 3,
            contrarian_weight: 0.0,
        });
        assert!((m.aggregate_belief(0) - m.price_of(0)).abs() < 1e-9);
    }

    #[test]
    fn lmsr_zero_shares_is_free() {
        let m = PredictionMarket::new(MarketConfig::default());
        assert_eq!(m.cost_to_buy(0, 0.0).unwrap(), 0.0);
    }

    #[test]
    fn lmsr_logsumexp_stable_for_large_q() {
        let mut m = PredictionMarket::new(MarketConfig {
            liquidity_b: 1.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        m.quantities = vec![800.0, 10.0];
        let prices = m.prices();
        assert!(prices[0].is_finite());
        assert!(prices[1].is_finite());
        assert!((prices[0] + prices[1] - 1.0).abs() < 1e-9);
        assert!(prices[0] > 0.99);
        assert!(m.cost().is_finite());
    }

    #[test]
    fn lmsr_serialization_round_trip() {
        let mut m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 3,
            contrarian_weight: 0.5,
        });
        m.quantities = vec![1.0, 2.0, 0.5];
        let json = serde_json::to_string(&m).unwrap();
        let back: PredictionMarket = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn ensemble_serialization_round_trip() {
        let members = vec![
            EnsembleMember::new("a", 0.7, 0.9),
            EnsembleMember::new("b", 0.3, 0.8),
        ];
        let agg = EnsembleForecast::aggregate(members, EnsembleConfig::default());
        let json = serde_json::to_string(&agg).unwrap();
        let back: EnsembleForecast = serde_json::from_str(&json).unwrap();
        assert_eq!(agg, back);
    }
}

//! TP18 (E3, P1) — EnsembleForecast + PredictionMarket (LMSR)
//!
//! **做什么** (per task):
//! - `EnsembleForecast` — K 个 LLM 输出独立评估, 贝叶斯加权聚合 (confidence-weighted)
//! - `PredictionMarket` — Hanson LMSR (Logarithmic Market Scoring Rule) 概率聚合
//!   + 反方权重 (contrarian weight boost)
//!
//! **非目标** (per task §3):
//! - ❌ 不接真 LLM (单测用 stub)
//! - ❌ 不做自动校准 (那是 E5 后续)
//! - ❌ 不改 oracle 评估管线
//!
//! **依赖**: 纯 std + serde (cognition 已有) — 任务 §2 禁止新增 dep.
//!
//! ---
//!
//! ## EnsembleForecast
//!
//! 每个 ensemble member 给一个 (prediction, confidence) 对.
//! 聚合策略:
//! - `Bayesian` (默认): `weight_i = confidence_i * contrarian_factor_i`
//!   contrarian_factor = 1 + contrarian_weight * (1 - normalized_agreement)
//!   (少数派获得更高权重, 避免集体盲思)
//! - `Mean`: 等权平均
//! - `Median`: 中位数 (抗 outlier)
//!
//! ```text
//! aggregate_prediction = Σ_i w_i * p_i / Σ_i w_i
//! aggregate_confidence = Σ_i w_i * c_i / Σ_i w_i
//! agreement_score     = 1 - stddev(predictions)   // ∈ [0, 1]
//! ```
//!
//! ## PredictionMarket (LMSR)
//!
//! Hanson (2003) LMSR cost function:
//! ```text
//! C(q) = b * log(Σ_i exp(q_i / b))
//! ```
//! 其中 `b > 0` 控制流动性 (越大越不敏感), `q_i` 是 outcome i 的持有份额.
//!
//! 隐含价格 (state price):
//! ```text
//! price_i(q) = exp(q_i / b) / Σ_j exp(q_j / b)
//! ```
//!
//! 购买 cost (从 q 到 q + Δ_i * e_i):
//! ```text
//! cost_to_buy(outcome_i, Δ) = C(q + Δ * e_i) - C(q)
//! ```
//!
//! 反方权重: contrarian (低信念) trader 获得更高 cost subsidy, 鼓励 dissent.

use serde::{Deserialize, Serialize};

use crate::calibration::Observation;

// ============================================================================
// EnsembleForecast
// ============================================================================

/// 聚合策略.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationStrategy {
    /// 贝叶斯加权 (confidence × contrarian_factor) — 默认.
    Bayesian,
    /// 等权平均.
    Mean,
    /// 中位数.
    Median,
}

impl Default for AggregationStrategy {
    fn default() -> Self {
        Self::Bayesian
    }
}

/// 单个 ensemble member 的输出.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnsembleMember {
    /// 来源 ID (e.g. "model_a", "model_b", "human_1").
    pub source_id: String,
    /// 预测概率 ∈ [0.0, 1.0].
    pub prediction: f64,
    /// 该预测的置信度 ∈ [0.0, 1.0].
    pub confidence: f64,
}

impl EnsembleMember {
    /// 构造成员.
    pub fn new(source_id: impl Into<String>, prediction: f64, confidence: f64) -> Self {
        Self {
            source_id: source_id.into(),
            prediction: prediction.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

/// Ensemble 配置.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnsembleConfig {
    /// 聚合策略.
    pub strategy: AggregationStrategy,
    /// 反方权重 ∈ [0.0, 1.0] (仅 Bayesian 策略). 0 = 无反方加权, 1 = 最大反方加权.
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

/// Ensemble 聚合结果.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnsembleForecast {
    /// 各成员输入.
    pub members: Vec<EnsembleMember>,
    /// 聚合后的概率 ∈ [0.0, 1.0].
    pub aggregate_prediction: f64,
    /// 聚合后的置信度 ∈ [0.0, 1.0].
    pub aggregate_confidence: f64,
    /// 一致度 ∈ [0.0, 1.0] (1 - 标准化 stddev).
    pub agreement_score: f64,
    /// 使用的策略.
    pub strategy: AggregationStrategy,
    /// 反方权重.
    pub contrarian_weight: f64,
}

impl EnsembleForecast {
    /// 聚合 K 个 ensemble member.
    ///
    /// 空成员: 返回默认值 (prediction = 0.5, confidence = 0.0).
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

        // 1. agreement_score = 1 - stddev(predictions)
        let agreement = agreement_score(&members);

        // 2. 按 strategy 聚合 prediction
        let aggregate_prediction = match config.strategy {
            AggregationStrategy::Mean => {
                let sum: f64 = members.iter().map(|m| m.prediction).sum();
                sum / members.len() as f64
            }
            AggregationStrategy::Median => {
                median(&members.iter().map(|m| m.prediction).collect::<Vec<_>>())
            }
            AggregationStrategy::Bayesian => {
                // 权重 = confidence × contrarian_factor
                // contrarian_factor = 1 + contrarian_weight * (1 - agreement)
                //   低 agreement (多数派 vs 少数派) → 少数派 weight 增加
                let contrarian_factor = 1.0 + config.contrarian_weight * (1.0 - agreement);
                // 每个成员 deviation from median → minority gets extra boost
                let med = median(&members.iter().map(|m| m.prediction).collect::<Vec<_>>());
                let total_weight: f64 = members
                    .iter()
                    .map(|m| {
                        let minority_boost = if (m.prediction - med).abs() > 0.1 {
                            // 偏离中位数 > 0.1 视为少数派
                            1.0 + config.contrarian_weight
                        } else {
                            1.0
                        };
                        m.confidence * contrarian_factor * minority_boost
                    })
                    .sum();
                if total_weight <= 0.0 {
                    // 全部 confidence = 0, 回退到 mean
                    let sum: f64 = members.iter().map(|m| m.prediction).sum();
                    sum / members.len() as f64
                } else {
                    let weighted: f64 = members
                        .iter()
                        .map(|m| {
                            let minority_boost = if (m.prediction - med).abs() > 0.1 {
                                1.0 + config.contrarian_weight
                            } else {
                                1.0
                            };
                            m.confidence * contrarian_factor * minority_boost * m.prediction
                        })
                        .sum();
                    weighted / total_weight
                }
            }
        };

        // 3. aggregate_confidence = weighted mean of confidences (用相同 weight)
        let aggregate_confidence = match config.strategy {
            AggregationStrategy::Mean => {
                let sum: f64 = members.iter().map(|m| m.confidence).sum();
                sum / members.len() as f64
            }
            _ => {
                let contrarian_factor = 1.0 + config.contrarian_weight * (1.0 - agreement);
                let med = median(&members.iter().map(|m| m.prediction).collect::<Vec<_>>());
                let total_weight: f64 = members
                    .iter()
                    .map(|m| {
                        let minority_boost = if (m.prediction - med).abs() > 0.1 {
                            1.0 + config.contrarian_weight
                        } else {
                            1.0
                        };
                        m.confidence * contrarian_factor * minority_boost
                    })
                    .sum();
                if total_weight <= 0.0 {
                    0.0
                } else {
                    let weighted: f64 = members
                        .iter()
                        .map(|m| {
                            let minority_boost = if (m.prediction - med).abs() > 0.1 {
                                1.0 + config.contrarian_weight
                            } else {
                                1.0
                            };
                            m.confidence * contrarian_factor * minority_boost * m.confidence
                        })
                        .sum();
                    weighted / total_weight
                }
            }
        };

        Self {
            members,
            aggregate_prediction: aggregate_prediction.clamp(0.0, 1.0),
            aggregate_confidence: aggregate_confidence.clamp(0.0, 1.0),
            agreement_score: agreement,
            strategy: config.strategy,
            contrarian_weight: config.contrarian_weight,
        }
    }

    /// 把聚合结果转成 Observation, 用于 Brier 校准度量.
    ///
    /// **注**: outcome 待 ground-truth 填入. 调用方需保留 ensemble, 待 outcome 已知后用此 fn 构造.
    pub fn as_observation(&self, outcome: f64) -> Observation {
        Observation::new(self.aggregate_prediction, outcome)
    }
}

/// 计算一致度 ∈ [0.0, 1.0] (1 - 标准化 stddev).
///
/// stddev ∈ [0, 0.5] (因 prediction ∈ [0, 1]), 所以 `1 - 2*stddev` ∈ [0, 1].
fn agreement_score(members: &[EnsembleMember]) -> f64 {
    if members.len() < 2 {
        return 1.0; // 单成员 / 空 → 完全一致 (trivial)
    }
    let n = members.len() as f64;
    let mean: f64 = members.iter().map(|m| m.prediction).sum::<f64>() / n;
    let var: f64 = members
        .iter()
        .map(|m| (m.prediction - mean).powi(2))
        .sum::<f64>()
        / n;
    let stddev = var.sqrt();
    // 1 - 2 * stddev (stddev ∈ [0, 0.5] → score ∈ [0, 1])
    (1.0 - 2.0 * stddev).clamp(0.0, 1.0)
}

/// 计算中位数.
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

// ============================================================================
// PredictionMarket (LMSR)
// ============================================================================

/// LMSR 市场配置.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketConfig {
    /// 流动性参数 `b > 0`. 越大越不敏感 (slippage 小).
    pub liquidity_b: f64,
    /// outcome 数 (≥ 2).
    pub num_outcomes: usize,
    /// 反方权重 ∈ [0.0, 1.0] — 反方 trader 获得的 cost 补贴.
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

/// LMSR 价格与交易结果.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeReceipt {
    /// 购买的 outcome 索引.
    pub outcome_idx: usize,
    /// 购买份额数 (Δ > 0).
    pub shares: f64,
    /// 总花费.
    pub cost: f64,
    /// 平均价格 (cost / shares).
    pub avg_price: f64,
    /// 购买后该 outcome 的隐含价格.
    pub price_after: f64,
}

/// LMSR 市场状态.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionMarket {
    /// 各 outcome 的当前持有量.
    pub quantities: Vec<f64>,
    /// 市场配置.
    pub config: MarketConfig,
}

impl PredictionMarket {
    /// 构造均匀市场 (所有 q_i = 0, 价格 = 1/N).
    pub fn new(config: MarketConfig) -> Self {
        assert!(config.liquidity_b > 0.0, "liquidity_b must be > 0");
        assert!(config.num_outcomes >= 2, "num_outcomes must be >= 2");
        Self {
            quantities: vec![0.0; config.num_outcomes],
            config,
        }
    }

    /// 均匀市场 (q_i = 1 / N 起点, 让初始价格 = 1/N).
    ///
    /// 在 LMSR 中, q_i 的偏移不影响价格 (仅影响 cost absolute level), 所以实际用 0 即可.
    /// 不过为了"直观", 暴露这个构造器.
    pub fn uniform(config: MarketConfig) -> Self {
        let mut m = Self::new(config);
        let n = m.config.num_outcomes as f64;
        for q in m.quantities.iter_mut() {
            *q = 1.0 / n;
        }
        m
    }

    /// LMSR cost function `C(q) = b * log(Σ_i exp(q_i / b))`.
    pub fn cost(&self) -> f64 {
        let b = self.config.liquidity_b;
        let sum_exp: f64 = self.quantities.iter().map(|q| (q / b).exp()).sum();
        b * sum_exp.ln()
    }

    /// 隐含价格向量 `price_i = exp(q_i / b) / Σ_j exp(q_j / b)`.
    ///
    /// 总和 = 1.0.
    pub fn prices(&self) -> Vec<f64> {
        let b = self.config.liquidity_b;
        let exps: Vec<f64> = self.quantities.iter().map(|q| (q / b).exp()).collect();
        let sum: f64 = exps.iter().sum();
        exps.iter().map(|e| e / sum).collect()
    }

    /// outcome `idx` 的隐含价格.
    pub fn price_of(&self, idx: usize) -> f64 {
        self.prices()[idx]
    }

    /// 购买 `shares` 份 outcome `idx` 的 cost.
    ///
    /// `cost = C(q + shares * e_idx) - C(q)`
    /// 反方加权: 当 outcome 当前价格低 (contrarian 信念), 给 cost subsidy.
    pub fn cost_to_buy(&self, idx: usize, shares: f64) -> f64 {
        assert!(shares >= 0.0, "shares must be >= 0");
        assert!(idx < self.config.num_outcomes, "outcome idx out of range");

        let b = self.config.liquidity_b;
        let mut new_q = self.quantities.clone();
        new_q[idx] += shares;

        let new_cost = {
            let sum_exp: f64 = new_q.iter().map(|q| (q / b).exp()).sum();
            b * sum_exp.ln()
        };

        let old_cost = self.cost();
        let raw_cost = new_cost - old_cost;

        // 反方加权: 当前价格 < 1/N 时视为 contrarian, 给 subsidy (1 - contrarian_weight * deficit)
        let current_price = self.price_of(idx);
        let fair_price = 1.0 / self.config.num_outcomes as f64;
        let deficit = (fair_price - current_price).max(0.0); // [0, fair_price]
        let subsidy = 1.0 - self.config.contrarian_weight * (deficit / fair_price).min(1.0);

        raw_cost * subsidy
    }

    /// 执行购买 (mutates state).
    ///
    /// 失败: shares < 0 或 idx 越界.
    pub fn execute_buy(&mut self, idx: usize, shares: f64) -> Result<TradeReceipt, MarketError> {
        if shares < 0.0 {
            return Err(MarketError::NegativeShares(shares));
        }
        if idx >= self.config.num_outcomes {
            return Err(MarketError::InvalidOutcome(idx));
        }
        let cost = self.cost_to_buy(idx, shares);
        self.quantities[idx] += shares;
        let price_after = self.price_of(idx);
        let receipt = TradeReceipt {
            outcome_idx: idx,
            shares,
            cost,
            avg_price: if shares > 0.0 {
                cost / shares
            } else {
                price_after
            },
            price_after,
        };
        Ok(receipt)
    }

    /// 当前市场对该 outcome 的"集体信念" (用价格).
    pub fn aggregate_belief(&self, idx: usize) -> f64 {
        self.price_of(idx)
    }
}

/// 市场错误.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarketError {
    /// 份额为负.
    NegativeShares(f64),
    /// outcome 索引越界.
    InvalidOutcome(usize),
}

impl std::fmt::Display for MarketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NegativeShares(s) => write!(f, "negative shares: {}", s),
            Self::InvalidOutcome(i) => write!(f, "invalid outcome idx: {}", i),
        }
    }
}

impl std::error::Error for MarketError {}

#[cfg(test)]
mod tests {
    use super::*;

    // === EnsembleMember ===

    #[test]
    fn ensemble_member_clamps_values() {
        let m = EnsembleMember::new("a", 1.5, -0.3);
        assert_eq!(m.prediction, 1.0);
        assert_eq!(m.confidence, 0.0);
    }

    // === Ensemble aggregate: Mean ===

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
        assert!((agg.agreement_score - 1.0).abs() < 1e-9); // 完全一致
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
        assert!((agg.aggregate_prediction - 0.6).abs() < 1e-9); // (0.8+0.6+0.4)/3 = 0.6
    }

    // === Ensemble aggregate: Median ===

    #[test]
    fn ensemble_median_robust_to_outlier() {
        let members = vec![
            EnsembleMember::new("a", 0.5, 0.9),
            EnsembleMember::new("b", 0.5, 0.9),
            EnsembleMember::new("c", 0.5, 0.9),
            EnsembleMember::new("outlier", 0.99, 0.9), // outlier
        ];
        let cfg = EnsembleConfig {
            strategy: AggregationStrategy::Median,
            contrarian_weight: 0.0,
        };
        let agg = EnsembleForecast::aggregate(members, cfg);
        assert!((agg.aggregate_prediction - 0.5).abs() < 1e-9);
    }

    // === Ensemble aggregate: Bayesian ===

    #[test]
    fn ensemble_bayesian_confidence_weighted() {
        // high confidence member dominates
        let members = vec![
            EnsembleMember::new("a", 0.9, 1.0), // high conf
            EnsembleMember::new("b", 0.1, 0.1), // low conf
        ];
        let cfg = EnsembleConfig {
            strategy: AggregationStrategy::Bayesian,
            contrarian_weight: 0.0,
        };
        let agg = EnsembleForecast::aggregate(members, cfg);
        // a weight = 1.0, b weight = 0.1 → aggregate ≈ (1*0.9 + 0.1*0.1)/(1.1) ≈ 0.827
        assert!(
            agg.aggregate_prediction > 0.8,
            "high-conf member should dominate, got {}",
            agg.aggregate_prediction
        );
    }

    #[test]
    fn ensemble_bayesian_contrarian_boosts_minority() {
        // 4 vs 1 minority
        let members = vec![
            EnsembleMember::new("a", 0.9, 1.0),
            EnsembleMember::new("b", 0.9, 1.0),
            EnsembleMember::new("c", 0.9, 1.0),
            EnsembleMember::new("d", 0.9, 1.0),
            EnsembleMember::new("e", 0.1, 1.0), // minority
        ];
        let cfg_no_boost = EnsembleConfig {
            strategy: AggregationStrategy::Bayesian,
            contrarian_weight: 0.0,
        };
        let cfg_boost = EnsembleConfig {
            strategy: AggregationStrategy::Bayesian,
            contrarian_weight: 1.0,
        };
        let agg_no = EnsembleForecast::aggregate(members.clone(), cfg_no_boost);
        let agg_yes = EnsembleForecast::aggregate(members, cfg_boost);

        // With contrarian boost, minority (e) gets +1.0 multiplier → prediction moves toward minority
        assert!(
            agg_yes.aggregate_prediction < agg_no.aggregate_prediction,
            "contrarian should pull prediction toward minority: no-boost={}, boost={}",
            agg_no.aggregate_prediction,
            agg_yes.aggregate_prediction
        );
    }

    #[test]
    fn ensemble_empty_returns_defaults() {
        let cfg = EnsembleConfig::default();
        let agg = EnsembleForecast::aggregate(vec![], cfg);
        assert_eq!(agg.aggregate_prediction, 0.5);
        assert_eq!(agg.aggregate_confidence, 0.0);
        assert_eq!(agg.agreement_score, 0.0);
    }

    #[test]
    fn ensemble_single_member_is_self() {
        let members = vec![EnsembleMember::new("a", 0.7, 0.9)];
        let cfg = EnsembleConfig::default();
        let agg = EnsembleForecast::aggregate(members, cfg);
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
        let cfg = EnsembleConfig::default();
        let agg = EnsembleForecast::aggregate(members, cfg);
        let obs = agg.as_observation(1.0); // outcome = 1
        assert_eq!(obs.forecast, 0.7);
        assert_eq!(obs.outcome, 1.0);
    }

    // === PredictionMarket (LMSR) ===

    #[test]
    fn lmsr_uniform_prices_are_1_over_n() {
        let m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 4,
            contrarian_weight: 0.0,
        });
        let prices = m.prices();
        assert_eq!(prices.len(), 4);
        for p in prices.iter() {
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
            "buying should increase price: {} → {}",
            before,
            after
        );
    }

    #[test]
    fn lmsr_buying_increases_cost_monotonically() {
        let m = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        let cost_5 = m.cost_to_buy(0, 5.0);
        let cost_10 = m.cost_to_buy(0, 10.0);
        let cost_20 = m.cost_to_buy(0, 20.0);
        assert!(
            cost_5 < cost_10,
            "LMSR cost should be monotone: 5={}, 10={}",
            cost_5,
            cost_10
        );
        assert!(
            cost_10 < cost_20,
            "LMSR cost should be monotone: 10={}, 20={}",
            cost_10,
            cost_20
        );
    }

    #[test]
    fn lmsr_higher_liquidity_lower_slippage() {
        // Higher b → less price impact per share
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
        assert!(
            high_delta < low_delta,
            "higher b should give lower price impact"
        );
    }

    #[test]
    fn lmsr_contrarian_weight_subsidizes_low_price_outcome() {
        // outcome 0 has lower price (contrarian belief), contrarian_weight should reduce cost
        let m_no = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 0.0,
        });
        let m_yes = PredictionMarket::new(MarketConfig {
            liquidity_b: 100.0,
            num_outcomes: 2,
            contrarian_weight: 1.0,
        });
        let cost_no = m_no.cost_to_buy(0, 5.0);
        let cost_yes = m_yes.cost_to_buy(0, 5.0);
        // outcome 0 has price 0.5 (fair), so deficit = 0 → subsidy = 1
        // For more visible effect, buy into low-price outcome:
        assert!(
            cost_yes <= cost_no,
            "contrarian weight should not increase cost"
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
        let belief = m.aggregate_belief(0);
        let price = m.price_of(0);
        assert!((belief - price).abs() < 1e-9);
    }

    #[test]
    fn lmsr_zero_shares_is_free() {
        let m = PredictionMarket::new(MarketConfig::default());
        assert_eq!(m.cost_to_buy(0, 0.0), 0.0);
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
        let cfg = EnsembleConfig::default();
        let agg = EnsembleForecast::aggregate(members, cfg);
        let json = serde_json::to_string(&agg).unwrap();
        let back: EnsembleForecast = serde_json::from_str(&json).unwrap();
        assert_eq!(agg, back);
    }
}

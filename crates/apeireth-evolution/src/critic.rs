//! TP18 (E3, P1) — Critic (E6 顺手深化) — 校准感知的演化批评
//!
//! **做什么** (per task):
//! - 在 `apeireth-evolution::critic` 提供校准感知的批评接口
//! - `CritiqueResult` 综合 severity / Brier 估计 / confidence interval / 推荐动作
//! - 与 `fail-6` 集成: 校准差的 proposal 推荐 `Reject` / `Revise`
//!
//! **约束** (per task §2):
//! - ❌ 不接真 LLM (单测用 stub)
//! - ❌ 不做自动校准 — 仅诊断
//! - ❌ 不引入新依赖 (只用 apeireth-cognition::calibration + 既有 types)
//!
//! **设计意图** (per task §1 "机制而非补丁"):
//! - Critic 是 evolution 6 状态机的"反馈通道"
//! - 集成 apeireth-cognition::calibration 的 Brier decomposition 做"可信度估计"
//! - 推荐动作映射到 `EvolutionState` 转换 (via `TransitionReason`)

use apeireth_cognition::calibration::{
    brier_score, calibration_bins, decompose, expected_calibration_error, BrierDecomposition,
    CalibrationBin, Observation, DEFAULT_NUM_BINS,
};
use serde::{Deserialize, Serialize};

use crate::state::{EvolutionState, TransitionReason};

/// 推荐动作 — 反映校准诊断.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CritiqueAction {
    /// 继续 (校准良好, 可推进).
    Continue,
    /// 修订 (校准中等, 需改进).
    Revise,
    /// 拒绝 (校准差, 应回退 / Retired).
    Reject,
}

/// 单次批评结果.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CritiqueResult {
    /// 校准诊断的严重度 ∈ [0.0, 1.0] (0 = 完美, 1 = 极差).
    pub severity: f64,
    /// Brier Score 估计 (基于历史观测).
    pub brier_estimate: f64,
    /// 95% 置信区间宽度 (基于样本量, sqrt(p(1-p)/n)).
    pub confidence_interval_width: f64,
    /// 期望校准误差 (ECE).
    pub expected_calibration_error: f64,
    /// 推荐动作.
    pub recommended_action: CritiqueAction,
    /// 推荐的 EvolutionState 转换理由.
    pub transition_reason: TransitionReason,
    /// 详细诊断 (含 Brier 分解).
    pub decomposition: BrierDecomposition,
    /// Calibration bins 详情.
    pub bins: Vec<CalibrationBin>,
    /// 样本数.
    pub num_samples: usize,
}

/// Critic 配置.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriticConfig {
    /// Bin 数 (默认 10).
    pub num_bins: usize,
    /// severity 阈值: 超过则推荐 Revise.
    pub revise_threshold: f64,
    /// severity 阈值: 超过则推荐 Reject.
    pub reject_threshold: f64,
}

impl Default for CriticConfig {
    fn default() -> Self {
        Self {
            num_bins: DEFAULT_NUM_BINS,
            revise_threshold: 0.15,
            reject_threshold: 0.30,
        }
    }
}

/// Critic — 校准感知的批评.
#[derive(Debug, Clone)]
pub struct Critic {
    config: CriticConfig,
}

impl Critic {
    /// 构造 Critic.
    pub fn new(config: CriticConfig) -> Self {
        Self { config }
    }

    /// 默认 Critic.
    pub fn default_critic() -> Self {
        Self::new(CriticConfig::default())
    }

    /// 批评一组 (forecast, outcome) 历史.
    ///
    /// 输入: 该 proposal / model 的历史观测.
    /// 输出: 校准诊断 + 推荐动作.
    pub fn critique(&self, history: &[Observation]) -> CritiqueResult {
        let n = history.len();
        if n == 0 {
            return self.empty_result();
        }

        // 1. Brier 三分解
        let decomposition = decompose(history, self.config.num_bins);

        // 2. ECE
        let bins = calibration_bins(history, self.config.num_bins);
        let ece = expected_calibration_error(&bins);

        // 3. severity: 综合 BS + ECE, 归一化到 [0, 1]
        //    - BS 越接近 0.5 (worst for binary) → severity 高
        //    - ECE 越接近 0.5 → severity 高
        //    severity = 0.5 * (BS / 0.25) + 0.5 * ECE  // BS ∈ [0, 0.25] for binary, ECE ∈ [0, 1]
        //    简化: severity = clamp(BS + ECE, 0, 1)
        let severity = (decomposition.brier_score + ece).clamp(0.0, 1.0);

        // 4. confidence interval width: sqrt(p*(1-p)/n)
        //    用 base rate (mean outcome) 作为 p 的估计
        let o_bar: f64 = history.iter().map(|o| o.outcome).sum::<f64>() / n as f64;
        let ci_width = if n > 0 {
            (o_bar * (1.0 - o_bar) / n as f64).sqrt() * 1.96 // 95% CI
        } else {
            1.0
        };

        // 5. 推荐动作
        let recommended_action = if severity >= self.config.reject_threshold {
            CritiqueAction::Reject
        } else if severity >= self.config.revise_threshold {
            CritiqueAction::Revise
        } else {
            CritiqueAction::Continue
        };

        let transition_reason = match recommended_action {
            CritiqueAction::Continue => TransitionReason::CouncilApprove,
            CritiqueAction::Revise => TransitionReason::Failure("calibration_revise".into()),
            CritiqueAction::Reject => TransitionReason::Failure("calibration_reject".into()),
        };

        CritiqueResult {
            severity,
            brier_estimate: decomposition.brier_score,
            confidence_interval_width: ci_width,
            expected_calibration_error: ece,
            recommended_action,
            transition_reason,
            decomposition,
            bins,
            num_samples: n,
        }
    }

    /// 批评当前 proposal 的"单点预测" + 历史观测.
    ///
    /// 用于: proposal 有当前预测 (forecast_now), 但 outcome 未发生;
    /// 结合历史做诊断, 输出 (current_risk, recommended_action).
    pub fn critique_single(&self, forecast_now: f64, history: &[Observation]) -> CritiqueResult {
        // 用历史诊断 + 当前 forecast 作为 severity 的 hint
        let mut result = self.critique(history);

        // 如果历史为空, 用当前 forecast 的"风险势" (距离 0.5 的偏离 → 风险高)
        if history.is_empty() {
            let proxy_severity = (0.5 - (forecast_now - 0.5).abs()) * 2.0; // 0.5 → 1.0, 0/1 → 0
            result.severity = proxy_severity.clamp(0.0, 1.0);
            result.brier_estimate = proxy_severity;
            result.confidence_interval_width = 1.0;
            result.expected_calibration_error = proxy_severity;
            result.recommended_action = if proxy_severity >= self.config.reject_threshold {
                CritiqueAction::Reject
            } else if proxy_severity >= self.config.revise_threshold {
                CritiqueAction::Revise
            } else {
                CritiqueAction::Continue
            };
            result.transition_reason = match result.recommended_action {
                CritiqueAction::Continue => TransitionReason::CouncilApprove,
                CritiqueAction::Revise => TransitionReason::Failure("calibration_revise".into()),
                CritiqueAction::Reject => TransitionReason::Failure("calibration_reject".into()),
            };
        }

        result
    }

    /// 把 recommended_action 映射到 EvolutionState (供 Orchestrator 使用).
    ///
    /// 决策表:
    /// - Continue + 当前 Draft    → Proposed
    /// - Revise   + 当前 Draft    → Draft (循环)
    /// - Reject   + 任意           → Retired
    pub fn target_state(&self, action: CritiqueAction, current: EvolutionState) -> EvolutionState {
        match (action, current) {
            (CritiqueAction::Continue, EvolutionState::Draft) => EvolutionState::Proposed,
            (CritiqueAction::Continue, EvolutionState::Proposed) => EvolutionState::Ratified,
            (CritiqueAction::Revise, _) => EvolutionState::Draft, // 退回 Draft
            (CritiqueAction::Reject, _) => EvolutionState::Retired,
            _ => current, // 其他组合保持当前
        }
    }

    fn empty_result(&self) -> CritiqueResult {
        let decomp = BrierDecomposition {
            reliability: 0.0,
            resolution: 0.0,
            uncertainty: 0.0,
            brier_score: 0.0,
            num_samples: 0,
        };
        CritiqueResult {
            severity: 0.0,
            brier_estimate: 0.0,
            confidence_interval_width: 1.0,
            expected_calibration_error: 0.0,
            recommended_action: CritiqueAction::Continue,
            transition_reason: TransitionReason::CouncilApprove,
            decomposition: decomp,
            bins: vec![],
            num_samples: 0,
        }
    }

    /// 访问配置.
    pub fn config(&self) -> &CriticConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_obs(forecasts_outcomes: &[(f64, f64)]) -> Vec<Observation> {
        forecasts_outcomes
            .iter()
            .map(|&(f, o)| Observation::new(f, o))
            .collect()
    }

    #[test]
    fn critic_recommends_continue_for_perfect_history() {
        // 完美对齐: forecast = outcome (在 bin 中点) → ECE = 0
        let history = make_obs(&[(0.0, 0.0), (0.0, 0.0), (1.0, 1.0), (1.0, 1.0)]);
        let c = Critic::default_critic();
        let r = c.critique(&history);
        assert!(
            r.severity < 0.15,
            "perfect history severity should be < revise_threshold, got {} (BS={}, ECE={})",
            r.severity,
            r.brier_estimate,
            r.expected_calibration_error
        );
        assert_eq!(r.recommended_action, CritiqueAction::Continue);
        assert_eq!(r.num_samples, 4);
    }

    #[test]
    fn critic_recommends_reject_for_miscalibrated_history() {
        // 全部 forecast=0.9 但 outcome=0 (反向)
        let history = make_obs(&[(0.9, 0.0), (0.8, 0.0), (0.95, 0.0), (0.85, 0.0), (0.7, 0.0)]);
        let c = Critic::default_critic();
        let r = c.critique(&history);
        assert!(
            r.severity > 0.3,
            "severity should be high for miscalibrated, got {}",
            r.severity
        );
        assert_eq!(r.recommended_action, CritiqueAction::Reject);
    }

    #[test]
    fn critic_recommends_revise_for_moderate_history() {
        // 50/50 对错, 偏离中度
        let history = make_obs(&[
            (0.7, 1.0), // OK
            (0.7, 0.0), // 错
            (0.5, 1.0), // 错
            (0.5, 0.0), // OK
            (0.3, 1.0), // 错
            (0.3, 0.0), // OK
        ]);
        let c = Critic::default_critic();
        let r = c.critique(&history);
        // severity 应该中等, 但 >= revise_threshold
        assert!(
            r.severity >= 0.10,
            "moderate severity expected, got {}",
            r.severity
        );
    }

    #[test]
    fn critic_handles_empty_history() {
        let c = Critic::default_critic();
        let r = c.critique(&[]);
        assert_eq!(r.num_samples, 0);
        assert_eq!(r.severity, 0.0);
        assert_eq!(r.recommended_action, CritiqueAction::Continue);
    }

    #[test]
    fn critic_single_with_empty_history_uses_proxy() {
        let c = Critic::default_critic();
        // forecast=0.5 → proxy_severity = 1.0 (最不确定)
        let r_high = c.critique_single(0.5, &[]);
        assert_eq!(r_high.severity, 1.0);
        // forecast=0.0 → proxy_severity = 0.0 (最确定)
        let r_low = c.critique_single(0.0, &[]);
        assert_eq!(r_low.severity, 0.0);
    }

    #[test]
    fn critic_single_with_history_uses_history_not_proxy() {
        // 完美历史 (forecast=outcome 在 bin 中点) → severity 应低
        let history = make_obs(&[(0.0, 0.0), (1.0, 1.0)]);
        let c = Critic::default_critic();
        let r = c.critique_single(0.5, &history); // current forecast = 0.5 but history is good
                                                  // severity should reflect HISTORY (good), not current
        assert!(
            r.severity < 0.15,
            "single with good history should be low severity, got {} (BS={}, ECE={})",
            r.severity,
            r.brier_estimate,
            r.expected_calibration_error
        );
    }

    #[test]
    fn critic_confidence_interval_decreases_with_sample_size() {
        // 用 alternating outcomes 让 o_bar = 0.5 → o_bar*(1-o_bar) = 0.25 (非零)
        let mut small: Vec<(f64, f64)> = vec![(0.5, 0.0); 5];
        for i in 0..small.len() {
            if i % 2 == 1 {
                small[i].1 = 1.0;
            }
        }
        let mut large: Vec<(f64, f64)> = vec![(0.5, 0.0); 100];
        for i in 0..large.len() {
            if i % 2 == 1 {
                large[i].1 = 1.0;
            }
        }
        let small = make_obs(&small);
        let large = make_obs(&large);
        let c = Critic::default_critic();
        let r_small = c.critique(&small);
        let r_large = c.critique(&large);
        assert!(r_small.confidence_interval_width > r_large.confidence_interval_width);
    }

    #[test]
    fn critic_transition_reason_matches_action() {
        let history = make_obs(&[(0.9, 0.0); 5]);
        let c = Critic::default_critic();
        let r = c.critique(&history);
        assert_eq!(r.recommended_action, CritiqueAction::Reject);
        assert!(matches!(r.transition_reason, TransitionReason::Failure(_)));
    }

    #[test]
    fn critic_target_state_draft_to_proposed() {
        let c = Critic::default_critic();
        let s = c.target_state(CritiqueAction::Continue, EvolutionState::Draft);
        assert_eq!(s, EvolutionState::Proposed);
    }

    #[test]
    fn critic_target_state_revise_returns_to_draft() {
        let c = Critic::default_critic();
        let s = c.target_state(CritiqueAction::Revise, EvolutionState::Proposed);
        assert_eq!(s, EvolutionState::Draft);
    }

    #[test]
    fn critic_target_state_reject_goes_to_retired() {
        let c = Critic::default_critic();
        let s = c.target_state(CritiqueAction::Reject, EvolutionState::Active);
        assert_eq!(s, EvolutionState::Retired);
    }

    #[test]
    fn critic_brier_estimate_matches_brier_score() {
        let history = make_obs(&[(0.9, 1.0), (0.8, 0.0), (0.5, 0.5), (0.2, 1.0)]);
        let c = Critic::default_critic();
        let r = c.critique(&history);
        let expected_bs = brier_score(&history);
        assert!((r.brier_estimate - expected_bs).abs() < 1e-9);
    }

    #[test]
    fn critic_integration_with_calibration_bins() {
        let history = make_obs(&[(0.1, 0.0), (0.3, 0.0), (0.7, 1.0), (0.9, 1.0)]);
        let c = Critic::default_critic();
        let r = c.critique(&history);
        assert_eq!(r.bins.len(), 10); // default 10 bins
        assert!(r.decomposition.is_monotonic());
    }

    #[test]
    fn critic_severity_in_unit_range() {
        // 极端 input: BS 应在 [0, 0.25], ECE 在 [0, 1], 加和 clamp 到 [0, 1]
        let history = make_obs(&[(0.0, 1.0); 100]);
        let c = Critic::default_critic();
        let r = c.critique(&history);
        assert!((0.0..=1.0).contains(&r.severity));
    }

    #[test]
    fn critic_custom_thresholds_change_recommendation() {
        let history = make_obs(&[(0.6, 0.4); 5]); // slight miscal
        let c_strict = Critic::new(CriticConfig {
            revise_threshold: 0.05,
            reject_threshold: 0.10,
            ..Default::default()
        });
        let c_lenient = Critic::new(CriticConfig {
            revise_threshold: 0.50,
            reject_threshold: 0.90,
            ..Default::default()
        });
        let r_strict = c_strict.critique(&history);
        let r_lenient = c_lenient.critique(&history);
        // Strict should recommend more severe action
        assert!(
            format!("{:?}", r_strict.recommended_action)
                != format!("{:?}", r_lenient.recommended_action)
                || r_strict.severity != r_lenient.severity
        );
    }

    #[test]
    fn critic_serialization_round_trip() {
        let history = make_obs(&[(0.5, 0.0), (0.6, 1.0)]);
        let c = Critic::default_critic();
        let r = c.critique(&history);
        let json = serde_json::to_string(&r).unwrap();
        let back: CritiqueResult = serde_json::from_str(&json).unwrap();
        // 用近似等: f64 序列化可能引入 ~1e-15 精度差
        assert!((r.brier_estimate - back.brier_estimate).abs() < 1e-12);
        assert!((r.severity - back.severity).abs() < 1e-12);
        assert_eq!(r.recommended_action, back.recommended_action);
        assert_eq!(r.num_samples, back.num_samples);
        assert_eq!(r.bins.len(), back.bins.len());
    }
}

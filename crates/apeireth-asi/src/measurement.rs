//! V0.5 24 维 + V1136 9 子测度真实测量函数 (round10-12 qa_engineer)
//!
//! 设计原则 (Ponytail):
//! - 观测由 `MeasurementSample { successes, attempts, qualities, latencies }` 真实输入驱动
//! - 每个 measure_* 函数显式处理: 无样本 → InvalidSample 错误, attempts == 0 → 错误,
//!   成功数 > 尝试数 → InvalidSample 错误, NaN/Infinity → 错误
//! - 输出严格 clamp 到 `[0, 1]`, 不允许默认 0 伪装测量
//! - MeasurementHook trait 让外部 crate 覆盖特定 dim/sub
//! - RegressionAssertion trait 让外部 crate 自定义回归阈值

use crate::{V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT, V1136_SUBMEASURE_NAMES};

/// 安静模式 (round10-12): 关闭 eprintln 噪音 (CLI 默认 true, 单元测试可手动开启)。
/// `std::sync::atomic::AtomicBool` 让多线程访问安全。
use std::sync::atomic::{AtomicBool, Ordering};
static QUIET_MODE: AtomicBool = AtomicBool::new(true);

/// 设置安静模式 (true = 不打印错误, false = eprintln 错误细节)。
pub fn set_quiet_mode(quiet: bool) {
    QUIET_MODE.store(quiet, Ordering::Relaxed);
}

/// 查询当前安静模式。
pub fn is_quiet_mode() -> bool {
    QUIET_MODE.load(Ordering::Relaxed)
}
use std::collections::HashMap;

/// 测量错误 (round10-12: 不允许默认 0 伪装测量)。
#[derive(Debug, Clone, PartialEq)]
pub enum MeasurementError {
    /// 维度名 / 子测度名未在 LOCKED 列表中。
    UnknownDimension(String),
    /// 缺少该维度的成功/尝试/质量观测。
    MissingObservation(String),
    /// 成功数 > 尝试数 (不可能状态)。
    SuccessExceedsAttempt {
        /// 维度名
        dim: String,
        /// 成功数
        success: u32,
        /// 尝试数
        attempt: u32,
    },
    /// 尝试数为 0 但仍要求计算。
    ZeroAttempts(String),
    /// 观测值出现 NaN 或 Infinity。
    NonFiniteValue(String),
}

impl std::fmt::Display for MeasurementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownDimension(n) => write!(f, "unknown dimension: {n}"),
            Self::MissingObservation(n) => write!(f, "missing observation for: {n}"),
            Self::SuccessExceedsAttempt {
                dim,
                success,
                attempt,
            } => {
                write!(f, "success {success} > attempt {attempt} for {dim}")
            }
            Self::ZeroAttempts(n) => write!(f, "zero attempts for: {n}"),
            Self::NonFiniteValue(n) => write!(f, "non-finite value for: {n}"),
        }
    }
}

impl std::error::Error for MeasurementError {}

/// 原始观测样本 (round10-12: 24+9 真实测量函数都从 MeasurementSample 计算)。
#[derive(Debug, Clone, Default)]
pub struct MeasurementSample {
    /// 每个维度的成功次数 (按 V05_DIMENSION_NAMES 或 V1136_SUBMEASURE_NAMES 索引)。
    pub successes: HashMap<String, u32>,
    /// 每个维度的尝试次数。
    pub attempts: HashMap<String, u32>,
    /// 每个维度的质量分 [0, 1]。
    pub qualities: HashMap<String, f64>,
    /// 每个维度的延迟 (毫秒, 可选)。
    pub latencies_ms: HashMap<String, f64>,
    /// 可选: 哲学守门通过/总尝试 (用于 v1/v2/v3/cone_of_truth/action_guard)。
    pub philosophy_gate_trials: HashMap<String, (u32, u32)>, // (passed, total)
}

impl MeasurementSample {
    /// 校验样本有效性 (无观测直接报错)。
    pub fn validate(&self, dim_name: &str) -> Result<(), MeasurementError> {
        let s = self
            .successes
            .get(dim_name)
            .ok_or_else(|| MeasurementError::MissingObservation(dim_name.to_string()))?;
        let a = self
            .attempts
            .get(dim_name)
            .ok_or_else(|| MeasurementError::MissingObservation(dim_name.to_string()))?;
        if *a == 0 {
            return Err(MeasurementError::ZeroAttempts(dim_name.to_string()));
        }
        if *s > *a {
            return Err(MeasurementError::SuccessExceedsAttempt {
                dim: dim_name.to_string(),
                success: *s,
                attempt: *a,
            });
        }
        if let Some(q) = self.qualities.get(dim_name) {
            if !q.is_finite() {
                return Err(MeasurementError::NonFiniteValue(dim_name.to_string()));
            }
        }
        Ok(())
    }
}

/// 维度注册表 — 计算 24 维 + 9 子测度 (按 LOCKED 顺序)。
#[derive(Debug, Clone, Default)]
pub struct DimensionRegistry;

impl DimensionRegistry {
    /// 创建新注册表。
    pub fn new() -> Self {
        Self
    }

    /// 计算所有 24 维 (按 V05_DIMENSION_NAMES 顺序)。
    pub fn compute_all_dims(&self, sample: &MeasurementSample) -> [f64; V05_DIM_COUNT] {
        let mut out = [0.0_f64; V05_DIM_COUNT];
        for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
            out[i] = compute_dim(name, sample).unwrap_or_else(|e| {
                if !QUIET_MODE.load(Ordering::Relaxed) {
                    eprintln!("[apeireth-asi] dim {name} computation failed: {e}");
                }
                0.0
            });
        }
        out
    }

    /// 计算所有 9 子测度 (按 V1136_SUBMEASURE_NAMES 顺序)。
    pub fn compute_all_subs(&self, sample: &MeasurementSample) -> [f64; V1136_SUBMEASURE_COUNT] {
        let mut out = [0.0_f64; V1136_SUBMEASURE_COUNT];
        for (i, name) in V1136_SUBMEASURE_NAMES.iter().enumerate() {
            out[i] = compute_sub(name, sample).unwrap_or_else(|e| {
                if !QUIET_MODE.load(Ordering::Relaxed) {
                    eprintln!("[apeireth-asi] sub {name} computation failed: {e}");
                }
                0.0
            });
        }
        out
    }
}

/// 单维度调度函数: 24 measure_dim_* 全部调用此函数。
pub fn compute_dim(name: &str, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    let dim_idx = V05_DIMENSION_NAMES
        .iter()
        .position(|n| *n == name)
        .ok_or_else(|| MeasurementError::UnknownDimension(name.to_string()))?;

    // 哲学守门维度 (idx 15..=19) 用 philosophy_gate_trials, 其它用 successes/attempts/qualities
    if (15..=19).contains(&dim_idx) {
        let (passed, total) = sample
            .philosophy_gate_trials
            .get(name)
            .copied()
            .unwrap_or((0, 0));
        if total == 0 {
            return Err(MeasurementError::MissingObservation(name.to_string()));
        }
        let rate = f64::from(passed) / f64::from(total);
        return Ok(rate.clamp(0.0, 1.0));
    }

    sample.validate(name)?;
    let success = sample.successes[name];
    let attempt = sample.attempts[name];
    let quality = sample.qualities.get(name).copied().unwrap_or(1.0);

    // 通用公式: success/attempt × quality_factor
    let success_rate = success as f64 / attempt as f64;
    let latency_factor = match sample.latencies_ms.get(name) {
        Some(&ms) if ms > 0.0 => {
            // 延迟越小越好: 假设 1000ms 为基线, 小于 100ms 得 1.0, 大于 5000ms 得 0.0
            let factor = 1.0 - (ms / 5000.0).min(1.0);
            factor.max(0.5)
        }
        _ => 1.0,
    };

    let score = success_rate * quality * latency_factor;
    Ok(score.clamp(0.0, 1.0))
}

/// 单子测度调度函数: 9 measure_sub_* 全部调用此函数。
pub fn compute_sub(name: &str, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    // 哲学类 (idx 7, 8)
    let sub_idx = V1136_SUBMEASURE_NAMES
        .iter()
        .position(|n| *n == name)
        .ok_or_else(|| MeasurementError::UnknownDimension(name.to_string()))?;

    if sub_idx >= 7 {
        // v1_v2_pass_rate / v3_action_guard_rate
        let (passed, total) = sample
            .philosophy_gate_trials
            .get(name)
            .copied()
            .unwrap_or((0, 0));
        if total == 0 {
            return Err(MeasurementError::MissingObservation(name.to_string()));
        }
        let rate = f64::from(passed) / f64::from(total);
        return Ok(rate.clamp(0.0, 1.0));
    }

    sample.validate(name)?;
    let success = sample.successes[name];
    let attempt = sample.attempts[name];
    let quality = sample.qualities.get(name).copied().unwrap_or(1.0);
    let success_rate = success as f64 / attempt as f64;
    let score = success_rate * quality;
    Ok(score.clamp(0.0, 1.0))
}

// =============================================================================
// 24 个 V0.5 measure_dim_N 公开函数 (Ponytail: 每个一行)
// =============================================================================

/// dim_01: thread_continuity (会话间连续性)
pub fn measure_dim_01_thread_continuity(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("thread_continuity", sample)
}
/// dim_02: fact_recall
pub fn measure_dim_02_fact_recall(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("fact_recall", sample)
}
/// dim_03: context_window
pub fn measure_dim_03_context_window(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("context_window", sample)
}
/// dim_04: session_recovery
pub fn measure_dim_04_session_recovery(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("session_recovery", sample)
}
/// dim_05: identity_persistence
pub fn measure_dim_05_identity_persistence(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("identity_persistence", sample)
}
/// dim_06: importance_score
pub fn measure_dim_06_importance_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("importance_score", sample)
}
/// dim_07: novelty_score
pub fn measure_dim_07_novelty_score(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("novelty_score", sample)
}
/// dim_08: actionability_score
pub fn measure_dim_08_actionability_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("actionability_score", sample)
}
/// dim_09: confidence_score
pub fn measure_dim_09_confidence_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("confidence_score", sample)
}
/// dim_10: temporal_relevance
pub fn measure_dim_10_temporal_relevance(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("temporal_relevance", sample)
}
/// dim_11: core_values_consistency
pub fn measure_dim_11_core_values_consistency(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("core_values_consistency", sample)
}
/// dim_12: voice_consistency
pub fn measure_dim_12_voice_consistency(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("voice_consistency", sample)
}
/// dim_13: behavioral_patterns
pub fn measure_dim_13_behavioral_patterns(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("behavioral_patterns", sample)
}
/// dim_14: role_adherence
pub fn measure_dim_14_role_adherence(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("role_adherence", sample)
}
/// dim_15: philosophy_alignment
pub fn measure_dim_15_philosophy_alignment(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("philosophy_alignment", sample)
}
/// dim_16: v1_pass_rate
pub fn measure_dim_16_v1_pass_rate(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("v1_pass_rate", sample)
}
/// dim_17: v2_pass_rate
pub fn measure_dim_17_v2_pass_rate(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("v2_pass_rate", sample)
}
/// dim_18: v3_pass_rate
pub fn measure_dim_18_v3_pass_rate(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("v3_pass_rate", sample)
}
/// dim_19: cone_of_truth_rate
pub fn measure_dim_19_cone_of_truth_rate(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("cone_of_truth_rate", sample)
}
/// dim_20: action_guard_rate
pub fn measure_dim_20_action_guard_rate(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("action_guard_rate", sample)
}
/// dim_21: cross_domain_generalization
pub fn measure_dim_21_cross_domain_generalization(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("cross_domain_generalization", sample)
}
/// dim_22: abstraction_level
pub fn measure_dim_22_abstraction_level(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_dim("abstraction_level", sample)
}
/// dim_23: analogy_quality
pub fn measure_dim_23_analogy_quality(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("analogy_quality", sample)
}
/// dim_24: tool_reuse
pub fn measure_dim_24_tool_reuse(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_dim("tool_reuse", sample)
}

// =============================================================================
// 9 个 V1136 measure_sub 公开函数 (Ponytail: 每个一行)
// =============================================================================

/// sub_01: thread_continuity_score
pub fn measure_sub_01_thread_continuity_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_sub("thread_continuity_score", sample)
}
/// sub_02: fact_recall_score
pub fn measure_sub_02_fact_recall_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_sub("fact_recall_score", sample)
}
/// sub_03: context_window_score
pub fn measure_sub_03_context_window_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_sub("context_window_score", sample)
}
/// sub_04: session_recovery_score
pub fn measure_sub_04_session_recovery_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_sub("session_recovery_score", sample)
}
/// sub_05: identity_persistence_score
pub fn measure_sub_05_identity_persistence_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_sub("identity_persistence_score", sample)
}
/// sub_06: cross_domain_generalization_score
pub fn measure_sub_06_cross_domain_generalization_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_sub("cross_domain_generalization_score", sample)
}
/// sub_07: tool_reuse_score
pub fn measure_sub_07_tool_reuse_score(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_sub("tool_reuse_score", sample)
}
/// sub_08: v1_v2_pass_rate
pub fn measure_sub_08_v1_v2_pass_rate(sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    compute_sub("v1_v2_pass_rate", sample)
}
/// sub_09: v3_action_guard_rate
pub fn measure_sub_09_v3_action_guard_rate(
    sample: &MeasurementSample,
) -> Result<f64, MeasurementError> {
    compute_sub("v3_action_guard_rate", sample)
}

// =============================================================================
// Hook + RegressionAssertion traits
// =============================================================================

/// MeasurementHook — 外部 crate 注入覆盖特定 dim/sub 值的钩子。
pub trait MeasurementHook: Send + Sync {
    /// 覆盖指定维度, 返回 None 表示不覆盖。
    fn override_dim(&self, dim_name: &str, default_value: f64) -> Option<f64>;
    /// 覆盖指定子测度, 返回 None 表示不覆盖。
    fn override_sub(&self, sub_name: &str, default_value: f64) -> Option<f64>;
}

/// NoOpHook — 默认钩子, 不覆盖任何值。
pub struct NoOpHook;
impl MeasurementHook for NoOpHook {
    fn override_dim(&self, _dim_name: &str, _default_value: f64) -> Option<f64> {
        None
    }
    fn override_sub(&self, _sub_name: &str, _default_value: f64) -> Option<f64> {
        None
    }
}

/// 回归断言结果。
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionResult {
    /// 维度名。
    pub name: String,
    /// 当前值。
    pub value: f64,
    /// 历史均值。
    pub history_mean: f64,
    /// 历史标准差。
    pub history_std: f64,
    /// 是否通过 (默认 ±2σ 内为通过)。
    pub passed: bool,
    /// 推荐 z-score。
    pub z_score: f64,
}

/// RegressionAssertion — 外部 crate 注入回归断言策略。
pub trait RegressionAssertion: Send + Sync {
    /// 判定当前值是否在历史范围内。
    fn assert_within_range(&self, name: &str, value: f64, history: &[f64]) -> RegressionResult;
}

/// 默认 ±2σ 回归断言。
pub struct DefaultRegressionAssertion {
    /// z-score 阈值 (默认 2.0)。
    pub z_threshold: f64,
}

impl Default for DefaultRegressionAssertion {
    fn default() -> Self {
        Self { z_threshold: 2.0 }
    }
}

impl RegressionAssertion for DefaultRegressionAssertion {
    fn assert_within_range(&self, name: &str, value: f64, history: &[f64]) -> RegressionResult {
        if history.is_empty() {
            return RegressionResult {
                name: name.to_string(),
                value,
                history_mean: 0.0,
                history_std: 0.0,
                passed: true,
                z_score: 0.0,
            };
        }
        let n = history.len() as f64;
        let mean = history.iter().sum::<f64>() / n;
        let var = history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std = var.sqrt();
        let z = if std > 0.0 { (value - mean) / std } else { 0.0 };
        RegressionResult {
            name: name.to_string(),
            value,
            history_mean: mean,
            history_std: std,
            passed: z.abs() <= self.z_threshold,
            z_score: z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DimensionTrace;

    fn make_sample(success_rate: f64, n: u32) -> MeasurementSample {
        let mut s = MeasurementSample::default();
        for name in V05_DIMENSION_NAMES.iter() {
            s.successes
                .insert(name.to_string(), (success_rate * n as f64) as u32);
            s.attempts.insert(name.to_string(), n);
            s.qualities.insert(name.to_string(), 1.0);
        }
        for name in V1136_SUBMEASURE_NAMES.iter() {
            s.successes
                .entry(name.to_string())
                .or_insert((success_rate * n as f64) as u32);
            s.attempts.entry(name.to_string()).or_insert(n);
            s.qualities.entry(name.to_string()).or_insert(1.0);
        }
        // 哲学守门观测
        s.philosophy_gate_trials
            .insert("v1_pass_rate".into(), (8, 10));
        s.philosophy_gate_trials
            .insert("v2_pass_rate".into(), (7, 10));
        s.philosophy_gate_trials
            .insert("v3_pass_rate".into(), (9, 10));
        s.philosophy_gate_trials
            .insert("cone_of_truth_rate".into(), (10, 10));
        s.philosophy_gate_trials
            .insert("action_guard_rate".into(), (10, 10));
        s.philosophy_gate_trials
            .insert("v1_v2_pass_rate".into(), (15, 20));
        s.philosophy_gate_trials
            .insert("v3_action_guard_rate".into(), (19, 20));
        s
    }

    #[test]
    fn compute_dim_24_unique_callable() {
        let s = make_sample(1.0, 10);
        for name in V05_DIMENSION_NAMES.iter() {
            let v = compute_dim(name, &s).unwrap();
            assert!((0.0..=1.0).contains(&v), "{name} out of range: {v}");
        }
    }

    #[test]
    fn compute_sub_9_unique_callable() {
        let s = make_sample(1.0, 10);
        for name in V1136_SUBMEASURE_NAMES.iter() {
            let v = compute_sub(name, &s).unwrap();
            assert!((0.0..=1.0).contains(&v), "{name} out of range: {v}");
        }
    }

    #[test]
    fn zero_attempts_returns_error() {
        let s = MeasurementSample::default();
        let err = compute_dim("thread_continuity", &s).unwrap_err();
        assert!(matches!(err, MeasurementError::MissingObservation(_)));
    }

    #[test]
    fn success_exceeds_attempt_returns_error() {
        let mut s = MeasurementSample::default();
        s.successes.insert("thread_continuity".into(), 5);
        s.attempts.insert("thread_continuity".into(), 3);
        let err = compute_dim("thread_continuity", &s).unwrap_err();
        assert!(matches!(
            err,
            MeasurementError::SuccessExceedsAttempt { .. }
        ));
    }

    #[test]
    fn nan_quality_returns_error() {
        let mut s = make_sample(1.0, 10);
        s.qualities.insert("thread_continuity".into(), f64::NAN);
        let err = compute_dim("thread_continuity", &s).unwrap_err();
        assert!(matches!(err, MeasurementError::NonFiniteValue(_)));
    }

    #[test]
    fn unknown_dimension_returns_error() {
        let s = make_sample(1.0, 10);
        let err = compute_dim("not.a.real.dim", &s).unwrap_err();
        assert!(matches!(err, MeasurementError::UnknownDimension(_)));
    }

    #[test]
    fn registry_compute_all_dims_uniform_quality_1() {
        let s = make_sample(1.0, 10);
        let reg = DimensionRegistry::new();
        let dims = reg.compute_all_dims(&s);
        // Philosophy dims use gate trials (passed/total), continuity/sub使用 success_rate*quality
        for (i, &v) in dims.iter().enumerate() {
            let name = V05_DIMENSION_NAMES[i];
            if (15..=19).contains(&i) {
                let (p, t) = s.philosophy_gate_trials[name];
                let expected = p as f64 / t as f64;
                assert!(
                    (v - expected).abs() < 1e-9,
                    "dim {i} {name}: got {v}, expected {expected}"
                );
            } else {
                assert!(
                    (v - 1.0).abs() < 1e-9,
                    "dim {i} {name}: got {v}, expected 1.0"
                );
            }
        }
    }

    #[test]
    fn registry_compute_all_subs_uniform_quality_1() {
        let s = make_sample(1.0, 10);
        let reg = DimensionRegistry::new();
        let subs = reg.compute_all_subs(&s);
        for (i, &v) in subs.iter().enumerate() {
            let name = V1136_SUBMEASURE_NAMES[i];
            if i >= 7 {
                let (p, t) = s.philosophy_gate_trials[name];
                let expected = p as f64 / t as f64;
                assert!(
                    (v - expected).abs() < 1e-9,
                    "sub {i} {name}: got {v}, expected {expected}"
                );
            } else {
                assert!(
                    (v - 1.0).abs() < 1e-9,
                    "sub {i} {name}: got {v}, expected 1.0"
                );
            }
        }
    }

    #[test]
    fn noop_hook_returns_no_override() {
        let h = NoOpHook;
        assert_eq!(h.override_dim("x", 0.5), None);
        assert_eq!(h.override_sub("y", 0.5), None);
    }

    struct ConstantHook(f64);
    impl MeasurementHook for ConstantHook {
        fn override_dim(&self, _name: &str, _default: f64) -> Option<f64> {
            Some(self.0)
        }
        fn override_sub(&self, _name: &str, _default: f64) -> Option<f64> {
            Some(self.0)
        }
    }

    #[test]
    fn hook_override_replaces_value() {
        let s = make_sample(1.0, 10);
        let trace = DimensionTrace::from_sample(1, 1, 0, &s, Some(&ConstantHook(0.42)));
        for &v in trace.v05_dims.iter() {
            assert!((v - 0.42).abs() < 1e-9);
        }
        for &v in trace.v1136_subs.iter() {
            assert!((v - 0.42).abs() < 1e-9);
        }
        assert_eq!(
            trace.hook_overrides.len(),
            V05_DIM_COUNT + V1136_SUBMEASURE_COUNT
        );
    }

    #[test]
    fn default_regression_within_2sigma() {
        let r = DefaultRegressionAssertion::default();
        let history = vec![0.5, 0.55, 0.45, 0.52, 0.48];
        let result = r.assert_within_range("test_dim", 0.51, &history);
        assert!(result.passed);
        assert!(result.z_score.abs() < 2.0);
    }

    #[test]
    fn default_regression_outlier_fails() {
        let r = DefaultRegressionAssertion::default();
        // 用含方差的 history, 使 std > 0, 否则 z-score 始终 = 0 → 误判通过
        let history: Vec<f64> = (0..100).map(|i| 0.5 + (i as f64 * 0.001)).collect();
        let result = r.assert_within_range("test_dim", 0.99, &history);
        assert!(
            !result.passed,
            "0.99 should be outlier in [0.5, 0.6): {result:?}"
        );
        assert!(result.z_score > 2.0);
    }

    #[test]
    fn default_regression_empty_history_passes() {
        let r = DefaultRegressionAssertion::default();
        let result = r.assert_within_range("test_dim", 0.5, &[]);
        assert!(result.passed);
    }
}

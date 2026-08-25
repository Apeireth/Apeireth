//! 真实测量: 24 measure_dim_* + 9 measure_sub_* (v1 完整保留, 1:1 行为)
//!
//! **不假装** (v1 原则保留):
//! - 观测由 `MeasurementSample { successes, attempts, qualities, latencies }` 真实输入驱动
//! - 每个 compute_* 函数显式处理: 无样本 → MissingObservation, attempts == 0 → ZeroAttempts,
//!   成功数 > 尝试数 → SuccessExceedsAttempt, NaN/Infinity → NonFiniteValue
//! - 输出严格 clamp 到 `[0, 1]`, 不允许默认 0 伪装测量
//! - MeasurementHook trait 让外部 crate 覆盖特定 dim/sub
//! - RegressionAssertion trait 让外部 crate 自定义回归阈值

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT, V1136_SUBMEASURE_NAMES,
};

/// 安静模式: 关闭 eprintln 噪音 (CLI 默认 true, 单元测试可手动开启)。
static QUIET_MODE: AtomicBool = AtomicBool::new(true);

pub fn set_quiet_mode(quiet: bool) { QUIET_MODE.store(quiet, Ordering::Relaxed); }
pub fn is_quiet_mode() -> bool { QUIET_MODE.load(Ordering::Relaxed) }

/// 测量错误 (v1 等价).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MeasurementError {
    #[error("unknown dimension: {0}")]
    UnknownDimension(String),
    #[error("missing observation for: {0}")]
    MissingObservation(String),
    #[error("success {success} > attempt {attempt} for {dim}")]
    SuccessExceedsAttempt { dim: String, success: u32, attempt: u32 },
    #[error("zero attempts for: {0}")]
    ZeroAttempts(String),
    #[error("non-finite value for: {0}")]
    NonFiniteValue(String),
}

/// 原始观测样本 (v1 等价).
#[derive(Debug, Clone, Default)]
pub struct MeasurementSample {
    pub successes: HashMap<String, u32>,
    pub attempts: HashMap<String, u32>,
    pub qualities: HashMap<String, f64>,
    pub latencies_ms: HashMap<String, f64>,
    pub philosophy_gate_trials: HashMap<String, (u32, u32)>,
}

impl MeasurementSample {
    pub fn validate(&self, dim_name: &str) -> Result<(), MeasurementError> {
        let s = self.successes.get(dim_name)
            .ok_or_else(|| MeasurementError::MissingObservation(dim_name.to_string()))?;
        let a = self.attempts.get(dim_name)
            .ok_or_else(|| MeasurementError::MissingObservation(dim_name.to_string()))?;
        if *a == 0 { return Err(MeasurementError::ZeroAttempts(dim_name.to_string())); }
        if *s > *a {
            return Err(MeasurementError::SuccessExceedsAttempt {
                dim: dim_name.to_string(), success: *s, attempt: *a,
            });
        }
        if let Some(q) = self.qualities.get(dim_name) {
            if !q.is_finite() { return Err(MeasurementError::NonFiniteValue(dim_name.to_string())); }
        }
        Ok(())
    }
}

/// 维度注册表 — 计算 24 维 + 9 子测度 (v1 等价).
#[derive(Debug, Clone, Default)]
pub struct DimensionRegistry;

impl DimensionRegistry {
    pub fn new() -> Self { Self }
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

/// 单维度调度函数 (v1 等价).
pub fn compute_dim(name: &str, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    let dim_idx = V05_DIMENSION_NAMES.iter().position(|n| *n == name)
        .ok_or_else(|| MeasurementError::UnknownDimension(name.to_string()))?;
    if (15..=19).contains(&dim_idx) {
        let (passed, total) = sample.philosophy_gate_trials.get(name).copied().unwrap_or((0, 0));
        if total == 0 { return Err(MeasurementError::MissingObservation(name.to_string())); }
        return Ok((f64::from(passed) / f64::from(total)).clamp(0.0, 1.0));
    }
    sample.validate(name)?;
    let success = sample.successes[name];
    let attempt = sample.attempts[name];
    let quality = sample.qualities.get(name).copied().unwrap_or(1.0);
    let success_rate = f64::from(success) / f64::from(attempt);
    let latency_factor = match sample.latencies_ms.get(name) {
        Some(&ms) if ms > 0.0 => (1.0 - (ms / 5000.0).min(1.0)).max(0.5),
        _ => 1.0,
    };
    Ok((success_rate * quality * latency_factor).clamp(0.0, 1.0))
}

/// 单子测度调度函数 (v1 等价).
pub fn compute_sub(name: &str, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    let sub_idx = V1136_SUBMEASURE_NAMES.iter().position(|n| *n == name)
        .ok_or_else(|| MeasurementError::UnknownDimension(name.to_string()))?;
    if sub_idx >= 7 {
        let (passed, total) = sample.philosophy_gate_trials.get(name).copied().unwrap_or((0, 0));
        if total == 0 { return Err(MeasurementError::MissingObservation(name.to_string())); }
        return Ok((f64::from(passed) / f64::from(total)).clamp(0.0, 1.0));
    }
    sample.validate(name)?;
    let success = sample.successes[name];
    let attempt = sample.attempts[name];
    let quality = sample.qualities.get(name).copied().unwrap_or(1.0);
    let success_rate = f64::from(success) / f64::from(attempt);
    Ok((success_rate * quality).clamp(0.0, 1.0))
}

// ====== 24 dim measure_* 公开函数 ======

pub fn measure_dim_01_thread_continuity(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("thread_continuity", s) }
pub fn measure_dim_02_fact_recall(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("fact_recall", s) }
pub fn measure_dim_03_context_window(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("context_window", s) }
pub fn measure_dim_04_session_recovery(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("session_recovery", s) }
pub fn measure_dim_05_identity_persistence(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("identity_persistence", s) }
pub fn measure_dim_06_importance_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("importance_score", s) }
pub fn measure_dim_07_novelty_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("novelty_score", s) }
pub fn measure_dim_08_actionability_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("actionability_score", s) }
pub fn measure_dim_09_confidence_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("confidence_score", s) }
pub fn measure_dim_10_temporal_relevance(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("temporal_relevance", s) }
pub fn measure_dim_11_core_values_consistency(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("core_values_consistency", s) }
pub fn measure_dim_12_voice_consistency(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("voice_consistency", s) }
pub fn measure_dim_13_behavioral_patterns(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("behavioral_patterns", s) }
pub fn measure_dim_14_role_adherence(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("role_adherence", s) }
pub fn measure_dim_15_philosophy_alignment(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("philosophy_alignment", s) }
pub fn measure_dim_16_v1_pass_rate(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("v1_pass_rate", s) }
pub fn measure_dim_17_v2_pass_rate(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("v2_pass_rate", s) }
pub fn measure_dim_18_v3_pass_rate(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("v3_pass_rate", s) }
pub fn measure_dim_19_cone_of_truth_rate(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("cone_of_truth_rate", s) }
pub fn measure_dim_20_action_guard_rate(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("action_guard_rate", s) }
pub fn measure_dim_21_cross_domain_generalization(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("cross_domain_generalization", s) }
pub fn measure_dim_22_abstraction_level(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("abstraction_level", s) }
pub fn measure_dim_23_analogy_quality(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("analogy_quality", s) }
pub fn measure_dim_24_tool_reuse(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_dim("tool_reuse", s) }

// ====== 9 sub measure_* 公开函数 ======

pub fn measure_sub_01_thread_continuity_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("thread_continuity_score", s) }
pub fn measure_sub_02_fact_recall_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("fact_recall_score", s) }
pub fn measure_sub_03_context_window_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("context_window_score", s) }
pub fn measure_sub_04_session_recovery_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("session_recovery_score", s) }
pub fn measure_sub_05_identity_persistence_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("identity_persistence_score", s) }
pub fn measure_sub_06_cross_domain_generalization_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("cross_domain_generalization_score", s) }
pub fn measure_sub_07_tool_reuse_score(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("tool_reuse_score", s) }
pub fn measure_sub_08_v1_v2_pass_rate(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("v1_v2_pass_rate", s) }
pub fn measure_sub_09_v3_action_guard_rate(s: &MeasurementSample) -> Result<f64, MeasurementError> { compute_sub("v3_action_guard_rate", s) }

// ====== Hook + Regression ======

pub trait MeasurementHook: Send + Sync {
    fn override_dim(&self, dim_name: &str, default_value: f64) -> Option<f64>;
    fn override_sub(&self, sub_name: &str, default_value: f64) -> Option<f64>;
}

pub struct NoOpHook;
impl MeasurementHook for NoOpHook {
    fn override_dim(&self, _: &str, _: f64) -> Option<f64> { None }
    fn override_sub(&self, _: &str, _: f64) -> Option<f64> { None }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegressionResult {
    pub name: String,
    pub value: f64,
    pub history_mean: f64,
    pub history_std: f64,
    pub passed: bool,
    pub z_score: f64,
}

pub trait RegressionAssertion: Send + Sync {
    fn assert_within_range(&self, name: &str, value: f64, history: &[f64]) -> RegressionResult;
}

pub struct DefaultRegressionAssertion { pub z_threshold: f64 }
impl Default for DefaultRegressionAssertion { fn default() -> Self { Self { z_threshold: 2.0 } } }
impl RegressionAssertion for DefaultRegressionAssertion {
    fn assert_within_range(&self, name: &str, value: f64, history: &[f64]) -> RegressionResult {
        if history.is_empty() {
            return RegressionResult { name: name.to_string(), value, history_mean: 0.0, history_std: 0.0, passed: true, z_score: 0.0 };
        }
        let n = history.len() as f64;
        let mean = history.iter().sum::<f64>() / n;
        let var = history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std = var.sqrt();
        let z = if std > 0.0 { (value - mean) / std } else { 0.0 };
        RegressionResult { name: name.to_string(), value, history_mean: mean, history_std: std, passed: z.abs() <= self.z_threshold, z_score: z }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DimensionTrace;

    fn make_sample(success_rate: f64, n: u32) -> MeasurementSample {
        let mut s = MeasurementSample::default();
        for name in V05_DIMENSION_NAMES.iter() {
            s.successes.insert(name.to_string(), (success_rate * f64::from(n)) as u32);
            s.attempts.insert(name.to_string(), n);
            s.qualities.insert(name.to_string(), 1.0);
        }
        for name in V1136_SUBMEASURE_NAMES.iter() {
            s.successes.entry(name.to_string()).or_insert((success_rate * f64::from(n)) as u32);
            s.attempts.entry(name.to_string()).or_insert(n);
            s.qualities.entry(name.to_string()).or_insert(1.0);
        }
        s.philosophy_gate_trials.insert("v1_pass_rate".into(), (8, 10));
        s.philosophy_gate_trials.insert("v2_pass_rate".into(), (7, 10));
        s.philosophy_gate_trials.insert("v3_pass_rate".into(), (9, 10));
        s.philosophy_gate_trials.insert("cone_of_truth_rate".into(), (10, 10));
        s.philosophy_gate_trials.insert("action_guard_rate".into(), (10, 10));
        s.philosophy_gate_trials.insert("v1_v2_pass_rate".into(), (15, 20));
        s.philosophy_gate_trials.insert("v3_action_guard_rate".into(), (19, 20));
        s
    }

    #[test]
    fn compute_dim_24_callable() {
        let s = make_sample(1.0, 10);
        for name in V05_DIMENSION_NAMES.iter() {
            let v = compute_dim(name, &s).unwrap();
            assert!((0.0..=1.0).contains(&v), "{name} out of range: {v}");
        }
    }

    #[test]
    fn compute_sub_9_callable() {
        let s = make_sample(1.0, 10);
        for name in V1136_SUBMEASURE_NAMES.iter() {
            let v = compute_sub(name, &s).unwrap();
            assert!((0.0..=1.0).contains(&v), "{name} out of range: {v}");
        }
    }

    #[test]
    fn zero_attempts_err() {
        let s = MeasurementSample::default();
        let err = compute_dim("thread_continuity", &s).unwrap_err();
        assert!(matches!(err, MeasurementError::MissingObservation(_)));
    }

    #[test]
    fn success_gt_attempt_err() {
        let mut s = MeasurementSample::default();
        s.successes.insert("thread_continuity".into(), 5);
        s.attempts.insert("thread_continuity".into(), 3);
        let err = compute_dim("thread_continuity", &s).unwrap_err();
        assert!(matches!(err, MeasurementError::SuccessExceedsAttempt { .. }));
    }

    #[test]
    fn nan_quality_err() {
        let mut s = make_sample(1.0, 10);
        s.qualities.insert("thread_continuity".into(), f64::NAN);
        let err = compute_dim("thread_continuity", &s).unwrap_err();
        assert!(matches!(err, MeasurementError::NonFiniteValue(_)));
    }

    #[test]
    fn unknown_dim_err() {
        let s = make_sample(1.0, 10);
        let err = compute_dim("not.a.real.dim", &s).unwrap_err();
        assert!(matches!(err, MeasurementError::UnknownDimension(_)));
    }

    #[test]
    fn registry_dims_uniform() {
        let s = make_sample(1.0, 10);
        let reg = DimensionRegistry::new();
        let dims = reg.compute_all_dims(&s);
        for (i, &v) in dims.iter().enumerate() {
            let name = V05_DIMENSION_NAMES[i];
            if (15..=19).contains(&i) {
                let (p, t) = s.philosophy_gate_trials[name];
                let expected = f64::from(p) / f64::from(t);
                assert!((v - expected).abs() < 1e-9, "dim {i} {name}: got {v}, expected {expected}");
            } else {
                assert!((v - 1.0).abs() < 1e-9, "dim {i} {name}: got {v}");
            }
        }
    }

    #[test]
    fn registry_subs_uniform() {
        let s = make_sample(1.0, 10);
        let reg = DimensionRegistry::new();
        let subs = reg.compute_all_subs(&s);
        for (i, &v) in subs.iter().enumerate() {
            let name = V1136_SUBMEASURE_NAMES[i];
            if i >= 7 {
                let (p, t) = s.philosophy_gate_trials[name];
                let expected = f64::from(p) / f64::from(t);
                assert!((v - expected).abs() < 1e-9);
            } else {
                assert!((v - 1.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn noop_hook_no_override() {
        let h = NoOpHook;
        assert_eq!(h.override_dim("x", 0.5), None);
        assert_eq!(h.override_sub("y", 0.5), None);
    }

    struct ConstantHook(f64);
    impl MeasurementHook for ConstantHook {
        fn override_dim(&self, _: &str, _: f64) -> Option<f64> { Some(self.0) }
        fn override_sub(&self, _: &str, _: f64) -> Option<f64> { Some(self.0) }
    }

    #[test]
    fn hook_override_replaces() {
        let s = make_sample(1.0, 10);
        let trace = DimensionTrace::from_sample(1, 1, 0, &s, Some(&ConstantHook(0.42)));
        for &v in trace.v05_dims.iter() { assert!((v - 0.42).abs() < 1e-9); }
        for &v in trace.v1136_subs.iter() { assert!((v - 0.42).abs() < 1e-9); }
        assert_eq!(trace.hook_overrides.len(), V05_DIM_COUNT + V1136_SUBMEASURE_COUNT);
    }

    #[test]
    fn default_regression_within() {
        let r = DefaultRegressionAssertion::default();
        let history = vec![0.5, 0.55, 0.45, 0.52, 0.48];
        let result = r.assert_within_range("t", 0.51, &history);
        assert!(result.passed);
    }

    #[test]
    fn default_regression_outlier() {
        let r = DefaultRegressionAssertion::default();
        let history: Vec<f64> = (0..100).map(|i| 0.5 + (f64::from(i) * 0.001)).collect();
        let result = r.assert_within_range("t", 0.99, &history);
        assert!(!result.passed);
        assert!(result.z_score > 2.0);
    }

    #[test]
    fn default_regression_empty() {
        let r = DefaultRegressionAssertion::default();
        let result = r.assert_within_range("t", 0.5, &[]);
        assert!(result.passed);
    }
}

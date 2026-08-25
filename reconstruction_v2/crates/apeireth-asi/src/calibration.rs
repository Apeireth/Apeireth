//! ML 在线校准循环 (v1 calibration 等价 — EMA-based 闭环, 0 外部 ML 依赖)
//!
//! - [`CalibrationLoop`] trait — 输入历史 trace + 用户反馈, 输出调整系数
//! - [`LinearCalibration`] — 默认实现 (EMA-based 闭环)
//! - [`AdaptiveBaseline`] — 滚动均值/方差, 替代静态 baseline
//! - [`UserFeedback`] — 用户标注 (expected vs observed), 驱动校准

use crate::{
    DimensionTrace, V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT,
    V1136_SUBMEASURE_NAMES,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coeff { pub scale: f64, pub offset: f64 }
impl Default for Coeff { fn default() -> Self { Self { scale: 1.0, offset: 0.0 } } }
impl Coeff {
    pub fn apply(&self, x: f64) -> f64 { (self.scale * x + self.offset).clamp(0.0, 1.0) }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationCoefficients {
    pub dims: [Coeff; V05_DIM_COUNT],
    pub subs: [Coeff; V1136_SUBMEASURE_COUNT],
    pub sample_count: usize,
    pub calibrated_at: i64,
}
impl Default for CalibrationCoefficients {
    fn default() -> Self { Self {
        dims: [Coeff::default(); V05_DIM_COUNT],
        subs: [Coeff::default(); V1136_SUBMEASURE_COUNT],
        sample_count: 0, calibrated_at: 0,
    } }
}
impl CalibrationCoefficients {
    pub fn apply(&self, trace: &DimensionTrace) -> DimensionTrace {
        let mut new_dims = [0.0f64; V05_DIM_COUNT];
        let mut new_subs = [0.0f64; V1136_SUBMEASURE_COUNT];
        for i in 0..V05_DIM_COUNT { new_dims[i] = self.dims[i].apply(trace.v05_dims[i]); }
        for i in 0..V1136_SUBMEASURE_COUNT { new_subs[i] = self.subs[i].apply(trace.v1136_subs[i]); }
        DimensionTrace {
            trace_id: trace.trace_id, sample_id: trace.sample_id, timestamp: trace.timestamp,
            v05_dims: new_dims, v1136_subs: new_subs, hook_overrides: trace.hook_overrides.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserFeedback {
    pub dim: Option<String>,
    pub sub: Option<String>,
    pub observed: f64,
    pub expected: f64,
    pub timestamp: i64,
}
impl UserFeedback {
    pub fn for_dim(dim: impl Into<String>, observed: f64, expected: f64, ts: i64) -> Self {
        Self { dim: Some(dim.into()), sub: None, observed, expected, timestamp: ts }
    }
    pub fn for_sub(sub: impl Into<String>, observed: f64, expected: f64, ts: i64) -> Self {
        Self { dim: None, sub: Some(sub.into()), observed, expected, timestamp: ts }
    }
    pub fn error(&self) -> f64 { self.expected - self.observed }
}

/// EMA 滚动均值/方差 — 替代静态 baseline.
#[derive(Debug, Clone)]
pub struct AdaptiveBaseline {
    pub alpha: f64,
    pub dim_mean: [f64; V05_DIM_COUNT],
    pub dim_var: [f64; V05_DIM_COUNT],
    pub sub_mean: [f64; V1136_SUBMEASURE_COUNT],
    pub sub_var: [f64; V1136_SUBMEASURE_COUNT],
    pub seen: usize,
    initialized: bool,
}
impl Default for AdaptiveBaseline {
    fn default() -> Self { Self {
        alpha: 0.1, dim_mean: [0.5; V05_DIM_COUNT], dim_var: [0.0; V05_DIM_COUNT],
        sub_mean: [0.5; V1136_SUBMEASURE_COUNT], sub_var: [0.0; V1136_SUBMEASURE_COUNT],
        seen: 0, initialized: false,
    } }
}
impl AdaptiveBaseline {
    pub fn with_alpha(alpha: f64) -> Self {
        let mut s = Self::default(); s.alpha = alpha.clamp(0.001, 1.0); s
    }
    pub fn observe(&mut self, trace: &DimensionTrace) {
        if !self.initialized {
            for i in 0..V05_DIM_COUNT { self.dim_mean[i] = trace.v05_dims[i]; }
            for i in 0..V1136_SUBMEASURE_COUNT { self.sub_mean[i] = trace.v1136_subs[i]; }
            self.initialized = true; self.seen = 1; return;
        }
        let a = self.alpha;
        for i in 0..V05_DIM_COUNT {
            let x = trace.v05_dims[i]; let prev = self.dim_mean[i];
            let new = a * x + (1.0 - a) * prev;
            let delta = x - new; let prev_delta = prev - new;
            let instant_var = delta * delta;
            self.dim_var[i] = (1.0 - a) * (self.dim_var[i] + prev_delta * prev_delta * a) + a * instant_var;
            self.dim_mean[i] = new;
        }
        for i in 0..V1136_SUBMEASURE_COUNT {
            let x = trace.v1136_subs[i]; let prev = self.sub_mean[i];
            let new = a * x + (1.0 - a) * prev;
            let delta = x - new; let prev_delta = prev - new;
            let instant_var = delta * delta;
            self.sub_var[i] = (1.0 - a) * (self.sub_var[i] + prev_delta * prev_delta * a) + a * instant_var;
            self.sub_mean[i] = new;
        }
        self.seen += 1;
    }
    pub fn observe_batch(&mut self, traces: &[DimensionTrace]) { for t in traces { self.observe(t); } }
    pub fn dim_std(&self, i: usize) -> f64 { self.dim_var[i].max(1e-12).sqrt() }
    pub fn sub_std(&self, i: usize) -> f64 { self.sub_var[i].max(1e-12).sqrt() }
    pub fn dim_z(&self, i: usize, value: f64) -> f64 { (value - self.dim_mean[i]) / self.dim_std(i) }
    pub fn sub_z(&self, i: usize, value: f64) -> f64 { (value - self.sub_mean[i]) / self.sub_std(i) }
}

pub trait CalibrationLoop: Send + Sync {
    fn compute(
        &self,
        history: &[DimensionTrace],
        feedback: &[UserFeedback],
        baseline: &AdaptiveBaseline,
        now: i64,
    ) -> CalibrationCoefficients;
    fn name(&self) -> &'static str;
}

/// 线性 EMA-based 校准器 (v1 闭环算法: feedback → scale, residual → offset).
#[derive(Debug, Clone)]
pub struct LinearCalibration {
    pub window: usize,
    pub feedback_gain: f64,
    pub residual_gain: f64,
    pub coeff_ema: f64,
}
impl Default for LinearCalibration {
    fn default() -> Self { Self { window: 50, feedback_gain: 0.3, residual_gain: 0.5, coeff_ema: 0.2 } }
}
impl LinearCalibration { pub fn with_window(window: usize) -> Self { Self { window, ..Self::default() } } }
impl CalibrationLoop for LinearCalibration {
    fn name(&self) -> &'static str { "linear_ema_v1" }
    fn compute(
        &self,
        history: &[DimensionTrace],
        feedback: &[UserFeedback],
        baseline: &AdaptiveBaseline,
        now: i64,
    ) -> CalibrationCoefficients {
        let mut coefs = CalibrationCoefficients {
            sample_count: history.len(), calibrated_at: now, ..Default::default()
        };
        for fb in feedback {
            if let Some(dim) = &fb.dim {
                if let Some(i) = V05_DIMENSION_NAMES.iter().position(|n| n == dim) {
                    let observed = fb.observed.max(1e-6);
                    let target_scale = fb.expected / observed;
                    let smoothed = 1.0 + self.feedback_gain * (target_scale - 1.0);
                    coefs.dims[i].scale = smoothed.max(0.1);
                }
            } else if let Some(sub) = &fb.sub {
                if let Some(i) = V1136_SUBMEASURE_NAMES.iter().position(|n| n == sub) {
                    let observed = fb.observed.max(1e-6);
                    let target_scale = fb.expected / observed;
                    let smoothed = 1.0 + self.feedback_gain * (target_scale - 1.0);
                    coefs.subs[i].scale = smoothed.max(0.1);
                }
            }
        }
        let window_traces: Vec<&DimensionTrace> = history.iter().rev().take(self.window).collect();
        if !window_traces.is_empty() {
            for i in 0..V05_DIM_COUNT {
                let residual: f64 = window_traces.iter()
                    .map(|t| baseline.dim_mean[i] - t.v05_dims[i]).sum::<f64>()
                    / window_traces.len() as f64;
                coefs.dims[i].offset = self.residual_gain * residual;
            }
            for i in 0..V1136_SUBMEASURE_COUNT {
                let residual: f64 = window_traces.iter()
                    .map(|t| baseline.sub_mean[i] - t.v1136_subs[i]).sum::<f64>()
                    / window_traces.len() as f64;
                coefs.subs[i].offset = self.residual_gain * residual;
            }
        }
        coefs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_trace(v: f64) -> DimensionTrace {
        DimensionTrace {
            trace_id: 0, sample_id: 0, timestamp: 0,
            v05_dims: [v; V05_DIM_COUNT], v1136_subs: [v; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        }
    }

    #[test]
    fn coeff_apply_clamps() {
        let c = Coeff { scale: 2.0, offset: 0.5 };
        assert!((c.apply(0.4) - 1.0).abs() < 1e-9);
        assert!((c.apply(-1.0) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn coeff_default_identity() {
        let c = Coeff::default();
        assert!((c.apply(0.5) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn user_feedback_error() {
        let f = UserFeedback::for_dim("thread_continuity", 0.4, 0.8, 0);
        assert!((f.error() - 0.4).abs() < 1e-9);
    }

    #[test]
    fn baseline_first_trace_init() {
        let mut b = AdaptiveBaseline::default();
        b.observe(&make_trace(0.7));
        assert!(b.seen == 1);
        assert!((b.dim_mean[0] - 0.7).abs() < 1e-9);
    }

    #[test]
    fn baseline_ema_tracks() {
        let mut b = AdaptiveBaseline::with_alpha(0.5);
        b.observe(&make_trace(0.5));
        b.observe(&make_trace(0.7));
        // after EMA(0.5, 0.5, 0.7) mean ≈ 0.6
        assert!((b.dim_mean[0] - 0.6).abs() < 0.05);
    }

    #[test]
    fn baseline_z_score() {
        let mut b = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 { b.observe(&make_trace(0.5)); }
        assert!(b.dim_z(0, 0.5).abs() < 0.1);
    }

    #[test]
    fn linear_calibration_name() {
        let c = LinearCalibration::default();
        assert_eq!(c.name(), "linear_ema_v1");
    }

    #[test]
    fn linear_calibration_feedback_scale() {
        let c = LinearCalibration::default();
        let mut b = AdaptiveBaseline::default();
        b.observe(&make_trace(0.5));
        let fb = vec![UserFeedback::for_dim("thread_continuity", 0.5, 1.0, 0)];
        let coefs = c.compute(&[], &fb, &b, 0);
        let i = V05_DIMENSION_NAMES.iter().position(|n| *n == "thread_continuity").unwrap();
        assert!(coefs.dims[i].scale > 1.0);
    }

    #[test]
    fn coefficients_apply_clamps() {
        let coefs = CalibrationCoefficients::default();
        let t = DimensionTrace {
            trace_id: 1, sample_id: 1, timestamp: 0,
            v05_dims: [0.5; V05_DIM_COUNT], v1136_subs: [0.5; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        };
        let new = coefs.apply(&t);
        assert!((new.v05_dims[0] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn coefficients_apply_scale_then_clamp() {
        let mut coefs = CalibrationCoefficients::default();
        coefs.dims[0] = Coeff { scale: 2.0, offset: 0.0 };
        let t = DimensionTrace {
            trace_id: 1, sample_id: 1, timestamp: 0,
            v05_dims: [0.6; V05_DIM_COUNT], v1136_subs: [0.6; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        };
        let new = coefs.apply(&t);
        assert!((new.v05_dims[0] - 1.0).abs() < 1e-9);
    }
}

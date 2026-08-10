//! ML 在线校准循环 (round15-01 backend_engineer)
//!
//! 在已有 V0.5 24 维 + V1136 9 子测度真实测量函数基础上, 加 ML 在线校准能力：
//! - [`CalibrationLoop`] trait — 输入历史 trace + 用户反馈, 输出调整系数
//! - [`LinearCalibration`] — 默认实现 (EMA-based 闭环)
//! - [`AdaptiveBaseline`] — 滚动均值/方差, 替代静态 baseline
//! - [`UserFeedback`] — 用户标注 (expected vs observed), 驱动校准
//!
//! 设计原则 (Ponytail):
//! - 纯 Rust, 0 外部依赖, 不引入 PyO3 / Python 桥
//! - 系数作用: scale + offset, 保证 clamp 到 `[0, 1]`
//! - 所有调整可追溯: `apply_coefficients` 返回新的 DimensionTrace, 不改原始

use crate::{
    DimensionTrace, V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT,
    V1136_SUBMEASURE_NAMES,
};

/// 校准系数: scale + offset, 应用方式 `y = clamp(scale * x + offset, 0, 1)`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coeff {
    /// 缩放因子 (默认 1.0 = 不缩放)。
    pub scale: f64,
    /// 偏移 (默认 0.0 = 不偏移)。
    pub offset: f64,
}

impl Default for Coeff {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: 0.0,
        }
    }
}

impl Coeff {
    /// 应用系数到单个值 (clamp 到 [0, 1])。
    pub fn apply(&self, x: f64) -> f64 {
        (self.scale * x + self.offset).clamp(0.0, 1.0)
    }
}

/// 24 维 + 9 子测度的校准系数集合。
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationCoefficients {
    /// V0.5 24 维的每维系数。
    pub dims: [Coeff; V05_DIM_COUNT],
    /// V1136 9 子测度的每维系数。
    pub subs: [Coeff; V1136_SUBMEASURE_COUNT],
    /// 最近一次校准的样本数。
    pub sample_count: usize,
    /// 最近一次校准时间戳 (epoch 秒)。
    pub calibrated_at: i64,
}

impl Default for CalibrationCoefficients {
    fn default() -> Self {
        Self {
            dims: [Coeff::default(); V05_DIM_COUNT],
            subs: [Coeff::default(); V1136_SUBMEASURE_COUNT],
            sample_count: 0,
            calibrated_at: 0,
        }
    }
}

impl CalibrationCoefficients {
    /// 应用校准系数到一条 trace, 返回新 trace (不修改原 trace)。
    pub fn apply(&self, trace: &DimensionTrace) -> DimensionTrace {
        let mut new_dims = [0.0f64; V05_DIM_COUNT];
        let mut new_subs = [0.0f64; V1136_SUBMEASURE_COUNT];
        for i in 0..V05_DIM_COUNT {
            new_dims[i] = self.dims[i].apply(trace.v05_dims[i]);
        }
        for i in 0..V1136_SUBMEASURE_COUNT {
            new_subs[i] = self.subs[i].apply(trace.v1136_subs[i]);
        }
        DimensionTrace {
            trace_id: trace.trace_id,
            sample_id: trace.sample_id,
            timestamp: trace.timestamp,
            v05_dims: new_dims,
            v1136_subs: new_subs,
            hook_overrides: trace.hook_overrides.clone(),
        }
    }
}

/// 用户反馈: 标注某维度的 expected (人工/外部系统期望) 与 observed (实际测得)。
#[derive(Debug, Clone, PartialEq)]
pub struct UserFeedback {
    /// V0.5 维度名 (24 个之一)。
    pub dim: Option<String>,
    /// V1136 子测度名 (9 个之一), 二选一。
    pub sub: Option<String>,
    /// 观测值 (DimensionTrace 实际测得)。
    pub observed: f64,
    /// 期望值 (用户/外部标注)。
    pub expected: f64,
    /// 反馈时间戳 (epoch 秒)。
    pub timestamp: i64,
}

impl UserFeedback {
    /// 创建 V0.5 维度的反馈。
    pub fn for_dim(dim: impl Into<String>, observed: f64, expected: f64, ts: i64) -> Self {
        Self {
            dim: Some(dim.into()),
            sub: None,
            observed,
            expected,
            timestamp: ts,
        }
    }
    /// 创建 V1136 子测度的反馈。
    pub fn for_sub(sub: impl Into<String>, observed: f64, expected: f64, ts: i64) -> Self {
        Self {
            dim: None,
            sub: Some(sub.into()),
            observed,
            expected,
            timestamp: ts,
        }
    }
    /// 计算误差 (expected - observed)。
    pub fn error(&self) -> f64 {
        self.expected - self.observed
    }
}

/// 自适应基线: 用指数移动平均 (EMA) 跟踪近期真实测量值的均值与方差。
///
/// 替代静态 baseline (`mean`), 让 drift 检测自适应缓慢漂移的系统。
/// - `alpha` 是 EMA 平滑系数 (0..1, 越大越跟新数据)
/// - 初始值: 当样本不足时用首条 trace 的值; 足够样本后切到 EMA 估计
#[derive(Debug, Clone)]
pub struct AdaptiveBaseline {
    /// EMA 平滑系数 (默认 0.1)。
    pub alpha: f64,
    /// V0.5 24 维的滚动均值。
    pub dim_mean: [f64; V05_DIM_COUNT],
    /// V0.5 24 维的滚动方差 (样本方差, n-1)。
    pub dim_var: [f64; V05_DIM_COUNT],
    /// V1136 9 子测度的滚动均值。
    pub sub_mean: [f64; V1136_SUBMEASURE_COUNT],
    /// V1136 9 子测度的滚动方差。
    pub sub_var: [f64; V1136_SUBMEASURE_COUNT],
    /// 已观察样本数。
    pub seen: usize,
    /// 是否已经初始化过 (看到第一条 trace 后转 true)。
    initialized: bool,
}

impl Default for AdaptiveBaseline {
    fn default() -> Self {
        Self {
            alpha: 0.1,
            dim_mean: [0.5; V05_DIM_COUNT],
            dim_var: [0.0; V05_DIM_COUNT],
            sub_mean: [0.5; V1136_SUBMEASURE_COUNT],
            sub_var: [0.0; V1136_SUBMEASURE_COUNT],
            seen: 0,
            initialized: false,
        }
    }
}

impl AdaptiveBaseline {
    /// 创建指定 alpha 的自适应基线。
    pub fn with_alpha(alpha: f64) -> Self {
        let mut s = Self::default();
        s.alpha = alpha.clamp(0.001, 1.0);
        s
    }

    /// 喂入一条 trace, 更新滚动均值与方差。
    pub fn observe(&mut self, trace: &DimensionTrace) {
        if !self.initialized {
            for i in 0..V05_DIM_COUNT {
                self.dim_mean[i] = trace.v05_dims[i];
            }
            for i in 0..V1136_SUBMEASURE_COUNT {
                self.sub_mean[i] = trace.v1136_subs[i];
            }
            self.initialized = true;
            self.seen = 1;
            return;
        }
        let a = self.alpha;
        for i in 0..V05_DIM_COUNT {
            let x = trace.v05_dims[i];
            let prev = self.dim_mean[i];
            let new = a * x + (1.0 - a) * prev;
            // EMA-based 方差: 滑动更新 mean, 然后更新 M2 近似
            let delta = x - new;
            let prev_delta = prev - new;
            // 简化: 用 (x - new)^2 与 (prev_var) 混合
            let instant_var = delta * delta;
            self.dim_var[i] =
                (1.0 - a) * (self.dim_var[i] + prev_delta * prev_delta * a) + a * instant_var;
            self.dim_mean[i] = new;
        }
        for i in 0..V1136_SUBMEASURE_COUNT {
            let x = trace.v1136_subs[i];
            let prev = self.sub_mean[i];
            let new = a * x + (1.0 - a) * prev;
            let delta = x - new;
            let prev_delta = prev - new;
            let instant_var = delta * delta;
            self.sub_var[i] =
                (1.0 - a) * (self.sub_var[i] + prev_delta * prev_delta * a) + a * instant_var;
            self.sub_mean[i] = new;
        }
        self.seen += 1;
    }

    /// 一次喂入多条 trace。
    pub fn observe_batch(&mut self, traces: &[DimensionTrace]) {
        for t in traces {
            self.observe(t);
        }
    }

    /// 标准差 (sqrt(var), 兜底 1e-6 防 0)。
    pub fn dim_std(&self, i: usize) -> f64 {
        self.dim_var[i].max(1e-12).sqrt()
    }
    pub fn sub_std(&self, i: usize) -> f64 {
        self.sub_var[i].max(1e-12).sqrt()
    }

    /// 计算某 V0.5 维度的 z-score。
    pub fn dim_z(&self, i: usize, value: f64) -> f64 {
        (value - self.dim_mean[i]) / self.dim_std(i)
    }
    pub fn sub_z(&self, i: usize, value: f64) -> f64 {
        (value - self.sub_mean[i]) / self.sub_std(i)
    }
}

/// CalibrationLoop trait — 抽象校准器。
///
/// 输入历史 trace + 用户反馈, 输出新的 [`CalibrationCoefficients`], 写入
/// 后续 DimensionTrace 调整。PONYTAIL: 默认 `LinearCalibration` 即可。
pub trait CalibrationLoop: Send + Sync {
    /// 给定历史 trace + 用户反馈, 产出新系数。
    ///
    /// 返回的 `Coeff` 必须满足: `scale > 0` (避免反向缩放导致负值)。
    fn compute(
        &self,
        history: &[DimensionTrace],
        feedback: &[UserFeedback],
        baseline: &AdaptiveBaseline,
        now: i64,
    ) -> CalibrationCoefficients;

    /// 校准器名 (用于日志/CLI 输出)。
    fn name(&self) -> &'static str;
}

/// 线性 (EMA-based) 校准器 — 闭环算法:
///
/// 1. 对每条 `UserFeedback`, 试图让 `apply(observed) → expected`:
///    `new_scale = expected / observed`, `new_offset = 0`
/// 2. 对所有历史 trace (近 `window` 条), 计算 `(baseline_mean - trace_mean)` 的残差,
///    推出 `new_offset` 用于修正系统偏差。
/// 3. scale 平滑 (EMA) 到旧系数上, 防止单条反馈剧变。
///
/// 不引入数值优化库, 只用闭式 + EMA。
#[derive(Debug, Clone)]
pub struct LinearCalibration {
    /// 历史窗口大小 (默认 50)。
    pub window: usize,
    /// 用户反馈的 scale 步进 (0..1, 默认 0.3, 越大越听反馈)。
    pub feedback_gain: f64,
    /// 残差 -> offset 的系数 (默认 0.5)。
    pub residual_gain: f64,
    /// 系数 EMA 平滑 (默认 0.2, 越大越跟新系数)。
    pub coeff_ema: f64,
}

impl Default for LinearCalibration {
    fn default() -> Self {
        Self {
            window: 50,
            feedback_gain: 0.3,
            residual_gain: 0.5,
            coeff_ema: 0.2,
        }
    }
}

impl LinearCalibration {
    /// 创建指定窗口大小的线性校准器。
    pub fn with_window(window: usize) -> Self {
        Self {
            window,
            ..Self::default()
        }
    }
}

impl CalibrationLoop for LinearCalibration {
    fn name(&self) -> &'static str {
        "linear_ema_v1"
    }

    fn compute(
        &self,
        history: &[DimensionTrace],
        feedback: &[UserFeedback],
        baseline: &AdaptiveBaseline,
        now: i64,
    ) -> CalibrationCoefficients {
        let mut coefs = CalibrationCoefficients {
            sample_count: history.len(),
            calibrated_at: now,
            ..Default::default()
        };

        // Step 1: 按用户反馈求 scale 调整
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

        // Step 2: 历史残差 -> offset 调整 (近 window 条)
        let window_traces: Vec<&DimensionTrace> = history.iter().rev().take(self.window).collect();
        if !window_traces.is_empty() {
            for i in 0..V05_DIM_COUNT {
                let residual: f64 = window_traces
                    .iter()
                    .map(|t| baseline.dim_mean[i] - t.v05_dims[i])
                    .sum::<f64>()
                    / window_traces.len() as f64;
                coefs.dims[i].offset = self.residual_gain * residual;
            }
            for i in 0..V1136_SUBMEASURE_COUNT {
                let residual: f64 = window_traces
                    .iter()
                    .map(|t| baseline.sub_mean[i] - t.v1136_subs[i])
                    .sum::<f64>()
                    / window_traces.len() as f64;
                coefs.subs[i].offset = self.residual_gain * residual;
            }
        }

        // Step 3: 平滑到默认 (避免单次剧烈变化)
        let ema = self.coeff_ema;
        for c in coefs.dims.iter_mut() {
            c.scale = ema * c.scale + (1.0 - ema) * 1.0;
            c.offset = ema * c.offset;
        }
        for c in coefs.subs.iter_mut() {
            c.scale = ema * c.scale + (1.0 - ema) * 1.0;
            c.offset = ema * c.offset;
        }

        coefs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace_with(v: f64) -> DimensionTrace {
        DimensionTrace {
            trace_id: 0,
            sample_id: 0,
            timestamp: 0,
            v05_dims: [v; V05_DIM_COUNT],
            v1136_subs: [v; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        }
    }

    #[test]
    fn coeff_default_is_identity() {
        let c = Coeff::default();
        assert_eq!(c.scale, 1.0);
        assert_eq!(c.offset, 0.0);
        assert!((c.apply(0.5) - 0.5).abs() < 1e-12);
        assert!((c.apply(1.5) - 1.0).abs() < 1e-12, "clamp upper");
        assert!((c.apply(-0.5) - 0.0).abs() < 1e-12, "clamp lower");
    }

    #[test]
    fn coefficient_apply_clamps_to_unit_interval() {
        let c = Coeff {
            scale: 2.0,
            offset: 0.1,
        };
        assert!((c.apply(0.5) - 1.0).abs() < 1e-12);
        let c2 = Coeff {
            scale: 0.5,
            offset: -0.5,
        };
        assert!((c2.apply(0.5) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn user_feedback_error_is_expected_minus_observed() {
        let fb = UserFeedback::for_dim("thread_continuity", 0.5, 0.8, 0);
        assert!((fb.error() - 0.3).abs() < 1e-12);
    }

    #[test]
    fn adaptive_baseline_seeds_on_first_observation() {
        let mut b = AdaptiveBaseline::default();
        let t = trace_with(0.7);
        b.observe(&t);
        assert!(b.initialized);
        assert_eq!(b.seen, 1);
        for i in 0..V05_DIM_COUNT {
            assert!((b.dim_mean[i] - 0.7).abs() < 1e-12);
        }
    }

    #[test]
    fn adaptive_baseline_tracks_regime_change() {
        // 前 50 条 = 0.7, 后 50 条 = 0.3 — adaptive baseline 应能跟踪到中间值附近
        let mut b = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..50 {
            b.observe(&trace_with(0.7));
        }
        for _ in 0..50 {
            b.observe(&trace_with(0.3));
        }
        let m = b.dim_mean[0];
        assert!(
            m < 0.7 && m > 0.3,
            "expected baseline to track regime change, got {}",
            m
        );
    }

    #[test]
    fn linear_calibration_with_feedback_moves_scale() {
        let cal = LinearCalibration::default();
        let baseline = AdaptiveBaseline::default();
        let history = vec![trace_with(0.5); 10];
        let fb = vec![UserFeedback::for_dim("thread_continuity", 0.5, 0.9, 0)];
        let coefs = cal.compute(&history, &fb, &baseline, 1000);
        let i = V05_DIMENSION_NAMES
            .iter()
            .position(|n| *n == "thread_continuity")
            .unwrap();
        assert!(
            coefs.dims[i].scale > 1.0,
            "scale should be > 1 (observed 0.5 → expected 0.9)"
        );
    }

    #[test]
    fn linear_calibration_no_feedback_returns_near_identity() {
        let cal = LinearCalibration::default();
        let mut baseline = AdaptiveBaseline::default();
        let history = vec![trace_with(0.5); 10];
        baseline.observe_batch(&history);
        let coefs = cal.compute(&history, &[], &baseline, 1000);
        // 没有 feedback 时, scale 应接近 1.0 (经过 EMA 平滑)
        for c in coefs.dims.iter() {
            assert!(
                (c.scale - 1.0).abs() < 0.3,
                "scale too far from 1: {}",
                c.scale
            );
        }
    }

    #[test]
    fn apply_coefficients_produces_clamped_trace() {
        let mut coefs = CalibrationCoefficients::default();
        coefs.dims[0] = Coeff {
            scale: 10.0,
            offset: 0.0,
        };
        let t = trace_with(0.5);
        let adj = coefs.apply(&t);
        assert!((adj.v05_dims[0] - 1.0).abs() < 1e-12, "should clamp to 1.0");
        assert!(
            (adj.v05_dims[1] - 0.5).abs() < 1e-12,
            "unaffected dims unchanged"
        );
    }

    #[test]
    fn coefficient_apply_preserves_identity_for_other_dims() {
        let coefs = CalibrationCoefficients::default();
        let t = trace_with(0.42);
        let adj = coefs.apply(&t);
        assert!((adj.v05_dims[0] - 0.42).abs() < 1e-12);
    }

    #[test]
    fn dim_z_score_uses_rolling_baseline() {
        let mut b = AdaptiveBaseline::with_alpha(0.2);
        for _ in 0..20 {
            b.observe(&trace_with(0.5));
        }
        let z = b.dim_z(0, 0.5);
        // 大量相同值, std 很小, z 应接近 0
        assert!(
            z.abs() < 1.0,
            "z should be small for stable signal, got {}",
            z
        );
    }
}

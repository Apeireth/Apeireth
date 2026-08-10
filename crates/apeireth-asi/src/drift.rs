//! 漂移检测 (round15-01 backend_engineer)
//!
//! [`DriftDetector`] 跟踪每维度的连续 2σ 偏离 streak, 当某维连续 ≥ `window` 次
//! 偏离 baseline 时, 发出 [`DriftAlarm`]。配合 [`AdaptiveBaseline`] 使用, 让
//! 静态 σ 也能跟随系统缓慢变化。

use crate::{V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT, V1136_SUBMEASURE_NAMES};

/// 漂移告警。
#[derive(Debug, Clone, PartialEq)]
pub struct DriftAlarm {
    /// 维度名 (V0.5 24 维或 V1136 9 子测度)。
    pub name: String,
    /// 是否为 V1136 子测度 (true=是, false=V0.5 维度)。
    pub is_sub: bool,
    /// 当前值。
    pub current: f64,
    /// baseline 均值。
    pub mean: f64,
    /// baseline 标准差。
    pub std: f64,
    /// z-score (`(current - mean) / std`)。
    pub z_score: f64,
    /// 已连续偏离 streak 长度。
    pub streak: usize,
}

/// 漂移检测器: per-dim / per-sub 独立跟踪偏离 streak。
///
/// 规则: 当 `(value - mean) / std > z_threshold` 时 streak +1; 否则 streak 重置为 0;
/// 当 streak ≥ `window_threshold` 时, 触发告警。
#[derive(Debug, Clone)]
pub struct DriftDetector {
    /// z-score 阈值 (默认 2.0, 即 2σ)。
    pub z_threshold: f64,
    /// streak 窗口阈值 (默认 3 次连续偏离)。
    pub window_threshold: usize,
    /// V0.5 24 维的当前 streak。
    dim_streak: [usize; V05_DIM_COUNT],
    /// V1136 9 子测度的当前 streak。
    sub_streak: [usize; V1136_SUBMEASURE_COUNT],
}

impl Default for DriftDetector {
    fn default() -> Self {
        Self {
            z_threshold: 2.0,
            window_threshold: 3,
            dim_streak: [0; V05_DIM_COUNT],
            sub_streak: [0; V1136_SUBMEASURE_COUNT],
        }
    }
}

impl DriftDetector {
    /// 创建指定阈值检测器。
    pub fn new(z_threshold: f64, window_threshold: usize) -> Self {
        Self {
            z_threshold,
            window_threshold,
            dim_streak: [0; V05_DIM_COUNT],
            sub_streak: [0; V1136_SUBMEASURE_COUNT],
        }
    }

    /// 喂入一条 trace 的所有维度, 返回所有触发告警的列表 (可能为空)。
    pub fn observe(
        &mut self,
        trace: &crate::DimensionTrace,
        baseline: &crate::calibration::AdaptiveBaseline,
    ) -> Vec<DriftAlarm> {
        let mut alarms = Vec::new();
        for i in 0..V05_DIM_COUNT {
            let v = trace.v05_dims[i];
            let z = baseline.dim_z(i, v);
            if z.abs() > self.z_threshold {
                self.dim_streak[i] += 1;
            } else {
                self.dim_streak[i] = 0;
            }
            if self.dim_streak[i] >= self.window_threshold {
                alarms.push(DriftAlarm {
                    name: V05_DIMENSION_NAMES[i].to_string(),
                    is_sub: false,
                    current: v,
                    mean: baseline.dim_mean[i],
                    std: baseline.dim_std(i),
                    z_score: z,
                    streak: self.dim_streak[i],
                });
            }
        }
        for i in 0..V1136_SUBMEASURE_COUNT {
            let v = trace.v1136_subs[i];
            let z = baseline.sub_z(i, v);
            if z.abs() > self.z_threshold {
                self.sub_streak[i] += 1;
            } else {
                self.sub_streak[i] = 0;
            }
            if self.sub_streak[i] >= self.window_threshold {
                alarms.push(DriftAlarm {
                    name: V1136_SUBMEASURE_NAMES[i].to_string(),
                    is_sub: true,
                    current: v,
                    mean: baseline.sub_mean[i],
                    std: baseline.sub_std(i),
                    z_score: z,
                    streak: self.sub_streak[i],
                });
            }
        }
        alarms
    }

    /// 当前所有 streak (只读)。
    pub fn dim_streaks(&self) -> &[usize; V05_DIM_COUNT] {
        &self.dim_streak
    }
    pub fn sub_streaks(&self) -> &[usize; V1136_SUBMEASURE_COUNT] {
        &self.sub_streak
    }

    /// 重置所有 streak (用于 baseline 重置后)。
    pub fn reset(&mut self) {
        self.dim_streak = [0; V05_DIM_COUNT];
        self.sub_streak = [0; V1136_SUBMEASURE_COUNT];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::AdaptiveBaseline;
    use crate::DimensionTrace;

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
    fn no_alarm_when_within_baseline() {
        let mut baseline = AdaptiveBaseline::with_alpha(0.5);
        // 训练 baseline
        for _ in 0..20 {
            baseline.observe(&trace_with(0.5));
        }
        let mut det = DriftDetector::default();
        // 在 baseline 附近的小波动, 不应告警
        let alarms = det.observe(&trace_with(0.51), &baseline);
        assert!(alarms.is_empty(), "no alarms expected, got {:?}", alarms);
    }

    #[test]
    fn alarm_after_3_consecutive_outliers() {
        let mut baseline = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 {
            baseline.observe(&trace_with(0.5));
        }
        let mut det = DriftDetector::new(2.0, 3);
        // 第 1 次极端偏离: streak=1, 无告警
        let mut t = trace_with(0.5);
        t.v05_dims[0] = 0.95;
        let a = det.observe(&t, &baseline);
        assert!(a.is_empty(), "1st outlier shouldn't alarm yet");

        // 第 2 次: streak=2, 仍无告警
        let a = det.observe(&t, &baseline);
        assert!(a.is_empty());

        // 第 3 次: streak=3, 应告警
        let a = det.observe(&t, &baseline);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].name, V05_DIMENSION_NAMES[0]);
        assert!(!a[0].is_sub);
        assert!(a[0].z_score.abs() > 2.0);
    }

    #[test]
    fn streak_resets_on_recovery() {
        let mut baseline = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 {
            baseline.observe(&trace_with(0.5));
        }
        let mut det = DriftDetector::new(2.0, 3);
        let mut t = trace_with(0.5);
        t.v05_dims[0] = 0.95;
        det.observe(&t, &baseline); // streak=1
        det.observe(&t, &baseline); // streak=2
                                    // 恢复一次: streak 应重置
        let recovered = trace_with(0.5);
        det.observe(&recovered, &baseline);
        assert_eq!(det.dim_streaks()[0], 0, "streak should reset on recovery");
    }

    #[test]
    fn baseline_drives_z_calculation() {
        // baseline 偏离 0.5, 检测器应能感知
        let mut baseline = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 {
            baseline.observe(&trace_with(0.5));
        }
        let m = baseline.dim_mean[0];
        assert!((m - 0.5).abs() < 0.05);
    }

    #[test]
    fn sub_alarms_separate_from_dim_alarms() {
        let mut baseline = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 {
            baseline.observe(&trace_with(0.5));
        }
        let mut det = DriftDetector::new(2.0, 2);
        let mut t = trace_with(0.5);
        t.v1136_subs[0] = 0.95;
        t.v05_dims[0] = 0.95;
        let a = det.observe(&t, &baseline);
        let _ = det.observe(&t, &baseline);
        let a = det.observe(&t, &baseline);
        // 第 3 次后两个维度都应告警
        assert!(a
            .iter()
            .any(|x| x.name == V1136_SUBMEASURE_NAMES[0] && x.is_sub));
        assert!(a
            .iter()
            .any(|x| x.name == V05_DIMENSION_NAMES[0] && !x.is_sub));
    }

    #[test]
    fn reset_clears_all_streaks() {
        let mut baseline = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 {
            baseline.observe(&trace_with(0.5));
        }
        let mut det = DriftDetector::new(2.0, 2);
        let mut t = trace_with(0.5);
        t.v05_dims[0] = 0.95;
        det.observe(&t, &baseline);
        det.observe(&t, &baseline);
        assert!(det.dim_streaks()[0] > 0);
        det.reset();
        assert_eq!(det.dim_streaks()[0], 0);
        assert_eq!(det.sub_streaks()[0], 0);
    }
}

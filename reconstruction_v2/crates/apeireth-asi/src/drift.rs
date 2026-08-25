//! 漂移检测 (v1 等价 — 连续 z-score streak 检测)
use crate::{V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT, V1136_SUBMEASURE_NAMES};

#[derive(Debug, Clone, PartialEq)]
pub struct DriftAlarm {
    pub name: String,
    pub is_sub: bool,
    pub current: f64,
    pub mean: f64,
    pub std: f64,
    pub z_score: f64,
    pub streak: usize,
}

#[derive(Debug, Clone)]
pub struct DriftDetector {
    pub z_threshold: f64,
    pub window_threshold: usize,
    dim_streak: [usize; V05_DIM_COUNT],
    sub_streak: [usize; V1136_SUBMEASURE_COUNT],
}
impl Default for DriftDetector {
    fn default() -> Self { Self { z_threshold: 2.0, window_threshold: 3, dim_streak: [0; V05_DIM_COUNT], sub_streak: [0; V1136_SUBMEASURE_COUNT] } }
}
impl DriftDetector {
    pub fn new(z_threshold: f64, window_threshold: usize) -> Self {
        Self { z_threshold, window_threshold, dim_streak: [0; V05_DIM_COUNT], sub_streak: [0; V1136_SUBMEASURE_COUNT] }
    }
    pub fn observe(
        &mut self,
        trace: &crate::DimensionTrace,
        baseline: &crate::calibration::AdaptiveBaseline,
    ) -> Vec<DriftAlarm> {
        let mut alarms = Vec::new();
        for i in 0..V05_DIM_COUNT {
            let v = trace.v05_dims[i]; let z = baseline.dim_z(i, v);
            if z.abs() > self.z_threshold { self.dim_streak[i] += 1; } else { self.dim_streak[i] = 0; }
            if self.dim_streak[i] >= self.window_threshold {
                alarms.push(DriftAlarm {
                    name: V05_DIMENSION_NAMES[i].to_string(), is_sub: false, current: v,
                    mean: baseline.dim_mean[i], std: baseline.dim_std(i), z_score: z, streak: self.dim_streak[i],
                });
            }
        }
        for i in 0..V1136_SUBMEASURE_COUNT {
            let v = trace.v1136_subs[i]; let z = baseline.sub_z(i, v);
            if z.abs() > self.z_threshold { self.sub_streak[i] += 1; } else { self.sub_streak[i] = 0; }
            if self.sub_streak[i] >= self.window_threshold {
                alarms.push(DriftAlarm {
                    name: V1136_SUBMEASURE_NAMES[i].to_string(), is_sub: true, current: v,
                    mean: baseline.sub_mean[i], std: baseline.sub_std(i), z_score: z, streak: self.sub_streak[i],
                });
            }
        }
        alarms
    }
    pub fn dim_streaks(&self) -> &[usize; V05_DIM_COUNT] { &self.dim_streak }
    pub fn sub_streaks(&self) -> &[usize; V1136_SUBMEASURE_COUNT] { &self.sub_streak }
    pub fn reset(&mut self) { self.dim_streak = [0; V05_DIM_COUNT]; self.sub_streak = [0; V1136_SUBMEASURE_COUNT]; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::AdaptiveBaseline;
    use crate::DimensionTrace;
    fn trace_with(v: f64) -> DimensionTrace {
        DimensionTrace { trace_id: 0, sample_id: 0, timestamp: 0, v05_dims: [v; V05_DIM_COUNT], v1136_subs: [v; V1136_SUBMEASURE_COUNT], hook_overrides: vec![] }
    }
    #[test]
    fn no_alarm_within_baseline() {
        let mut b = AdaptiveBaseline::with_alpha(0.5);
        for _ in 0..20 { b.observe(&trace_with(0.5)); }
        let mut det = DriftDetector::default();
        let alarms = det.observe(&trace_with(0.51), &b);
        assert!(alarms.is_empty());
    }
    #[test]
    fn alarm_after_3_outliers() {
        let mut b = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 { b.observe(&trace_with(0.5)); }
        let mut det = DriftDetector::new(2.0, 3);
        let mut t = trace_with(0.5); t.v05_dims[0] = 0.95;
        let a = det.observe(&t, &b); assert!(a.is_empty());
        let a = det.observe(&t, &b); assert!(a.is_empty());
        let a = det.observe(&t, &b);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].name, V05_DIMENSION_NAMES[0]);
        assert!(!a[0].is_sub);
    }
    #[test]
    fn streak_resets_on_recovery() {
        let mut b = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 { b.observe(&trace_with(0.5)); }
        let mut det = DriftDetector::new(2.0, 3);
        let mut t = trace_with(0.5); t.v05_dims[0] = 0.95;
        det.observe(&t, &b); det.observe(&t, &b);
        det.observe(&trace_with(0.5), &b);
        assert_eq!(det.dim_streaks()[0], 0);
    }
    #[test]
    fn sub_alarm_separate() {
        let mut b = AdaptiveBaseline::with_alpha(0.1);
        for _ in 0..30 { b.observe(&trace_with(0.5)); }
        let mut det = DriftDetector::new(2.0, 2);
        let mut t = trace_with(0.5); t.v1136_subs[0] = 0.95; t.v05_dims[0] = 0.95;
        det.observe(&t, &b); det.observe(&t, &b); det.observe(&t, &b);
        let a = det.observe(&t, &b);
        // already past threshold on 3rd
        assert!(a.iter().any(|x| x.name == V1136_SUBMEASURE_NAMES[0] && x.is_sub));
    }
    #[test]
    fn reset_clears() {
        let mut det = DriftDetector::new(2.0, 2);
        assert_eq!(det.dim_streaks()[0], 0);
        det.reset();
        assert_eq!(det.dim_streaks()[0], 0);
        assert_eq!(det.sub_streaks()[0], 0);
    }
}

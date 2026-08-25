//! 重新校准调度器 (v1 等价)
use crate::calibration::{AdaptiveBaseline, CalibrationCoefficients, CalibrationLoop, UserFeedback};
use crate::DimensionTrace;

#[derive(Debug, Clone)]
pub struct ScheduleReport {
    pub trigger_count: usize,
    pub history_size: usize,
    pub feedback_count: usize,
    pub new_coefficients: CalibrationCoefficients,
    pub dry_run: bool,
    pub reason: String,
}

#[derive(Debug)]
pub struct RecalibrationScheduler {
    pub every_n: usize,
    pub count: usize,
    pending_feedback: Vec<UserFeedback>,
    last_history_size: usize,
    pub history: Vec<CalibrationCoefficients>,
}
impl Default for RecalibrationScheduler {
    fn default() -> Self { Self { every_n: 100, count: 0, pending_feedback: Vec::new(), last_history_size: 0, history: Vec::new() } }
}
impl RecalibrationScheduler {
    pub fn with_every_n(every_n: usize) -> Self { Self { every_n: every_n.max(1), ..Self::default() } }
    pub fn observe(
        &mut self,
        trace: &DimensionTrace,
        baseline: &mut AdaptiveBaseline,
        calibrator: &dyn CalibrationLoop,
        now: i64,
        history_window: usize,
    ) -> Option<ScheduleReport> {
        baseline.observe(trace);
        self.count += 1;
        if self.count % self.every_n == 0 {
            Some(self.run_now(baseline, calibrator, now, history_window, false, format!("scheduled @ M={}", self.every_n)))
        } else { None }
    }
    pub fn force_run(
        &mut self,
        baseline: &AdaptiveBaseline,
        calibrator: &dyn CalibrationLoop,
        now: i64,
        history_window: usize,
        dry_run: bool,
    ) -> ScheduleReport {
        self.run_now(baseline, calibrator, now, history_window, dry_run, "manual".to_string())
    }
    pub fn add_feedback(&mut self, fb: UserFeedback) { self.pending_feedback.push(fb); }
    pub fn drain_feedback(&mut self) -> Vec<UserFeedback> { std::mem::take(&mut self.pending_feedback) }
    pub fn pending_feedback_count(&self) -> usize { self.pending_feedback.len() }
    fn run_now(
        &mut self,
        baseline: &AdaptiveBaseline,
        calibrator: &dyn CalibrationLoop,
        now: i64,
        history_window: usize,
        dry_run: bool,
        reason: String,
    ) -> ScheduleReport {
        let _ = history_window;
        let feedback = self.drain_feedback();
        let new = calibrator.compute(&[], &feedback, baseline, now);
        let report = ScheduleReport {
            trigger_count: self.count,
            history_size: self.last_history_size,
            feedback_count: feedback.len(),
            new_coefficients: new.clone(),
            dry_run,
            reason: reason.clone(),
        };
        if !dry_run {
            self.history.push(new);
            if self.history.len() > 64 { self.history.remove(0); }
        }
        report
    }
    pub fn run_with_history(
        &mut self,
        history: &[DimensionTrace],
        baseline: &AdaptiveBaseline,
        calibrator: &dyn CalibrationLoop,
        now: i64,
        dry_run: bool,
        reason: &str,
    ) -> ScheduleReport {
        let feedback = self.drain_feedback();
        self.last_history_size = history.len();
        let new = calibrator.compute(history, &feedback, baseline, now);
        let report = ScheduleReport {
            trigger_count: self.count,
            history_size: history.len(),
            feedback_count: feedback.len(),
            new_coefficients: new.clone(),
            dry_run,
            reason: reason.to_string(),
        };
        if !dry_run {
            self.history.push(new);
            if self.history.len() > 64 { self.history.remove(0); }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::LinearCalibration;
    use crate::{V05_DIM_COUNT, V1136_SUBMEASURE_COUNT};
    fn trace_with(v: f64) -> DimensionTrace {
        DimensionTrace { trace_id: 0, sample_id: 0, timestamp: 0, v05_dims: [v; V05_DIM_COUNT], v1136_subs: [v; V1136_SUBMEASURE_COUNT], hook_overrides: vec![] }
    }
    #[test]
    fn no_fire_below_threshold() {
        let mut sched = RecalibrationScheduler::with_every_n(100);
        let mut b = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        for i in 0..99 {
            let r = sched.observe(&trace_with(0.5), &mut b, &cal, 0, 50);
            assert!(r.is_none());
        }
    }
    #[test]
    fn fires_at_exact_multiple() {
        let mut sched = RecalibrationScheduler::with_every_n(100);
        let mut b = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        let mut fired = 0;
        for i in 1..=200 {
            if sched.observe(&trace_with(0.5), &mut b, &cal, i64::from(i), 50).is_some() { fired += 1; }
        }
        assert_eq!(fired, 2);
    }
    #[test]
    fn feedback_consumed() {
        let mut sched = RecalibrationScheduler::with_every_n(10);
        let mut b = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        sched.add_feedback(UserFeedback::for_dim("thread_continuity", 0.5, 0.9, 0));
        sched.add_feedback(UserFeedback::for_dim("fact_recall", 0.4, 0.8, 0));
        assert_eq!(sched.pending_feedback_count(), 2);
        for i in 1..=10 { sched.observe(&trace_with(0.5), &mut b, &cal, i64::from(i), 50); }
        assert_eq!(sched.pending_feedback_count(), 0);
    }
    #[test]
    fn dry_run_no_history() {
        let mut sched = RecalibrationScheduler::with_every_n(10);
        let b = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        let r = sched.force_run(&b, &cal, 0, 50, true);
        assert!(r.dry_run);
        assert!(sched.history.is_empty());
    }
    #[test]
    fn run_with_history_uses_history() {
        let mut sched = RecalibrationScheduler::default();
        let mut b = AdaptiveBaseline::with_alpha(0.5);
        let hist: Vec<DimensionTrace> = (0..50).map(|_| trace_with(0.5)).collect();
        b.observe_batch(&hist);
        let cal = LinearCalibration::default();
        let r = sched.run_with_history(&hist, &b, &cal, 0, false, "test");
        assert!(!r.dry_run);
        assert_eq!(r.history_size, 50);
        assert_eq!(sched.history.len(), 1);
    }
    #[test]
    fn force_run_increments_history() {
        let mut sched = RecalibrationScheduler::default();
        let b = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        sched.force_run(&b, &cal, 0, 50, false);
        sched.force_run(&b, &cal, 1, 50, false);
        assert_eq!(sched.history.len(), 2);
    }
}

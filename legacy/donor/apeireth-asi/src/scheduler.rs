//! 重新校准调度器 (round15-01 backend_engineer)
//!
//! [`RecalibrationScheduler`] 跟踪累计测量次数, 每达到 `every_n` 次就触发一次
//! [`CalibrationLoop::compute`], 输出新 [`CalibrationCoefficients`] 并写回。
//!
//! 设计原则 (Ponytail):
//! - 调度器本身只跟踪计数 + 上次校准历史切片, 不做 fit
//! - 触发频率与计算量可调 (默认每 100 次一次)
//! - 干跑 (dry-run) 模式: 只计算新系数但不修改 `Coefficients`, 用于 CLI 预览

use crate::calibration::{
    AdaptiveBaseline, CalibrationCoefficients, CalibrationLoop, UserFeedback,
};
use crate::DimensionTrace;

/// 调度报告: 一次 recalibration 的执行结果。
#[derive(Debug, Clone)]
pub struct ScheduleReport {
    /// 触发的总测量次数 (从 0 开始计)。
    pub trigger_count: usize,
    /// 本次使用的历史 trace 数。
    pub history_size: usize,
    /// 本次使用的用户反馈数。
    pub feedback_count: usize,
    /// 新算出的系数 (与 `prev` 比对可看到变化量)。
    pub new_coefficients: CalibrationCoefficients,
    /// 是不是 dry-run。
    pub dry_run: bool,
    /// 触发原因 (例如 "scheduled @ M=100" 或 "manual")。
    pub reason: String,
}

/// 重新校准调度器。
#[derive(Debug)]
pub struct RecalibrationScheduler {
    /// 每 N 次测量触发一次 (默认 100)。
    pub every_n: usize,
    /// 累计测量计数 (从 0)。
    pub count: usize,
    /// 累计 pending 反馈 (未消费)。
    pending_feedback: Vec<UserFeedback>,
    /// 上一次校准时使用的历史 trace 数 (用于诊断)。
    last_history_size: usize,
    /// 系数历史 (每次 recalibration 追加一条, 用于回溯)。
    pub history: Vec<CalibrationCoefficients>,
}

impl Default for RecalibrationScheduler {
    fn default() -> Self {
        Self {
            every_n: 100,
            count: 0,
            pending_feedback: Vec::new(),
            last_history_size: 0,
            history: Vec::new(),
        }
    }
}

impl RecalibrationScheduler {
    /// 创建指定 every_n 的调度器。
    pub fn with_every_n(every_n: usize) -> Self {
        Self {
            every_n: every_n.max(1),
            ..Self::default()
        }
    }

    /// 喂入一条新测量 trace。
    ///
    /// 返回 `Some(ScheduleReport)` 表示触发了重新校准; 否则返回 `None`。
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
            Some(self.run_now(
                baseline,
                calibrator,
                now,
                history_window,
                false,
                format!("scheduled @ M={}", self.every_n),
            ))
        } else {
            None
        }
    }

    /// 强制立即跑一次 (无论是否达到 M)。
    pub fn force_run(
        &mut self,
        baseline: &AdaptiveBaseline,
        calibrator: &dyn CalibrationLoop,
        now: i64,
        history_window: usize,
        dry_run: bool,
    ) -> ScheduleReport {
        self.run_now(
            baseline,
            calibrator,
            now,
            history_window,
            dry_run,
            "manual".to_string(),
        )
    }

    /// 添加用户反馈 (在下次 recalibration 时被消费)。
    pub fn add_feedback(&mut self, fb: UserFeedback) {
        self.pending_feedback.push(fb);
    }

    /// 取出所有 pending feedback (不保留)。
    pub fn drain_feedback(&mut self) -> Vec<UserFeedback> {
        std::mem::take(&mut self.pending_feedback)
    }

    /// 当前 pending feedback 数。
    pub fn pending_feedback_count(&self) -> usize {
        self.pending_feedback.len()
    }

    fn run_now(
        &mut self,
        baseline: &AdaptiveBaseline,
        calibrator: &dyn CalibrationLoop,
        now: i64,
        history_window: usize,
        dry_run: bool,
        reason: String,
    ) -> ScheduleReport {
        // 取最近 history_window 条 trace: 但我们没有 trace 缓冲, 这里返回空 history,
        // calibrator 仍可用 baseline 推出 offset。
        // 真实生产: 外层 caller 把 history 传入更精确的 run_with_history。
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
            if self.history.len() > 64 {
                self.history.remove(0); // bounded
            }
        }
        let _ = reason;
        report
    }

    /// 用显式 history 跑一次校准 (更精确, 把 caller 持有的 trace 历史传入)。
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
            if self.history.len() > 64 {
                self.history.remove(0);
            }
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
    fn scheduler_does_not_fire_below_threshold() {
        let mut sched = RecalibrationScheduler::with_every_n(100);
        let mut baseline = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        for i in 0..99 {
            let result = sched.observe(
                &trace_with(0.5 + f64::from(i) * 0.001),
                &mut baseline,
                &cal,
                0,
                50,
            );
            assert!(result.is_none(), "should not fire at count {}", i + 1);
        }
    }

    #[test]
    fn scheduler_fires_at_exact_multiple() {
        let mut sched = RecalibrationScheduler::with_every_n(100);
        let mut baseline = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        let mut fired = 0;
        for i in 1..=200 {
            let result = sched.observe(&trace_with(0.5), &mut baseline, &cal, i64::from(i), 50);
            if result.is_some() {
                fired += 1;
            }
        }
        assert_eq!(fired, 2, "should fire at count 100 and 200");
    }

    #[test]
    fn feedback_is_consumed_during_recalibration() {
        let mut sched = RecalibrationScheduler::with_every_n(10);
        let mut baseline = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        sched.add_feedback(UserFeedback::for_dim("thread_continuity", 0.5, 0.9, 0));
        sched.add_feedback(UserFeedback::for_dim("fact_recall", 0.4, 0.8, 0));
        assert_eq!(sched.pending_feedback_count(), 2);

        for i in 1..=10 {
            sched.observe(&trace_with(0.5), &mut baseline, &cal, i64::from(i), 50);
        }
        // 反馈已被消费
        assert_eq!(sched.pending_feedback_count(), 0);
    }

    #[test]
    fn dry_run_does_not_store_history() {
        let mut sched = RecalibrationScheduler::with_every_n(10);
        let baseline = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        let report = sched.force_run(&baseline, &cal, 0, 50, true);
        assert!(report.dry_run);
        assert!(sched.history.is_empty());
        assert_eq!(report.reason, "manual");
    }

    #[test]
    fn apply_run_with_history_uses_explicit_history() {
        let mut sched = RecalibrationScheduler::default();
        let mut baseline = AdaptiveBaseline::with_alpha(0.5);
        let history: Vec<DimensionTrace> = (0..50).map(|_| trace_with(0.5)).collect();
        baseline.observe_batch(&history);
        let cal = LinearCalibration::default();
        let report = sched.run_with_history(&history, &baseline, &cal, 0, false, "test");
        assert!(!report.dry_run);
        assert_eq!(report.history_size, 50);
        assert_eq!(sched.history.len(), 1);
    }

    #[test]
    fn force_run_increments_history() {
        let mut sched = RecalibrationScheduler::default();
        let baseline = AdaptiveBaseline::default();
        let cal = LinearCalibration::default();
        let _ = sched.force_run(&baseline, &cal, 0, 50, false);
        let _ = sched.force_run(&baseline, &cal, 1, 50, false);
        assert_eq!(sched.history.len(), 2);
    }
}

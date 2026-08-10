//! Integration tests for ML online calibration loop (round15-01 backend_engineer)
//!
//! Covers the 5 mandated scenarios:
//! 1. drift_detected — 连续偏离触发告警
//! 2. recal_scheduled — 每 M 次测量触发 recalibration
//! 3. user_feedback_apply — 反馈调整系数
//! 4. rolling_baseline — AdaptiveBaseline 跟踪 regime 变化
//! 5. adaptive_threshold — DriftDetector z 阈值动态跟随 baseline

use apeireth_asi::{
    AdaptiveBaseline, CalibrationLoop, DimensionTrace, DriftDetector, LinearCalibration,
    RecalibrationScheduler, UserFeedback, V05_DIMENSION_NAMES, V05_DIM_COUNT,
    V1136_SUBMEASURE_COUNT, V1136_SUBMEASURE_NAMES,
};

fn trace_uniform(value: f64) -> DimensionTrace {
    DimensionTrace {
        trace_id: 0,
        sample_id: 0,
        timestamp: 0,
        v05_dims: [value; V05_DIM_COUNT],
        v1136_subs: [value; V1136_SUBMEASURE_COUNT],
        hook_overrides: vec![],
    }
}

/// Scenario 1: 连续 N 次偏离 baseline → DriftDetector 触发告警。
#[test]
fn drift_detected_after_3_consecutive_outliers() {
    let mut baseline = AdaptiveBaseline::with_alpha(0.05);
    // 用稳态 0.5 训练 baseline
    for _ in 0..50 {
        baseline.observe(&trace_uniform(0.5));
    }

    let mut detector = DriftDetector::new(2.0, 3);
    let idx = V05_DIMENSION_NAMES
        .iter()
        .position(|n| *n == "thread_continuity")
        .unwrap();

    // 制造一个 extreme outlier trace (dim[idx] = 0.95)
    let mut outlier = trace_uniform(0.5);
    outlier.v05_dims[idx] = 0.95;

    let mut triggered = false;
    for i in 1..=10 {
        let alarms = detector.observe(&outlier, &baseline);
        if i >= 3 {
            // 第 3 次开始应触发
            assert!(!alarms.is_empty(), "expected alarms at iteration {i}");
            assert!(
                alarms.iter().any(|a| a.name == V05_DIMENSION_NAMES[idx]),
                "alarm should be on the dim we perturbed"
            );
            triggered = true;
        } else {
            assert!(alarms.is_empty(), "no alarms at iteration {i}");
        }
    }
    assert!(triggered, "drift must trigger at some iteration");
}

/// Scenario 2: 每 M 次测量触发 RecalibrationScheduler。
#[test]
fn recal_scheduled_every_m_measurements() {
    let mut baseline = AdaptiveBaseline::default();
    let mut scheduler = RecalibrationScheduler::with_every_n(10);
    let cal = LinearCalibration::default();
    let mut fire_count = 0usize;

    for seq in 1..=50 {
        let report = scheduler.observe(&trace_uniform(0.5), &mut baseline, &cal, i64::from(seq), 25);
        if report.is_some() {
            fire_count += 1;
        }
    }
    // 50 / 10 = 5 次触发
    assert_eq!(
        fire_count, 5,
        "expected 5 firings in 50 observations with M=10"
    );
    assert_eq!(
        scheduler.history.len(),
        5,
        "history should record 5 coefficients"
    );
}

/// Scenario 3: 用户反馈调整系数 (scale + offset)。
#[test]
fn user_feedback_apply_shifts_scale() {
    let mut baseline = AdaptiveBaseline::default();
    for _ in 0..20 {
        baseline.observe(&trace_uniform(0.5));
    }
    let history: Vec<DimensionTrace> = (0..20).map(|_| trace_uniform(0.5)).collect();

    let cal = LinearCalibration::default();
    let feedback = vec![
        UserFeedback::for_dim("thread_continuity", 0.4, 0.9, 100),
        UserFeedback::for_sub(V1136_SUBMEASURE_NAMES[0], 0.5, 0.95, 101),
    ];

    let coefs = cal.compute(&history, &feedback, &baseline, 1000);
    let dim_idx = V05_DIMENSION_NAMES
        .iter()
        .position(|n| *n == "thread_continuity")
        .unwrap();
    assert!(
        coefs.dims[dim_idx].scale > 1.0,
        "feedback expected→0.9 / observed→0.4 should push scale > 1, got {}",
        coefs.dims[dim_idx].scale
    );

    // apply 后 thread_continuity 应变高
    let t = trace_uniform(0.4);
    let adjusted = coefs.apply(&t);
    assert!(
        adjusted.v05_dims[dim_idx] > t.v05_dims[dim_idx],
        "adjusted should exceed observed: got {}",
        adjusted.v05_dims[dim_idx]
    );
}

/// Scenario 4: AdaptiveBaseline 滚动均值跟踪 regime 变化。
#[test]
fn rolling_baseline_tracks_regime_change() {
    let mut baseline = AdaptiveBaseline::with_alpha(0.1);
    // 前 50 = 0.7
    for _ in 0..50 {
        baseline.observe(&trace_uniform(0.7));
    }
    let m1 = baseline.dim_mean[0];
    assert!(
        (m1 - 0.7).abs() < 1e-6,
        "baseline should start at 0.7, got {m1}"
    );

    // 后 50 = 0.3 (regime change)
    for _ in 0..50 {
        baseline.observe(&trace_uniform(0.3));
    }
    let m2 = baseline.dim_mean[0];
    // EMA 应能拉到中间 (不卡在 0.7)
    assert!(
        m2 < 0.7,
        "baseline should drop after regime change, got {m2}"
    );
    assert!(m2 > 0.3, "baseline should not fully snap to 0.3, got {m2}");

    // 再来 100 条 0.3, baseline 应接近 0.3
    for _ in 0..100 {
        baseline.observe(&trace_uniform(0.3));
    }
    let m3 = baseline.dim_mean[0];
    assert!(
        m3 < 0.4,
        "after long 0.3 regime, baseline should be < 0.4, got {m3}"
    );
}

/// Scenario 5: AdaptiveBaseline 替换静态 baseline → DriftDetector z 阈值自适应。
#[test]
fn adaptive_threshold_follows_baseline_shift() {
    // 关键不变量: baseline 漂移时, 旧值不应再触发 drift alarm。
    let mut baseline = AdaptiveBaseline::with_alpha(0.2);
    // train on 0.5
    for _ in 0..30 {
        baseline.observe(&trace_uniform(0.5));
    }
    // 切换 regime: 大量 0.7
    for _ in 0..50 {
        baseline.observe(&trace_uniform(0.7));
    }
    let m = baseline.dim_mean[0];
    assert!(m > 0.55, "baseline should follow regime, got {m}");

    // 现在喂 0.7 — 不应该 alarm (因为 baseline 已经跟随)
    let mut detector = DriftDetector::new(2.0, 3);
    for _ in 0..5 {
        let alarms = detector.observe(&trace_uniform(0.7), &baseline);
        assert!(
            alarms.is_empty(),
            "no alarms expected once baseline follows regime, got {alarms:?}"
        );
    }
}

/// Bonus: RecalibrationScheduler + DriftDetector + AdaptiveBaseline 联合端到端。
#[test]
fn end_to_end_calibration_loop_handles_realistic_stream() {
    let mut baseline = AdaptiveBaseline::with_alpha(0.05);
    let mut detector = DriftDetector::new(2.0, 3);
    let mut scheduler = RecalibrationScheduler::with_every_n(50);
    let cal = LinearCalibration::default();

    let mut drifts = 0usize;
    let mut recals = 0usize;

    for seq in 1..=150 {
        // regime: 1..=50 = 0.5; 51..=100 = 0.55; 101..=150 = 0.55 with extreme on dim 0
        let v = if seq <= 50 { 0.5 } else { 0.55 };
        let mut t = trace_uniform(v);
        if seq > 100 {
            let idx = V05_DIMENSION_NAMES
                .iter()
                .position(|n| *n == "thread_continuity")
                .unwrap();
            t.v05_dims[idx] = 0.9;
        }
        if seq == 80 {
            scheduler.add_feedback(UserFeedback::for_dim("thread_continuity", 0.55, 0.9, 80));
        }

        let alarms = detector.observe(&t, &baseline);
        drifts += alarms.len();

        let rep = scheduler.observe(&t, &mut baseline, &cal, i64::from(seq), 25);
        if rep.is_some() {
            recals += 1;
        }
    }

    assert!(drifts > 0, "expected at least some drift alarms, got 0");
    // 150 / 50 = 3 次重新校准
    assert_eq!(recals, 3, "expected exactly 3 recalibrations");
    assert_eq!(scheduler.history.len(), 3);
}

/// Bonus: feedback 在 recalibration 后被消费。
#[test]
fn user_feedback_consumed_only_on_recalibration() {
    let mut baseline = AdaptiveBaseline::default();
    let mut scheduler = RecalibrationScheduler::with_every_n(5);
    let cal = LinearCalibration::default();

    scheduler.add_feedback(UserFeedback::for_dim("fact_recall", 0.3, 0.8, 0));
    assert_eq!(scheduler.pending_feedback_count(), 1);

    // 喂 4 条 — 不应触发
    for seq in 1..=4 {
        scheduler.observe(&trace_uniform(0.5), &mut baseline, &cal, i64::from(seq), 25);
    }
    assert_eq!(
        scheduler.pending_feedback_count(),
        1,
        "feedback must NOT be consumed yet"
    );

    // 第 5 条 — 触发
    scheduler.observe(&trace_uniform(0.5), &mut baseline, &cal, 5, 25);
    assert_eq!(
        scheduler.pending_feedback_count(),
        0,
        "feedback should be consumed"
    );
}

/// Bonus: dry-run 模式不写入历史。
#[test]
fn dry_run_mode_does_not_persist() {
    let mut scheduler = RecalibrationScheduler::default();
    let baseline = AdaptiveBaseline::default();
    let cal = LinearCalibration::default();

    let report = scheduler.force_run(&baseline, &cal, 0, 25, true);
    assert!(report.dry_run);
    assert_eq!(scheduler.history.len(), 0);

    scheduler.force_run(&baseline, &cal, 1, 25, false);
    assert_eq!(scheduler.history.len(), 1);
}

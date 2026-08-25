//! Integration test — 通过 apeireth-asi::xxx 调用全部 v1 pub 表面
//! 验证 v2 workspace build 中 `apeireth-asi::xxx` 路径可用

use apeireth_asi::{
    // lib.rs
    AsiV05Scores, V1136Submeasures, DimensionTrace,
    V05_DIMENSION_NAMES, V05_DIM_COUNT, V1136_SUBMEASURE_COUNT, V1136_SUBMEASURE_NAMES,
    placeholder,
    // measurement
    MeasurementSample, MeasurementError, DimensionRegistry,
    compute_dim, compute_sub,
    measure_dim_01_thread_continuity, measure_dim_24_tool_reuse,
    measure_sub_01_thread_continuity_score, measure_sub_09_v3_action_guard_rate,
    MeasurementHook, NoOpHook, RegressionAssertion, DefaultRegressionAssertion, RegressionResult,
    set_quiet_mode, is_quiet_mode,
    // calibration
    Coeff, CalibrationCoefficients, UserFeedback,
    AdaptiveBaseline, CalibrationLoop, LinearCalibration,
    // drift
    DriftAlarm, DriftDetector,
    // history
    TraceRepository,
    // render
    ascii_sparkline, format_trace_table, DiagnosticReport, diagnose_weakest,
    // scheduler
    RecalibrationScheduler, ScheduleReport,
    // tokenizer
    count_tokens, count_tokens_batch,
    // stats
    mean, variance_pop, variance_sample, stddev_pop, stddev_sample,
    median, percentile, z_score, min_max_scale, Welford,
    // dim_enhance
    PerDimEnhance, NoveltyScoreEnhance, ActionabilityScoreEnhance,
    ConfidenceScoreEnhance, TemporalRelevanceEnhance,
    CoreValuesConsistencyEnhance, VoiceConsistencyEnhance,
    enhance_measurement,
    // llm_judge
    LlmJudgeDim, JudgeResult,
};

#[test]
fn pub_surface_via_crate_path_compiles() {
    // 只调用一遍 — 编译过即可, 行为各 unit test 已覆盖
    let _ = placeholder();
    let _ = V05_DIM_COUNT;
    let _ = V05_DIMENSION_NAMES;
    let _ = V1136_SUBMEASURE_COUNT;
    let _ = V1136_SUBMEASURE_NAMES;
    let _: AsiV05Scores = AsiV05Scores::default();
    let _: V1136Submeasures = V1136Submeasures::default();
    let _ = DriftDetector::default();
    let _ = TraceRepository::new();
    let _ = RecalibrationScheduler::default();
    let _ = LinearCalibration::default();
    let _ = AdaptiveBaseline::default();
    let _ = CalibrationCoefficients::default();
    let _ = Coeff::default();
    let _ = UserFeedback::for_dim("x", 0.5, 0.6, 0);
    let _ = DimensionRegistry::new();
    let _ = NoOpHook;
    let _ = DefaultRegressionAssertion::default();
    let _ = NoveltyScoreEnhance;
    let _ = ascii_sparkline(&[0.1, 0.5]);
    let _ = count_tokens("hi");
    let _ = count_tokens_batch(&["a", "b"]);
    let _ = mean(&[1.0, 2.0]);
    let _ = stddev_pop(&[1.0, 2.0]);
    let _ = variance_pop(&[1.0]);
    let _ = variance_sample(&[1.0, 2.0]);
    let _ = stddev_sample(&[1.0, 2.0]);
    let _ = median(&mut [1.0, 2.0]);
    let _ = percentile(&mut [1.0, 2.0], 0.5);
    let _ = z_score(&[1.0, 2.0]);
    let _ = min_max_scale(&[1.0, 2.0]);
    let _ = Welford::new();
    set_quiet_mode(true);
    let _ = is_quiet_mode();
    let _: LlmJudgeDim = LlmJudgeDim::CoreValuesConsistency;
}

#[test]
fn end_to_end_pipeline() {
    // 1. 构造 sample
    let mut s = MeasurementSample::default();
    s.successes.insert("thread_continuity".into(), 8);
    s.attempts.insert("thread_continuity".into(), 10);
    s.qualities.insert("thread_continuity".into(), 0.8);
    for n in V05_DIMENSION_NAMES.iter() {
        s.successes.entry((*n).to_string()).or_insert(8);
        s.attempts.entry((*n).to_string()).or_insert(10);
        s.qualities.entry((*n).to_string()).or_insert(0.9);
    }
    for n in V1136_SUBMEASURE_NAMES.iter() {
        s.successes.entry((*n).to_string()).or_insert(8);
        s.attempts.entry((*n).to_string()).or_insert(10);
        s.qualities.entry((*n).to_string()).or_insert(0.9);
    }
    s.philosophy_gate_trials.insert("v1_pass_rate".into(), (8, 10));
    s.philosophy_gate_trials.insert("v2_pass_rate".into(), (7, 10));
    s.philosophy_gate_trials.insert("v3_pass_rate".into(), (9, 10));
    s.philosophy_gate_trials.insert("cone_of_truth_rate".into(), (10, 10));
    s.philosophy_gate_trials.insert("action_guard_rate".into(), (10, 10));
    s.philosophy_gate_trials.insert("v1_v2_pass_rate".into(), (15, 20));
    s.philosophy_gate_trials.insert("v3_action_guard_rate".into(), (19, 20));

    // 2. 测量
    let dim_val = measure_dim_01_thread_continuity(&s).unwrap();
    assert!((0.0..=1.0).contains(&dim_val));
    let sub_val = measure_sub_01_thread_continuity_score(&s).unwrap();
    assert!((0.0..=1.0).contains(&sub_val));
    let v = measure_dim_24_tool_reuse(&s).unwrap();
    assert!((0.0..=1.0).contains(&v));
    let p = measure_sub_09_v3_action_guard_rate(&s).unwrap();
    assert!((0.0..=1.0).contains(&p));

    // 3. trace
    let hook: Option<&dyn MeasurementHook> = None;
    let trace = DimensionTrace::from_sample(1, 1, 0, &s, hook);
    assert!((trace.mean_v05() - trace.v05_dims.iter().sum::<f64>() / 24.0).abs() < 1e-9);

    // 4. repo + render + diagnose
    let mut repo = TraceRepository::new();
    repo.append(trace.clone());
    assert_eq!(repo.len(), 1);
    let table = format_trace_table(&trace);
    assert!(table.contains("Mean V0.5"));
    let report = diagnose_weakest(&trace, 3);
    assert_eq!(report.weakest_dims.len(), 3);
    let line = ascii_sparkline(&trace.v05_dims.to_vec());
    assert_eq!(line.chars().count(), V05_DIM_COUNT);

    // 5. legacy projection
    let legacy = AsiV05Scores::from_trace(&trace);
    let sub_legacy = V1136Submeasures::from_trace(&trace);
    assert!((0.0..=1.0).contains(&legacy.continuity));
    assert_eq!(sub_legacy.continuity_5.len(), 5);

    // 6. baseline + drift
    let mut b = AdaptiveBaseline::with_alpha(0.1);
    b.observe(&trace);
    let mut det = DriftDetector::default();
    let _alarms = det.observe(&trace, &b);

    // 7. regression
    let ra = DefaultRegressionAssertion::default();
    let r: RegressionResult = ra.assert_within_range("test", 0.5, &[0.4, 0.5, 0.6]);
    assert!(r.passed);

    // 8. scheduler + calibration
    let mut sched = RecalibrationScheduler::default();
    let cal = LinearCalibration::default();
    let report: ScheduleReport = sched.force_run(&b, &cal, 0, 50, false);
    assert!(!report.dry_run);

    // 9. tokenizer + stats
    let toks = count_tokens("hello 你好");
    assert!(toks > 0);
    let m = mean(&[1.0, 2.0, 3.0]);
    assert!((m - 2.0).abs() < 1e-9);

    // 10. enhance
    let enhanced = enhance_measurement("novelty_score", 0.8, &s).unwrap();
    assert!((0.0..=1.0).contains(&enhanced));

    // 11. quiet mode
    set_quiet_mode(true);
    assert!(is_quiet_mode());

    // 12. error types compile
    let _: MeasurementError = MeasurementError::MissingObservation("x".into());
    let _: MeasurementError = MeasurementError::UnknownDimension("x".into());
    let _: MeasurementError = MeasurementError::ZeroAttempts("x".into());
    let _: MeasurementError = MeasurementError::NonFiniteValue("x".into());
    let _: MeasurementError = MeasurementError::SuccessExceedsAttempt { dim: "x".into(), success: 5, attempt: 3 };
}

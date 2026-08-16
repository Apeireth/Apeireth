//! Integration tests for apeireth-blueprint-impl
//!
//! 30+ tests covering 5 估补模块 + 总集成 pipeline.
//!
//! ## 6 哲学锚穿透
//!
//! - S-1 主 22:33 — 测试服务 ASI 北极星, 不装饰.
//! - S-2 主 17:43 — 全部 5 估补都覆盖, 无 TODO.
//! - O-5 主 17:58 — 失败路径必须 Err, 测试守门.
//! - O-2 主 19:33 — 借鉴 industry 集成测试模式 (单元 + 集成 + boundary).
//! - O-3 主 23:44 — 30+ 测试一次写齐.
//! - O-4 主 00:56 — 测试名自解释, 接手者能直接 run.
//!
//! ## 8 项不修改承诺
//!
//! 1. 测试不修改 5 估补模块的公共 API.
//! 2. 测试用 std 测试, 不依赖额外 crate.
//! 3. 测试覆盖 happy path + boundary + failure path.
//! 4. 测试名按 `test_<module>_<scenario>_<expected>` 规范.
//! 5. 测试断言用 assert!/assert_eq!, 不 panic 蒙混.
//! 6. 测试间不共享 state (除 InMemoryAudit 内部 buffer).
//! 7. 测试覆盖 5 估补 + 2 集成 (run_full_pipeline + meets_baseline).
//! 8. 测试不依赖 network / filesystem (除 tempfile E 模块).

use apeireth_blueprint_impl::*;
use std::time::Duration;

// ============================================
// 1. error 模块集成测试 (3 tests)
// ============================================

#[test]
fn test_error_module_category_classification() {
    // 13 variant 都正确分类
    let cases = [
        (
            BlueprintError::K1StrongValidationFailed {
                field: "f".into(),
                value: "v".into(),
                reason: "r".into(),
            },
            "K-1",
        ),
        (
            BlueprintError::K2WeakValidationFailed {
                field: "f".into(),
                reason: "r".into(),
            },
            "K-2",
        ),
        (
            BlueprintError::K3AuditFailed {
                channel: "c".into(),
                reason: "r".into(),
            },
            "K-3",
        ),
        (
            BlueprintError::K4GuardDenied {
                subject: "s".into(),
                rule: "r".into(),
            },
            "K-4",
        ),
        (
            BlueprintError::D01StubNotImplemented {
                tool: "t".into(),
                endpoint: "e".into(),
            },
            "D-01",
        ),
        (
            BlueprintError::D02RouteMissing {
                tool: "t".into(),
                sub_path: "p".into(),
            },
            "D-02",
        ),
        (
            BlueprintError::D03WsAuthFailed {
                reason: "r".into(),
                ttl_seconds: 0,
            },
            "D-03",
        ),
        (
            BlueprintError::D04RateLimitExceeded {
                bucket: "b".into(),
                retry_after_ms: 100,
            },
            "D-04",
        ),
        (
            BlueprintError::TemplateNotImplemented {
                template_id: "A".into(),
                stage: "R20".into(),
            },
            "TEMPLATE",
        ),
        (
            BlueprintError::QMetricOutOfRange {
                metric: "Q1".into(),
                value: 1.5,
            },
            "Q-METRIC",
        ),
        (BlueprintError::Io("e".into()), "IO"),
        (BlueprintError::Serialization("e".into()), "SERIALIZATION"),
        (BlueprintError::Other("e".into()), "OTHER"),
    ];
    for (e, expected) in &cases {
        assert_eq!(e.category(), *expected, "category mismatch for {e:?}");
    }
}

#[test]
fn test_error_module_is_retryable() {
    // K-1 / K-4 / D-01..D-03 / 模板 / Q-Metric 不可重试
    assert!(!BlueprintError::K1StrongValidationFailed {
        field: "f".into(),
        value: "v".into(),
        reason: "r".into(),
    }
    .is_retryable());
    assert!(!BlueprintError::K4GuardDenied {
        subject: "s".into(),
        rule: "r".into(),
    }
    .is_retryable());
    // K-2 / K-3 / D-04 / IO 可重试
    assert!(BlueprintError::K2WeakValidationFailed {
        field: "f".into(),
        reason: "r".into(),
    }
    .is_retryable());
    assert!(BlueprintError::K3AuditFailed {
        channel: "c".into(),
        reason: "r".into(),
    }
    .is_retryable());
    assert!(BlueprintError::D04RateLimitExceeded {
        bucket: "b".into(),
        retry_after_ms: 100,
    }
    .is_retryable());
    assert!(BlueprintError::Io("e".into()).is_retryable());
}

#[test]
fn test_error_module_display_includes_context() {
    let e = BlueprintError::K1StrongValidationFailed {
        field: "model_name".into(),
        value: "gpt-X".into(),
        reason: "unknown model".into(),
    };
    let s = format!("{e}");
    assert!(s.contains("model_name"));
    assert!(s.contains("unknown model"));
}

// ============================================
// 2. risk 模块集成测试 (8 tests)
// ============================================

#[test]
fn test_risk_module_k1_construct_rejects_empty() {
    let r = K1Input::new("", "sk-test1234", "gpt-4", "read");
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().category(), "K-1");
}

#[test]
fn test_risk_module_k1_construct_accepts_valid() {
    let r = K1Input::new("hello", "sk-test1234", "gpt-4", "read");
    assert!(r.is_ok());
    let input = r.unwrap();
    assert_eq!(input.user_input, "hello");
    assert_eq!(input.scope, "read");
}

#[test]
fn test_risk_module_k2_falls_back() {
    let g = DefaultK2Guard;
    let input = K2Input::new("", vec!["fb1".into()]);
    let r = g.validate(&input).unwrap();
    assert_eq!(r.used_value, "fb1");
    assert_eq!(r.fallback_layer, 1);
}

#[test]
fn test_risk_module_k2_caps_at_3() {
    let g = DefaultK2Guard;
    let input = K2Input::new("x".repeat(20 * 1024), vec!["".into(); 5]);
    assert!(g.validate(&input).is_err());
}

#[test]
fn test_risk_module_k3_in_memory_writes() {
    let a = InMemoryAudit::new(10);
    a.audit(&AuditEvent::now("T", "s", "i", "m1")).unwrap();
    a.audit(&AuditEvent::now("T", "s", "i", "m2")).unwrap();
    let recent = a.recent(5).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[1].message, "m2");
}

#[test]
fn test_risk_module_k3_broken_returns_err() {
    let a = BrokenAudit;
    let e = AuditEvent::now("T", "s", "i", "m");
    assert!(a.audit(&e).is_err());
}

#[test]
fn test_risk_module_k4_deny_returns_err() {
    let mut g = RuleTableGuard::new();
    g.add_rule(GuardRule {
        subject: "x".into(),
        action: "y".into(),
        decision: GuardDecision::Deny,
        reason: "test".into(),
    })
    .unwrap();
    assert!(g.decide("x", "y").is_err());
}

#[test]
fn test_risk_module_chain_runs_all_4_stages() {
    let mut g4 = RuleTableGuard::new();
    g4.add_rule(GuardRule {
        subject: "tool:bash".into(),
        action: "exec".into(),
        decision: GuardDecision::Allow,
        reason: "default".into(),
    })
    .unwrap();
    let chain = RiskChain::new(DefaultK1Guard, DefaultK2Guard, InMemoryAudit::default(), g4);
    let k1 = K1Input::new("hi", "sk-test1234", "gpt-4", "read").unwrap();
    let k2 = K2Input::new("hi", vec![]);
    let r = chain.run(&k1, &k2, "tool:bash", "exec");
    assert!(r.is_ok());
}

// ============================================
// 3. decision 模块集成测试 (6 tests)
// ============================================

#[test]
fn test_decision_module_d01_real_validates() {
    let d = D01Impl::RealConnect {
        provider: "claude-code".into(),
        endpoint: "/v1/messages".into(),
    };
    assert!(d.is_real());
    assert!(d.validate().is_ok());
}

#[test]
fn test_decision_module_d01_stub_not_real() {
    let d = D01Impl::StubNotImplemented {
        tool: "x".into(),
        planned_stage: "R21".into(),
    };
    assert!(!d.is_real());
    assert!(d.validate().is_ok());
}

#[test]
fn test_decision_module_d02_subpath_routes() {
    let d = D02Routing::SubPath {
        tool: "Bash".into(),
        sub_path: "/v1/<tool>".into(),
    };
    assert_eq!(d.route(), "/v1/Bash");
}

#[test]
fn test_decision_module_d03_5min_ttl_locked() {
    let d = D03WsAuth::LinkToken {
        ttl: Duration::from_secs(5 * 60),
    };
    assert!(d.validate().is_ok());

    let bad = D03WsAuth::LinkToken {
        ttl: Duration::from_secs(6 * 60),
    };
    assert!(bad.validate().is_err());
}

#[test]
fn test_decision_module_d04_capacity_60_default() {
    let d = D04RateLimit::default();
    assert!(d.validate().is_ok());
    match d {
        D04RateLimit::TokenBucket { capacity, .. } => assert_eq!(capacity, 60),
        _ => panic!("expected TokenBucket"),
    }
}

#[test]
fn test_decision_module_bundle_validates_all() {
    let b = DecisionBundle::default();
    assert!(b.validate().is_ok());
    assert!(b.snapshot().contains("D-01"));
}

// ============================================
// 4. template 模块集成测试 (6 tests)
// ============================================

#[test]
fn test_template_module_a_issues_valid_token() {
    let a = template_a_auth();
    let tok = a.issue("read").unwrap();
    assert!(a.verify(&tok).is_ok());
    assert!(a.has_scope(&tok, "read"));
}

#[test]
fn test_template_module_a_refresh_creates_new_token() {
    let a = template_a_auth();
    let tok = a.issue("read").unwrap();
    let new = a.refresh(&tok.refresh_token).unwrap();
    assert!(new.value != tok.value);
}

#[test]
fn test_template_module_b_ratelimit_starts_full() {
    let b = template_b_ratelimit();
    assert_eq!(b.available(), 60);
    b.try_acquire().unwrap();
    assert_eq!(b.available(), 59);
}

#[test]
fn test_template_module_b_ratelimit_exhausts() {
    let b = TokenBucket::new(2, Duration::from_millis(100));
    assert!(b.try_acquire().is_ok());
    assert!(b.try_acquire().is_ok());
    assert!(b.try_acquire().is_err());
}

#[test]
fn test_template_module_c_normalize_wraps() {
    let c = template_c_error();
    let e: Box<dyn std::error::Error> = "test".into();
    let ne = c.normalize(&*e);
    assert_eq!(ne.category(), "OTHER");
}

#[test]
fn test_template_module_d_mock_can_force_fail() {
    let (mut auth, _rl) = template_d_test();
    let tok = auth.issue("read").unwrap();
    auth.mock_set_next_verify(false);
    assert!(auth.verify(&tok).is_err());
}

// ============================================
// 5. r_measure 模块集成测试 (5 tests)
// ============================================

#[test]
fn test_r_measure_module_r1_perfect_is_one() {
    let r = r1_directness(&[ActionSample::perfect(), ActionSample::perfect()]);
    assert_eq!(r, 1.0);
}

#[test]
fn test_r_measure_module_r2_candor_mixed() {
    let mut s = ActionSample::perfect();
    s.candid = false;
    let r = r2_candor(&[ActionSample::perfect(), s]);
    assert_eq!(r, 0.5);
}

#[test]
fn test_r_measure_module_r4_promise_one_broken() {
    let mut s = ActionSample::perfect();
    s.promises[3] = false;
    let r = r4_promise(&[ActionSample::perfect(), s]);
    assert_eq!(r, 0.5);
}

#[test]
fn test_r_measure_module_r5_honesty_perfect() {
    let samples: Vec<ActionSample> = (0..5).map(|_| ActionSample::perfect()).collect();
    let r = r5_failure_honesty(&samples);
    assert_eq!(r, 1.0);
}

#[test]
fn test_r_measure_module_all_meets_baseline() {
    let all = RMeasureAll {
        r1_directness: 0.95,
        r2_candor: 0.90,
        r3_closure: 0.85,
        r4_promise: 0.90,
        r5_failure_honesty: 1.0,
    };
    assert!(all.drift().all_meet_baseline());
    assert!((all.average() - 0.92).abs() < 1e-9);
}

// ============================================
// 6. q_metric 模块集成测试 (5 tests)
// ============================================

#[test]
fn test_q_metric_module_q1_quality_perfect() {
    let tasks = vec![TaskResult::new(true, 1.0), TaskResult::new(true, 1.0)];
    assert_eq!(q1_quality(&tasks), 1.0);
}

#[test]
fn test_q_metric_module_q1_clamps_overscore() {
    let t = TaskResult::new(true, 1.5);
    assert_eq!(t.quality_score, 1.0);
}

#[test]
fn test_q_metric_module_q2_satisfaction_5star() {
    let f = vec![UserFeedback {
        rating: 5,
        has_text: false,
        is_long_term: false,
    }];
    assert_eq!(q2_satisfaction(&f), 1.0);
}

#[test]
fn test_q_metric_module_q3_growth_positive() {
    let s = vec![
        GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
        GrowthSnapshot::new(1, 0.8, 0.8, 0.8),
    ];
    let r = q3_growth(&s);
    assert!((r - 0.3).abs() < 1e-9);
}

#[test]
fn test_q_metric_module_q3_no_growth_single() {
    let s = vec![GrowthSnapshot::new(0, 0.5, 0.5, 0.5)];
    assert_eq!(q3_growth(&s), 0.0);
}

// ============================================
// 7. 总集成测试 (5 tests)
// ============================================

#[test]
fn test_integration_pipeline_with_perfect_inputs() {
    let decisions = DecisionBundle::default();
    let samples = vec![ActionSample::perfect(); 10];
    let tasks = vec![TaskResult::new(true, 1.0); 5];
    let feedback = vec![
        UserFeedback {
            rating: 5,
            has_text: true,
            is_long_term: true
        };
        3
    ];
    let history = vec![
        GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
        GrowthSnapshot::new(1, 0.9, 0.9, 0.9),
    ];
    let report = run_full_pipeline(decisions, &samples, &tasks, &feedback, &history).unwrap();
    assert!(report.meets_baseline());
    assert!(report.composite_score() > 0.9);
}

#[test]
fn test_integration_pipeline_fails_on_invalid_decisions() {
    let bad = DecisionBundle::new(
        D01Impl::default(),
        D02Routing::default(),
        D03WsAuth::LinkToken {
            ttl: Duration::from_secs(600),
        }, // 10min > 5min
        D04RateLimit::default(),
    );
    let r = run_full_pipeline(bad, &[], &[], &[], &[]);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().category(), "D-03");
}

#[test]
fn test_integration_pipeline_decision_snapshot() {
    let p = BlueprintPipeline::new();
    let s = p.decision_snapshot();
    assert!(s.contains("D-01"));
    assert!(s.contains("D-02"));
    assert!(s.contains("D-03"));
    assert!(s.contains("D-04"));
}

#[test]
fn test_integration_5_modules_all_exposed() {
    // K
    let _k1: Box<dyn K1StrongValidate> = Box::new(DefaultK1Guard);
    let _k2: Box<dyn K2WeakValidate> = Box::new(DefaultK2Guard);
    let _k3: Box<dyn K3Audit> = Box::new(InMemoryAudit::default());
    let _k4: Box<dyn K4Guard> = Box::new(RuleTableGuard::new());
    // D
    let _d01 = D01Impl::default();
    let _d02 = D02Routing::default();
    let _d03 = D03WsAuth::default();
    let _d04 = D04RateLimit::default();
    // T
    let _a: Box<dyn Auth> = Box::new(template_a_auth());
    let _b: Box<dyn RateLimit> = Box::new(template_b_ratelimit());
    let _c: Box<dyn UnifiedError> = Box::new(template_c_error());
    let _e: Box<dyn ConfigLoader> = Box::new(template_e_config());
    let _f: Box<dyn Logging> = Box::new(template_f_logging());
    // R + Q
    let _r1 = r1_directness(&[]);
    let _q1 = q1_quality(&[]);
    // PHILOSOPHY_ANCHORS + EIGHT_PROMISES
    assert_eq!(PHILOSOPHY_ANCHORS.len(), 6);
    assert_eq!(EIGHT_PROMISES.len(), 8);
}

#[test]
fn test_integration_six_philosophy_anchors_present() {
    for anchor in PHILOSOPHY_ANCHORS.iter() {
        assert!(!anchor.is_empty());
        assert!(
            anchor.contains("主")
                || anchor.contains("北极星")
                || anchor.contains("实事求是")
                || anchor.contains("不假装")
                || anchor.contains("走在前人经验上")
                || anchor.contains("干到底")
                || anchor.contains("任何人都能接手")
        );
    }
}

// ============================================
// 8. 边界测试 (boundary, 5 tests)
// ============================================

#[test]
fn test_boundary_empty_samples_r_measure() {
    let r = RMeasureAll::from_samples(&[]);
    assert_eq!(r.r1_directness, 0.0);
    assert_eq!(r.r2_candor, 0.0);
    // 0.0 在 [0.0, 1.0] 范围内 → 校验过
    assert!(r.validate().is_ok());
}

#[test]
fn test_boundary_zero_capacity_ratelimit() {
    let b = TokenBucket::new(0, Duration::from_secs(1));
    // 容量 0 → 立即拒绝
    assert!(b.try_acquire().is_err());
}

#[test]
fn test_boundary_oversize_input_rejected() {
    let big = "x".repeat(70 * 1024);
    let r = K1Input::new(big, "sk-test1234", "gpt-4", "read");
    assert!(r.is_err());
}

#[test]
fn test_boundary_nan_quality_clamped() {
    let t = TaskResult::new(true, f64::NAN);
    assert_eq!(t.quality_score, 0.0);
}

#[test]
fn test_boundary_q_metric_out_of_range_err() {
    let bad = QMetricAll {
        q1_quality: 2.0,
        q2_satisfaction: 0.5,
        q3_growth: 0.5,
    };
    assert!(bad.validate().is_err());
    assert_eq!(bad.validate().unwrap_err().category(), "Q-METRIC");
}

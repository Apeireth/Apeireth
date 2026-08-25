//! # Blueprint Impl Demo
//!
//! End-to-end demo of all 5 估补 modules in pipeline.
//!
//! Run with: `cargo run --example blueprint_impl_demo -p apeireth-blueprint-impl`

use apeireth_blueprint_impl::*;
use std::time::Duration;

fn main() {
    println!("========================================");
    println!("apeireth-blueprint-impl — 5 估补 demo");
    println!("========================================\n");

    // --- 1. 4 决策打包 ---
    println!("[1/5] decision: 4 决策打包");
    let decisions = DecisionBundle::new(
        D01Impl::RealConnect {
            provider: "claude-code".into(),
            endpoint: "/v1/messages".into(),
        },
        D02Routing::SubPath {
            tool: "Bash".into(),
            sub_path: "/v1/<tool>".into(),
        },
        D03WsAuth::LinkToken {
            ttl: Duration::from_secs(5 * 60),
        },
        D04RateLimit::TokenBucket {
            capacity: 60,
            refill_interval: Duration::from_secs(1),
        },
    );
    decisions.validate().expect("decisions must validate");
    println!("    {}", decisions.snapshot());
    println!();

    // --- 2. risk 4 风险类 ---
    println!("[2/5] risk: K-1..K-4 4 风险类");
    let chain = default_risk_chain();
    let k1 =
        K1Input::new("hello world", "sk-test1234", "gpt-4", "read").expect("K1Input must be valid");
    let k2 = K2Input::new("hello", vec!["fallback1".into(), "fallback2".into()]);
    let (k2_result, decision) = chain
        .run(&k1, &k2, "tool:bash", "exec")
        .expect("risk chain must pass");
    println!(
        "    K-2 result: used_value={}, fallback_layer={}",
        k2_result.used_value, k2_result.fallback_layer
    );
    println!("    K-4 decision: {decision:?}");
    println!();

    // --- 3. template 6 模板 ---
    println!("[3/5] template: A-F 6 模板");
    let bundle = demo_template_bundle();
    let tok = bundle
        .execute("read", "tool:bash")
        .expect("template bundle must succeed");
    println!("    A. 鉴权 token issued: value={}", tok.value);
    println!("    B. 限流 remaining: {}", bundle.rate_limit.available());
    println!("    C. 错误处理: unified BlueprintError");
    let (mock_auth, _mock_rl) = template_d_test();
    let _ = mock_auth.issue("read");
    println!("    D. 测试模板: mock_auth + always_allow_ratelimit ready");
    let _cfg = template_e_config();
    println!("    E. 配置管理: EnvFileConfig ready");
    let log = template_f_logging();
    log.trace("demo", "F. 日志 trace + audit ready");
    println!();

    // --- 4. R-Measure 5 维 ---
    println!("[4/5] r_measure: R-1..R-5 5 维");
    let samples = (0..100)
        .map(|_| ActionSample::perfect())
        .collect::<Vec<_>>();
    let r = RMeasureAll::from_samples(&samples);
    r.validate().expect("R-Measure must validate");
    println!("    R-1 直行率:    {}", r.r1_directness);
    println!("    R-2 直说率:    {}", r.r2_candor);
    println!("    R-3 闭环率:    {}", r.r3_closure);
    println!("    R-4 守门率:    {}", r.r4_promise);
    println!("    R-5 诚实率:    {}", r.r5_failure_honesty);
    println!("    平均:          {}", r.average());
    println!("    跟 R11 baseline 对比 (drift):");
    let drift = r.drift();
    println!("      R-1: {} (baseline 0.9063)", drift.r1);
    println!("      R-2: {} (baseline 0.8532)", drift.r2);
    println!("      R-4: {} (baseline 0.8682)", drift.r4);
    println!();

    // --- 5. Q-Metric 3 维 ---
    println!("[5/5] q_metric: Q1..Q3 3 维");
    let tasks = (0..10)
        .map(|_| TaskResult::new(true, 1.0))
        .collect::<Vec<_>>();
    let feedback = (0..5)
        .map(|_| UserFeedback {
            rating: 5,
            has_text: true,
            is_long_term: true,
        })
        .collect::<Vec<_>>();
    let history = vec![
        GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
        GrowthSnapshot::new(1, 0.9, 0.9, 0.9),
    ];
    let q = QMetricAll::from_inputs(&tasks, &feedback, &history);
    q.validate().expect("Q-Metric must validate");
    println!("    Q1 任务质量:    {}", q.q1_quality);
    println!("    Q2 用户满意:    {}", q.q2_satisfaction);
    println!("    Q3 长期成长:    {}", q.q3_growth);
    println!("    平均:          {}", q.average());
    println!();

    // --- 总集成 ---
    println!("=== 总集成 (BlueprintPipeline) ===");
    let report = run_full_pipeline(decisions, &samples, &tasks, &feedback, &history)
        .expect("full pipeline must succeed");
    println!("    决策快照:     {}", report.decision_snapshot);
    println!(
        "    R-Measure:    {} (avg)",
        report.r_measure.map(|m| m.average()).unwrap_or(0.0)
    );
    println!(
        "    Q-Metric:     {} (avg)",
        report.q_metric.map(|m| m.average()).unwrap_or(0.0)
    );
    println!("    Composite:    {}", report.composite_score());
    println!("    meets_baseline: {}", report.meets_baseline());

    println!("\n========================================");
    println!("6 哲学锚 (S-1/S-2/O-5/O-2/O-3/O-4):");
    for (i, anchor) in PHILOSOPHY_ANCHORS.iter().enumerate() {
        println!("    {}. {}", i + 1, anchor);
    }

    println!("\n8 项不修改承诺:");
    for promise in EIGHT_PROMISES.iter() {
        println!("    {}", promise);
    }

    println!("\nDemo complete. ✅");
}

//! R129-6 Stage 6 K4 健康守护 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage6_k4_health_run`
//!
//! 演示 K4 健康守护: 5 维度 (R11Compat / AsiCritical / PyBridge / Security / Performance) 自检

use apeireth_pybridge::{
    stage6_health_check, stage6_health_healthy, stage6_health_summary, HealthDimension,
    HealthStatus,
};

fn main() {
    println!("=== R129-6 Stage 6 K4 健康守护 (anyone-can-run) ===\n");

    // 1. 5 维度演示
    let dims = [
        (HealthDimension::R11Compat, "R11_Compat"),
        (HealthDimension::AsiCritical, "ASI_Critical"),
        (HealthDimension::PyBridge, "PyBridge"),
        (HealthDimension::Security, "Security"),
        (HealthDimension::Performance, "Performance"),
    ];
    println!("  K4 5 dimensions:");
    for (d, name) in dims.iter() {
        println!("    [{d}] {name}");
    }
    assert_eq!(HealthDimension::N_DIMENSIONS, 5);

    // 2. 4 状态演示
    println!("\n  HealthStatus 4 levels:");
    for s in [
        HealthStatus::Unknown,
        HealthStatus::Warn,
        HealthStatus::Crit,
        HealthStatus::Ok,
    ] {
        println!("    {s} score={}", s.score());
    }

    // 3. 跑全维度自检
    let r = stage6_health_check();
    println!("\n  Full self-check:");
    println!("    R11 modules: {}", r.r11_module_count);
    println!("    ASI modules: {}", r.asi_module_count);
    println!("    python_ext: {}", r.python_ext_active);
    println!("    checks: {}", r.checks.len());
    println!(
        "    score: {}/{} ({:.1}%)",
        r.total_score,
        r.max_score,
        r.score_percent()
    );
    println!("    all_ok: {}", r.all_ok);
    println!(
        "    ok/warn/crit/unknown: {}/{}/{}/{}",
        r.n_ok, r.n_warn, r.n_crit, r.n_unknown
    );

    // 4. 维度聚合
    println!("\n  Per-dimension:");
    for (i, (d, name)) in dims.iter().enumerate() {
        println!(
            "    {name}: status={} score={}/100",
            r.dimension_status[i], r.dimension_scores[i]
        );
        let _ = d;
    }

    // 5. 摘要
    println!("\n  summary: {}", stage6_health_summary());
    println!("  healthy: {}", stage6_health_healthy());

    println!("\n=== K4 done (5-dimension ASI self-check) ===");
}

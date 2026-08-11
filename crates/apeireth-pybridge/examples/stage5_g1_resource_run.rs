//! R129-5 ASI Python 整合 Stage 5 治理 — G1 资源治理 example
//!
//! 跑: `cargo run --example stage5_g1_resource_run -p apeireth-pybridge`
//!
//! 演示: G1 资源治理 4 维度 (rate/memory/time/count) + 7 ASI Python 模块 + 3 路径 (Allow/Throttle/Reject)

use apeireth_pybridge::*;

fn main() {
    println!("=== R129-5 G1 资源治理 example ===\n");

    // 1. 健康度
    let health = resource_governance_health();
    println!("G1 资源治理 ({}):", health.version);
    println!("  dimensions: {}", health.dimension_count);
    println!("  ASI modules: {}", health.asi_module_count);
    println!("  ok: {}\n", health.is_ok);

    // 2. 创建治理引擎 (自动引导 7 ASI Python 模块)
    let mut governor = ResourceGovernor::new();
    println!("引导 7 ASI Python 模块:");
    for module in [
        "apeireth.v1077_asi_v04_full_measurement",
        "apeireth.v1400_asi_self_framework",
        "apeireth.v1447_asi_cross_modular_audit",
        "apeireth.v1457_asi_six_deployment_operational_runbook",
        "apeireth.v1458_asi_north_star_ceiling_chain_audit",
        "apeireth.v1467_asi_audit_http_gateway_history_diff",
        "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence",
    ] {
        let quota = governor.quota_for(module);
        println!(
            "  {} → rate={} mem={}MB time={}ms count={}",
            module.split('.').nth(1).unwrap_or(""),
            quota.rate_per_sec,
            quota.memory_bytes / (1024 * 1024),
            quota.time_ms,
            quota.count_max
        );
    }
    println!();

    // 3. 演示 3 路径 (V1458 ceiling critical strict)
    println!("3 路径演示 (V1458 ceiling_critical strict, rate quota = 10):");
    let v1458 = "apeireth.v1458_asi_north_star_ceiling_chain_audit";
    for (used, expected) in [
        (5, "Allow"),
        (8, "Throttle"),
        (11, "Reject"),
    ] {
        let action = governor.check(v1458, ResourceDimension::Rate, used);
        println!(
            "  rate used={} → {} (expected {})",
            used,
            action.name(),
            expected
        );
    }
    println!();

    // 4. 跑 audit_all (7 modules × 4 dims = 28 events)
    let report = resource_governance_summary();
    println!("全模块 audit 结果:");
    println!("{}", report);

    println!("=== G1 资源治理 example done ===");
}

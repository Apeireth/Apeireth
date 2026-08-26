//! R129-5 ASI Python 整合 Stage 5 治理 — G4 演进治理 example
//!
//! 跑: `cargo run --example stage5_g4_evolution_run -p apeireth-pybridge`
//!
//! 演示: G4 演进治理 4 规则 (Add/Upgrade/Downgrade/Retire) + 4 演进类型 + 3 状态
//! 借鉴 superpowers 234 (Skill lifecycle) + langgraph 829 (node lifecycle) + kani 4502 (invariant)

use apeireth_pybridge::*;

fn main() {
    println!("=== R129-5 G4 演进治理 example ===\n");

    // 1. 健康度
    let health = evolution_governance_health();
    println!("G4 演进治理 ({}):", health.version);
    println!("  rules: {}", health.rule_count);
    println!("  kinds: {}", health.kind_count);
    println!("  stages: {}", health.stage_count);
    println!("  ASI modules: {}", health.asi_module_count);
    println!("  ok: {}\n", health.is_ok);

    // 2. 4 演进类型
    println!("4 演进类型 (借 superpowers Skill lifecycle + langgraph node lifecycle):");
    for kind in EvolutionKind::ALL {
        println!(
            "  {}: {} (number={})",
            kind.name(),
            kind.description(),
            kind.number()
        );
    }
    println!();

    // 3. 4 演进规则
    println!("4 演进规则 (1:1 跟 4 类型):");
    for rule in EvolutionRule::ALL {
        println!(
            "  {}: {} (number={})",
            rule.name(),
            rule.description(),
            rule.number()
        );
    }
    println!();

    // 4. 7 ASI Python 模块
    let engine = EvolutionEngine::new();
    println!("7 关键 ASI Python 模块 (Stage 1 1:1):");
    for (module, version) in [
        ("apeireth.v1077_asi_v04_full_measurement", 1077),
        ("apeireth.v1400_asi_self_framework", 1400),
        ("apeireth.v1447_asi_cross_modular_audit", 1447),
        (
            "apeireth.v1457_asi_six_deployment_operational_runbook",
            1457,
        ),
        ("apeireth.v1458_asi_north_star_ceiling_chain_audit", 1458),
        ("apeireth.v1467_asi_audit_http_gateway_history_diff", 1467),
        (
            "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence",
            1470,
        ),
    ] {
        let v = engine.current_version(module);
        println!("  v{} → {}", v, module);
        assert_eq!(v, version);
    }
    println!();

    // 5. 跑 audit_default (4 规则 × 1 module)
    println!("跑 audit_default (4 规则 × V1077):");
    let report = evolution_governance_summary();
    println!("{}", report);

    // 6. 4 规则演示
    println!("4 规则演示 (V1077 完整生命周期):");
    let mut engine = EvolutionEngine::new();
    let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);

    // R1 Add
    let r1 = engine.check_r1_new_module_safe(&ctx);
    println!("  R1 NewModuleSafe (Add) → {}", r1.name());

    // R2 Upgrade
    let r2 = engine.check_r2_upgrade_backward_compat(&ctx, 1078);
    println!(
        "  R2 UpgradeBackwardCompat (Upgrade 1077→1078) → {}",
        r2.name()
    );

    // R3 Downgrade
    let r3 = engine.check_r3_downgrade_justified(&ctx, 1076, "perf issue");
    println!(
        "  R3 DowngradeJustified (Downgrade 1077→1076 + reason) → {}",
        r3.name()
    );

    // R4 Retire
    let r4 = engine.check_r4_retire_confirmed(&ctx, true, true);
    println!("  R4 RetireConfirmed (Retire + 3 方确认) → {}", r4.name());

    println!("\n=== G4 演进治理 example done ===");
}

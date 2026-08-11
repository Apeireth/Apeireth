//! R129-5 ASI Python 整合 Stage 5 治理 — G2 权限治理 example
//!
//! 跑: `cargo run --example stage5_g2_permission_run -p apeireth-pybridge`
//!
//! 演示: G2 权限治理 6 重守门 v7 (1:1 跟 B4) + 3 状态 (Allow/Deny/AuditRequired) + 4 Stage

use apeireth_pybridge::*;

fn main() {
    println!("=== R129-5 G2 权限治理 example ===\n");

    // 1. 健康度
    let health = permission_governance_health();
    println!("G2 权限治理 ({}):", health.version);
    println!("  layers: {} (6-fold v7, 1:1 跟 B4)", health.layer_count);
    println!("  stages: {}", health.stage_count);
    println!("  ok: {}\n", health.is_ok);

    // 2. 6 重守门
    println!("6 重守门 v7 (1:1 跟 B4 严守):");
    for layer in PermissionLayer::ALL {
        println!(
            "  {}: {} (number={})",
            layer.name(),
            layer.description(),
            layer.number()
        );
    }
    println!();

    // 3. 演示 3 状态
    println!("3 状态演示:");

    // Safe context → Allow
    let mut engine = PermissionEngine::new();
    let safe_ctx = PermissionContext::safe_default();
    let d = engine.check(safe_ctx);
    println!(
        "  safe ctx (asi_stage=1, module=0, resource=0) → {}",
        d.name()
    );

    // High resource → AuditRequired
    let mut engine = PermissionEngine::new();
    let high_resource_ctx = PermissionContext {
        resource_used: 85,
        ..PermissionContext::safe_default()
    };
    let d = engine.check(high_resource_ctx);
    println!(
        "  high resource (resource=85, threshold 80) → {}",
        d.name()
    );

    // Invalid module_id → Deny
    let mut engine = PermissionEngine::new();
    let invalid_ctx = PermissionContext {
        module_id: 99,
        ..PermissionContext::safe_default()
    };
    let d = engine.check(invalid_ctx);
    println!(
        "  invalid module (module_id=99, max 6) → {}",
        d.name()
    );
    println!();

    // 4. Stage 4 严格守门 (R129-4 自治)
    println!("Stage 4 严格守门 (per R129-4 自治 + audit 必):");
    let mut engine = PermissionEngine::new().with_stage4_strict();
    let stage4_ctx = PermissionContext {
        asi_stage: 4,
        module_id: 4, // V1458 ceiling critical
        audit_required: false,
        ..PermissionContext::safe_default()
    };
    let d = engine.check(stage4_ctx);
    println!(
        "  stage 4 + V1458 + no audit → {} (L4 GuardCheck 触发 audit)",
        d.name()
    );
    println!();

    // 5. 4 Stage 默认检查
    let report = permission_governance_summary();
    println!("4 Stage 默认检查 (Stage 1-3 + R129-4):");
    println!("{}", report);

    println!("=== G2 权限治理 example done ===");
}

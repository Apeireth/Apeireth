//! R129-18 Stage 7 I6 G2+K3 权限+安全集成 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage7_i6_permission_security_run`
//!
//! 演示 I6: 6 permission layer × 7 security gate (6 重 v7 + G7 跨语言) = 42 绑定

use apeireth_pybridge::{
    stage7_i6_healthy, stage7_i6_summary, PermissionLayer, PermissionSecurityCoordinator,
    SecurityGate, SecurityVerdict,
};

fn main() {
    println!("=== R129-18 Stage 7 I6 G2+K3 权限+安全集成 (anyone-can-run) ===\n");

    println!("[1] public API");
    println!("    summary: {}", stage7_i6_summary());
    println!("    healthy: {}", stage7_i6_healthy());

    let mut c = PermissionSecurityCoordinator::new();

    println!("\n[2] check 演示 (6 重 v7 baseline + G7 跨语言)");
    println!("    2.1 6 重 v7 baseline (L1TypeCheck+G1Identity ... L6ProvenanceCheck+G6Audit):");
    for (l, g) in [
        (PermissionLayer::L1TypeCheck, SecurityGate::G1Identity),
        (PermissionLayer::L2ScopeCheck, SecurityGate::G2Goal),
        (PermissionLayer::L3RateCheck, SecurityGate::G3Capability),
        (PermissionLayer::L4GuardCheck, SecurityGate::G4Compliance),
        (PermissionLayer::L5AuditCheck, SecurityGate::G5Resource),
        (PermissionLayer::L6ProvenanceCheck, SecurityGate::G6Audit),
    ] {
        let v = c.check(0, l, g);
        println!("      {l:?} + {g:?} → {v:?}");
    }

    println!("\n    2.2 G7 跨语言 (6 layer × G7CrossLanguage = 6 绑定):");
    for l in &[
        PermissionLayer::L1TypeCheck,
        PermissionLayer::L2ScopeCheck,
        PermissionLayer::L3RateCheck,
        PermissionLayer::L4GuardCheck,
        PermissionLayer::L5AuditCheck,
        PermissionLayer::L6ProvenanceCheck,
    ] {
        let v = c.check(0, *l, SecurityGate::G7CrossLanguage);
        println!("      {l:?} + G7CrossLanguage → {v:?}");
    }

    println!("\n[3] 报告统计");
    println!("    total: {}", c.report.event_count());
    println!("    allow: {}", c.report.allow_count());
    println!("    block: {}", c.report.block_count());

    println!("\n=== I6 done (B4 6 重 v7 + G7 跨语言 严守) ===");
    // 静默 SecuretyVerdict 枚举引用 (避免 unused warning)
    let _ = SecurityVerdict::Allow;
}

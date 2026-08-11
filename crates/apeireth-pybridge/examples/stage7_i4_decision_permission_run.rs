//! R129-18 Stage 7 I4 D4+G2 决策+权限集成 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage7_i4_decision_permission_run`
//!
//! 演示 I4: 5 decision policy × 6 permission layer (1:1 跟 B4 6 重 v7 严守) = 30 绑定

use apeireth_pybridge::{
    stage7_i4_healthy, stage7_i4_summary, DecisionPermissionCoordinator, DecisionPolicy,
    PermissionContext, PermissionLayer,
};

fn main() {
    println!("=== R129-18 Stage 7 I4 D4+G2 决策+权限集成 (anyone-can-run) ===\n");

    println!("[1] public API");
    println!("    summary: {}", stage7_i4_summary());
    println!("    healthy: {}", stage7_i4_healthy());

    let mut c = DecisionPermissionCoordinator::new();
    let ctx = PermissionContext::safe_default();

    println!("\n[2] decide 演示 (不同 policy → 不同 rule)");

    // 2.1 Conservative → deny
    let r = c.decide(1, DecisionPolicy::Conservative, PermissionLayer::L1TypeCheck, &ctx);
    println!("    Conservative + L1TypeCheck → {r}");

    // 2.2 Cautious → audit_required
    let r = c.decide(2, DecisionPolicy::Cautious, PermissionLayer::L1TypeCheck, &ctx);
    println!("    Cautious + L1TypeCheck → {r}");

    // 2.3 Balanced → allow
    let r = c.decide(3, DecisionPolicy::Balanced, PermissionLayer::L1TypeCheck, &ctx);
    println!("    Balanced + L1TypeCheck → {r}");

    // 2.4 Aggressive → allow
    let r = c.decide(4, DecisionPolicy::Aggressive, PermissionLayer::L6ProvenanceCheck, &ctx);
    println!("    Aggressive + L6ProvenanceCheck → {r}");

    println!("\n[3] 报告统计");
    println!("    total: {}", c.report.event_count());
    println!("    allow: {}", c.report.allow_count());
    println!("    deny: {}", c.report.deny_count());
    println!("    audit_required: {}", c.report.audit_required_count());

    println!("\n=== I4 done (B4 6 重 v7 严守) ===");
}

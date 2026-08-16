//! R129-18 Stage 7 I1 D1+G1 工具+资源集成 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage7_i1_tool_resource_run`
//!
//! 演示 I1: 5 default tool × 4 resource dim = 20 绑定 + check_and_call 三种动作 (Allow/Reject/Throttle)

use apeireth_pybridge::{
    stage7_i1_healthy, stage7_i1_summary, GovernanceAction, ResourceDimension,
    ToolResourceCoordinator,
};

fn main() {
    println!("=== R129-18 Stage 7 I1 D1+G1 工具+资源集成 (anyone-can-run) ===\n");

    // 1. 演示公共 API
    println!("[1] public API");
    println!("    summary: {}", stage7_i1_summary());
    println!("    healthy: {}", stage7_i1_healthy());

    // 2. 演示协调器 3 动作
    let mut c = ToolResourceCoordinator::new();
    println!("\n[2] 协调器 check_and_call 演示");

    // 2.1 已知 tool + dim → Allow
    let a = c.check_and_call(1, "executor", ResourceDimension::Rate);
    println!("    executor + Rate → {a:?}");

    // 2.2 已知 tool + dim → Allow
    let a = c.check_and_call(2, "reflector", ResourceDimension::Memory);
    println!("    reflector + Memory → {a:?}");

    // 2.3 未知 tool → Reject
    let a = c.check_and_call(3, "nonexistent", ResourceDimension::Count);
    println!("    nonexistent + Count → {a:?}");

    // 3. 报告统计
    println!("\n[3] 报告统计");
    println!("    total events: {}", c.report.event_count());
    println!("    allow: {}", c.report.allow_count());
    println!("    throttle: {}", c.report.throttle_count());
    println!("    reject: {}", c.report.reject_count());

    println!("\n=== I1 done ===");
}

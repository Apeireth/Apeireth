//! R129-18 Stage 7 I3 D3+G3 记忆+形式化集成 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage7_i3_memory_formal_run`
//!
//! 演示 I3: 7 memory kind × 8 invariant = 56 绑定 + verify_memory 8 invariant

use apeireth_pybridge::{
    stage7_i3_healthy, stage7_i3_summary, MemoryFormalCoordinator, MemoryKind,
    STAGE7_I3_HARNESS_COUNT,
};

fn main() {
    println!("=== R129-18 Stage 7 I3 D3+G3 记忆+形式化集成 (anyone-can-run) ===\n");

    println!("[1] public API");
    println!("    summary: {}", stage7_i3_summary());
    println!("    healthy: {}", stage7_i3_healthy());

    let mut c = MemoryFormalCoordinator::new();
    println!("\n[2] verify_memory 演示 (1 kind × 8 invariant = 8 passed)");

    // 2.1 verify 1 个 kind, 应 8 pass
    let passed = c.verify_memory(1, MemoryKind::ToolInvocation);
    println!("    ToolInvocation: passed={passed}/{}", STAGE7_I3_HARNESS_COUNT);

    // 2.2 verify 多个 kind
    println!("\n[3] verify 多个 kind 演示");
    for k in [
        MemoryKind::ToolInvocation,
        MemoryKind::DecisionMake,
        MemoryKind::AuditCheckpoint,
    ] {
        let p = c.verify_memory(0, k);
        println!("    {k:?}: passed={p}/{}", STAGE7_I3_HARNESS_COUNT);
    }

    println!("\n[4] 报告统计");
    println!("    total events: {}", c.report.event_count());
    println!("    passed: {}", c.report.passed_count());
    println!("    failed: {}", c.report.failed_count());

    println!("\n=== I3 done ===");
}

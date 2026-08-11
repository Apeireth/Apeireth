//! R129-18 Stage 7 I2 D2+K1 反思+错误集成 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage7_i2_reflection_error_run`
//!
//! 演示 I2: 8 reflection node × 4 error kind = 32 绑定 + reflect_and_recover

use apeireth_pybridge::{
    stage7_i2_healthy, stage7_i2_summary, ReflectionErrorCoordinator, ErrorKind,
};

fn main() {
    println!("=== R129-18 Stage 7 I2 D2+K1 反思+错误集成 (anyone-can-run) ===\n");

    println!("[1] public API");
    println!("    summary: {}", stage7_i2_summary());
    println!("    healthy: {}", stage7_i2_healthy());

    let mut c = ReflectionErrorCoordinator::new();
    println!("\n[2] reflect_and_recover 演示 (auto_retry 策略)");

    // 2.1 observe + Bridge → success
    let ok = c.reflect_and_recover(1, "observe", ErrorKind::Bridge);
    println!("    observe + Bridge → ok={ok}");

    // 2.2 analyze + Contract → success
    let ok = c.reflect_and_recover(2, "analyze", ErrorKind::Contract);
    println!("    analyze + Contract → ok={ok}");

    // 2.3 nonexistent node → fail
    let ok = c.reflect_and_recover(3, "nonexistent", ErrorKind::Bridge);
    println!("    nonexistent → ok={ok}");

    // 3. 4 kinds 全 recover
    println!("\n[3] 4 kinds 全 recover 演示");
    for k in [
        ErrorKind::Transport,
        ErrorKind::Conversion,
        ErrorKind::Bridge,
        ErrorKind::Contract,
    ] {
        let ok = c.reflect_and_recover(0, "reflect", k);
        println!("    reflect + {k:?} → ok={ok}");
    }

    println!("\n[4] 报告统计");
    println!("    total: {}", c.report.event_count());
    println!("    success: {}", c.report.recovery_success_count());
    println!("    failed: {}", c.report.recovery_failed_count());

    println!("\n=== I2 done ===");
}

//! R129-6 Stage 6 K1 错误守护 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage6_k1_error_run`
//!
//! 演示 K1 错误守护: 4 类 (Transport/Conversion/Bridge/Contract) + 4 严重度 + 错误事件记录

use apeireth_pybridge::{
    stage6_error_healthy, stage6_error_summary, stage6_record_error, ErrorEvent, ErrorKind,
    ErrorSeverity,
};

fn main() {
    println!("=== R129-6 Stage 6 K1 错误守护 (anyone-can-run) ===\n");

    // 1. 演示 4 类错误
    for (kind, sev) in [
        (ErrorKind::Transport, ErrorSeverity::Info),
        (ErrorKind::Conversion, ErrorSeverity::Warn),
        (ErrorKind::Bridge, ErrorSeverity::Error),
        (ErrorKind::Contract, ErrorSeverity::Critical),
    ] {
        let ev = stage6_record_error(kind, sev, "demo", &format!("{kind} test"));
        println!("  recorded: [{ev:?}]");
    }

    // 2. 演示 1 个完整链式 ErrorEvent
    let ev = ErrorEvent::new(
        ErrorKind::Bridge,
        ErrorSeverity::Error,
        "pybridge.call",
        "module not found",
    )
    .with_location("bridge.rs:42")
    .with_recovery("use Rust fallback")
    .with_cause("pyo3::PyImportError")
    .with_timestamp(1_700_000_000);
    println!("\n  chained event:\n{ev}");

    // 3. 摘要
    println!("\n  summary: {}", stage6_error_summary());
    println!("  healthy: {}", stage6_error_healthy());

    println!("\n=== K1 done ===");
}

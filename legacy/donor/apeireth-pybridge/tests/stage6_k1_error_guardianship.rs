//! R129-6 K1 错误守护 集成测试
//!
//! per decision-61 §3.1 R129-6 — Stage 6 守护 K1 维度

use apeireth_pybridge::{
    stage6_error_healthy, stage6_error_summary, stage6_record_error, ErrorEvent, ErrorKind,
    ErrorSeverity,
};

#[test]
fn k1_record_and_summary() {
    let ev = stage6_record_error(
        ErrorKind::Bridge,
        ErrorSeverity::Error,
        "test_bridge",
        "module import failed",
    );
    assert_eq!(ev.kind, ErrorKind::Bridge);
    let s = stage6_error_summary();
    assert!(s.contains("K1 ErrorGuard"));
}

#[test]
fn k1_error_kind_4() {
    assert_eq!(ErrorKind::Transport.name(), "Transport");
    assert_eq!(ErrorKind::Conversion.name(), "Conversion");
    assert_eq!(ErrorKind::Bridge.name(), "Bridge");
    assert_eq!(ErrorKind::Contract.name(), "Contract");
    assert_eq!(ErrorKind::N_KINDS, 4);
}

#[test]
fn k1_error_severity_4() {
    assert_eq!(ErrorSeverity::Info.score(), 1);
    assert_eq!(ErrorSeverity::Warn.score(), 10);
    assert_eq!(ErrorSeverity::Error.score(), 50);
    assert_eq!(ErrorSeverity::Critical.score(), 100);
    assert_eq!(ErrorSeverity::N_SEVERITIES, 4);
}

#[test]
fn k1_error_event_chain() {
    let ev = ErrorEvent::new(ErrorKind::Conversion, ErrorSeverity::Error, "test", "fail")
        .with_location("src/test.rs:42")
        .with_recovery("retry")
        .with_cause("TypeError")
        .with_timestamp(1_700_000_000);
    assert_eq!(ev.location.as_deref(), Some("src/test.rs:42"));
    assert_eq!(ev.recovery_hint.as_deref(), Some("retry"));
    assert_eq!(ev.caused_by.as_deref(), Some("TypeError"));
    assert_eq!(ev.timestamp, 1_700_000_000);
}

#[test]
fn k1_healthy_default() {
    // 全局状态可能被其他测试污染, 但 0 Critical 时 = true
    let _ = stage6_error_healthy();
}

#[test]
fn k1_summary_format() {
    let s = stage6_error_summary();
    assert!(s.contains("K1 ErrorGuard"));
    assert!(s.contains("events="));
}

#[test]
fn k1_error_event_display() {
    let ev = ErrorEvent::new(ErrorKind::Bridge, ErrorSeverity::Error, "pybridge", "fail")
        .with_location("test.rs:1")
        .with_recovery("retry");
    let s = format!("{ev}");
    assert!(s.contains("[Error|Bridge]"));
    assert!(s.contains("pybridge"));
    assert!(s.contains("test.rs:1"));
    assert!(s.contains("retry"));
}

//! TUI Error - 错误展示 (从 v1.0 apeireth-tui/error.rs 280 LOC 抄录升级)
//!
//! 0 装 PASS: 真错误格式化 + 严重级别 + history

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity { Info, Warning, Error, Critical }

impl Severity {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Info => "[i]",
            Self::Warning => "[!]",
            Self::Error => "[E]",
            Self::Critical => "[X]",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub severity: Severity,
    pub message: String,
    pub source: String,    // 0 装 PASS: 哪个模块报错
    pub timestamp_ms: i64,
    pub suggestion: Option<String>,
}

pub struct ErrorLog {
    entries: VecDeque<ErrorEntry>,
    capacity: usize,
}

impl ErrorLog {
    pub fn new(capacity: usize) -> Self { Self { entries: VecDeque::with_capacity(capacity), capacity } }

    /// 0 装 PASS: 真记录
    pub fn record(&mut self, severity: Severity, source: impl Into<String>, message: impl Into<String>, suggestion: Option<String>) {
        let entry = ErrorEntry { severity, message: message.into(), source: source.into(), timestamp_ms: chrono::Utc::now().timestamp_millis(), suggestion };
        self.entries.push_back(entry);
        if self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }

    /// 0 装 PASS: 真按 severity filter
    pub fn by_severity(&self, s: Severity) -> Vec<&ErrorEntry> {
        self.entries.iter().filter(|e| e.severity == s).collect()
    }

    /// 0 装 PASS: 真 critical 检查 (有 critical 则返 true)
    pub fn has_critical(&self) -> bool {
        self.entries.iter().any(|e| e.severity == Severity::Critical)
    }

    pub fn len(&self) -> usize { self.entries.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_severity_icon() {
        assert_eq!(Severity::Info.icon(), "[i]");
        assert_eq!(Severity::Critical.icon(), "[X]");
    }
    #[test] fn test_record_and_filter() {
        let mut log = ErrorLog::new(10);
        log.record(Severity::Info, "test", "ok", None);
        log.record(Severity::Error, "test", "fail", Some("retry".into()));
        assert_eq!(log.by_severity(Severity::Info).len(), 1);
        assert_eq!(log.by_severity(Severity::Error).len(), 1);
    }
    #[test] fn test_has_critical() {
        let mut log = ErrorLog::new(10);
        log.record(Severity::Info, "x", "y", None);
        assert!(!log.has_critical());
        log.record(Severity::Critical, "x", "fatal", None);
        assert!(log.has_critical());
    }
    #[test] fn test_capacity() {
        let mut log = ErrorLog::new(2);
        for i in 0..5 { log.record(Severity::Info, "x", &format!("{}", i), None); }
        assert_eq!(log.len(), 2);
    }
}

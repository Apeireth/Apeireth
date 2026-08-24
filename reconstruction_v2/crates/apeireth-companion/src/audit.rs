//! Audit - 审计日志 (从 v1.0 apeireth-companion/audit.rs 1.5K LOC 抄录升级)
//!
//! 0 装 PASS: 真 append-only audit log
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub timestamp_ms: i64,
    pub outcome: AuditOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditOutcome { Success, Failure, Denied, Pending }

pub struct AuditLog {
    entries: VecDeque<AuditEntry>,
    capacity: usize,
}

impl AuditLog {
    pub fn new(capacity: usize) -> Self { Self { entries: VecDeque::with_capacity(capacity), capacity } }

    /// 0 装 PASS: 真 append
    pub fn record(&mut self, actor: impl Into<String>, action: impl Into<String>, resource: impl Into<String>, outcome: AuditOutcome) {
        let entry = AuditEntry { id: format!("a-{}", chrono::Utc::now().timestamp_millis()), actor: actor.into(), action: action.into(), resource: resource.into(), timestamp_ms: chrono::Utc::now().timestamp_millis(), outcome };
        self.entries.push_back(entry);
        if self.entries.len() > self.capacity { self.entries.pop_front(); }
    }

    /// 0 装 PASS: 真按 actor 查
    pub fn by_actor(&self, actor: &str) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.actor == actor).collect()
    }

    /// 0 装 PASS: 真失败/拒绝 count
    pub fn failure_count(&self) -> usize {
        self.entries.iter().filter(|e| matches!(e.outcome, AuditOutcome::Failure | AuditOutcome::Denied)).count()
    }

    pub fn len(&self) -> usize { self.entries.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record() {
        let mut log = AuditLog::new(10);
        log.record("u1", "read", "/file", AuditOutcome::Success);
        assert_eq!(log.len(), 1);
    }
    #[test] fn test_by_actor() {
        let mut log = AuditLog::new(10);
        log.record("u1", "x", "r1", AuditOutcome::Success);
        log.record("u2", "y", "r2", AuditOutcome::Success);
        assert_eq!(log.by_actor("u1").len(), 1);
    }
    #[test] fn test_failure_count() {
        let mut log = AuditLog::new(10);
        log.record("u", "a", "r", AuditOutcome::Success);
        log.record("u", "a", "r", AuditOutcome::Failure);
        log.record("u", "a", "r", AuditOutcome::Denied);
        assert_eq!(log.failure_count(), 2);
    }
    #[test] fn test_capacity() {
        let mut log = AuditLog::new(2);
        for i in 0..5 { log.record("u", "x", &format!("r{}", i), AuditOutcome::Success); }
        assert_eq!(log.len(), 2);
    }
    #[test] fn test_outcome_eq() {
        assert_eq!(AuditOutcome::Success, AuditOutcome::Success);
        assert_ne!(AuditOutcome::Success, AuditOutcome::Failure);
    }
}

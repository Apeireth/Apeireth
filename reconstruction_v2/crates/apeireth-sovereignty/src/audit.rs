//! Sovereignty 审计日志

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const EVENT_KIND_COUNT_HARDCODE: usize = 4;
pub const AUDIT_LEVEL_COUNT_HARDCODE: usize = 5;
pub const K1_STRICT_CHECK_COUNT_HARDCODE: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    Access, Modify, Delete, Export,
}

impl EventKind {
    pub fn required_min_level(self) -> AuditLevel {
        match self {
            EventKind::Access => AuditLevel::Read,
            EventKind::Modify => AuditLevel::Write,
            EventKind::Delete => AuditLevel::Admin,
            EventKind::Export => AuditLevel::Owner,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self { EventKind::Access => "access", EventKind::Modify => "modify", EventKind::Delete => "delete", EventKind::Export => "export" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditLevel { Read = 1, Write = 2, Admin = 3, Owner = 4, Root = 5 }

impl AuditLevel {
    pub fn as_str(self) -> &'static str {
        match self { Self::Read => "read", Self::Write => "write", Self::Admin => "admin", Self::Owner => "owner", Self::Root => "root" }
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum AuditError {
    #[error("K-1.a 强校验失败: actor 字段为空")]
    K1ActorEmpty,
    #[error("K-1.b 强校验失败: resource 字段为空")]
    K1ResourceEmpty,
    #[error("K-1.c 强校验失败: 事件 {event:?} 要求至少 {required:?} 级, 实际为 {actual:?}")]
    K1LevelInsufficient { event: EventKind, required: AuditLevel, actual: AuditLevel },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub kind: EventKind,
    pub actor: String,
    pub resource: String,
    pub level: AuditLevel,
    pub reason: String,
    pub timestamp_ms: i64,
}

impl AuditEvent {
    pub fn new(kind: EventKind, actor: impl Into<String>, resource: impl Into<String>, level: AuditLevel, reason: impl Into<String>) -> Self {
        Self { id: format!("audit-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)), kind, actor: actor.into(), resource: resource.into(), level, reason: reason.into(), timestamp_ms: chrono::Utc::now().timestamp_millis() }
    }
    pub fn with_id(mut self, id: impl Into<String>) -> Self { self.id = id.into(); self }
    pub fn validate_k1(&self) -> Result<(), AuditError> {
        if self.actor.trim().is_empty() { return Err(AuditError::K1ActorEmpty); }
        if self.resource.trim().is_empty() { return Err(AuditError::K1ResourceEmpty); }
        let required = self.kind.required_min_level();
        if self.level < required { return Err(AuditError::K1LevelInsufficient { event: self.kind, required, actual: self.level }); }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuditLog { events: Vec<AuditEvent> }

impl AuditLog {
    pub fn new() -> Self { Self::default() }
    pub fn try_record(&mut self, event: AuditEvent) -> Result<(), AuditError> {
        event.validate_k1()?;
        self.events.push(event);
        Ok(())
    }
    pub fn record(&mut self, kind: EventKind, actor: impl Into<String>, resource: impl Into<String>, level: AuditLevel, reason: impl Into<String>) -> Result<(), AuditError> {
        let event = AuditEvent::new(kind, actor, resource, level, reason);
        self.try_record(event)
    }
    pub fn len(&self) -> usize { self.events.len() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
    pub fn filter_by_actor(&self, actor: &str) -> Vec<&AuditEvent> { self.events.iter().filter(|e| e.actor == actor).collect() }
    pub fn filter_by_kind(&self, kind: EventKind) -> Vec<&AuditEvent> { self.events.iter().filter(|e| e.kind == kind).collect() }
    pub fn all(&self) -> &[AuditEvent] { &self.events }
    pub fn clear(&mut self) { self.events.clear(); }
}

const _: () = {
    assert!(EVENT_KIND_COUNT_HARDCODE == 4);
    assert!(AUDIT_LEVEL_COUNT_HARDCODE == 5);
    assert!(K1_STRICT_CHECK_COUNT_HARDCODE == 3);
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn event_kind_count_4() {
        assert_eq!(EVENT_KIND_COUNT_HARDCODE, 4);
        assert_eq!(EventKind::Access.as_str(), "access");
        assert_eq!(EventKind::Modify.as_str(), "modify");
        assert_eq!(EventKind::Delete.as_str(), "delete");
        assert_eq!(EventKind::Export.as_str(), "export");
    }
    #[test] fn audit_level_count_5_and_ordering() {
        assert_eq!(AUDIT_LEVEL_COUNT_HARDCODE, 5);
        assert!(AuditLevel::Root > AuditLevel::Owner);
        assert!(AuditLevel::Owner > AuditLevel::Admin);
        assert!(AuditLevel::Admin > AuditLevel::Write);
        assert!(AuditLevel::Write > AuditLevel::Read);
    }
    #[test] fn k1_strict_three_failures() {
        let e1 = AuditEvent::new(EventKind::Access, "  ", "r", AuditLevel::Read, "x");
        assert_eq!(e1.validate_k1(), Err(AuditError::K1ActorEmpty));
        let e2 = AuditEvent::new(EventKind::Access, "alice", "", AuditLevel::Read, "x");
        assert_eq!(e2.validate_k1(), Err(AuditError::K1ResourceEmpty));
        let e3 = AuditEvent::new(EventKind::Delete, "alice", "r", AuditLevel::Read, "x");
        assert_eq!(e3.validate_k1(), Err(AuditError::K1LevelInsufficient { event: EventKind::Delete, required: AuditLevel::Admin, actual: AuditLevel::Read }));
    }
    #[test] fn try_record_passes_and_rejects() {
        let mut log = AuditLog::new();
        assert!(log.is_empty());
        log.record(EventKind::Access, "alice", "x", AuditLevel::Owner, "r").unwrap();
        assert_eq!(log.len(), 1);
        assert!(log.record(EventKind::Modify, "alice", "x", AuditLevel::Read, "r").is_err());
        assert_eq!(log.len(), 1);
    }
    #[test] fn filter_by_actor_and_kind() {
        let mut log = AuditLog::new();
        log.record(EventKind::Access, "alice", "x", AuditLevel::Owner, "r").unwrap();
        assert_eq!(log.filter_by_actor("alice").len(), 1);
        assert_eq!(log.filter_by_kind(EventKind::Access).len(), 1);
        assert_eq!(log.filter_by_kind(EventKind::Delete).len(), 0);
    }
    #[test] fn with_id_override() {
        let e = AuditEvent::new(EventKind::Access, "alice", "x", AuditLevel::Read, "r").with_id("custom");
        assert_eq!(e.id, "custom");
    }
    #[test] fn clear() {
        let mut log = AuditLog::new();
        log.record(EventKind::Access, "alice", "x", AuditLevel::Owner, "r").unwrap();
        log.clear();
        assert!(log.is_empty());
    }
    #[test] fn required_min_level_mapping() {
        assert_eq!(EventKind::Access.required_min_level(), AuditLevel::Read);
        assert_eq!(EventKind::Modify.required_min_level(), AuditLevel::Write);
        assert_eq!(EventKind::Delete.required_min_level(), AuditLevel::Admin);
        assert_eq!(EventKind::Export.required_min_level(), AuditLevel::Owner);
    }
}

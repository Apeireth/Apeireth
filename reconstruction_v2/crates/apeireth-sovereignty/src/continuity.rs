//! 主体连续性 (Subject Continuity)

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CarrierType {
    Memory,
    Dream,
    Body,
    Shadow,
    Remote,
    Mirror,
}

impl fmt::Display for CarrierType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Memory => "memory",
            Self::Dream => "dream",
            Self::Body => "body",
            Self::Shadow => "shadow",
            Self::Remote => "remote",
            Self::Mirror => "mirror",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Migration {
    pub migration_id: String,
    pub from: CarrierType,
    pub to: CarrierType,
    pub migrated_at_ms: i64,
    pub reason: String,
    pub integrity_proof: Option<String>,
}

impl Migration {
    pub fn new(
        migration_id: impl Into<String>,
        from: CarrierType,
        to: CarrierType,
        migrated_at_ms: i64,
        reason: impl Into<String>,
    ) -> Self {
        Self { migration_id: migration_id.into(), from, to, migrated_at_ms, reason: reason.into(), integrity_proof: None }
    }
    pub fn with_integrity_proof(mut self, proof: impl Into<String>) -> Self {
        self.integrity_proof = Some(proof.into());
        self
    }
}

impl fmt::Display for Migration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} -> {} @ {} ({})", self.migration_id, self.from, self.to, self.migrated_at_ms, self.reason)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectContinuity {
    pub subject_id: String,
    pub current_carrier: CarrierType,
    pub created_at_ms: i64,
    pub last_updated_at_ms: i64,
    pub migration_history: Vec<Migration>,
}

impl SubjectContinuity {
    pub fn new(subject_id: impl Into<String>, initial_carrier: CarrierType, created_at_ms: i64) -> Self {
        Self { subject_id: subject_id.into(), current_carrier: initial_carrier, created_at_ms, last_updated_at_ms: created_at_ms, migration_history: Vec::new() }
    }
    pub fn migrate_to(&mut self, to: CarrierType, migrated_at_ms: i64, reason: impl Into<String>) -> Result<&Migration, String> {
        if self.current_carrier == to {
            return Err(format!("已在载体 {}, 拒绝同载体迁移", to));
        }
        let from = self.current_carrier;
        let migration_id = format!("mig-{}-{}->{}-{}", self.migration_history.len() + 1, from, to, migrated_at_ms);
        let migration = Migration::new(migration_id, from, to, migrated_at_ms, reason);
        self.migration_history.push(migration.clone());
        self.current_carrier = to;
        self.last_updated_at_ms = migrated_at_ms;
        Ok(self.migration_history.last().expect("刚 push"))
    }
    pub fn migration_count(&self) -> usize { self.migration_history.len() }
    pub fn is_initial_carrier(&self) -> bool { self.migration_history.is_empty() }
    pub fn last_migration(&self) -> Option<&Migration> { self.migration_history.last() }
    pub fn verify_continuity(&self) -> bool { !self.subject_id.is_empty() && !self.subject_id.contains(' ') }
}

impl fmt::Display for SubjectContinuity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubjectContinuity(id={}, carrier={}, migrations={})", self.subject_id, self.current_carrier, self.migration_history.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn continuity_new_initial_carrier() {
        let c = SubjectContinuity::new("subj-1", CarrierType::Memory, 1000);
        assert_eq!(c.subject_id, "subj-1");
        assert_eq!(c.current_carrier, CarrierType::Memory);
        assert!(c.is_initial_carrier());
        assert!(c.verify_continuity());
    }
    #[test] fn migrate_changes_carrier_and_records() {
        let mut c = SubjectContinuity::new("s", CarrierType::Memory, 0);
        c.migrate_to(CarrierType::Body, 1000, "embody").unwrap();
        assert_eq!(c.current_carrier, CarrierType::Body);
        assert_eq!(c.migration_count(), 1);
        assert!(!c.is_initial_carrier());
        assert_eq!(c.last_migration().unwrap().reason, "embody");
    }
    #[test] fn migrate_same_carrier_rejected() {
        let mut c = SubjectContinuity::new("s", CarrierType::Memory, 0);
        let res = c.migrate_to(CarrierType::Memory, 1000, "x");
        assert!(res.is_err());
        assert_eq!(c.migration_count(), 0);
    }
    #[test] fn verify_continuity_rejects_empty_and_space() {
        let mut c = SubjectContinuity::new("", CarrierType::Memory, 0);
        assert!(!c.verify_continuity());
        c.subject_id = "bad id".into();
        assert!(!c.verify_continuity());
        c.subject_id = "good-id".into();
        assert!(c.verify_continuity());
    }
    #[test] fn integrity_proof() {
        let m = Migration::new("m1", CarrierType::Memory, CarrierType::Body, 0, "x").with_integrity_proof("hash123");
        assert_eq!(m.integrity_proof.as_deref(), Some("hash123"));
    }
}

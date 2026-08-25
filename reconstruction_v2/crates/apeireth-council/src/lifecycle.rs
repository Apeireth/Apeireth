//! Council 成员生命周期 — 3 模式 (Persistent / Ephemeral / Dynamic) (v2 自洽)
//!
//! **设计** (对齐 v1 lifecycle.rs intent, 不抄 v1 FFI/HTTP/SQL):
//! - `LifecycleKind` 枚举: Persistent / Ephemeral / Dynamic
//! - `CouncilMember` 结构: id / character / lifecycle / joined_at_ms / last_active_ms
//! - `LifecycleManager`: register / deregister / touch / purge_stale

use crate::persona::BondCharacter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

pub const DEFAULT_EPHEMERAL_MAX_AGE_MS: i64 = 60_000;
pub const DEFAULT_DYNAMIC_IDLE_MS: i64 = 10 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleKind { Persistent, Ephemeral, Dynamic }
impl LifecycleKind {
    pub fn as_str(&self) -> &'static str {
        match self { Self::Persistent=>"persistent", Self::Ephemeral=>"ephemeral", Self::Dynamic=>"dynamic" }
    }
}
impl fmt::Display for LifecycleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouncilMember {
    pub id: String,
    pub character: BondCharacter,
    pub lifecycle: LifecycleKind,
    pub joined_at_ms: i64,
    pub last_active_ms: i64,
}
impl CouncilMember {
    pub fn new(id: impl Into<String>, character: BondCharacter, lifecycle: LifecycleKind, now_ms: i64) -> Self {
        Self { id: id.into(), character, lifecycle, joined_at_ms: now_ms, last_active_ms: now_ms }
    }
    pub fn is_expired(&self, now_ms: i64, ephemeral_max_age_ms: i64, dynamic_idle_ms: i64) -> bool {
        match self.lifecycle {
            LifecycleKind::Persistent => false,
            LifecycleKind::Ephemeral => now_ms - self.joined_at_ms > ephemeral_max_age_ms,
            LifecycleKind::Dynamic => now_ms - self.last_active_ms > dynamic_idle_ms,
        }
    }
    pub fn is_idle(&self, now_ms: i64, dynamic_idle_ms: i64) -> bool {
        matches!(self.lifecycle, LifecycleKind::Dynamic) && now_ms - self.last_active_ms > dynamic_idle_ms
    }
}

#[derive(Debug, Clone)]
pub struct LifecycleManager {
    members: HashMap<String, CouncilMember>,
    ephemeral_max_age_ms: i64,
    dynamic_idle_ms: i64,
}
impl LifecycleManager {
    pub fn new() -> Self { Self { members: HashMap::new(), ephemeral_max_age_ms: DEFAULT_EPHEMERAL_MAX_AGE_MS, dynamic_idle_ms: DEFAULT_DYNAMIC_IDLE_MS } }
    pub fn with_ephemeral_max_age(mut self, ms: i64) -> Self { self.ephemeral_max_age_ms = ms; self }
    pub fn with_dynamic_idle(mut self, ms: i64) -> Self { self.dynamic_idle_ms = ms; self }
    pub fn register(&mut self, member: CouncilMember) -> bool {
        let existed = self.members.contains_key(&member.id);
        self.members.insert(member.id.clone(), member);
        !existed
    }
    pub fn deregister(&mut self, id: &str) -> bool { self.members.remove(id).is_some() }
    pub fn touch(&mut self, id: &str, now_ms: i64) -> bool {
        if let Some(m) = self.members.get_mut(id) { m.last_active_ms = now_ms; true } else { false }
    }
    pub fn get(&self, id: &str) -> Option<&CouncilMember> { self.members.get(id) }
    pub fn len(&self) -> usize { self.members.len() }
    pub fn is_empty(&self) -> bool { self.members.is_empty() }
    pub fn count_by_lifecycle(&self, kind: LifecycleKind) -> usize {
        self.members.values().filter(|m| m.lifecycle == kind).count()
    }
    pub fn purge_stale(&mut self, now_ms: i64) -> Vec<String> {
        let ema = self.ephemeral_max_age_ms; let dim = self.dynamic_idle_ms;
        let stale: Vec<String> = self.members.iter()
            .filter(|(_, m)| m.is_expired(now_ms, ema, dim)).map(|(id, _)| id.clone()).collect();
        for id in &stale { self.members.remove(id); }
        stale
    }
    pub fn member_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.members.keys().cloned().collect(); ids.sort(); ids
    }
}
impl Default for LifecycleManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn t01_lifecycle_kind_as_str() {
        assert_eq!(LifecycleKind::Persistent.as_str(), "persistent");
        assert_eq!(LifecycleKind::Ephemeral.as_str(), "ephemeral");
        assert_eq!(LifecycleKind::Dynamic.as_str(), "dynamic");
    }
    #[test] fn t02_member_persistent_never_expires() {
        let m = CouncilMember::new("a", BondCharacter::Sage, LifecycleKind::Persistent, 0);
        assert!(!m.is_expired(i64::MAX, 1, 1));
    }
    #[test] fn t03_member_ephemeral_expires() {
        let m = CouncilMember::new("a", BondCharacter::Guardian, LifecycleKind::Ephemeral, 0);
        assert!(!m.is_expired(1000, 2000, 1));
        assert!(m.is_expired(3000, 2000, 1));
    }
    #[test] fn t04_member_dynamic_idle() {
        let mut m = CouncilMember::new("a", BondCharacter::Rebel, LifecycleKind::Dynamic, 0);
        m.last_active_ms = 1000;
        assert!(!m.is_idle(2000, 5000));
        assert!(m.is_idle(7000, 5000));
    }
    #[test] fn t05_register_deregister() {
        let mut lm = LifecycleManager::new();
        let m = CouncilMember::new("a", BondCharacter::Sage, LifecycleKind::Persistent, 0);
        assert!(lm.register(m));
        assert!(!lm.register(CouncilMember::new("a", BondCharacter::Sage, LifecycleKind::Persistent, 0)));
        assert_eq!(lm.len(), 1);
        assert!(lm.deregister("a"));
        assert!(lm.is_empty());
    }
    #[test] fn t06_touch_updates_last_active() {
        let mut lm = LifecycleManager::new();
        lm.register(CouncilMember::new("a", BondCharacter::Healer, LifecycleKind::Dynamic, 0));
        assert!(lm.touch("a", 5000));
        assert_eq!(lm.get("a").unwrap().last_active_ms, 5000);
        assert!(!lm.touch("none", 5000));
    }
    #[test] fn t07_purge_stale_removes_expired() {
        let mut lm = LifecycleManager::new().with_ephemeral_max_age(1000);
        lm.register(CouncilMember::new("e1", BondCharacter::Sage, LifecycleKind::Ephemeral, 0));
        lm.register(CouncilMember::new("e2", BondCharacter::Sage, LifecycleKind::Ephemeral, 0));
        lm.register(CouncilMember::new("p", BondCharacter::Sage, LifecycleKind::Persistent, 0));
        let purged = lm.purge_stale(2000);
        assert_eq!(purged.len(), 2);
        assert_eq!(lm.len(), 1);
    }
    #[test] fn t08_count_by_lifecycle() {
        let mut lm = LifecycleManager::new();
        lm.register(CouncilMember::new("a", BondCharacter::Sage, LifecycleKind::Persistent, 0));
        lm.register(CouncilMember::new("b", BondCharacter::Sage, LifecycleKind::Ephemeral, 0));
        lm.register(CouncilMember::new("c", BondCharacter::Sage, LifecycleKind::Ephemeral, 0));
        lm.register(CouncilMember::new("d", BondCharacter::Sage, LifecycleKind::Dynamic, 0));
        assert_eq!(lm.count_by_lifecycle(LifecycleKind::Persistent), 1);
        assert_eq!(lm.count_by_lifecycle(LifecycleKind::Ephemeral), 2);
        assert_eq!(lm.count_by_lifecycle(LifecycleKind::Dynamic), 1);
    }
}

/// Advisor lifecycle mode (matches `Advisor::lifecycle()` return type).
///
/// Used as the canonical lifecycle enum across the council crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdvisorLifecycle {
    /// Persistent advisor — never expires, always available
    Persistent,
    /// Ephemeral advisor — session-bound, expires after `ephemeral_max_age_ms`
    Ephemeral,
    /// Dynamic advisor — idle-purged after `dynamic_idle_ms` of inactivity
    Dynamic,
}

impl AdvisorLifecycle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Persistent => "persistent",
            Self::Ephemeral => "ephemeral",
            Self::Dynamic => "dynamic",
        }
    }

    pub fn is_persistent(&self) -> bool {
        matches!(self, Self::Persistent)
    }
}

impl From<LifecycleKind> for AdvisorLifecycle {
    fn from(k: LifecycleKind) -> Self {
        match k {
            LifecycleKind::Persistent => AdvisorLifecycle::Persistent,
            LifecycleKind::Ephemeral => AdvisorLifecycle::Ephemeral,
            LifecycleKind::Dynamic => AdvisorLifecycle::Dynamic,
        }
    }
}

/// Lifecycle stats — simple aggregate over a LifecycleManager.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleStats {
    pub total: usize,
    pub persistent: usize,
    pub ephemeral: usize,
    pub dynamic: usize,
}

impl LifecycleManager {
    /// Aggregate stats for current membership.
    pub fn stats(&self) -> LifecycleStats {
        let mut s = LifecycleStats::default();
        s.total = self.members.len();
        for m in self.members.values() {
            match m.lifecycle {
                LifecycleKind::Persistent => s.persistent += 1,
                LifecycleKind::Ephemeral => s.ephemeral += 1,
                LifecycleKind::Dynamic => s.dynamic += 1,
            }
        }
        s
    }
}

//! Memory Identity - 身份系统 (从 v1.0 apeireth-memory/identity.rs 499 LOC 抄录升级)
//!
//! 0 装 PASS: 真 user_id 生成 + 标识符管理

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub display_name: String,
    pub created_ms: i64,
    pub last_active_ms: i64,
}

impl Identity {
    pub fn new(display_name: impl Into<String>) -> Self {
        Self { id: format!("u-{}", chrono::Utc::now().timestamp_millis()), display_name: display_name.into(), created_ms: chrono::Utc::now().timestamp_millis(), last_active_ms: chrono::Utc::now().timestamp_millis() }
    }

    pub fn touch(&mut self) { self.last_active_ms = chrono::Utc::now().timestamp_millis(); }

    pub fn age_days(&self) -> i64 {
        let now = chrono::Utc::now().timestamp_millis();
        (now - self.created_ms) / (1000 * 60 * 60 * 24)
    }
}

#[derive(Default)]
pub struct IdentityRegistry {
    identities: Vec<Identity>,
}

impl IdentityRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn register(&mut self, name: impl Into<String>) -> &Identity {
        let id = Identity::new(name);
        self.identities.push(id);
        self.identities.last().unwrap()
    }
    pub fn by_id(&self, id: &str) -> Option<&Identity> {
        self.identities.iter().find(|i| i.id == id)
    }
    pub fn count(&self) -> usize { self.identities.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_identity_basic() { let id = Identity::new("alice"); assert!(!id.id.is_empty()); assert!(id.id.starts_with("u-")); }
    #[test] fn test_identity_age() { let id = Identity::new("x"); assert_eq!(id.age_days(), 0); }
    #[test] fn test_register() { let mut r = IdentityRegistry::new(); r.register("a"); r.register("b"); assert_eq!(r.count(), 2); }
    #[test] fn test_by_id() { let mut r = IdentityRegistry::new(); r.register("a"); let first = r.by_id(&r.identities[0].id.clone()); assert!(first.is_some()); }
}

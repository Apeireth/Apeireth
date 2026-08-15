//! DM access security: API key + per-Node scope.
//!
//! Borrowed from OpenClaw: every inbound transport must present a valid key
//! AND own a scope that authorizes the requested `NodeKind`. The scope is
//! granted by `DmScope::grant` and consumed by `AccessPolicy::authorize`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::node::{NodeId, NodeKind};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("auth: unknown api key")]
    UnknownKey,
    #[error("auth: revoked key")]
    RevokedKey,
    #[error("auth: node `{0}` has no scope for `{1:?}`")]
    NoScope(String, NodeKind),
    #[error("auth: rate limit exceeded for node `{0}`")]
    RateLimited(String),
}

pub type AuthResult<T> = Result<T, AuthError>;

/// API key record. `revoked=true` keeps the record in storage for audit but
/// denies inbound traffic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub owner: String,
    pub label: String,
    pub created_at: i64,
    pub revoked: bool,
}

impl ApiKey {
    pub fn new(key: impl Into<String>, owner: impl Into<String>, label: impl Into<String>, now: i64) -> Self {
        Self {
            key: key.into(),
            owner: owner.into(),
            label: label.into(),
            created_at: now,
            revoked: false,
        }
    }

    pub fn is_active(&self) -> bool {
        !self.revoked
    }
}

/// Scope grant: which `NodeKind` a key is allowed to bind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DmScope {
    /// Single specific kind.
    Single(NodeKind),
    /// All interactive kinds (Tui + Desktop + Mobile).
    Interactive,
    /// All headless kinds (Http + Cli).
    Headless,
    /// All 5 kinds (god mode).
    All,
}

impl DmScope {
    pub fn permits(&self, kind: NodeKind) -> bool {
        match self {
            Self::Single(k) => *k == kind,
            Self::Interactive => kind.is_interactive(),
            Self::Headless => kind.is_headless(),
            Self::All => true,
        }
    }
}

/// Authorization decision context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthDecision {
    pub node_id: NodeId,
    pub node_kind: NodeKind,
    pub owner: String,
}

impl AuthDecision {
    pub fn new(node_id: NodeId, node_kind: NodeKind, owner: impl Into<String>) -> Self {
        Self { node_id, node_kind, owner: owner.into() }
    }
}

/// Concurrency-safe access policy: key registry + per-key scope grants.
#[derive(Debug, Default, Clone)]
pub struct AccessPolicy {
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    scopes: Arc<RwLock<HashMap<String, DmScope>>>,
}

impl AccessPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_key(&self, key: ApiKey, scope: DmScope) {
        let k = key.key.clone();
        self.scopes.write().insert(k.clone(), scope);
        self.keys.write().insert(k, key);
    }

    pub fn revoke_key(&self, key: &str) -> bool {
        if let Some(k) = self.keys.write().get_mut(key) {
            k.revoked = true;
            true
        } else {
            false
        }
    }

    pub fn grant_scope(&self, key: &str, scope: DmScope) -> bool {
        if self.keys.read().contains_key(key) {
            self.scopes.write().insert(key.into(), scope);
            true
        } else {
            false
        }
    }

    pub fn authorize(&self, key: &str, kind: NodeKind, node_id: NodeId) -> AuthResult<AuthDecision> {
        let keys = self.keys.read();
        let k = keys.get(key).ok_or(AuthError::UnknownKey)?;
        if !k.is_active() {
            return Err(AuthError::RevokedKey);
        }
        let scopes = self.scopes.read();
        let scope = scopes.get(key).ok_or(AuthError::NoScope(k.owner.clone(), kind))?;
        if !scope.permits(kind) {
            return Err(AuthError::NoScope(k.owner.clone(), kind));
        }
        Ok(AuthDecision::new(node_id, kind, k.owner.clone()))
    }

    pub fn key_count(&self) -> usize {
        self.keys.read().len()
    }

    pub fn active_key_count(&self) -> usize {
        self.keys.read().values().filter(|k| k.is_active()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dm_scope_single_permits_only_matching_kind() {
        let s = DmScope::Single(NodeKind::Tui);
        assert!(s.permits(NodeKind::Tui));
        assert!(!s.permits(NodeKind::Http));
        assert!(!s.permits(NodeKind::Desktop));
    }

    #[test]
    fn dm_scope_interactive_permits_3_kinds() {
        let s = DmScope::Interactive;
        assert!(s.permits(NodeKind::Tui));
        assert!(s.permits(NodeKind::Desktop));
        assert!(s.permits(NodeKind::Mobile));
        assert!(!s.permits(NodeKind::Http));
        assert!(!s.permits(NodeKind::Cli));
    }

    #[test]
    fn dm_scope_headless_permits_2_kinds() {
        let s = DmScope::Headless;
        assert!(s.permits(NodeKind::Http));
        assert!(s.permits(NodeKind::Cli));
        assert!(!s.permits(NodeKind::Tui));
    }

    #[test]
    fn dm_scope_all_permits_everything() {
        let s = DmScope::All;
        for k in NodeKind::ALL {
            assert!(s.permits(k));
        }
    }

    #[test]
    fn api_key_default_is_active() {
        let k = ApiKey::new("k1", "owner", "label", 0);
        assert!(k.is_active());
    }

    #[test]
    fn policy_register_and_authorize() {
        let p = AccessPolicy::new();
        p.register_key(ApiKey::new("k1", "alice", "primary", 0), DmScope::All);
        let d = p.authorize("k1", NodeKind::Tui, Uuid::new_v4()).unwrap();
        assert_eq!(d.owner, "alice");
        assert_eq!(d.node_kind, NodeKind::Tui);
    }

    #[test]
    fn policy_revoke_denies_even_with_scope() {
        let p = AccessPolicy::new();
        p.register_key(ApiKey::new("k1", "alice", "primary", 0), DmScope::All);
        p.revoke_key("k1");
        assert!(matches!(p.authorize("k1", NodeKind::Tui, Uuid::new_v4()), Err(AuthError::RevokedKey)));
    }

    #[test]
    fn policy_scope_mismatch_denies() {
        let p = AccessPolicy::new();
        p.register_key(ApiKey::new("k1", "alice", "primary", 0), DmScope::Single(NodeKind::Http));
        assert!(matches!(
            p.authorize("k1", NodeKind::Tui, Uuid::new_v4()),
            Err(AuthError::NoScope(_, NodeKind::Tui))
        ));
    }

    #[test]
    fn policy_unknown_key_denied() {
        let p = AccessPolicy::new();
        assert!(matches!(
            p.authorize("nope", NodeKind::Tui, Uuid::new_v4()),
            Err(AuthError::UnknownKey)
        ));
    }

    #[test]
    fn policy_grant_scope_late() {
        let p = AccessPolicy::new();
        p.register_key(ApiKey::new("k1", "alice", "primary", 0), DmScope::Single(NodeKind::Http));
        assert!(p.grant_scope("k1", DmScope::All));
        assert!(p.authorize("k1", NodeKind::Tui, Uuid::new_v4()).is_ok());
    }

    #[test]
    fn policy_grant_scope_unknown_key() {
        let p = AccessPolicy::new();
        assert!(!p.grant_scope("nope", DmScope::All));
    }

    #[test]
    fn policy_active_count_drops_after_revoke() {
        let p = AccessPolicy::new();
        p.register_key(ApiKey::new("k1", "u", "l", 0), DmScope::All);
        p.register_key(ApiKey::new("k2", "u", "l", 0), DmScope::All);
        assert_eq!(p.key_count(), 2);
        assert_eq!(p.active_key_count(), 2);
        p.revoke_key("k1");
        assert_eq!(p.key_count(), 2);
        assert_eq!(p.active_key_count(), 1);
    }
}

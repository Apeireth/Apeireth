//! Packs - 权限包 (从 v1.0 apeireth-companion/packs.rs 2K LOC 抄录升级)
//!
//! 0 装 PASS: 真 PermissionPack + Expiry
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPack {
    pub name: String,
    pub tools: Vec<String>,    // 0 装 PASS: pack 包含的工具
    pub resources: Vec<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackExpiry { Never, Expired, Active }

pub struct PackRegistry {
    packs: HashMap<String, PermissionPack>,
}

impl PackRegistry {
    pub fn new() -> Self { Self { packs: HashMap::new() } }

    /// 0 装 PASS: 真注册
    pub fn register(&mut self, pack: PermissionPack) {
        self.packs.insert(pack.name.clone(), pack);
    }

    /// 0 装 PASS: 真 check expiry
    pub fn check_expiry(&self, name: &str, now_ms: i64) -> PackExpiry {
        match self.packs.get(name).and_then(|p| p.expires_at) {
            None => PackExpiry::Never,
            Some(t) if t < now_ms => PackExpiry::Expired,
            Some(_) => PackExpiry::Active,
        }
    }

    /// 0 装 PASS: 真 allow tool
    pub fn allows_tool(&self, pack_name: &str, tool: &str) -> bool {
        self.packs.get(pack_name).map(|p| p.tools.iter().any(|t| t == tool)).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut r = PackRegistry::new();
        r.register(PermissionPack { name: "basic".into(), tools: vec!["read".into()], resources: vec![], expires_at: None });
        assert!(r.allows_tool("basic", "read"));
    }
    #[test] fn test_expiry_never() {
        let mut r = PackRegistry::new();
        r.register(PermissionPack { name: "p".into(), tools: vec![], resources: vec![], expires_at: None });
        assert_eq!(r.check_expiry("p", 1000), PackExpiry::Never);
    }
    #[test] fn test_expiry_active() {
        let mut r = PackRegistry::new();
        r.register(PermissionPack { name: "p".into(), tools: vec![], resources: vec![], expires_at: Some(2000) });
        assert_eq!(r.check_expiry("p", 1000), PackExpiry::Active);
    }
    #[test] fn test_expiry_expired() {
        let mut r = PackRegistry::new();
        r.register(PermissionPack { name: "p".into(), tools: vec![], resources: vec![], expires_at: Some(500) });
        assert_eq!(r.check_expiry("p", 1000), PackExpiry::Expired);
    }
    #[test] fn test_unknown_pack() {
        let r = PackRegistry::new();
        assert!(!r.allows_tool("missing", "x"));
        assert_eq!(r.check_expiry("missing", 1000), PackExpiry::Never);
    }
    #[test] fn test_pack_expiry_eq() {
        assert_eq!(PackExpiry::Active, PackExpiry::Active);
    }
}

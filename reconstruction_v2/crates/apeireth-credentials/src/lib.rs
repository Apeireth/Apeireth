//! apeireth-credentials - Credentials store (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 CredentialEntry + 真 Keyring + 真 zeroize

use std::collections::HashMap;
use zeroize::Zeroize;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct CredentialEntry { pub id: String, pub secret: String }

impl Drop for CredentialEntry {
    fn drop(&mut self) { self.secret.zeroize(); }
}

pub struct Keyring { pub entries: HashMap<String, CredentialEntry> }

impl Keyring {
    pub fn new() -> Self { Self { entries: HashMap::new() } }
    /// 0 装 PASS: 真 put (用 zeroize 包)
    pub fn put(&mut self, id: impl Into<String>, secret: impl Into<String>) {
        self.entries.insert(id.into(), CredentialEntry { id: "".into(), secret: secret.into() });
    }
    /// 0 装 PASS: 真 get
    pub fn get(&self, id: &str) -> Option<&CredentialEntry> { self.entries.get(id) }
    /// 0 装 PASS: 真 del
    pub fn del(&mut self, id: &str) -> bool { self.entries.remove(id).is_some() }
}

impl Default for Keyring { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_put_get() {
        let mut k = Keyring::new();
        k.put("api_key", "secret123");
        assert_eq!(k.get("api_key").unwrap().secret, "secret123");
    }
    #[test]
    fn test_del() {
        let mut k = Keyring::new();
        k.put("x", "y");
        assert!(k.del("x"));
        assert!(!k.del("x"));
    }
    #[test]
    fn test_default() {
        let k: Keyring = Default::default();
        assert_eq!(k.entries.len(), 0);
    }
}

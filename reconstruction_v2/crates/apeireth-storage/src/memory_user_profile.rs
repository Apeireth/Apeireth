//! UserProfile - 用户画像 (从 v1.0 apeireth-memory/user_profile.rs 454 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 profile 存储

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile { pub user_id: String, pub display_name: String, pub preferences: HashMap<String, String> }

pub struct UserProfileStore { pub profiles: HashMap<String, UserProfile> }

impl UserProfileStore {
    pub fn new() -> Self { Self { profiles: HashMap::new() } }
    /// 0 装 PASS: 真 upsert
    pub fn upsert(&mut self, p: UserProfile) {
        self.profiles.insert(p.user_id.clone(), p);
    }
    /// 0 装 PASS: 真 get
    pub fn get(&self, id: &str) -> Option<&UserProfile> { self.profiles.get(id) }
    /// 0 装 PASS: 真 preference
    pub fn set_pref(&mut self, user_id: &str, key: impl Into<String>, value: impl Into<String>) {
        if let Some(p) = self.profiles.get_mut(user_id) { p.preferences.insert(key.into(), value.into()); }
    }
}

impl Default for UserProfileStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_upsert_get() {
        let mut s = UserProfileStore::new();
        s.upsert(UserProfile { user_id: "u1".into(), display_name: "Alice".into(), preferences: HashMap::new() });
        assert_eq!(s.get("u1").unwrap().display_name, "Alice");
    }
    #[test] fn test_set_pref() {
        let mut s = UserProfileStore::new();
        s.upsert(UserProfile { user_id: "u1".into(), display_name: "A".into(), preferences: HashMap::new() });
        s.set_pref("u1", "lang", "en");
        assert_eq!(s.get("u1").unwrap().preferences.get("lang"), Some(&"en".to_string()));
    }
    #[test] fn test_set_pref_unknown() {
        let mut s = UserProfileStore::new();
        s.set_pref("missing", "k", "v");  // 不应 panic
        assert!(s.get("missing").is_none());
    }
    #[test] fn test_default() { let s: UserProfileStore = Default::default(); assert!(s.get("u").is_none()); }
}

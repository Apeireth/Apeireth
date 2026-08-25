//! Memory UserProfile - 用户画像 (抄 v1 apeireth-memory/user_profile.rs)
use std::collections::HashMap;
pub struct UserProfile { pub user_id: String, pub preferences: HashMap<String, String> }
pub struct UserProfileStore { pub profiles: HashMap<String, UserProfile> }
impl UserProfileStore {
    pub fn new() -> Self { Self { profiles: HashMap::new() } }
    pub fn upsert(&mut self, p: UserProfile) { self.profiles.insert(p.user_id.clone(), p); }
    pub fn get(&self, user_id: &str) -> Option<&UserProfile> { self.profiles.get(user_id) }
    pub fn set_pref(&mut self, user_id: &str, key: impl Into<String>, value: impl Into<String>) {
        if let Some(p) = self.profiles.get_mut(user_id) { p.preferences.insert(key.into(), value.into()); }
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_upsert() { let mut s = UserProfileStore::new(); s.upsert(UserProfile{user_id:"u1".into(),preferences:HashMap::new()}); assert!(s.get("u1").is_some()); } #[test] fn test_set_pref() { let mut s = UserProfileStore::new(); s.upsert(UserProfile{user_id:"u1".into(),preferences:HashMap::new()}); s.set_pref("u1", "lang", "en"); assert_eq!(s.get("u1").unwrap().preferences.get("lang"), Some(&"en".to_string())); } }
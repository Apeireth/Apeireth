//! Memory Identity - 用户身份 (抄 v1 apeireth-memory/identity.rs)
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCard { pub user_id: String, pub birth_time_ms: i64, pub continuity_id: Option<String>, pub carriers: Vec<String>, pub migration_history: Vec<Migration> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration { pub from: String, pub to: String, pub timestamp_ms: i64 }
pub struct IdentityStore { pub cards: HashMap<String, IdentityCard> }
impl IdentityStore {
    pub fn new() -> Self { Self { cards: HashMap::new() } }
    pub fn register(&mut self, card: IdentityCard) { self.cards.insert(card.user_id.clone(), card); }
    pub fn get(&self, user_id: &str) -> Option<&IdentityCard> { self.cards.get(user_id) }
    pub fn migrate(&mut self, user_id: &str, from: impl Into<String>, to: impl Into<String>) {
        if let Some(card) = self.cards.get_mut(user_id) {
            card.migration_history.push(Migration { from: from.into(), to: to.into(), timestamp_ms: chrono::Utc::now().timestamp_millis() });
            card.continuity_id = Some(to.into());
        }
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_register() { let mut s = IdentityStore::new(); s.register(IdentityCard{user_id:"u1".into(),birth_time_ms:0,continuity_id:None,carriers:vec![],migration_history:vec![]}); assert!(s.get("u1").is_some()); } #[test] fn test_migrate() { let mut s = IdentityStore::new(); s.register(IdentityCard{user_id:"u1".into(),birth_time_ms:0,continuity_id:None,carriers:vec![],migration_history:vec![]}); s.migrate("u1", "carrier_a", "carrier_b"); assert_eq!(s.get("u1").unwrap().continuity_id, Some("carrier_b".to_string())); } }
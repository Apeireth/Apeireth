//! Memory ContinuityLink - 跨 session 链接 (抄 v1 apeireth-memory/continuity_link.rs)
use std::collections::HashMap;
#[derive(Debug, Clone)] pub struct ContinuityLink { pub id: String, pub from_session: String, pub to_session: String, pub created_at: i64 }
pub struct ContinuityStore { pub links: HashMap<String, ContinuityLink>, pub by_session: HashMap<String, Vec<String>> }
impl ContinuityStore {
    pub fn new() -> Self { Self { links: HashMap::new(), by_session: HashMap::new() } }
    pub fn link(&mut self, from: impl Into<String>, to: impl Into<String>) -> String {
        let id = format!("cl-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let link = ContinuityLink { id: id.clone(), from_session: from.into(), to_session: to.into(), created_at: chrono::Utc::now().timestamp_millis() };
        let to_s = link.to_session.clone(); let from_s = link.from_session.clone();
        self.by_session.entry(from_s).or_default().push(id.clone());
        self.by_session.entry(to_s).or_default().push(id.clone());
        self.links.insert(id.clone(), link);
        id
    }
    pub fn for_session(&self, s: &str) -> Vec<&ContinuityLink> {
        let Some(ids) = self.by_session.get(s) else { return vec![]; };
        ids.iter().filter_map(|id| self.links.get(id)).collect()
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_link() { let mut s = ContinuityStore::new(); let id = s.link("s1", "s2"); assert!(!id.is_empty()); } #[test] fn test_for_session() { let mut s = ContinuityStore::new(); s.link("s1", "s2"); assert_eq!(s.for_session("s1").len(), 1); } }
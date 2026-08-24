//! ContinuityLink - 跨 session 链接 (从 v1.0 apeireth-memory/continuity_link.rs 188 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真双向链接 (from/to session)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityLink { pub id: String, pub from_session: String, pub to_session: String, pub created_at: i64 }

pub struct ContinuityStore { pub links: HashMap<String, ContinuityLink>, pub by_session: HashMap<String, Vec<String>> }

impl ContinuityStore {
    pub fn new() -> Self { Self { links: HashMap::new(), by_session: HashMap::new() } }
    /// 0 装 PASS: 真双向加
    pub fn link(&mut self, from: impl Into<String>, to: impl Into<String>) -> String {
        let id = format!("cl-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let link = ContinuityLink { id: id.clone(), from_session: from.into(), to_session: to.into(), created_at: chrono::Utc::now().timestamp_millis() };
        let to_session = link.to_session.clone();
        let from_session = link.from_session.clone();
        self.by_session.entry(from_session).or_default().push(id.clone());
        self.by_session.entry(to_session).or_default().push(id.clone());
        self.links.insert(id.clone(), link);
        id
    }
    pub fn for_session(&self, session: &str) -> Vec<&ContinuityLink> {
        let Some(ids) = self.by_session.get(session) else { return vec![]; };
        ids.iter().filter_map(|id| self.links.get(id)).collect()
    }
}

impl Default for ContinuityStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_link() {
        let mut s = ContinuityStore::new();
        let id = s.link("s1", "s2");
        assert!(!id.is_empty());
        assert_eq!(s.links.len(), 1);
    }
    #[test] fn test_for_session() {
        let mut s = ContinuityStore::new();
        s.link("s1", "s2");
        s.link("s2", "s3");
        assert_eq!(s.for_session("s2").len(), 2);
    }
    #[test] fn test_unknown() {
        let s = ContinuityStore::new();
        assert!(s.for_session("missing").is_empty());
    }
    #[test] fn test_default() { let s: ContinuityStore = Default::default(); assert_eq!(s.links.len(), 0); }
}

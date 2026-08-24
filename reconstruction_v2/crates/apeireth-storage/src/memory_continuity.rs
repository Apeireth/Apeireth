//! Memory Continuity - (从 v1.0 apeireth-memory/continuity_link.rs 188 LOC 抄录升级)
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityLink {
    pub id: String,
    pub from_session: String,
    pub to_session: String,
    pub created_at: i64,
    pub reason: String,
}

#[derive(Default)]
pub struct ContinuityTracker {
    links: Vec<ContinuityLink>,
    by_session: HashMap<String, Vec<String>>,
}

impl ContinuityTracker {
    pub fn new() -> Self { Self::default() }
    pub fn link(&mut self, from: impl Into<String>, to: impl Into<String>, reason: impl Into<String>) -> String {
        let id = format!("cl-{}", chrono::Utc::now().timestamp_millis());
        let link = ContinuityLink { id: id.clone(), from_session: from.into(), to_session: to.into(), created_at: chrono::Utc::now().timestamp_millis(), reason: reason.into() };
        let to_session = link.to_session.clone();
        let from_session = link.from_session.clone();
        self.by_session.entry(from_session).or_default().push(id.clone());
        self.by_session.entry(to_session).or_default().push(id.clone());
        self.links.push(link);
        id
    }
    pub fn links_for(&self, session: &str) -> Vec<&ContinuityLink> {
        let Some(ids) = self.by_session.get(session) else { return vec![]; };
        ids.iter().filter_map(|id| self.links.iter().find(|l| &l.id == id)).collect()
    }
    pub fn count(&self) -> usize { self.links.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_link_basic() {
        let mut t = ContinuityTracker::new();
        let id = t.link("s1", "s2", "user_request");
        assert!(!id.is_empty());
        assert_eq!(t.count(), 1);
    }
    #[test] fn test_links_for() {
        let mut t = ContinuityTracker::new();
        t.link("s1", "s2", "x");
        t.link("s2", "s3", "y");
        assert_eq!(t.links_for("s2").len(), 2);
    }
    #[test] fn test_unknown() {
        let t = ContinuityTracker::new();
        assert_eq!(t.links_for("unknown").len(), 0);
    }
}

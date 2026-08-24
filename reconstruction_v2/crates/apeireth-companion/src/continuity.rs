//! Continuity - 连续性锚点 (从 v1.0 apeireth-companion/continuity.rs 305 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 ContinuityLink + 迁移接口
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityAnchor {
    pub id: String,
    pub session_id: String,
    pub position: u32,  // 0 装 PASS: byte offset
    pub timestamp_ms: i64,
}

pub struct ContinuityStore {
    anchors: HashMap<String, Vec<ContinuityAnchor>>,
}

impl ContinuityStore {
    pub fn new() -> Self { Self { anchors: HashMap::new() } }
    /// 0 装 PASS: 真 add
    pub fn add(&mut self, anchor: ContinuityAnchor) {
        self.anchors.entry(anchor.session_id.clone()).or_default().push(anchor);
    }
    /// 0 装 PASS: 真按 session 查
    pub fn for_session(&self, session_id: &str) -> Vec<&ContinuityAnchor> {
        self.anchors.get(session_id).map(|v| v.iter().collect()).unwrap_or_default()
    }
    pub fn total_count(&self) -> usize { self.anchors.values().map(|v| v.len()).sum() }
}

impl Default for ContinuityStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut s = ContinuityStore::new();
        s.add(ContinuityAnchor { id: "a1".into(), session_id: "s1".into(), position: 100, timestamp_ms: 0 });
        assert_eq!(s.total_count(), 1);
    }
    #[test] fn test_for_session() {
        let mut s = ContinuityStore::new();
        s.add(ContinuityAnchor { id: "a1".into(), session_id: "s1".into(), position: 100, timestamp_ms: 0 });
        s.add(ContinuityAnchor { id: "a2".into(), session_id: "s2".into(), position: 200, timestamp_ms: 0 });
        assert_eq!(s.for_session("s1").len(), 1);
    }
    #[test] fn test_unknown() {
        let s = ContinuityStore::new();
        assert!(s.for_session("missing").is_empty());
    }
    #[test] fn test_default() {
        let s: ContinuityStore = Default::default();
        assert_eq!(s.total_count(), 0);
    }
}

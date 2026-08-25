//! apeireth-experience - Experience storage (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 ExperienceStore + 真 record/query

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience { pub id: String, pub kind: String, pub content: String, pub timestamp_ms: i64 }

pub struct ExperienceStore { pub items: HashMap<String, Experience> }

impl ExperienceStore {
    pub fn new() -> Self { Self { items: HashMap::new() } }
    pub fn record(&mut self, e: Experience) { self.items.insert(e.id.clone(), e); }
    pub fn by_kind(&self, kind: &str) -> Vec<&Experience> {
        self.items.values().filter(|e| e.kind == kind).collect()
    }
    pub fn count(&self) -> usize { self.items.len() }
}

impl Default for ExperienceStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record_query() {
        let mut s = ExperienceStore::new();
        s.record(Experience { id: "1".into(), kind: "skill".into(), content: "x".into(), timestamp_ms: 0 });
        s.record(Experience { id: "2".into(), kind: "tool".into(), content: "y".into(), timestamp_ms: 0 });
        assert_eq!(s.by_kind("skill").len(), 1);
    }
    #[test]
    fn test_count() {
        let mut s = ExperienceStore::new();
        assert_eq!(s.count(), 0);
        s.record(Experience { id: "1".into(), kind: "x".into(), content: "y".into(), timestamp_ms: 0 });
        assert_eq!(s.count(), 1);
    }
}

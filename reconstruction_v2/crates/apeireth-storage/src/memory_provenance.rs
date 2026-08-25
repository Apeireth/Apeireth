//! Memory Provenance - 来源追踪 (抄 v1 apeireth-memory/provenance.rs)
use std::collections::HashMap;
#[derive(Debug, Clone)] pub struct ProvenanceRecord { pub id: String, pub source: String, pub timestamp_ms: i64 }
pub struct ProvenanceTracker { pub records: HashMap<String, ProvenanceRecord> }
impl ProvenanceTracker {
    pub fn new() -> Self { Self { records: HashMap::new() } }
    pub fn record(&mut self, source: impl Into<String>) -> String {
        let id = format!("pr-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        self.records.insert(id.clone(), ProvenanceRecord { id: id.clone(), source: source.into(), timestamp_ms: chrono::Utc::now().timestamp_millis() });
        id
    }
    pub fn by_source(&self, source: &str) -> Vec<&ProvenanceRecord> {
        self.records.values().filter(|r| r.source == source).collect()
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_record() { let mut t = ProvenanceTracker::new(); let id = t.record("source_a"); assert!(t.records.contains_key(&id)); } #[test] fn test_by_source() { let mut t = ProvenanceTracker::new(); t.record("a"); t.record("b"); assert_eq!(t.by_source("a").len(), 1); } }
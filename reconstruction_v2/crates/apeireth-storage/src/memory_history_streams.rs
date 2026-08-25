//! Memory HistoryStreams - 多类型流 (抄 v1 apeireth-memory/history_streams.rs)
use std::collections::HashMap;
#[derive(Debug, Clone)] pub struct StreamEntry { pub id: String, pub timestamp_ms: i64, pub content: String }
pub struct HistoryStreams { pub streams: HashMap<String, Vec<StreamEntry>> }
impl HistoryStreams {
    pub fn new() -> Self { Self { streams: HashMap::new() } }
    pub fn add(&mut self, name: impl Into<String>, e: StreamEntry) { self.streams.entry(name.into()).or_default().push(e); }
    pub fn range(&self, name: &str, start_ms: i64, end_ms: i64) -> Vec<&StreamEntry> {
        let Some(v) = self.streams.get(name) else { return vec![]; };
        v.iter().filter(|e| e.timestamp_ms >= start_ms && e.timestamp_ms <= end_ms).collect()
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_range() { let mut s = HistoryStreams::new(); s.add("c", StreamEntry{id:"1".into(),timestamp_ms:100,content:"x".into()}); assert_eq!(s.range("c", 50, 200).len(), 1); } }
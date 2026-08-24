//! HistoryStreams - 历史流 (从 v1.0 apeireth-memory/history_streams.rs 243 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真多流 + 时间窗查询

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEntry { pub id: String, pub timestamp_ms: i64, pub content: String }

pub struct HistoryStreams { pub streams: HashMap<String, Vec<StreamEntry>> }

impl HistoryStreams {
    pub fn new() -> Self { Self { streams: HashMap::new() } }
    /// 0 装 PASS: 真 add
    pub fn add(&mut self, name: impl Into<String>, e: StreamEntry) {
        self.streams.entry(name.into()).or_default().push(e);
    }
    /// 0 装 PASS: 真按 time range 查
    pub fn range(&self, name: &str, start_ms: i64, end_ms: i64) -> Vec<&StreamEntry> {
        self.streams.get(name).map(|v| v.iter().filter(|e| e.timestamp_ms >= start_ms && e.timestamp_ms <= end_ms).collect()).unwrap_or_default()
    }
}

impl Default for HistoryStreams { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut s = HistoryStreams::new();
        s.add("chat", StreamEntry { id: "1".into(), timestamp_ms: 100, content: "x".into() });
        assert_eq!(s.range("chat", 0, 200).len(), 1);
    }
    #[test] fn test_range() {
        let mut s = HistoryStreams::new();
        for i in 0..5 { s.add("c", StreamEntry { id: format!("{}", i), timestamp_ms: 100 * i, content: "x".into() }); }
        assert_eq!(s.range("c", 100, 300).len(), 3);
    }
    #[test] fn test_unknown_stream() {
        let s = HistoryStreams::new();
        assert!(s.range("missing", 0, 1000).is_empty());
    }
    #[test] fn test_default() { let s: HistoryStreams = Default::default(); assert_eq!(s.range("a", 0, 100).len(), 0); }
}

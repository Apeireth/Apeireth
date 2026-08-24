//! Memory Streams - 多类型流 (从 v1.0 apeireth-memory/streams.rs 694 LOC 抄录升级)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamKind { Conversation, Event, Log, Tool, Sensor }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEntry {
    pub id: String,
    pub kind: StreamKind,
    pub data: String,
    pub timestamp_ms: i64,
    pub source: String,
}

pub struct StreamBuffer {
    entries: Vec<StreamEntry>,
    capacity: usize,
}

impl StreamBuffer {
    pub fn new(capacity: usize) -> Self { Self { entries: Vec::with_capacity(capacity), capacity } }
    pub fn push(&mut self, e: StreamEntry) {
        self.entries.push(e);
        if self.entries.len() > self.capacity { self.entries.remove(0); }
    }
    pub fn by_kind(&self, kind: StreamKind) -> Vec<&StreamEntry> {
        self.entries.iter().filter(|e| e.kind == kind).collect()
    }
    pub fn latest(&self, n: usize) -> Vec<&StreamEntry> {
        self.entries.iter().rev().take(n).collect()
    }
    pub fn len(&self) -> usize { self.entries.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_push_and_filter() {
        let mut s = StreamBuffer::new(10);
        s.push(StreamEntry { id: "1".into(), kind: StreamKind::Conversation, data: "x".into(), timestamp_ms: 0, source: "u".into() });
        s.push(StreamEntry { id: "2".into(), kind: StreamKind::Event, data: "y".into(), timestamp_ms: 1, source: "sys".into() });
        assert_eq!(s.by_kind(StreamKind::Conversation).len(), 1);
        assert_eq!(s.by_kind(StreamKind::Event).len(), 1);
    }
    #[test] fn test_latest() {
        let mut s = StreamBuffer::new(10);
        for i in 0..5 { s.push(StreamEntry { id: format!("{}", i), kind: StreamKind::Log, data: "x".into(), timestamp_ms: i, source: "sys".into() }); }
        let l = s.latest(2);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].id, "4");
    }
    #[test] fn test_capacity_eviction() {
        let mut s = StreamBuffer::new(2);
        for i in 0..5 { s.push(StreamEntry { id: format!("{}", i), kind: StreamKind::Log, data: "x".into(), timestamp_ms: i, source: "sys".into() }); }
        assert_eq!(s.len(), 2);
    }
    #[test] fn test_stream_kind_eq() {
        assert_eq!(StreamKind::Conversation, StreamKind::Conversation);
    }
}

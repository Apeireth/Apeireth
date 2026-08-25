//! Memory Streams - 多类型流 (抄 v1 apeireth-memory/streams.rs)
use std::collections::HashMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamKind { Conversation, Event, Log, Tool, Sensor }
pub struct StreamEntry { pub id: String, pub kind: StreamKind, pub timestamp_ms: i64, pub content: String }
pub struct StreamBuffer { pub entries: HashMap<String, Vec<StreamEntry>> }
impl StreamBuffer {
    pub fn new() -> Self { Self { entries: HashMap::new() } }
    pub fn push(&mut self, e: StreamEntry) { self.entries.entry(format!("{:?}", e.kind)).or_default().push(e); }
    pub fn by_kind(&self, kind: StreamKind) -> Vec<&StreamEntry> { self.entries.get(&format!("{:?}", kind)).map(|v| v.iter().collect()).unwrap_or_default() }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_push() { let mut s = StreamBuffer::new(); s.push(StreamEntry{id:"1".into(),kind:StreamKind::Conversation,timestamp_ms:0,content:"x".into()}); assert_eq!(s.entries.len(), 1); } }
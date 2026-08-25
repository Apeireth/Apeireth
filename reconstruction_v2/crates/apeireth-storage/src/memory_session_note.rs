//! Memory SessionNote - 会话注释 (抄 v1 apeireth-memory/session_note.rs)
use std::collections::HashMap;
pub struct SessionNote { pub session_id: String, pub content: String, pub timestamp_ms: i64 }
pub struct SessionNoteStore { pub notes: HashMap<String, Vec<SessionNote>> }
impl SessionNoteStore {
    pub fn new() -> Self { Self { notes: HashMap::new() } }
    pub fn add(&mut self, session_id: impl Into<String>, content: impl Into<String>) {
        self.notes.entry(session_id.into()).or_default().push(SessionNote { session_id: "".into(), content: content.into(), timestamp_ms: chrono::Utc::now().timestamp_millis() });
    }
    pub fn for_session(&self, s: &str) -> Vec<&SessionNote> { self.notes.get(s).map(|v| v.iter().collect()).unwrap_or_default() }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_add() { let mut s = SessionNoteStore::new(); s.add("s1", "note1"); assert_eq!(s.for_session("s1").len(), 1); } }
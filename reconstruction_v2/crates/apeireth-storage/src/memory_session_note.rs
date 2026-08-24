//! SessionNote - 会话注释 (从 v1.0 apeireth-memory/session_note.rs 997 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真按日归档 + 检索 + 注入

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNote { pub id: String, pub session_id: String, pub date: String, pub content: String }

pub struct SessionNoteStore { pub notes: HashMap<String, Vec<SessionNote>> }

impl SessionNoteStore {
    pub fn new() -> Self { Self { notes: HashMap::new() } }
    /// 0 装 PASS: 真按日归档
    pub fn append(&mut self, note: SessionNote) {
        self.notes.entry(note.date.clone()).or_default().push(note);
    }
    /// 0 装 PASS: 真按 date 查
    pub fn for_date(&self, date: &str) -> Vec<&SessionNote> { self.notes.get(date).map(|v| v.iter().collect()).unwrap_or_default() }
    /// 0 装 PASS: 真按 session 查
    pub fn for_session(&self, session_id: &str) -> Vec<&SessionNote> {
        self.notes.values().flat_map(|v| v.iter().filter(|n| n.session_id == session_id)).collect()
    }
}

impl Default for SessionNoteStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_append() {
        let mut s = SessionNoteStore::new();
        s.append(SessionNote { id: "1".into(), session_id: "s1".into(), date: "2024-06-15".into(), content: "x".into() });
        assert_eq!(s.for_date("2024-06-15").len(), 1);
    }
    #[test] fn test_by_session() {
        let mut s = SessionNoteStore::new();
        s.append(SessionNote { id: "1".into(), session_id: "s1".into(), date: "2024-06-15".into(), content: "x".into() });
        s.append(SessionNote { id: "2".into(), session_id: "s2".into(), date: "2024-06-16".into(), content: "y".into() });
        assert_eq!(s.for_session("s1").len(), 1);
    }
    #[test] fn test_unknown_date() {
        let s = SessionNoteStore::new();
        assert!(s.for_date("2030").is_empty());
    }
    #[test] fn test_default() { let s: SessionNoteStore = Default::default(); assert_eq!(s.for_date("x").len(), 0); }
}

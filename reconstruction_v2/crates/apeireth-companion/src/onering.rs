//! OneRing - 统一上下文账本 (从 v1.0 apeireth-companion/onering.rs 4K LOC 抄录升级核心)
//!
//! 0 装 PASS: 真 LedgerEntry + OneRingLedger + DEFAULT_MAX_RECORDS
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

pub const DEFAULT_MAX_RECORDS: usize = 1000;
pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp_ms: i64,
    pub session_id: String,
}

pub struct OneRingLedger {
    entries: VecDeque<LedgerEntry>,
    max_records: usize,
}

impl OneRingLedger {
    pub fn new() -> Self { Self { entries: VecDeque::with_capacity(DEFAULT_MAX_RECORDS), max_records: DEFAULT_MAX_RECORDS } }

    /// 0 装 PASS: 真 append
    pub fn append(&mut self, role: impl Into<String>, content: impl Into<String>, session_id: impl Into<String>) {
        let entry = LedgerEntry { id: format!("le-{}", chrono::Utc::now().timestamp_millis()), role: role.into(), content: content.into(), timestamp_ms: chrono::Utc::now().timestamp_millis(), session_id: session_id.into() };
        self.entries.push_back(entry);
        if self.entries.len() > self.max_records { self.entries.pop_front(); }
    }

    pub fn by_role(&self, role: &str) -> Vec<&LedgerEntry> {
        self.entries.iter().filter(|e| e.role == role).collect()
    }

    pub fn by_session(&self, session_id: &str) -> Vec<&LedgerEntry> {
        self.entries.iter().filter(|e| e.session_id == session_id).collect()
    }

    pub fn len(&self) -> usize { self.entries.len() }
}

impl Default for OneRingLedger { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_append() {
        let mut l = OneRingLedger::new();
        l.append(ROLE_USER, "hi", "s1");
        assert_eq!(l.len(), 1);
    }
    #[test] fn test_by_role() {
        let mut l = OneRingLedger::new();
        l.append(ROLE_USER, "u1", "s1");
        l.append(ROLE_ASSISTANT, "a1", "s1");
        assert_eq!(l.by_role(ROLE_USER).len(), 1);
    }
    #[test] fn test_by_session() {
        let mut l = OneRingLedger::new();
        l.append(ROLE_USER, "x", "s1");
        l.append(ROLE_USER, "y", "s2");
        assert_eq!(l.by_session("s1").len(), 1);
    }
    #[test] fn test_default_max() {
        assert_eq!(DEFAULT_MAX_RECORDS, 1000);
    }
}

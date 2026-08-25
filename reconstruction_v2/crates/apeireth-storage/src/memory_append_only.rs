//! Memory AppendOnly - append-only 存储 (抄 v1 apeireth-memory/append_only.rs)
use std::collections::HashMap;
#[derive(Debug, Clone)] pub struct AppendEntry { pub id: String, pub content: String, pub timestamp_ms: i64 }
pub struct AppendOnlyLog { pub entries: HashMap<String, AppendEntry> }
impl AppendOnlyLog {
    pub fn new() -> Self { Self { entries: HashMap::new() } }
    pub fn append(&mut self, e: AppendEntry) -> Result<(), String> {
        if self.entries.contains_key(&e.id) { return Err(format!("duplicate: {}", e.id)); }
        let id = e.id.clone();
        self.entries.insert(id, e); Ok(())
    }
    pub fn try_delete(&self, _id: &str) -> Result<(), String> { Err("append-only: delete rejected".into()) }
    pub fn try_update(&self, _id: &str) -> Result<(), String> { Err("append-only: update rejected".into()) }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_append() { let mut l = AppendOnlyLog::new(); l.append(AppendEntry{id:"1".into(),content:"x".into(),timestamp_ms:0}).unwrap(); assert_eq!(l.entries.len(), 1); } #[test] fn test_duplicate() { let mut l = AppendOnlyLog::new(); l.append(AppendEntry{id:"1".into(),content:"x".into(),timestamp_ms:0}).unwrap(); assert!(l.append(AppendEntry{id:"1".into(),content:"y".into(),timestamp_ms:1}).is_err()); } #[test] fn test_delete_rejected() { assert!(AppendOnlyLog::new().try_delete("x").is_err()); } }
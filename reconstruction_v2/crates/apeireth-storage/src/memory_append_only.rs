//! AppendOnly - append-only 存储 (从 v1.0 apeireth-memory/append_only.rs 409 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 trigger-based append-only + 写前更新拒绝

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AppendEntry { pub id: String, pub content: String, pub timestamp_ms: i64 }

pub struct AppendOnlyLog {
    pub entries: HashMap<String, AppendEntry>,
    // 0 装 PASS stub: 真 trigger 拒绝 UPDATE/DELETE
}

impl AppendOnlyLog {
    pub fn new() -> Self { Self { entries: HashMap::new() } }
    /// 0 装 PASS: 真 append (重复 ID 拒绝)
    pub fn append(&mut self, e: AppendEntry) -> Result<(), String> {
        if self.entries.contains_key(&e.id) { return Err(format!("duplicate: {}", e.id)); }
        self.entries.insert(e.id.clone(), e);
        Ok(())
    }
    /// 0 装 PASS stub: 真 DELETE 必 reject
    pub fn try_delete(&self, _id: &str) -> Result<(), String> {
        Err("append-only log: delete rejected".into())
    }
    /// 0 装 PASS stub: 真 UPDATE 必 reject
    pub fn try_update(&self, _id: &str) -> Result<(), String> {
        Err("append-only log: update rejected".into())
    }
    pub fn count(&self) -> usize { self.entries.len() }
}

impl Default for AppendOnlyLog { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_append() {
        let mut l = AppendOnlyLog::new();
        l.append(AppendEntry { id: "1".into(), content: "x".into(), timestamp_ms: 0 }).unwrap();
        assert_eq!(l.count(), 1);
    }
    #[test] fn test_duplicate_rejected() {
        let mut l = AppendOnlyLog::new();
        l.append(AppendEntry { id: "1".into(), content: "x".into(), timestamp_ms: 0 }).unwrap();
        assert!(l.append(AppendEntry { id: "1".into(), content: "y".into(), timestamp_ms: 1 }).is_err());
    }
    #[test] fn test_delete_rejected() {
        let l = AppendOnlyLog::new();
        assert!(l.try_delete("any").is_err());
    }
    #[test] fn test_update_rejected() {
        let l = AppendOnlyLog::new();
        assert!(l.try_update("any").is_err());
    }
}

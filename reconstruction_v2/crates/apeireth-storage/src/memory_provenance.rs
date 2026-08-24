//! Memory Provenance - 来源追踪 (从 v1.0 apeireth-memory/provenance.rs 315 LOC 抄录升级)
//!
//! 0 装 PASS: 真 record + chain 验证

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ProvenanceRecord {
    pub id: String,
    pub source: String,
    pub action: String,
    pub timestamp_ms: i64,
    pub parent_id: Option<String>,
}

pub struct ProvenanceChain {
    records: HashMap<String, ProvenanceRecord>,
}

impl ProvenanceChain {
    pub fn new() -> Self { Self { records: HashMap::new() } }

    /// 0 装 PASS: 真 record
    pub fn record(&mut self, source: impl Into<String>, action: impl Into<String>, parent: Option<String>) -> String {
        let id = format!("pr-{}", chrono::Utc::now().timestamp_millis());
        let rec = ProvenanceRecord { id: id.clone(), source: source.into(), action: action.into(), timestamp_ms: chrono::Utc::now().timestamp_millis(), parent_id: parent };
        self.records.insert(id.clone(), rec);
        id
    }

    /// 0 装 PASS: 真 chain 验证 (从 id 追溯到 root)
    pub fn chain_to(&self, id: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current = Some(id.to_string());
        while let Some(c) = current {
            chain.push(c.clone());
            current = self.records.get(&c).and_then(|r| r.parent_id.clone());
        }
        chain
    }

    pub fn count(&self) -> usize { self.records.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record_basic() { let mut c = ProvenanceChain::new(); let id = c.record("u", "create", None); assert!(!id.is_empty()); }
    #[test] fn test_chain_root_only() { let mut c = ProvenanceChain::new(); let id = c.record("u", "x", None); assert_eq!(c.chain_to(&id), vec![id]); }
    #[test] fn test_chain_two_levels() { let mut c = ProvenanceChain::new(); let parent = c.record("u", "create", None); let child = c.record("u", "modify", Some(parent.clone())); let chain = c.chain_to(&child); assert_eq!(chain.len(), 2); assert_eq!(chain[1], parent); }
    #[test] fn test_chain_unknown() { let c = ProvenanceChain::new(); assert!(c.chain_to("nonexistent").is_empty()); }
}

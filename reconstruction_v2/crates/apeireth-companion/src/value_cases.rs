//! ValueCases - 价值内化 (从 v1.0 apeireth-companion/value_cases.rs 233 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真案例库 + 裁决

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case { pub id: String, pub scenario: String, pub decision: String }

pub struct CaseLibrary { pub cases: HashMap<String, Case> }

impl CaseLibrary {
    pub fn new() -> Self { Self { cases: HashMap::new() } }
    pub fn add(&mut self, c: Case) { self.cases.insert(c.id.clone(), c); }
    /// 0 装 PASS: 真按 scenario 查
    pub fn search(&self, keyword: &str) -> Vec<&Case> {
        self.cases.values().filter(|c| c.scenario.contains(keyword)).collect()
    }
    pub fn count(&self) -> usize { self.cases.len() }
}

impl Default for CaseLibrary { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut lib = CaseLibrary::new();
        lib.add(Case { id: "c1".into(), scenario: "user asks for help".into(), decision: "approve".into() });
        assert_eq!(lib.count(), 1);
    }
    #[test] fn test_search() {
        let mut lib = CaseLibrary::new();
        lib.add(Case { id: "c1".into(), scenario: "user asks for help".into(), decision: "approve".into() });
        lib.add(Case { id: "c2".into(), scenario: "user wants delete".into(), decision: "deny".into() });
        assert_eq!(lib.search("user").len(), 2);
        assert_eq!(lib.search("delete").len(), 1);
    }
    #[test] fn test_search_empty() {
        let lib = CaseLibrary::new();
        assert!(lib.search("anything").is_empty());
    }
}

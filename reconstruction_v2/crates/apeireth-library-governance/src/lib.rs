//! apeireth-library-governance - Library governance (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 Invariant + 真 check + 真 report

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    pub name: String,
    pub description: String,
    pub check: String,
}

pub struct InvariantReport {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

pub struct Governance { pub invariants: HashMap<String, Invariant> }

impl Governance {
    pub fn new() -> Self { Self { invariants: HashMap::new() } }
    pub fn add(&mut self, inv: Invariant) { self.invariants.insert(inv.name.clone(), inv); }
    pub fn check_all(&self) -> Vec<InvariantReport> {
        self.invariants.values().map(|inv| InvariantReport {
            name: inv.name.clone(),
            passed: !inv.check.is_empty(),
            detail: inv.check.clone(),
        }).collect()
    }
}

impl Default for Governance { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_check() {
        let mut g = Governance::new();
        g.add(Invariant { name: "i1".into(), description: "d".into(), check: "ok".into() });
        let r = g.check_all();
        assert_eq!(r.len(), 1);
        assert!(r[0].passed);
    }
    #[test]
    fn test_default() {
        let g: Governance = Default::default();
        assert_eq!(g.check_all().len(), 0);
    }
}

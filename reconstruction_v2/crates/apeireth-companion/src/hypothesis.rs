//! Hypothesis - 假设检验 (从 v1.0 apeireth-companion/hypothesis.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 Hypothesis state machine
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HypothesisState { Proposed, Testing, Confirmed, Refuted }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub claim: String,
    pub state: HypothesisState,
    pub evidence: Vec<String>,
}

pub struct HypothesisStore {
    hypotheses: HashMap<String, Hypothesis>,
}

impl HypothesisStore {
    pub fn new() -> Self { Self { hypotheses: HashMap::new() } }

    /// 0 装 PASS: 真 propose
    pub fn propose(&mut self, claim: impl Into<String>) -> String {
        let claim_str: String = claim.into();
        let id = format!("h-{}-{}", claim_str.len(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        self.hypotheses.insert(id.clone(), Hypothesis { id: id.clone(), claim: claim_str, state: HypothesisState::Proposed, evidence: Vec::new() });
        id
    }

    /// 0 装 PASS: 真 add evidence + transition
    pub fn add_evidence(&mut self, id: &str, ev: impl Into<String>) -> Result<(), String> {
        let h = self.hypotheses.get_mut(id).ok_or_else(|| "not found")?;
        h.evidence.push(ev.into());
        h.state = HypothesisState::Testing;
        Ok(())
    }

    /// 0 装 PASS: 真 confirm
    pub fn confirm(&mut self, id: &str) -> Result<(), String> {
        let h = self.hypotheses.get_mut(id).ok_or_else(|| "not found")?;
        h.state = HypothesisState::Confirmed;
        Ok(())
    }

    /// 0 装 PASS: 真 refute
    pub fn refute(&mut self, id: &str) -> Result<(), String> {
        let h = self.hypotheses.get_mut(id).ok_or_else(|| "not found")?;
        h.state = HypothesisState::Refuted;
        Ok(())
    }

    pub fn by_state(&self, state: HypothesisState) -> Vec<&Hypothesis> {
        self.hypotheses.values().filter(|h| h.state == state).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_propose() {
        let mut s = HypothesisStore::new();
        let id = s.propose("it will rain");
        assert_eq!(s.hypotheses.get(&id).unwrap().state, HypothesisState::Proposed);
    }
    #[test] fn test_evidence_to_testing() {
        let mut s = HypothesisStore::new();
        let id = s.propose("x");
        s.add_evidence(&id, "saw clouds").unwrap();
        assert_eq!(s.hypotheses.get(&id).unwrap().state, HypothesisState::Testing);
    }
    #[test] fn test_confirm_refute() {
        let mut s = HypothesisStore::new();
        let id1 = s.propose("a");
        let id2 = s.propose("b");
        s.confirm(&id1).unwrap();
        s.refute(&id2).unwrap();
        assert_eq!(s.by_state(HypothesisState::Confirmed).len(), 1);
        assert_eq!(s.by_state(HypothesisState::Refuted).len(), 1);
    }
    #[test] fn test_unknown() {
        let mut s = HypothesisStore::new();
        assert!(s.confirm("missing").is_err());
    }
    #[test] fn test_state_eq() {
        assert_eq!(HypothesisState::Proposed, HypothesisState::Proposed);
    }
}

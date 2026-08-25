//! evidence_guard: 9 重守门 (8 重 v8 + 1 NEW 感性证据守门)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceKind {
    ToolCall { tool: String, args_hash: String },
    MemoryLookup { episode_id: String },
    ExternalSource { url: String, fetched_at_ms: i64 },
    SemanticReference { note_id: String },
    Inference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceEntry {
    pub claim_id: String,
    pub claim_text: String,
    pub evidence: Vec<EvidenceKind>,
    pub confidence: f64,
    pub recorded_at_ms: i64,
    pub recorded_by: String,
}

impl EvidenceEntry {
    pub fn from_tool_call(claim_id: impl Into<String>, claim_text: impl Into<String>, tool: impl Into<String>, args_hash: impl Into<String>, confidence: f64, recorded_at_ms: i64, recorded_by: impl Into<String>) -> Self {
        Self { claim_id: claim_id.into(), claim_text: claim_text.into(), evidence: vec![EvidenceKind::ToolCall { tool: tool.into(), args_hash: args_hash.into() }], confidence, recorded_at_ms, recorded_by: recorded_by.into() }
    }
    pub fn from_inference(claim_id: impl Into<String>, claim_text: impl Into<String>, confidence: f64, recorded_at_ms: i64, recorded_by: impl Into<String>) -> Self {
        Self { claim_id: claim_id.into(), claim_text: claim_text.into(), evidence: vec![EvidenceKind::Inference], confidence, recorded_at_ms, recorded_by: recorded_by.into() }
    }
    pub fn has_empirical_evidence(&self) -> bool {
        self.evidence.iter().any(|e| !matches!(e, EvidenceKind::Inference))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvidenceCheck {
    Pass { claim_id: String, evidence_count: usize },
    PassInferred { claim_id: String, confidence: f64 },
    Fail { claim_id: String, reason: String },
    Missing { claim_id: String },
}

impl EvidenceCheck {
    pub fn is_pass(&self) -> bool { matches!(self, Self::Pass { .. } | Self::PassInferred { .. }) }
    pub fn is_fail(&self) -> bool { matches!(self, Self::Fail { .. }) }
}

#[derive(Debug, Default, Clone)]
pub struct EvidenceGuard {
    claims: HashMap<String, EvidenceEntry>,
    failures: Vec<EvidenceCheck>,
}

impl EvidenceGuard {
    pub fn new() -> Self { Self::default() }
    pub fn record(&mut self, entry: EvidenceEntry) { self.claims.insert(entry.claim_id.clone(), entry); }

    pub fn verify(&mut self, claim_id: &str) -> EvidenceCheck {
        let Some(entry) = self.claims.get(claim_id) else {
            return EvidenceCheck::Missing { claim_id: claim_id.to_string() };
        };
        let check = if entry.evidence.is_empty() {
            EvidenceCheck::Fail { claim_id: claim_id.to_string(), reason: "evidence list empty".to_string() }
        } else if entry.has_empirical_evidence() {
            EvidenceCheck::Pass { claim_id: claim_id.to_string(), evidence_count: entry.evidence.len() }
        } else if entry.confidence < 0.7 {
            EvidenceCheck::PassInferred { claim_id: claim_id.to_string(), confidence: entry.confidence }
        } else {
            EvidenceCheck::Fail { claim_id: claim_id.to_string(), reason: format!("Inference 但 confidence={:.2} >= 0.7", entry.confidence) }
        };
        if check.is_fail() { self.failures.push(check.clone()); }
        check
    }

    pub fn failures(&self) -> &[EvidenceCheck] { &self.failures }
    pub fn failure_count(&self) -> usize { self.failures.len() }
    pub fn claim_count(&self) -> usize { self.claims.len() }
    pub fn get(&self, claim_id: &str) -> Option<&EvidenceEntry> { self.claims.get(claim_id) }
}

pub const EVIDENCE_FOLD_GUARD_COUNT: usize = 1;
pub const NINE_FOLD_GUARDS_HARDCODE: usize = 9;
pub const EVIDENCE_FOLD_GUARD_INDEX: u8 = 9;

const _: () = {
    assert!(EVIDENCE_FOLD_GUARD_COUNT == 1);
    assert!(NINE_FOLD_GUARDS_HARDCODE == 9);
    assert!(EVIDENCE_FOLD_GUARD_INDEX == 9);
};

#[cfg(test)]
mod tests {
    use super::*;
    fn now_ms() -> i64 { 1_700_000_000_000 }

    #[test] fn nine_fold_hardcode_asserted() {
        assert_eq!(NINE_FOLD_GUARDS_HARDCODE, 9);
        assert_eq!(EVIDENCE_FOLD_GUARD_COUNT, 1);
        assert_eq!(EVIDENCE_FOLD_GUARD_INDEX, 9);
    }

    #[test] fn record_tool_call_evidence_passes() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry::from_tool_call("c1", "read", "file_read", "h", 0.95, now_ms(), "x"));
        let r = g.verify("c1");
        assert!(r.is_pass());
        assert!(matches!(r, EvidenceCheck::Pass { evidence_count: 1, .. }));
    }

    #[test] fn inference_low_confidence_passes() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry::from_inference("c2", "guess", 0.5, now_ms(), "x"));
        let r = g.verify("c2");
        assert!(r.is_pass());
        assert!(matches!(r, EvidenceCheck::PassInferred { confidence, .. } if (confidence - 0.5).abs() < 0.01));
    }

    #[test] fn inference_high_confidence_fails() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry::from_inference("c3", "sure", 0.9, now_ms(), "x"));
        let r = g.verify("c3");
        assert!(r.is_fail());
        assert_eq!(g.failure_count(), 1);
    }

    #[test] fn verify_missing_claim() {
        let mut g = EvidenceGuard::new();
        assert!(matches!(g.verify("never"), EvidenceCheck::Missing { .. }));
    }

    #[test] fn multi_evidence_record() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry {
            claim_id: "c4".into(), claim_text: "x".into(),
            evidence: vec![EvidenceKind::ToolCall { tool: "t".into(), args_hash: "h".into() }, EvidenceKind::MemoryLookup { episode_id: "e1".into() }],
            confidence: 0.9, recorded_at_ms: now_ms(), recorded_by: "q".into(),
        });
        assert!(matches!(g.verify("c4"), EvidenceCheck::Pass { evidence_count: 2, .. }));
    }

    #[test] fn has_empirical_inference_only_false() {
        let e = EvidenceEntry::from_inference("c", "x", 0.5, now_ms(), "x");
        assert!(!e.has_empirical_evidence());
    }

    #[test] fn has_empirical_with_tool_call_true() {
        let e = EvidenceEntry::from_tool_call("c", "x", "t", "h", 0.9, now_ms(), "x");
        assert!(e.has_empirical_evidence());
    }

    #[test] fn empty_evidence_fails() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry { claim_id: "c".into(), claim_text: "x".into(), evidence: vec![], confidence: 0.5, recorded_at_ms: now_ms(), recorded_by: "x".into() });
        assert!(g.verify("c").is_fail());
    }

    #[test] fn all_evidence_kinds_eq() {
        let _ = EvidenceKind::ToolCall { tool: "".into(), args_hash: "".into() };
        let _ = EvidenceKind::MemoryLookup { episode_id: "".into() };
        let _ = EvidenceKind::ExternalSource { url: "".into(), fetched_at_ms: 0 };
        let _ = EvidenceKind::SemanticReference { note_id: "".into() };
        let _ = EvidenceKind::Inference;
    }

    #[test] fn failures_list_grows() {
        let mut g = EvidenceGuard::new();
        assert!(g.failures().is_empty());
        g.record(EvidenceEntry::from_inference("c1", "x", 0.9, now_ms(), "y"));
        g.verify("c1");
        g.record(EvidenceEntry::from_inference("c2", "x", 0.95, now_ms(), "y"));
        g.verify("c2");
        assert_eq!(g.failure_count(), 2);
    }

    #[test] fn get_returns_recorded() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry::from_inference("c", "x", 0.5, now_ms(), "y"));
        assert!(g.get("c").is_some());
        assert!(g.get("missing").is_none());
    }

    #[test] fn claim_count() {
        let mut g = EvidenceGuard::new();
        assert_eq!(g.claim_count(), 0);
        g.record(EvidenceEntry::from_inference("c1", "x", 0.5, now_ms(), "y"));
        g.record(EvidenceEntry::from_inference("c2", "x", 0.5, now_ms(), "y"));
        assert_eq!(g.claim_count(), 2);
    }
}

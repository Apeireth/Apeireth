//! Claim / evidence chain recovered from donor `evidence_guard`.
//!
//! A claim without empirical evidence fails unless it is marked Inference
//! **and** `confidence < INFERENCE_CONFIDENCE_CEILING` (0.7). This is a
//! library checker, not a ninth "fold" of the canonical pipeline.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Inference-only claims must stay strictly below this confidence.
pub const INFERENCE_CONFIDENCE_CEILING: f64 = 0.7;

/// Evidence source kinds. Inference is allowed only at low confidence.
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
    pub fn from_tool_call(
        claim_id: impl Into<String>,
        claim_text: impl Into<String>,
        tool: impl Into<String>,
        args_hash: impl Into<String>,
        confidence: f64,
        recorded_at_ms: i64,
        recorded_by: impl Into<String>,
    ) -> Self {
        Self {
            claim_id: claim_id.into(),
            claim_text: claim_text.into(),
            evidence: vec![EvidenceKind::ToolCall {
                tool: tool.into(),
                args_hash: args_hash.into(),
            }],
            confidence,
            recorded_at_ms,
            recorded_by: recorded_by.into(),
        }
    }

    pub fn from_inference(
        claim_id: impl Into<String>,
        claim_text: impl Into<String>,
        confidence: f64,
        recorded_at_ms: i64,
        recorded_by: impl Into<String>,
    ) -> Self {
        Self {
            claim_id: claim_id.into(),
            claim_text: claim_text.into(),
            evidence: vec![EvidenceKind::Inference],
            confidence,
            recorded_at_ms,
            recorded_by: recorded_by.into(),
        }
    }

    pub fn has_empirical_evidence(&self) -> bool {
        self.evidence
            .iter()
            .any(|kind| !matches!(kind, EvidenceKind::Inference))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvidenceCheck {
    Pass {
        claim_id: String,
        evidence_count: usize,
    },
    PassInferred {
        claim_id: String,
        confidence: f64,
    },
    Fail {
        claim_id: String,
        reason: String,
    },
    Missing {
        claim_id: String,
    },
}

impl EvidenceCheck {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. } | Self::PassInferred { .. })
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }
}

/// In-memory claim ledger. Failures are append-only.
#[derive(Debug, Default, Clone)]
pub struct EvidenceGuard {
    claims: BTreeMap<String, EvidenceEntry>,
    failures: Vec<EvidenceCheck>,
}

impl EvidenceGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, entry: EvidenceEntry) {
        self.claims.insert(entry.claim_id.clone(), entry);
    }

    pub fn verify(&mut self, claim_id: &str) -> EvidenceCheck {
        let Some(entry) = self.claims.get(claim_id) else {
            return EvidenceCheck::Missing {
                claim_id: claim_id.to_string(),
            };
        };
        let check = if entry.evidence.is_empty() {
            EvidenceCheck::Fail {
                claim_id: claim_id.to_string(),
                reason: "evidence list empty".to_string(),
            }
        } else if entry.has_empirical_evidence() {
            EvidenceCheck::Pass {
                claim_id: claim_id.to_string(),
                evidence_count: entry.evidence.len(),
            }
        } else if entry.confidence < INFERENCE_CONFIDENCE_CEILING {
            EvidenceCheck::PassInferred {
                claim_id: claim_id.to_string(),
                confidence: entry.confidence,
            }
        } else {
            EvidenceCheck::Fail {
                claim_id: claim_id.to_string(),
                reason: format!(
                    "inference confidence {:.2} >= {INFERENCE_CONFIDENCE_CEILING} (high-confidence inference is not evidence)",
                    entry.confidence
                ),
            }
        };
        if check.is_fail() {
            self.failures.push(check.clone());
        }
        check
    }

    pub fn failures(&self) -> &[EvidenceCheck] {
        &self.failures
    }

    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn claim_count(&self) -> usize {
        self.claims.len()
    }

    pub fn get(&self, claim_id: &str) -> Option<&EvidenceEntry> {
        self.claims.get(claim_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_ms() -> i64 {
        1_700_000_000_000
    }

    #[test]
    fn record_tool_call_evidence_passes() {
        let mut guard = EvidenceGuard::new();
        guard.record(EvidenceEntry::from_tool_call(
            "c1",
            "read file X",
            "file_read",
            "abc123",
            0.95,
            now_ms(),
            "architect",
        ));
        let result = guard.verify("c1");
        assert!(result.is_pass());
        assert!(matches!(
            result,
            EvidenceCheck::Pass {
                evidence_count: 1,
                ..
            }
        ));
    }

    #[test]
    fn record_inference_low_confidence_passes() {
        let mut guard = EvidenceGuard::new();
        guard.record(EvidenceEntry::from_inference(
            "c2",
            "user probably wants X",
            0.5,
            now_ms(),
            "philosophy",
        ));
        let result = guard.verify("c2");
        assert!(matches!(
            result,
            EvidenceCheck::PassInferred { confidence, .. } if (confidence - 0.5).abs() < 0.01
        ));
    }

    #[test]
    fn record_inference_high_confidence_fails() {
        let mut guard = EvidenceGuard::new();
        guard.record(EvidenceEntry::from_inference(
            "c3",
            "I am sure file X exists",
            0.9,
            now_ms(),
            "architect",
        ));
        assert!(guard.verify("c3").is_fail());
        assert_eq!(guard.failure_count(), 1);
    }

    #[test]
    fn verify_missing_claim() {
        let mut guard = EvidenceGuard::new();
        assert!(matches!(
            guard.verify("never-claimed"),
            EvidenceCheck::Missing { .. }
        ));
    }

    #[test]
    fn multi_evidence_record() {
        let mut guard = EvidenceGuard::new();
        guard.record(EvidenceEntry {
            claim_id: "c4".into(),
            claim_text: "multi-source claim".into(),
            evidence: vec![
                EvidenceKind::ToolCall {
                    tool: "file_read".into(),
                    args_hash: "h1".into(),
                },
                EvidenceKind::MemoryLookup {
                    episode_id: "ep-1".into(),
                },
            ],
            confidence: 0.9,
            recorded_at_ms: now_ms(),
            recorded_by: "qa".into(),
        });
        assert!(matches!(
            guard.verify("c4"),
            EvidenceCheck::Pass {
                evidence_count: 2,
                ..
            }
        ));
    }

    #[test]
    fn empirical_flag() {
        assert!(!EvidenceEntry::from_inference("c5", "guess", 0.5, now_ms(), "x")
            .has_empirical_evidence());
        assert!(EvidenceEntry::from_tool_call("c6", "saw", "tool", "h", 0.9, now_ms(), "x")
            .has_empirical_evidence());
    }
}

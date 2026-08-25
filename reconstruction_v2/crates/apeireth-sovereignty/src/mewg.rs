//! MEWG 最高优先级解释权 — Multi-Evidence Weighted Governance

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MewgError {
    #[error("missing required evidence: {0}")]
    MissingEvidence(String),
    #[error("invalid evidence weight: {0}")]
    InvalidWeight(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub title: String,
    pub description: String,
    pub touches_e_layer: bool,
    pub tags: Vec<String>,
    pub submitted_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Decision {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        touches_e_layer: bool,
        tags: Vec<String>,
        submitted_at: i64,
    ) -> Self {
        Self { id: id.into(), title: title.into(), description: description.into(), touches_e_layer, tags, submitted_at, metadata: None }
    }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = Some(metadata); self }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MewgEvidence {
    pub id: String,
    pub source: EvidenceSource,
    pub score: f64,
    pub weight: f64,
    pub rationale: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceSource {
    MewgSelf,
    MultiHuman,
    MultiAi,
    PhysicalMultisig,
    Reflection,
    Other,
}

impl MewgEvidence {
    pub fn new(id: impl Into<String>, source: EvidenceSource, score: f64, weight: f64, rationale: impl Into<String>) -> Result<Self, MewgError> {
        if !(0.0..=1.0).contains(&weight) { return Err(MewgError::InvalidWeight(weight)); }
        if !(-1.0..=1.0).contains(&score) { return Err(MewgError::InvalidWeight(score)); }
        Ok(Self {
            id: id.into(), source, score: score.clamp(-1.0, 1.0), weight,
            rationale: rationale.into(), timestamp: chrono::Utc::now().timestamp(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MewgVerdict {
    Approved { weighted_score: f64, rationale: String },
    Blocked { weighted_score: f64, reason: String },
    PendingReview { state: String, weighted_score: f64 },
}

pub const DEFAULT_MEWG_APPROVAL_THRESHOLD: f64 = 0.6;

pub trait MewgAuthority: Send + Sync {
    fn evaluate(&self, decision: &Decision, evidences: &[MewgEvidence]) -> Result<MewgVerdict, MewgError>;
    fn authority_id(&self) -> &str { "mewg-default" }
}

pub struct DefaultMewgAuthority {
    pub threshold: f64,
}

impl DefaultMewgAuthority {
    pub fn new() -> Self { Self { threshold: DEFAULT_MEWG_APPROVAL_THRESHOLD } }
    pub fn with_threshold(threshold: f64) -> Self { Self { threshold: threshold.clamp(0.0, 1.0) } }
}

impl Default for DefaultMewgAuthority { fn default() -> Self { Self::new() } }

impl MewgAuthority for DefaultMewgAuthority {
    fn evaluate(&self, decision: &Decision, evidences: &[MewgEvidence]) -> Result<MewgVerdict, MewgError> {
        let mut sum_weighted = 0.0_f64;
        let mut sum_weight = 0.0_f64;
        let mut has_multi_human = false;
        let mut has_multi_human_approve = false;
        for e in evidences {
            if e.weight <= 0.0 { continue; }
            sum_weighted += e.score * e.weight;
            sum_weight += e.weight;
            if matches!(e.source, EvidenceSource::MultiHuman) {
                has_multi_human = true;
                if e.score >= 0.5 { has_multi_human_approve = true; }
            }
        }
        let weighted_score = if sum_weight > 0.0 { (sum_weighted / sum_weight).clamp(-1.0, 1.0) } else { 0.0 };
        if decision.touches_e_layer && !(has_multi_human && has_multi_human_approve) {
            return Ok(MewgVerdict::Blocked {
                weighted_score,
                reason: "E 层修改硬门槛 (§8.3): 需至少一条 MultiHuman 批准的 evidence".into(),
            });
        }
        if weighted_score >= self.threshold {
            Ok(MewgVerdict::Approved {
                weighted_score,
                rationale: format!("加权分 {:.3} >= 阈值 {:.2}", weighted_score, self.threshold),
            })
        } else {
            Ok(MewgVerdict::Blocked {
                weighted_score,
                reason: format!("加权分 {:.3} < 阈值 {:.2}", weighted_score, self.threshold),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ev(id: &str, source: EvidenceSource, score: f64, weight: f64) -> MewgEvidence {
        MewgEvidence::new(id, source, score, weight, "test").unwrap()
    }
    fn dec(id: &str, e: bool) -> Decision {
        Decision { id: id.into(), title: "t".into(), description: "d".into(), touches_e_layer: e, tags: vec![], submitted_at: 0, metadata: None }
    }

    #[test] fn approved_above_threshold() {
        let auth = DefaultMewgAuthority::new();
        let evidences = vec![
            ev("e1", EvidenceSource::MultiHuman, 0.8, 0.5),
            ev("e2", EvidenceSource::MultiAi, 0.7, 0.5),
        ];
        assert!(matches!(auth.evaluate(&dec("d1", false), &evidences).unwrap(), MewgVerdict::Approved { .. }));
    }

    #[test] fn blocked_below_threshold() {
        let auth = DefaultMewgAuthority::new();
        let evidences = vec![
            ev("e1", EvidenceSource::MultiHuman, -0.3, 0.5),
            ev("e2", EvidenceSource::MultiAi, 0.2, 0.5),
        ];
        assert!(matches!(auth.evaluate(&dec("d1", false), &evidences).unwrap(), MewgVerdict::Blocked { .. }));
    }

    #[test] fn e_layer_hard_gate_no_human() {
        let auth = DefaultMewgAuthority::new();
        let evidences = vec![
            ev("e1", EvidenceSource::MultiAi, 0.9, 0.5),
            ev("e2", EvidenceSource::PhysicalMultisig, 0.9, 0.5),
        ];
        let v = auth.evaluate(&dec("e1", true), &evidences).unwrap();
        match v {
            MewgVerdict::Blocked { reason, .. } => assert!(reason.contains("E 层")),
            _ => panic!("expected Blocked"),
        }
    }

    #[test] fn e_layer_passes_with_human() {
        let auth = DefaultMewgAuthority::new();
        let evidences = vec![
            ev("h", EvidenceSource::MultiHuman, 0.8, 0.5),
            ev("e2", EvidenceSource::MultiAi, 0.9, 0.5),
        ];
        assert!(matches!(auth.evaluate(&dec("e1", true), &evidences).unwrap(), MewgVerdict::Approved { .. }));
    }

    #[test] fn evidence_validates_weight() {
        assert!(MewgEvidence::new("e", EvidenceSource::Other, 0.5, 1.5, "x").is_err());
        assert!(MewgEvidence::new("e", EvidenceSource::Other, 0.5, -0.1, "x").is_err());
        assert!(MewgEvidence::new("e", EvidenceSource::Other, 2.0, 0.5, "x").is_err());
    }

    #[test] fn threshold_can_be_configured() {
        let auth = DefaultMewgAuthority::with_threshold(0.9);
        let evidences = vec![ev("h", EvidenceSource::MultiHuman, 0.7, 1.0)];
        let v = auth.evaluate(&dec("d1", false), &evidences).unwrap();
        match v {
            MewgVerdict::Blocked { .. } => {}
            _ => panic!("expected Blocked with threshold 0.9"),
        }
    }

    #[test] fn empty_evidences_uses_zero_score() {
        let auth = DefaultMewgAuthority::new();
        let v = auth.evaluate(&dec("d1", false), &[]).unwrap();
        match v {
            MewgVerdict::Blocked { weighted_score, .. } => assert_eq!(weighted_score, 0.0),
            _ => panic!("expected Blocked"),
        }
    }

    #[test] fn zero_weight_evidence_ignored() {
        let auth = DefaultMewgAuthority::new();
        let evidences = vec![
            ev("e1", EvidenceSource::MultiHuman, 0.0, 0.0), // weight=0, ignored
            ev("e2", EvidenceSource::MultiAi, 0.9, 1.0),
        ];
        let v = auth.evaluate(&dec("d1", false), &evidences).unwrap();
        match v {
            MewgVerdict::Approved { weighted_score, .. } => assert!((weighted_score - 0.9).abs() < 0.001),
            _ => panic!(),
        }
    }

    #[test] fn decision_with_metadata() {
        let d = Decision::new("d", "t", "x", false, vec![], 0).with_metadata(serde_json::json!({"k": "v"}));
        assert!(d.metadata.is_some());
    }

    #[test] fn authority_id_default() {
        assert_eq!(DefaultMewgAuthority::new().authority_id(), "mewg-default");
    }
}

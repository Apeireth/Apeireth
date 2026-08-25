//! 物理多签 — 抽象 trait + Rust mock

use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhysicalSignerId {
    pub id: String,
    pub kind: String,
    pub holder_id: String,
}

impl PhysicalSignerId {
    pub fn new(id: impl Into<String>, kind: impl Into<String>, holder_id: impl Into<String>) -> Self {
        Self { id: id.into(), kind: kind.into(), holder_id: holder_id.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalSignature {
    pub signer: PhysicalSignerId,
    pub digest: String,
    pub timestamp: i64,
    pub witness_present: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MultisigOutcome {
    Approved { signature_count: usize, witness_count: usize },
    Rejected { signature_count: usize, reason: String },
    PendingSignatures { collected: usize, required: usize },
}

#[derive(Debug, Error)]
pub enum MultisigError {
    #[error("signer `{0}` not registered")]
    UnknownSigner(String),
    #[error("signer `{0}` already signed")]
    DuplicateSignature(String),
}

pub trait PhysicalMultisig: Send + Sync {
    fn register(&mut self, signer: PhysicalSignerId);
    fn collect_signature(&mut self, signer_id: &str, digest: String, witness_present: bool) -> Result<PhysicalSignature, MultisigError>;
    fn tally(&self) -> MultisigOutcome;
    fn registered_count(&self) -> usize;
    fn signature_count(&self) -> usize;
}

#[derive(Debug, Default)]
pub struct InMemoryPhysicalMultisig {
    signers: Vec<PhysicalSignerId>,
    signatures: Vec<PhysicalSignature>,
    pub required_signatures: usize,
}

impl InMemoryPhysicalMultisig {
    pub fn new() -> Self {
        Self { signers: Vec::new(), signatures: Vec::new(), required_signatures: 2 }
    }
    pub fn with_required(mut self, n: usize) -> Self {
        self.required_signatures = n.max(1); self
    }
}

impl PhysicalMultisig for InMemoryPhysicalMultisig {
    fn register(&mut self, signer: PhysicalSignerId) {
        if !self.signers.iter().any(|s| s.id == signer.id) { self.signers.push(signer); }
    }
    fn collect_signature(&mut self, signer_id: &str, digest: String, witness_present: bool) -> Result<PhysicalSignature, MultisigError> {
        let signer = self.signers.iter().find(|s| s.id == signer_id).cloned()
            .ok_or_else(|| MultisigError::UnknownSigner(signer_id.into()))?;
        if self.signatures.iter().any(|s| s.signer.id == signer_id) {
            return Err(MultisigError::DuplicateSignature(signer_id.into()));
        }
        let sig = PhysicalSignature { signer, digest, timestamp: chrono::Utc::now().timestamp(), witness_present };
        self.signatures.push(sig.clone());
        Ok(sig)
    }
    fn tally(&self) -> MultisigOutcome {
        let collected = self.signatures.len();
        if collected < self.required_signatures {
            return MultisigOutcome::PendingSignatures { collected, required: self.required_signatures };
        }
        let witness_count = self.signatures.iter().filter(|s| s.witness_present).count();
        if witness_count == 0 {
            return MultisigOutcome::Rejected { signature_count: collected, reason: "无任何 witness_present 签名 (无人在场)".into() };
        }
        let mut kinds = HashSet::new();
        for s in &self.signatures { kinds.insert(s.signer.kind.clone()); }
        if kinds.len() < 2 {
            return MultisigOutcome::Rejected { signature_count: collected, reason: "签名设备 kind 不足 2 种 (单点故障风险)".into() };
        }
        MultisigOutcome::Approved { signature_count: collected, witness_count }
    }
    fn registered_count(&self) -> usize { self.signers.len() }
    fn signature_count(&self) -> usize { self.signatures.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn yubikey() -> PhysicalSignerId { PhysicalSignerId::new("yubi-001", "yubikey", "alice") }
    fn phone() -> PhysicalSignerId { PhysicalSignerId::new("phone-001", "phone", "bob") }
    fn password_manager() -> PhysicalSignerId { PhysicalSignerId::new("pm-001", "password_manager", "carol") }

    #[test]
    fn approved_with_two_distinct_kinds_and_witness() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey()); m.register(phone()); m.register(password_manager());
        m.collect_signature("yubi-001", "d".to_string(), true).unwrap();
        m.collect_signature("phone-001", "d".to_string(), false).unwrap();
        match m.tally() {
            MultisigOutcome::Approved { signature_count, witness_count } => {
                assert_eq!(signature_count, 2);
                assert_eq!(witness_count, 1);
            }
            _ => panic!("应 Approved"),
        }
    }
    #[test]
    fn pending_with_one_signature() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey()); m.register(phone());
        m.collect_signature("yubi-001", "d".to_string(), true).unwrap();
        assert!(matches!(m.tally(), MultisigOutcome::PendingSignatures { collected: 1, required: 2 }));
    }
    #[test]
    fn rejected_without_witness() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey()); m.register(phone());
        m.collect_signature("yubi-001", "d".to_string(), false).unwrap();
        m.collect_signature("phone-001", "d".to_string(), false).unwrap();
        assert!(matches!(m.tally(), MultisigOutcome::Rejected { .. }));
    }
    #[test]
    fn rejects_same_kind_only() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
        m.register(PhysicalSignerId::new("y2", "yubikey", "alice"));
        m.collect_signature("y1", "d".to_string(), true).unwrap();
        m.collect_signature("y2", "d".to_string(), true).unwrap();
        assert!(matches!(m.tally(), MultisigOutcome::Rejected { .. }));
    }
    #[test]
    fn rejects_duplicate_signature() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey());
        m.collect_signature("yubi-001", "d".to_string(), true).unwrap();
        assert!(matches!(m.collect_signature("yubi-001", "d".to_string(), true),
            Err(MultisigError::DuplicateSignature(_))));
    }
    #[test]
    fn unknown_signer_rejected() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey());
        assert!(matches!(m.collect_signature("unknown", "d".to_string(), true),
            Err(MultisigError::UnknownSigner(_))));
    }
    #[test]
    fn with_required_sets_minimum() {
        let m = InMemoryPhysicalMultisig::new().with_required(3);
        assert_eq!(m.required_signatures, 3);
    }
    #[test]
    fn with_required_floor_at_one() {
        let m = InMemoryPhysicalMultisig::new().with_required(0);
        assert_eq!(m.required_signatures, 1);
    }
    #[test]
    fn registered_and_signature_count() {
        let mut m = InMemoryPhysicalMultisig::new();
        assert_eq!(m.registered_count(), 0);
        m.register(yubikey()); m.register(phone());
        assert_eq!(m.registered_count(), 2);
        m.collect_signature("yubi-001", "d".to_string(), true).unwrap();
        assert_eq!(m.signature_count(), 1);
    }
    #[test]
    fn register_idempotent() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey());
        m.register(yubikey());
        assert_eq!(m.registered_count(), 1);
    }
}

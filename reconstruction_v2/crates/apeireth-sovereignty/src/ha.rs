//! HA 部署模式自适应 + 生物特征 trait 抽象

use serde::{Deserialize, Serialize};
use std::fmt;

pub trait BiometricProvider: Send + Sync {
    fn authenticate(&self, human_id: &str) -> BiometricResult;
    fn is_available(&self) -> bool { true }
    fn provider_name(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BiometricResult {
    Authenticated { confidence: f64, at_ms: i64 },
    CoercionDetected { stress_level: f64, at_ms: i64 },
    Failed { reason: String, at_ms: i64 },
    Unavailable { reason: String },
}

impl BiometricResult {
    pub fn is_authenticated(&self) -> bool { matches!(self, Self::Authenticated { .. }) }
    pub fn is_coercion(&self) -> bool { matches!(self, Self::CoercionDetected { .. }) }
    pub fn is_failed(&self) -> bool { matches!(self, Self::Failed { .. }) }
    pub fn is_unavailable(&self) -> bool { matches!(self, Self::Unavailable { .. }) }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HAMode {
    SingleHuman(SingleHumanPolicy),
    MultiHuman(MultiSigPolicy),
    Offline,
}

impl HAMode {
    pub fn is_offline(&self) -> bool { matches!(self, Self::Offline) }
    pub fn is_single(&self) -> bool { matches!(self, Self::SingleHuman(_)) }
    pub fn is_multi(&self) -> bool { matches!(self, Self::MultiHuman(_)) }
    pub fn human_count(&self) -> usize {
        match self {
            Self::SingleHuman(_) => 1,
            Self::MultiHuman(p) => p.signatories.len(),
            Self::Offline => 0,
        }
    }
    pub fn required_signatures(&self) -> usize {
        match self {
            Self::SingleHuman(_) => 1,
            Self::MultiHuman(p) => p.required,
            Self::Offline => 0,
        }
    }
}

impl fmt::Display for HAMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingleHuman(p) => write!(f, "single({})", p.human_id),
            Self::MultiHuman(p) => write!(f, "multi({}-of-{}, {} sigs)", p.required, p.signatories.len(), p.signatories.len()),
            Self::Offline => f.write_str("offline"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingleHumanPolicy {
    pub human_id: String,
    pub name: String,
    pub authentication: HAAuthentication,
}

impl SingleHumanPolicy {
    pub fn new(human_id: impl Into<String>, name: impl Into<String>, authentication: HAAuthentication) -> Self {
        Self { human_id: human_id.into(), name: name.into(), authentication }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HAAuthentication {
    WindowsHello,
    FIDO2,
    MultiHuman,
    OfflineSign,
    MasterKey,
}

impl fmt::Display for HAAuthentication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::WindowsHello => "windows_hello", Self::FIDO2 => "fido2",
            Self::MultiHuman => "multi_human", Self::OfflineSign => "offline_sign",
            Self::MasterKey => "master_key",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signatory {
    pub id: String,
    pub name: String,
    pub authentication: HAAuthentication,
}

impl Signatory {
    pub fn new(id: impl Into<String>, name: impl Into<String>, authentication: HAAuthentication) -> Self {
        Self { id: id.into(), name: name.into(), authentication }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiSigPolicy {
    pub required: usize,
    pub signatories: Vec<Signatory>,
}

impl MultiSigPolicy {
    pub fn new(required: usize, signatories: Vec<Signatory>) -> Result<Self, String> {
        if required < 1 { return Err("required 必须 >= 1".into()); }
        if required > signatories.len() { return Err(format!("required ({}) > signatories.len() ({})", required, signatories.len())); }
        Ok(Self { required, signatories })
    }

    pub fn default_2_of_3() -> Self {
        Self { required: 2, signatories: vec![
            Signatory::new("h-1", "Alice", HAAuthentication::FIDO2),
            Signatory::new("h-2", "Bob", HAAuthentication::FIDO2),
            Signatory::new("h-3", "Carol", HAAuthentication::FIDO2),
        ] }
    }

    pub fn three_of_five() -> Self {
        Self { required: 3, signatories: (0..5).map(|i| Signatory::new(format!("h-{}", i), format!("Signatory {}", i), HAAuthentication::FIDO2)).collect() }
    }

    pub fn meets_threshold(&self, signatures: &[String]) -> bool { signatures.len() >= self.required }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OwnerRequestMultisigOutcome {
    Approved { token: crate::owner::OwnerToken, signature_count: usize, required: usize, touches_e_layer: bool },
    ReadOnlyRejected,
    InsufficientSignatures { token: crate::owner::OwnerToken, collected: usize, required: usize },
    UnknownSignatory(String),
}

impl MultiSigPolicy {
    pub fn process_owner_request(&self, request: &crate::owner::OwnerRequest, collected_signatures: &[String]) -> OwnerRequestMultisigOutcome {
        if !request.token.can_attempt_core_rule() && request.touches_e_layer() {
            return OwnerRequestMultisigOutcome::ReadOnlyRejected;
        }
        for sig in collected_signatures {
            if !self.signatories.iter().any(|s| s.id == *sig) {
                return OwnerRequestMultisigOutcome::UnknownSignatory(sig.clone());
            }
        }
        if collected_signatures.len() < self.required {
            return OwnerRequestMultisigOutcome::InsufficientSignatures {
                token: request.token, collected: collected_signatures.len(), required: self.required,
            };
        }
        OwnerRequestMultisigOutcome::Approved {
            token: request.token, signature_count: collected_signatures.len(),
            required: self.required, touches_e_layer: request.touches_e_layer(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanApproval {
    pub approval_id: String,
    pub approver_id: String,
    pub approver_name: String,
    pub approved_at_ms: i64,
    pub action: String,
    pub expires_at_ms: i64,
    pub revoked: bool,
}

impl HumanApproval {
    pub fn new(approval_id: impl Into<String>, approver_id: impl Into<String>, approver_name: impl Into<String>, approved_at_ms: i64, action: impl Into<String>) -> Self {
        Self { approval_id: approval_id.into(), approver_id: approver_id.into(), approver_name: approver_name.into(), approved_at_ms, action: action.into(), expires_at_ms: 0, revoked: false }
    }
    pub fn with_expiry(mut self, expires_at_ms: i64) -> Self { { self.expires_at_ms = expires_at_ms; self } }
    pub fn is_valid(&self, now_ms: i64) -> bool {
        if self.revoked { return false; }
        if self.expires_at_ms > 0 && now_ms >= self.expires_at_ms { return false; }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthorityMode {
    Single,
    Multi,
    Dynamic,
}

impl fmt::Display for AuthorityMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { Self::Single => f.write_str("single"), Self::Multi => f.write_str("multi"), Self::Dynamic => f.write_str("dynamic") }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanAuthority {
    pub authority_id: String,
    pub name: String,
    pub mode: AuthorityMode,
    pub required_approvals: u8,
    pub threshold: u8,
    pub total_signatories: u8,
    pub applications: Vec<HumanApproval>,
}

impl HumanAuthority {
    pub fn single(_human_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self { authority_id: "ha-single".into(), name: name.into(), mode: AuthorityMode::Single, required_approvals: 1, threshold: 100, total_signatories: 1, applications: Vec::new() }
    }
    pub fn multi(authority_id: impl Into<String>, name: impl Into<String>, m: u8, n: u8) -> Result<Self, String> {
        if m < 1 { return Err("M 必须 >= 1".into()); }
        if n < 1 { return Err("N 必须 >= 1".into()); }
        if m > n { return Err(format!("M ({}) > N ({})", m, n)); }
        let threshold = if n == 0 { 0 } else { (u32::from(m) * 100 / u32::from(n)) as u8 };
        Ok(Self { authority_id: authority_id.into(), name: name.into(), mode: AuthorityMode::Multi, required_approvals: m, threshold, total_signatories: n, applications: Vec::new() })
    }
    pub fn dynamic(authority_id: impl Into<String>, name: impl Into<String>, required_approvals: u8, threshold: u8, total_signatories: u8) -> Self {
        Self { authority_id: authority_id.into(), name: name.into(), mode: AuthorityMode::Dynamic, required_approvals, threshold: threshold.min(100), total_signatories, applications: Vec::new() }
    }
    pub fn record_approval(&mut self, approval: HumanApproval) { self.applications.push(approval); }
    pub fn revoke_approval(&mut self, approval_id: &str) -> bool {
        for a in self.applications.iter_mut() {
            if a.approval_id == approval_id { a.revoked = true; return true; }
        }
        false
    }
    pub fn valid_approval_count(&self, now_ms: i64) -> usize {
        self.applications.iter().filter(|a| a.is_valid(now_ms)).count()
    }
    pub fn valid_approval_percentage(&self, now_ms: i64) -> u8 {
        if self.total_signatories == 0 { return 0; }
        let valid = self.valid_approval_count(now_ms) as u32;
        let pct = (valid * 100) / u32::from(self.total_signatories);
        pct.min(100) as u8
    }
    pub fn meets_authority(&self, now_ms: i64) -> bool {
        let valid_count = self.valid_approval_count(now_ms);
        let valid_pct = self.valid_approval_percentage(now_ms);
        match self.mode {
            AuthorityMode::Single => valid_count >= self.required_approvals as usize && valid_pct >= self.threshold && valid_count >= 1,
            AuthorityMode::Multi => valid_count >= self.required_approvals as usize && valid_pct >= self.threshold,
            AuthorityMode::Dynamic => valid_count >= self.required_approvals as usize,
        }
    }
}

impl fmt::Display for HumanAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mode {
            AuthorityMode::Single => write!(f, "HA[single:{}]", self.name),
            AuthorityMode::Multi => write!(f, "HA[multi:{}-of-{} threshold={}%]", self.required_approvals, self.total_signatories, self.threshold),
            AuthorityMode::Dynamic => write!(f, "HA[dynamic:{} required={} threshold={}%]", self.name, self.required_approvals, self.threshold),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthorityMultisigOutcome {
    Approved { token: crate::owner::OwnerToken, authority_id: String, signature_count: usize, required: usize, threshold: u8, touches_e_layer: bool },
    ReadOnlyRejected,
    InsufficientSignatures { token: crate::owner::OwnerToken, collected: usize, required: usize },
    ThresholdNotMet { token: crate::owner::OwnerToken, valid_count: usize, percentage: u8, required_threshold: u8 },
    UnknownSignatory(String),
}

impl MultiSigPolicy {
    pub fn process_owner_request_with_authority(&self, request: &crate::owner::OwnerRequest, collected_signatures: &[String], authority: &HumanAuthority, _now_ms: i64) -> AuthorityMultisigOutcome {
        if !request.token.can_attempt_core_rule() && request.touches_e_layer() {
            return AuthorityMultisigOutcome::ReadOnlyRejected;
        }
        for sig in collected_signatures {
            if !self.signatories.iter().any(|s| s.id == *sig) {
                return AuthorityMultisigOutcome::UnknownSignatory(sig.clone());
            }
        }
        let required = authority.required_approvals as usize;
        if collected_signatures.len() < required {
            return AuthorityMultisigOutcome::InsufficientSignatures {
                token: request.token, collected: collected_signatures.len(), required,
            };
        }
        match authority.mode {
            AuthorityMode::Single => {
                if authority.threshold != 100 {
                    return AuthorityMultisigOutcome::ThresholdNotMet {
                        token: request.token, valid_count: collected_signatures.len(),
                        percentage: 100, required_threshold: 100,
                    };
                }
                AuthorityMultisigOutcome::Approved {
                    token: request.token, authority_id: authority.authority_id.clone(),
                    signature_count: collected_signatures.len(), required, threshold: authority.threshold,
                    touches_e_layer: request.touches_e_layer(),
                }
            }
            AuthorityMode::Multi => {
                let n = authority.total_signatories.max(1) as usize;
                let percentage = ((collected_signatures.len() * 100) / n).min(100) as u8;
                if percentage < authority.threshold {
                    return AuthorityMultisigOutcome::ThresholdNotMet {
                        token: request.token, valid_count: collected_signatures.len(),
                        percentage, required_threshold: authority.threshold,
                    };
                }
                AuthorityMultisigOutcome::Approved {
                    token: request.token, authority_id: authority.authority_id.clone(),
                    signature_count: collected_signatures.len(), required, threshold: authority.threshold,
                    touches_e_layer: request.touches_e_layer(),
                }
            }
            AuthorityMode::Dynamic => {
                AuthorityMultisigOutcome::Approved {
                    token: request.token, authority_id: authority.authority_id.clone(),
                    signature_count: collected_signatures.len(), required, threshold: authority.threshold,
                    touches_e_layer: request.touches_e_layer(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::owner::{OwnerAction, OwnerRequest, OwnerToken};
    use crate::mock_biometric::MockBiometric;

    fn sigs(names: &[&str]) -> Vec<String> { names.iter().map(|n| (*n).to_string()).collect() }
    fn req(token: OwnerToken) -> OwnerRequest {
        OwnerRequest::new("req-test", token, OwnerAction::AuditQuery, "test-user", "test")
    }

    #[test]
    fn ha_mode_required_signatures() {
        let single = HAMode::SingleHuman(SingleHumanPolicy::new("h1", "Alice", HAAuthentication::WindowsHello));
        assert_eq!(single.required_signatures(), 1);
        let multi = HAMode::MultiHuman(MultiSigPolicy::default_2_of_3());
        assert_eq!(multi.required_signatures(), 2);
        assert_eq!(multi.human_count(), 3);
        let off = HAMode::Offline;
        assert_eq!(off.required_signatures(), 0);
        assert!(off.is_offline());
    }

    #[test]
    fn biometric_provider_trait_methods() {
        let b = MockBiometric::new();
        assert!(b.is_available());
        assert_eq!(b.provider_name(), "mock-biometric");
    }

    #[test]
    fn biometric_result_predicates() {
        let a = BiometricResult::Authenticated { confidence: 0.9, at_ms: 0 };
        assert!(a.is_authenticated());
        let c = BiometricResult::CoercionDetected { stress_level: 0.5, at_ms: 0 };
        assert!(c.is_coercion());
        let f = BiometricResult::Failed { reason: "x".into(), at_ms: 0 };
        assert!(f.is_failed());
        let u = BiometricResult::Unavailable { reason: "x".into() };
        assert!(u.is_unavailable());
    }

    #[test]
    fn single_human_policy_new() {
        let p = SingleHumanPolicy::new("h1", "Alice", HAAuthentication::WindowsHello);
        assert_eq!(p.human_id, "h1");
        assert_eq!(p.name, "Alice");
        assert_eq!(p.authentication, HAAuthentication::WindowsHello);
    }

    #[test]
    fn multi_sig_policy_new_validates_m_lt_n() {
        assert!(MultiSigPolicy::new(2, vec![
            Signatory::new("a", "A", HAAuthentication::FIDO2),
            Signatory::new("b", "B", HAAuthentication::FIDO2),
        ]).is_ok());
        assert!(MultiSigPolicy::new(3, vec![
            Signatory::new("a", "A", HAAuthentication::FIDO2),
        ]).is_err());
        assert!(MultiSigPolicy::new(0, vec![
            Signatory::new("a", "A", HAAuthentication::FIDO2),
        ]).is_err());
    }

    #[test]
    fn multi_sig_default_2_of_3() {
        let p = MultiSigPolicy::default_2_of_3();
        assert_eq!(p.required, 2);
        assert_eq!(p.signatories.len(), 3);
    }

    #[test]
    fn multi_sig_three_of_five() {
        let p = MultiSigPolicy::three_of_five();
        assert_eq!(p.required, 3);
        assert_eq!(p.signatories.len(), 5);
    }

    #[test]
    fn meets_threshold() {
        let p = MultiSigPolicy::default_2_of_3();
        assert!(p.meets_threshold(&["a".into(), "b".into()]));
        assert!(!p.meets_threshold(&["a".into()]));
    }

    #[test]
    fn process_owner_request_master_approved() {
        let p = MultiSigPolicy::default_2_of_3();
        let r = req(OwnerToken::Master);
        let outcome = p.process_owner_request(&r, &sigs(&["h-1", "h-2"]));
        assert!(matches!(outcome, OwnerRequestMultisigOutcome::Approved { signature_count: 2, .. }));
    }

    #[test]
    fn process_owner_request_readonly_rejected() {
        let p = MultiSigPolicy::default_2_of_3();
        let r = OwnerRequest::new("r", OwnerToken::ReadOnly, OwnerAction::ModifyL0Threshold, "u", "x");
        let outcome = p.process_owner_request(&r, &sigs(&["h-1", "h-2"]));
        assert!(matches!(outcome, OwnerRequestMultisigOutcome::ReadOnlyRejected));
    }

    #[test]
    fn process_owner_request_insufficient() {
        let p = MultiSigPolicy::default_2_of_3();
        let r = req(OwnerToken::Master);
        let outcome = p.process_owner_request(&r, &sigs(&["h-1"]));
        assert!(matches!(outcome, OwnerRequestMultisigOutcome::InsufficientSignatures { collected: 1, required: 2, .. }));
    }

    #[test]
    fn process_owner_request_unknown_signatory() {
        let p = MultiSigPolicy::default_2_of_3();
        let r = req(OwnerToken::Master);
        let outcome = p.process_owner_request(&r, &sigs(&["h-1", "unknown"]));
        assert!(matches!(outcome, OwnerRequestMultisigOutcome::UnknownSignatory(_)));
    }

    #[test]
    fn approval_validity_check() {
        let mut a = HumanApproval::new("a1", "h-1", "Alice", 1000, "x");
        assert!(a.is_valid(2000));
        a.revoked = true;
        assert!(!a.is_valid(2000));
        a.revoked = false;
        a = a.with_expiry(2000);
        assert!(a.is_valid(1999));
        assert!(!a.is_valid(2000));
        assert!(!a.is_valid(3000));
    }

    #[test]
    fn authority_single_defaults() {
        let h = HumanAuthority::single("h-1", "Alice");
        assert_eq!(h.required_approvals, 1);
        assert_eq!(h.threshold, 100);
        assert_eq!(h.total_signatories, 1);
        assert_eq!(h.mode, AuthorityMode::Single);
    }

    #[test]
    fn authority_multi_2_of_3_threshold() {
        let h = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        assert_eq!(h.required_approvals, 2);
        assert_eq!(h.threshold, 66);
    }

    #[test]
    fn authority_multi_3_of_5_threshold() {
        let h = HumanAuthority::multi("ha-2", "board", 3, 5).unwrap();
        assert_eq!(h.threshold, 60);
    }

    #[test]
    fn authority_multi_rejects_m_gt_n() {
        assert!(HumanAuthority::multi("x", "x", 4, 3).is_err());
        assert!(HumanAuthority::multi("x", "x", 0, 3).is_err());
        assert!(HumanAuthority::multi("x", "x", 1, 0).is_err());
    }

    #[test]
    fn authority_dynamic_user_defined() {
        let h = HumanAuthority::dynamic("d-1", "ctx", 2, 80, 4);
        assert_eq!(h.mode, AuthorityMode::Dynamic);
        assert_eq!(h.threshold, 80);
    }

    #[test]
    fn authority_records_and_revokes() {
        let mut h = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        h.record_approval(HumanApproval::new("a1", "h-1", "A", 1000, "x"));
        h.record_approval(HumanApproval::new("a2", "h-2", "B", 1000, "x"));
        assert_eq!(h.applications.len(), 2);
        assert!(h.revoke_approval("a1"));
        assert_eq!(h.valid_approval_count(2000), 1);
        assert!(!h.revoke_approval("nonexistent"));
    }

    #[test]
    fn authority_valid_percentage() {
        let mut h = HumanAuthority::multi("ha-1", "team", 2, 4).unwrap();
        h.record_approval(HumanApproval::new("a1", "h-1", "A", 1000, "x"));
        h.record_approval(HumanApproval::new("a2", "h-2", "B", 1000, "x"));
        assert_eq!(h.valid_approval_percentage(2000), 50);
    }

    #[test]
    fn meets_authority_single() {
        let mut h = HumanAuthority::single("h-1", "Alice");
        h.record_approval(HumanApproval::new("a1", "h-1", "Alice", 1000, "x"));
        assert!(h.meets_authority(2000));
    }

    #[test]
    fn meets_authority_multi_2_of_3() {
        let mut h = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        h.record_approval(HumanApproval::new("a1", "h-1", "A", 1000, "x"));
        h.record_approval(HumanApproval::new("a2", "h-2", "B", 1000, "x"));
        assert!(h.meets_authority(2000));
        let h1 = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        assert!(!h1.meets_authority(2000));
    }

    #[test]
    fn meets_authority_dynamic() {
        let mut h = HumanAuthority::dynamic("d-1", "ctx", 2, 50, 5);
        h.record_approval(HumanApproval::new("a1", "h-1", "A", 1000, "x"));
        h.record_approval(HumanApproval::new("a2", "h-2", "B", 1000, "x"));
        assert!(h.meets_authority(2000));
    }

    #[test]
    fn process_request_with_authority_single() {
        let policy = MultiSigPolicy { required: 1, signatories: vec![Signatory::new("h-1", "Alice", HAAuthentication::FIDO2)] };
        let ha = HumanAuthority::single("h-1", "Alice");
        let r = req(OwnerToken::Master);
        let outcome = policy.process_owner_request_with_authority(&r, &sigs(&["h-1"]), &ha, 1000);
        assert!(matches!(outcome, AuthorityMultisigOutcome::Approved { .. }));
    }

    #[test]
    fn process_request_with_authority_multi() {
        let policy = MultiSigPolicy::default_2_of_3();
        let ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        let r = req(OwnerToken::Master);
        let outcome = policy.process_owner_request_with_authority(&r, &sigs(&["h-1", "h-2"]), &ha, 1000);
        assert!(matches!(outcome, AuthorityMultisigOutcome::Approved { signature_count: 2, .. }));
    }

    #[test]
    fn process_request_with_authority_insufficient() {
        let policy = MultiSigPolicy::default_2_of_3();
        let ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        let r = req(OwnerToken::Master);
        let outcome = policy.process_owner_request_with_authority(&r, &sigs(&["h-1"]), &ha, 1000);
        assert!(matches!(outcome, AuthorityMultisigOutcome::InsufficientSignatures { .. }));
    }

    #[test]
    fn process_request_with_authority_unknown() {
        let policy = MultiSigPolicy::default_2_of_3();
        let ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        let r = req(OwnerToken::Master);
        let outcome = policy.process_owner_request_with_authority(&r, &sigs(&["h-1", "unknown"]), &ha, 1000);
        assert!(matches!(outcome, AuthorityMultisigOutcome::UnknownSignatory(_)));
    }

    #[test]
    fn process_request_with_authority_readonly() {
        let policy = MultiSigPolicy::default_2_of_3();
        let ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        let r = OwnerRequest::new("r", OwnerToken::ReadOnly, OwnerAction::ModifyL0Threshold, "u", "x");
        let outcome = policy.process_owner_request_with_authority(&r, &sigs(&["h-1", "h-2", "h-3"]), &ha, 1000);
        assert!(matches!(outcome, AuthorityMultisigOutcome::ReadOnlyRejected));
    }

    #[test]
    fn process_request_with_authority_dynamic() {
        let policy = MultiSigPolicy::three_of_five();
        let ha = HumanAuthority::dynamic("d-1", "ctx", 3, 50, 5);
        let r = req(OwnerToken::Master);
        let outcome = policy.process_owner_request_with_authority(&r, &sigs(&["h-0", "h-1", "h-2"]), &ha, 1000);
        assert!(matches!(outcome, AuthorityMultisigOutcome::Approved { signature_count: 3, threshold: 50, .. }));
    }

    #[test]
    fn ha_mode_display_format() {
        let single = HAMode::SingleHuman(SingleHumanPolicy::new("h1", "Alice", HAAuthentication::WindowsHello));
        let s = format!("{}", single);
        assert!(s.contains("single"));
        assert!(s.contains("h1"));
        let multi = HAMode::MultiHuman(MultiSigPolicy::default_2_of_3());
        let s2 = format!("{}", multi);
        assert!(s2.contains("multi"));
        let off = HAMode::Offline;
        assert_eq!(format!("{}", off), "offline");
    }

    #[test]
    fn authority_display_format() {
        let s = HumanAuthority::single("h-1", "Alice");
        assert_eq!(s.to_string(), "HA[single:Alice]");
        let m = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        assert_eq!(m.to_string(), "HA[multi:2-of-3 threshold=66%]");
        let d = HumanAuthority::dynamic("d-1", "ctx", 2, 80, 5);
        assert_eq!(d.to_string(), "HA[dynamic:ctx required=2 threshold=80%]");
    }
}

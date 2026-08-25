//! HA 部署模式

use crate::ha::{AuthorityMode, BiometricProvider, BiometricResult, HumanAuthority, MultiSigPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentContext {
    ExistenceLayer,
    NormalLayer,
    EmergencyLayer,
    ReflectionLayer,
}

impl DeploymentContext {
    pub fn threshold_adjustment(&self) -> i32 {
        match self {
            Self::ExistenceLayer => 20,
            Self::NormalLayer => 0,
            Self::EmergencyLayer => -20,
            Self::ReflectionLayer => 30,
        }
    }
    pub fn requires_reflection(&self) -> bool {
        matches!(self, Self::ReflectionLayer | Self::ExistenceLayer)
    }
    pub fn min_threshold(&self) -> u8 {
        match self {
            Self::EmergencyLayer => 30,
            _ => 50,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentMode {
    Single,
    Multi,
    Dynamic,
}

impl DeploymentMode {
    pub fn select_for_context(ctx: DeploymentContext, existing_total: u8) -> Self {
        match existing_total {
            0 => Self::Dynamic,
            1 => Self::Single,
            _ => match ctx {
                DeploymentContext::ExistenceLayer | DeploymentContext::ReflectionLayer => Self::Multi,
                _ => Self::Dynamic,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentOutcome {
    ApprovedSingle { signature_id: String, confidence: f64, context: DeploymentContext },
    ApprovedMulti { valid_signatures: usize, required: usize, effective_threshold: u8, context: DeploymentContext },
    ApprovedDynamic { valid_signatures: usize, required: usize, base_threshold: u8, adjusted_threshold: u8, context: DeploymentContext },
    RejectedSingleHighRisk { risk: String, max_allowed: String },
    RejectedMultiInsufficient { have: usize, need: usize, actual_pct: u8, threshold: u8 },
    RejectedDynamicInsufficient { actual_pct: u8, adjusted_threshold: u8, context: DeploymentContext },
    RejectedReflectionPending { remaining_ms: i64 },
}

impl DeploymentOutcome {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::ApprovedSingle { .. } | Self::ApprovedMulti { .. } | Self::ApprovedDynamic { .. })
    }
    pub fn is_rejected(&self) -> bool { !self.is_approved() }
}

pub struct HADeploymentEnforcer<'a> {
    pub mode: DeploymentMode,
    pub authority: &'a HumanAuthority,
    pub multi_policy: Option<&'a MultiSigPolicy>,
    pub biometric: Option<&'a dyn BiometricProvider>,
    pub context: DeploymentContext,
}

impl<'a> HADeploymentEnforcer<'a> {
    pub fn single(authority: &'a HumanAuthority, biometric: &'a dyn BiometricProvider, context: DeploymentContext) -> Self {
        Self { mode: DeploymentMode::Single, authority, multi_policy: None, biometric: Some(biometric), context }
    }
    pub fn multi(authority: &'a HumanAuthority, policy: &'a MultiSigPolicy, context: DeploymentContext) -> Self {
        Self { mode: DeploymentMode::Multi, authority, multi_policy: Some(policy), biometric: None, context }
    }
    pub fn dynamic(authority: &'a HumanAuthority, context: DeploymentContext) -> Self {
        Self { mode: DeploymentMode::Dynamic, authority, multi_policy: None, biometric: None, context }
    }

    pub fn enforce(&self, collected_signatures: &[String], risk_level: &str, now_ms: i64) -> DeploymentOutcome {
        match self.mode {
            DeploymentMode::Single => self.enforce_single(collected_signatures, risk_level, now_ms),
            DeploymentMode::Multi => self.enforce_multi(collected_signatures, now_ms),
            DeploymentMode::Dynamic => self.enforce_dynamic(collected_signatures, now_ms),
        }
    }

    fn enforce_single(&self, collected_signatures: &[String], risk_level: &str, now_ms: i64) -> DeploymentOutcome {
        if collected_signatures.len() != 1 {
            return DeploymentOutcome::RejectedMultiInsufficient { have: collected_signatures.len(), need: 1, actual_pct: 0, threshold: 100 };
        }
        let risk_rank = match risk_level.to_ascii_lowercase().as_str() {
            "low" | "info" => 0,
            "medium" => 1,
            "high" | "critical" | "nuclear" => 2,
            _ => 0,
        };
        if risk_rank >= 2 {
            return DeploymentOutcome::RejectedSingleHighRisk { risk: risk_level.to_string(), max_allowed: "medium".to_string() };
        }
        if let Some(bio) = self.biometric {
            let sig_id = &collected_signatures[0];
            let result = bio.authenticate(sig_id);
            if let BiometricResult::Authenticated { confidence, .. } = result {
                if !self.authority.meets_authority(now_ms) {
                    return DeploymentOutcome::RejectedMultiInsufficient {
                        have: self.authority.valid_approval_count(now_ms), need: 1,
                        actual_pct: self.authority.valid_approval_percentage(now_ms), threshold: 100,
                    };
                }
                return DeploymentOutcome::ApprovedSingle { signature_id: sig_id.clone(), confidence, context: self.context };
            }
        }
        if !self.authority.meets_authority(now_ms) {
            return DeploymentOutcome::RejectedMultiInsufficient {
                have: self.authority.valid_approval_count(now_ms), need: 1,
                actual_pct: self.authority.valid_approval_percentage(now_ms), threshold: 100,
            };
        }
        DeploymentOutcome::ApprovedSingle { signature_id: collected_signatures[0].clone(), confidence: 1.0, context: self.context }
    }

    fn enforce_multi(&self, collected_signatures: &[String], now_ms: i64) -> DeploymentOutcome {
        let valid = self.authority.valid_approval_count(now_ms);
        let pct = self.authority.valid_approval_percentage(now_ms);
        let required = self.authority.required_approvals as usize;
        let threshold = self.authority.threshold;
        if collected_signatures.len() > self.authority.total_signatories as usize {
            return DeploymentOutcome::RejectedMultiInsufficient { have: valid, need: required, actual_pct: pct, threshold };
        }
        if valid >= required && pct >= threshold {
            DeploymentOutcome::ApprovedMulti { valid_signatures: valid, required, effective_threshold: threshold, context: self.context }
        } else {
            DeploymentOutcome::RejectedMultiInsufficient { have: valid, need: required, actual_pct: pct, threshold }
        }
    }

    fn enforce_dynamic(&self, _collected_signatures: &[String], now_ms: i64) -> DeploymentOutcome {
        let valid = self.authority.valid_approval_count(now_ms);
        let pct = self.authority.valid_approval_percentage(now_ms);
        let required = self.authority.required_approvals as usize;
        let base = self.authority.threshold;
        let adjustment = self.context.threshold_adjustment();
        let adjusted = (i32::from(base) + adjustment).clamp(i32::from(self.context.min_threshold()), 100) as u8;
        if valid >= required && pct >= adjusted {
            DeploymentOutcome::ApprovedDynamic { valid_signatures: valid, required, base_threshold: base, adjusted_threshold: adjusted, context: self.context }
        } else {
            DeploymentOutcome::RejectedDynamicInsufficient { actual_pct: pct, adjusted_threshold: adjusted, context: self.context }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentReflectionTracker {
    pub started_at_ms: i64,
    pub period_ms: i64,
}

impl DeploymentReflectionTracker {
    pub fn new(started_at_ms: i64, period_ms: i64) -> Self { Self { started_at_ms, period_ms } }
    pub fn is_in_reflection(&self, now_ms: i64) -> bool { now_ms < self.started_at_ms + self.period_ms }
    pub fn remaining_ms(&self, now_ms: i64) -> i64 { (self.started_at_ms + self.period_ms - now_ms).max(0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ha::{HumanApproval, HAAuthentication, Signatory};
    use crate::mock_biometric::MockBiometric;

    fn auth_with_approvals(n: u8, mode: AuthorityMode) -> HumanAuthority {
        let mut h = match mode {
            AuthorityMode::Single => HumanAuthority::single("h-1", "Alice"),
            AuthorityMode::Multi => HumanAuthority::multi("ha-1", "team", 2, n).unwrap(),
            AuthorityMode::Dynamic => HumanAuthority::dynamic("d-1", "ctx", 2, 50, n),
        };
        for i in 0..n {
            h.record_approval(HumanApproval::new(format!("ap-{}", i), format!("h-{}", i), format!("S{}", i), 1000, "test"));
        }
        h
    }

    #[test] fn existence_layer_threshold_plus_20() {
        assert_eq!(DeploymentContext::ExistenceLayer.threshold_adjustment(), 20);
        assert!(DeploymentContext::ExistenceLayer.requires_reflection());
    }
    #[test] fn normal_layer_threshold_zero() {
        assert_eq!(DeploymentContext::NormalLayer.threshold_adjustment(), 0);
        assert!(!DeploymentContext::NormalLayer.requires_reflection());
    }
    #[test] fn emergency_layer_threshold_minus_20() {
        assert_eq!(DeploymentContext::EmergencyLayer.threshold_adjustment(), -20);
        assert_eq!(DeploymentContext::EmergencyLayer.min_threshold(), 30);
    }
    #[test] fn reflection_layer_threshold_plus_30() {
        assert_eq!(DeploymentContext::ReflectionLayer.threshold_adjustment(), 30);
        assert!(DeploymentContext::ReflectionLayer.requires_reflection());
    }
    #[test] fn select_for_context_zero_returns_dynamic() {
        assert_eq!(DeploymentMode::select_for_context(DeploymentContext::NormalLayer, 0), DeploymentMode::Dynamic);
    }
    #[test] fn select_for_context_one_returns_single() {
        assert_eq!(DeploymentMode::select_for_context(DeploymentContext::NormalLayer, 1), DeploymentMode::Single);
    }
    #[test] fn select_for_context_multi_e_layer_returns_multi() {
        assert_eq!(DeploymentMode::select_for_context(DeploymentContext::ExistenceLayer, 5), DeploymentMode::Multi);
    }
    #[test] fn select_for_context_multi_normal_returns_dynamic() {
        assert_eq!(DeploymentMode::select_for_context(DeploymentContext::NormalLayer, 5), DeploymentMode::Dynamic);
    }
    #[test] fn single_mode_approved_with_one_signature() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let e = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        assert!(e.enforce(&["h-1".into()], "low", 2000).is_approved());
    }
    #[test] fn single_mode_rejected_zero_signatures() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let e = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        assert!(e.enforce(&[], "low", 2000).is_rejected());
    }
    #[test] fn single_mode_rejected_two_signatures() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let e = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        assert!(e.enforce(&["h-1".into(), "h-2".into()], "low", 2000).is_rejected());
    }
    #[test] fn single_mode_rejected_high_risk() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let e = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        let o = e.enforce(&["h-1".into()], "high", 2000);
        match o {
            DeploymentOutcome::RejectedSingleHighRisk { risk, max_allowed } => {
                assert_eq!(risk, "high");
                assert_eq!(max_allowed, "medium");
            }
            _ => panic!("expected RejectedSingleHighRisk"),
        }
    }
    #[test] fn single_mode_rejected_critical() {
        let ha = auth_with_approvals(1, AuthorityMode::Single);
        let bio = MockBiometric::new();
        let e = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
        assert!(e.enforce(&["h-1".into()], "critical", 2000).is_rejected());
    }
    #[test] fn multi_mode_2_of_3_approved() {
        let ha = auth_with_approvals(3, AuthorityMode::Multi);
        let policy = MultiSigPolicy::default_2_of_3();
        let e = HADeploymentEnforcer::multi(&ha, &policy, DeploymentContext::ExistenceLayer);
        assert!(e.enforce(&["h-0".into(), "h-1".into()], "high", 2000).is_approved());
    }
    #[test] fn multi_mode_1_of_3_rejected() {
        let mut ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        ha.record_approval(HumanApproval::new("ap-0", "h-0", "S0", 1000, "x"));
        let policy = MultiSigPolicy::default_2_of_3();
        let e = HADeploymentEnforcer::multi(&ha, &policy, DeploymentContext::ExistenceLayer);
        let o = e.enforce(&["h-0".into()], "high", 2000);
        match o {
            DeploymentOutcome::RejectedMultiInsufficient { have, need, .. } => {
                assert_eq!(have, 1);
                assert_eq!(need, 2);
            }
            _ => panic!("expected RejectedMultiInsufficient"),
        }
    }
    #[test] fn dynamic_mode_e_layer_raises() {
        let ha = auth_with_approvals(5, AuthorityMode::Dynamic);
        let e = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::ExistenceLayer);
        let o = e.enforce(&[], "high", 2000);
        match o {
            DeploymentOutcome::ApprovedDynamic { base_threshold, adjusted_threshold, .. } => {
                assert_eq!(base_threshold, 50);
                assert_eq!(adjusted_threshold, 70);
            }
            _ => panic!("expected ApprovedDynamic"),
        }
    }
    #[test] fn dynamic_mode_emergency_lowers() {
        let ha = auth_with_approvals(5, AuthorityMode::Dynamic);
        let e = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::EmergencyLayer);
        let o = e.enforce(&[], "low", 2000);
        match o {
            DeploymentOutcome::ApprovedDynamic { adjusted_threshold, .. } => assert_eq!(adjusted_threshold, 30),
            _ => panic!(),
        }
    }
    #[test] fn dynamic_mode_emergency_floor_30() {
        let mut ha = HumanAuthority::dynamic("d-1", "ctx", 1, 10, 5);
        for i in 0..5 {
            ha.record_approval(HumanApproval::new(format!("ap-{}", i), format!("h-{}", i), format!("S{}", i), 1000, "x"));
        }
        let e = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::EmergencyLayer);
        let o = e.enforce(&[], "low", 2000);
        match o {
            DeploymentOutcome::ApprovedDynamic { adjusted_threshold, .. } => assert!(adjusted_threshold >= 30),
            _ => panic!(),
        }
    }
    #[test] fn dynamic_mode_reflection_plus_30() {
        let ha = auth_with_approvals(5, AuthorityMode::Dynamic);
        let e = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::ReflectionLayer);
        let o = e.enforce(&[], "low", 2000);
        match o {
            DeploymentOutcome::ApprovedDynamic { adjusted_threshold, .. } => assert_eq!(adjusted_threshold, 80),
            _ => panic!(),
        }
    }
    #[test] fn reflection_tracker_active_in_window() {
        let t = DeploymentReflectionTracker::new(1000, 7 * 86_400_000);
        assert!(t.is_in_reflection(5_000_000));
        assert!(!t.is_in_reflection(8 * 86_400_000));
    }
    #[test] fn reflection_tracker_remaining() {
        let t = DeploymentReflectionTracker::new(1000, 10_000);
        assert_eq!(t.remaining_ms(2000), 9000);
        assert_eq!(t.remaining_ms(11_000), 0);
        assert_eq!(t.remaining_ms(20_000), 0);
    }
    #[test] fn outcome_predicates() {
        let s = DeploymentOutcome::ApprovedSingle { signature_id: "s".into(), confidence: 0.9, context: DeploymentContext::NormalLayer };
        let m = DeploymentOutcome::ApprovedMulti { valid_signatures: 2, required: 2, effective_threshold: 66, context: DeploymentContext::NormalLayer };
        let d = DeploymentOutcome::ApprovedDynamic { valid_signatures: 3, required: 3, base_threshold: 50, adjusted_threshold: 70, context: DeploymentContext::ExistenceLayer };
        assert!(s.is_approved());
        assert!(m.is_approved());
        assert!(d.is_approved());
        assert!(!s.is_rejected());
        let r1 = DeploymentOutcome::RejectedSingleHighRisk { risk: "high".into(), max_allowed: "medium".into() };
        let r2 = DeploymentOutcome::RejectedMultiInsufficient { have: 1, need: 2, actual_pct: 33, threshold: 66 };
        let r3 = DeploymentOutcome::RejectedDynamicInsufficient { actual_pct: 20, adjusted_threshold: 70, context: DeploymentContext::ExistenceLayer };
        let r4 = DeploymentOutcome::RejectedReflectionPending { remaining_ms: 1000 };
        assert!(r1.is_rejected());
        assert!(r2.is_rejected());
        assert!(r3.is_rejected());
        assert!(r4.is_rejected());
    }
}

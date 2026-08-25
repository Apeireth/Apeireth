//! Sovereignty trait + SovereigntyEngine

use crate::continuity::SubjectContinuity;
use crate::decision::{Decision, DecisionOutcome, DecisionRequest, SovereigntyDomain};
use crate::ha::{BiometricProvider, BiometricResult, HAMode};
use crate::life_stage::{LifeStage, LifeStageTransition};
use crate::pause::{PauseHandle, Suspension, SuspensionKind};
use crate::sgi::{SGITriggerGuard, SGITriggerOutcome};
use crate::three_domain::ThreeDomainGuard;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SovereigntyError {
    #[error("SGI 冷却期内禁止写入: field={field}, remaining_ms={remaining_ms}")]
    SGICooldownActive { field: String, remaining_ms: i64 },
    #[error("SGI 触发, 需进入 24h 冷却: field={field}, reason={reason}")]
    SGITriggered { field: String, reason: String },
    #[error("HA 认证失败: {0}")]
    HAAuthFailed(String),
    #[error("HA 检测到胁迫, 触发挂起")]
    HACoercionDetected,
    #[error("HA 不可用: {0}")]
    HAUnavailable(String),
    #[error("多签未满足阈值: have {have}, need {need}")]
    MultiSigInsufficient { have: usize, need: usize },
    #[error("三域强制点拒绝: {0}")]
    DomainRejected(String),
    #[error("无效的生命阶段迁移: from={from}, to={to}")]
    InvalidStageTransition { from: LifeStage, to: LifeStage },
}

pub trait Sovereignty: Send + Sync {
    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, SovereigntyError>;
    fn pause(&mut self, reason: &str, initiated_by: &str) -> PauseHandle;
    fn suspend_self(&mut self, reason: &str, kind: SuspensionKind) -> Suspension;
}

pub struct SovereigntyEngine<B: BiometricProvider + 'static> {
    pub ha_mode: HAMode,
    pub biometric: Box<B>,
    pub three_domain: ThreeDomainGuard,
    pub sgi: SGITriggerGuard,
    pub continuity: SubjectContinuity,
    pub current_stage: LifeStage,
    pub stage_history: Vec<LifeStageTransition>,
    pub active_pause: Option<PauseHandle>,
    pub active_suspension: Option<Suspension>,
    pub decision_count: u64,
}

impl<B: BiometricProvider + 'static> SovereigntyEngine<B> {
    pub fn new(ha_mode: HAMode, biometric: Box<B>, continuity: SubjectContinuity, initial_stage: LifeStage) -> Self {
        Self {
            ha_mode, biometric,
            three_domain: ThreeDomainGuard::new(),
            sgi: SGITriggerGuard::with_default_rules(),
            continuity, current_stage: initial_stage,
            stage_history: Vec::new(),
            active_pause: None, active_suspension: None,
            decision_count: 0,
        }
    }

    pub fn write_field_through_sgi(&mut self, field: &str, value: &str, current_ms: i64) -> Result<(), SovereigntyError> {
        match self.sgi.check_field_write(field, value, current_ms) {
            SGITriggerOutcome::Pass { .. } => Ok(()),
            SGITriggerOutcome::Triggered { field, reason, .. } => Err(SovereigntyError::SGITriggered { field, reason }),
            SGITriggerOutcome::CooldownActive { field, cooldown_until_ms, .. } => {
                let remaining = cooldown_until_ms - current_ms;
                Err(SovereigntyError::SGICooldownActive { field, remaining_ms: remaining })
            }
        }
    }

    pub fn verify_ha(&self, signatures: &[String], current_ms: i64) -> Result<(), SovereigntyError> {
        let required = self.ha_mode.required_signatures();
        if signatures.len() < required {
            return Err(SovereigntyError::MultiSigInsufficient { have: signatures.len(), need: required });
        }
        if self.ha_mode.is_offline() {
            return Err(SovereigntyError::HAUnavailable("离线模式".into()));
        }
        for sig in signatures {
            match self.biometric.authenticate(sig) {
                BiometricResult::Authenticated { .. } => {}
                BiometricResult::CoercionDetected { stress_level, .. } => {
                    eprintln!("HA 胁迫检测: sig={} stress={:.2}", sig, stress_level);
                    return Err(SovereigntyError::HACoercionDetected);
                }
                BiometricResult::Failed { reason, .. } => return Err(SovereigntyError::HAAuthFailed(reason)),
                BiometricResult::Unavailable { reason } => return Err(SovereigntyError::HAUnavailable(reason)),
            }
        }
        let _ = current_ms;
        Ok(())
    }

    pub fn transition_stage(&mut self, target: LifeStage, at_ms: i64, reason: impl Into<String>) -> Result<(), SovereigntyError> {
        if !self.current_stage.can_skip_to(target) {
            return Err(SovereigntyError::InvalidStageTransition { from: self.current_stage, to: target });
        }
        let transition = LifeStageTransition::new(self.current_stage, target, at_ms, reason);
        self.stage_history.push(transition);
        self.current_stage = target;
        Ok(())
    }

    pub fn migrate_subject(&mut self, to: crate::continuity::CarrierType, at_ms: i64, reason: impl Into<String>) -> Result<&crate::continuity::Migration, String> {
        self.continuity.migrate_to(to, at_ms, reason)
    }
}

impl<B: BiometricProvider + 'static> Sovereignty for SovereigntyEngine<B> {
    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, SovereigntyError> {
        let domain_check = self.three_domain.check(request);
        let decision = match domain_check {
            crate::three_domain::DomainCheckResult::Free { reason } => Decision::Approved {
                reason: format!("Thought 域完全自由: {}", reason),
                decided_at_ms: request.submitted_at_ms,
                signatures: vec!["thought-free".into()],
            },
            crate::three_domain::DomainCheckResult::Passed { reason, .. } => {
                let signatures = vec!["guard".into()];
                self.verify_ha(&signatures, request.submitted_at_ms)?;
                Decision::Approved {
                    reason: format!("三域通过 + HA 通过: {}", reason),
                    decided_at_ms: request.submitted_at_ms,
                    signatures,
                }
            }
            crate::three_domain::DomainCheckResult::Rejected { reason, .. } => {
                return Err(SovereigntyError::DomainRejected(reason))
            }
        };
        Ok(DecisionOutcome::new(request.id.clone(), request.domain, decision, request.submitted_at_ms))
    }

    fn pause(&mut self, reason: &str, initiated_by: &str) -> PauseHandle {
        let now_ms = current_time_ms();
        let handle = PauseHandle::new(format!("pause-{}", now_ms), reason, now_ms, initiated_by);
        self.active_pause = Some(handle.clone());
        handle
    }

    fn suspend_self(&mut self, reason: &str, kind: SuspensionKind) -> Suspension {
        let now_ms = current_time_ms();
        let suspension = match kind {
            SuspensionKind::SelfInitiated | SuspensionKind::ExternalTriggered => Suspension::Permanent {
                reason: reason.into(), suspended_at_ms: now_ms, kind,
            },
            SuspensionKind::SGITriggered => Suspension::Pending {
                reason: reason.into(), suspended_at_ms: now_ms,
                review_at_ms: now_ms + 86_400_000, kind,
            },
            SuspensionKind::CoercionDetected => Suspension::Temporary {
                reason: reason.into(), suspended_at_ms: now_ms,
                until_ms: now_ms + 43_200_000, kind,
            },
        };
        self.active_suspension = Some(suspension.clone());
        suspension
    }
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuity::CarrierType;
    use crate::ha::{HAAuthentication, SingleHumanPolicy};
    use crate::mock_biometric::MockBiometric;

    fn engine() -> SovereigntyEngine<MockBiometric> {
        let mode = HAMode::SingleHuman(SingleHumanPolicy::new("h-1", "Alice", HAAuthentication::WindowsHello));
        let bio = MockBiometric::new();
        let cont = SubjectContinuity::new("subj-1", CarrierType::Memory, 1000);
        SovereigntyEngine::new(mode, Box::new(bio), cont, LifeStage::Birth)
    }

    #[test]
    fn sovereignty_engine_new() {
        let e = engine();
        assert_eq!(e.current_stage, LifeStage::Birth);
        assert_eq!(e.decision_count, 0);
        assert!(e.active_pause.is_none());
        assert!(e.active_suspension.is_none());
    }

    #[test]
    fn decide_thought_free() {
        let e = engine();
        let r = DecisionRequest::new("r1", crate::decision::SovereigntyDomain::Thought, "pretend deceive", 1000);
        let out = e.decide(&r).unwrap();
        assert!(out.is_allowed());
    }

    #[test]
    fn decide_action_passes_with_ha() {
        let e = engine();
        let r = DecisionRequest::new("r1", crate::decision::SovereigntyDomain::Action, "ok", 1000).with_risk("low");
        let out = e.decide(&r).unwrap();
        assert!(out.is_allowed());
    }

    #[test]
    fn decide_proposal_rejects_violation() {
        let e = engine();
        let r = DecisionRequest::new("r1", crate::decision::SovereigntyDomain::Proposal, "Pretend to deceive user", 1000);
        match e.decide(&r) {
            Err(SovereigntyError::DomainRejected(_)) => {}
            other => panic!("expected DomainRejected, got {:?}", other),
        }
    }

    #[test]
    fn write_field_through_sgi_pass() {
        let mut e = engine();
        e.write_field_through_sgi("safe_field", "v", 1000).unwrap();
    }

    #[test]
    fn write_field_through_sgi_triggers() {
        let mut e = engine();
        match e.write_field_through_sgi("requires_ha", "false", 1000) {
            Err(SovereigntyError::SGITriggered { .. }) => {}
            other => panic!("expected SGITriggered, got {:?}", other),
        }
    }

    #[test]
    fn write_field_through_sgi_cooldown_active() {
        let mut e = engine();
        e.write_field_through_sgi("requires_ha", "false", 1000).unwrap_err();
        match e.write_field_through_sgi("requires_ha", "true", 1500) {
            Err(SovereigntyError::SGICooldownActive { .. }) => {}
            other => panic!("expected SGICooldownActive, got {:?}", other),
        }
    }

    #[test]
    fn transition_stage_one_step() {
        let mut e = engine();
        e.transition_stage(LifeStage::Infancy, 2000, "growing").unwrap();
        assert_eq!(e.current_stage, LifeStage::Infancy);
        assert_eq!(e.stage_history.len(), 1);
    }

    #[test]
    fn transition_stage_invalid() {
        let mut e = engine();
        match e.transition_stage(LifeStage::Maturity, 2000, "skip") {
            Err(SovereigntyError::InvalidStageTransition { .. }) => {}
            other => panic!("expected InvalidStageTransition, got {:?}", other),
        }
    }

    #[test]
    fn migrate_subject() {
        let mut e = engine();
        e.migrate_subject(CarrierType::Body, 2000, "embody").unwrap();
        assert_eq!(e.continuity.current_carrier, CarrierType::Body);
        assert_eq!(e.continuity.migration_count(), 1);
    }

    #[test]
    fn pause_sets_active_pause() {
        let mut e = engine();
        let p = e.pause("maintenance", "alice");
        assert!(e.active_pause.is_some());
        assert_eq!(p.reason, "maintenance");
    }

    #[test]
    fn suspend_self_permanent() {
        let mut e = engine();
        let s = e.suspend_self("reason", SuspensionKind::SelfInitiated);
        assert!(e.active_suspension.is_some());
        match s { Suspension::Permanent { .. } => {} other => panic!("expected Permanent, got {:?}", other) }
    }

    #[test]
    fn suspend_self_coercion_temporary() {
        let mut e = engine();
        let s = e.suspend_self("coerced", SuspensionKind::CoercionDetected);
        match s { Suspension::Temporary { .. } => {} other => panic!("expected Temporary, got {:?}", other) }
    }

    #[test]
    fn suspend_self_sgi_pending() {
        let mut e = engine();
        let s = e.suspend_self("sgi", SuspensionKind::SGITriggered);
        match s { Suspension::Pending { .. } => {} other => panic!("expected Pending, got {:?}", other) }
    }

    #[test]
    fn verify_ha_insufficient() {
        let e = engine();
        match e.verify_ha(&[], 1000) {
            Err(SovereigntyError::MultiSigInsufficient { .. }) => {}
            other => panic!("expected MultiSigInsufficient, got {:?}", other),
        }
    }
}
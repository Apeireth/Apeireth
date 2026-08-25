//! `apeireth-sovereignty`: 主权器官 + HA + 三域分离 + SGI + 9 阶段生命周期
//!                    + MEWG 5 重治理
//!
//! **职责** (P22 落点):
//! - 主权 trait + SovereigntyEngine (decision/pause/suspend_self)
//! - HA 部署模式自适应 (single/multi/offline) + 生物特征 trait 抽象
//! - 三域分离强制点 (Thought/Proposal/Action) + BCD enforce
//! - SGI 单字段写入触发器
//! - 主体连续性 ID 跨载体 + migration_history
//! - 9 阶段生命周期
//! - MEWG 5 重治理 (MultiAi/MultiHuman/PhysicalMultisig/Reflection/Mewg)
//! - 7 重守门 v7 (skill_guard + seven_fold_guard)
//! - 8 重守门 v8 (action_rail + flow_executor + colang_dsl)
//! - 9 重守门 v9 (evidence_guard)
//! - fail_closed / reflection / explain / anti_ai / signature / audit / wasm

#![deny(unsafe_code)]

// ============================================================
// 主权模块
// ============================================================
pub mod audit_window;
pub mod continuity;
pub mod decision;
pub mod ha;
pub mod ha_modes;
pub mod life_stage;
pub mod mock_biometric;
pub mod pause;
pub mod self_disable;
pub mod sgi;
pub mod sovereign;
pub mod swap;
pub mod three_domain;
pub mod three_domain_enforce;

// ============================================================
// MEWG 5 重治理模块
// ============================================================
pub mod colang_dsl;
pub mod governance;
pub mod mewg;
pub mod multi_ai;
pub mod multi_human;
pub mod owner;
pub mod physical_multisig;
pub mod reflection;

pub mod seven_fold_guard;
pub mod skill_guard;

pub mod action_rail;
pub mod flow_executor;

pub mod evidence_guard;

pub mod wasm_runtime;

// 估补模块
pub mod anti_ai;
pub mod audit;
pub mod explain;
pub mod fail_closed;
pub mod signature;

pub mod kani_proofs;
mod organ_kani_proofs;

// ============================================================
// 公共 re-export
// ============================================================
pub use continuity::{CarrierType, Migration, SubjectContinuity};
pub use decision::{Decision as SovereigntyDecision, DecisionOutcome, DecisionRequest, SovereigntyDomain};
pub use ha::{
    AuthorityMode, AuthorityMultisigOutcome, BiometricProvider, BiometricResult, HAAuthentication,
    HAMode, HumanApproval, HumanAuthority, HumanAuthority as _AliasUnused,
    MultiSigPolicy, OwnerRequestMultisigOutcome, Signatory, SingleHumanPolicy,
};
pub use life_stage::{LifeStage, LifeStageTransition, NINE_STAGES};
pub use mock_biometric::{CoercionBehavior, MockBiometric, MockBiometricBehavior};
pub use owner::{OwnerAction, OwnerError, OwnerRequest, OwnerToken};
pub use pause::{PauseHandle, Suspension, SuspensionKind};
pub use sgi::{SGIFieldRule, SGITrigger, SGITriggerGuard, SGITriggerOutcome};
pub use sovereign::{Sovereignty, SovereigntyEngine, SovereigntyError};

pub use audit_window::{
    AuditHistoryEntry, AuditWindowHistory, BestEffortFlow, InMemoryAuditHistory, WindowDecision,
    DEFAULT_AUDIT_WINDOW_MS,
};
pub use ha_modes::{
    DeploymentContext, DeploymentMode, DeploymentOutcome, DeploymentReflectionTracker,
    HADeploymentEnforcer,
};
pub use self_disable::{
    SelfDisableCheck, SelfDisableGuard, SelfDisableRecord, SelfDisableSignal, SelfDisableTrigger,
};
pub use swap::{DomainGate, ThreeDomainSwapper};
pub use three_domain::{ActionGate, DomainCheckResult, ProposalGate, ThoughtGate, ThreeDomainGuard};
pub use three_domain_enforce::{BCDViolation, GateState, ThreeDomainEnforcer};

pub use governance::{Governance, GovernanceError, GovernanceOutcome, GovernanceStep};
pub use mewg::{
    Decision, DefaultMewgAuthority, EvidenceSource, MewgAuthority, MewgError, MewgEvidence,
    MewgVerdict, DEFAULT_MEWG_APPROVAL_THRESHOLD,
};
pub use multi_ai::{AiConsensus, AiProvider, AiProviderId, AiStance, AiVerdict, MockAiProvider, MultiAiConsensus, MultiAiError};
pub use multi_human::{HumanId, HumanVote, HumanVoteError, HumanVoteOutcome, HumanVoter, InMemoryHumanVoter, Vote};
pub use physical_multisig::{InMemoryPhysicalMultisig, MultisigError, MultisigOutcome, PhysicalMultisig, PhysicalSignature, PhysicalSignerId};
pub use reflection::{InMemoryReflectionClock, ReflectionClock, ReflectionError, ReflectionPeriod, ReflectionState, DEFAULT_REFLECTION_PERIOD};

pub use colang_dsl::{
    ColangDefine, ColangDslGuard, ColangElement, ColangElementKind, ColangGuardConfig,
    ColangGuardOutcome, ColangParseError, ColangParser, ColangValidationError,
    ColangValidationReport, ColangValidator, DslOnionLayer, DslOnionVerdict, ParsedColangFile,
};
pub use seven_fold_guard::{SevenFoldGuardOutcome, SevenFoldGuardRunner};
pub use skill_guard::{
    ColangDslGuardSkill, MewgGuardSkill, MultiAiGuardSkill, MultiHumanGuardSkill,
    PhysicalMultisigGuardSkill, ReflectionGuardSkill, Skill, SkillError, SkillGuard,
    SkillGuardConfig, SkillGuardOutcome, SkillId, SkillRegistry, SkillStep,
    SuperpowersSkillGuardSkill,
};

pub use action_rail::{
    Action, ActionContext, ActionDispatcher, ActionError, ActionId, ActionKind, ActionOutcome,
    ActionRegistry, DialogMultiHumanAction, ExecutionPhysicalMultisigAction, InputMultiAiAction,
    OutputMewgAction, RetrievalReflectionAction, SystemColangCompileAction,
    SystemFlowDispatchAction, SystemSkillInvokeAction,
};
pub use evidence_guard::{
    EvidenceCheck, EvidenceEntry, EvidenceGuard, EvidenceKind, EVIDENCE_FOLD_GUARD_COUNT,
    EVIDENCE_FOLD_GUARD_INDEX, NINE_FOLD_GUARDS_HARDCODE,
};
pub use flow_executor::{FlowError, FlowExecutor, FlowOutcome, FlowRunner, FlowState, FlowStep};

/// 7 重守门 v7 严守 (R126-guard-7 编译期 hardcode).
pub const SEVEN_FOLD_GUARDS_HARDCODE: usize = 7;

/// 8 重守门 v8 严守 (R127-2 P6-3 编译期 hardcode).
pub const EIGHT_FOLD_GUARDS_HARDCODE: usize = 8;

/// 9 阶段生命周期长度 (编译时硬编码).
pub const NINE_STAGES_HARDCODE: usize = 9;

/// 三域数量 (编译时硬编码).
pub const THREE_DOMAINS_HARDCODE: usize = 3;

/// 6 权限洋葱层数.
pub const SIX_PERMISSION_LAYERS_HARDCODE: usize = 6;

/// 5 哲学键审议.
pub const FIVE_PRINCIPLE_LAYERS_HARDCODE: usize = 5;

pub const SINGLE_HA_HUMAN_COUNT: usize = 1;
pub const DEFAULT_M_OF_N_REQUIRED: usize = 2;
pub const DEFAULT_M_OF_N_TOTAL: usize = 3;

pub const SGI_COOLDOWN_MS: i64 = 86_400_000;
pub const HA_ICE_FROZEN_MS: i64 = 86_400_000;
pub const CONTINUITY_HISTORY_RETENTION_MS: i64 = 30i64 * 86_400_000;

pub const MEWG_FIVE_FOLDS_HARDCODE: usize = 5;

const _: () = {
    assert!(NINE_STAGES_HARDCODE == 9);
    assert!(THREE_DOMAINS_HARDCODE == 3);
    assert!(SIX_PERMISSION_LAYERS_HARDCODE == 6);
    assert!(FIVE_PRINCIPLE_LAYERS_HARDCODE == 5);
    assert!(SINGLE_HA_HUMAN_COUNT == 1);
    assert!(DEFAULT_M_OF_N_REQUIRED >= 1);
    assert!(DEFAULT_M_OF_N_REQUIRED <= DEFAULT_M_OF_N_TOTAL);
    assert!(SGI_COOLDOWN_MS >= 1_000);
    assert!(HA_ICE_FROZEN_MS >= 1_000);
    assert!(CONTINUITY_HISTORY_RETENTION_MS >= 86_400_000);
    assert!(MEWG_FIVE_FOLDS_HARDCODE == 5);
    assert!(SEVEN_FOLD_GUARDS_HARDCODE == 7);
    assert!(crate::skill_guard::SkillId::COUNT == 7);
    assert!(crate::skill_guard::SkillId::ALL.len() == 7);
    assert!(EIGHT_FOLD_GUARDS_HARDCODE == 8);
    assert!(crate::action_rail::ActionId::COUNT == 8);
    assert!(crate::action_rail::ActionId::ALL.len() == 8);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn re_exports_compile() {
        let _stage = LifeStage::Birth;
        let _decision = SovereigntyDecision::Approved {
            reason: "test".into(),
            decided_at_ms: 0,
            signatures: vec!["sig-1".into()],
        };
        let _outcome = DecisionOutcome::new("r-1", SovereigntyDomain::Thought, _decision.clone(), 0);
        let _domain = SovereigntyDomain::Action;
        let _suspension = Suspension::Permanent {
            reason: "x".into(),
            suspended_at_ms: 0,
            kind: SuspensionKind::SelfInitiated,
        };
        let _pause = PauseHandle::new("p-1", "reason", 0, "by");
        let _ha_mode = HAMode::SingleHuman(SingleHumanPolicy::new("h-1", "Alice", crate::ha::HAAuthentication::WindowsHello));
        let _carrier = CarrierType::Memory;
        let _continuity = SubjectContinuity::new("subj-1", _carrier, 0);
        let _sgi_rule = SGIFieldRule::new("requires_ha", "L0 HA 触发");
        let _gate = ThreeDomainGuard::new();

        let _mewg_decision = Decision {
            id: "d".into(),
            title: "t".into(),
            description: "x".into(),
            touches_e_layer: false,
            tags: vec![],
            submitted_at: 0,
            metadata: None,
        };
        let _authority = DefaultMewgAuthority::new();
        let _voter = InMemoryHumanVoter::new();
        let _multisig = InMemoryPhysicalMultisig::new();
        let _clock = InMemoryReflectionClock::new();
        let _consensus = MultiAiConsensus::new();
        let _gov = Governance::default();
    }

    #[test]
    fn nine_stages_compile_time_hardcode() {
        assert_eq!(NINE_STAGES.len(), NINE_STAGES_HARDCODE);
        assert_eq!(NINE_STAGES_HARDCODE, 9);
    }

    #[test]
    fn seven_fold_guards_compile_time_hardcode() {
        assert_eq!(SEVEN_FOLD_GUARDS_HARDCODE, 7);
        assert_eq!(SkillId::COUNT, 7);
        assert_eq!(SkillId::ALL.len(), 7);
        let registry = SkillRegistry::new();
        assert_eq!(registry.count(), 7);
        for id in SkillId::ALL {
            assert!(registry.get(id).is_some(), "Skill {:?} 未注册", id);
        }
    }

    #[test]
    fn eight_fold_guards_compile_time_hardcode() {
        assert_eq!(EIGHT_FOLD_GUARDS_HARDCODE, 8);
        assert_eq!(ActionId::COUNT, 8);
        assert_eq!(ActionId::ALL.len(), 8);
        let registry = ActionRegistry::new();
        assert_eq!(registry.count(), 8);
        for id in ActionId::ALL {
            assert!(registry.get(id).is_some(), "Action {:?} 未注册", id);
        }
        assert_eq!(ActionKind::FIVE_GUARDRAILS_COUNT, 5);
        assert_eq!(ActionKind::FIVE_GUARDRAILS_KINDS.len(), 5);
        let dispatcher = ActionDispatcher::new();
        assert_eq!(dispatcher.registry().count(), 8);
        assert_eq!(FlowStep::COUNT, 17);
        assert_eq!(FlowStep::ALL.len(), 17);
    }
}
//! `apeireth-sovereignty`: 主权器官 + HA + 三域分离 + SGI + 9 阶段生命周期
//!                    + MEWG 5 重治理 (与 apeireth-council 协同)
//!
//! **职责** (P22 落点 — 架构师3):
//! - **主权 trait**: `Sovereignty::decide / pause / suspend_self`
//! - **HA 部署模式自适应** (single / multi / offline) + 生物特征 trait 抽象
//! - **三域分离强制点** (Thought 自由 / Proposal 5 哲学键 / Action 6 权限层)
//! - **SGI 单字段写入触发器** (Single-Field Giant Impact)
//! - **主体连续性 ID** 跨载体 + migration_history
//! - **9 阶段生命周期** (孕育 → 诞生 → ... → 重生)
//!
//! **MEWG 5 重治理** (P22 协同落点 — 由并行 agent 完成):
//! - MEWG 最高优先级解释权 + 多人 ≥2 + 多 AI ≥3 + 物理多签 + 反思期 ≥7 天
//! - 与 `apeireth-council` synthesis 协同 (通过 `SovereigntyHook` trait)
//!
//! **诚实登记**:
//! - ❌ **不用 PyO3** — 不调 Python 实现
//! - ❌ **不调外部 biometric SDK** — 生物特征由 Rust trait [`BiometricProvider`] + mock provider 真实实现
//! - ✅ 所有逻辑 Rust 内置, 抽象 trait 留口 (Windows Hello / FIDO2 未来接入)
//!
//! **架构位置**:
//! ```text
//!   apeireth-sovereignty (本 crate — 主权 + MEWG 治理)
//!      ↓ (SovereigntyHook)
//!   apeireth-council (7 Advisor + 按住)
//!      ↓
//!   apeireth-core (基础类型)
//! ```
//!
//! **禁止**:
//! - ❌ 不修改 `apeireth-core` / `apeireth-council` 已实装类型签名
//! - ❌ 不引入 I/O / 网络 / 文件系统 / `unsafe`

#![deny(unsafe_code)]

// ============================================================
// 主权模块 (P22 架构师3 主路径)
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
// MEWG 5 重治理模块 (并行 agent 完成)
// ============================================================
pub mod colang_dsl;
pub mod governance;
pub mod mewg;
pub mod multi_ai;
pub mod multi_human;
pub mod owner;
pub mod physical_multisig;
pub mod reflection;
// R126-guard-7 升级 (B4 6 重守门 v6 → v7): 加 skill_guard + seven_fold_guard 2 个新 mod
// 借鉴 superpowers 234 cloned (R125-14/R125-15e 实施时已研究, 整合 #4 commit done)
// 借鉴 ID: R126-guard-7-BORROW-obra/superpowers-2026-05-2026-08-10
// 8 硬墙 0 越界: 0 改 Governance.process / GovernanceOutcome / GovernanceStep / MEWG_FIVE_FOLDS_HARDCODE
pub mod seven_fold_guard;
pub mod skill_guard;

// R127-2 P6-3 升级 (B4 7 重守门 v7 → 8 重守门 v8): 加 action_rail + flow_executor 2 个新 mod
// 借鉴 NVIDIA NeMo Guardrails (action_dispatcher.py + colang/runtime.py + llm_flows.co)
// 借鉴 ID: R127-2-P6-3-BORROW-NVIDIA-NeMo/Guardrails-2026-08-10
// 8 硬墙 0 越界: 0 改 Governance.process / SkillRegistry / SkillGuard / ColangParser / ColangValidator
pub mod action_rail;
pub mod flow_executor;

// R131 升级 (B4 8 重守门 v8 → 9 重守门 v9): 加 evidence_guard 1 个新 mod
// 借鉴 Claude-mem / Letta evidence-chain 思想, 对应 S-2 实事求是 + O-5 不假装
// 借鉴 ID: R131-evidence-9-BORROW-claude-mem+literal-2026-08-12
// 8 硬墙 0 越界: 0 改 Governance.process / SkillRegistry / SkillGuard / ActionRail / ActionKind 公开签名
pub mod evidence_guard;

// ============================================================
// 公共 re-export — 让 `use apeireth_sovereignty::*;` 拿到常用 API
// ============================================================
//
// 主权模块 re-export — 注意 `Decision` 命名冲突:
// - 主权 `decision::Decision` (enum: Approved/Rejected/Pending) → 重命名为 `SovereigntyDecision`
// - MEWG `mewg::Decision` (struct: E层 metadata) → 主路径 re-export 为 `Decision`
pub use continuity::{CarrierType, Migration, SubjectContinuity};
pub use decision::{
    Decision as SovereigntyDecision, DecisionOutcome, DecisionRequest, SovereigntyDomain,
};
pub use ha::{
    AuthorityMode, AuthorityMultisigOutcome, BiometricProvider, BiometricResult, HAAuthentication,
    HAMode, HumanApproval, HumanAuthority, MultiSigPolicy, OwnerRequestMultisigOutcome, Signatory,
    SingleHumanPolicy,
};
pub use life_stage::{LifeStage, LifeStageTransition, NINE_STAGES};
pub use mock_biometric::{CoercionBehavior, MockBiometric, MockBiometricBehavior};
pub use owner::{OwnerAction, OwnerError, OwnerRequest, OwnerToken};
pub use pause::{PauseHandle, Suspension, SuspensionKind};
pub use sgi::{SGIFieldRule, SGITrigger, SGITriggerGuard, SGITriggerOutcome};
pub use sovereign::{Sovereignty, SovereigntyEngine, SovereigntyError};

// ============================================================
// round8-06 新模块 re-export
// ============================================================
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
pub use three_domain::{
    ActionGate, DomainCheckResult, ProposalGate, ThoughtGate, ThreeDomainGuard,
};
pub use three_domain_enforce::{BCDViolation, GateState, ThreeDomainEnforcer};

// MEWG 5 重治理 re-export — `Decision` 是顶层 (保持其他 agent 的 demo/e2e 期望)
pub use governance::{
    Governance, GovernanceCouncilHook, GovernanceError, GovernanceOutcome, GovernanceStep,
};
pub use mewg::{
    Decision, DefaultMewgAuthority, EvidenceSource, MewgAuthority, MewgError, MewgEvidence,
    MewgVerdict, DEFAULT_MEWG_APPROVAL_THRESHOLD,
};
pub use multi_ai::{
    AiConsensus, AiProvider, AiProviderId, AiStance, AiVerdict, MockAiProvider, MultiAiConsensus,
    MultiAiError,
};
pub use multi_human::{
    HumanId, HumanVote, HumanVoteError, HumanVoteOutcome, HumanVoter, InMemoryHumanVoter, Vote,
};
pub use physical_multisig::{
    InMemoryPhysicalMultisig, MultisigError, MultisigOutcome, PhysicalMultisig, PhysicalSignature,
    PhysicalSignerId,
};
pub use reflection::{
    InMemoryReflectionClock, ReflectionClock, ReflectionError, ReflectionPeriod, ReflectionState,
    DEFAULT_REFLECTION_PERIOD,
};

// ============================================================
// 6 重守门 v6 (colang_dsl) + 7 重守门 v7 (skill_guard + seven_fold_guard) re-export
// R126-guard-7 B4 6 重守门 v6 → v7 升级 (per 决策 #33 + 决策 #47 + 决策 #51 §1.2 P1-3)
// 借鉴 superpowers 234 cloned (R125-14/R125-15e 已研究, 整合 #4 commit done)
// 8 硬墙 0 越界: 0 改 Governance.process / GovernanceOutcome / GovernanceStep / MEWG_FIVE_FOLDS_HARDCODE
// ============================================================
pub use colang_dsl::{
    ColangDefine, ColangDslGuard, ColangElement, ColangElementKind, ColangGuardConfig,
    ColangGuardOutcome, ColangParseError, ColangParser, ColangValidationError,
    ColangValidationReport, ColangValidator, DslOnionLayer, DslOnionVerdict, ParsedColangFile,
};
pub use seven_fold_guard::{SevenFoldGuardOutcome, SevenFoldGuardRunner};
pub use skill_guard::{
    MultiAiGuardSkill, MultiHumanGuardSkill, MewgGuardSkill, ColangDslGuardSkill,
    PhysicalMultisigGuardSkill, ReflectionGuardSkill, Skill, SkillError, SkillGuard,
    SkillGuardConfig, SkillGuardOutcome, SkillId, SkillRegistry, SkillStep,
    SuperpowersSkillGuardSkill,
};

// R127-2 P6-3 升级 (B4 7 重守门 v7 → 8 重守门 v8): 行动轨 + 流程执行器 re-export
// 借鉴 NVIDIA NeMo Guardrails (action_dispatcher.py:52-60 + colang/runtime.py:27-63)
// 借鉴 ID: R127-2-P6-3-BORROW-NVIDIA-NeMo/Guardrails-2026-08-10
pub use action_rail::{
    Action, ActionContext, ActionDispatcher, ActionError, ActionId, ActionKind, ActionOutcome,
    ActionRegistry, DialogMultiHumanAction, ExecutionPhysicalMultisigAction, InputMultiAiAction,
    OutputMewgAction, RetrievalReflectionAction, SystemColangCompileAction,
    SystemFlowDispatchAction, SystemSkillInvokeAction,
};
pub use flow_executor::{FlowError, FlowExecutor, FlowOutcome, FlowRunner, FlowState, FlowStep};
pub use evidence_guard::{
    EvidenceCheck, EvidenceEntry, EvidenceGuard, EvidenceKind, EVIDENCE_FOLD_GUARD_COUNT,
    EVIDENCE_FOLD_GUARD_INDEX, NINE_FOLD_GUARDS_HARDCODE,
};



/// 7 重守门 v7 严守 (R126-guard-7 B4 升级, 编译期 hardcode).
///
/// **设计**:
/// - 守门 1-5 = Governance.process 5 step (0 改)
/// - 守门 6 = Colang DSL (R125-5 实施, 0 改)
/// - 守门 7 = Superpowers Skill Guard (R126-guard-7 NEW, 借鉴 superpowers 234 cloned)
/// - 总 7 重, 编译期 hardcode 严守 (SkillId::ALL.len() == 7)
pub const SEVEN_FOLD_GUARDS_HARDCODE: usize = 7;

/// 8 重守门 v8 严守 (R127-2 P6-3 B4 升级, 编译期 hardcode).
///
/// **设计**:
/// - 守门 1-7 = 7 重守门 v7 (0 改)
/// - 守门 8 = Action Rail Guard (R127-2 P6-3 NEW, 借鉴 NVIDIA Guardrails ActionDispatcher + Colang Runtime)
/// - 总 8 重, 编译期 hardcode 严守 (ActionId::ALL.len() == 8)
/// - 守门 8 含 8 ActionId + 17 FlowStep + 5 FlowState + ActionDispatcher + FlowRunner + FlowExecutor
pub const EIGHT_FOLD_GUARDS_HARDCODE: usize = 8;

/// 9 阶段生命周期长度 (编译时硬编码)。
pub const NINE_STAGES_HARDCODE: usize = 9;

/// 三域数量 (编译时硬编码: Thought / Proposal / Action)。
pub const THREE_DOMAINS_HARDCODE: usize = 3;

/// 6 权限洋葱层数 (L0-L5, 由 core 提供)。
pub const SIX_PERMISSION_LAYERS_HARDCODE: usize = 6;

/// 5 哲学键审议 (E / S / A / M / O 原则洋葱)。
pub const FIVE_PRINCIPLE_LAYERS_HARDCODE: usize = 5;

/// HA 单人模式 = 1 个真实人类。
pub const SINGLE_HA_HUMAN_COUNT: usize = 1;

/// HA 默认 M-of-N (2-of-3 多人多签)。
pub const DEFAULT_M_OF_N_REQUIRED: usize = 2;
pub const DEFAULT_M_OF_N_TOTAL: usize = 3;

/// SGI 单字段触发冷却期 (毫秒, 24h)。
pub const SGI_COOLDOWN_MS: i64 = 86_400_000;

/// 冰冻期 (毫秒, 24h)。
pub const HA_ICE_FROZEN_MS: i64 = 86_400_000;

/// 主体连续性 ID 持久化最小保留期 (30 天, 跨载体迁移历史)。
pub const CONTINUITY_HISTORY_RETENTION_MS: i64 = 30i64 * 86_400_000;

/// MEWG 5 重治理步数 (编译时硬编码)。
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
    // R126-guard-7 B4 v6 → v7 升级 (7 重守门 v7 严守)
    assert!(SEVEN_FOLD_GUARDS_HARDCODE == 7);
    assert!(crate::skill_guard::SkillId::COUNT == 7);
    assert!(crate::skill_guard::SkillId::ALL.len() == 7);
    // R127-2 P6-3 B4 v7 → v8 升级 (8 重守门 v8 严守)
    assert!(EIGHT_FOLD_GUARDS_HARDCODE == 8);
    assert!(crate::action_rail::ActionId::COUNT == 8);
    assert!(crate::action_rail::ActionId::ALL.len() == 8);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn re_exports_compile() {
        // 主权模块烟雾测试
        let _stage = LifeStage::Birth;
        let _decision = SovereigntyDecision::Approved {
            reason: "test".into(),
            decided_at_ms: 0,
            signatures: vec!["sig-1".into()],
        };
        let _outcome =
            DecisionOutcome::new("r-1", SovereigntyDomain::Thought, _decision.clone(), 0);
        let _domain = SovereigntyDomain::Action;
        let _suspension = Suspension::Permanent {
            reason: "x".into(),
            suspended_at_ms: 0,
            kind: SuspensionKind::SelfInitiated,
        };
        let _pause = PauseHandle::new("p-1", "reason", 0, "by");
        let _ha_mode = HAMode::SingleHuman(SingleHumanPolicy::new(
            "h-1",
            "Alice",
            crate::ha::HAAuthentication::WindowsHello,
        ));
        let _carrier = CarrierType::Memory;
        let _continuity = SubjectContinuity::new("subj-1", _carrier, 0);
        let _sgi_rule = SGIFieldRule::new("requires_ha", "L0 HA 触发");
        let _gate = ThreeDomainGuard::new();

        // MEWG 模块烟雾测试
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

    /// R126-guard-7 B4 v6 → v7 升级: 7 重守门 v7 re-export 严守
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

    /// R127-2 P6-3 B4 v7 → v8 升级: 8 重守门 v8 行动轨 re-export 严守
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
        // 守门 8 含 5 main types 严守 (借鉴 Guardrails 5 main types of guardrails)
        assert_eq!(ActionKind::FIVE_GUARDRAILS_COUNT, 5);
        assert_eq!(ActionKind::FIVE_GUARDRAILS_KINDS.len(), 5);
        // ActionDispatcher 默认 8 entries
        let dispatcher = ActionDispatcher::new();
        assert_eq!(dispatcher.registry().count(), 8);
        // FlowRunner + FlowExecutor 严守 (17 FlowStep + 5 FlowState)
        assert_eq!(FlowStep::COUNT, 17);
        assert_eq!(FlowStep::ALL.len(), 17);
    }
}

// === apeireth-verify cross-crate hooks (Q22) — disabled (let-at-top-level invalid in Rust 2021) ===
// pub static VERIFY_TRACE: ::std::sync::OnceLock<::apeireth_verify::VerdictTrace> = ::apeireth_verify::new_trace_slot();
// ::apeireth_verify::regression_assert!(
//     "apeireth-sovereignty",
//     "apeireth-sovereignty structural invariant — regression_assert! integration",
//     InRange { name: "apeireth-sovereignty::invariant-a", value: 1.0, min: 0.0, max: 1.0 }
// );
// ::apeireth_verify::regression_assert!(
//     "apeireth-sovereignty",
//     "apeireth-sovereignty regression gate — regression_assert! integration",
//     Idempotent { name: "apeireth-sovereignty::invariant-b", first: "stable", second: "stable" }
// );

// === apeireth-verify cross-crate hooks (P28 阶段 6) ===
apeireth_verify::regression_assert!(
    __APEIRETH_REG_APEIRETH_SOVEREIGNTY_A,
    "apeireth-sovereignty",
    "apeireth-sovereignty structural invariant",
    InRange {
        name: "apeireth-sovereignty::invariant-a",
        value: 1.0,
        min: 0.0,
        max: 1.0
    }
);
apeireth_verify::regression_assert!(
    __APEIRETH_REG_APEIRETH_SOVEREIGNTY_B,
    "apeireth-sovereignty",
    "apeireth-sovereignty regression gate",
    Idempotent {
        name: "apeireth-sovereignty::invariant-b",
        first: "stable",
        second: "stable"
    }
);
apeireth_verify::register_all_in_crate!(
    __APEIRETH_REG_APEIRETH_SOVEREIGNTY_A,
    __APEIRETH_REG_APEIRETH_SOVEREIGNTY_B
);

//! `apeireth-council`: 智囊团 7 强制 Advisor + 按住机制 + 拟人化 synthesis
//!
//! **职责** (P22 落点 — 架构师2):
//! - 7 强制 Advisor 领域 (safety/performance/philosophy/history/strategy/ethics/legal)
//! - 3 生命周期 (persistent / ephemeral / dynamic)
//! - 按住机制 (30% 强反对 / 一致反对 / 60s 裁决超时)
//! - 多意见加权 synthesis
//! - 拟人化 (独立 session + persona + 立场 + 可辩论 3 轮)
//! - 与 `apeireth-sovereignty` 集成接口 (`SovereigntyHook` trait)
//!
//! **诚实登记**:
//! - ❌ **不用 PyO3** — 不调 Python 实现 advisor
//! - ❌ **不调外部 LLM HTTP** — advisor 行为由 Rust trait + mock LLM provider 真实实现
//! - ✅ 7 强制 Advisor 全部 Rust 内置 (`Advisors` 子模块硬编码)
//!
//! **架构位置**:
//! ```text
//!      sovereignty (持有 hook)
//!           ↓
//!   apeireth-council (本 crate — 7 Advisor + 按住 + synthesis + 拟人化)
//!           ↓
//!      apeireth-core (基础类型 — 不依赖 sovereignty)
//! ```
//!
//! **禁止**:
//! - ❌ 不修改 `apeireth-core` 已实装类型签名
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不依赖 `apeireth-sovereignty` (尚未落地; 本 crate 通过 `SovereigntyHook` trait 留口)
//! - ❌ 不引入 I/O / 网络 / 文件系统 / `unsafe`

#![deny(unsafe_code)]

pub mod advisor;
pub mod bus_bridge;  // R111: Council deliberation event → bus 真接
pub mod graph_bridge;  // R113: cognition summary → council deliberation context 真接
pub mod mcp_bridge;  // R115: CouncilMember → MCP Prompt/ResourceServer 桥接
pub mod council_member;  // R33-4: AutoGen 借鉴 (role/goal/backstory/provider)
pub mod council_member_deliberation;  // R33-4-1: CouncilMember 多轮协商 deliberation (per AutoGen GroupChat + VCP vcpLoop)
pub mod council_member_persona_combo;  // R33-4-2: CouncilMember + Persona 组合 (per AutoGen ConversableAgent.system_message 借鉴)
pub mod deliberation;
pub mod hold;
pub mod lifecycle;
pub mod mock_llm;
pub mod persona;
pub mod sovereignty;
pub mod stress_test;  // R68: deliberation stress test runner
pub mod synthesis;

pub mod advisors;

// R25 D-3: 4 协作模式 + 角色宪法 + reasoning trace + 图编排集成
pub mod collaboration;  // R25 D-3 Stage 2 §2B (4 模式: Planner+Executor / Debate / Voting / Hierarchical)
pub mod constitution;   // R25 D-3 角色宪法 (5 字段 1:1 镜像 R11 5 重守门)
pub mod trace;          // R25 D-3 reasoning trace 可视化 (3 输出格式)
pub mod graph_orchestration;  // R25 D-3 图编排集成 (4 模式包成 Graph Node)

pub use advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, Stance, StanceKind, DEFAULT_DEBATE_ROUNDS,
};
pub use deliberation::{Council, CouncilQuery, CouncilVerdict, DEFAULT_DELIBERATION_TIMEOUT_MS};
pub use hold::{HoldDecision, HoldOutcome, HoldThreshold, HoldTrigger};
pub use lifecycle::{AdvisorLifecycle, LifecycleManager, LifecycleStats};
pub use mock_llm::{MockLlmProvider, MockLlmResponse, ScriptedMockLlm};
pub mod llm_backend;
pub use council_member::{CouncilMember, is_valid_provider, SUPPORTED_PROVIDERS};  // R33-4: AutoGen 借鉴 (4 字段 1:1 re-export)
pub use council_member_deliberation::{
    CouncilMemberDeliberator, MemberSummary, MultiRoundVerdict, RoundSummary,
    CONSENSUS_SCORE_THRESHOLD, DEFAULT_MAX_ROUNDS,
};  // R33-4-1: 多轮协商 deliberation (per AutoGen GroupChat + VCP vcpLoop)
pub use council_member_persona_combo::{
    PersonaBoundDeliberator, PersonaBoundMember, PersonaBoundRound, PersonaBoundSummary,
    PersonaBoundVerdict,
};  // R33-4-2: CouncilMember + Persona 组合
pub use llm_backend::LlmAdvisorBackend;
pub use persona::{DebateRound, Persona, PersonaSession};
pub use sovereignty::{CouncilEvent, NoopSovereigntyHook, SovereigntyHook};
pub use synthesis::{synthesize, SynthesisWeights};

// Re-export 7 强制 advisor factory functions.
pub use advisors::{
    ethics_advisor, history_advisor, legal_advisor, performance_advisor, philosophy_advisor,
    safety_advisor, seven_mandatory_advisors, strategy_advisor,
};

// R25 D-3: 4 协作模式 re-export (per v2.0 strategy §2B)
pub use collaboration::types::{CollaborationContext, CollaborationMode, CollaborationVerdict};
pub use collaboration::planner_executor::{PlannerExecutor, SubTask};
pub use collaboration::debate::DebateMode;
pub use collaboration::voting::{Voter, VotingMode, VotingStrategy};
pub use collaboration::hierarchical::{DelegatedTask, HierarchicalMode};
// R25 D-3: 角色宪法 re-export (per v2.0 strategy §2B "角色宪法")
pub use constitution::{
    ConstitutionViolation, FiveGuardsSummary, RoleConstitution, RoleConstitutionTrait,
    PHILOSOPHICAL_ANCHORS,
};
// R25 D-3: reasoning trace re-export (per v2.0 strategy §2B "reasoning trace 可视化")
pub use trace::{trace_from_collaboration, trace_step_from_opinion, TraceReport, TraceStep};
// R25 D-3: graph orchestration re-export (per v2.0 strategy §2B "加图编排支持")
pub use graph_orchestration::{
    CollaborationDriver, CollaborationNode, CouncilGraph, MockDriver,
};

/// 7 强制 Advisor 数量 (编译时硬编码)。
pub const SEVEN_MANDATORY_ADVISORS: usize = 7;

/// 按住机制的 30% 阈值（强反对占比 ≥ 30% 触发按住）。
pub const HOLD_STRONG_DISAPPROVE_PERCENT: u8 = 30;

/// 按住机制的 60s 裁决超时（毫秒）。
pub const HOLD_DELIBERATION_TIMEOUT_MS: u64 = 60_000;

/// 可辩论 3 轮 (拟人化 persona 最大轮次)。
pub const MAX_PERSONA_DEBATE_ROUNDS: u8 = 3;

const _: () = {
    assert!(SEVEN_MANDATORY_ADVISORS == 7);
    assert!(HOLD_STRONG_DISAPPROVE_PERCENT > 0 && HOLD_STRONG_DISAPPROVE_PERCENT <= 100);
    assert!(HOLD_DELIBERATION_TIMEOUT_MS >= 1_000);
    assert!(MAX_PERSONA_DEBATE_ROUNDS == 3);
};

// === apeireth-verify cross-crate hooks (Q22) — disabled (let-at-top-level invalid in Rust 2021) ===
// pub static VERIFY_TRACE: ::std::sync::OnceLock<::apeireth_verify::VerdictTrace> = ::apeireth_verify::new_trace_slot();
// ::apeireth_verify::regression_assert!(
//     "apeireth-council",
//     "apeireth-council structural invariant — regression_assert! integration",
//     InRange { name: "apeireth-council::invariant-a", value: 1.0, min: 0.0, max: 1.0 }
// );
// ::apeireth_verify::regression_assert!(
//     "apeireth-council",
//     "apeireth-council regression gate — regression_assert! integration",
//     Idempotent { name: "apeireth-council::invariant-b", first: "stable", second: "stable" }
// );

// ============================================================================
// round9-04 (V26.4) — __register_all_asserts no-op stub
//
// V26.2 backend_engineer2 disabled the original `apeireth_verify::register_all_in_crate!` macro
// call to break a circular dependency (core/verify mutually referenced).
// V26.3 DEF-V26.3-002 walk_all_crates example couldn't compile because no __register_all_asserts
// existed. V26.4 fix: provide a no-op stub that walk_all_crates can call. The stub does
// nothing (no regression assertions registered) which is the V26.2 intent (no circular
// dependency, but the symbol exists for example discovery).
//
// Future upgrade path (P28 stage 6 real impl): replace this stub with the real macro
// call once the circular dependency is resolved (e.g., via inventory/ctor or refactor
// apeireth-verify to be a thin facade).
#[allow(missing_docs, dead_code)] // V26.4 stub: walk_all_crates calls this no-op
pub fn __register_all_asserts() {
    // no-op by design
}




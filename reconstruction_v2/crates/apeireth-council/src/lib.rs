//! `apeireth-council`: 智囊团 7 强制 Advisor + 按住机制 + 拟人化 synthesis
//!
//! **职责** (P22 落点 — 架构师2):
//! - 7 强制 Advisor 领域 (safety/performance/philosophy/history/strategy/ethics/legal)
//! - 3 生命周期 (persistent / ephemeral / dynamic)
//! - 按住机制 (30% 强反对 / 一致反对 / 60s 裁决超时)
//! - 多意见加权 synthesis
//! - 拟人化 (独立 session + persona + 立场 + 可辩论 3 轮)
//! - 与 `apeireth-governance` 集成接口 (`SovereigntyHook` trait)
//!
//! **v2 适配**:
//! - v1 依赖 `apeireth-api` / `apeireth-graph` / `apeireth-mcp` / `apeireth-pipeline-g5` (v1 独有 crate).
//!   v2 没有这 4 个 crate. 本 crate 在内部定义等价 runtime:
//! - `graph_runtime` — Node/Graph/State/Edge 拓扑排序执行 (替代 apeireth-graph)
//! - `mcp_runtime` — Prompt/Resource/JsonRpc (替代 apeireth-mcp)
//! - `pipeline_runtime` — Pipeline<T,I,O> + Stage<I,O> 5 阶段 (替代 apeireth-pipeline-g5)
//! - LLM 路径走 `apeireth-asi::llm_judge` 本地等价 trait (替代 apeireth-api::llm)
//!
//! **诚实登记**:
//! - ❌ **不用 PyO3** — 不调 Python 实现 advisor
//! - ❌ **不调外部 LLM HTTP** — advisor 行为由 Rust trait + mock LLM provider 真实实现
//! - ✅ 7 强制 Advisor 全部 Rust 内置 (`Advisors` 子模块硬编码)
//!
//! **架构位置**:
//! ```text
//!      apeireth-governance (持有 hook)
//!           ↓
//!   apeireth-council (本 crate — 7 Advisor + 按住 + synthesis + 拟人化)
//!           ↓
//!      apeireth-core (基础类型 — 不依赖 governance)
//! ```
//!
//! **禁止**:
//! - ❌ 不修改 `apeireth-core` 已实装类型签名
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不依赖外部 LLM HTTP
//! - ❌ 不引入 I/O / 网络 / 文件系统 / `unsafe`

#![deny(unsafe_code)]

// ---- Local runtime modules (替代 v1 4 个外部 crate) ----
pub mod graph_runtime;
pub mod mcp_runtime;
pub mod pipeline_runtime;

// ---- 27 个核心模块 (verbatim from v1, 部分做了 v2 适配标注) ----
pub mod advisor;
mod organ_kani_proofs;
pub mod bus_bridge;
pub mod checkpoint;
pub mod checkpoint_integration;
pub mod council_member;
pub mod council_member_deliberation;
pub mod council_member_persona_combo;
pub mod delegation_matrix;
pub mod deliberation;
pub mod graph_bridge;          // v2 适配: 本地 CognitionSummary 替代 apeireth_graph
pub mod graph_orchestration;    // v2 适配: 用 graph_runtime 替代 apeireth_graph
pub mod hold;
pub mod lifecycle;
pub mod mcp_bridge;             // v2 适配: 用 mcp_runtime 替代 apeireth_mcp
pub mod mock_llm;
pub mod persona;
pub mod session_capture;
pub mod sovereign;
pub mod sovereignty;
pub mod stress_test;
pub mod synthesis;

pub mod advisors;

pub mod collaboration;
pub mod constitution;
pub mod g5_council_bridge;      // v2 适配: 用 pipeline_runtime 替代 apeireth_pipeline_g5
pub mod group_chat;
pub mod trace;
pub mod llm_backend;            // v2 适配: 用 apeireth_asi::llm_judge 替代 apeireth_api::llm
pub mod multi_model_backend;    // v2 适配: 同上

pub use advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, Stance, StanceKind, DEFAULT_DEBATE_ROUNDS,
};
pub use deliberation::{
    Council, CouncilQuery, CouncilVerdict, DeliberationStreamEvent, DEFAULT_DELIBERATION_TIMEOUT_MS,
};
pub use hold::{HoldDecision, HoldOutcome, HoldThreshold, HoldTrigger};
pub use lifecycle::{AdvisorLifecycle, LifecycleManager, LifecycleStats};
pub use mock_llm::{MockLlmProvider, MockLlmResponse, ScriptedMockLlm};
pub use council_member::{is_valid_provider, CouncilMember, SUPPORTED_PROVIDERS};
pub use council_member_deliberation::{
    CouncilMemberDeliberator, MemberSummary, MultiRoundVerdict, RoundSummary,
    CONSENSUS_SCORE_THRESHOLD, DEFAULT_MAX_ROUNDS,
};
pub use council_member_persona_combo::{
    PersonaBoundDeliberator, PersonaBoundMember, PersonaBoundRound, PersonaBoundSummary,
    PersonaBoundVerdict,
};
pub use llm_backend::LlmAdvisorBackend;
pub use persona::{DebateRound, Persona, PersonaSession};
pub use sovereignty::{CouncilEvent, NoopSovereigntyHook, SovereigntyHook};
pub use synthesis::{synthesize, SynthesisWeights};

pub use advisors::{
    ethics_advisor, history_advisor, legal_advisor, performance_advisor, philosophy_advisor,
    safety_advisor, seven_mandatory_advisors, strategy_advisor,
};

pub use collaboration::debate::DebateMode;
pub use collaboration::hierarchical::{DelegatedTask, HierarchicalMode};
pub use collaboration::planner_executor::{PlannerExecutor, SubTask};
pub use collaboration::types::{CollaborationContext, CollaborationMode, CollaborationVerdict};
pub use collaboration::voting::{Voter, VotingMode, VotingStrategy};
pub use constitution::{
    ConstitutionViolation, FiveGuardsSummary, RoleConstitution, RoleConstitutionTrait,
    PHILOSOPHICAL_ANCHORS,
};
pub use trace::{trace_from_collaboration, trace_step_from_opinion, TraceReport, TraceStep};
pub use graph_orchestration::{CollaborationDriver, CollaborationNode, CouncilGraph, MockDriver};

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

// ============ __register_all_asserts no-op stub (V26.4 兼容) ============
#[allow(missing_docs, dead_code)]
pub fn __register_all_asserts() {
    // no-op by design
}
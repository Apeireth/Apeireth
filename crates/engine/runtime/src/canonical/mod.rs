//! The canonical Apeireth runtime.
//!
//! # Ownership
//!
//! This module owns **orchestration**: sessions, provider routing, the agent
//! loop, behavior/capability registries, approvals, and abstract ports. It owns
//! no production assembly or concrete capability implementations, no
//! translation (that is `apeireth-protocol`), and no policy (that is
//! `apeireth-governance`).
//!
//! # Layering
//!
//! ```text
//!   apeireth-core        primitives
//!         ^
//!   apeireth-protocol    translation
//!         ^
//!   apeireth-plugin      capabilities        apeireth-governance -> core
//!         ^                                          ^
//!   apeireth-runtime::canonical  ------------------- '
//! ```
//!
//! Nothing below may depend on anything above it.
//!
//! The crate boundary is intentionally the same as this canonical module:
//! there is no second orchestration driver or legacy runtime dependency.

pub mod approval;
pub mod capability;
// B6 · Phase 5 (research, 默认关闭): 审批状态机形式化 (RA-5 规格, 模型级验证).
pub mod research_approval_sm;
pub use research_approval_sm::{
    research_allowed_recovery, research_run_fault_injection, ResearchApprovalError,
    ResearchApprovalEvent, ResearchApprovalMachine, ResearchApprovalRecord, ResearchApprovalStatus,
    ResearchCompensation, ResearchFaultInjectionReport, ResearchIdempotency,
    ResearchRecoveryAction, ResearchRecoveryAdvice, ResearchSideEffectCategory,
    ResearchSideEffectDescriptor,
};
pub mod error;
pub mod events;
pub mod execute;
pub mod heartbeat;
pub mod module;
// OrganOrchestrator 类似 v1 AwakeCompanion 真实施 (per R11 spec + 子代理 R12 真实施).
// **0 装诚实真账 (子代理 R12 独立判断)**:
// - 本 module 是 spec 部分真实施 (估 30-45 分钟), 完整 1-3 周估待 (per R11 §8.4).
// - 9 organ process 串联 + 8 重 gate 真实存在 + 5 状态机 forward-declared + L0-L5 骨架.
// - 0 触碰 cognitive.rs 12 slot (LOCKED, 子代理 K 核验).
// - 0 引新外部 dep (Cargo.lock 0 行 diff).
// Bridge from the plugin `LlmFactory` contract onto the runtime-owned
// `ModuleInvoker`, so organ LLM calls (W1/W2) can ride the canonical
// completion-governed path once organ wiring lands. Adapter only: it holds
// no provider, budget, or governance authority of its own.
// The ONE canonical organ-ownership module (AfterTurn post-turn cognition).
// Long-lived: 7 deterministic organs + OrganOrchestrator backend; transient
// per invocation: W1/W2 built from ctx.invoker_handle() and dropped.
pub mod provider;
pub mod runtime;
pub mod session;
pub mod subloop;
pub mod trace;
// L0-L5 自升级 cycle driver (Stage 5 完整化, per R11 §7 + v2-architecture-reflection.md §6).
// **0 装诚实**: L0/L2/L3/L4 真接 governance + Orchestrator; L5 建议模式不自动跑 git tag;
// L1 接 SelfAssessmentStore. 主人 Veto dashboard 留 v2.0.0 release 接入.

pub use apeireth_governance::TurnSecurityContext;
pub use approval::{
    operation_fingerprint, operation_fingerprint_with_invocation, ApprovalDecision, ApprovalStatus,
    PendingApproval, PendingApprovalView,
};
pub use capability::{CapabilityProvider, CapabilityRegistry};
pub use error::{RuntimeError, RuntimeResult};
pub use events::{
    event_sink, CompositeEventSink, CompositeRuntimeEventSink, NoopRuntimeEventSink, RuntimeEvent,
    RuntimeEventSink,
};
pub use execute::{ApprovalResolution, TurnOutcome, TurnRequest, TurnResponse};
pub use heartbeat::{FlowLock, HeartbeatScheduler, HeartbeatTask, HeartbeatTriggerSource};
pub use module::{
    AgentModule, BehaviorModule, HookPoint, InvocationContext, InvocationOrigin, Module,
    ModuleContext, ModuleDirective, ModuleError, ModuleInvocationError, ModuleInvocationRequest,
    ModuleInvocationResponse, ModuleInvoker, ModuleManifest, ModuleOutcome, ModuleRegistry,
    PromptOverlay, DEFAULT_MAX_INVOCATION_DEPTH, DEFAULT_MAX_MODULE_INVOCATIONS,
};
pub use provider::{ProviderHealth, ProviderRouter, RoutedCompletion};
pub use runtime::{
    plugin_ids, Runtime, RuntimeBuilder, RuntimeCapabilitySnapshot, RuntimeConfig,
    RuntimeHealthSnapshot, RuntimeModelSnapshot, RuntimeModuleSnapshot, RuntimeProviderSnapshot,
    RuntimeSnapshot, DEFAULT_MAX_ROUNDS,
};
pub use session::{
    InMemorySessionStore, Session, SessionEvent, SessionEventKind, SessionManager, SessionStore,
};
pub use subloop::{
    RuntimeSubLoopSpawner, SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
pub use trace::{ExecutionTrace, TraceEntry, TraceEvent};

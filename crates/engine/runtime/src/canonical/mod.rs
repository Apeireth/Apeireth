//! The canonical Apeireth runtime.
//!
//! # Ownership
//!
//! This module owns **orchestration**: sessions, provider routing, the agent
//! loop, and the composition root that holds them. It owns no translation (that
//! is `apeireth-protocol`), no capability implementations (those are plugins),
//! and no policy (that is `apeireth-governance`).
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
pub mod cognitive;
pub mod error;
pub mod execute;
pub mod module;
// OrganOrchestrator 类似 v1 AwakeCompanion 真实施 (per R11 spec + 子代理 R12 真实施).
// **0 装诚实真账 (子代理 R12 独立判断)**:
// - 本 module 是 spec 部分真实施 (估 30-45 分钟), 完整 1-3 周估待 (per R11 §8.4).
// - 9 organ process 串联 + 8 重 gate 真实存在 + 5 状态机 forward-declared + L0-L5 骨架.
// - 0 触碰 cognitive.rs 12 slot (LOCKED, 子代理 K 核验).
// - 0 引新外部 dep (Cargo.lock 0 行 diff).
pub mod orchestrator;
// Bridge from the plugin `LlmFactory` contract onto the runtime-owned
// `ModuleInvoker`, so organ LLM calls (W1/W2) can ride the canonical
// completion-governed path once organ wiring lands. Adapter only: it holds
// no provider, budget, or governance authority of its own.
pub mod organ_llm_bridge;
// The ONE canonical organ-ownership module (AfterTurn post-turn cognition).
// Long-lived: 7 deterministic organs + OrganOrchestrator backend; transient
// per invocation: W1/W2 built from ctx.invoker_handle() and dropped.
pub mod organ_module;
pub mod preference_learning;
pub mod production;
pub mod provider;
pub mod runtime;
pub mod session;
pub mod subloop;
pub mod tool_modules;
pub mod trace;
// L0-L5 自升级 cycle driver (Stage 5 完整化, per R11 §7 + v2-architecture-reflection.md §6).
// **0 装诚实**: L0/L2/L3/L4 真接 governance + Orchestrator; L5 建议模式不自动跑 git tag;
// L1 接 SelfAssessmentStore. 主人 Veto dashboard 留 v2.0.0 release 接入.
pub mod upgrade_cycle;

pub use approval::{
    operation_fingerprint, operation_fingerprint_with_invocation, ApprovalDecision, ApprovalStatus,
    PendingApproval, PendingApprovalView,
};
pub use cognitive::{
    turn_request_from_perception, CognitiveModuleEvent, CognitiveTelemetry, CouncilModule,
    JudgeConfig, JudgeModule, JudgeObservations, JudgeResult, JudgeVerdict, MemoryRecallModule,
    MemoryWritebackModule, ModuleMetricsSnapshot, PreferenceRecallModule, SelfAssessmentModule,
    COUNCIL_MODULE_ID, DEFERRED_COGNITIVE_SLOTS, JUDGE_MODULE_ID, MEMORY_RECALL_MODULE_ID,
    MEMORY_WRITEBACK_MODULE_ID, PREFERENCE_RECALL_MODULE_ID, SELF_ASSESSMENT_MODULE_ID,
};
pub use error::{RuntimeError, RuntimeResult};
pub use execute::{ApprovalResolution, TurnOutcome, TurnRequest, TurnResponse};
pub use module::{
    AgentModule, HookPoint, InvocationContext, InvocationOrigin, Module, ModuleContext,
    ModuleDirective, ModuleError, ModuleInvocationError, ModuleInvocationRequest,
    ModuleInvocationResponse, ModuleInvoker, ModuleManifest, ModuleOutcome, ModuleRegistry,
    PromptOverlay, DEFAULT_MAX_INVOCATION_DEPTH, DEFAULT_MAX_MODULE_INVOCATIONS,
};
pub use organ_llm_bridge::{InvokerLlmFactory, InvokerLlmInstance, INVOKER_LLM_FACTORY_NAME};
pub use organ_module::{OrganModule, OrganModuleObservation, ORGAN_MODULE_ID};
pub use preference_learning::{
    PreferenceEvidence, PreferenceLearningModule, PreferenceLearningStats, PreferencePolarity,
    PREFERENCE_LEARNING_MODULE_ID,
};
pub use production::{
    CognitiveBackends, CognitiveModuleConfig, ProductionBackends, ProductionCognitiveModules,
    ProductionModules, ProductionModulesConfig,
};
pub use provider::{ProviderHealth, ProviderRouter, RoutedCompletion};
pub use runtime::{plugin_ids, Runtime, RuntimeBuilder, RuntimeConfig, DEFAULT_MAX_ROUNDS};
pub use session::{
    InMemorySessionStore, Session, SessionEvent, SessionEventKind, SessionManager, SessionStore,
    SqliteSessionStore,
};
pub use subloop::{
    RuntimeSubLoopSpawner, SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
pub use tool_modules::{
    FetchModule, FilesystemModule, McpModule, RepoModule, SearchModule, ShellModule,
};
pub use trace::{ExecutionTrace, TraceEntry, TraceEvent};

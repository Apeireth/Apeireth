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
pub mod production;
pub mod provider;
pub mod runtime;
pub mod session;
pub mod trace;

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
    AgentModule, HookPoint, InvocationContext, InvocationOrigin, ModuleContext, ModuleDirective,
    ModuleError, ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse,
    ModuleInvoker, ModuleManifest, ModuleOutcome, PromptOverlay, DEFAULT_MAX_INVOCATION_DEPTH,
    DEFAULT_MAX_MODULE_INVOCATIONS,
};
pub use production::{CognitiveBackends, CognitiveModuleConfig, ProductionCognitiveModules};
pub use provider::{ProviderHealth, ProviderRouter, RoutedCompletion};
pub use runtime::{plugin_ids, Runtime, RuntimeBuilder, RuntimeConfig, DEFAULT_MAX_ROUNDS};
pub use session::{
    InMemorySessionStore, Session, SessionEvent, SessionEventKind, SessionManager, SessionStore,
    SqliteSessionStore,
};
pub use trace::{ExecutionTrace, TraceEntry, TraceEvent};

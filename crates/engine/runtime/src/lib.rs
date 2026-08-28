//! Canonical Apeireth runtime.
//!
//! The runtime owns session lifecycle, provider routing, the agent loop, and
//! approval resumption. Providers and tools arrive as injected plugins;
//! transport translation belongs to adapters and policy belongs to governance.

#![deny(unsafe_code)]

/// Canonical runtime composition root and execution model.
pub mod canonical;

pub use canonical::{
    operation_fingerprint, operation_fingerprint_with_invocation, plugin_ids,
    turn_request_from_perception, AgentModule, ApprovalDecision, ApprovalResolution,
    ApprovalStatus, CognitiveBackends, CognitiveModuleConfig, CognitiveModuleEvent,
    CognitiveTelemetry, CouncilModule, FetchModule, FilesystemModule, ExecutionTrace, HookPoint, InMemorySessionStore,
    InvocationContext, InvocationOrigin, JudgeConfig, JudgeModule, JudgeObservations, JudgeResult,
    JudgeVerdict, McpModule, MemoryRecallModule, MemoryWritebackModule, Module, ModuleContext, ModuleDirective,
    ModuleError, ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse,
    ModuleInvoker, ModuleManifest, ModuleMetricsSnapshot, ModuleOutcome, ModuleRegistry,
    PendingApproval, PendingApprovalView, PreferenceRecallModule, ProductionBackends, ProductionCognitiveModules,
    ProductionModules, ProductionModulesConfig, PromptOverlay, ProviderHealth, ProviderRouter, RepoModule, RoutedCompletion, Runtime, RuntimeBuilder,
    RuntimeConfig, RuntimeError, RuntimeResult, RuntimeSubLoopSpawner, SearchModule, SelfAssessmentModule, Session, SessionEvent,
    SessionEventKind, SessionManager, SessionStore, ShellModule, SqliteSessionStore, SubLoopError, SubLoopResult, SubLoopSpec, SubLoopSpawner, TraceEntry, TraceEvent,
    TurnOutcome, TurnRequest, TurnResponse, COUNCIL_MODULE_ID, DEFAULT_MAX_INVOCATION_DEPTH,
    DEFAULT_MAX_MODULE_INVOCATIONS, DEFAULT_MAX_ROUNDS, DEFERRED_COGNITIVE_SLOTS, JUDGE_MODULE_ID,
    MEMORY_RECALL_MODULE_ID, MEMORY_WRITEBACK_MODULE_ID, PREFERENCE_RECALL_MODULE_ID,
    SELF_ASSESSMENT_MODULE_ID,
};

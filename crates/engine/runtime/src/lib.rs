//! Canonical Apeireth runtime.
//!
//! The runtime owns session lifecycle, provider routing, the agent loop, and
//! approval resumption. Providers and tools arrive as injected plugins;
//! transport translation belongs to adapters and policy belongs to governance.

#![deny(unsafe_code)]

/// Canonical runtime mechanism kernel and execution model.
pub mod canonical;

pub use canonical::{
    budget_context_blocks, budget_context_blocks_with_spill, compaction_entries, fold_surface,
    fork_placeholder, open_tool_calls, operation_fingerprint,
    operation_fingerprint_with_invocation, plan_mode_exit_declaration, plugin_ids, AgentModule,
    ApprovalDecision, ApprovalResolution, ApprovalStatus, BehaviorModule, CapabilityProvider,
    CapabilityRegistry, CompositeEventSink, CompositeRuntimeEventSink, ContextBlock,
    ContextProjectionError, ContextProjector, ExecutionTrace, ForkRecord, HookPoint,
    InMemorySessionStore, InvocationContext, InvocationOrigin, LogEntry, Module, ModuleContext,
    ModuleDirective, ModuleError, ModuleInvocationError, ModuleInvocationRequest,
    ModuleInvocationResponse, ModuleInvoker, ModuleManifest, ModuleOutcome, ModuleRegistry,
    PendingApproval, PendingApprovalView, PromptOverlay, ProviderHealth, ProviderRouter,
    RoutedCompletion, Runtime, RuntimeBuilder, RuntimeCapabilitySnapshot, RuntimeConfig,
    RuntimeError, RuntimeEvent, RuntimeEventSink, RuntimeHealthSnapshot, RuntimeModelSnapshot,
    RuntimeModuleSnapshot, RuntimeProviderSnapshot, RuntimeResult, RuntimeSnapshot,
    RuntimeSubLoopSpawner, Session, SessionEvent, SessionEventKind, SessionManager, SessionStore,
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec, SurfaceOp, SurfaceSegment,
    SurfaceSource, SurfaceView, TraceEntry, TraceEvent, TurnOutcome, TurnRequest, TurnResponse,
    DEFAULT_CONTEXT_BUDGET_CHARS, DEFAULT_MAX_INVOCATION_DEPTH, DEFAULT_MAX_MODULE_INVOCATIONS,
    DEFAULT_MAX_ROUNDS, PLAN_MODE_EXIT_TOOL_NAME,
};

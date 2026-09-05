//! Canonical Apeireth runtime.
//!
//! The runtime owns session lifecycle, provider routing, the agent loop, and
//! approval resumption. Providers and tools arrive as injected plugins;
//! transport translation belongs to adapters and policy belongs to governance.

#![deny(unsafe_code)]

/// Canonical runtime mechanism kernel and execution model.
pub mod canonical;

pub use canonical::{
    operation_fingerprint, operation_fingerprint_with_invocation, plugin_ids, AgentModule,
    ApprovalDecision, ApprovalResolution, ApprovalStatus, BehaviorModule, CapabilityProvider,
    CapabilityRegistry, CompositeEventSink, CompositeRuntimeEventSink, ExecutionTrace, HookPoint,
    InMemorySessionStore, InvocationContext, InvocationOrigin, Module, ModuleContext,
    ModuleDirective, ModuleError, ModuleInvocationError, ModuleInvocationRequest,
    ModuleInvocationResponse, ModuleInvoker, ModuleManifest, ModuleOutcome, ModuleRegistry,
    PendingApproval, PendingApprovalView, PromptOverlay, ProviderHealth, ProviderRouter,
    RoutedCompletion, Runtime, RuntimeBuilder, RuntimeCapabilitySnapshot, RuntimeConfig,
    RuntimeError, RuntimeEvent, RuntimeEventSink, RuntimeHealthSnapshot, RuntimeModelSnapshot,
    RuntimeModuleSnapshot, RuntimeProviderSnapshot, RuntimeResult, RuntimeSnapshot,
    RuntimeSubLoopSpawner, Session, SessionEvent, SessionEventKind, SessionManager, SessionStore,
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec, TraceEntry, TraceEvent, TurnOutcome,
    TurnRequest, TurnResponse, DEFAULT_MAX_INVOCATION_DEPTH, DEFAULT_MAX_MODULE_INVOCATIONS,
    DEFAULT_MAX_ROUNDS,
};

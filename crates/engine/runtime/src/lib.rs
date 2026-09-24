//! Canonical Apeireth runtime.
//!
//! The runtime owns session lifecycle, provider routing, the agent loop, and
//! approval resumption. Providers, tools, and turn-observing behaviors all
//! arrive as **modules** through one registry: there is no plugin world beside
//! a module world. Transport translation belongs to adapters and policy belongs
//! to governance.

#![deny(unsafe_code)]

/// Canonical runtime mechanism kernel and execution model.
pub mod canonical;

pub use canonical::{
    module_ids, operation_fingerprint, operation_fingerprint_with_invocation, plugin_ids,
    AgentModule, ApprovalDecision, ApprovalResolution, ApprovalStatus, BehaviorModule,
    CompositeEventSink, CompositeRuntimeEventSink, ContextProjectionError, ContextProjector,
    ExecutionTrace, HookPoint, InMemorySessionStore, InvocationContext, InvocationOrigin, Module,
    ModuleContext, ModuleDirective, ModuleError, ModuleExports, ModuleInitContext,
    ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker,
    ModuleManager, ModuleManifest, ModuleOrigin, ModuleOutcome, ModuleRegistry, PendingApproval,
    PendingApprovalView, PromptOverlay, ProviderHealth, ProviderRouter, RoutedCompletion, Runtime,
    RuntimeBuilder, RuntimeCapabilitySnapshot, RuntimeConfig, RuntimeError, RuntimeEvent,
    RuntimeEventSink, RuntimeHealthSnapshot, RuntimeModelSnapshot, RuntimeModuleInvoker,
    RuntimeModuleSnapshot, RuntimeProviderSnapshot, RuntimeResult, RuntimeSnapshot,
    RuntimeSubLoopSpawner, Session, SessionEvent, SessionEventKind, SessionManager, SessionStore,
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec, TraceEntry, TraceEvent, TurnOutcome,
    TurnRequest, TurnResponse, DEFAULT_MAX_INVOCATION_DEPTH, DEFAULT_MAX_MODULE_INVOCATIONS,
    DEFAULT_MAX_ROUNDS,
};

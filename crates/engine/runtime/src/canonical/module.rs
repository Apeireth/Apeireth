//! Generic cognitive-module ABI for the canonical runtime.
//!
//! The runtime owns the hook lifecycle and the isolated provider side-call
//! boundary. A module owns policy; the runtime never branches on a module's
//! domain or name.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_protocol::canonical::{
    NormalizedMessage, NormalizedRequest, NormalizedResponse, ToolCall, ToolResult,
};
use async_trait::async_trait;
use thiserror::Error;

use super::provider::ProviderRouter;

/// Lifecycle points at which a module may observe or influence a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookPoint {
    /// The user input has entered the current turn's persistent context.
    TurnStart,
    /// Immediately before one provider invocation.
    BeforeModelCall,
    /// Immediately after a provider response and before branch handling.
    AfterModelResponse,
    /// Immediately before one tool is dispatched.
    BeforeToolCall,
    /// Immediately after one tool result is available.
    AfterToolResult,
    /// Immediately before a candidate answer is committed to the transcript.
    BeforeFinalCommit,
    /// After a turn has been committed successfully.
    ///
    /// This point is observational; its directive cannot undo the durable
    /// commit.
    AfterTurn,
    /// When an observable runtime or module failure occurs.
    ///
    /// This point is best-effort observation; its directive cannot recover or
    /// replace the original failure.
    OnError,
}

/// Minimal identity for a registered module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleManifest {
    /// Stable module identifier.
    pub id: String,
    /// Human-readable module name.
    pub name: String,
}

impl ModuleManifest {
    /// Create a minimal manifest.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// A provider-request-only prompt message.
///
/// An overlay is deliberately not a session message. It is copied into one
/// provider request and then dropped, so it cannot alter the persisted
/// conversation or appear automatically in a later turn.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptOverlay {
    message: NormalizedMessage,
}

impl PromptOverlay {
    /// Add a transient system message to one provider invocation.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            message: NormalizedMessage::system(content),
        }
    }

    /// The normalized message that will be sent to the provider.
    pub fn message(&self) -> &NormalizedMessage {
        &self.message
    }
}

/// The generic control result of a module hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleDirective {
    /// Continue the canonical runtime path.
    Continue,
    /// Ask the canonical loop to spend another logical round with feedback.
    Retry {
        /// Transient feedback for the next provider invocation.
        feedback: String,
    },
    /// Reject the current turn without committing its candidate.
    Stop {
        /// Human- and caller-facing reason.
        reason: String,
    },
}

/// The value returned by one module hook.
///
/// When several modules observe the same hook, the canonical runtime invokes
/// them in registration order, concatenates their overlays in that order, and
/// resolves directives by strength: `Stop` overrides `Retry`, which overrides
/// `Continue`. Equal-strength directives keep the first module's result, and a
/// directive does not short-circuit the remaining modules for that hook.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleOutcome {
    /// Prompt messages to prepend to the next provider request in this turn.
    pub prompt_overlays: Vec<PromptOverlay>,
    /// Generic control directive.
    pub directive: ModuleDirective,
}

impl ModuleOutcome {
    /// Continue without changing the provider request.
    pub fn continue_() -> Self {
        Self {
            prompt_overlays: Vec::new(),
            directive: ModuleDirective::Continue,
        }
    }

    /// Ask for another canonical provider round.
    pub fn retry(feedback: impl Into<String>) -> Self {
        Self {
            prompt_overlays: Vec::new(),
            directive: ModuleDirective::Retry {
                feedback: feedback.into(),
            },
        }
    }

    /// Stop the current turn without committing the candidate.
    pub fn stop(reason: impl Into<String>) -> Self {
        Self {
            prompt_overlays: Vec::new(),
            directive: ModuleDirective::Stop {
                reason: reason.into(),
            },
        }
    }

    /// Add one transient overlay to this outcome.
    #[must_use]
    pub fn with_prompt_overlay(mut self, overlay: PromptOverlay) -> Self {
        self.prompt_overlays.push(overlay);
        self
    }

    /// Add one transient system overlay to this outcome.
    #[must_use]
    pub fn with_system_overlay(mut self, content: impl Into<String>) -> Self {
        self.prompt_overlays.push(PromptOverlay::system(content));
        self
    }
}

/// Origin and nesting information for a runtime invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationOrigin {
    /// The top-level user turn.
    UserTurn,
    /// An isolated invocation requested by a module.
    Module {
        /// The module that requested the side-call.
        module_id: String,
    },
}

/// Invocation metadata exposed to modules without exposing runtime internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationContext {
    /// Where this invocation originated.
    pub origin: InvocationOrigin,
    /// Nesting depth. Top-level turns are zero; isolated module calls are one.
    pub depth: u8,
}

impl InvocationContext {
    /// Context for a canonical user turn.
    pub fn user_turn() -> Self {
        Self {
            origin: InvocationOrigin::UserTurn,
            depth: 0,
        }
    }
}

/// Maximum nesting depth supported by the first isolated side-call ABI.
pub const DEFAULT_MAX_INVOCATION_DEPTH: u8 = 1;

/// Request for one isolated module-side provider call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInvocationRequest {
    /// Optional transient system instruction.
    pub system: Option<String>,
    /// Isolated user-facing input for the side-call.
    pub input: String,
    /// Explicit model, or the current top-level model when absent.
    pub model: Option<String>,
}

impl ModuleInvocationRequest {
    /// Create an isolated request with a system instruction and input.
    pub fn isolated(system: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            system: Some(system.into()),
            input: input.into(),
            model: None,
        }
    }

    /// Select a model for this isolated call.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Response from one isolated module-side provider call.
#[derive(Debug, Clone)]
pub struct ModuleInvocationResponse {
    /// The normalized provider response.
    pub response: NormalizedResponse,
    /// Provider that served the call.
    pub served_by: CapabilityId,
}

impl ModuleInvocationResponse {
    /// Text returned by the isolated call.
    pub fn text(&self) -> &str {
        &self.response.content
    }
}

/// Failure from an isolated module-side provider call.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ModuleInvocationError {
    /// The per-turn side-call budget has been exhausted.
    #[error("module invocation budget exceeded (limit: {limit})")]
    BudgetExceeded {
        /// Maximum number of side-calls allowed in the turn.
        limit: usize,
    },
    /// The invocation nesting boundary was exceeded.
    #[error("module invocation depth {depth} exceeds maximum {maximum}")]
    RecursionLimit {
        /// Requested depth.
        depth: u8,
        /// Supported maximum.
        maximum: u8,
    },
    /// No model was available for the isolated call.
    #[error("module invocation has no model")]
    NoModel,
    /// The canonical provider router could not serve the side-call.
    #[error("module side invocation failed: {reason}")]
    Provider {
        /// Legible provider/runtime failure.
        reason: String,
    },
}

/// Failure returned by a module hook.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ModuleError {
    /// A module-specific failure with a legible message.
    #[error("{0}")]
    Message(String),
    /// A hook required a model candidate that was not present.
    #[error("module hook requires a candidate model response")]
    MissingCandidate,
    /// An isolated side-call failed.
    #[error(transparent)]
    Invocation(#[from] ModuleInvocationError),
}

impl From<String> for ModuleError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for ModuleError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

/// Runtime-owned side-call capability exposed to modules.
#[async_trait]
pub trait ModuleInvoker: Send + Sync {
    /// Perform one isolated provider invocation.
    async fn invoke(
        &self,
        request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError>;
}

/// Read-only context supplied to a module hook.
pub struct ModuleContext<'a> {
    /// Session being executed.
    pub session_id: &'a SessionId,
    /// Model selected for the canonical turn.
    pub model: &'a str,
    /// Persistent transcript visible at this point in the turn.
    pub messages: &'a [NormalizedMessage],
    /// Provider candidate, when the hook runs after a model response.
    pub candidate: Option<&'a NormalizedResponse>,
    /// Tool call currently being considered.
    pub tool_call: Option<&'a ToolCall>,
    /// Tool result currently available.
    pub tool_result: Option<&'a ToolResult>,
    /// Invocation origin and nesting metadata.
    pub invocation: &'a InvocationContext,
    /// The module receiving this context.
    pub module_id: &'a str,
    /// Error text for [`HookPoint::OnError`], when available.
    pub error: Option<&'a str>,
    pub(crate) invoker: &'a dyn ModuleInvoker,
}

impl<'a> ModuleContext<'a> {
    /// The candidate response, when this hook has one.
    pub fn candidate(&self) -> Option<&'a NormalizedResponse> {
        self.candidate
    }

    /// The tool call, when this hook has one.
    pub fn tool_call(&self) -> Option<&'a ToolCall> {
        self.tool_call
    }

    /// The tool result, when this hook has one.
    pub fn tool_result(&self) -> Option<&'a ToolResult> {
        self.tool_result
    }

    /// Runtime-owned isolated model invocation capability.
    pub fn invoker(&self) -> &'a dyn ModuleInvoker {
        self.invoker
    }
}

use apeireth_plugin::ToolCapability;

/// A module participating in the canonical runtime lifecycle and providing capabilities.
///
/// Hook calls are sequential and deterministic. A hook error aborts an
/// in-progress turn and is reported through [`HookPoint::OnError`] when
/// possible; `AfterTurn` is already post-commit and `OnError` is best effort.
/// Module directives cannot execute capabilities or alter a tool invocation.
/// Modules can also contribute tool capabilities to the unified capability registry.
#[async_trait]
pub trait Module: Send + Sync {
    /// Stable module identity.
    fn manifest(&self) -> &ModuleManifest;

    /// Observe a lifecycle point and return transient policy effects.
    async fn on_hook(
        &self,
        _hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        Ok(ModuleOutcome::continue_())
    }

    /// Tool capabilities contributed by this module.
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        Vec::new()
    }
}

/// Compatibility alias for the canonical [`Module`] trait.
pub use Module as AgentModule;

/// Canonical registry for all runtime modules.
///
/// Ensures deterministic registration ordering, unique module IDs,
/// and aggregates tool capabilities exposed by modules.
#[derive(Default, Clone)]
pub struct ModuleRegistry {
    modules: Vec<Arc<dyn Module>>,
}

impl ModuleRegistry {
    /// Create an empty module registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one module, rejecting duplicate or empty IDs.
    pub fn register(&mut self, module: Arc<dyn Module>) -> Result<(), String> {
        let manifest = module.manifest();
        if manifest.id.is_empty() {
            return Err("registered modules must have a non-empty id".to_string());
        }
        if self.modules.iter().any(|m| m.manifest().id == manifest.id) {
            return Err(format!("duplicate module id {:?}", manifest.id));
        }
        self.modules.push(module);
        Ok(())
    }

    /// All registered modules in deterministic registration order.
    pub fn modules(&self) -> &[Arc<dyn Module>] {
        &self.modules
    }

    /// Tool capabilities contributed by all registered modules.
    pub fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.modules.iter().flat_map(|m| m.tools()).collect()
    }

    /// Find a tool capability by tool name across all modules.
    pub fn find_tool_by_name(&self, name: &str) -> Option<Arc<dyn ToolCapability>> {
        for module in &self.modules {
            for tool in module.tools() {
                if tool.declaration().name == name {
                    return Some(tool);
                }
            }
        }
        None
    }

    /// Number of registered modules.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Whether the registry contains no modules.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl From<Vec<Arc<dyn Module>>> for ModuleRegistry {
    fn from(modules: Vec<Arc<dyn Module>>) -> Self {
        Self { modules }
    }
}

/// Default maximum number of isolated module provider calls in one turn.
pub const DEFAULT_MAX_MODULE_INVOCATIONS: usize = 8;

/// Per-turn side-call accounting shared by all registered modules.
pub(crate) struct ModuleTurnState {
    used: AtomicUsize,
    max: usize,
}

impl ModuleTurnState {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            used: AtomicUsize::new(0),
            max,
        }
    }

    pub(crate) fn with_used(max: usize, used: usize) -> Self {
        Self {
            used: AtomicUsize::new(used),
            max,
        }
    }

    pub(crate) fn used(&self) -> usize {
        self.used.load(Ordering::Relaxed)
    }

    pub(crate) fn set_used(&self, used: usize) {
        self.used.store(used, Ordering::Relaxed);
    }

    fn reserve(&self) -> Result<(), ModuleInvocationError> {
        loop {
            let used = self.used.load(Ordering::Relaxed);
            if used >= self.max {
                return Err(ModuleInvocationError::BudgetExceeded { limit: self.max });
            }
            if self
                .used
                .compare_exchange(used, used + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(());
            }
        }
    }
}

/// Runtime-owned implementation of the isolated module invoker.
pub(crate) struct RuntimeModuleInvoker<'a> {
    router: &'a ProviderRouter,
    current_model: &'a str,
    module_id: &'a str,
    state: Arc<ModuleTurnState>,
    depth: u8,
}

impl<'a> RuntimeModuleInvoker<'a> {
    pub(crate) fn new(
        router: &'a ProviderRouter,
        current_model: &'a str,
        module_id: &'a str,
        state: Arc<ModuleTurnState>,
        parent_depth: u8,
    ) -> Self {
        Self {
            router,
            current_model,
            module_id,
            state,
            depth: parent_depth.saturating_add(1),
        }
    }
}

#[async_trait]
impl ModuleInvoker for RuntimeModuleInvoker<'_> {
    async fn invoke(
        &self,
        request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
        if self.depth > DEFAULT_MAX_INVOCATION_DEPTH {
            return Err(ModuleInvocationError::RecursionLimit {
                depth: self.depth,
                maximum: DEFAULT_MAX_INVOCATION_DEPTH,
            });
        }
        let model = request
            .model
            .unwrap_or_else(|| self.current_model.to_string());
        if model.is_empty() {
            return Err(ModuleInvocationError::NoModel);
        }
        self.state.reserve()?;

        let mut messages = Vec::with_capacity(2);
        if let Some(system) = request.system {
            messages.push(NormalizedMessage::system(system));
        }
        messages.push(NormalizedMessage::user(request.input));

        // `complete` intentionally sends no tool declarations. It also does
        // not enter the session store or the module hook bus, so this is an
        // isolated side-call rather than a nested agent loop.
        let routed = self
            .router
            .complete(&NormalizedRequest::new(model, messages))
            .await
            .map_err(|error| ModuleInvocationError::Provider {
                reason: format!("module {}: {error}", self.module_id),
            })?;

        Ok(ModuleInvocationResponse {
            response: routed.response,
            served_by: routed.served_by,
        })
    }
}

//! Generic cognitive-module ABI for the canonical runtime.
//!
//! The runtime owns the hook lifecycle and the isolated provider side-call
//! boundary. A module owns policy; the runtime never branches on a module's
//! domain or name.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_governance::TurnSecurityContext;
use apeireth_plugin::agents::{
    AgentConfig, AgentError, AgentErrorCode, AgentExecutionMode, AgentTask, AgentTaskOrigin,
};
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedResponse, ToolCall, ToolResult};
use async_trait::async_trait;
use thiserror::Error;

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

/// Compatibility content block returned by a behavior producer.
///
/// The message shape is retained for readers and producers. RegisteredBehavior
/// discards its role and records the text as untrusted ToolEventV1 data. Runtime
/// projects a matched assistant/tool pair once; it never injects this as system
/// policy. Facts persist, while business-history readers exclude module pairs.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptOverlay {
    message: NormalizedMessage,
    /// Source label when the block carries cross-source content that must be
    /// disclosed through the untrusted reference envelope at the provider
    /// funnel; `None` marks text the caller owns (or text that already is an
    /// envelope disclosure), which passes the funnel byte-for-byte.
    cross_source: Option<String>,
}

impl PromptOverlay {
    /// Construct a compatibility block; the system role is not provider authority.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            message: NormalizedMessage::system(content),
            cross_source: None,
        }
    }

    /// A compatibility block carrying cross-source content — material
    /// recalled from outside the current turn (other sessions, external
    /// stores). `source` names where the content came from and is rendered
    /// inside the disclosure envelope; the provider-request funnel discloses
    /// the block through the untrusted reference envelope before composition
    /// as data inside its authenticated tool result.
    pub fn system_cross_source(source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            message: NormalizedMessage::system(content),
            cross_source: Some(source.into()),
        }
    }

    /// The cross-source label, when the block carries cross-source content.
    pub fn cross_source_label(&self) -> Option<&str> {
        self.cross_source.as_deref()
    }

    /// The producer representation; Runtime sends its text through V1 pairing.
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
/// them through registered tools in registration order, commits V1 data, and
/// resolves directives by strength: `Stop` overrides `Retry`, which overrides
/// `Continue`. Equal-strength directives keep the first module's result, and a
/// directive does not short-circuit the remaining modules for that hook.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleOutcome {
    /// Producer blocks converted to untrusted V1 data, never system instructions.
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
    /// Canonical completion governance refused the isolated call.
    #[error("module side invocation refused by governance: {reason}")]
    Denied {
        /// Why the isolated completion was refused.
        reason: String,
    },
    /// Isolated completions cannot create a hidden human approval.
    #[error("module side invocation requires approval which is not permitted: {reason}")]
    ApprovalRequired {
        /// Why a human would have been asked.
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
    pub invoker: &'a dyn ModuleInvoker,
    pub invoker_handle: Arc<dyn ModuleInvoker>,
    pub subloop: &'a dyn super::subloop::SubLoopSpawner,
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

    /// Owned handle onto the same per-turn isolated invoker as [`Self::invoker`].
    ///
    /// An owned `ModuleInvoker` handle carries one invocation's session, trace,
    /// model and budget context. It is intended for asynchronous work spawned
    /// within that invocation and must not be retained as persistent module
    /// state. Clones of the handle share this invocation's
    /// `ModuleTurnState` budget; they never mint a fresh one. The next turn
    /// receives a different handle with different turn context.
    ///
    /// Both accessors dereference the same per-turn invoker object; the owned
    /// form exists so a hook can move the capability into async work (for
    /// example an `LlmFactory` adapter) that outlives the borrowed context.
    pub fn invoker_handle(&self) -> Arc<dyn ModuleInvoker> {
        Arc::clone(&self.invoker_handle)
    }

    /// Runtime-owned bounded SubLoop spawner.
    pub fn subloop(&self) -> &'a dyn super::subloop::SubLoopSpawner {
        self.subloop
    }
}

use apeireth_plugin::ToolCapability;

/// A behavior module participating in the canonical runtime lifecycle.
///
/// Hook calls are sequential and deterministic. A hook error aborts an
/// in-progress turn and is reported through [`HookPoint::OnError`] when
/// possible; `AfterTurn` is already post-commit and `OnError` is best effort.
/// Module directives cannot execute capabilities or alter a tool invocation.
/// Dispatchable capabilities are deliberately not part of this trait. Register
/// them through [`super::capability::CapabilityRegistry`] instead.
#[async_trait]
pub trait BehaviorModule: Send + Sync {
    /// Stable module identity.
    fn manifest(&self) -> &ModuleManifest;

    /// Whether this module can expose a shared store without mutating it.
    /// Host mapping refuses read-only sharing unless the implementation opts in.
    fn read_only_profile_projection(&self) -> bool {
        false
    }

    /// Authenticated adapter entry. Host mappings can select isolated resources here.
    /// The default preserves the public compatibility callback ABI.
    async fn on_hook_authenticated(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
        _execution: &apeireth_plugin::ToolExecutionContext,
    ) -> Result<ModuleOutcome, ModuleError> {
        self.on_hook(hook, ctx).await
    }

    /// Observe a lifecycle point and return transient policy effects.
    async fn on_hook(
        &self,
        _hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        Ok(ModuleOutcome::continue_())
    }
}

/// Compatibility alias for the renamed behavior trait.
pub use BehaviorModule as Module;

/// Canonical registry for all runtime modules.
///
/// Ensures deterministic registration ordering, unique module IDs,
/// and aggregates tool capabilities exposed by modules.
#[derive(Default, Clone)]
pub struct BehaviorRegistry {
    modules: Vec<Arc<dyn BehaviorModule>>,
}

impl BehaviorRegistry {
    /// Create an empty module registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one module, rejecting duplicate or empty IDs.
    pub fn register(&mut self, module: Arc<dyn BehaviorModule>) -> Result<(), String> {
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
    pub fn modules(&self) -> &[Arc<dyn BehaviorModule>] {
        &self.modules
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

impl From<Vec<Arc<dyn BehaviorModule>>> for BehaviorRegistry {
    fn from(modules: Vec<Arc<dyn BehaviorModule>>) -> Self {
        Self { modules }
    }
}

/// Reject duplicate capability ids or model-facing names between two tool sets.
pub fn reject_tool_identity_collisions(
    incumbent: &[Arc<dyn ToolCapability>],
    challengers: &[Arc<dyn ToolCapability>],
    challenger_owner: &str,
) -> Result<(), String> {
    let mut by_id: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let mut by_name: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for tool in incumbent {
        by_id.insert(tool.id().to_string(), tool.declaration().name);
        by_name.insert(tool.declaration().name, tool.id().to_string());
    }
    for tool in challengers {
        let id = tool.id().to_string();
        let name = tool.declaration().name;
        if let Some(existing_name) = by_id.get(&id) {
            return Err(format!(
                "duplicate capability id {id} from {challenger_owner} collides with existing tool {existing_name:?}"
            ));
        }
        if let Some(existing_id) = by_name.get(&name) {
            return Err(format!(
                "duplicate tool name {name:?} from {challenger_owner} collides with existing capability {existing_id}"
            ));
        }
        by_id.insert(id, name.clone());
        by_name.insert(name, tool.id().to_string());
    }
    Ok(())
}

impl std::ops::Deref for BehaviorRegistry {
    type Target = [Arc<dyn BehaviorModule>];

    fn deref(&self) -> &Self::Target {
        &self.modules
    }
}

impl<'a> IntoIterator for &'a BehaviorRegistry {
    type Item = &'a Arc<dyn BehaviorModule>;
    type IntoIter = std::slice::Iter<'a, Arc<dyn Module>>;

    fn into_iter(self) -> Self::IntoIter {
        self.modules.iter()
    }
}

/// Default maximum number of isolated module provider calls in one turn.
///
/// Must fit the deepest legitimate combination in one candidate pass —
/// Council (≤7 advisors) + Judge (1) = 8 — **plus** retry passes. The
/// production "cognitive depth = deep" preset (judge + council) consumes the
/// full pass budget on the first pass; a judge-requested regeneration then
/// needs a second full pass. 24 leaves headroom for retries while still
/// bounding per-turn LLM cost (2026-09-19 real-machine finding: the old limit
/// of 8 made the deep preset abort every turn whose first candidate was
/// retried).
pub const DEFAULT_MAX_MODULE_INVOCATIONS: usize = 24;

/// Per-turn side-call accounting shared by all registered modules.
pub struct ModuleTurnState {
    used: AtomicUsize,
    max: usize,
}

impl ModuleTurnState {
    pub fn new(max: usize) -> Self {
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

    pub(crate) fn reserve(&self) -> Result<(), ModuleInvocationError> {
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
///
/// An owned turn-scoped handle: it carries one invocation's session, trace,
/// model and budget context, clones share that invocation's `ModuleTurnState`,
/// and it is not task-local or global ambient authority — it exists only where
/// the runtime hands it out for the current invocation. It must not be
/// retained as persistent module state across turns.
#[derive(Clone)]
pub struct RuntimeModuleInvoker {
    runtime: Arc<super::Runtime>,
    origin: AgentTaskOrigin,
    security: Option<TurnSecurityContext>,
    current_model: Arc<str>,
    module_id: Arc<str>,
    state: Arc<ModuleTurnState>,
    depth: u8,
}

impl RuntimeModuleInvoker {
    /// Bind the invoker to the existing instance service and originating turn.
    /// A Runtime without that service fails closed; there is no provider fallback.
    pub fn new(
        runtime: Arc<super::Runtime>,
        origin: AgentTaskOrigin,
        security: Option<TurnSecurityContext>,
        current_model: impl Into<Arc<str>>,
        module_id: impl Into<Arc<str>>,
        state: Arc<ModuleTurnState>,
        parent_depth: u8,
    ) -> Self {
        Self {
            runtime,
            origin,
            security,
            current_model: current_model.into(),
            module_id: module_id.into(),
            state,
            depth: parent_depth.saturating_add(1),
        }
    }

    fn task_error(&self, error: AgentError) -> ModuleInvocationError {
        match error.code {
            AgentErrorCode::BudgetExceeded if self.state.used() >= self.state.limit() => {
                ModuleInvocationError::BudgetExceeded {
                    limit: self.state.limit(),
                }
            }
            AgentErrorCode::Denied => ModuleInvocationError::Denied {
                reason: error.message,
            },
            AgentErrorCode::ApprovalRequired | AgentErrorCode::PendingApproval => {
                ModuleInvocationError::ApprovalRequired {
                    reason: error.message,
                }
            }
            _ => ModuleInvocationError::Provider {
                reason: format!("module {}: {error}", self.module_id),
            },
        }
    }
}

#[async_trait]
impl ModuleInvoker for RuntimeModuleInvoker {
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
        // Fast refusal avoids audit-only children after exhaustion. The atomic
        // reservation happens solely at each actual Provider admission.
        if self.state.used() >= self.state.limit() {
            return Err(ModuleInvocationError::BudgetExceeded {
                limit: self.state.limit(),
            });
        }
        let child = self
            .runtime
            .spawn_side_agent(
                &self.module_id,
                self.origin.session,
                AgentConfig {
                    model: Some(model.clone()),
                    ..Default::default()
                },
                self.security.clone(),
            )
            .await
            .map_err(|error| self.task_error(error))?;
        let result = child
            .agent
            .execute(AgentTask {
                messages: vec![NormalizedMessage::user(request.input)],
                system: request.system,
                model: Some(model),
                max_rounds: 1,
                timeout: None,
                mode: AgentExecutionMode::SingleShot,
                request_gate: Some(self.state.clone()),
                origin: Some(self.origin.clone()),
            })
            .await;
        // Reuse the instance cancellation/drain boundary on success and error.
        // Dropping an interrupted adapter revokes via SideAgent's scope guard.
        child
            .agent
            .stop()
            .await
            .map_err(|error| self.task_error(error))?;
        let reply = result.map_err(|error| self.task_error(error))?;
        Ok(ModuleInvocationResponse {
            response: reply.response,
            served_by: reply.served_by,
        })
    }
}

impl apeireth_plugin::ExtensionCatalog for BehaviorRegistry {
    fn extension_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .modules()
            .iter()
            .map(|module| module.manifest().id.clone())
            .collect();
        ids.sort();
        ids
    }

    fn describe(&self, id: &str) -> Option<apeireth_plugin::ExtensionDescriptor> {
        let module = self
            .modules()
            .iter()
            .find(|module| module.manifest().id == id)?;
        let manifest = module.manifest();
        Some(apeireth_plugin::ExtensionDescriptor::new(
            manifest.id.clone(),
            apeireth_plugin::ExtensionKind::BehaviorModule,
            "apeireth-runtime",
            0,
        ))
    }
}

impl apeireth_plugin::agents::AgentRequestGate for ModuleTurnState {
    fn reserve(&self) -> apeireth_plugin::agents::AgentResult<()> {
        self.reserve().map_err(|e| {
            apeireth_plugin::agents::AgentError::new(
                apeireth_plugin::agents::AgentErrorCode::BudgetExceeded,
                e.to_string(),
            )
        })
    }
}
impl ModuleTurnState {
    pub(crate) fn limit(&self) -> usize {
        self.max
    }
}

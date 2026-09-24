//! The module ABI: one runtime entity with lifecycle, hooks, and capabilities.
//!
//! # The one idea
//!
//! **A module is the only runtime extension entity.** A module may own a
//! lifecycle, observe hooks, and export any number of capabilities of any
//! kinds. There is no second primitive called "plugin": a "plugin" is the
//! product-layer name for a module package a user installs after the fact,
//! and it lands in exactly the same registry, manifest schema, lifecycle, and
//! capability index as a system module. Builtin and installed modules differ
//! only in [`ModuleOrigin`], which is provenance and never runtime semantics.
//!
//! # Why the hook surface lives here
//!
//! The runtime used to own the hook ABI (`HookPoint`, `ModuleContext`,
//! `ModuleOutcome`, ...) in its kernel crate while the capability/lifecycle ABI
//! lived here, which forced two parallel traits — one for behavior, one for
//! capability ownership. Both halves only ever needed core and protocol types,
//! so both now live in this foundation crate. The runtime implements two
//! *ports* declared here ([`ModuleInvoker`], [`SubLoopSpawner`]) and hands them
//! to modules through [`ModuleContext`]; a module therefore never sees a
//! runtime type, and the layering still runs core <- protocol <- plugin <-
//! runtime.
//!
//! # Shape
//!
//! ```text
//! ModuleManifest  declares  ->  CapabilityDescriptor (id + kind)
//!       |                                |
//! ModuleRegistry (canonical)      CapabilityRegistry (id -> owner index)
//!       \                                /
//!        `------  ModuleManager  -------'
//!           lifecycle + deps + hook index + typed views
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedResponse, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{PluginError, PluginResult};
use crate::manifest::ModuleManifest;
use crate::provider::ProviderCapability;
use crate::subloop::SubLoopSpawner;
use crate::tool::ToolCapability;

/// Where a module came from.
///
/// Package provenance, and nothing else. It is recorded so diagnostics and a
/// future package manager can say *how a component arrived*; it never changes
/// how the module is loaded, dispatched, governed, or indexed. A system module
/// and an installed module take byte-identical paths through manifest,
/// registry, lifecycle, dependency resolution, and capability indexing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ModuleOrigin {
    /// Compiled into the binary and registered by the embedder.
    #[default]
    System,
    /// Installed by the user through a package.
    Installed,
    /// A local development build, loaded ahead of any installed copy.
    Development,
}

impl ModuleOrigin {
    /// Every origin, in declaration order.
    pub const ALL: [Self; 3] = [Self::System, Self::Installed, Self::Development];
}

/// How a module's capability set relates to its manifest.
///
/// The distinction exists because one real case cannot enumerate its
/// capabilities in a static manifest: a module wrapping an MCP server learns
/// which tools exist only by asking the server, possibly after boot. Naming the
/// two modes is what lets the strict manifest agreement check stay strict for
/// every other module instead of being relaxed for all of them.
///
/// This is a **manifest declaration** (see [`crate::manifest::ModuleManifest`]):
/// one fact, declared once. [`Module::capability_ownership`] is a derived view
/// of that declaration and nothing authoritative reads past it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityOwnership {
    /// Every exported capability is declared in the manifest, and every
    /// declared capability is exported. The manager enforces the agreement in
    /// both directions at registration.
    #[default]
    Static,
    /// The module owns capabilities that cannot be declared up front. Its
    /// manifest declares no capabilities; whatever it exports at registration is
    /// indexed, and further tools may be admitted after boot through
    /// [`Module::add_tool`]. Global uniqueness is enforced exactly as for a
    /// static module.
    Dynamic,
}

impl std::fmt::Display for ModuleOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::System => "system",
            Self::Installed => "installed",
            Self::Development => "development",
        })
    }
}

/// Lifecycle points at which a module may observe or influence a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

impl HookPoint {
    /// Every hook point, in canonical lifecycle order.
    pub const ALL: [Self; 8] = [
        Self::TurnStart,
        Self::BeforeModelCall,
        Self::AfterModelResponse,
        Self::BeforeToolCall,
        Self::AfterToolResult,
        Self::BeforeFinalCommit,
        Self::AfterTurn,
        Self::OnError,
    ];
}

impl std::fmt::Display for HookPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TurnStart => "TurnStart",
            Self::BeforeModelCall => "BeforeModelCall",
            Self::AfterModelResponse => "AfterModelResponse",
            Self::BeforeToolCall => "BeforeToolCall",
            Self::AfterToolResult => "AfterToolResult",
            Self::BeforeFinalCommit => "BeforeFinalCommit",
            Self::AfterTurn => "AfterTurn",
            Self::OnError => "OnError",
        })
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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
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
#[derive(Debug, thiserror::Error)]
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
///
/// Implemented by the runtime (or any host that embeds one) and handed to a
/// module through [`ModuleContext::invoker`]. A module never constructs one and
/// never retains one across turns.
#[async_trait]
pub trait ModuleInvoker: Send + Sync {
    /// Perform one isolated provider invocation.
    async fn invoke(
        &self,
        request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError>;
}

/// Read-only context supplied to a module hook.
///
/// Every field is borrowed from the turn that is running; nothing here outlives
/// the hook call. The two capability fields ([`Self::invoker`] and
/// [`Self::subloop`]) are runtime-owned ports: a module may use them for this
/// hook's isolated work and must not store them as persistent state.
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
    /// Runtime-owned isolated model invocation capability.
    pub invoker: &'a dyn ModuleInvoker,
    /// Runtime-owned bounded SubLoop spawner.
    pub subloop: &'a dyn SubLoopSpawner,
    /// Owned handle onto the same per-turn isolated invoker as `invoker`.
    pub invoker_handle: Arc<dyn ModuleInvoker>,
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
    /// [`ModuleTurnState`] budget; they never mint a fresh one. The next turn
    /// receives a different handle with different turn context.
    ///
    /// Both accessors dereference the same per-turn invoker object; the owned
    /// form exists so a hook can move the capability into async work (for
    /// example an `LlmFactory` adapter) that outlives the borrowed context.
    pub fn invoker_handle(&self) -> Arc<dyn ModuleInvoker> {
        Arc::clone(&self.invoker_handle)
    }

    /// Runtime-owned bounded SubLoop spawner.
    pub fn subloop(&self) -> &'a dyn SubLoopSpawner {
        self.subloop
    }
}

/// What a module hands to the runtime: the capabilities it can dispatch.
///
/// Deliberately a plain aggregate with builder methods. One owner may export
/// many capabilities of many kinds — a module that contributes both
/// `provider.acme` and `tool.acme_search` is one unit of lifecycle with two
/// entries in the capability index.
#[derive(Default)]
pub struct ModuleExports {
    /// Dispatchable tool capabilities.
    pub tools: Vec<Arc<dyn ToolCapability>>,
    /// Dispatchable provider capabilities.
    pub providers: Vec<Arc<dyn ProviderCapability>>,
}

impl ModuleExports {
    /// Empty exports.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add tool capabilities.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<Arc<dyn ToolCapability>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Add provider capabilities.
    #[must_use]
    pub fn with_providers(mut self, providers: Vec<Arc<dyn ProviderCapability>>) -> Self {
        self.providers.extend(providers);
        self
    }

    /// Whether this module exports nothing dispatchable.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.providers.is_empty()
    }

    /// Total number of exported capabilities.
    pub fn len(&self) -> usize {
        self.tools.len() + self.providers.len()
    }
}

impl std::fmt::Debug for ModuleExports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The trait objects are not Debug; the ids are the useful part.
        let tools: Vec<&str> = self.tools.iter().map(|t| t.id().as_str()).collect();
        let providers: Vec<&str> = self.providers.iter().map(|p| p.id().as_str()).collect();
        f.debug_struct("ModuleExports")
            .field("tools", &tools)
            .field("providers", &providers)
            .finish()
    }
}

/// What a module is given when it starts.
///
/// Everything a module needs from the outside world arrives here, which is what
/// keeps modules from reaching for globals, ambient environment, or fixed paths.
pub struct ModuleInitContext {
    /// Time source. Modules must read time from this, never from `Utc::now()`,
    /// so that a virtual clock makes their behaviour reproducible.
    pub clock: Arc<dyn apeireth_core::kernel::Clock>,
    /// Secret lookup. A module needing an API key resolves it here by logical
    /// name; it never reads a file path or an environment variable directly.
    pub credentials: Arc<dyn crate::credentials::CredentialResolver>,
    /// The trace covering this start-up, so a module's own events correlate with
    /// the boot that caused them.
    pub trace: TraceId,
}

impl ModuleInitContext {
    /// Assemble a context.
    pub fn new(
        clock: Arc<dyn apeireth_core::kernel::Clock>,
        credentials: Arc<dyn crate::credentials::CredentialResolver>,
        trace: TraceId,
    ) -> Self {
        Self {
            clock,
            credentials,
            trace,
        }
    }
}

impl std::fmt::Debug for ModuleInitContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Neither the clock nor the resolver is Debug, and printing the resolver
        // would be a poor idea even if it were.
        f.debug_struct("ModuleInitContext")
            .field("trace", &self.trace)
            .finish_non_exhaustive()
    }
}

/// The one runtime extension entity.
///
/// A module unifies what used to be two traits: the capability-and-lifecycle
/// contract (`Plugin`) and the turn-observation contract (`BehaviorModule`).
/// One module may therefore own all three — lifecycle, hooks, and capabilities
/// — and no registration path cares which combination a given module uses.
///
/// Defaults exist for every member except [`Self::manifest`], so a module
/// implements only what it needs:
///
/// - a capability-only module (a tool bag) needs `manifest` + `tools`;
/// - a hook-only module needs `manifest` + `on_hook`;
/// - a provider module needs `manifest` + `providers` (and usually
///   `initialize`, to capture its credential resolver).
///
/// [`Self::add_tool`] / [`Self::remove_tool`] exist for modules with real
/// dynamic capability ownership (MCP); every other module reports that dynamic
/// registration is unsupported, and the manager rejects the call before it
/// reaches the module anyway.
///
/// This phase supports **static, in-process modules only**. Dynamic library
/// loading, WASM, hot reload, remote modules, and a marketplace are explicitly
/// out of scope; the contract is shaped so they remain possible later, and no
/// part of it is designed around them now.
#[async_trait]
pub trait Module: Send + Sync {
    /// What this module declares about itself. Must be stable across calls.
    fn manifest(&self) -> &ModuleManifest;

    /// Acquire resources and become ready to serve.
    ///
    /// Called at most once, after every declared dependency is active. Returning
    /// an error moves this module to `Failed` and aborts the boot. The default
    /// is a no-op for modules that need no start-up resources; the manager calls
    /// it for every module regardless.
    async fn initialize(&self, _ctx: &ModuleInitContext) -> PluginResult<()> {
        Ok(())
    }

    /// Release resources.
    ///
    /// Called in reverse dependency order. Errors are reported but do not stop
    /// the remaining modules from shutting down.
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    /// Every capability this module can currently dispatch to.
    ///
    /// The single capability surface: the manager reads exports and nothing
    /// else. The default composes [`Self::tools`] and [`Self::providers`], so a
    /// module overrides either of those (or `exports` itself) and never both
    /// spellings of the same fact.
    fn exports(&self) -> ModuleExports {
        ModuleExports::new()
            .with_tools(self.tools())
            .with_providers(self.providers())
    }

    /// How this module's capability set relates to its manifest.
    ///
    /// A derived view of the manifest's `capability_ownership` declaration —
    /// the manager consults the manifest directly and never this method, so
    /// overriding it cannot create a second declaration. Configure ownership
    /// through [`crate::manifest::ModuleManifest::with_capability_ownership`].
    fn capability_ownership(&self) -> CapabilityOwnership {
        self.manifest().capability_ownership
    }

    /// Tool capabilities this module provides.
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        Vec::new()
    }

    /// Provider capabilities this module provides.
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        Vec::new()
    }

    /// Observe a lifecycle point and return transient policy effects.
    ///
    /// Only called for the hook points the manifest declares: declaration is
    /// the dispatch list, and a module that handles a point in here without
    /// declaring it never receives it. Hook calls are sequential and
    /// deterministic. A hook error aborts an in-progress turn and is reported
    /// through [`HookPoint::OnError`] when possible; `AfterTurn` is already
    /// post-commit and `OnError` is best effort. Module directives cannot
    /// execute capabilities or alter a tool invocation.
    async fn on_hook(
        &self,
        _hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        Ok(ModuleOutcome::continue_())
    }

    /// Add one dynamic capability this module owns.
    ///
    /// Only modules with real dynamic ownership (MCP servers discovering tools
    /// at runtime) accept this. The default refuses, and the manager enforces
    /// the policy before it ever asks: a module whose
    /// [`Self::capability_ownership`] is [`CapabilityOwnership::Static`] never
    /// sees the call at all.
    async fn add_tool(&self, _tool: Arc<dyn ToolCapability>) -> PluginResult<()> {
        Err(PluginError::DynamicCapabilityUnsupported {
            module: self.manifest().id.clone(),
        })
    }

    /// Remove one dynamic capability this module owns.
    ///
    /// Returns whether the module actually held it. The default never does.
    async fn remove_tool(&self, _id: &CapabilityId) -> PluginResult<bool> {
        Ok(false)
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
    used: std::sync::atomic::AtomicUsize,
    max: usize,
}

impl ModuleTurnState {
    /// Fresh accounting with `max` side-calls allowed this turn.
    pub fn new(max: usize) -> Self {
        Self {
            used: std::sync::atomic::AtomicUsize::new(0),
            max,
        }
    }

    /// Accounting restored from a suspended continuation, so a resumed turn
    /// keeps the budget it had already spent.
    pub fn with_used(max: usize, used: usize) -> Self {
        Self {
            used: std::sync::atomic::AtomicUsize::new(used),
            max,
        }
    }

    /// How many side-calls this turn has already spent.
    pub fn used(&self) -> usize {
        self.used.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Overwrite the spent count, for a restored continuation.
    pub fn set_used(&self, used: usize) {
        self.used.store(used, std::sync::atomic::Ordering::Relaxed);
    }

    /// Reserve one side-call, or report the budget exhausted.
    pub fn reserve(&self) -> Result<(), ModuleInvocationError> {
        loop {
            let used = self.used.load(std::sync::atomic::Ordering::Relaxed);
            if used >= self.max {
                return Err(ModuleInvocationError::BudgetExceeded { limit: self.max });
            }
            if self
                .used
                .compare_exchange(
                    used,
                    used + 1,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Relaxed,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }
}

/// Reject duplicate capability ids or model-facing names between two tool sets.
///
/// Kept as a free function so a module with its own private capability bag
/// (MCP) can police itself; cross-module uniqueness is enforced once by the
/// module manager, which owns the single capability index.
pub fn reject_tool_identity_collisions(
    incumbent: &[Arc<dyn ToolCapability>],
    challengers: &[Arc<dyn ToolCapability>],
    challenger_owner: &str,
) -> Result<(), String> {
    let mut by_id: BTreeMap<String, String> = BTreeMap::new();
    let mut by_name: BTreeMap<String, String> = BTreeMap::new();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::CapabilityKind;
    use crate::manifest::ModuleManifest;

    struct Echo {
        id: CapabilityId,
        name: String,
    }

    #[async_trait]
    impl ToolCapability for Echo {
        fn id(&self) -> &CapabilityId {
            &self.id
        }
        fn declaration(&self) -> apeireth_protocol::canonical::NormalizedTool {
            apeireth_protocol::canonical::NormalizedTool {
                name: self.name.clone(),
                description: Some("echo".into()),
                parameters: apeireth_protocol::canonical::ToolParameters::new(),
                strict: false,
            }
        }
        async fn invoke(&self, call: &ToolCall) -> apeireth_protocol::canonical::ToolResult {
            apeireth_protocol::canonical::ToolResult::ok(&call.id, call.arguments.clone())
        }
    }

    /// A module that only overrides `tools`: everything else takes its default.
    struct ToolBag {
        manifest: ModuleManifest,
        tool: Arc<Echo>,
    }

    #[async_trait]
    impl Module for ToolBag {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }
        fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
            vec![Arc::clone(&self.tool) as Arc<dyn ToolCapability>]
        }
    }

    /// A module that only observes a hook: lifecycle and exports default away.
    struct Watcher {
        manifest: ModuleManifest,
    }

    #[async_trait]
    impl Module for Watcher {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }
        async fn on_hook(
            &self,
            _hook: HookPoint,
            _ctx: &ModuleContext<'_>,
        ) -> Result<ModuleOutcome, ModuleError> {
            Ok(ModuleOutcome::retry("look again"))
        }
    }

    fn tool_bag(id: &str, capability: &str, name: &str) -> Arc<dyn Module> {
        let manifest = ModuleManifest::system(id, "1.0.0", "test tool bag")
            .declare_capability(
                CapabilityId::new(capability).unwrap(),
                CapabilityKind::Tool,
                "test tool",
            )
            .unwrap();
        Arc::new(ToolBag {
            manifest,
            tool: Arc::new(Echo {
                id: CapabilityId::new(capability).unwrap(),
                name: name.into(),
            }),
        })
    }

    #[tokio::test]
    async fn one_module_carries_lifecycle_hooks_and_capabilities_at_once() {
        let module = tool_bag("builtin.bag", "tool.echo", "echo");
        let ctx = ModuleInitContext::new(
            apeireth_core::kernel::system_clock(),
            Arc::new(crate::credentials::NoCredentials),
            TraceId::new(),
        );

        assert!(module.initialize(&ctx).await.is_ok());
        let exports = module.exports();
        assert_eq!(exports.tools.len(), 1, "capability surface");
        assert_eq!(exports.tools[0].id().as_str(), "tool.echo");

        // A hook-only module observes without exporting; a capability-only
        // module exports without observing. Both are the same trait.
        let watcher: Arc<dyn Module> = Arc::new(Watcher {
            manifest: ModuleManifest::system("builtin.watcher", "1.0.0", "watches"),
        });
        assert!(watcher.exports().is_empty());

        assert!(module.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn a_module_without_dynamic_ownership_refuses_dynamic_tools() {
        let module = tool_bag("builtin.bag", "tool.echo", "echo");
        let tool: Arc<dyn ToolCapability> = Arc::new(Echo {
            id: CapabilityId::new("tool.late").unwrap(),
            name: "late".into(),
        });
        let err = module.add_tool(tool).await.unwrap_err();
        assert!(matches!(
            err,
            PluginError::DynamicCapabilityUnsupported { .. }
        ));
    }

    #[test]
    fn a_declaration_and_an_id_prefix_can_never_disagree_about_kind() {
        // CapabilityDescriptor::new enforces the prefix, which is what lets the
        // manager infer a dynamic tool's kind from its id alone.
        for kind in CapabilityKind::ALL {
            let id = CapabilityId::new(format!("{}.example", kind.id_prefix())).unwrap();
            assert_eq!(CapabilityKind::from_id(&id), Some(kind));
        }
    }

    #[test]
    fn tool_identity_collisions_are_reported_per_reason() {
        let a: Arc<dyn ToolCapability> = Arc::new(Echo {
            id: CapabilityId::new("tool.echo").unwrap(),
            name: "echo".into(),
        });
        let same_id: Arc<dyn ToolCapability> = Arc::new(Echo {
            id: CapabilityId::new("tool.echo").unwrap(),
            name: "echo2".into(),
        });
        let same_name: Arc<dyn ToolCapability> = Arc::new(Echo {
            id: CapabilityId::new("tool.other").unwrap(),
            name: "echo".into(),
        });
        assert!(
            reject_tool_identity_collisions(&[a.clone()], &[same_id], "mcp")
                .unwrap_err()
                .contains("duplicate capability id")
        );
        assert!(reject_tool_identity_collisions(&[a], &[same_name], "mcp")
            .unwrap_err()
            .contains("duplicate tool name"));
    }

    #[test]
    fn every_hook_point_has_a_distinct_name() {
        let names: Vec<String> = HookPoint::ALL.iter().map(|h| h.to_string()).collect();
        let count = names.len();
        let unique: std::collections::BTreeSet<&str> = names.iter().map(String::as_str).collect();
        assert_eq!(unique.len(), count);
    }
}

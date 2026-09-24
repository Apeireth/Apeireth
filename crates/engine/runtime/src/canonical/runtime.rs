//! The runtime kernel's mechanism container.
//!
//! # One place where the system is assembled
//!
//! [`Runtime`] is the single canonical execution container. The production
//! assembly crate, CLI, gateway, desktop, and tests all use the same instance;
//! concrete providers, behaviors, capabilities, and session adapters arrive
//! through explicit ports and registries.
//!
//! # One module world
//!
//! The runtime holds exactly one module registry. Everything a capability needs
//! — ownership, lifecycle, dependency order, dispatch, the tool and provider
//! views — is answered by the single [`ModuleManager`]. There is no second
//! registry for behavior modules, no third for bare tool capabilities, and no
//! bridge that merges two worlds at dispatch time: a tool, a provider, and a
//! turn-observing module are the same kind of object, and
//! [`Runtime::tools`] reports what actually is.
//!
//! # What it borrows, and what it refuses, from `UnifiedRuntimeHost`
//!
//! Borrowed: the idea of *one living runtime* rather than a fresh object graph
//! per request.
//!
//! Refused: everything else about its shape. That type had twenty public fields,
//! constructed `MinimaxAdapter::new()` and `"MiniMax-Text-01"` inline, and stored
//! `api_key: String`. Consequently it could not serve a second vendor, could not
//! be built without a key, and exposed a secret to every `Debug` print.
//!
//! Here: providers arrive as module capabilities, so the
//! runtime names no vendor anywhere; credentials arrive through an injected
//! resolver, so no secret is stored on the struct; and the fields are private,
//! so the surface is the methods rather than the layout.
//!
//! # Booting without keys
//!
//! The default credential resolver resolves nothing. A runtime with no keys
//! configured still builds, still starts its plugins, and still answers
//! questions about its capabilities — it simply cannot complete. Requiring a key
//! to construct the object makes every test that does not need one pay for it.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;

use async_trait::async_trait;
use tokio::sync::Mutex as TokioMutex;

use apeireth_core::kernel::{
    system_clock, ApprovalId, CapabilityId, Clock, ModuleId, SessionId, Timestamp, TraceId,
};
use apeireth_governance::{DenyUnconfigured, GovernanceHook};
use apeireth_plugin::ModuleManager as PluginManager;
use apeireth_plugin::{
    CapabilityKind, CredentialResolver, Module as Plugin, ModuleExports,
    ModuleInitContext as PluginContext, ModuleManager, ModuleManifest, NoCredentials,
    ToolCapability,
};
use apeireth_protocol::canonical::{ModelDescriptor, ModelFeature};
use serde::Serialize;

use super::approval::PendingApprovalView;
use super::context::{ContextProjector, NoContextProjector};
use super::error::{RuntimeError, RuntimeResult};
use super::events::{
    CompositeRuntimeEventSink, NoopRuntimeEventSink, RuntimeEvent, RuntimeEventSink,
};
use super::module::{HookPoint, Module, ModuleRegistry, DEFAULT_MAX_MODULE_INVOCATIONS};
use super::provider::ProviderRouter;
use super::session::{InMemorySessionStore, SessionManager, SessionStore};

/// How many logical execution rounds one turn may take before the runtime stops it.
///
/// This is a structural guard, not a policy: it applies even when no governance
/// is configured. Eight is enough for realistic tool chains and small enough that
/// a model or module stuck in a loop fails fast. Module retries consume a slot.
pub const DEFAULT_MAX_ROUNDS: u32 = 8;

/// Default lifetime of a pending approval before it expires.
pub const DEFAULT_APPROVAL_TTL_MS: u64 = 5 * 60 * 1000;

/// Runtime-wide settings.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Model used when a request does not name one.
    pub default_model: Option<String>,
    /// Logical round limit for one turn, including module retry attempts.
    pub max_rounds: u32,
    /// How long a pending approval stays resumable, in milliseconds.
    pub approval_ttl_ms: u64,
    /// Maximum isolated module provider calls in one top-level turn.
    pub max_module_invocations: usize,
}

/// Secret-free diagnostic projection of the live runtime graph.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSnapshot {
    pub default_model: Option<String>,
    pub max_rounds: u32,
    pub max_module_invocations: usize,
    pub providers: Vec<RuntimeProviderSnapshot>,
    pub models: Vec<RuntimeModelSnapshot>,
    pub behavior_modules: Vec<RuntimeModuleSnapshot>,
    pub capabilities: Vec<RuntimeCapabilitySnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeProviderSnapshot {
    pub id: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<RuntimeHealthSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeHealthSnapshot {
    pub healthy: bool,
    pub latency_p50_ms: u64,
    pub error_rate: f64,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeModelSnapshot {
    pub id: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeModuleSnapshot {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeCapabilitySnapshot {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub available: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            default_model: None,
            max_rounds: DEFAULT_MAX_ROUNDS,
            approval_ttl_ms: DEFAULT_APPROVAL_TTL_MS,
            max_module_invocations: DEFAULT_MAX_MODULE_INVOCATIONS,
        }
    }
}

/// Per-session serialization for turns and approval resolution.
///
/// This is deliberately not a global runtime mutex. Different sessions may
/// proceed concurrently; the same session cannot start a new turn or resolve
/// an approval while another operation on that session is still in progress.
///
/// M20: the map is bounded (`MAX_SESSION_LOCKS`). An unbounded lock map grows
/// linearly with the number of distinct sessions a long-running process ever
/// touched — each entry is small, but the map is never pruned. Entries are
/// only evicted when no waiter holds them (`Arc::strong_count == 1`), so an
/// in-flight turn can never lose its serialization point.
#[derive(Debug, Default)]
pub struct SessionLocks {
    locks: TokioMutex<BTreeMap<SessionId, Arc<TokioMutex<()>>>>,
}

/// 同时驻留的 per-session 锁上限 (M20)。4096 个并发会话对一个本地运行时
/// 已是极端值; 超限时只驱逐"无等待者"的条目。
const MAX_SESSION_LOCKS: usize = 4096;

/// poison 容错读锁 (L 组): 持锁线程 panic 只使 `PoisonError` 被标记, 锁
/// 保护的数据并未损坏。事件 sink / 能力注册表这类非关键路径不应把一次
/// panic 放大为之后所有操作的级联 panic。
fn read_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// poison 容错写锁 (L 组): 同上。
fn write_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl SessionLocks {
    pub(crate) async fn acquire(&self, session: SessionId) -> Arc<TokioMutex<()>> {
        let mut map = self.locks.lock().await;
        if let Some(existing) = map.get(&session) {
            return Arc::clone(existing);
        }
        // 无等待者 (仅 map 自己持有一份 Arc) 才可驱逐: 正在 hold 或等待该
        // 锁的任务不会因为淘汰而拿到另一个互斥体。
        if map.len() >= MAX_SESSION_LOCKS {
            let evictable = map
                .iter()
                .find(|(_, lock)| Arc::strong_count(lock) == 1)
                .map(|(id, _)| *id);
            if let Some(evictable) = evictable {
                map.remove(&evictable);
            }
        }
        let entry = map
            .entry(session)
            .or_insert_with(|| Arc::new(TokioMutex::new(())));
        Arc::clone(entry)
    }
}

/// The assembled runtime.
pub struct Runtime {
    /// The one canonical module registry: ownership, lifecycle, dependency
    /// order, capability index, hooks. Nothing else holds a second copy of any
    /// of those facts.
    pub(super) modules: ModuleManager,
    // Arc so per-turn invoker handles can carry the one canonical router
    // without borrowing the runtime. Still exactly one router instance.
    pub(super) providers: Arc<ProviderRouter>,
    pub(super) sessions: SessionManager,
    pub(super) governance: Arc<dyn GovernanceHook>,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) config: RuntimeConfig,
    pub(super) session_locks: SessionLocks,
    pub(super) event_sink: RwLock<Arc<dyn RuntimeEventSink>>,
    pub(super) context_projector: Arc<dyn ContextProjector>,
    /// Remote capability execution/query port (mesh). Absent when this runtime
    /// stands alone: behavior is then identical to a pre-port runtime.
    ///
    /// Behind a lock and settable after build (`set_remote_port`) because the
    /// port implementation (a mesh node) is itself constructed over this
    /// runtime — the cycle is broken the same way `set_event_sink` does it.
    pub(super) remote_port: RwLock<Option<Arc<dyn apeireth_mesh::RemoteCapabilityPort>>>,
}

impl Runtime {
    /// Start building a runtime.
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::new()
    }

    /// The module manager, and through it the one canonical registry pair.
    ///
    /// Compatibility spelling: the module manager is the only manager there is.
    pub fn plugins(&self) -> &PluginManager {
        &self.modules
    }

    /// The module manager: lifecycle, dependency order, capability index.
    pub fn module_manager(&self) -> &ModuleManager {
        &self.modules
    }

    /// The session manager.
    pub fn sessions(&self) -> &SessionManager {
        &self.sessions
    }

    /// The governance hook every action is checked against.
    pub fn governance(&self) -> &Arc<dyn GovernanceHook> {
        &self.governance
    }

    /// The runtime's time source.
    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }

    /// Runtime settings.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Return a live, secret-free projection for diagnostics and capability
    /// discovery. It never includes credentials, prompts, tool arguments, or
    /// provider metadata that may contain vendor-specific secrets.
    pub fn snapshot(&self) -> RuntimeSnapshot {
        let providers = self
            .providers
            .provider_ids()
            .into_iter()
            .map(|id| {
                let health = self.providers.health(&id);
                RuntimeProviderSnapshot {
                    id: id.to_string(),
                    available: health.as_ref().is_none_or(|value| value.healthy),
                    health: health.map(|value| RuntimeHealthSnapshot {
                        healthy: value.healthy,
                        latency_p50_ms: value.latency_p50_ms,
                        error_rate: value.error_rate,
                        consecutive_failures: value.consecutive_failures,
                    }),
                }
            })
            .collect();
        let models = self
            .providers
            .model_descriptors()
            .into_iter()
            .map(|model: ModelDescriptor| RuntimeModelSnapshot {
                id: model.id.to_string(),
                provider: model.provider.to_string(),
                display_name: model.display_name,
                context_window: model.context_window,
                max_output_tokens: model.max_output_tokens,
                features: model
                    .features
                    .into_iter()
                    .map(|feature| match feature {
                        ModelFeature::ToolCalls => "tool_calls",
                        ModelFeature::Streaming => "streaming",
                        ModelFeature::Vision => "vision",
                        ModelFeature::SystemPrompt => "system_prompt",
                        ModelFeature::StructuredOutput => "structured_output",
                    })
                    .map(str::to_string)
                    .collect(),
            })
            .collect();
        let behavior_modules = self
            .modules()
            .iter()
            .map(|module| RuntimeModuleSnapshot {
                id: module.manifest().id.to_string(),
                name: module.manifest().name.clone(),
            })
            .collect();
        // One source: every dispatchable tool with its owning module. There is
        // no second world to merge in, so nothing is double-counted and nothing
        // is silently dropped from the listing.
        let capabilities = self
            .modules
            .tool_entries()
            .into_iter()
            .map(|(owner, capability)| RuntimeCapabilitySnapshot {
                id: capability.id().to_string(),
                name: capability.declaration().name,
                owner: owner.to_string(),
                available: true,
            })
            .collect::<Vec<_>>();
        RuntimeSnapshot {
            default_model: self.config.default_model.clone(),
            max_rounds: self.config.max_rounds,
            max_module_invocations: self.config.max_module_invocations,
            providers,
            models,
            behavior_modules,
            capabilities,
        }
    }

    /// Every tool that can currently be dispatched to.
    ///
    /// A single view over the one capability index — the runtime no longer
    /// merges a module world with a plugin world, so a tool cannot appear twice
    /// or be missing because it arrived through the "other" path.
    pub fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.modules.active_tools()
    }

    /// Every dispatchable tool together with its owning module.
    ///
    /// The honest listing for diagnostics and admin surfaces: what exists, and
    /// who owns it, from the one source of truth.
    pub fn tool_entries(&self) -> Vec<(ModuleId, Arc<dyn ToolCapability>)> {
        self.modules.tool_entries()
    }

    /// Which modules declare that they observe each hook point.
    ///
    /// A derived view over the manifests, not a second registry.
    pub fn hook_index(&self) -> std::collections::BTreeMap<HookPoint, Vec<ModuleId>> {
        self.modules.hook_index()
    }

    /// Install (or replace) the remote capability port after build.
    ///
    /// The port implementation is typically a mesh node constructed over this
    /// very runtime, so injection happens after assembly; a runtime that never
    /// calls this stays entirely local.
    pub fn set_remote_port(&self, port: Arc<dyn apeireth_mesh::RemoteCapabilityPort>) {
        *self
            .remote_port
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(port);
    }

    /// The remote capability port, when one is installed.
    pub(super) fn remote_port(
        &self,
    ) -> std::sync::RwLockReadGuard<'_, Option<Arc<dyn apeireth_mesh::RemoteCapabilityPort>>> {
        self.remote_port
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Replace the host event sink without changing the runtime graph.
    ///
    /// This is used by adapters that are assembled after the runtime. Event
    /// delivery is observational and never changes the turn result.
    pub fn set_event_sink(&self, sink: Arc<dyn RuntimeEventSink>) {
        // poison 容错 (L 组): 持锁线程 panic 后, 事件 sink 本身并未损坏;
        // 直接 expect 会把一次 sink panic 升级为之后所有 set/emit 的全线崩溃。
        *write_lock_or_recover(&self.event_sink) = sink;
    }

    /// Add an observer while preserving sinks installed by another
    /// composition root. Gateway setup uses this so dataset/audit observers
    /// attached during runtime bootstrap are not silently discarded.
    pub fn add_event_sink(&self, sink: Arc<dyn RuntimeEventSink>) {
        let existing = read_lock_or_recover(&self.event_sink).clone();
        *write_lock_or_recover(&self.event_sink) =
            Arc::new(CompositeRuntimeEventSink::new(vec![existing, sink]));
    }

    pub(crate) fn emit_event(&self, event: RuntimeEvent) {
        read_lock_or_recover(&self.event_sink).emit(event);
    }

    /// Register a dynamic tool on a named module after build.
    ///
    /// The one path by which a capability can appear after boot. The module
    /// manager rejects the tool unless its capability id and model-facing name
    /// are unique across every module and the named module genuinely owns
    /// dynamic capability — there is no second registry for it to slip into.
    pub async fn register_dynamic_tool(
        &mut self,
        module_id: &str,
        tool: Arc<dyn ToolCapability>,
    ) -> Result<(), RuntimeError> {
        let owner = ModuleId::new(module_id)
            .map_err(|error| RuntimeError::misconfigured(error.to_string()))?;
        self.modules
            .register_dynamic_tool(&owner, tool)
            .await
            .map_err(RuntimeError::Plugin)
    }

    /// Remove one dynamic tool from a named module after build.
    ///
    /// Returns whether anything was actually removed.
    pub async fn unregister_dynamic_tool(
        &mut self,
        module_id: &str,
        capability_id: &apeireth_core::kernel::CapabilityId,
    ) -> Result<bool, RuntimeError> {
        let owner = ModuleId::new(module_id)
            .map_err(|error| RuntimeError::misconfigured(error.to_string()))?;
        self.modules
            .unregister_dynamic_tool(&owner, capability_id)
            .await
            .map_err(RuntimeError::Plugin)
    }

    /// Model-facing tool declarations for all active tools.
    ///
    /// The unified model catalog: local declarations (canonical index) plus
    /// remote declarations (capability port projection), with contested names
    /// excluded. With no port installed this is exactly the local list, in the
    /// same order.
    pub fn tool_declarations(&self) -> Vec<apeireth_protocol::canonical::NormalizedTool> {
        self.tool_route_table().model_catalog()
    }

    /// Rebuild the ephemeral name → target routing table from the two
    /// read-only views. Derived per query; nothing is cached.
    pub(super) fn tool_route_table(&self) -> super::tool_route::ToolRouteTable {
        use super::tool_route::LocalToolView;
        // Local candidates in the canonical id order the previous
        // `tool_declarations` used, so no-mesh output is byte-identical.
        let local: Vec<LocalToolView> = self
            .modules
            .active_tools()
            .into_iter()
            .filter_map(|tool| {
                let owner = self.modules.capabilities().owner(tool.id()).ok()?.clone();
                Some(LocalToolView {
                    owner,
                    capability_id: tool.id().clone(),
                    declaration: tool.declaration(),
                })
            })
            .collect();
        let remote = self
            .remote_port
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|port| port.remote_tools())
            .unwrap_or_default();
        super::tool_route::ToolRouteTable::derive(local, remote)
    }

    /// The configured context projector.
    pub fn context_projector(&self) -> &Arc<dyn ContextProjector> {
        &self.context_projector
    }

    /// The router over every provider that can currently serve a completion.
    ///
    /// Its members come from the one capability index, which is why the runtime
    /// can name no vendor: it knows only that some module exports
    /// `provider.something`.
    pub fn providers(&self) -> &ProviderRouter {
        &self.providers
    }

    /// The shared router behind an [`Arc`](std::sync::Arc), for turn-scoped
    /// invoker construction. This is the same single router instance the
    /// runtime serves every completion from.
    pub(super) fn providers_arc(&self) -> Arc<ProviderRouter> {
        Arc::clone(&self.providers)
    }

    /// The modules registered with this runtime, in registration order.
    pub fn modules(&self) -> Vec<&Arc<dyn Module>> {
        self.modules.modules().modules()
    }

    /// The canonical module registry: ownership, lifecycle, capability index.
    pub fn module_registry(&self) -> &ModuleRegistry {
        self.modules.modules()
    }

    /// Pending approvals waiting on a human for one session.
    ///
    /// This is a read projection of session-owned governance state. It never
    /// executes a tool and never mutates policy.
    pub async fn pending_approvals(
        &self,
        session: SessionId,
    ) -> RuntimeResult<Vec<PendingApprovalView>> {
        let Some(loaded) = self.sessions.load(&session).await? else {
            return Ok(Vec::new());
        };
        let now = Timestamp::from_clock(self.clock.as_ref());
        Ok(loaded
            .approvals
            .values()
            .filter(|approval| approval.status.is_pending() && !approval.is_expired(now))
            .map(PendingApprovalView::from)
            .collect())
    }

    /// Stop every module in reverse start order.
    ///
    /// Returns the failures encountered; shutdown continues past each one.
    pub async fn shutdown(&mut self) -> Vec<apeireth_plugin::PluginError> {
        self.modules.shutdown_all().await
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("modules", &self.modules.modules().len())
            .field("capabilities", &self.modules.capabilities().len())
            .field("providers", &self.providers.len())
            .field("governance", &self.governance.name())
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

/// Assembles a [`Runtime`].
///
/// Every dependency has a working default, so the smallest useful runtime is
/// `Runtime::builder().build().await`. Defaults are inert rather than
/// surprising: no credentials, fail-closed capability dispatch, memory-backed
/// sessions. Completions remain allowed so a zero-module kernel can still chat.
///
/// There is one registration path for everything: [`Self::with_module`].
/// [`Self::with_plugin`] is the historical spelling of the same call, and
/// [`Self::with_capability`] wraps loose tool capabilities in one system module
/// so that they still enter the same registry, lifecycle, and capability index
/// as any other module instead of living in a registry of their own.
pub struct RuntimeBuilder {
    clock: Arc<dyn Clock>,
    credentials: Arc<dyn CredentialResolver>,
    session_store: Arc<dyn SessionStore>,
    governance: Arc<dyn GovernanceHook>,
    modules: Vec<Arc<dyn Module>>,
    capabilities: Vec<Arc<dyn ToolCapability>>,
    event_sink: Arc<dyn RuntimeEventSink>,
    context_projector: Arc<dyn ContextProjector>,
    fallback_order: Option<Vec<CapabilityId>>,
    config: RuntimeConfig,
    remote_port: Option<Arc<dyn apeireth_mesh::RemoteCapabilityPort>>,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBuilder {
    /// A builder with default dependencies.
    pub fn new() -> Self {
        Self {
            clock: system_clock(),
            credentials: Arc::new(NoCredentials),
            session_store: Arc::new(InMemorySessionStore::new()),
            governance: Arc::new(DenyUnconfigured),
            modules: Vec::new(),
            capabilities: Vec::new(),
            event_sink: Arc::new(NoopRuntimeEventSink),
            context_projector: Arc::new(NoContextProjector),
            fallback_order: None,
            config: RuntimeConfig::default(),
            remote_port: None,
        }
    }

    /// Use a specific time source.
    ///
    /// Supplying a virtual clock is what makes a turn's timing reproducible.
    #[must_use]
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Supply secrets to plugins at start-up.
    #[must_use]
    pub fn with_credentials(mut self, credentials: Arc<dyn CredentialResolver>) -> Self {
        self.credentials = credentials;
        self
    }

    /// Use a specific session backend.
    #[must_use]
    pub fn with_session_store(mut self, store: Arc<dyn SessionStore>) -> Self {
        self.session_store = store;
        self
    }

    /// Check every action against this hook.
    #[must_use]
    pub fn with_governance(mut self, governance: Arc<dyn GovernanceHook>) -> Self {
        self.governance = governance;
        self
    }

    /// Add a module. Order is irrelevant for start-up, which comes from declared
    /// dependencies; registration order is preserved wherever dependencies do
    /// not force otherwise, including hook dispatch.
    #[must_use]
    pub fn with_module(mut self, module: Arc<dyn Module>) -> Self {
        self.modules.push(module);
        self
    }

    /// Compatibility spelling of [`Self::with_module`].
    ///
    /// A plugin was never a second runtime primitive — it was always a module
    /// with a different name. This entry point feeds the same registry.
    #[must_use]
    pub fn with_plugin(mut self, plugin: Arc<dyn Plugin>) -> Self {
        self.modules.push(plugin);
        self
    }

    /// Register a dispatchable capability without a module of its own.
    ///
    /// The capability is owned by one system module ([`CAPABILITY_MODULE_ID`])
    /// that is registered through the ordinary path: same manifest schema, same
    /// lifecycle, same capability index. A capability cannot exist in this
    /// runtime without an owner.
    #[must_use]
    pub fn with_capability(mut self, capability: Arc<dyn ToolCapability>) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Compatibility spelling for tool capability registration.
    #[must_use]
    pub fn with_tool(mut self, capability: Arc<dyn ToolCapability>) -> Self {
        self.with_capability(capability)
    }

    /// Send canonical lifecycle events to a host-owned sink.
    #[must_use]
    pub fn with_event_sink(mut self, sink: Arc<dyn RuntimeEventSink>) -> Self {
        self.event_sink = sink;
        self
    }

    /// Project transcript messages immediately before provider requests.
    #[must_use]
    pub fn with_context_projector(mut self, projector: Arc<dyn ContextProjector>) -> Self {
        self.context_projector = projector;
        self
    }

    ///
    /// Providers absent from `order` remain usable and are tried after every
    /// listed one. Without this, providers are tried in registration order.
    #[must_use]
    pub fn with_fallback_order(mut self, order: Vec<CapabilityId>) -> Self {
        self.fallback_order = Some(order);
        self
    }

    /// Model used when a request does not name one.
    #[must_use]
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.config.default_model = Some(model.into());
        self
    }

    /// Round limit for one turn.
    #[must_use]
    pub fn with_max_rounds(mut self, rounds: u32) -> Self {
        self.config.max_rounds = rounds;
        self
    }

    /// Pending-approval lifetime, in milliseconds.
    #[must_use]
    pub fn with_approval_ttl(mut self, ttl_ms: u64) -> Self {
        self.config.approval_ttl_ms = ttl_ms;
        self
    }

    /// Route model tool calls across the mesh through this remote capability
    /// port. Without it the runtime is entirely local, byte-equivalent to a
    /// pre-port build.
    #[must_use]
    pub fn with_remote_port(mut self, port: Arc<dyn apeireth_mesh::RemoteCapabilityPort>) -> Self {
        self.remote_port = Some(port);
        self
    }

    /// Set the per-turn budget for isolated module provider calls.
    #[must_use]
    pub fn with_max_module_invocations(mut self, invocations: usize) -> Self {
        self.config.max_module_invocations = invocations;
        self
    }

    /// Register every module, start them in dependency order, and assemble.
    ///
    /// Modules are started here rather than lazily on first use, so that a
    /// misconfiguration surfaces at boot rather than inside somebody's first
    /// request. All validation — duplicate module ids, duplicate capability ids,
    /// duplicate model-facing tool names, missing dependencies, dependency
    /// cycles, declared-but-unimplemented capabilities — happens before the
    /// first `initialize` runs.
    pub async fn build(self) -> RuntimeResult<Runtime> {
        if self.config.max_rounds == 0 {
            return Err(RuntimeError::misconfigured(
                "max_rounds must be at least 1, otherwise no turn can ever run",
            ));
        }
        if self.config.max_module_invocations == 0 {
            return Err(RuntimeError::misconfigured(
                "max_module_invocations must be at least 1",
            ));
        }

        // One registry for everything. Registration order (modules, then the
        // compatibility plugin entries land in the same list as modules, then
        // the loose-capability system module) is what start order and hook
        // dispatch follow wherever no declared dependency forces otherwise.
        let mut manager = ModuleManager::new();
        for module in self.modules {
            manager.register(module).map_err(RuntimeError::Plugin)?;
        }
        if !self.capabilities.is_empty() {
            let owned = RuntimeCapabilityModule::new(self.capabilities)?;
            manager
                .register(Arc::new(owned))
                .map_err(RuntimeError::Plugin)?;
        }

        let ctx = PluginContext::new(
            Arc::clone(&self.clock),
            Arc::clone(&self.credentials),
            TraceId::new(),
        );
        manager.start_all(&ctx).await?;

        // Providers are read out of the capability index *after* start-up, so
        // the router contains exactly those whose modules actually came up. A
        // provider whose module failed to initialize is never routed to.
        let mut providers =
            ProviderRouter::new(manager.active_providers(), Arc::clone(&self.clock));
        if let Some(order) = self.fallback_order {
            providers = providers.with_fallback_order(order);
        }

        Ok(Runtime {
            modules: manager,
            providers: Arc::new(providers),
            sessions: SessionManager::new(self.session_store, Arc::clone(&self.clock)),
            governance: self.governance,
            clock: self.clock,
            config: self.config,
            session_locks: SessionLocks::default(),
            event_sink: RwLock::new(self.event_sink),
            context_projector: self.context_projector,
            remote_port: RwLock::new(self.remote_port),
        })
    }
}

/// Module id of the system module that owns capabilities registered without a
/// module of their own.
pub const CAPABILITY_MODULE_ID: &str = "builtin.runtime.capabilities";

/// The system module that owns loose tool capabilities.
///
/// Registering a bare `ToolCapability` is a convenience for embedders and
/// tests; it is not a second registry. The capability is still owned by a
/// module with a real manifest, which is why the manifest declares each
/// capability it exports and why the ordinary uniqueness checks apply to them.
struct RuntimeCapabilityModule {
    manifest: ModuleManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

impl RuntimeCapabilityModule {
    /// Take ownership of `tools`, declaring each one in the manifest.
    ///
    /// Fails loudly rather than dropping anything: a capability that cannot be
    /// declared is a configuration error, and silently omitting it would make
    /// the tool vanish with no explanation.
    fn new(tools: Vec<Arc<dyn ToolCapability>>) -> RuntimeResult<Self> {
        let mut manifest = ModuleManifest::system(
            CAPABILITY_MODULE_ID,
            env!("CARGO_PKG_VERSION"),
            "Runtime-registered tool capabilities (system origin)",
        )
        .with_name("runtime capabilities");
        let mut owned = Vec::with_capacity(tools.len());
        for tool in tools {
            let id = tool.id().clone();
            if CapabilityKind::from_id(&id) != Some(CapabilityKind::Tool) {
                // A tool capability must carry the conventional `tool.*` prefix
                // so the index classifies it exactly like a declared one.
                return Err(RuntimeError::misconfigured(format!(
                    "capability {id} registered as a tool must use the tool.* prefix"
                )));
            }
            let description = tool
                .declaration()
                .description
                .unwrap_or_else(|| "runtime-registered tool capability".to_string());
            manifest = manifest
                .declare_capability(id, CapabilityKind::Tool, description)
                .map_err(RuntimeError::Plugin)?;
            owned.push(tool);
        }
        Ok(Self {
            manifest,
            tools: owned,
        })
    }
}

#[async_trait]
impl Module for RuntimeCapabilityModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn exports(&self) -> ModuleExports {
        ModuleExports::new().with_tools(self.tools.clone())
    }
}

/// Module ids known to a runtime, in id order. Convenience for diagnostics.
///
/// Historical spelling kept for callers that still say "plugin".
pub fn plugin_ids(runtime: &Runtime) -> Vec<&ModuleId> {
    runtime.module_manager().modules().ids().collect()
}

/// Module ids known to a runtime, in id order.
pub fn module_ids(runtime: &Runtime) -> Vec<&ModuleId> {
    runtime.module_manager().modules().ids().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{Lifecycle, Timestamp, VirtualClock};
    use apeireth_governance::{DenyCapabilities, GovernancePipeline};
    use apeireth_plugin::{
        CapabilityKind, PluginManifest, PluginResult, StaticCredentials, ToolCapability,
    };
    use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct Echo(CapabilityId);

    #[async_trait]
    impl ToolCapability for Echo {
        fn id(&self) -> &CapabilityId {
            &self.0
        }
        fn declaration(&self) -> NormalizedTool {
            NormalizedTool {
                name: "echo".into(),
                description: Some("echo".into()),
                parameters: ToolParameters::new(),
                strict: false,
            }
        }
        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            ToolResult::ok(&call.id, call.arguments.clone())
        }
    }

    /// Captures whatever credential it was offered at start-up.
    struct KeyReader {
        manifest: PluginManifest,
        seen: Mutex<Option<String>>,
    }

    #[async_trait]
    impl Plugin for KeyReader {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }
        async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
            *self.seen.lock().unwrap() = ctx
                .credentials
                .resolve("provider.fake.api_key")
                .map(|s| s.expose().to_string());
            Ok(())
        }
        async fn shutdown(&self) -> PluginResult<()> {
            Ok(())
        }
        fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
            vec![Arc::new(Echo(CapabilityId::new("tool.echo").unwrap()))]
        }
    }

    fn key_reader() -> Arc<KeyReader> {
        Arc::new(KeyReader {
            manifest: PluginManifest::new(
                ModuleId::new("builtin.key_reader").unwrap(),
                "1.0.0",
                "reads a key at boot",
            )
            .declare_capability(
                CapabilityId::new("tool.echo").unwrap(),
                CapabilityKind::Tool,
                "echo",
            )
            .unwrap(),
            seen: Mutex::new(None),
        })
    }

    #[tokio::test]
    async fn the_smallest_runtime_builds_with_no_arguments() {
        let runtime = Runtime::builder().build().await.unwrap();
        assert_eq!(runtime.module_manager().modules().len(), 0);
        assert!(runtime.tools().is_empty());
        assert!(runtime.providers().is_empty());
        assert_eq!(runtime.config().max_rounds, DEFAULT_MAX_ROUNDS);
    }

    #[tokio::test]
    async fn a_runtime_builds_and_boots_without_any_credentials() {
        let plugin = key_reader();
        let runtime = Runtime::builder()
            .with_plugin(plugin.clone())
            .build()
            .await
            .unwrap();

        assert_eq!(
            runtime
                .module_manager()
                .state(&ModuleId::new("builtin.key_reader").unwrap())
                .unwrap(),
            Lifecycle::Active,
            "no key must not prevent boot"
        );
        assert_eq!(*plugin.seen.lock().unwrap(), None);
        assert_eq!(runtime.tools().len(), 1);
    }

    #[tokio::test]
    async fn credentials_reach_plugins_through_the_injected_resolver() {
        let plugin = key_reader();
        let _runtime = Runtime::builder()
            .with_plugin(plugin.clone())
            .with_credentials(Arc::new(
                StaticCredentials::new().with("provider.fake.api_key", "sk-injected"),
            ))
            .build()
            .await
            .unwrap();

        assert_eq!(
            plugin.seen.lock().unwrap().as_deref(),
            Some("sk-injected"),
            "the plugin must receive the key without the runtime storing it"
        );
    }

    #[tokio::test]
    async fn the_runtime_never_prints_a_secret() {
        let runtime = Runtime::builder()
            .with_plugin(key_reader())
            .with_credentials(Arc::new(
                StaticCredentials::new().with("provider.fake.api_key", "sk-injected"),
            ))
            .build()
            .await
            .unwrap();

        let printed = format!("{runtime:?}");
        assert!(!printed.contains("sk-injected"), "{printed}");
    }

    #[tokio::test]
    async fn the_injected_clock_is_the_one_sessions_are_stamped_with() {
        let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ));
        let runtime = Runtime::builder().with_clock(clock).build().await.unwrap();

        let session = runtime
            .sessions()
            .load_or_create(apeireth_core::kernel::SessionId::new())
            .await
            .unwrap();
        assert_eq!(session.created_at.epoch_millis(), 1_700_000_000_000);
    }

    #[tokio::test]
    async fn a_zero_round_limit_is_rejected_at_build() {
        let err = Runtime::builder()
            .with_max_rounds(0)
            .build()
            .await
            .unwrap_err();
        assert!(matches!(err, RuntimeError::Misconfigured(_)), "{err}");
    }

    #[tokio::test]
    async fn a_duplicate_plugin_is_rejected_at_build() {
        let err = Runtime::builder()
            .with_plugin(key_reader())
            .with_plugin(key_reader())
            .build()
            .await
            .unwrap_err();
        assert!(matches!(err, RuntimeError::Plugin(_)), "{err}");
    }

    #[tokio::test]
    async fn governance_is_held_as_configured() {
        let runtime = Runtime::builder()
            .with_governance(Arc::new(GovernancePipeline::new().with(Arc::new(
                DenyCapabilities::new().deny(CapabilityId::new("tool.example").unwrap()),
            ))))
            .build()
            .await
            .unwrap();
        assert_eq!(runtime.governance().name(), "pipeline");
    }

    #[tokio::test]
    async fn default_governance_is_fail_closed_for_capabilities() {
        let runtime = Runtime::builder().build().await.unwrap();
        assert_eq!(runtime.governance().name(), "deny_unconfigured");
    }

    #[tokio::test]
    async fn shutdown_stops_the_plugins_it_started() {
        let mut runtime = Runtime::builder()
            .with_plugin(key_reader())
            .build()
            .await
            .unwrap();
        let id = ModuleId::new("builtin.key_reader").unwrap();

        assert_eq!(
            runtime.module_manager().state(&id).unwrap(),
            Lifecycle::Active
        );
        assert!(runtime.shutdown().await.is_empty());
        assert_eq!(
            runtime.module_manager().state(&id).unwrap(),
            Lifecycle::Stopped
        );
    }

    #[tokio::test]
    async fn plugin_ids_are_reported_in_id_order() {
        let runtime = Runtime::builder()
            .with_plugin(key_reader())
            .build()
            .await
            .unwrap();
        let ids: Vec<&str> = plugin_ids(&runtime).iter().map(|i| i.as_str()).collect();
        assert_eq!(ids, ["builtin.key_reader"]);
    }

    #[tokio::test]
    async fn a_loose_capability_is_owned_by_the_system_capability_module() {
        // The `with_capability`/`with_tool` compat entries do not open a second
        // registry: the loose tool is owned by one system module, registered
        // through the ordinary path with an ordinary manifest.
        let runtime = Runtime::builder()
            .with_tool(
                Arc::new(Echo(CapabilityId::new("tool.echo").unwrap())) as Arc<dyn ToolCapability>
            )
            .build()
            .await
            .unwrap();
        let owner = runtime
            .module_manager()
            .capabilities()
            .owner(&CapabilityId::new("tool.echo").unwrap())
            .unwrap();
        assert_eq!(owner.as_str(), CAPABILITY_MODULE_ID);
        assert_eq!(runtime.tools().len(), 1);
    }

    #[tokio::test]
    async fn compatibility_entries_collide_in_the_one_capability_index() {
        // `with_plugin` and `with_tool` both feed the one manager: a loose
        // capability re-declaring a plugin-owned id must be rejected at build
        // against the same capability index — possible only if there is one.
        let result = Runtime::builder()
            .with_plugin(key_reader())
            .with_tool(
                Arc::new(Echo(CapabilityId::new("tool.echo").unwrap())) as Arc<dyn ToolCapability>
            )
            .build()
            .await;
        let Err(error) = result else {
            panic!("a loose duplicate of a plugin-owned capability must be rejected");
        };
        assert!(matches!(error, RuntimeError::Plugin(_)), "{error}");
    }
}

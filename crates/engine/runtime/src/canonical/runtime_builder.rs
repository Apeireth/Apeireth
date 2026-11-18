//! Runtime assembly.
//!
//! The builder lives beside the container it assembles: the struct, its
//! defaults, and every `with_*` port stay verbatim what they were, but the
//! file that holds them no longer competes with the container for room.
//!
//! Nothing here changes behavior. The builder still assembles exactly the
//! runtime [`Runtime`] describes, and the container still reaches it through
//! [`Runtime::builder`].

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use tokio::sync::Mutex as TokioMutex;

use apeireth_core::agent_contact::ContactRegistry;
use apeireth_core::kernel::{
    system_clock, ApprovalId, CapabilityId, Clock, PluginId, SessionId, Timestamp, TraceId,
};
use apeireth_governance::{DenyUnconfigured, GovernanceHook};
use apeireth_orchestration::call_scheduler::CallScheduler;
use apeireth_orchestration::compaction_checkpoint::{CompactionEngine, SummaryGenerator};
use apeireth_orchestration::context_budget::SpillWriter;
use apeireth_orchestration::repetition_advisory::{
    RepetitionAdvisory, RepetitionDetector, RepetitionPolicy,
};
use apeireth_orchestration::runtime_invariants::InvariantAuditor;
use apeireth_plugin::{
    CredentialResolver, NoCredentials, Plugin, PluginContext, PluginManager, ToolCapability,
    ToolCompletionFact, ToolCompletionObserver,
};
use apeireth_protocol::canonical::{ModelDescriptor, ModelFeature};

use super::capability::CapabilityRegistry;
use super::context::{ContextProjector, NoContextProjector};
use super::error::{RuntimeError, RuntimeResult};
use super::events::{
    CompositeRuntimeEventSink, NoopRuntimeEventSink, RuntimeEvent, RuntimeEventSink,
};
use super::module::{BehaviorModule, BehaviorRegistry, DEFAULT_MAX_MODULE_INVOCATIONS};
use super::provider::ProviderRouter;
use super::runtime::{Runtime, RuntimeConfig};
use super::session::{InMemorySessionStore, SessionManager, SessionStore};
use super::session_locks::SessionLocks;
use super::workspace::WorkspaceRegistry;

/// Assembles a [`Runtime`].
///
/// Every dependency has a working default, so the smallest useful runtime is
/// `Runtime::builder().build().await`. Defaults are inert rather than
/// surprising: no credentials, fail-closed capability dispatch, memory-backed
/// sessions. Completions remain allowed so a zero-module kernel can still chat.
pub struct RuntimeBuilder {
    clock: Arc<dyn Clock>,
    credentials: Arc<dyn CredentialResolver>,
    session_store: Arc<dyn SessionStore>,
    governance: Arc<dyn GovernanceHook>,
    plugins: Vec<Arc<dyn Plugin>>,
    modules: Vec<Arc<dyn BehaviorModule>>,
    principal_authorizations: Option<Arc<super::principal_authority::PrincipalAuthorizations>>,
    capabilities: Vec<Arc<dyn ToolCapability>>,
    event_sink: Arc<dyn RuntimeEventSink>,
    context_projector: Arc<dyn ContextProjector>,
    fallback_order: Option<Vec<CapabilityId>>,
    config: RuntimeConfig,
    dispatch_limits: super::dispatcher::DispatchLimits,
    invariant_auditor: Option<Arc<InvariantAuditor>>,
    summary_generator: Option<Arc<dyn SummaryGenerator>>,
    call_scheduler: Option<Arc<CallScheduler>>,
    session_manager: Option<Arc<SessionManager>>,
    /// 通用工具完成观察口（不注入 = 零观测 = 零行为变化）。
    tool_completion_observer: Option<Arc<dyn ToolCompletionObserver>>,
    /// 出网闸（不注入 = 默认放行 = 零行为变化）。
    egress_gate: Option<Arc<dyn super::egress_gate::EgressGate>>,
    system_one: Option<Arc<super::system_one::SystemOneGate>>,
    pub(super) agent_factory: Option<Arc<dyn apeireth_plugin::agents::ModuleAgentFactory>>,
    pub(super) v1_protocol: bool,
    /// Remote capability port (mesh). Absent = this runtime stays local.
    remote_port: Option<Arc<dyn apeireth_mesh::RemoteCapabilityPort>>,
    /// Contact registry the turn-admission gate reads. Absent = no registry was
    /// assembled; an unbound session still runs, a bound one is refused.
    contact_registry: Option<Arc<dyn ContactRegistry>>,
    /// Workspace registry the turn-admission gate reads. Absent = same fail
    /// closed rule as [`Self::contact_registry`].
    workspace_registry: Option<Arc<dyn WorkspaceRegistry>>,
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
            plugins: Vec::new(),
            modules: Vec::new(),
            principal_authorizations: None,
            capabilities: Vec::new(),
            event_sink: Arc::new(NoopRuntimeEventSink),
            context_projector: Arc::new(NoContextProjector),
            fallback_order: None,
            config: RuntimeConfig::default(),
            dispatch_limits: super::dispatcher::DispatchLimits::default(),
            invariant_auditor: None,
            summary_generator: None,
            call_scheduler: None,
            session_manager: None,
            tool_completion_observer: None,
            egress_gate: None,
            system_one: None,
            agent_factory: None,
            v1_protocol: false,
            remote_port: None,
            contact_registry: None,
            workspace_registry: None,
        }
    }

    /// Install the contact registry the turn-admission gate validates against.
    ///
    /// The composition root passes **the same instance** it hands to the
    /// contract surface, so a binding written over HTTP is the fact the gate
    /// reads. `None` keeps the historical behavior for unbound sessions and
    /// makes bound ones fail closed.
    #[must_use]
    pub fn with_contact_registry(mut self, registry: Option<Arc<dyn ContactRegistry>>) -> Self {
        self.contact_registry = registry;
        self
    }

    /// Install the workspace registry the turn-admission gate validates against.
    ///
    /// Same sharing rule as [`Self::with_contact_registry`].
    #[must_use]
    pub fn with_workspace_registry(mut self, registry: Option<Arc<dyn WorkspaceRegistry>>) -> Self {
        self.workspace_registry = registry;
        self
    }

    /// Select a mandatory supervisor service for enrolled internal execution.
    /// An absent/unbound service refuses enrolled effects; compatibility defaults stay unchanged.
    #[must_use]
    pub fn with_system_one(mut self, gate: Arc<super::system_one::SystemOneGate>) -> Self {
        self.system_one = Some(gate);
        self
    }

    /// Whether a mandatory supervisor is currently bound.
    ///
    /// Read-only probe for composition roots that must decide **before** building
    /// whether a feature which needs enrolled internal execution can be armed at
    /// all (the dispatcher refuses `Internal`/`Protected` tools with
    /// `SystemOneUnavailable` when no supervisor is bound). It never binds one.
    #[must_use]
    pub fn system_one_configured(&self) -> bool {
        self.system_one.is_some()
    }

    #[must_use]
    pub fn with_principal_authorizations(
        mut self,
        authority: Arc<super::principal_authority::PrincipalAuthorizations>,
    ) -> Self {
        self.principal_authorizations = Some(authority);
        self
    }

    #[must_use]
    pub fn with_dispatch_limits(mut self, limits: super::dispatcher::DispatchLimits) -> Self {
        self.dispatch_limits = limits;
        self
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

    /// Use an already-assembled [`SessionManager`] (W3-① 域平面共用写闸):
    ///
    /// 与 [`RuntimeBuilder::with_session_store`] 二选一; 两者都在场时本方法生效。
    /// 用途: 组合根先组装默认用户域的写闸, 再把**同一把**交给运行时与域平面 ——
    /// 每域恰一把会话写闸 (W2-③ 同会话串行承诺不拆)。
    #[must_use]
    pub fn with_session_manager(mut self, manager: Arc<SessionManager>) -> Self {
        self.session_manager = Some(manager);
        self
    }

    /// Check every action against this hook.
    #[must_use]
    pub fn with_governance(mut self, governance: Arc<dyn GovernanceHook>) -> Self {
        self.governance = governance;
        self
    }

    /// Add a plugin. Order is irrelevant; start order comes from declared
    /// dependencies.
    #[must_use]
    pub fn with_plugin(mut self, plugin: Arc<dyn Plugin>) -> Self {
        self.plugins.push(plugin);
        self
    }

    /// Elect the typed public/private contract of already-selected tool modules.
    ///
    /// `election` is the assembly's auditable table (module id → contract): an id
    /// absent from it keeps the compatibility contract verbatim, and a module that
    /// already carries a non-default contract is left untouched, so an explicit
    /// election always wins and repeated application is idempotent. Election only
    /// restricts a surface (`module_visible` / `model_visible` / effect class); it
    /// never grants authority and never changes a wire name or a frozen payload.
    pub fn with_module_contracts(
        mut self,
        election: &[(&str, apeireth_plugin::module_contract::ModuleToolContract)],
    ) -> RuntimeResult<Self> {
        use apeireth_plugin::module_contract::ModuleToolContract;
        for slot in &mut self.plugins {
            let id = slot.manifest().id.as_str();
            let Some((_, contract)) = election.iter().find(|(elected, _)| *elected == id) else {
                continue;
            };
            if slot
                .tools()
                .iter()
                .any(|tool| tool.module_contract() != ModuleToolContract::default())
            {
                continue;
            }
            let inner = Arc::clone(slot);
            *slot = apeireth_plugin::tool_module::elect_module_contract(inner, contract)
                .map_err(RuntimeError::misconfigured)?;
        }
        Ok(self)
    }

    /// Register a cognitive module through authenticated fixed Hooks and V1 tools.
    /// Modules run in registration order. The Host must grant their internal
    /// capabilities explicitly; registration does not authorize execution.
    #[must_use]
    pub fn with_registered_behavior(mut self, module: Arc<dyn BehaviorModule>) -> Self {
        self.modules.push(module);
        self
    }

    /// Compatibility name for [`Self::with_registered_behavior`].
    #[must_use]
    pub fn with_module(self, module: Arc<dyn BehaviorModule>) -> Self {
        self.with_registered_behavior(module)
    }

    /// Compatibility alias that makes the behavior/capability distinction
    /// explicit at composition sites.
    #[must_use]
    pub fn with_behavior(self, module: Arc<dyn BehaviorModule>) -> Self {
        self.with_module(module)
    }

    /// Register a dispatchable capability independently of behavior hooks.
    #[must_use]
    pub fn with_capability(mut self, capability: Arc<dyn ToolCapability>) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Compatibility spelling for tool capability registration.
    #[must_use]
    pub fn with_tool(mut self, capability: Arc<dyn ToolCapability>) -> Self {
        self.capabilities.push(capability);
        self
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

    /// 注入出网闸（ADR-0014 §1.2：把上下文发给模型就是行动）。不注入 = 默认放行。
    #[must_use]
    pub fn with_egress_gate(mut self, gate: Arc<dyn super::egress_gate::EgressGate>) -> Self {
        self.egress_gate = Some(gate);
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

    /// Cap the tool calls one round may dispatch (clamped to at least 1).
    #[must_use]
    pub fn with_max_tool_calls_per_round(mut self, calls: usize) -> Self {
        self.config.max_tool_calls_per_round = calls;
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

    /// Set the total character budget for the assembled injected-context blocks.
    ///
    /// Small values force aggressive trimming of non-core injected context
    /// (memory / organ / lesson blocks) while core identity / system / safety
    /// blocks are never truncated. See [`DEFAULT_CONTEXT_BUDGET_CHARS`].
    #[must_use]
    pub fn with_context_budget_chars(mut self, chars: usize) -> Self {
        self.config.context_budget_chars = chars;
        self
    }

    /// Bind the data directory that keeps the full originals of truncated
    /// injected-context long tails (under `root/spill/`).
    ///
    /// With a sink bound, a long-tail cut becomes a head/tail preview with a
    /// retrieval guide pointing at the spilled original. Without one, the
    /// overflow path keeps the full content inline: a successful call is never
    /// turned into an error, and content is never silently lost.
    #[must_use]
    pub fn with_context_spill_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.config.context_spill_root = Some(root.into());
        self
    }

    /// Set the repeated-identical-call advisory policy (thresholds, exclusion
    /// list, preview budget). The channel is advisory only and never blocks.
    #[must_use]
    pub fn with_repetition_policy(mut self, policy: RepetitionPolicy) -> Self {
        self.config.repetition_advisory = policy;
        self
    }

    /// Enable the plan-mode collaborative-state projection.
    ///
    /// The mechanism is opt-in so a default runtime is unchanged. Once
    /// enabled, the `exit_plan_mode` control is resident in the tool catalog
    /// in both postures and switching postures alters only the prompt /
    /// behavior-convention projection — never the tool catalog, sandbox
    /// posture, or approval decisions.
    #[must_use]
    pub fn with_plan_mode_enabled(mut self, enabled: bool) -> Self {
        self.config.plan_mode_enabled = enabled;
        self
    }

    /// Install a domain-neutral observer of completed tool calls.
    ///
    /// The observer receives exactly one [`ToolCompletionFact`] per executed
    /// call, after the canonical `ToolFailed` session event and
    /// `CapabilityCompleted` trace entry have landed. It is observational only:
    /// the runtime ignores anything it sees, so results, events, retries, and
    /// approvals are unchanged. Without an observer the completion bookkeeping
    /// takes no extra lock and writes no extra state (byte-for-byte identical).
    ///
    /// Domain owners implement the trait in [`apeireth_plugin::observation`];
    /// the kernel never names a domain crate or a concrete observer.
    #[must_use]
    pub fn with_tool_completion_observer(
        mut self,
        observer: Arc<dyn ToolCompletionObserver>,
    ) -> Self {
        self.tool_completion_observer = Some(observer);
        self
    }

    /// Set the session-scoped cognitive quota ceilings (opt-in spend gate).
    ///
    /// `None` on a dimension = no limit; consumption metering runs either way.
    /// See [`super::quota::SessionQuotaLimits`] for the exhausted-behavior
    /// contract.
    #[must_use]
    pub fn with_session_quota(mut self, quota: super::quota::SessionQuotaLimits) -> Self {
        self.config.quota = quota;
        self
    }

    /// Install the unit-price source behind the cost dimension of the session
    /// quota ([`super::quota::CostRateTable`]).
    ///
    /// Without it every provider round is counted as unpriced (the cost total
    /// is honestly partial; nothing is ever invented). With it, each round is
    /// converted at the round model's unit price onto the session ledger at
    /// the round metering point.
    #[must_use]
    pub fn with_cost_rates(mut self, rates: Arc<dyn super::quota::CostRateTable>) -> Self {
        self.config.cost_rates = Some(rates);
        self
    }
    /// Install the runtime-invariant consumption point.
    ///
    /// Without it, no invariant checking happens at the dispatch boundary.
    #[must_use]
    pub fn with_invariant_auditor(mut self, auditor: Arc<InvariantAuditor>) -> Self {
        self.invariant_auditor = Some(auditor);
        self
    }

    /// Install the summary generator behind compress checkpoints.
    ///
    /// Without it, no compaction ever runs: the derived provider view equals
    /// the transcript exactly. With it, a transcript that crosses the shared
    /// overflow trigger is surface-replaced by one accepted summary — and any
    /// summary failure leaves the session exactly as it was.
    #[must_use]
    pub fn with_summary_generator(mut self, generator: Arc<dyn SummaryGenerator>) -> Self {
        self.summary_generator = Some(generator);
        self
    }

    /// Install the parallel-call classification scheduler.
    ///
    /// Without it, every tool call dispatches strictly serially. With it, only
    /// calls the scheduler's classifier explicitly allows may overlap inside a
    /// bounded rolling pool; everything else keeps serial dispatch semantics,
    /// and results always commit in model order.
    #[must_use]
    pub fn with_call_scheduler(mut self, scheduler: Arc<CallScheduler>) -> Self {
        self.call_scheduler = Some(scheduler);
        self
    }

    /// Register the plugins, start them in dependency order, and assemble.
    ///
    /// Plugins are started here rather than lazily on first use, so that a
    /// misconfiguration surfaces at boot rather than inside somebody's first
    /// request.
    pub async fn build(self) -> RuntimeResult<Runtime> {
        let host = self.build_agent_runtime_compat().await?;
        let mut slot = host.inner.kernel.write().await;
        let view = slot.execution_view();
        let mut runtime = std::mem::replace(&mut *slot, view);
        drop(slot);
        runtime.instance_host = Some(host);
        Ok(runtime)
    }

    pub(super) async fn build_kernel(mut self) -> RuntimeResult<Runtime> {
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
        let mut registered_behaviors = BTreeMap::new();
        let mut module_registry = BehaviorRegistry::new();
        for module in self.modules {
            let adapter = Arc::new(
                super::behavior_adapter::RegisteredBehavior::new(module.clone())
                    .map_err(RuntimeError::misconfigured)?,
            );
            self.plugins.push(adapter.clone());
            registered_behaviors.insert(module.manifest().id.clone(), adapter);
            module_registry
                .register(module)
                .map_err(RuntimeError::misconfigured)?;
        }

        let capability_registry = CapabilityRegistry::new();
        // Internal control commands are registered, not branched on in the
        // execution loop. Plan mode is the first (and currently only) one; a
        // runtime with the mechanism off registers nothing at all.
        if self.config.plan_mode_enabled {
            capability_registry
                .register_control(Arc::new(
                    super::plan_mode_control::PlanModeExitControl::new(),
                ))
                .map_err(RuntimeError::misconfigured)?;
        }

        let mut manager = PluginManager::new();
        for plugin in self.plugins {
            manager.register(plugin)?;
        }

        let mut ctx = PluginContext::new(
            Arc::clone(&self.clock),
            Arc::clone(&self.credentials),
            TraceId::new(),
        );
        if let Some(factory) = self.agent_factory {
            ctx = ctx.with_agent_factory(factory);
        }
        let declared_tools = manager
            .plugins()
            .ids()
            .flat_map(|id| manager.plugins().get(id).expect("registered").tools())
            .collect::<Vec<_>>();
        crate::canonical::module::reject_tool_identity_collisions(
            &self.capabilities,
            &declared_tools,
            "plugin tools",
        )
        .map_err(RuntimeError::misconfigured)?;
        crate::canonical::control::reject_control_tool_collisions(
            &capability_registry.controls(),
            &declared_tools,
        )
        .map_err(RuntimeError::misconfigured)?;
        crate::canonical::control::reject_control_tool_collisions(
            &capability_registry.controls(),
            &self.capabilities,
        )
        .map_err(RuntimeError::misconfigured)?;
        if !self.capabilities.is_empty() {
            manager.register(Arc::new(
                apeireth_plugin::ToolModule::new(
                    PluginId::new("host.legacy.tools").expect("static module id"),
                    self.capabilities,
                )?
                .with_legacy_events(),
            ))?;
        }
        manager.start_all_or_rollback(&ctx).await?;
        super::module_host::populate_tool_catalog(&capability_registry, &mut manager).await?;

        // Providers are read out of the capability registry *after* start-up, so
        // the router contains exactly those whose plugins actually came up. A
        // provider whose plugin failed to initialize is never routed to.
        let mut providers =
            ProviderRouter::new(manager.active_providers(), Arc::clone(&self.clock));
        providers
            .accounting
            .set_rates(self.config.cost_rates.clone());
        if let Some(gate) = self.egress_gate.clone() {
            providers = providers.with_egress_gate(gate);
        }
        if let Some(order) = self.fallback_order {
            providers = providers.with_fallback_order(order);
        }

        let metrics = Arc::new(super::metrics::MetricsRegistry::new(
            self.config.metrics_window_turns,
        ));

        let context_spill = self
            .config
            .context_spill_root
            .clone()
            .map(|root| Arc::new(SpillWriter::new(root)));

        // W3-①: 注入的会话写闸优先 (与域平面共用同一把); 未注入则按 store 自建。
        let sessions = match self.session_manager {
            Some(manager) => manager,
            None => Arc::new(SessionManager::new(
                self.session_store,
                Arc::clone(&self.clock),
            )),
        };

        let dispatcher = Arc::new(
            super::dispatcher::ToolDispatcher::new(self.dispatch_limits)
                .with_usage(providers.accounting.clone())
                .with_system_one(self.system_one, self.principal_authorizations.clone()),
        );
        Ok(Runtime {
            instances: Default::default(),
            message_invocation: None,
            instance_host: None,
            task: None,
            plugins: manager,
            v1_protocol: self.v1_protocol,
            providers: Arc::new(providers),
            sessions,
            governance: self.governance,
            clock: self.clock,
            config: self.config,
            modules: module_registry,
            principal_authorizations: self.principal_authorizations,
            registered_behaviors,
            capabilities: capability_registry,
            dispatcher,
            session_locks: Arc::new(SessionLocks::default()),
            event_sink: Arc::new(RwLock::new(self.event_sink)),
            context_projector: self.context_projector,
            repetition_detectors: Arc::new(Mutex::new(BTreeMap::new())),
            invariant_auditor: self.invariant_auditor,
            context_spill,
            compaction: self
                .summary_generator
                .map(|generator| Arc::new(CompactionEngine::new(generator))),
            call_scheduler: self.call_scheduler,
            metrics,
            operation_idempotency: super::operation_idempotency::OperationIdempotency::new(
                super::operation_idempotency::DEFAULT_OPERATION_CAPACITY,
            ),
            tool_completion_observer: self.tool_completion_observer,
            remote_port: RwLock::new(self.remote_port),
            contact_registry: self.contact_registry,
            workspace_registry: self.workspace_registry,
        })
    }
}

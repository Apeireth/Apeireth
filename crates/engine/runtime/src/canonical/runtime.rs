//! The runtime kernel's mechanism container.
//!
//! # One place where the system is assembled
//!
//! [`Runtime`] is the single canonical execution container. The production
//! assembly crate, CLI, gateway, desktop, and tests all use the same instance;
//! concrete providers, behaviors, capabilities, and session adapters arrive
//! through explicit ports and registries.
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
//! Here: providers arrive as capabilities from the plugin registry, so the
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
use serde::Serialize;

use super::approval::PendingApprovalView;
use super::capability::CapabilityRegistry;
use super::context::{ContextProjector, NoContextProjector};
use super::error::{RuntimeError, RuntimeResult};
use super::events::{
    CompositeRuntimeEventSink, NoopRuntimeEventSink, RuntimeEvent, RuntimeEventSink,
};
use super::module::{BehaviorModule, BehaviorRegistry, DEFAULT_MAX_MODULE_INVOCATIONS};
use super::provider::ProviderRouter;
pub use super::runtime_builder::RuntimeBuilder;
use super::session::{InMemorySessionStore, SessionManager, SessionStore};
use super::session_locks::SessionLocks;
use super::workspace::WorkspaceRegistry;

/// How many logical execution rounds one turn may take before the runtime stops it.
///
/// This is a structural guard, not a policy: it applies even when no governance
/// is configured. Eight is enough for realistic tool chains and small enough that
/// a model or module stuck in a loop fails fast. Module retries consume a slot.
pub const DEFAULT_MAX_ROUNDS: u32 = 8;

/// Smallest per-turn round limit a configuration value is clamped to.
pub const MIN_TURN_ROUNDS: u32 = 1;

/// Largest per-turn round limit a configuration value is clamped to.
pub const MAX_TURN_ROUNDS: u32 = 64;

/// Smallest per-round tool-call limit a configuration value is clamped to.
pub const MIN_TOOL_CALL_LIMIT: usize = 1;

/// Largest per-round tool-call limit a configuration value is clamped to.
pub const MAX_TOOL_CALL_LIMIT: usize = 64;

/// Resolve the per-turn round limit from a raw configuration value.
///
/// Budget knobs are explicit: an out-of-range integer is clamped into
/// `MIN_TURN_ROUNDS..=MAX_TURN_ROUNDS`, while an absent, empty, or
/// unparseable value falls back to [`DEFAULT_MAX_ROUNDS`]. Nothing else is
/// invented.
pub fn parse_turn_round_limit(raw: Option<&str>) -> u32 {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<i64>().ok())
        .map(|value| value.clamp(i64::from(MIN_TURN_ROUNDS), i64::from(MAX_TURN_ROUNDS)) as u32)
        .unwrap_or(DEFAULT_MAX_ROUNDS)
}

/// Resolve the per-round tool-call limit from a raw configuration value.
///
/// Same explicit-budget contract as [`parse_turn_round_limit`]: out-of-range
/// values clamp into `MIN_TOOL_CALL_LIMIT..=MAX_TOOL_CALL_LIMIT`, absent or
/// unparseable values fall back to the default of 16.
pub fn parse_tool_call_limit(raw: Option<&str>) -> usize {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<i64>().ok())
        .map(|value| value.clamp(MIN_TOOL_CALL_LIMIT as i64, MAX_TOOL_CALL_LIMIT as i64) as usize)
        .unwrap_or(super::execute::MAX_TOOL_CALLS_PER_ROUND)
}

/// Default lifetime of a pending approval before it expires.
pub const DEFAULT_APPROVAL_TTL_MS: u64 = 5 * 60 * 1000;

/// Default total character budget for the assembled injected-context blocks.
///
/// The injected context (memory-injection evidence, organ products, reflexion
/// lessons and the like) is assembled as named blocks under one total budget:
/// core blocks (identity / system-convention / safety) are never truncated,
/// per-block caps apply before the total, and non-core overflow is cut greedily
/// from the longest block first. 24 000 characters is a deliberate ~6 k-token
/// ceiling at a conservative ~4 characters/token for mixed prose (CJK packs
/// fewer characters per token, so this stays on the safe side), leaving ample
/// head-room in a typical context window for the conversation transcript and
/// the reply. It is a soft guard on injected context only — not a substitute
/// for a model's context-window limit, which the context projector owns.
pub const DEFAULT_CONTEXT_BUDGET_CHARS: usize = 24_000;

/// Runtime-wide settings.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Model used when a request does not name one.
    pub default_model: Option<String>,
    /// Logical round limit for one turn, including module retry attempts.
    pub max_rounds: u32,
    /// Maximum tool calls one round may dispatch. Calls beyond the limit in a
    /// single round are truncated with a recorded trace entry and answered
    /// with synthetic results, so every call in the assistant message still
    /// has exactly one answer.
    pub max_tool_calls_per_round: usize,
    /// How long a pending approval stays resumable, in milliseconds.
    pub approval_ttl_ms: u64,
    /// Maximum isolated module provider calls in one top-level turn.
    pub max_module_invocations: usize,
    /// Total character budget for the assembled injected-context blocks (core
    /// blocks are never truncated; see [`DEFAULT_CONTEXT_BUDGET_CHARS`]).
    pub context_budget_chars: usize,
    /// Data directory whose `spill/` subdirectory receives the full originals
    /// of truncated injected-context long tails. `None` binds no spill sink;
    /// long-tail cuts then keep the full content inline rather than lose it.
    pub context_spill_root: Option<PathBuf>,
    /// Policy for the repeated-identical-call advisory channel: thresholds,
    /// exclusion list, and preview budget. Purely advisory; it never blocks.
    pub repetition_advisory: RepetitionPolicy,
    /// Enable the plan-mode collaborative-state projection: a session posture
    /// folded from the session event log whose only effect is the
    /// prompt/behavior-convention projection. When enabled, the `exit_plan_mode`
    /// control is resident in the tool catalog in *both* postures, so switching
    /// postures changes the prompt projection and nothing else. Default `false`:
    /// a runtime that never engages the mechanism is byte-for-byte unchanged.
    pub plan_mode_enabled: bool,
    /// Session-scoped cognitive quota ceilings (opt-in spend gate). `None` on a
    /// dimension means no limit; the metering itself always runs so the
    /// self-report's consumption column stays real. See
    /// [`super::quota`] for the accounting and exhausted-behavior contract.
    pub quota: super::quota::SessionQuotaLimits,
    /// Unit-price source for the cost dimension of the session quota
    /// ([`super::quota::CostRateTable`]). Every provider round whose model has
    /// a row here is converted onto the ledger's cost column at the round
    /// metering point; a model without a row is counted as an unpriced round
    /// instead (never invented). `None` = no price source installed, so every
    /// round is unpriced and the cost total is honestly partial.
    pub cost_rates: Option<Arc<dyn super::quota::CostRateTable>>,
    /// 滚动窗口大小 (G-P1 指标面, 拍/层): 指标注册表每层保留最近 N 个回合
    /// 收束拍 (进程内环形, 不落盘)。默认 [`super::metrics::DEFAULT_METRICS_WINDOW_TURNS`]。
    pub metrics_window_turns: usize,
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
            max_tool_calls_per_round: super::execute::MAX_TOOL_CALLS_PER_ROUND,
            approval_ttl_ms: DEFAULT_APPROVAL_TTL_MS,
            max_module_invocations: DEFAULT_MAX_MODULE_INVOCATIONS,
            context_budget_chars: DEFAULT_CONTEXT_BUDGET_CHARS,
            context_spill_root: None,
            repetition_advisory: RepetitionPolicy::default(),
            plan_mode_enabled: false,
            quota: super::quota::SessionQuotaLimits::default(),
            cost_rates: None,
            metrics_window_turns: super::metrics::DEFAULT_METRICS_WINDOW_TURNS,
        }
    }
}

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

/// The assembled runtime.
pub struct Runtime {
    pub(super) instances: std::sync::Weak<super::instances::Inner>,
    pub(super) message_invocation: Option<Arc<super::module_messages::MessageInvocation>>,
    pub(super) instance_host: Option<super::instances::AgentRuntime>,
    pub(super) task: Option<Arc<super::instances::TaskExecution>>,
    pub(super) plugins: PluginManager,
    pub(super) v1_protocol: bool,
    // Arc so per-turn invoker handles can carry the one canonical router
    // without borrowing the runtime. Still exactly one router instance.
    pub(super) providers: Arc<ProviderRouter>,
    // W3-① Arc 形态: 默认用户域的会话写闸与域平面 (`DomainSessionPlane`) 共用
    // **同一把** SessionManager —— 每域恰一把写闸 (W2-③ 同会话串行承诺不拆)。
    pub(super) sessions: Arc<SessionManager>,
    pub(super) governance: Arc<dyn GovernanceHook>,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) config: RuntimeConfig,
    pub(super) modules: BehaviorRegistry,
    pub(super) principal_authorizations:
        Option<Arc<super::principal_authority::PrincipalAuthorizations>>,
    pub(super) registered_behaviors:
        BTreeMap<String, Arc<super::behavior_adapter::RegisteredBehavior>>,
    pub(super) capabilities: CapabilityRegistry,
    pub(super) dispatcher: Arc<super::dispatcher::ToolDispatcher>,
    pub(super) session_locks: Arc<SessionLocks>,
    pub(super) event_sink: Arc<RwLock<Arc<dyn RuntimeEventSink>>>,
    pub(super) context_projector: Arc<dyn ContextProjector>,
    /// Session-scoped streak state for the repeated-identical-call advisory
    /// channel. Advisory only: it never blocks and never decides anything.
    pub(super) repetition_detectors: Arc<Mutex<BTreeMap<SessionId, RepetitionDetector>>>,
    /// Optional runtime-invariant consumption point (observation layer). When
    /// absent, no invariant checking happens at the dispatch boundary.
    pub(super) invariant_auditor: Option<Arc<InvariantAuditor>>,
    /// Full-original overflow store for truncated injected-context long tails
    /// (`<root>/spill/`). `None` = no sink bound; overflow handling then keeps
    /// full content inline instead of losing it.
    pub(super) context_spill: Option<Arc<SpillWriter>>,
    /// Compress-checkpoint engine (summary replacement over an append-only
    /// transcript). `None` = no summary generator installed; the derived view
    /// then equals the transcript exactly and nothing is ever compacted.
    pub(super) compaction: Option<Arc<CompactionEngine>>,
    /// Parallel-call classification scheduler (mutual-exclusion barriers over a
    /// bounded rolling pool, model-order result commit). `None` = every tool
    /// call dispatches strictly serially, one at a time.
    pub(super) call_scheduler: Option<Arc<CallScheduler>>,
    /// G-P1 指标面: 回合收束指标的进程内滚动窗口 (会话 + 全局两层),回合
    /// 收束埋点写这里, `/v1/panel/metrics` 从这里读 (见 [`super::metrics`])。
    pub(super) metrics: Arc<super::metrics::MetricsRegistry>,
    /// C2-1 操作幂等表: (session, operation) 作用域内的终态回放/冲突裁决,
    /// 进程内、重启即失 (见 [`super::operation_idempotency`])。
    pub(super) operation_idempotency: super::operation_idempotency::OperationIdempotency,
    /// Optional domain-neutral observer of completed tool calls.
    ///
    /// The kernel emits one [`ToolCompletionFact`] at the single completion
    /// bookkeeping point (serial dispatch, parallel commit, cancel drain).
    /// With no observer installed, completion bookkeeping is exactly its
    /// historical two records — no extra lock, no domain state.
    pub(super) tool_completion_observer: Option<Arc<dyn ToolCompletionObserver>>,
    /// Remote capability execution/query port (mesh). Absent when this runtime
    /// stands alone: behavior is then identical to a pre-port runtime.
    ///
    /// Behind a lock and settable after build (`set_remote_port`) because the
    /// port implementation (a mesh node) is itself constructed over this
    /// runtime — the cycle is broken by late injection, the same way the
    /// event sink is injected after assembly.
    pub(super) remote_port: RwLock<Option<Arc<dyn apeireth_mesh::RemoteCapabilityPort>>>,
    /// Contact registry the turn-admission gate reads, when one is installed.
    ///
    /// The instance is the same one the contract surface writes through, so a
    /// binding written over HTTP is the very fact the gate validates. `None`
    /// means no registry was assembled: an unbound session still runs, and a
    /// bound one is refused (fail closed).
    pub(super) contact_registry: Option<Arc<dyn ContactRegistry>>,
    /// Workspace registry the turn-admission gate reads, when one is installed.
    ///
    /// Shared with the contract surface for the same reason as
    /// [`Self::contact_registry`].
    pub(super) workspace_registry: Option<Arc<dyn WorkspaceRegistry>>,
}

impl Runtime {
    /// Start building a runtime.
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::new()
    }

    /// The plugin manager, and through it both canonical registries.
    pub fn plugins(&self) -> &PluginManager {
        &self.plugins
    }

    /// The session manager.
    pub fn sessions(&self) -> &SessionManager {
        &self.sessions
    }

    /// The session manager as a shared handle (W3-①): 组合根把它注册进域平面,
    /// 让默认用户域的会话面与运行时共用同一把写闸 (同一实例, 非同一存储的两把闸)。
    pub fn sessions_arc(&self) -> &Arc<SessionManager> {
        &self.sessions
    }

    /// The contact registry the turn-admission gate reads.
    ///
    /// Shared handle: the caller gets the same instance the runtime validates
    /// bindings against, so a fact written through the contract surface is the
    /// fact the gate sees. `None` means no registry was assembled.
    pub fn contact_registry(&self) -> Option<Arc<dyn ContactRegistry>> {
        self.contact_registry.clone()
    }

    /// The workspace registry the turn-admission gate reads.
    ///
    /// Shared handle, same contract as [`Self::contact_registry`].
    pub fn workspace_registry(&self) -> Option<Arc<dyn WorkspaceRegistry>> {
        self.workspace_registry.clone()
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

    /// 指标注册表 (G-P1 指标数据源): 回合收束指标的进程内滚动窗口。
    pub fn metrics(&self) -> &super::metrics::MetricsRegistry {
        &self.metrics
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
                id: module.manifest().id.clone(),
                name: module.manifest().name.clone(),
            })
            .collect();
        let capabilities = self
            .capabilities
            .entries()
            .into_iter()
            .map(|(owner, capability)| RuntimeCapabilitySnapshot {
                id: capability.id().to_string(),
                name: capability.declaration().name,
                owner,
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

    /// The single executable catalog for managed and compatibility capabilities.
    pub fn capability_registry(&self) -> &CapabilityRegistry {
        &self.capabilities
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
    /// A user message entered this session: any repetition streak is cleared.
    pub(crate) fn note_user_message(&self, session: SessionId) {
        let mut map = self.lock_repetition_detectors();
        map.entry(session)
            .or_insert_with(|| RepetitionDetector::new(self.config.repetition_advisory.clone()))
            .observe_user_message();
    }

    /// Observe one executed tool call and return a repetition advisory when
    /// the consecutive-identical streak crosses a configured threshold.
    ///
    /// This is the advisory channel only: the call has already run when this is
    /// consulted, and the returned text never changes the tool result.
    pub(crate) fn note_tool_call(
        &self,
        session: SessionId,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Option<RepetitionAdvisory> {
        let mut map = self.lock_repetition_detectors();
        map.entry(session)
            .or_insert_with(|| RepetitionDetector::new(self.config.repetition_advisory.clone()))
            .observe_tool_call(tool_name, arguments)
    }

    /// Drop a session's repetition state at a turn boundary.
    pub(crate) fn clear_repetition_state(&self, session: SessionId) {
        self.lock_repetition_detectors().remove(&session);
    }

    fn lock_repetition_detectors(
        &self,
    ) -> std::sync::MutexGuard<'_, BTreeMap<SessionId, RepetitionDetector>> {
        self.repetition_detectors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The runtime-invariant auditor, when one was configured. The auditor is
    /// the consumption point for audit-event streams; it observes and (in
    /// fail-fast mode) blocks, but never changes a governance verdict.
    pub fn invariant_auditor(&self) -> Option<&Arc<InvariantAuditor>> {
        self.invariant_auditor.as_ref()
    }

    /// Register a dynamic tool on a named module after build.
    ///
    /// The tool is rejected if its capability id or model-facing name collides
    /// with any already-visible module or plugin tool.
    pub fn register_dynamic_tool(
        &self,
        module_id: &str,
        tool: Arc<dyn ToolCapability>,
    ) -> Result<(), RuntimeError> {
        if let Some(host) = self.agent_runtime() {
            return host
                .inner
                .kernel
                .try_read()
                .map_err(|_| RuntimeError::misconfigured("control plane is busy"))?
                .register_dynamic_tool(module_id, tool);
        }
        let visible_plugins = self.plugins.active_tools();
        crate::canonical::module::reject_tool_identity_collisions(
            &visible_plugins,
            std::slice::from_ref(&tool),
            module_id,
        )
        .map_err(RuntimeError::misconfigured)?;
        self.capabilities
            .register(module_id, tool)
            .map_err(RuntimeError::misconfigured)
    }

    /// Model-facing tool declarations for all active tools.
    ///
    /// The unified model catalog: every local tool in the order the
    /// registry-driven list has always been built in, then the control
    /// declarations, then the remote declarations projected through the
    /// capability port, with contested names excluded. With no port installed
    /// this is byte-identical to the local list, in the same order.
    ///
    /// The resident `exit_plan_mode` control arrives through the control
    /// declarations: the runtime registers it under exactly the same
    /// plan-mode posture rule it always used (registered only when the
    /// plan-mode mechanism is on), so this merge does not change the
    /// catalog in either posture.
    pub fn tool_declarations(&self) -> Vec<apeireth_protocol::canonical::NormalizedTool> {
        let routes = self.tool_route_table();
        let mut declarations = self.local_tool_declarations();
        declarations.retain(|tool| !routes.is_ambiguous(&tool.name));
        declarations.extend(routes.remote_model_catalog());
        declarations
    }

    /// The local, registry-driven half of the model-facing catalog.
    ///
    /// This is the list the runtime has always published locally: the tool
    /// declarations of the capability registry, then the control
    /// declarations. Session scopes restrict both local and remote entries
    /// by capability identity after name collisions have been excluded.
    pub(super) fn local_tool_declarations(
        &self,
    ) -> Vec<apeireth_protocol::canonical::NormalizedTool> {
        let mut declarations: Vec<apeireth_protocol::canonical::NormalizedTool> = self
            .capabilities
            .declarations()
            .into_iter()
            .collect::<Vec<_>>();
        // Control declarations are part of the one registry-driven catalog.
        declarations.extend(self.capabilities.control_declarations());
        declarations
    }

    /// Rebuild the ephemeral name -> target routing table from the two
    /// read-only views. Derived per query; nothing is cached.
    ///
    /// Registry tools and current peer projections are routing candidates.
    /// Registered control names stay local: peer adverts cannot shadow them.
    pub(super) fn tool_route_table(&self) -> super::tool_route::ToolRouteTable {
        use super::tool_route::LocalToolView;
        let local: Vec<LocalToolView> = self
            .capabilities
            .entries()
            .into_iter()
            .filter_map(|(owner, tool)| {
                Some(LocalToolView {
                    owner: PluginId::new(owner.as_str()).ok()?,
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
            .unwrap_or_default()
            .into_iter()
            .filter(|tool| {
                self.capabilities
                    .find_control_by_name(&tool.declaration.name)
                    .is_none()
            })
            .collect();
        super::tool_route::ToolRouteTable::derive(local, remote)
    }

    /// The configured context projector.
    pub fn context_projector(&self) -> &Arc<dyn ContextProjector> {
        &self.context_projector
    }

    /// 已安装的并发调度器（`None` = 未安装 ⇒ 并行工具派发不会发生）。
    ///
    /// 暴露给装配级测试与自述面：并行窗口的可用性取决于**是否装上了**
    /// [`apeireth_orchestration::call_scheduler::CallScheduler`]，
    /// 「装上了」这件事应当可被外部验证，而不是只能读装配代码。
    pub fn call_scheduler(
        &self,
    ) -> Option<&Arc<apeireth_orchestration::call_scheduler::CallScheduler>> {
        self.call_scheduler.as_ref()
    }

    /// The router over every provider that can currently serve a completion.
    ///
    /// Its members come from the capability registry, which is why the runtime
    /// can name no vendor: it knows only that some plugin declared
    /// `provider.something`.
    pub fn providers(&self) -> &ProviderRouter {
        &self.providers
    }

    /// The modules registered with this runtime, in execution order.
    pub fn modules(&self) -> &[Arc<dyn BehaviorModule>] {
        self.modules.modules()
    }

    /// The module registry holding all registered modules.
    pub fn module_registry(&self) -> &BehaviorRegistry {
        &self.modules
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

    /// Stop every plugin in reverse start order.
    ///
    /// Returns the failures encountered; shutdown continues past each one.
    pub async fn shutdown(&mut self) -> Vec<apeireth_plugin::PluginError> {
        if let Some(host) = &self.instance_host {
            if host.has_managed_main() {
                return match Box::pin(host.shutdown()).await {
                    Ok(()) => Vec::new(),
                    Err(error) => vec![apeireth_plugin::PluginError::shutdown_failed(
                        PluginId::new("host.main").expect("static owner"),
                        error.to_string(),
                    )],
                };
            }
            if let Err(error) = host.drain_instances().await {
                return vec![apeireth_plugin::PluginError::shutdown_failed(
                    apeireth_core::kernel::PluginId::new("host.compat").expect("static owner"),
                    error.to_string(),
                )];
            }
        }
        let failures = self.plugins.shutdown_all().await;
        self.capabilities.shutdown_legacy().await;
        failures
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("plugins", &self.plugins.plugins().len())
            .field("capabilities", &self.plugins.records().len())
            .field("providers", &self.providers.len())
            .field("governance", &self.governance.name())
            .field("config", &self.config)
            .field("modules", &self.modules.len())
            .finish_non_exhaustive()
    }
}

/// Plugin ids known to a runtime, in id order. Convenience for diagnostics.
pub fn plugin_ids(runtime: &Runtime) -> Vec<&PluginId> {
    runtime.plugins.plugins().ids().collect()
}

#[cfg(test)]
#[path = "runtime/tests.rs"]
mod tests;

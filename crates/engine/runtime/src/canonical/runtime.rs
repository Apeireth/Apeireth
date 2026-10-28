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
use super::session::{InMemorySessionStore, SessionManager, SessionStore};
use super::session_locks::SessionLocks;

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
    pub(super) registered_behaviors:
        BTreeMap<String, Arc<super::behavior_adapter::RegisteredBehavior>>,
    pub(super) capabilities: CapabilityRegistry,
    pub(super) dispatcher: Arc<super::dispatcher::ToolDispatcher>,
    pub(super) session_locks: SessionLocks,
    pub(super) event_sink: RwLock<Arc<dyn RuntimeEventSink>>,
    pub(super) context_projector: Arc<dyn ContextProjector>,
    /// Session-scoped streak state for the repeated-identical-call advisory
    /// channel. Advisory only: it never blocks and never decides anything.
    pub(super) repetition_detectors: Mutex<BTreeMap<SessionId, RepetitionDetector>>,
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
    /// Optional domain-neutral observer of completed tool calls.
    ///
    /// The kernel emits one [`ToolCompletionFact`] at the single completion
    /// bookkeeping point (serial dispatch, parallel commit, cancel drain).
    /// With no observer installed, completion bookkeeping is exactly its
    /// historical two records — no extra lock, no domain state.
    pub(super) tool_completion_observer: Option<Arc<dyn ToolCompletionObserver>>,
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

    /// The repeated-identical-call advisory policy in force.
    pub fn repetition_policy(&self) -> &RepetitionPolicy {
        &self.config.repetition_advisory
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
    /// With the plan-mode mechanism enabled this also carries the resident
    /// `exit_plan_mode` control — identically in both plan-mode postures, so a
    /// posture switch never changes the tool catalog.
    pub fn tool_declarations(&self) -> Vec<apeireth_protocol::canonical::NormalizedTool> {
        let mut declarations: Vec<apeireth_protocol::canonical::NormalizedTool> = self
            .capabilities
            .declarations()
            .into_iter()
            .collect::<Vec<_>>();
        // Control declarations are part of the one registry-driven catalog.
        declarations.extend(self.capabilities.control_declarations());
        declarations
    }

    /// The configured context projector.
    pub fn context_projector(&self) -> &Arc<dyn ContextProjector> {
        &self.context_projector
    }

    /// The router over every provider that can currently serve a completion.
    ///
    /// Its members come from the capability registry, which is why the runtime
    /// can name no vendor: it knows only that some plugin declared
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
    registered_behavior_ids: std::collections::BTreeSet<String>,
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
    pub(super) agent_factory: Option<Arc<dyn apeireth_plugin::agents::ModuleAgentFactory>>,
    pub(super) v1_protocol: bool,
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
            registered_behavior_ids: Default::default(),
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
            agent_factory: None,
            v1_protocol: false,
        }
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

    /// Register a cognitive module. Modules run in registration order.
    #[must_use]
    pub fn with_registered_behavior(mut self, module: Arc<dyn BehaviorModule>) -> Self {
        self.registered_behavior_ids
            .insert(module.manifest().id.clone());
        self.modules.push(module);
        self
    }

    pub fn with_module(mut self, module: Arc<dyn BehaviorModule>) -> Self {
        self.modules.push(module);
        self
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

    /// Set the metrics rolling window (G-P1, 拍/层).
    ///
    /// Each layer (per session + global) keeps the last N turn-closure beats in
    /// process memory. The value is clamped to `1..=`[`super::metrics::MAX_METRICS_WINDOW_TURNS`],
    /// so the effective window always equals `config.metrics_window_turns`.
    #[must_use]
    pub fn with_metrics_window(mut self, turns: usize) -> Self {
        self.config.metrics_window_turns = turns.clamp(1, super::metrics::MAX_METRICS_WINDOW_TURNS);
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
    pub async fn build(mut self) -> RuntimeResult<Runtime> {
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
            if self.registered_behavior_ids.contains(&module.manifest().id) {
                let adapter = Arc::new(
                    super::behavior_adapter::RegisteredBehavior::new(module.clone())
                        .map_err(RuntimeError::misconfigured)?,
                );
                self.plugins.push(adapter.clone());
                registered_behaviors.insert(module.manifest().id.clone(), adapter);
            }
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
                .with_usage(providers.accounting.clone()),
        );
        Ok(Runtime {
            plugins: manager,
            v1_protocol: self.v1_protocol,
            providers: Arc::new(providers),
            sessions,
            governance: self.governance,
            clock: self.clock,
            config: self.config,
            modules: module_registry,
            registered_behaviors,
            capabilities: capability_registry,
            dispatcher,
            session_locks: SessionLocks::default(),
            event_sink: RwLock::new(self.event_sink),
            context_projector: self.context_projector,
            repetition_detectors: Mutex::new(BTreeMap::new()),
            invariant_auditor: self.invariant_auditor,
            context_spill,
            compaction: self
                .summary_generator
                .map(|generator| Arc::new(CompactionEngine::new(generator))),
            call_scheduler: self.call_scheduler,
            metrics,
            tool_completion_observer: self.tool_completion_observer,
        })
    }
}

/// Plugin ids known to a runtime, in id order. Convenience for diagnostics.
pub fn plugin_ids(runtime: &Runtime) -> Vec<&PluginId> {
    runtime.plugins.plugins().ids().collect()
}

#[cfg(test)]
#[path = "runtime/tests.rs"]
mod tests;

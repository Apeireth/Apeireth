//! The bridge: configuration in, dynamic tools out.
//!
//! One [`McpToolBridge`] owns one connection per enabled server, the dynamic
//! tool catalog, and the shared five-stage pipeline every external call
//! walks. [`McpToolBridge::refresh`] is the dynamic catalog entry: it runs
//! `tools/list` on every live link (re-opening a dropped link, which re-runs
//! the handshake and re-discovers), reconciles the catalog, and reports what
//! must be registered and unregistered; duplicate identities are refused.
//!
//! A reconnect re-discovers the tool list on its own: the connection hands
//! that list to the bridge (the rediscovery sink), which adopts it into the
//! catalog right away — the catalog does not drift from the live server
//! between two refreshes. The next refresh then carries the adopted tools
//! into the report the registry is fed from.
//!
//! # Two refresh shapes
//!
//! * [`McpToolBridge::refresh`] — **fail-all**: the first unreachable server
//!   aborts the round and the caller gets no delta at all (the startup
//!   wiring wants that: half a delta is worse than none).
//! * [`McpToolBridge::refresh_outcome`] — **per-server isolation**: an
//!   unreachable server contributes no delta (its tools stay where they are
//!   — an unreachable server is not evidence that its tools are gone), the
//!   healthy servers' delta is still reported, and the failures are reported
//!   next to it. The periodic refresh wants that: one dead server must not
//!   freeze the others, and a dead server must not take the live tools out
//!   of the registry.
//!
//! # Where the authorization surface lives
//!
//! The bridge holds **one** permission policy, and it is meant to be the
//! **shared production policy**: [`McpToolBridge::new_with_policy`] and
//! [`McpToolBridge::from_connections_with_policy`] take the same
//! `Arc<Mutex<PermissionPolicy>>` the runtime governance chain reads, so the
//! bridge's deny gate and the runtime's decision can never disagree ("the
//! operator authorized it but the bridge does not know" / "the bridge
//! authorized it but the runtime denies it").
//!
//! The default constructors keep a bridge-local policy for library and test
//! use. Until the wiring batch hands the bridge the production policy, the
//! bridge's policy is **not** the production policy — a library-level gate,
//! not a production one.
//!
//! # Where the deny surface lives
//!
//! The **deny** side is not part of the authorization surface: it is the
//! operator's blocklist, and it lives in the configuration — one
//! `deny_tools` list per server ([`super::config::McpServerSpec`]). The
//! bridge seeds the monotonic deny gate from that list when it is assembled,
//! so the gate is armed **before the first discovery**.
//!
//! The gate is the **one true source of "what is denied"**, and the bridge
//! consults it in two places:
//!
//! * **the directory**: a denied tool never enters the dynamic directory —
//!   it is not wrapped, not registered, and the model never sees it in the
//!   tool catalog;
//! * **the pipeline**: the gate refuses the call at the guard stage, which
//!   runs **before the tool executes**, so a handle obtained out of band
//!   still cannot run a denied tool.
//!
//! The deny side therefore beats every release: the read-only preset, the
//! operator allowlist (`auto_authorize_tools`) and a family grant all
//! decide *authorization*, and none of them can bring a denied tool back.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::PermissionPolicy;
use apeireth_plugin::ToolCapability;
use thiserror::Error;

use crate::exec_pipeline::{AroundPolicy, PipelinedCapability, ToolExecutionPipeline};
use crate::mcp::McpError;

use super::catalog::{McpCatalogError, McpToolCatalog, McpToolEntry};
use super::config::{McpBridgeOptions, McpConfigError, McpServerConfig};
use super::connection::{DiscoveredTool, McpRediscoverySink, McpServerConnection};
use super::governance::{
    McpFamilyGrants, McpPermissionMapping, McpRiskMappingHook, McpToolDenyGuard,
};
use super::tool::{mcp_output_schema, McpDynamicTool};
use super::SessionRedactionDomain;

/// Where discovered dynamic tools land (a module bag, a runtime registry).
pub trait McpToolRegistry: Send + Sync {
    /// Register one dynamic tool. Duplicate identities are refused.
    fn register_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String>;

    /// Unregister one dynamic tool by capability identity.
    fn unregister_tool(&self, capability_id: &CapabilityId);
}

/// What one catalog refresh changed.
pub struct DiscoveryReport {
    /// Newly discovered tools, already wrapped in the shared pipeline.
    pub registered: Vec<Arc<dyn ToolCapability>>,
    /// Entries that disappeared and must be unregistered.
    pub removed: Vec<McpToolEntry>,
    /// The full catalog after the refresh.
    pub live: Vec<McpToolEntry>,
}

/// What one refresh round changed when the servers are refreshed one by one.
///
/// A round that treats every server independently must not let one
/// unreachable server decide the fate of the others: the healthy servers'
/// delta is real, and the failed server's tools stay where they are — an
/// unreachable server is **not** evidence that its tools are gone.
pub struct RefreshOutcome {
    /// The delta of the servers that answered.
    pub report: DiscoveryReport,
    /// One entry per server that could not be reached.
    pub failures: Vec<McpBridgeError>,
}

impl DiscoveryReport {
    /// Model-facing names of the newly registered tools.
    pub fn registered_names(&self) -> Vec<String> {
        self.registered
            .iter()
            .map(|tool| tool.declaration().name)
            .collect()
    }

    /// Push this refresh into a registry: unregister the gone, register the
    /// new. Fail-closed: a refused registration is an error, never a skip.
    pub fn apply_to(&self, registry: &dyn McpToolRegistry) -> Result<(), McpBridgeError> {
        for entry in &self.removed {
            registry.unregister_tool(&entry.capability_id);
        }
        for tool in &self.registered {
            registry
                .register_tool(Arc::clone(tool))
                .map_err(McpBridgeError::Registration)?;
        }
        Ok(())
    }
}

/// The MCP tool bridge.
pub struct McpToolBridge {
    config: McpServerConfig,
    options: McpBridgeOptions,
    connections: Vec<Arc<McpServerConnection>>,
    catalog: Arc<McpToolCatalog>,
    policy: Arc<Mutex<PermissionPolicy>>,
    family_grants: Arc<McpFamilyGrants>,
    deny_guard: Arc<McpToolDenyGuard>,
    pipeline: Arc<ToolExecutionPipeline>,
    tools: Mutex<BTreeMap<String, McpToolHandle>>,
    /// The capabilities the read-only preset released in the **previous**
    /// round.
    ///
    /// The set is what makes the preset **revocable**: a server that stops
    /// declaring a tool read-only must not keep the grant the preset gave it.
    preset_grants: Mutex<BTreeSet<CapabilityId>>,
    /// Serializes the refresh rounds: two concurrent rounds would race on
    /// the same directory and the same policy.
    refresh_lock: tokio::sync::Mutex<()>,
    /// The **session redaction domain** every tool of this bridge sweeps its
    /// result face in.
    ///
    /// `None` = **not wired**: each tool keeps its own domain, exactly as
    /// before this batch (one alias per tool, no cross-tool recognition). The
    /// caller injects one domain with
    /// [`McpToolBridge::with_session_domain`] when the session owns a single
    /// domain — then every tool of the bridge sweeps in it, and the
    /// provider-facing egress gate can hold the same one.
    session_domain: Option<Arc<Mutex<SessionRedactionDomain>>>,
}

/// One wrapped dynamic tool plus the directory entry it was built from.
///
/// The entry is kept next to the tool so the wrapped-tool map can be
/// reconciled against the directory: a tool whose descriptor changed is
/// re-wrapped, and a tool the directory no longer lists leaves the live set.
struct McpToolHandle {
    entry: McpToolEntry,
    tool: Arc<dyn ToolCapability>,
}

/// The directory side of the reconnect hand-over.
///
/// A reconnect re-discovered a tool list; the list is adopted into the
/// directory of the server it belongs to, with the same
/// validate-then-mutate discipline as a refresh: one unbuildable identity
/// aborts the whole adoption, and a refused list leaves the directory
/// untouched.
///
/// The deny gate is consulted here too: a denied tool does not enter the
/// directory even when the server re-advertises it after a reconnect — the
/// gate owns the deny set, not the discovery path.
struct CatalogRediscoverySink {
    catalog: Arc<McpToolCatalog>,
    deny_guard: Arc<McpToolDenyGuard>,
}

impl McpRediscoverySink for CatalogRediscoverySink {
    fn on_rediscovery(&self, server: &str, tools: Vec<DiscoveredTool>) -> Result<(), String> {
        let mut entries = Vec::with_capacity(tools.len());
        for tool in &tools {
            let entry = McpToolEntry::new(server, tool).map_err(|error| error.to_string())?;
            if self.deny_guard.is_blocked(entry.capability_id.as_str()) {
                continue;
            }
            entries.push(entry);
        }
        self.catalog
            .replace_server(server, entries)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

impl McpToolBridge {
    /// Build the bridge over `config`, opening channels matching each spec.
    ///
    /// The bridge keeps its **own** permission policy here — a library-level
    /// gate. Production wants the shared policy instead: see
    /// [`McpToolBridge::new_with_policy`].
    pub fn new(config: McpServerConfig, options: McpBridgeOptions) -> Result<Self, McpBridgeError> {
        Self::new_with_policy(
            config,
            options,
            Arc::new(Mutex::new(PermissionPolicy::new())),
        )
    }

    /// Build the bridge over `config` with the **shared production policy**.
    ///
    /// `policy` is the same handle the runtime governance chain reads, so the
    /// bridge's deny gate and the runtime's decision can never disagree. This
    /// is the constructor the wiring batch uses; the default [`McpToolBridge::new`]
    /// keeps a bridge-local policy for library and test use.
    pub fn new_with_policy(
        config: McpServerConfig,
        options: McpBridgeOptions,
        policy: Arc<Mutex<PermissionPolicy>>,
    ) -> Result<Self, McpBridgeError> {
        config.validate().map_err(McpBridgeError::Config)?;
        let connections = config
            .enabled_servers()
            .map(|spec| {
                Arc::new(McpServerConnection::from_spec(spec.clone(), &options))
                    as Arc<McpServerConnection>
            })
            .collect();
        Self::from_connections_with_policy(config, options, connections, policy)
    }

    /// Build the bridge over pre-built connections (one per enabled server,
    /// in configuration order).
    ///
    /// The bridge keeps its **own** permission policy here — a library-level
    /// gate. Production wants the shared policy instead: see
    /// [`McpToolBridge::from_connections_with_policy`].
    pub fn from_connections(
        config: McpServerConfig,
        options: McpBridgeOptions,
        connections: Vec<Arc<McpServerConnection>>,
    ) -> Result<Self, McpBridgeError> {
        Self::from_connections_with_policy(
            config,
            options,
            connections,
            Arc::new(Mutex::new(PermissionPolicy::new())),
        )
    }

    /// Build the bridge over pre-built connections with the **shared
    /// production policy**.
    ///
    /// `policy` is the same handle the runtime governance chain reads, so the
    /// bridge's deny gate and the runtime's decision can never disagree.
    pub fn from_connections_with_policy(
        config: McpServerConfig,
        options: McpBridgeOptions,
        connections: Vec<Arc<McpServerConnection>>,
        policy: Arc<Mutex<PermissionPolicy>>,
    ) -> Result<Self, McpBridgeError> {
        config.validate().map_err(McpBridgeError::Config)?;
        let enabled = config.enabled_servers().count();
        if connections.len() != enabled {
            return Err(McpBridgeError::Config(McpConfigError::InvalidSpec {
                name: "<bridge>".to_string(),
                reason: format!(
                    "expected {enabled} connection(s) for enabled servers, got {}",
                    connections.len()
                ),
            }));
        }
        let catalog = Arc::new(McpToolCatalog::new());
        let family_grants = Arc::new(McpFamilyGrants::new());
        let deny_guard = Arc::new(McpToolDenyGuard::new());
        // The operator's deny list is armed here — before the first
        // discovery — so a denied tool is never discovered into the surface
        // and never reaches the pipeline.
        seed_deny_list(&config, &connections, &deny_guard)?;
        let pipeline = build_pipeline(&options, &catalog, &policy, &family_grants, &deny_guard);
        // Every connection gets the directory side of the reconnect
        // hand-over: a re-discovered tool list reaches the directory
        // without waiting for the next explicit refresh.
        let sink = Arc::new(CatalogRediscoverySink {
            catalog: Arc::clone(&catalog),
            deny_guard: Arc::clone(&deny_guard),
        });
        for connection in &connections {
            connection.set_rediscovery_sink(Arc::clone(&sink) as Arc<dyn McpRediscoverySink>);
        }
        Ok(Self {
            config,
            options,
            connections,
            catalog,
            policy,
            family_grants,
            deny_guard,
            pipeline,
            tools: Mutex::new(BTreeMap::new()),
            preset_grants: Mutex::new(BTreeSet::new()),
            refresh_lock: tokio::sync::Mutex::new(()),
            session_domain: None,
        })
    }

    /// Give every tool of this bridge **one** session redaction domain.
    ///
    /// This is the injection face of "one session, one domain": the tools
    /// this bridge discovers sweep their result faces in `domain`, so the
    /// alias of one value is the same alias in every tool of the session —
    /// and the provider-facing egress gate, holding the same domain,
    /// recognises (and can restore) what those tools produced.
    ///
    /// **Not wired by default**: without this call every tool keeps its own
    /// domain, exactly as before this batch — the behaviour of an unwired
    /// bridge is unchanged.
    #[must_use]
    pub fn with_session_domain(mut self, domain: Arc<Mutex<SessionRedactionDomain>>) -> Self {
        self.session_domain = Some(domain);
        self
    }

    /// The session redaction domain the tools of this bridge sweep in.
    ///
    /// `None` = **not wired** (each tool keeps its own domain).
    pub fn session_domain(&self) -> Option<&Arc<Mutex<SessionRedactionDomain>>> {
        self.session_domain.as_ref()
    }

    /// The loaded configuration.
    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }

    /// The options in force.
    pub fn options(&self) -> &McpBridgeOptions {
        &self.options
    }

    /// The live dynamic tool catalog.
    pub fn catalog(&self) -> &Arc<McpToolCatalog> {
        &self.catalog
    }

    /// The shared permission policy the risk mapping consults.
    ///
    /// With [`McpToolBridge::new_with_policy`] this is the **production**
    /// policy — the same handle the runtime governance chain reads.
    pub fn policy(&self) -> &Arc<Mutex<PermissionPolicy>> {
        &self.policy
    }

    /// The bridge-local family grant registry the deny gate consults.
    ///
    /// Bridge-local until the governance base ships the production family
    /// helpers (see [`McpFamilyGrants`]).
    pub fn family_grants(&self) -> &Arc<McpFamilyGrants> {
        &self.family_grants
    }

    /// The deny-only blocklist consulted by the guard stage.
    pub fn deny_guard(&self) -> &Arc<McpToolDenyGuard> {
        &self.deny_guard
    }

    /// The pipeline every external call walks.
    pub fn pipeline(&self) -> &Arc<ToolExecutionPipeline> {
        &self.pipeline
    }

    /// One connection per enabled server, in configuration order.
    pub fn connections(&self) -> &[Arc<McpServerConnection>] {
        &self.connections
    }

    /// The redacted startup log: URLs may be recorded, token / secret
    /// parameters never are.
    pub fn startup_log(&self) -> Vec<String> {
        self.config.redacted_startup_log()
    }

    /// A new module activation gets a fresh catalog and fresh generation-bound tools.
    pub(crate) fn reopen(&self) {
        self.tools.lock().unwrap_or_else(|e| e.into_inner()).clear();
        for connection in &self.connections {
            // Removing an existing server cannot create a name collision.
            let _ = self
                .catalog
                .replace_server(connection.server_name(), Vec::new());
            connection.reopen();
        }
    }
    pub(crate) fn revoke(&self) {
        for connection in &self.connections {
            connection.revoke();
        }
    }

    /// Refresh the whole dynamic catalog: (re)connect where needed, run
    /// `tools/list` everywhere, reconcile, and report the delta.
    ///
    /// **Fail-all**: the first server that cannot be reached aborts the whole
    /// round — the caller gets no delta at all rather than a half-applied
    /// one. Use [`McpToolBridge::refresh_outcome`] when the servers must be
    /// refreshed independently.
    pub async fn refresh(&self) -> Result<DiscoveryReport, McpBridgeError> {
        let _refresh = self.refresh_lock.lock().await;
        let mut registered = Vec::new();
        let mut removed = Vec::new();
        let mut removed_names: BTreeSet<String> = BTreeSet::new();
        for connection in &self.connections {
            self.refresh_server(
                connection,
                &mut registered,
                &mut removed,
                &mut removed_names,
            )
            .await?;
        }
        self.authorize_discovered();
        Ok(DiscoveryReport {
            registered,
            removed,
            live: self.catalog.list(),
        })
    }

    /// Refresh every server independently: one unreachable server does not
    /// decide the fate of the others.
    ///
    /// The failed server contributes **no** delta — neither additions nor
    /// removals: an unreachable server is not evidence that its tools are
    /// gone (withdrawing them would be a false removal the model would pay
    /// for). The healthy servers' delta is still reported, and the failures
    /// are reported next to it so the caller can observe them instead of
    /// swallowing them.
    pub async fn refresh_outcome(&self) -> RefreshOutcome {
        let mut registered = Vec::new();
        let mut removed = Vec::new();
        let mut removed_names: BTreeSet<String> = BTreeSet::new();
        let mut failures = Vec::new();
        for connection in &self.connections {
            if let Err(error) = self
                .refresh_server(
                    connection,
                    &mut registered,
                    &mut removed,
                    &mut removed_names,
                )
                .await
            {
                failures.push(error);
            }
        }
        self.authorize_discovered();
        RefreshOutcome {
            report: DiscoveryReport {
                registered,
                removed,
                live: self.catalog.list(),
            },
            failures,
        }
    }

    /// Refresh one server and fold its delta into the round being built.
    ///
    /// The per-server body is **atomic with respect to the shared state**:
    /// the identity check and the directory replacement both validate
    /// before they change anything, so a failing server leaves the
    /// directory and the wrapped-tool map exactly as they were.
    async fn refresh_server(
        &self,
        connection: &Arc<McpServerConnection>,
        registered: &mut Vec<Arc<dyn ToolCapability>>,
        removed: &mut Vec<McpToolEntry>,
        removed_names: &mut BTreeSet<String>,
    ) -> Result<(), McpBridgeError> {
        let server = connection.server_name().to_string();
        let discovered = match connection.discover().await {
            Ok(tools) => tools,
            // No live link (first connect or a dropped one): open one,
            // which also re-discovers the tool list.
            Err(McpError::Transport(_)) => {
                connection
                    .connect()
                    .await
                    .map_err(|error| McpBridgeError::Connect {
                        server: server.clone(),
                        error,
                    })?
            }
            Err(error) => {
                return Err(McpBridgeError::Connect {
                    server: server.clone(),
                    error,
                })
            }
        };
        let mut entries = Vec::with_capacity(discovered.len());
        for tool in &discovered {
            let entry = McpToolEntry::new(&server, tool)?;
            // A denied tool never enters the directory: it is not wrapped,
            // not registered, and the model never sees it. The guard stage
            // refuses it anyway, so both halves of the deny surface agree.
            if self.is_denied(&entry.capability_id) {
                continue;
            }
            entries.push(entry);
        }
        // The directory is reconciled against this server's live list,
        // and the wrapped-tool map is reconciled against the directory:
        // a reconnect adopts a re-discovered list into the directory
        // (those tools are live) without wrapping them, so the map is
        // what the registry is handed — the reconciliation below is
        // what carries the adopted tools into this report.
        let (_added, gone) = self.catalog.replace_server(&server, entries.clone())?;
        let live: BTreeMap<String, McpToolEntry> = entries
            .into_iter()
            .map(|entry| (entry.model_name.clone(), entry))
            .collect();
        let mut tools = self
            .tools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Tools the map still holds that the directory no longer lists:
        // the server dropped them, or a reconnect adopted a smaller
        // list. They leave the live set.
        let mut stale: Vec<McpToolEntry> = tools
            .values()
            .filter(|handle| {
                handle.entry.server == server && !live.contains_key(&handle.entry.model_name)
            })
            .map(|handle| handle.entry.clone())
            .collect();
        for entry in &stale {
            tools.remove(&entry.model_name);
        }
        // Tools the directory lists that the map does not hold yet (a
        // reconnect adopted them), plus the ones whose descriptor
        // changed: both are wrapped here.
        for (model_name, entry) in &live {
            let previous = tools.get(model_name).map(|handle| handle.entry.clone());
            match previous {
                // Unchanged: the wrapped tool is still the right one.
                Some(previous) if previous == *entry => continue,
                // The descriptor changed: the replaced descriptor leaves
                // the live set — the registry must forget it before it
                // learns the new one (same identity, new descriptor).
                Some(previous) => stale.push(previous),
                None => {}
            }
            let tool = self.wrap_tool(entry.clone());
            tools.insert(
                model_name.clone(),
                McpToolHandle {
                    entry: entry.clone(),
                    tool: Arc::clone(&tool),
                },
            );
            registered.push(tool);
        }
        drop(tools);
        // The report carries both sides of the delta: what the registry
        // must learn (the wrapped tools above) and what it must forget
        // (the tools the directory dropped plus the ones the map still
        // held).
        for entry in gone.into_iter().chain(stale) {
            if !removed_names.insert(entry.model_name.clone()) {
                continue;
            }
            removed.push(entry);
        }
        Ok(())
    }

    /// Fold the discovered declarations into the bridge's permission policy.
    ///
    /// External tool permission declarations become explicit grants only
    /// where the read-only preset releases them; everything else stays
    /// outside the authorization surface until an operator authorizes it.
    ///
    /// The grant says "in the authorization surface"; the per-call
    /// approval marking says "every call still waits for a human" —
    /// external calls are approved one by one, the preset does not
    /// exempt them.
    fn authorize_discovered(&self) {
        if self.options.readonly_preset {
            // **预设撤销**：上一轮被预设放行的能力，若服务器这一轮不再声明
            // 只读 ⇒ 撤销那条授权（否则「曾经只读」会变成永久授权）。
            // 撤销只撤**预设放行**的那条，不动人批标记。
            {
                let mut policy = self
                    .policy
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let previous = self.preset_grants.lock().unwrap_or_else(|e| e.into_inner());
                for id in previous
                    .iter()
                    .filter(|id| !self.catalog.is_declared_read_only(id))
                {
                    policy.revoke(&apeireth_governance::Permission::ExecuteTool(
                        id.to_string(),
                    ));
                }
            }
            let mut policy = self
                .policy
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            McpPermissionMapping::authorize_readonly_declarations_for_approval(
                &mut policy,
                &self.catalog,
            );
            *self.preset_grants.lock().unwrap_or_else(|e| e.into_inner()) = self
                .catalog
                .list()
                .into_iter()
                .filter(|entry| entry.declared_read_only)
                .map(|entry| entry.capability_id)
                .collect();
        }

        // Operator-authored per-server allowlists become explicit grants,
        // independent of the read-only preset: they exist for servers that
        // under-declare read-only semantics.
        //
        // **Degraded semantics of `auto_authorize_tools`**: the allowlist
        // decides *whether the capability is granted* — whether it is inside
        // the authorization surface at all. It does **not** mean "exempt
        // from the per-call human approval": external calls are approved one
        // by one, allowlisted or not. An operator reading "auto_authorize"
        // must not conclude "no human will be asked" — hence the grant is
        // paired with the per-call approval marking.
        let mut policy = self
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for connection in &self.connections {
            let spec = connection.spec();
            if spec.auto_authorize_tools.is_empty() {
                continue;
            }
            McpPermissionMapping::authorize_listed_for_approval(
                &mut policy,
                &self.catalog,
                connection.server_name(),
                &spec.auto_authorize_tools,
            );
        }
    }

    /// The currently live pipeline-wrapped tools.
    pub fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .map(|handle| Arc::clone(&handle.tool))
            .collect()
    }

    /// Whether the deny gate blocks this capability.
    ///
    /// The gate is the one true source of the deny set: the configuration
    /// seeds it, and the directory filter and the pipeline guard both read
    /// it. A caller may also block a capability at runtime
    /// ([`McpToolDenyGuard::deny_capability`]) — the next refresh then takes
    /// the tool out of the directory and reports it as removed, so the
    /// registry forgets it and the model stops seeing it.
    fn is_denied(&self, capability_id: &CapabilityId) -> bool {
        self.deny_guard.is_blocked(capability_id.as_str())
    }

    fn wrap_tool(&self, entry: McpToolEntry) -> Arc<dyn ToolCapability> {
        let connection = self
            .connections
            .iter()
            .find(|connection| connection.server_name() == entry.server)
            .cloned()
            .expect("catalog entries always name a configured connection");
        // The session domain is the injection face: wired ⇒ every tool of the
        // session sweeps in one domain; not wired ⇒ each tool keeps its own
        // (the behaviour before this batch).
        let dynamic = match &self.session_domain {
            Some(domain) => Arc::new(McpDynamicTool::new_with_domain(
                entry,
                connection,
                Arc::clone(domain),
            )),
            None => Arc::new(McpDynamicTool::new(entry, connection)),
        };
        Arc::new(PipelinedCapability::new(
            dynamic,
            Arc::clone(&self.pipeline),
        ))
    }
}

/// Fold the operator's deny lists into the monotonic deny gate.
///
/// Two sources, one gate:
///
/// * the **configured** servers — the operator surface (the `deny_tools`
///   list of each server spec);
/// * the **live** connections — the runtime authority (a caller may hand the
///   bridge connections built from a spec of its own, and that spec's deny
///   list must hold too).
///
/// The gate owns the deny set, so both sources are folded into it before the
/// first discovery runs: a denied tool is never discovered into the surface
/// and never reaches the pipeline.
///
/// A name that cannot compose into a stable capability id refuses the
/// assembly (fail-closed) instead of being silently skipped.
fn seed_deny_list(
    config: &McpServerConfig,
    connections: &[Arc<McpServerConnection>],
    deny_guard: &McpToolDenyGuard,
) -> Result<usize, McpBridgeError> {
    let mut blocked = 0;
    for spec in config.enabled_servers() {
        blocked += deny_guard.deny_server_tools(&spec.name, &spec.deny_tools)?;
    }
    for connection in connections {
        blocked += deny_guard
            .deny_server_tools(connection.server_name(), &connection.spec().deny_tools)?;
    }
    Ok(blocked)
}

fn build_pipeline(
    options: &McpBridgeOptions,
    catalog: &Arc<McpToolCatalog>,
    policy: &Arc<Mutex<PermissionPolicy>>,
    family_grants: &Arc<McpFamilyGrants>,
    deny_guard: &Arc<McpToolDenyGuard>,
) -> Arc<ToolExecutionPipeline> {
    let risk_mapping = Arc::new(
        McpRiskMappingHook::new(
            Arc::clone(policy),
            Arc::clone(catalog),
            options.readonly_preset,
        )
        .with_family_grants(Arc::clone(family_grants)),
    );
    let around = AroundPolicy::new().with_timeout(options.call_timeout_ms, options.max_timeout_ms);
    let deny_guard: Arc<McpToolDenyGuard> = Arc::clone(deny_guard);
    let deny_guard: Arc<dyn apeireth_governance::ToolGuard> = deny_guard;
    Arc::new(
        ToolExecutionPipeline::new()
            .with_pre_hook(risk_mapping)
            .with_guard(deny_guard)
            .with_around(around)
            .with_output_schema(mcp_output_schema()),
    )
}

/// What the bridge refuses to do.
#[derive(Debug, Error)]
pub enum McpBridgeError {
    /// The configuration is defective.
    #[error(transparent)]
    Config(#[from] McpConfigError),
    /// The dynamic catalog refused an identity.
    #[error(transparent)]
    Catalog(#[from] McpCatalogError),
    /// One server could not be reached or spoken to.
    #[error("mcp server {server:?} is unreachable: {error}")]
    Connect {
        /// The server that failed.
        server: String,
        /// The transport / protocol failure.
        error: McpError,
    },
    /// A registry refused one dynamic tool.
    #[error("dynamic tool registration refused: {0}")]
    Registration(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    /// The bridge and its connections stay shareable across threads: the
    /// reconnect hand-over is a `Send + Sync` sink, so a connection (and the
    /// bridge that owns it) can still be shared.
    #[test]
    fn the_bridge_and_its_connections_stay_send_and_sync() {
        assert_send_sync::<McpToolBridge>();
        assert_send_sync::<McpServerConnection>();
    }
}

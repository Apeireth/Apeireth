//! The composition root.
//!
//! # One place where the system is assembled
//!
//! [`Runtime`] is the single application-level composition root. CLI, gateway,
//! desktop, and tests all build one and talk to it; none of them assembles
//! providers, sessions, or tool dispatch for itself. A gateway that manages its
//! own sessions is a second runtime wearing an HTTP hat, and the two drift.
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

use std::sync::Arc;

use apeireth_core::kernel::{system_clock, CapabilityId, Clock, PluginId, TraceId};
use apeireth_governance::{AllowAll, GovernanceHook};
use apeireth_plugin::{
    CredentialResolver, NoCredentials, Plugin, PluginContext, PluginManager, ToolCapability,
};

use super::error::{RuntimeError, RuntimeResult};
use super::provider::ProviderRouter;
use super::session::{InMemorySessionStore, SessionManager, SessionStore};

/// How many provider round-trips one turn may take before the runtime stops it.
///
/// This is a structural guard, not a policy: it applies even when no governance
/// is configured. Eight is enough for realistic tool chains and small enough that
/// a model stuck in a loop fails fast.
pub const DEFAULT_MAX_ROUNDS: u32 = 8;

/// Runtime-wide settings.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Model used when a request does not name one.
    pub default_model: Option<String>,
    /// Round limit for one turn.
    pub max_rounds: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            default_model: None,
            max_rounds: DEFAULT_MAX_ROUNDS,
        }
    }
}

/// The assembled runtime.
pub struct Runtime {
    pub(super) plugins: PluginManager,
    pub(super) providers: ProviderRouter,
    pub(super) sessions: SessionManager,
    pub(super) governance: Arc<dyn GovernanceHook>,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) config: RuntimeConfig,
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

    /// Every tool that can currently be dispatched to.
    pub fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.plugins.active_tools()
    }

    /// The router over every provider that can currently serve a completion.
    ///
    /// Its members come from the capability registry, which is why the runtime
    /// can name no vendor: it knows only that some plugin declared
    /// `provider.something`.
    pub fn providers(&self) -> &ProviderRouter {
        &self.providers
    }

    /// Stop every plugin in reverse start order.
    ///
    /// Returns the failures encountered; shutdown continues past each one.
    pub async fn shutdown(&mut self) -> Vec<apeireth_plugin::PluginError> {
        self.plugins.shutdown_all().await
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("plugins", &self.plugins.plugins().len())
            .field("capabilities", &self.plugins.capabilities().len())
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
/// surprising: no credentials, no policy, memory-backed sessions.
pub struct RuntimeBuilder {
    clock: Arc<dyn Clock>,
    credentials: Arc<dyn CredentialResolver>,
    session_store: Arc<dyn SessionStore>,
    governance: Arc<dyn GovernanceHook>,
    plugins: Vec<Arc<dyn Plugin>>,
    fallback_order: Option<Vec<CapabilityId>>,
    config: RuntimeConfig,
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
            governance: Arc::new(AllowAll),
            plugins: Vec::new(),
            fallback_order: None,
            config: RuntimeConfig::default(),
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

    /// Add a plugin. Order is irrelevant; start order comes from declared
    /// dependencies.
    #[must_use]
    pub fn with_plugin(mut self, plugin: Arc<dyn Plugin>) -> Self {
        self.plugins.push(plugin);
        self
    }

    /// Order in which providers are tried.
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

    /// Register the plugins, start them in dependency order, and assemble.
    ///
    /// Plugins are started here rather than lazily on first use, so that a
    /// misconfiguration surfaces at boot rather than inside somebody's first
    /// request.
    pub async fn build(self) -> RuntimeResult<Runtime> {
        if self.config.max_rounds == 0 {
            return Err(RuntimeError::misconfigured(
                "max_rounds must be at least 1, otherwise no turn can ever run",
            ));
        }

        let mut manager = PluginManager::new();
        for plugin in self.plugins {
            manager.register(plugin)?;
        }

        let ctx = PluginContext::new(
            Arc::clone(&self.clock),
            Arc::clone(&self.credentials),
            TraceId::new(),
        );
        manager.start_all(&ctx).await?;

        // Providers are read out of the capability registry *after* start-up, so
        // the router contains exactly those whose plugins actually came up. A
        // provider whose plugin failed to initialize is never routed to.
        let mut providers =
            ProviderRouter::new(manager.active_providers(), Arc::clone(&self.clock));
        if let Some(order) = self.fallback_order {
            providers = providers.with_fallback_order(order);
        }

        Ok(Runtime {
            plugins: manager,
            providers,
            sessions: SessionManager::new(self.session_store, Arc::clone(&self.clock)),
            governance: self.governance,
            clock: self.clock,
            config: self.config,
        })
    }
}

/// Plugin ids known to a runtime, in id order. Convenience for diagnostics.
pub fn plugin_ids(runtime: &Runtime) -> Vec<&PluginId> {
    runtime.plugins.plugins().ids().collect()
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
                PluginId::new("builtin.key_reader").unwrap(),
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
        assert_eq!(runtime.plugins().plugins().len(), 0);
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
                .plugins()
                .state(&PluginId::new("builtin.key_reader").unwrap())
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
                DenyCapabilities::new().deny(CapabilityId::new("tool.shell").unwrap()),
            ))))
            .build()
            .await
            .unwrap();
        assert_eq!(runtime.governance().name(), "pipeline");
    }

    #[tokio::test]
    async fn shutdown_stops_the_plugins_it_started() {
        let mut runtime = Runtime::builder()
            .with_plugin(key_reader())
            .build()
            .await
            .unwrap();
        let id = PluginId::new("builtin.key_reader").unwrap();

        assert_eq!(runtime.plugins().state(&id).unwrap(), Lifecycle::Active);
        assert!(runtime.shutdown().await.is_empty());
        assert_eq!(runtime.plugins().state(&id).unwrap(), Lifecycle::Stopped);
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
}

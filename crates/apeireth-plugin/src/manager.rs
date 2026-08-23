//! Plugin lifecycle, and the typed views over the registries.
//!
//! [`PluginManager`] owns both canonical registries and is the only thing that
//! mutates plugin state. Everything else asks it questions.
//!
//! # Boot discipline
//!
//! Validation happens before any plugin runs. [`PluginManager::register`] rejects
//! duplicate plugin ids and duplicate capabilities; [`PluginManager::start_all`]
//! resolves dependency order and rejects missing dependencies and cycles *before*
//! calling the first `initialize`. A boot that discovers a conflict halfway
//! through has already run half its start-up code with no clean way back.
//!
//! # Availability
//!
//! Dispatch is refused unless the owning plugin is [`Lifecycle::Active`]. A
//! declared capability whose plugin failed to start is *visible but not
//! callable*, and the error says which state blocked it — the alternative is a
//! capability that silently disappears from listings when its plugin breaks,
//! which is far harder to diagnose.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Lifecycle, PluginId};
use apeireth_protocol::canonical::NormalizedTool;

use crate::capability::{CapabilityDescriptor, CapabilityKind};
use crate::error::{PluginError, PluginResult};
use crate::plugin::{Plugin, PluginContext};
use crate::provider::ProviderCapability;
use crate::registry::{CapabilityRecord, CapabilityRegistry, PluginRegistry};
use crate::tool::ToolCapability;

/// Owns plugin lifecycle and answers capability queries.
#[derive(Default)]
pub struct PluginManager {
    plugins: PluginRegistry,
    capabilities: CapabilityRegistry,
    /// Order in which plugins were started, so shutdown can reverse it.
    start_order: Vec<PluginId>,
}

impl PluginManager {
    /// An empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a plugin and index its declared capabilities.
    ///
    /// All-or-nothing: if any capability collides, the plugin is not registered
    /// either.
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) -> PluginResult<()> {
        let manifest = plugin.manifest();
        let id = manifest.id.clone();

        if self.plugins.contains(&id) {
            return Err(PluginError::DuplicatePlugin(id));
        }
        // Index first: a capability collision must not leave the plugin behind.
        self.capabilities.index(&id, &manifest.capabilities)?;
        self.plugins.register(plugin)?;
        Ok(())
    }

    /// Start every registered plugin in dependency order.
    ///
    /// Returns the order used. On failure the already-started plugins remain
    /// active and the failing one is marked [`Lifecycle::Failed`]; the caller
    /// decides whether to shut down.
    pub async fn start_all(&mut self, ctx: &PluginContext) -> PluginResult<Vec<PluginId>> {
        let order = self.resolve_order()?;

        for id in &order {
            self.plugins.transition(id, Lifecycle::Initializing)?;
            let plugin = Arc::clone(self.plugins.get(id)?);

            match plugin.initialize(ctx).await {
                Ok(()) => {
                    self.plugins.transition(id, Lifecycle::Active)?;
                    self.start_order.push(id.clone());
                }
                Err(e) => {
                    self.plugins.transition(id, Lifecycle::Failed)?;
                    return Err(PluginError::init_failed(id.clone(), e.to_string()));
                }
            }
        }

        Ok(order)
    }

    /// Stop every active plugin in reverse start order.
    ///
    /// Every plugin is asked to stop even if an earlier one failed; a shutdown
    /// that abandons the rest because one misbehaved leaks more than it
    /// protects. Returns the failures that occurred, in the order they occurred.
    pub async fn shutdown_all(&mut self) -> Vec<PluginError> {
        let mut failures = Vec::new();
        let order: Vec<PluginId> = self.start_order.drain(..).rev().collect();

        for id in order {
            if self.plugins.transition(&id, Lifecycle::Stopping).is_err() {
                continue;
            }
            let Ok(plugin) = self.plugins.get(&id).map(Arc::clone) else {
                continue;
            };

            match plugin.shutdown().await {
                Ok(()) => {
                    let _ = self.plugins.transition(&id, Lifecycle::Stopped);
                }
                Err(e) => {
                    let _ = self.plugins.transition(&id, Lifecycle::Failed);
                    failures.push(PluginError::shutdown_failed(id, e.to_string()));
                }
            }
        }

        failures
    }

    /// Dependency-respecting start order.
    ///
    /// Kahn's algorithm over a `BTreeMap`, so the order is deterministic: for a
    /// given set of plugins the boot sequence is always identical, which makes
    /// boot logs comparable across runs.
    fn resolve_order(&self) -> PluginResult<Vec<PluginId>> {
        let mut pending: BTreeMap<PluginId, BTreeSet<PluginId>> = BTreeMap::new();

        for id in self.plugins.ids() {
            let manifest = self.plugins.get(id)?.manifest();
            let mut deps = BTreeSet::new();
            for dep in &manifest.dependencies {
                if !self.plugins.contains(dep) {
                    return Err(PluginError::MissingDependency {
                        dependent: id.clone(),
                        missing: dep.clone(),
                    });
                }
                deps.insert(dep.clone());
            }
            pending.insert(id.clone(), deps);
        }

        let mut order = Vec::with_capacity(pending.len());
        let mut ready: VecDeque<PluginId> = pending
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(id, _)| id.clone())
            .collect();

        while let Some(id) = ready.pop_front() {
            pending.remove(&id);
            order.push(id.clone());

            // Collect first, then push, to keep `ready` in id order.
            let mut newly_ready: Vec<PluginId> = Vec::new();
            for (other, deps) in &mut pending {
                if deps.remove(&id) && deps.is_empty() {
                    newly_ready.push(other.clone());
                }
            }
            ready.extend(newly_ready);
        }

        if !pending.is_empty() {
            let cycle: Vec<&str> = pending.keys().map(PluginId::as_str).collect();
            return Err(PluginError::DependencyCycle(cycle.join(", ")));
        }

        Ok(order)
    }

    /// The lifecycle state of a plugin.
    pub fn state(&self, id: &PluginId) -> PluginResult<Lifecycle> {
        self.plugins.state(id)
    }

    /// The canonical plugin registry.
    pub fn plugins(&self) -> &PluginRegistry {
        &self.plugins
    }

    /// The canonical capability index.
    pub fn capabilities(&self) -> &CapabilityRegistry {
        &self.capabilities
    }

    /// The full record for a capability: declaration, owner, and availability.
    pub fn record(&self, id: &CapabilityId) -> PluginResult<CapabilityRecord<'_>> {
        let owner = self.capabilities.owner(id)?;
        let entry = self
            .plugins
            .entry(owner)
            .ok_or_else(|| PluginError::UnknownPlugin(owner.clone()))?;
        let descriptor = entry
            .plugin
            .manifest()
            .capability(id)
            .ok_or_else(|| PluginError::UnknownCapability(id.clone()))?;

        Ok(CapabilityRecord {
            descriptor,
            owner,
            state: entry.state,
        })
    }

    /// Every declared capability with its owner and availability, in id order.
    pub fn records(&self) -> Vec<CapabilityRecord<'_>> {
        self.capabilities
            .ids()
            .filter_map(|id| self.record(id).ok())
            .collect()
    }

    /// Resolve `id` to an active tool, or explain why not.
    pub fn tool(&self, id: &CapabilityId) -> PluginResult<Arc<dyn ToolCapability>> {
        let (entry, descriptor) = self.dispatchable(id, CapabilityKind::Tool)?;
        entry
            .plugin
            .tools()
            .into_iter()
            .find(|t| t.id() == &descriptor.id)
            .ok_or_else(|| PluginError::UnknownCapability(id.clone()))
    }

    /// Resolve `id` to an active provider, or explain why not.
    pub fn provider(&self, id: &CapabilityId) -> PluginResult<Arc<dyn ProviderCapability>> {
        let (entry, descriptor) = self.dispatchable(id, CapabilityKind::Provider)?;
        entry
            .plugin
            .providers()
            .into_iter()
            .find(|p| p.id() == &descriptor.id)
            .ok_or_else(|| PluginError::UnknownCapability(id.clone()))
    }

    /// Every tool that can currently be dispatched to, in capability-id order.
    pub fn active_tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.capabilities
            .ids_of_kind(CapabilityKind::Tool)
            .filter_map(|id| self.tool(id).ok())
            .collect()
    }

    /// Every provider that can currently be dispatched to, in capability-id order.
    pub fn active_providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        self.capabilities
            .ids_of_kind(CapabilityKind::Provider)
            .filter_map(|id| self.provider(id).ok())
            .collect()
    }

    /// Model-facing declarations for every active tool.
    ///
    /// This is what the runtime attaches to a request, so a tool whose plugin is
    /// not active is never offered to a model in the first place.
    pub fn tool_declarations(&self) -> Vec<NormalizedTool> {
        self.active_tools()
            .iter()
            .map(|t| t.declaration())
            .collect()
    }

    /// The active tool whose model-facing name is `name`.
    ///
    /// Models emit names, not capability ids, so dispatch needs this lookup. A
    /// name collision between two active tools is resolved by capability-id
    /// order; the manager rejects duplicate *ids*, but two plugins may still
    /// choose the same short name, and that is a manifest-quality problem rather
    /// than a dispatch-time one.
    pub fn tool_by_name(&self, name: &str) -> Option<Arc<dyn ToolCapability>> {
        self.active_tools()
            .into_iter()
            .find(|t| t.declaration().name == name)
    }

    /// Look up a capability that must exist, be of `kind`, and be dispatchable.
    fn dispatchable(
        &self,
        id: &CapabilityId,
        kind: CapabilityKind,
    ) -> PluginResult<(&crate::registry::PluginEntry, &CapabilityDescriptor)> {
        let owner = self.capabilities.owner(id)?;
        let entry = self
            .plugins
            .entry(owner)
            .ok_or_else(|| PluginError::UnknownPlugin(owner.clone()))?;
        let descriptor = entry
            .plugin
            .manifest()
            .capability(id)
            .ok_or_else(|| PluginError::UnknownCapability(id.clone()))?;

        if descriptor.kind != kind {
            return Err(PluginError::KindMismatch {
                capability: id.clone(),
                actual: descriptor.kind.id_prefix().to_string(),
                expected: kind.id_prefix(),
            });
        }
        if !entry.state.is_dispatchable() {
            return Err(PluginError::NotActive {
                capability: id.clone(),
                plugin: owner.clone(),
                state: entry.state.as_str(),
            });
        }

        Ok((entry, descriptor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use apeireth_core::kernel::{system_clock, TraceId};
    use apeireth_protocol::canonical::{ToolCall, ToolParameters, ToolResult};
    use async_trait::async_trait;

    use crate::credentials::NoCredentials;
    use crate::manifest::PluginManifest;

    struct Echo {
        id: CapabilityId,
        name: String,
    }

    #[async_trait]
    impl ToolCapability for Echo {
        fn id(&self) -> &CapabilityId {
            &self.id
        }
        fn declaration(&self) -> NormalizedTool {
            NormalizedTool {
                name: self.name.clone(),
                description: Some("echo".into()),
                parameters: ToolParameters::new(),
                strict: false,
            }
        }
        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            ToolResult::ok(&call.id, call.arguments.clone())
        }
    }

    /// Records boot order so tests can assert dependencies were respected.
    #[derive(Default)]
    struct BootLog(std::sync::Mutex<Vec<String>>);

    impl BootLog {
        fn record(&self, id: &str) {
            self.0.lock().unwrap().push(id.to_string());
        }
        fn entries(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }

    struct TestPlugin {
        manifest: PluginManifest,
        tools: Vec<Arc<dyn ToolCapability>>,
        log: Arc<BootLog>,
        fail_init: bool,
        fail_shutdown: bool,
        inits: AtomicUsize,
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }
        async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
            self.inits.fetch_add(1, Ordering::SeqCst);
            self.log.record(self.manifest.id.as_str());
            if self.fail_init {
                return Err(PluginError::init_failed(
                    self.manifest.id.clone(),
                    "deliberate",
                ));
            }
            Ok(())
        }
        async fn shutdown(&self) -> PluginResult<()> {
            self.log.record(&format!("stop:{}", self.manifest.id));
            if self.fail_shutdown {
                return Err(PluginError::shutdown_failed(
                    self.manifest.id.clone(),
                    "deliberate",
                ));
            }
            Ok(())
        }
        fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
            self.tools.clone()
        }
    }

    struct Builder {
        id: String,
        tools: Vec<(String, String)>,
        deps: Vec<String>,
        log: Arc<BootLog>,
        fail_init: bool,
        fail_shutdown: bool,
    }

    impl Builder {
        fn new(id: &str, log: &Arc<BootLog>) -> Self {
            Self {
                id: id.into(),
                tools: Vec::new(),
                deps: Vec::new(),
                log: Arc::clone(log),
                fail_init: false,
                fail_shutdown: false,
            }
        }
        fn tool(mut self, cap: &str, name: &str) -> Self {
            self.tools.push((cap.into(), name.into()));
            self
        }
        fn dep(mut self, id: &str) -> Self {
            self.deps.push(id.into());
            self
        }
        fn failing_init(mut self) -> Self {
            self.fail_init = true;
            self
        }
        fn failing_shutdown(mut self) -> Self {
            self.fail_shutdown = true;
            self
        }
        fn build(self) -> Arc<dyn Plugin> {
            let mut manifest =
                PluginManifest::new(PluginId::new(&self.id).unwrap(), "1.0.0", "test plugin");
            let mut tools: Vec<Arc<dyn ToolCapability>> = Vec::new();
            for (cap, name) in &self.tools {
                let id = CapabilityId::new(cap.as_str()).unwrap();
                manifest = manifest
                    .declare_capability(id.clone(), CapabilityKind::Tool, "test tool")
                    .unwrap();
                tools.push(Arc::new(Echo {
                    id,
                    name: name.clone(),
                }));
            }
            for dep in &self.deps {
                manifest = manifest.depends_on(PluginId::new(dep.as_str()).unwrap());
            }
            Arc::new(TestPlugin {
                manifest,
                tools,
                log: self.log,
                fail_init: self.fail_init,
                fail_shutdown: self.fail_shutdown,
                inits: AtomicUsize::new(0),
            })
        }
    }

    fn context() -> PluginContext {
        PluginContext::new(system_clock(), Arc::new(NoCredentials), TraceId::new())
    }

    #[tokio::test]
    async fn plugins_start_in_dependency_order() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        // Registered in an order that contradicts the dependency chain.
        m.register(Builder::new("p.c", &log).dep("p.b").build())
            .unwrap();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.register(Builder::new("p.b", &log).dep("p.a").build())
            .unwrap();

        let order = m.start_all(&context()).await.unwrap();
        let order: Vec<&str> = order.iter().map(PluginId::as_str).collect();

        assert_eq!(order, ["p.a", "p.b", "p.c"]);
        assert_eq!(log.entries(), ["p.a", "p.b", "p.c"]);
    }

    #[tokio::test]
    async fn shutdown_runs_in_reverse_start_order() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.register(Builder::new("p.b", &log).dep("p.a").build())
            .unwrap();

        m.start_all(&context()).await.unwrap();
        let failures = m.shutdown_all().await;

        assert!(failures.is_empty());
        assert_eq!(log.entries(), ["p.a", "p.b", "stop:p.b", "stop:p.a"]);
        assert_eq!(
            m.state(&PluginId::new("p.a").unwrap()).unwrap(),
            Lifecycle::Stopped
        );
    }

    #[tokio::test]
    async fn one_bad_shutdown_does_not_abandon_the_others() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.register(Builder::new("p.b", &log).failing_shutdown().build())
            .unwrap();

        m.start_all(&context()).await.unwrap();
        let failures = m.shutdown_all().await;

        assert_eq!(failures.len(), 1);
        assert_eq!(
            m.state(&PluginId::new("p.b").unwrap()).unwrap(),
            Lifecycle::Failed
        );
        assert_eq!(
            m.state(&PluginId::new("p.a").unwrap()).unwrap(),
            Lifecycle::Stopped,
            "the healthy plugin must still have been stopped"
        );
    }

    #[tokio::test]
    async fn a_missing_dependency_is_caught_before_anything_starts() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).dep("p.absent").build())
            .unwrap();

        let err = m.start_all(&context()).await.unwrap_err();
        assert!(matches!(err, PluginError::MissingDependency { .. }));
        assert!(
            log.entries().is_empty(),
            "no plugin may run before validation completes"
        );
    }

    #[tokio::test]
    async fn a_dependency_cycle_is_caught_before_anything_starts() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).dep("p.b").build())
            .unwrap();
        m.register(Builder::new("p.b", &log).dep("p.a").build())
            .unwrap();

        let err = m.start_all(&context()).await.unwrap_err();
        match err {
            PluginError::DependencyCycle(members) => {
                assert!(
                    members.contains("p.a") && members.contains("p.b"),
                    "{members}"
                );
            }
            other => panic!("expected DependencyCycle, got {other:?}"),
        }
        assert!(log.entries().is_empty());
    }

    #[tokio::test]
    async fn a_failing_initialize_marks_the_plugin_failed_and_stops_the_boot() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.register(Builder::new("p.b", &log).dep("p.a").failing_init().build())
            .unwrap();
        m.register(Builder::new("p.c", &log).dep("p.b").build())
            .unwrap();

        let err = m.start_all(&context()).await.unwrap_err();
        assert!(matches!(err, PluginError::PluginFailed { .. }));

        assert_eq!(
            m.state(&PluginId::new("p.a").unwrap()).unwrap(),
            Lifecycle::Active
        );
        assert_eq!(
            m.state(&PluginId::new("p.b").unwrap()).unwrap(),
            Lifecycle::Failed
        );
        assert_eq!(
            m.state(&PluginId::new("p.c").unwrap()).unwrap(),
            Lifecycle::Registered,
            "a plugin after the failure must not have been started"
        );
    }

    #[tokio::test]
    async fn a_capability_is_visible_before_start_but_not_dispatchable() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();

        let id = CapabilityId::new("tool.echo").unwrap();

        let record = m.record(&id).unwrap();
        assert_eq!(record.owner.as_str(), "p.a");
        assert!(!record.is_available(), "not started yet");

        // `Arc<dyn ToolCapability>` is not Debug, so `unwrap_err` is unavailable.
        match m.tool(&id) {
            Err(PluginError::NotActive { state, .. }) => assert_eq!(state, "registered"),
            Err(other) => panic!("expected NotActive, got {other:?}"),
            Ok(_) => panic!("an inactive plugin's tool must not be dispatchable"),
        }
        assert!(m.active_tools().is_empty());
        assert!(m.tool_declarations().is_empty());

        m.start_all(&context()).await.unwrap();

        assert!(m.record(&id).unwrap().is_available());
        assert!(m.tool(&id).is_ok());
        assert_eq!(m.tool_declarations().len(), 1);
    }

    #[tokio::test]
    async fn asking_for_a_tool_by_a_provider_id_is_a_kind_mismatch() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();
        m.start_all(&context()).await.unwrap();

        match m.provider(&CapabilityId::new("tool.echo").unwrap()) {
            Err(e) => assert!(matches!(e, PluginError::KindMismatch { .. }), "{e:?}"),
            Ok(_) => panic!("a tool must not be resolvable as a provider"),
        }
    }

    #[tokio::test]
    async fn a_model_facing_name_resolves_to_its_capability() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(
            Builder::new("p.a", &log)
                .tool("tool.calculator", "calculator")
                .build(),
        )
        .unwrap();
        m.start_all(&context()).await.unwrap();

        let tool = m.tool_by_name("calculator").expect("resolvable by name");
        assert_eq!(tool.id().as_str(), "tool.calculator");
        assert!(m.tool_by_name("absent").is_none());
    }

    #[test]
    fn a_capability_collision_rejects_the_whole_plugin() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();

        let err = m
            .register(Builder::new("p.b", &log).tool("tool.echo", "echo2").build())
            .unwrap_err();
        assert!(matches!(err, PluginError::DuplicateCapability { .. }));

        assert_eq!(
            m.plugins().len(),
            1,
            "the rejected plugin must not be registered"
        );
        assert_eq!(m.capabilities().len(), 1);
    }

    #[tokio::test]
    async fn dispatch_reaches_the_owning_plugins_implementation() {
        let log = Arc::new(BootLog::default());
        let mut m = PluginManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();
        m.start_all(&context()).await.unwrap();

        let tool = m.tool(&CapabilityId::new("tool.echo").unwrap()).unwrap();
        let result = tool
            .invoke(&ToolCall {
                id: "call_1".into(),
                name: "echo".into(),
                arguments: serde_json::json!({ "v": 7 }),
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.render(), r#"{"v":7}"#);
    }
}

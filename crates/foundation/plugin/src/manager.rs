//! Module lifecycle, dependency resolution, and the typed views over the
//! registries.
//!
//! [`ModuleManager`] owns both canonical registries and is the only thing that
//! mutates module state. Everything else asks it questions.
//!
//! # Boot discipline
//!
//! Validation happens before any module runs. [`ModuleManager::register`]
//! rejects duplicate module ids, duplicate capabilities, undeclared or missing
//! implementations, and colliding model-facing tool names;
//! [`ModuleManager::start_all`] resolves dependency order and rejects missing
//! dependencies and cycles *before* calling the first `initialize`. A boot that
//! discovers a conflict halfway through has already run half its start-up code
//! with no clean way back.
//!
//! # Availability
//!
//! Dispatch is refused unless the owning module is [`Lifecycle::Active`]. A
//! declared capability whose module failed to start is *visible but not
//! callable*, and the error says which state blocked it — the alternative is a
//! capability that silently disappears from listings when its module breaks,
//! which is far harder to diagnose.
//!
//! # Dynamic capabilities
//!
//! Some modules own capabilities that only exist after boot (an MCP server's
//! discovered tools). [`ModuleManager::register_dynamic_tool`] admits one only
//! after checking that its capability id and model-facing name are globally
//! unique, then asks the owning module to take it, then indexes it. Because the
//! index is a derived view over what the owning module actually exports, the
//! two can never disagree about what exists.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Lifecycle, ModuleId};
use apeireth_protocol::canonical::NormalizedTool;

use crate::capability::CapabilityKind;
use crate::error::{ModuleResult, ModuleSystemError};
use crate::module::{CapabilityOwnership, HookPoint, Module, ModuleExports, ModuleInitContext};
use crate::provider::ProviderCapability;
use crate::registry::{CapabilityRecord, CapabilityRegistry, ModuleRegistry};
use crate::tool::ToolCapability;

/// Owns module lifecycle and answers capability queries.
#[derive(Default)]
pub struct ModuleManager {
    modules: ModuleRegistry,
    capabilities: CapabilityRegistry,
    /// Order in which modules were started, so shutdown can reverse it.
    start_order: Vec<ModuleId>,
}

impl ModuleManager {
    /// An empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a module and index its declared capabilities.
    ///
    /// All-or-nothing: if any capability collides, the module is not registered
    /// either.
    pub fn register(&mut self, module: Arc<dyn Module>) -> ModuleResult<()> {
        let manifest = module.manifest();
        let id = manifest.id.clone();

        if self.modules.contains(&id) {
            return Err(ModuleSystemError::DuplicateModule(id));
        }
        let exports = module.exports();
        match manifest.capability_ownership {
            // The manifest is the declaration: what it says and what the module
            // can actually do must agree in both directions.
            CapabilityOwnership::Static => {
                Self::validate_implementations(module.as_ref(), &exports)?
            }
            // A dynamic owner cannot enumerate its capabilities up front, so the
            // agreement check does not apply; global uniqueness still does.
            CapabilityOwnership::Dynamic => {}
        }
        self.validate_tool_names(&exports.tools)?;

        // Index first: a capability collision must not leave the module behind.
        // One all-or-nothing batch — the manifest's declarations plus, for a
        // dynamic owner, whatever it already holds — so a rejection part-way
        // through cannot leave a half-indexed module behind.
        let mut ids: Vec<CapabilityId> =
            manifest.capabilities.iter().map(|c| c.id.clone()).collect();
        if manifest.capability_ownership == CapabilityOwnership::Dynamic {
            // Whatever a dynamic owner already holds is live from the moment it
            // registers: its exports are the truth, exactly like a static
            // module's declarations are.
            ids.extend(exports.tools.iter().map(|tool| tool.id().clone()));
        }
        ids.sort();
        ids.dedup();
        self.capabilities.index_ids(&id, &ids)?;
        self.modules.register(module)?;
        Ok(())
    }

    /// Fail closed when the manifest and executable capability surface differ.
    fn validate_implementations(module: &dyn Module, exports: &ModuleExports) -> ModuleResult<()> {
        let manifest = module.manifest();
        let module_id = &manifest.id;

        let implemented_tools: BTreeSet<CapabilityId> =
            exports.tools.iter().map(|tool| tool.id().clone()).collect();
        let implemented_providers: BTreeSet<CapabilityId> = exports
            .providers
            .iter()
            .map(|provider| provider.id().clone())
            .collect();

        for tool in &exports.tools {
            let declared = manifest
                .capability(tool.id())
                .is_some_and(|descriptor| descriptor.kind == CapabilityKind::Tool);
            if !declared {
                return Err(ModuleSystemError::UndeclaredCapabilityImplementation {
                    module: module_id.clone(),
                    capability: tool.id().clone(),
                    kind: CapabilityKind::Tool.id_prefix(),
                });
            }
        }
        for provider in &exports.providers {
            let declared = manifest
                .capability(provider.id())
                .is_some_and(|descriptor| descriptor.kind == CapabilityKind::Provider);
            if !declared {
                return Err(ModuleSystemError::UndeclaredCapabilityImplementation {
                    module: module_id.clone(),
                    capability: provider.id().clone(),
                    kind: CapabilityKind::Provider.id_prefix(),
                });
            }
        }

        for descriptor in manifest.capabilities_of_kind(CapabilityKind::Tool) {
            if !implemented_tools.contains(&descriptor.id) {
                return Err(ModuleSystemError::MissingCapabilityImplementation {
                    module: module_id.clone(),
                    capability: descriptor.id.clone(),
                    kind: CapabilityKind::Tool.id_prefix(),
                });
            }
        }
        for descriptor in manifest.capabilities_of_kind(CapabilityKind::Provider) {
            if !implemented_providers.contains(&descriptor.id) {
                return Err(ModuleSystemError::MissingCapabilityImplementation {
                    module: module_id.clone(),
                    capability: descriptor.id.clone(),
                    kind: CapabilityKind::Provider.id_prefix(),
                });
            }
        }

        Ok(())
    }

    /// Ensure every model-facing tool name maps to exactly one capability id.
    ///
    /// Covers the whole manager, not just the candidate module: this is the one
    /// place a model-facing name is checked, and it is checked against every
    /// module's live export set.
    fn validate_tool_names(&self, candidate_tools: &[Arc<dyn ToolCapability>]) -> ModuleResult<()> {
        let mut names: BTreeMap<String, CapabilityId> = BTreeMap::new();

        for module_id in self.modules.ids() {
            let entry = self
                .modules
                .entry(module_id)
                .expect("registry id without entry");
            for tool in entry.module.exports().tools {
                names.insert(tool.declaration().name, tool.id().clone());
            }
        }

        for tool in candidate_tools {
            let name = tool.declaration().name;
            if let Some(incumbent) = names.get(&name) {
                if incumbent != tool.id() {
                    return Err(ModuleSystemError::AmbiguousToolName {
                        name,
                        incumbent: incumbent.clone(),
                        challenger: tool.id().clone(),
                    });
                }
            } else {
                names.insert(name, tool.id().clone());
            }
        }

        Ok(())
    }

    /// Start every registered module in dependency order.
    ///
    /// Returns the order used. On failure the already-started modules remain
    /// active and the failing one is marked [`Lifecycle::Failed`]; the caller
    /// decides whether to shut down.
    pub async fn start_all(&mut self, ctx: &ModuleInitContext) -> ModuleResult<Vec<ModuleId>> {
        let order = self.resolve_order()?;

        for id in &order {
            self.modules.transition(id, Lifecycle::Initializing)?;
            let module = Arc::clone(self.modules.get(id)?);

            match module.initialize(ctx).await {
                Ok(()) => {
                    self.modules.transition(id, Lifecycle::Active)?;
                    self.start_order.push(id.clone());
                }
                Err(e) => {
                    self.modules.transition(id, Lifecycle::Failed)?;
                    return Err(ModuleSystemError::init_failed(id.clone(), e.to_string()));
                }
            }
        }

        Ok(order)
    }

    /// Stop every active module in reverse start order.
    ///
    /// Every module is asked to stop even if an earlier one failed; a shutdown
    /// that abandons the rest because one misbehaved leaks more than it
    /// protects. Returns the failures that occurred, in the order they occurred.
    pub async fn shutdown_all(&mut self) -> Vec<ModuleSystemError> {
        let mut failures = Vec::new();
        let order: Vec<ModuleId> = self.start_order.drain(..).rev().collect();

        for id in order {
            if self.modules.transition(&id, Lifecycle::Stopping).is_err() {
                continue;
            }
            let Ok(module) = self.modules.get(&id).map(Arc::clone) else {
                continue;
            };

            match module.shutdown().await {
                Ok(()) => {
                    let _ = self.modules.transition(&id, Lifecycle::Stopped);
                }
                Err(e) => {
                    let _ = self.modules.transition(&id, Lifecycle::Failed);
                    failures.push(ModuleSystemError::shutdown_failed(id, e.to_string()));
                }
            }
        }

        failures
    }

    /// Dependency-respecting start order.
    ///
    /// Kahn's algorithm, with the ready set ordered by registration sequence:
    /// for a given set of modules the boot sequence is always identical (which
    /// makes boot logs comparable across runs), and it is the embedder's
    /// registration order everywhere a dependency does not force otherwise.
    fn resolve_order(&self) -> ModuleResult<Vec<ModuleId>> {
        let mut pending: BTreeMap<ModuleId, BTreeSet<ModuleId>> = BTreeMap::new();
        let mut seqs: BTreeMap<ModuleId, u64> = BTreeMap::new();

        for id in self.modules.ids() {
            let manifest = self.modules.get(id)?.manifest();
            let mut deps = BTreeSet::new();
            for dep in &manifest.dependencies {
                if !self.modules.contains(dep) {
                    return Err(ModuleSystemError::MissingDependency {
                        dependent: id.clone(),
                        missing: dep.clone(),
                    });
                }
                deps.insert(dep.clone());
            }
            pending.insert(id.clone(), deps);
            seqs.insert(id.clone(), self.modules.seq(id)?);
        }

        let mut order = Vec::with_capacity(pending.len());
        // Ready set kept sorted by registration sequence, not by id, so the
        // embedder's order survives wherever dependencies allow it.
        let mut ready: Vec<ModuleId> = pending
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(id, _)| id.clone())
            .collect();
        ready.sort_by_key(|id| seqs.get(id).copied().unwrap_or(u64::MAX));

        while let Some(id) = ready.first().cloned() {
            ready.remove(0);
            pending.remove(&id);
            order.push(id.clone());

            let mut newly_ready: Vec<ModuleId> = Vec::new();
            for (other, deps) in &mut pending {
                if deps.remove(&id) && deps.is_empty() {
                    newly_ready.push(other.clone());
                }
            }
            ready.extend(newly_ready);
            ready.sort_by_key(|id| seqs.get(id).copied().unwrap_or(u64::MAX));
        }

        if !pending.is_empty() {
            let cycle: Vec<&str> = pending.keys().map(ModuleId::as_str).collect();
            return Err(ModuleSystemError::DependencyCycle(cycle.join(", ")));
        }

        Ok(order)
    }

    /// Register one dynamic capability on an existing module after boot.
    ///
    /// The one path by which a capability can appear after `start_all`. It
    /// fails closed on any conflict: the capability id must not already be
    /// indexed by anyone, the model-facing name must not already be exported by
    /// anyone, and the owning module must genuinely accept dynamic tools. Only
    /// then is the module asked to take it and the index extended, so a rejected
    /// registration leaves the system exactly as it was.
    pub async fn register_dynamic_tool(
        &mut self,
        owner: &ModuleId,
        tool: Arc<dyn ToolCapability>,
    ) -> ModuleResult<()> {
        let id = tool.id().clone();
        if self.capabilities.contains(&id) {
            let incumbent = self.capabilities.owner(&id)?.clone();
            return Err(ModuleSystemError::DuplicateCapability {
                capability: id,
                incumbent,
                challenger: owner.clone(),
            });
        }
        let name = tool.declaration().name;
        if let Some(incumbent) = self.live_tool_name_owner(&name) {
            // A name claimed by this very capability is not a collision: the
            // tool may already sit in the owner's own bag (filled without this
            // path) and is being admitted now. Only a different capability id
            // under the same name is ambiguous.
            if incumbent != id {
                return Err(ModuleSystemError::AmbiguousToolName {
                    name,
                    incumbent,
                    challenger: id,
                });
            }
        }
        if !self.modules.contains(owner) {
            return Err(ModuleSystemError::UnknownModule(owner.clone()));
        }

        let module = Arc::clone(self.modules.get(owner)?);
        // The manager owns the policy: a module that did not declare dynamic
        // ownership in its manifest never sees the request, whatever its
        // `add_tool` would do.
        if module.manifest().capability_ownership != CapabilityOwnership::Dynamic {
            return Err(ModuleSystemError::DynamicCapabilityUnsupported {
                module: owner.clone(),
            });
        }
        // Implementation ownership lives with the module. If it already holds
        // this capability id (for example a tool placed in its own bag without
        // going through this path), that ownership is exactly what this call
        // would establish, so the existing implementation is adopted and only
        // the identity is indexed.
        let already_owned = module.exports().tools.iter().any(|t| t.id() == &id);
        if !already_owned {
            module.add_tool(Arc::clone(&tool)).await?;
        }
        self.capabilities.index_ids(owner, &[id])?;
        Ok(())
    }

    /// Remove one dynamic capability from a module and from the index.
    ///
    /// Returns whether anything was actually removed.
    pub async fn unregister_dynamic_tool(
        &mut self,
        owner: &ModuleId,
        capability: &CapabilityId,
    ) -> ModuleResult<bool> {
        let Some(index_owner) = self.capabilities.owner(capability).ok().cloned() else {
            return Ok(false);
        };
        if &index_owner != owner {
            return Err(ModuleSystemError::UnknownCapability(capability.clone()));
        }
        let module = Arc::clone(self.modules.get(owner)?);
        // Removal is the other half of dynamic ownership and is gated the same
        // way, so a static module's declared capabilities cannot be taken away
        // through this path at all.
        if module.manifest().capability_ownership != CapabilityOwnership::Dynamic {
            return Err(ModuleSystemError::DynamicCapabilityUnsupported {
                module: owner.clone(),
            });
        }
        let removed = module.remove_tool(capability).await?;
        if removed {
            self.capabilities.unindex(capability);
        }
        Ok(removed)
    }

    /// The capability id of whichever live tool claims this model-facing name.
    fn live_tool_name_owner(&self, name: &str) -> Option<CapabilityId> {
        self.modules
            .modules()
            .iter()
            .flat_map(|module| module.exports().tools)
            .find(|tool| tool.declaration().name == name)
            .map(|tool| tool.id().clone())
    }

    /// The lifecycle state of a module.
    pub fn state(&self, id: &ModuleId) -> ModuleResult<Lifecycle> {
        self.modules.state(id)
    }

    /// The canonical module registry.
    pub fn modules(&self) -> &ModuleRegistry {
        &self.modules
    }

    /// The canonical capability index.
    pub fn capabilities(&self) -> &CapabilityRegistry {
        &self.capabilities
    }

    /// The full record for a capability: declaration, owner, and availability.
    ///
    /// Only declared capabilities have a record; a dynamically registered tool
    /// has an owner and a dispatchable implementation but no manifest
    /// declaration, and is listed through [`Self::tool_entries`] instead.
    pub fn record(&self, id: &CapabilityId) -> ModuleResult<CapabilityRecord<'_>> {
        let owner = self.capabilities.owner(id)?;
        let entry = self
            .modules
            .entry(owner)
            .ok_or_else(|| ModuleSystemError::UnknownModule(owner.clone()))?;
        let descriptor = entry
            .module
            .manifest()
            .capability(id)
            .ok_or_else(|| ModuleSystemError::UnknownCapability(id.clone()))?;

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

    /// Every dispatchable tool with its owning module, in module order.
    ///
    /// This is the honest listing for diagnostics: it reflects what the owning
    /// modules actually export right now, which includes tools registered after
    /// boot. A tool filled straight into a module's own bag without passing
    /// through [`Self::register_dynamic_tool`] is not listed either: the
    /// capability index is the identity lookup, and existence is not decided by
    /// the bag.
    pub fn tool_entries(&self) -> Vec<(ModuleId, Arc<dyn ToolCapability>)> {
        let mut out = Vec::new();
        for module in self.modules.modules() {
            let id = module.manifest().id.clone();
            if self
                .modules
                .state(&id)
                .is_ok_and(|state| !state.is_dispatchable())
            {
                continue;
            }
            for tool in module.exports().tools {
                if self
                    .capabilities
                    .owner(tool.id())
                    .is_ok_and(|owner| owner == &id)
                {
                    out.push((id.clone(), tool));
                }
            }
        }
        out
    }

    /// Which modules declare that they observe `hook`, in registration order.
    ///
    /// A derived view over the manifests, never a second copy of the fact, and
    /// the first half of the dispatch list: the runtime calls a module's hook
    /// only for points the module declared here.
    pub fn modules_observing(&self, hook: HookPoint) -> Vec<ModuleId> {
        self.modules
            .modules()
            .iter()
            .filter(|module| module.manifest().observes_hook(hook))
            .map(|module| module.manifest().id.clone())
            .collect()
    }

    /// Modules that declared `hook` and can currently receive it, in
    /// registration order.
    ///
    /// The dispatch list itself: declaration gates which modules are called,
    /// and a module that is not [`Lifecycle::Active`] receives no runtime hook.
    /// Derived on demand from the manifests and lifecycle states; no part of it
    /// is stored twice, and a dynamic capability appearing or disappearing
    /// never changes it.
    pub fn dispatchable_observers(&self, hook: HookPoint) -> Vec<Arc<dyn Module>> {
        self.modules
            .modules()
            .into_iter()
            .filter(|module| module.manifest().observes_hook(hook))
            .filter(|module| {
                self.modules
                    .state(&module.manifest().id)
                    .is_ok_and(|state| state.is_dispatchable())
            })
            .map(Arc::clone)
            .collect()
    }

    /// Every module that declares at least one observed hook point.
    pub fn hook_index(&self) -> BTreeMap<HookPoint, Vec<ModuleId>> {
        let mut index: BTreeMap<HookPoint, Vec<ModuleId>> = BTreeMap::new();
        for module in self.modules.modules() {
            let id = module.manifest().id.clone();
            for hook in &module.manifest().hooks {
                index.entry(*hook).or_default().push(id.clone());
            }
        }
        index
    }

    /// Resolve `id` to an active tool, or explain why not.
    pub fn tool(&self, id: &CapabilityId) -> ModuleResult<Arc<dyn ToolCapability>> {
        let entry = self.dispatchable(id, CapabilityKind::Tool)?;
        entry
            .module
            .exports()
            .tools
            .into_iter()
            .find(|t| t.id() == id)
            .ok_or_else(|| ModuleSystemError::UnknownCapability(id.clone()))
    }

    /// Resolve `id` to an active provider, or explain why not.
    pub fn provider(&self, id: &CapabilityId) -> ModuleResult<Arc<dyn ProviderCapability>> {
        let entry = self.dispatchable(id, CapabilityKind::Provider)?;
        entry
            .module
            .exports()
            .providers
            .into_iter()
            .find(|p| p.id() == id)
            .ok_or_else(|| ModuleSystemError::UnknownCapability(id.clone()))
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
    /// This is what the runtime attaches to a request, so a tool whose module is
    /// not active is never offered to a model in the first place.
    pub fn tool_declarations(&self) -> Vec<NormalizedTool> {
        self.active_tools()
            .iter()
            .map(|tool| tool.declaration())
            .collect()
    }

    /// The active tool whose model-facing name is `name`.
    ///
    /// Models emit names, not capability ids, so dispatch needs this lookup.
    /// Registration rejects name collisions, so this lookup is unambiguous.
    pub fn tool_by_name(&self, name: &str) -> Option<Arc<dyn ToolCapability>> {
        self.active_tools()
            .into_iter()
            .find(|tool| tool.declaration().name == name)
    }

    /// Look up a capability that must exist, be of `kind`, and be dispatchable.
    ///
    /// Kind is inferred from the capability id's reserved prefix, which the
    /// declaration path already validated — so a dynamically registered tool is
    /// classified exactly like a statically declared one.
    fn dispatchable(
        &self,
        id: &CapabilityId,
        kind: CapabilityKind,
    ) -> ModuleResult<crate::registry::ModuleEntryRef<'_>> {
        let owner = self.capabilities.owner(id)?;
        let entry = self
            .modules
            .entry(owner)
            .ok_or_else(|| ModuleSystemError::UnknownModule(owner.clone()))?;

        let actual = CapabilityKind::from_id(id);
        if actual != Some(kind) {
            return Err(ModuleSystemError::KindMismatch {
                capability: id.clone(),
                actual: actual
                    .map(|k| k.id_prefix().to_string())
                    .unwrap_or_else(|| id.kind_segment().to_string()),
                expected: kind.id_prefix(),
            });
        }
        if !entry.state.is_dispatchable() {
            return Err(ModuleSystemError::NotActive {
                capability: id.clone(),
                module: owner.clone(),
                state: entry.state.as_str(),
            });
        }

        Ok(entry)
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
    use crate::manifest::ModuleManifest;
    use crate::provider::ProviderError;
    use crate::tool::ToolCapability;

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

    struct Scripted {
        id: CapabilityId,
    }

    #[async_trait]
    impl ProviderCapability for Scripted {
        fn id(&self) -> &CapabilityId {
            &self.id
        }
        fn models(&self) -> Vec<apeireth_protocol::canonical::ModelDescriptor> {
            Vec::new()
        }
        async fn complete(
            &self,
            _request: &apeireth_protocol::canonical::NormalizedRequest,
        ) -> Result<apeireth_protocol::canonical::NormalizedResponse, ProviderError> {
            Err(ProviderError::Refused {
                provider: self.id.to_string(),
                detail: "scripted provider never completes".into(),
            })
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

    struct TestModule {
        manifest: ModuleManifest,
        tools: Vec<Arc<dyn ToolCapability>>,
        providers: Vec<Arc<dyn ProviderCapability>>,
        log: Arc<BootLog>,
        fail_init: bool,
        fail_shutdown: bool,
        inits: AtomicUsize,
        dynamic: std::sync::Mutex<Vec<Arc<dyn ToolCapability>>>,
    }

    #[async_trait]
    impl Module for TestModule {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }
        async fn initialize(&self, _ctx: &ModuleInitContext) -> ModuleResult<()> {
            self.inits.fetch_add(1, Ordering::SeqCst);
            self.log.record(self.manifest.id.as_str());
            if self.fail_init {
                return Err(ModuleSystemError::init_failed(
                    self.manifest.id.clone(),
                    "deliberate",
                ));
            }
            Ok(())
        }
        async fn shutdown(&self) -> ModuleResult<()> {
            self.log.record(&format!("stop:{}", self.manifest.id));
            if self.fail_shutdown {
                return Err(ModuleSystemError::shutdown_failed(
                    self.manifest.id.clone(),
                    "deliberate",
                ));
            }
            Ok(())
        }
        fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
            // A dynamic tool is live the moment the manager admits it, so the
            // export set includes the bag.
            let mut out = self.tools.clone();
            out.extend(self.dynamic.lock().unwrap().clone());
            out
        }
        fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
            self.providers.clone()
        }
        async fn add_tool(&self, tool: Arc<dyn ToolCapability>) -> ModuleResult<()> {
            self.dynamic.lock().unwrap().push(tool);
            Ok(())
        }
        async fn remove_tool(&self, id: &CapabilityId) -> ModuleResult<bool> {
            let mut bag = self.dynamic.lock().unwrap();
            let before = bag.len();
            bag.retain(|tool| tool.id() != id);
            Ok(bag.len() != before)
        }
    }

    struct Builder {
        id: String,
        tools: Vec<(String, String)>,
        deps: Vec<String>,
        log: Arc<BootLog>,
        fail_init: bool,
        fail_shutdown: bool,
        dynamic: bool,
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
                dynamic: false,
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
        fn accepting_dynamic_tools(mut self) -> Self {
            self.dynamic = true;
            self
        }
        fn build(self) -> Arc<dyn Module> {
            let mut manifest = ModuleManifest::system(&self.id, "1.0.0", "test module")
                .observes(HookPoint::TurnStart);
            if self.dynamic {
                manifest = manifest.with_capability_ownership(CapabilityOwnership::Dynamic);
            }
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
                manifest = manifest.depends_on(ModuleId::new(dep.as_str()).unwrap());
            }
            Arc::new(TestModule {
                manifest,
                tools,
                providers: Vec::new(),
                log: self.log,
                fail_init: self.fail_init,
                fail_shutdown: self.fail_shutdown,
                inits: AtomicUsize::new(0),
                dynamic: std::sync::Mutex::new(Vec::new()),
            })
        }
    }

    /// A module exporting a provider capability, to prove one module may own
    /// both a tool and a provider.
    fn dual_module(id: &str, log: &Arc<BootLog>) -> Arc<dyn Module> {
        let manifest = ModuleManifest::system(id, "1.0.0", "dual module")
            .declare_capability(
                CapabilityId::new("tool.echo").unwrap(),
                CapabilityKind::Tool,
                "echo",
            )
            .unwrap()
            .declare_capability(
                CapabilityId::new("provider.dual").unwrap(),
                CapabilityKind::Provider,
                "dual provider",
            )
            .unwrap();
        Arc::new(TestModule {
            manifest,
            tools: vec![Arc::new(Echo {
                id: CapabilityId::new("tool.echo").unwrap(),
                name: "echo".into(),
            })],
            providers: vec![Arc::new(Scripted {
                id: CapabilityId::new("provider.dual").unwrap(),
            })],
            log: Arc::clone(log),
            fail_init: false,
            fail_shutdown: false,
            inits: AtomicUsize::new(0),
            dynamic: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn module_with(
        manifest: ModuleManifest,
        tools: Vec<Arc<dyn ToolCapability>>,
        log: &Arc<BootLog>,
    ) -> Arc<dyn Module> {
        Arc::new(TestModule {
            manifest,
            tools,
            providers: Vec::new(),
            log: Arc::clone(log),
            fail_init: false,
            fail_shutdown: false,
            inits: AtomicUsize::new(0),
            dynamic: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn context() -> ModuleInitContext {
        ModuleInitContext::new(system_clock(), Arc::new(NoCredentials), TraceId::new())
    }

    #[tokio::test]
    async fn modules_start_in_dependency_order() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        // Registered in an order that contradicts the dependency chain.
        m.register(Builder::new("p.c", &log).dep("p.b").build())
            .unwrap();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.register(Builder::new("p.b", &log).dep("p.a").build())
            .unwrap();

        let order = m.start_all(&context()).await.unwrap();
        let order: Vec<&str> = order.iter().map(ModuleId::as_str).collect();

        assert_eq!(order, ["p.a", "p.b", "p.c"]);
        assert_eq!(log.entries(), ["p.a", "p.b", "p.c"]);
    }

    #[tokio::test]
    async fn without_dependencies_start_order_is_registration_order() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        // Alphabetical order would be a, b, z — registration order wins because
        // no dependency forces otherwise.
        m.register(Builder::new("p.z", &log).build()).unwrap();
        m.register(Builder::new("p.m", &log).build()).unwrap();
        m.register(Builder::new("p.a", &log).build()).unwrap();

        let order = m.start_all(&context()).await.unwrap();
        let order: Vec<&str> = order.iter().map(ModuleId::as_str).collect();
        assert_eq!(order, ["p.z", "p.m", "p.a"]);
        assert_eq!(log.entries(), ["p.z", "p.m", "p.a"]);
    }

    #[tokio::test]
    async fn shutdown_runs_in_reverse_start_order() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.register(Builder::new("p.b", &log).dep("p.a").build())
            .unwrap();

        m.start_all(&context()).await.unwrap();
        let failures = m.shutdown_all().await;

        assert!(failures.is_empty());
        assert_eq!(log.entries(), ["p.a", "p.b", "stop:p.b", "stop:p.a"]);
        assert_eq!(
            m.state(&ModuleId::new("p.a").unwrap()).unwrap(),
            Lifecycle::Stopped
        );
    }

    #[tokio::test]
    async fn one_bad_shutdown_does_not_abandon_the_others() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.register(Builder::new("p.b", &log).failing_shutdown().build())
            .unwrap();

        m.start_all(&context()).await.unwrap();
        let failures = m.shutdown_all().await;

        assert_eq!(failures.len(), 1);
        assert_eq!(
            m.state(&ModuleId::new("p.b").unwrap()).unwrap(),
            Lifecycle::Failed
        );
        assert_eq!(
            m.state(&ModuleId::new("p.a").unwrap()).unwrap(),
            Lifecycle::Stopped,
            "the healthy module must still have been stopped"
        );
    }

    #[tokio::test]
    async fn a_missing_dependency_is_caught_before_anything_starts() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).dep("p.absent").build())
            .unwrap();

        let err = m.start_all(&context()).await.unwrap_err();
        assert!(matches!(err, ModuleSystemError::MissingDependency { .. }));
        assert!(
            log.entries().is_empty(),
            "no module may run before validation completes"
        );
    }

    #[tokio::test]
    async fn a_dependency_cycle_is_caught_before_anything_starts() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).dep("p.b").build())
            .unwrap();
        m.register(Builder::new("p.b", &log).dep("p.a").build())
            .unwrap();

        let err = m.start_all(&context()).await.unwrap_err();
        match err {
            ModuleSystemError::DependencyCycle(members) => {
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
    async fn a_failing_initialize_marks_the_module_failed_and_stops_the_boot() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.register(Builder::new("p.b", &log).dep("p.a").failing_init().build())
            .unwrap();
        m.register(Builder::new("p.c", &log).dep("p.b").build())
            .unwrap();

        let err = m.start_all(&context()).await.unwrap_err();
        assert!(matches!(err, ModuleSystemError::ModuleFailed { .. }));

        assert_eq!(
            m.state(&ModuleId::new("p.a").unwrap()).unwrap(),
            Lifecycle::Active
        );
        assert_eq!(
            m.state(&ModuleId::new("p.b").unwrap()).unwrap(),
            Lifecycle::Failed
        );
        assert_eq!(
            m.state(&ModuleId::new("p.c").unwrap()).unwrap(),
            Lifecycle::Registered,
            "a module after the failure must not have been started"
        );
    }

    #[tokio::test]
    async fn a_capability_is_visible_before_start_but_not_dispatchable() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();

        let id = CapabilityId::new("tool.echo").unwrap();

        let record = m.record(&id).unwrap();
        assert_eq!(record.owner.as_str(), "p.a");
        assert!(!record.is_available(), "not started yet");

        // `Arc<dyn ToolCapability>` is not Debug, so `unwrap_err` is unavailable.
        match m.tool(&id) {
            Err(ModuleSystemError::NotActive { state, .. }) => assert_eq!(state, "registered"),
            Err(other) => panic!("expected NotActive, got {other:?}"),
            Ok(_) => panic!("an inactive module's tool must not be dispatchable"),
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
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();
        m.start_all(&context()).await.unwrap();

        match m.provider(&CapabilityId::new("tool.echo").unwrap()) {
            Err(e) => assert!(matches!(e, ModuleSystemError::KindMismatch { .. }), "{e:?}"),
            Ok(_) => panic!("a tool must not be resolvable as a provider"),
        }
    }

    #[tokio::test]
    async fn a_model_facing_name_resolves_to_its_capability() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
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

    #[tokio::test]
    async fn one_module_may_own_both_a_tool_and_a_provider() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(dual_module("vendor.dual", &log)).unwrap();
        m.start_all(&context()).await.unwrap();

        assert!(m.tool(&CapabilityId::new("tool.echo").unwrap()).is_ok());
        assert!(m
            .provider(&CapabilityId::new("provider.dual").unwrap())
            .is_ok());
        assert_eq!(m.active_tools().len(), 1);
        assert_eq!(m.active_providers().len(), 1);
    }

    #[test]
    fn a_capability_collision_rejects_the_whole_module() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();

        let err = m
            .register(Builder::new("p.b", &log).tool("tool.echo", "echo2").build())
            .unwrap_err();
        assert!(matches!(err, ModuleSystemError::DuplicateCapability { .. }));

        assert_eq!(
            m.modules().len(),
            1,
            "the rejected module must not be registered"
        );
        assert_eq!(m.capabilities().len(), 1);
    }

    #[test]
    fn a_model_facing_tool_name_collision_rejects_the_whole_module() {
        let log = Arc::new(BootLog::default());
        let mut manager = ModuleManager::new();
        manager
            .register(
                Builder::new("p.a", &log)
                    .tool("tool.calculator_a", "calculator")
                    .build(),
            )
            .unwrap();

        let error = manager
            .register(
                Builder::new("p.b", &log)
                    .tool("tool.calculator_b", "calculator")
                    .build(),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ModuleSystemError::AmbiguousToolName {
                name,
                incumbent,
                challenger,
            } if name == "calculator"
                && incumbent.as_str() == "tool.calculator_a"
                && challenger.as_str() == "tool.calculator_b"
        ));
        assert_eq!(manager.modules().len(), 1);
        assert_eq!(manager.capabilities().len(), 1);
    }

    #[test]
    fn a_declared_tool_without_an_implementation_is_rejected() {
        let log = Arc::new(BootLog::default());
        let capability = CapabilityId::new("tool.calculator").unwrap();
        let manifest = ModuleManifest::system("p.calculator", "1.0.0", "calculator")
            .declare_capability(capability.clone(), CapabilityKind::Tool, "calculator")
            .unwrap();
        let mut manager = ModuleManager::new();

        let error = manager
            .register(module_with(manifest, Vec::new(), &log))
            .unwrap_err();

        assert!(matches!(
            error,
            ModuleSystemError::MissingCapabilityImplementation {
                capability: missing,
                kind: "tool",
                ..
            } if missing == capability
        ));
        assert!(manager.modules().is_empty());
        assert!(manager.capabilities().is_empty());
    }

    #[test]
    fn a_declared_provider_without_an_implementation_is_rejected() {
        let log = Arc::new(BootLog::default());
        let capability = CapabilityId::new("provider.acme").unwrap();
        let manifest = ModuleManifest::system("p.provider", "1.0.0", "provider")
            .declare_capability(capability.clone(), CapabilityKind::Provider, "provider")
            .unwrap();
        let mut manager = ModuleManager::new();

        let error = manager
            .register(module_with(manifest, Vec::new(), &log))
            .unwrap_err();

        assert!(matches!(
            error,
            ModuleSystemError::MissingCapabilityImplementation {
                capability: missing,
                kind: "provider",
                ..
            } if missing == capability
        ));
        assert!(manager.modules().is_empty());
        assert!(manager.capabilities().is_empty());
    }

    #[test]
    fn an_undeclared_tool_implementation_is_rejected() {
        let log = Arc::new(BootLog::default());
        let capability = CapabilityId::new("tool.secret").unwrap();
        let manifest = ModuleManifest::system("p.secret", "1.0.0", "secret");
        let tools: Vec<Arc<dyn ToolCapability>> = vec![Arc::new(Echo {
            id: capability.clone(),
            name: "secret".into(),
        })];
        let mut manager = ModuleManager::new();

        let error = manager
            .register(module_with(manifest, tools, &log))
            .unwrap_err();

        assert!(matches!(
            error,
            ModuleSystemError::UndeclaredCapabilityImplementation {
                capability: undeclared,
                kind: "tool",
                ..
            } if undeclared == capability
        ));
        assert!(manager.modules().is_empty());
        assert!(manager.capabilities().is_empty());
    }

    #[tokio::test]
    async fn dispatch_reaches_the_owning_modules_implementation() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
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

    #[tokio::test]
    async fn a_dynamic_tool_is_admitted_only_after_global_checks() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();
        m.register(
            Builder::new("module.mcp", &log)
                .accepting_dynamic_tools()
                .build(),
        )
        .unwrap();
        m.start_all(&context()).await.unwrap();

        let owner = ModuleId::new("module.mcp").unwrap();
        let late = Arc::new(Echo {
            id: CapabilityId::new("tool.mcp.custom_search").unwrap(),
            name: "mcp_custom_search".into(),
        });
        m.register_dynamic_tool(&owner, late).await.unwrap();

        // Immediately dispatchable and immediately listed.
        assert!(m
            .tool(&CapabilityId::new("tool.mcp.custom_search").unwrap())
            .is_ok());
        assert_eq!(m.tool_declarations().len(), 2);
        assert!(m.tool_by_name("mcp_custom_search").is_some());
        assert_eq!(
            m.capabilities()
                .owner(&CapabilityId::new("tool.mcp.custom_search").unwrap())
                .unwrap()
                .as_str(),
            "module.mcp",
        );

        // A second dynamic tool may not steal the id...
        let same_id = Arc::new(Echo {
            id: CapabilityId::new("tool.mcp.custom_search").unwrap(),
            name: "other_name".into(),
        });
        assert!(matches!(
            m.register_dynamic_tool(&owner, same_id).await.unwrap_err(),
            ModuleSystemError::DuplicateCapability { .. }
        ));

        // ...nor the model-facing name of a builtin tool.
        let stolen_name = Arc::new(Echo {
            id: CapabilityId::new("tool.mcp.imposter").unwrap(),
            name: "echo".into(),
        });
        assert!(matches!(
            m.register_dynamic_tool(&owner, stolen_name)
                .await
                .unwrap_err(),
            ModuleSystemError::AmbiguousToolName { .. }
        ));

        // Nothing was admitted by the two rejections.
        assert_eq!(m.tool_declarations().len(), 2);
    }

    #[tokio::test]
    async fn a_module_without_dynamic_ownership_refuses_the_registration() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).build()).unwrap();
        m.start_all(&context()).await.unwrap();

        let late = Arc::new(Echo {
            id: CapabilityId::new("tool.late").unwrap(),
            name: "late".into(),
        });
        let err = m
            .register_dynamic_tool(&ModuleId::new("p.a").unwrap(), late)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ModuleSystemError::DynamicCapabilityUnsupported { .. }
        ));
        assert!(
            !m.capabilities()
                .contains(&CapabilityId::new("tool.late").unwrap()),
            "a refused registration must not be indexed"
        );
    }

    #[tokio::test]
    async fn a_dynamic_tool_can_be_unregistered_again() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(
            Builder::new("module.mcp", &log)
                .accepting_dynamic_tools()
                .build(),
        )
        .unwrap();
        m.start_all(&context()).await.unwrap();

        let owner = ModuleId::new("module.mcp").unwrap();
        let id = CapabilityId::new("tool.mcp.late").unwrap();
        m.register_dynamic_tool(
            &owner,
            Arc::new(Echo {
                id: id.clone(),
                name: "late".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(m.tool_declarations().len(), 1);

        assert!(m.unregister_dynamic_tool(&owner, &id).await.unwrap());
        assert!(!m.capabilities().contains(&id));
        assert!(m.tool_declarations().is_empty());
        assert!(!m.unregister_dynamic_tool(&owner, &id).await.unwrap());
    }

    #[test]
    fn the_hook_index_is_a_derived_view_of_declared_intent() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();
        m.register(Builder::new("p.b", &log).build()).unwrap();

        let index = m.hook_index();
        assert_eq!(
            index.get(&HookPoint::TurnStart).cloned(),
            Some(vec![
                ModuleId::new("p.a").unwrap(),
                ModuleId::new("p.b").unwrap()
            ]),
            "declared intent is listed, derived from manifests, in registration order"
        );
        assert_eq!(
            m.modules_observing(HookPoint::AfterTurn),
            Vec::<ModuleId>::new(),
            "a module that observes nothing is not listed for anything"
        );
    }

    #[tokio::test]
    async fn tool_entries_list_live_tools_with_their_owner() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).tool("tool.echo", "echo").build())
            .unwrap();
        m.start_all(&context()).await.unwrap();

        let entries = m.tool_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0.as_str(), "p.a");
        assert_eq!(entries[0].1.id().as_str(), "tool.echo");
    }

    #[test]
    fn a_failed_dynamic_registration_leaves_no_ghost_capabilities() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(
            Builder::new("p.a", &log)
                .tool("tool.taken", "taken")
                .build(),
        )
        .unwrap();

        // A dynamic owner whose manifest declares `tool.alpha` while its live
        // bag already holds a tool whose id collides with `p.a`. The rejection
        // must cost the whole registration, not just the colliding half.
        let manifest = ModuleManifest::system("module.mcp", "1.0.0", "dynamic owner")
            .with_capability_ownership(CapabilityOwnership::Dynamic)
            .declare_capability(
                CapabilityId::new("tool.alpha").unwrap(),
                CapabilityKind::Tool,
                "alpha",
            )
            .unwrap();
        let colliding = Arc::new(Echo {
            id: CapabilityId::new("tool.taken").unwrap(),
            name: "other".into(),
        });
        let mcp = Arc::new(TestModule {
            manifest,
            tools: vec![colliding],
            providers: Vec::new(),
            log: Arc::clone(&log),
            fail_init: false,
            fail_shutdown: false,
            inits: AtomicUsize::new(0),
            dynamic: std::sync::Mutex::new(Vec::new()),
        });

        let err = m.register(mcp).unwrap_err();
        assert!(matches!(err, ModuleSystemError::DuplicateCapability { .. }));
        assert!(
            !m.capabilities()
                .contains(&CapabilityId::new("tool.alpha").unwrap()),
            "the non-colliding half of the batch must not leak into the index"
        );
        assert_eq!(m.modules().len(), 1);
        assert_eq!(m.capabilities().len(), 1);
    }

    #[tokio::test]
    async fn unregistering_one_dynamic_tool_leaves_the_others_live() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(
            Builder::new("module.mcp", &log)
                .accepting_dynamic_tools()
                .build(),
        )
        .unwrap();
        m.start_all(&context()).await.unwrap();

        let owner = ModuleId::new("module.mcp").unwrap();
        let a = CapabilityId::new("tool.mcp.a").unwrap();
        let b = CapabilityId::new("tool.mcp.b").unwrap();
        m.register_dynamic_tool(
            &owner,
            Arc::new(Echo {
                id: a.clone(),
                name: "mcp_a".into(),
            }),
        )
        .await
        .unwrap();
        m.register_dynamic_tool(
            &owner,
            Arc::new(Echo {
                id: b.clone(),
                name: "mcp_b".into(),
            }),
        )
        .await
        .unwrap();
        let hooks_before = m.hook_index();

        assert!(m.unregister_dynamic_tool(&owner, &a).await.unwrap());
        assert!(!m.capabilities().contains(&a));
        assert!(m.capabilities().contains(&b));
        assert!(m.tool(&b).is_ok());
        assert!(m.tool_by_name("mcp_b").is_some());
        assert!(m.tool_by_name("mcp_a").is_none());
        // Capability churn never touches the derived hook dispatch list.
        assert_eq!(m.hook_index(), hooks_before);
    }

    #[tokio::test]
    async fn a_module_that_is_not_active_receives_no_hooks() {
        let log = Arc::new(BootLog::default());
        let mut m = ModuleManager::new();
        m.register(Builder::new("p.a", &log).build()).unwrap();

        // The declaration view lists intent immediately ...
        assert_eq!(
            m.modules_observing(HookPoint::TurnStart),
            vec![ModuleId::new("p.a").unwrap()]
        );
        // ... but a module that has not started receives no runtime hook.
        assert!(m.dispatchable_observers(HookPoint::TurnStart).is_empty());

        m.start_all(&context()).await.unwrap();
        assert_eq!(m.dispatchable_observers(HookPoint::TurnStart).len(), 1);

        m.shutdown_all().await;
        assert!(
            m.dispatchable_observers(HookPoint::TurnStart).is_empty(),
            "a stopped module receives no runtime hook"
        );
    }
}

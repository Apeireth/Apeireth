//! Host authorization of existing stores; not a module registry or a migration.
use apeireth_memory::{MemoryCoordinator, MemoryScope};
use apeireth_plugin::{agents::AgentPrincipal, ToolExecutionContext};
use apeireth_runtime::canonical::module::{
    BehaviorModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use apeireth_runtime::canonical::principal_authority::{PrincipalAuthorizations, PrincipalGrant};
use apeireth_runtime::RuntimeBuilder;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

fn key(p: &AgentPrincipal) -> String {
    format!("{}/{}", p.profile, p.namespace)
}
/// Scope grants are read-only. Writes always target the target's namespace partition.
#[derive(Clone)]
pub struct ProfileMemoryAccess {
    pub coordinator: Arc<MemoryCoordinator>,
    pub read_scopes: Vec<MemoryScope>,
    pub writable_private: bool,
}
impl ProfileMemoryAccess {
    pub fn scopes(&self, principal: &AgentPrincipal) -> Vec<MemoryScope> {
        let mut scopes = self.read_scopes.clone();
        let private = MemoryScope::session(crate::memory_module::namespace_key(principal));
        if !scopes.contains(&private) {
            scopes.push(private);
        }
        scopes
    }
}
/// Store identity is supplied by the trusted Host, never by tool parameters.
/// Writable stores must be dedicated resources; use one stable ID per backing store.
#[derive(Clone)]
pub struct CognitiveStoreBinding {
    pub store: String,
    pub access: StoreAccess,
    pub module: Arc<dyn BehaviorModule>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StoreAccess {
    ReadOnlyShared,
    WritablePrivate,
}
#[derive(Clone)]
pub(crate) struct MappingLease {
    pub authority: PrincipalGrant,
    pub memory: Option<ProfileMemoryAccess>,
    modules: BTreeMap<String, Arc<dyn BehaviorModule>>,
}
#[derive(Default)]
struct Bindings {
    targets: BTreeMap<String, MappingLease>,
    // Reservations survive revoke: revocation does not erase ownership of durable data.
    owners: BTreeMap<String, Option<String>>,
}
#[derive(Default)]
pub struct ProfileMemoryMappings {
    authority: Arc<PrincipalAuthorizations>,
    bindings: Mutex<Bindings>,
}
impl ProfileMemoryMappings {
    pub fn authority(&self) -> Arc<PrincipalAuthorizations> {
        self.authority.clone()
    }
    /// Replacing a grant revokes its epoch. Existing sessions cannot adopt a new grant.
    pub fn grant(
        &self,
        principal: &AgentPrincipal,
        memory: Option<ProfileMemoryAccess>,
        stores: Vec<CognitiveStoreBinding>,
    ) -> Result<(), String> {
        let target = key(principal);
        let mut bindings = self.bindings.lock().unwrap_or_else(|e| e.into_inner());
        let mut owners = bindings.owners.clone();
        let mut modules = BTreeMap::new();
        for binding in stores {
            if binding.store.trim().is_empty() {
                return Err("source store identity is required".into());
            }
            if binding.access == StoreAccess::ReadOnlyShared
                && !binding.module.read_only_profile_projection()
            {
                return Err("module does not provide a read-only shared projection".into());
            }
            let owner = (binding.access == StoreAccess::WritablePrivate).then_some(target.clone());
            if owners.get(&binding.store).is_some_and(|old| old != &owner) {
                return Err(
                    "source store is already owned or shared; writable sharing is forbidden".into(),
                );
            }
            if binding.access == StoreAccess::WritablePrivate
                && bindings.targets.iter().any(|(other, lease)| {
                    other != &target
                        && lease
                            .modules
                            .values()
                            .any(|module| Arc::ptr_eq(module, &binding.module))
                })
            {
                return Err("module resource already belongs to another Profile".into());
            }
            owners.insert(binding.store, owner);
            let id = binding.module.manifest().id.clone();
            if modules.insert(id, binding.module).is_some() {
                return Err("duplicate mapped behavior".into());
            }
        }
        let authority = self.authority.grant(principal, modules.keys().cloned());
        bindings.owners = owners;
        bindings.targets.insert(
            target,
            MappingLease {
                authority,
                memory,
                modules,
            },
        );
        Ok(())
    }
    pub fn revoke(&self, principal: &AgentPrincipal) {
        let mut bindings = self.bindings.lock().unwrap_or_else(|e| e.into_inner());
        self.authority.revoke(principal);
        bindings.targets.remove(&key(principal));
    }
    pub(crate) fn lease(&self, ctx: &ToolExecutionContext) -> Option<MappingLease> {
        let principal = ctx.principal.as_ref()?;
        let bindings = self.bindings.lock().unwrap_or_else(|e| e.into_inner());
        let lease = bindings.targets.get(&key(principal))?;
        (lease.authority.is_active()
            && ctx.authorization.principal_epoch.as_deref() == Some(lease.authority.epoch()))
        .then(|| lease.clone())
    }
    /// Wraps the existing registered compatibility entry, including its legacy path.
    pub fn wrap(self: &Arc<Self>, legacy: Arc<dyn BehaviorModule>) -> Arc<dyn BehaviorModule> {
        Arc::new(MappedBehavior {
            manifest: legacy.manifest().clone(),
            legacy: Some(legacy),
            mappings: self.clone(),
        })
    }
    /// Selects all mapped behavior IDs once, through the existing Runtime builder.
    /// Legacy modules retain their original order; extra mapped slots are deterministic.
    pub fn register_into(
        self: &Arc<Self>,
        mut builder: RuntimeBuilder,
        legacy: &[Arc<dyn BehaviorModule>],
    ) -> RuntimeBuilder {
        let mut extra = BTreeMap::new();
        for lease in self
            .bindings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .targets
            .values()
        {
            for (id, module) in &lease.modules {
                extra.insert(id.clone(), module.manifest().clone());
            }
        }
        let mut registered = Vec::new();
        for module in legacy {
            extra.remove(&module.manifest().id);
            registered.push(self.wrap(module.clone()));
        }
        for manifest in extra.into_values() {
            registered.push(Arc::new(MappedBehavior {
                manifest,
                legacy: None,
                mappings: self.clone(),
            }) as Arc<dyn BehaviorModule>);
        }
        // These concrete memory slots form one ordered pipeline, including when
        // only a mapped Profile supplies them. Other modules retain their order.
        #[cfg(feature = "desktop")]
        order_memory_modules(&mut registered);
        for module in registered {
            builder = builder.with_registered_behavior(module);
        }
        builder.with_principal_authorizations(self.authority())
    }
}

#[cfg(feature = "desktop")]
fn order_memory_modules(modules: &mut Vec<Arc<dyn BehaviorModule>>) {
    let mut prefix = Vec::new();
    for id in ["cognitive.memory_recovery", "cognitive.memory_correction"] {
        if let Some(index) = modules.iter().position(|module| module.manifest().id == id) {
            prefix.push(modules.remove(index));
        }
    }
    let index = modules
        .iter()
        .position(|module| {
            matches!(
                module.manifest().id.as_str(),
                "cognitive.memory_recall" | "cognitive.memory_writeback"
            )
        })
        .unwrap_or(modules.len());
    modules.splice(index..index, prefix);
}
struct MappedBehavior {
    manifest: ModuleManifest,
    legacy: Option<Arc<dyn BehaviorModule>>,
    mappings: Arc<ProfileMemoryMappings>,
}
#[async_trait]
impl BehaviorModule for MappedBehavior {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn committed_evidence_access_for_principal(
        &self,
        principal: Option<&AgentPrincipal>,
    ) -> apeireth_runtime::CommittedEvidenceAccess {
        match principal {
            None => self
                .legacy
                .as_ref()
                .map(|module| module.committed_evidence_access_for_principal(None))
                .unwrap_or_default(),
            Some(principal) => self
                .mappings
                .bindings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .targets
                .get(&key(principal))
                .filter(|lease| lease.authority.is_active())
                .and_then(|lease| lease.modules.get(&self.manifest.id))
                .map(|module| module.committed_evidence_access_for_principal(Some(principal)))
                .unwrap_or_default(),
        }
    }
    fn persists_recoverable_operations_for_principal(
        &self,
        principal: Option<&AgentPrincipal>,
    ) -> bool {
        match principal {
            None => self
                .legacy
                .as_ref()
                .is_some_and(|module| module.persists_recoverable_operations_for_principal(None)),
            Some(principal) => self
                .mappings
                .bindings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .targets
                .get(&key(principal))
                .filter(|lease| lease.authority.is_active())
                .and_then(|lease| lease.modules.get(&self.manifest.id))
                .is_some_and(|module| {
                    module.persists_recoverable_operations_for_principal(Some(principal))
                }),
        }
    }
    async fn on_hook_authenticated(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
        execution: &ToolExecutionContext,
    ) -> Result<ModuleOutcome, ModuleError> {
        if execution.principal.is_none() {
            return match &self.legacy {
                Some(module) => module.on_hook_authenticated(hook, ctx, execution).await,
                None => Ok(ModuleOutcome::continue_()),
            };
        }
        let Some(lease) = self.mappings.lease(execution) else {
            return Ok(ModuleOutcome::continue_());
        };
        let Some(module) = lease.modules.get(&self.manifest.id) else {
            return Ok(ModuleOutcome::continue_());
        };
        let result = module.on_hook_authenticated(hook, ctx, execution).await;
        if !lease.authority.is_active() {
            return Err(ModuleError::Message("profile mapping revoked".into()));
        }
        result
    }
}

#[cfg(feature = "desktop")]
impl ProfileMemoryAccess {
    /// Host supplies the official journal for this private Cognitive DB.
    /// Grant all returned bindings together; recovery and correction share the writer.
    pub fn recoverable_modules(
        &self,
        principal: &AgentPrincipal,
        clock: Arc<dyn apeireth_core::kernel::Clock>,
        journal: Arc<apeireth_memory::writeback_recovery::SqliteWritebackJournal>,
        mappings: Arc<ProfileMemoryMappings>,
        store: &str,
    ) -> Result<Vec<CognitiveStoreBinding>, String> {
        let writer = Arc::new(
            self.writeback_module(principal, clock.clone())?
                .with_recovery_journal(journal.clone()),
        );
        writer.attach_profile_mappings(mappings.clone());
        let correction = crate::canonical::memory_correction::MemoryCorrectionModule::new(
            self.coordinator.clone(),
            clock,
        )
        .with_journal(journal)
        .with_profile_partition(crate::memory_module::namespace_key(principal))
        .with_profile_mappings(mappings);
        let modules: Vec<Arc<dyn BehaviorModule>> = vec![
            Arc::new(crate::canonical::memory_recovery::MemoryRecoveryModule::new(writer.clone())),
            Arc::new(correction),
            Arc::new(self.recall_module(principal)),
            writer,
        ];
        Ok(modules
            .into_iter()
            .map(|module| CognitiveStoreBinding {
                store: store.into(),
                access: StoreAccess::WritablePrivate,
                module,
            })
            .collect())
    }
    pub fn recall_module(
        &self,
        principal: &AgentPrincipal,
    ) -> crate::canonical::cognitive::MemoryRecallModule {
        crate::canonical::cognitive::MemoryRecallModule::new(self.coordinator.backend_arc())
            .with_coordinator(self.coordinator.clone())
            .with_profile_partition(crate::memory_module::namespace_key(principal))
            .with_authorized_scopes(self.scopes(principal))
    }
    pub fn writeback_module(
        &self,
        principal: &AgentPrincipal,
        clock: Arc<dyn apeireth_core::kernel::Clock>,
    ) -> Result<crate::canonical::cognitive::MemoryWritebackModule, String> {
        if !self.writable_private {
            return Err("read-only memory mapping cannot write back".into());
        }
        Ok(crate::canonical::cognitive::MemoryWritebackModule::new(
            self.coordinator.backend_arc(),
            clock,
        )
        .with_coordinator(self.coordinator.clone())
        .with_profile_partition(crate::memory_module::namespace_key(principal)))
    }
}

/// Explicit legacy preference sessions from one Host-selected store. Never falls
/// back to the caller's session or a global preference query; all mutations deny.
pub struct SharedPreferenceSource {
    store: Arc<dyn apeireth_plugin::preference::PreferenceStore>,
    sessions: Vec<apeireth_core::kernel::SessionId>,
}
impl SharedPreferenceSource {
    pub fn new(
        store: Arc<dyn apeireth_plugin::preference::PreferenceStore>,
        sessions: Vec<apeireth_core::kernel::SessionId>,
    ) -> Self {
        Self { store, sessions }
    }
}
impl apeireth_plugin::preference::PreferenceStore for SharedPreferenceSource {
    fn record(
        &self,
        _: &apeireth_plugin::preference::UserPreference,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<()> {
        Err("shared preference source is read-only".into())
    }
    fn forget(&self, _: &str) -> apeireth_plugin::memory_backend::CapabilityResult<()> {
        Err("shared preference source is read-only".into())
    }
    fn recall_for_context(
        &self,
        _: &apeireth_core::kernel::SessionId,
        topic: &str,
        limit: u32,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<
        Vec<apeireth_plugin::preference::UserPreference>,
    > {
        let mut result = Vec::new();
        for session in &self.sessions {
            for pref in self.store.recall_for_context(session, topic, limit)? {
                if self.sessions.contains(&pref.session_id)
                    && !result
                        .iter()
                        .any(|p: &apeireth_plugin::preference::UserPreference| p.id == pref.id)
                {
                    result.push(pref);
                }
            }
        }
        result.sort_by(|a, b| {
            b.confidence
                .total_cmp(&a.confidence)
                .then_with(|| b.created_at.cmp(&a.created_at))
                .then_with(|| a.id.cmp(&b.id))
        });
        result.truncate(limit as usize);
        Ok(result)
    }
    fn list_for_session(
        &self,
        session: &apeireth_core::kernel::SessionId,
    ) -> apeireth_plugin::memory_backend::CapabilityResult<
        Vec<apeireth_plugin::preference::UserPreference>,
    > {
        if !self.sessions.contains(session) {
            return Ok(vec![]);
        }
        Ok(self
            .store
            .list_for_session(session)?
            .into_iter()
            .filter(|pref| pref.session_id == *session)
            .collect())
    }
}

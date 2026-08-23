//! The canonical registries.
//!
//! # One truth, split by question
//!
//! [`PluginRegistry`] answers *"which plugins exist and what state is each in"*.
//! [`CapabilityRegistry`] answers *"who owns this capability id"*.
//!
//! Crucially, [`CapabilityRegistry`] is an **index, not a copy**. It maps
//! [`CapabilityId`] to the owning [`PluginId`] and stops there; the capability's
//! declaration continues to live in exactly one place, the owner's manifest. This
//! is the difference between a derived view and a second source of truth, and it
//! is the whole reason the registries can be trusted to agree.
//!
//! Other registries may be built on top as typed views — "give me every tool",
//! "give me every provider" — and [`crate::manager::PluginManager`] provides
//! exactly those. What they may not do is store their own copy of the facts.
//!
//! Both registries iterate in id order, so boot logs, capability listings, and
//! test assertions are reproducible.

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Lifecycle, PluginId};

use crate::capability::{CapabilityDescriptor, CapabilityKind};
use crate::error::{PluginError, PluginResult};
use crate::plugin::Plugin;

/// A registered plugin and its current state.
pub(crate) struct PluginEntry {
    pub(crate) plugin: Arc<dyn Plugin>,
    pub(crate) state: Lifecycle,
}

/// Which plugins exist, and what state each is in.
#[derive(Default)]
pub struct PluginRegistry {
    entries: BTreeMap<PluginId, PluginEntry>,
}

impl PluginRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a plugin in the [`Lifecycle::Registered`] state.
    ///
    /// Rejects a duplicate id rather than replacing: two plugins under one id
    /// means the effective behaviour depends on registration order.
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) -> PluginResult<()> {
        let id = plugin.manifest().id.clone();
        if self.entries.contains_key(&id) {
            return Err(PluginError::DuplicatePlugin(id));
        }
        self.entries.insert(
            id,
            PluginEntry {
                plugin,
                state: Lifecycle::Registered,
            },
        );
        Ok(())
    }

    /// Whether `id` is registered.
    pub fn contains(&self, id: &PluginId) -> bool {
        self.entries.contains_key(id)
    }

    /// The plugin registered under `id`.
    pub fn get(&self, id: &PluginId) -> PluginResult<&Arc<dyn Plugin>> {
        self.entries
            .get(id)
            .map(|e| &e.plugin)
            .ok_or_else(|| PluginError::UnknownPlugin(id.clone()))
    }

    /// The lifecycle state of `id`.
    pub fn state(&self, id: &PluginId) -> PluginResult<Lifecycle> {
        self.entries
            .get(id)
            .map(|e| e.state)
            .ok_or_else(|| PluginError::UnknownPlugin(id.clone()))
    }

    /// Apply a checked lifecycle transition to `id`.
    pub fn transition(&mut self, id: &PluginId, next: Lifecycle) -> PluginResult<()> {
        let entry = self
            .entries
            .get_mut(id)
            .ok_or_else(|| PluginError::UnknownPlugin(id.clone()))?;
        entry.state = entry.state.transition_to(id.as_str(), next)?;
        Ok(())
    }

    /// Registered plugin ids, in id order.
    pub fn ids(&self) -> impl Iterator<Item = &PluginId> {
        self.entries.keys()
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn entry(&self, id: &PluginId) -> Option<&PluginEntry> {
        self.entries.get(id)
    }
}

/// Which plugin owns which capability id.
///
/// An index over the manifests, never a copy of them.
#[derive(Debug, Default, Clone)]
pub struct CapabilityRegistry {
    owners: BTreeMap<CapabilityId, PluginId>,
}

impl CapabilityRegistry {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Index every capability a plugin declares.
    ///
    /// Rejects a capability already claimed by another plugin, naming both
    /// claimants. Registration is all-or-nothing: a manifest whose third
    /// capability collides leaves the index exactly as it was, so a rejected
    /// plugin cannot half-appear in the system.
    pub fn index(
        &mut self,
        owner: &PluginId,
        declared: &[CapabilityDescriptor],
    ) -> PluginResult<()> {
        for descriptor in declared {
            if let Some(incumbent) = self.owners.get(&descriptor.id) {
                return Err(PluginError::DuplicateCapability {
                    capability: descriptor.id.clone(),
                    incumbent: incumbent.clone(),
                    challenger: owner.clone(),
                });
            }
        }
        for descriptor in declared {
            self.owners.insert(descriptor.id.clone(), owner.clone());
        }
        Ok(())
    }

    /// The plugin that owns `id`.
    pub fn owner(&self, id: &CapabilityId) -> PluginResult<&PluginId> {
        self.owners
            .get(id)
            .ok_or_else(|| PluginError::UnknownCapability(id.clone()))
    }

    /// Whether `id` is known.
    pub fn contains(&self, id: &CapabilityId) -> bool {
        self.owners.contains_key(id)
    }

    /// Every known capability id, in id order.
    pub fn ids(&self) -> impl Iterator<Item = &CapabilityId> {
        self.owners.keys()
    }

    /// Known capability ids of a given kind, in id order.
    ///
    /// Filters on the id's reserved prefix, which
    /// [`CapabilityDescriptor::new`] has already checked against the declared
    /// kind, so this cannot disagree with the manifest.
    pub fn ids_of_kind(&self, kind: CapabilityKind) -> impl Iterator<Item = &CapabilityId> {
        self.owners
            .keys()
            .filter(move |id| id.kind_segment() == kind.id_prefix())
    }

    /// Number of indexed capabilities.
    pub fn len(&self) -> usize {
        self.owners.len()
    }

    /// Whether nothing is indexed.
    pub fn is_empty(&self) -> bool {
        self.owners.is_empty()
    }
}

/// A capability together with who owns it and whether it can be dispatched to.
///
/// Assembled on demand by borrowing from the registries; holds no state of its
/// own, so it cannot drift from them.
#[derive(Debug, Clone, Copy)]
pub struct CapabilityRecord<'a> {
    /// The declaration, borrowed from its owner's manifest.
    pub descriptor: &'a CapabilityDescriptor,
    /// The plugin that declared it.
    pub owner: &'a PluginId,
    /// The owner's current lifecycle state.
    pub state: Lifecycle,
}

impl CapabilityRecord<'_> {
    /// Whether this capability can currently be dispatched to.
    pub const fn is_available(&self) -> bool {
        self.state.is_dispatchable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PluginManifest;
    use crate::plugin::PluginContext;
    use async_trait::async_trait;

    struct Stub(PluginManifest);

    #[async_trait]
    impl Plugin for Stub {
        fn manifest(&self) -> &PluginManifest {
            &self.0
        }
        async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
            Ok(())
        }
        async fn shutdown(&self) -> PluginResult<()> {
            Ok(())
        }
    }

    fn stub(id: &str, caps: &[(&str, CapabilityKind)]) -> Arc<dyn Plugin> {
        let mut m = PluginManifest::new(PluginId::new(id).unwrap(), "1.0.0", "stub");
        for (cap, kind) in caps {
            m = m
                .declare_capability(CapabilityId::new(*cap).unwrap(), *kind, "stub capability")
                .unwrap();
        }
        Arc::new(Stub(m))
    }

    #[test]
    fn a_duplicate_plugin_id_is_rejected_rather_than_replacing() {
        let mut reg = PluginRegistry::new();
        reg.register(stub("builtin.a", &[])).unwrap();
        let err = reg.register(stub("builtin.a", &[])).unwrap_err();
        assert!(matches!(err, PluginError::DuplicatePlugin(_)));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn registration_starts_in_registered_and_transitions_are_checked() {
        let mut reg = PluginRegistry::new();
        reg.register(stub("builtin.a", &[])).unwrap();
        let id = PluginId::new("builtin.a").unwrap();

        assert_eq!(reg.state(&id).unwrap(), Lifecycle::Registered);
        assert!(
            reg.transition(&id, Lifecycle::Active).is_err(),
            "must not skip init"
        );

        reg.transition(&id, Lifecycle::Initializing).unwrap();
        reg.transition(&id, Lifecycle::Active).unwrap();
        assert_eq!(reg.state(&id).unwrap(), Lifecycle::Active);
    }

    #[test]
    fn two_plugins_cannot_claim_one_capability() {
        let a = stub("builtin.a", &[("tool.shell", CapabilityKind::Tool)]);
        let b = stub("builtin.b", &[("tool.shell", CapabilityKind::Tool)]);

        let mut caps = CapabilityRegistry::new();
        caps.index(&a.manifest().id, &a.manifest().capabilities)
            .unwrap();

        let err = caps
            .index(&b.manifest().id, &b.manifest().capabilities)
            .unwrap_err();

        match err {
            PluginError::DuplicateCapability {
                capability,
                incumbent,
                challenger,
            } => {
                assert_eq!(capability.as_str(), "tool.shell");
                assert_eq!(incumbent.as_str(), "builtin.a");
                assert_eq!(challenger.as_str(), "builtin.b");
            }
            other => panic!("expected DuplicateCapability, got {other:?}"),
        }
    }

    #[test]
    fn a_rejected_manifest_leaves_the_index_untouched() {
        let a = stub("builtin.a", &[("tool.shell", CapabilityKind::Tool)]);
        // `b` collides on its *second* capability; the first must not survive.
        let b = stub(
            "builtin.b",
            &[
                ("tool.unique", CapabilityKind::Tool),
                ("tool.shell", CapabilityKind::Tool),
            ],
        );

        let mut caps = CapabilityRegistry::new();
        caps.index(&a.manifest().id, &a.manifest().capabilities)
            .unwrap();
        assert!(caps
            .index(&b.manifest().id, &b.manifest().capabilities)
            .is_err());

        assert_eq!(caps.len(), 1, "partial registration must not leak");
        assert!(!caps.contains(&CapabilityId::new("tool.unique").unwrap()));
    }

    #[test]
    fn the_index_answers_ownership_and_filters_by_kind() {
        let p = stub(
            "vendor.acme",
            &[
                ("provider.acme", CapabilityKind::Provider),
                ("tool.acme_search", CapabilityKind::Tool),
            ],
        );
        let mut caps = CapabilityRegistry::new();
        caps.index(&p.manifest().id, &p.manifest().capabilities)
            .unwrap();

        let provider = CapabilityId::new("provider.acme").unwrap();
        assert_eq!(caps.owner(&provider).unwrap().as_str(), "vendor.acme");

        let tools: Vec<&str> = caps
            .ids_of_kind(CapabilityKind::Tool)
            .map(CapabilityId::as_str)
            .collect();
        assert_eq!(tools, ["tool.acme_search"]);

        let providers: Vec<&str> = caps
            .ids_of_kind(CapabilityKind::Provider)
            .map(CapabilityId::as_str)
            .collect();
        assert_eq!(providers, ["provider.acme"]);
    }

    #[test]
    fn an_unknown_capability_is_reported_rather_than_defaulted() {
        let caps = CapabilityRegistry::new();
        let err = caps
            .owner(&CapabilityId::new("tool.absent").unwrap())
            .unwrap_err();
        assert!(matches!(err, PluginError::UnknownCapability(_)));
    }

    #[test]
    fn iteration_is_id_ordered_regardless_of_registration_order() {
        let mut reg = PluginRegistry::new();
        reg.register(stub("builtin.z", &[])).unwrap();
        reg.register(stub("builtin.a", &[])).unwrap();
        reg.register(stub("builtin.m", &[])).unwrap();

        let ids: Vec<&str> = reg.ids().map(PluginId::as_str).collect();
        assert_eq!(ids, ["builtin.a", "builtin.m", "builtin.z"]);
    }
}

//! The canonical registries.
//!
//! # One truth, split by question
//!
//! [`ModuleRegistry`] answers *"which modules exist, in what state, and in what
//! order were they registered"*. [`CapabilityRegistry`] answers *"who owns this
//! capability id"*.
//!
//! Crucially, [`CapabilityRegistry`] is an **index, not a copy**. It maps
//! [`CapabilityId`] to the owning [`ModuleId`] and stops there; the capability's
//! declaration continues to live in exactly one place, the owner's manifest. This
//! is the difference between a derived view and a second source of truth, and it
//! is the whole reason the registries can be trusted to agree.
//!
//! Other views may be built on top — "give me every tool", "give me every
//! module that observes `TurnStart`" — and [`crate::manager::ModuleManager`]
//! provides exactly those. What they may not do is store their own copy of the
//! facts.
//!
//! Both registries iterate in id order, so boot logs, capability listings, and
//! test assertions are reproducible. Registration order is separately recorded
//! (see [`ModuleRegistry::seq`]) because start order must respect declared
//! dependencies while otherwise preserving the order the embedder chose.

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Lifecycle, ModuleId};

use crate::capability::{CapabilityDescriptor, CapabilityKind};
use crate::error::{ModuleResult, ModuleSystemError};
use crate::module::Module;

/// A module entry borrowed for the duration of a query.
pub(crate) struct ModuleEntryRef<'a> {
    pub(crate) module: &'a Arc<dyn Module>,
    pub(crate) state: Lifecycle,
}

/// One module's canonical record.
///
/// The module, its lifecycle state, and its registration sequence number live
/// in one struct so they cannot drift apart: there is no removal or replacement
/// path to desynchronize parallel stores, and adding one later cannot forget a
/// half of a module's bookkeeping.
struct ModuleEntry {
    /// Registration sequence number. Lower means registered earlier.
    seq: u64,
    /// The module object itself — the only copy.
    module: Arc<dyn Module>,
    /// Current lifecycle state.
    state: Lifecycle,
}

/// Which modules exist, and what state each is in.
///
/// The canonical ownership source: the `Arc<dyn Module>` objects live here and
/// nowhere else. One [`BTreeMap`] keyed by [`ModuleId`] holds each module's
/// entire record; registration order is derived from each entry's `seq` and is
/// never held in a second structure that could disagree.
#[derive(Default)]
pub struct ModuleRegistry {
    /// The one ownership store.
    entries: BTreeMap<ModuleId, ModuleEntry>,
    /// Next sequence number to hand out.
    next_seq: u64,
}

impl ModuleRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a module in the [`Lifecycle::Registered`] state.
    ///
    /// Rejects a duplicate id rather than replacing: two modules under one id
    /// means the effective behaviour depends on registration order. Also
    /// rejects an empty id, which could never be addressed.
    pub fn register(&mut self, module: Arc<dyn Module>) -> ModuleResult<()> {
        let id = module.manifest().id.clone();
        if self.entries.contains_key(&id) {
            return Err(ModuleSystemError::DuplicateModule(id));
        }
        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.insert(
            id,
            ModuleEntry {
                seq,
                module,
                state: Lifecycle::Registered,
            },
        );
        Ok(())
    }

    /// Whether `id` is registered.
    pub fn contains(&self, id: &ModuleId) -> bool {
        self.entries.contains_key(id)
    }

    /// The module registered under `id`.
    pub fn get(&self, id: &ModuleId) -> ModuleResult<&Arc<dyn Module>> {
        self.entry_of(id).map(|entry| &entry.module)
    }

    /// The lifecycle state of `id`.
    pub fn state(&self, id: &ModuleId) -> ModuleResult<Lifecycle> {
        self.entry_of(id).map(|entry| entry.state)
    }

    /// Apply a checked lifecycle transition to `id`.
    pub fn transition(&mut self, id: &ModuleId, next: Lifecycle) -> ModuleResult<()> {
        let entry = self
            .entries
            .get_mut(id)
            .ok_or_else(|| ModuleSystemError::UnknownModule(id.clone()))?;
        let state = entry.state.transition_to(id.as_str(), next)?;
        entry.state = state;
        Ok(())
    }

    /// Registration sequence number of `id`. Lower means registered earlier.
    ///
    /// This is what makes a dependency-respecting start order otherwise equal
    /// to the embedder's registration order.
    pub fn seq(&self, id: &ModuleId) -> ModuleResult<u64> {
        self.entry_of(id).map(|entry| entry.seq)
    }

    /// Every registered module, in registration order.
    pub fn modules(&self) -> Vec<&Arc<dyn Module>> {
        self.ordered_entries()
            .into_iter()
            .map(|entry| &entry.module)
            .collect()
    }

    /// Registered module ids, in id order.
    pub fn ids(&self) -> impl Iterator<Item = &ModuleId> {
        self.entries.keys()
    }

    /// Number of registered modules.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn entry_of(&self, id: &ModuleId) -> ModuleResult<&ModuleEntry> {
        self.entries
            .get(id)
            .ok_or_else(|| ModuleSystemError::UnknownModule(id.clone()))
    }

    /// Entries in registration order, derived from each `seq`.
    fn ordered_entries(&self) -> Vec<&ModuleEntry> {
        let mut ordered: Vec<&ModuleEntry> = self.entries.values().collect();
        ordered.sort_by_key(|entry| entry.seq);
        ordered
    }

    pub(crate) fn entry(&self, id: &ModuleId) -> Option<ModuleEntryRef<'_>> {
        let entry = self.entries.get(id)?;
        Some(ModuleEntryRef {
            module: &entry.module,
            state: entry.state,
        })
    }
}

impl<'a> IntoIterator for &'a ModuleRegistry {
    type Item = &'a Arc<dyn Module>;
    type IntoIter = std::vec::IntoIter<&'a Arc<dyn Module>>;

    fn into_iter(self) -> Self::IntoIter {
        self.modules().into_iter()
    }
}

/// Which module owns which capability id.
///
/// An index over the manifests, never a copy of them.
#[derive(Debug, Default, Clone)]
pub struct CapabilityRegistry {
    owners: BTreeMap<CapabilityId, ModuleId>,
}

impl CapabilityRegistry {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Index every capability a module declares.
    ///
    /// Rejects a capability already claimed by another module, naming both
    /// claimants. Registration is all-or-nothing: a manifest whose third
    /// capability collides leaves the index exactly as it was, so a rejected
    /// module cannot half-appear in the system.
    pub fn index(
        &mut self,
        owner: &ModuleId,
        declared: &[CapabilityDescriptor],
    ) -> ModuleResult<()> {
        let ids: Vec<CapabilityId> = declared.iter().map(|d| d.id.clone()).collect();
        self.index_ids(owner, &ids)
    }

    /// Index capability ids an owner holds, whether or not they are declared in
    /// a manifest.
    ///
    /// A dynamic owner's tools have no manifest declaration, so this is the path
    /// that makes them dispatchable and listed. Rejects an id already claimed by
    /// anyone, naming both claimants, and is all-or-nothing.
    pub fn index_ids(&mut self, owner: &ModuleId, ids: &[CapabilityId]) -> ModuleResult<()> {
        for id in ids {
            if let Some(incumbent) = self.owners.get(id) {
                return Err(ModuleSystemError::DuplicateCapability {
                    capability: id.clone(),
                    incumbent: incumbent.clone(),
                    challenger: owner.clone(),
                });
            }
        }
        for id in ids {
            self.owners.insert(id.clone(), owner.clone());
        }
        Ok(())
    }

    /// The module that owns `id`.
    pub fn owner(&self, id: &CapabilityId) -> ModuleResult<&ModuleId> {
        self.owners
            .get(id)
            .ok_or_else(|| ModuleSystemError::UnknownCapability(id.clone()))
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
    /// kind, so this cannot disagree with the manifest. The same convention is
    /// what lets a dynamically registered capability be classified without a
    /// manifest declaration.
    pub fn ids_of_kind(&self, kind: CapabilityKind) -> impl Iterator<Item = &CapabilityId> {
        self.owners
            .keys()
            .filter(move |id| id.kind_segment() == kind.id_prefix())
    }

    /// Drop `id` from the index. Returns whether it was there.
    pub fn unindex(&mut self, id: &CapabilityId) -> bool {
        self.owners.remove(id).is_some()
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
    /// The module that declared it.
    pub owner: &'a ModuleId,
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
    use crate::manifest::ModuleManifest;
    use async_trait::async_trait;

    struct Stub(ModuleManifest);

    #[async_trait]
    impl crate::module::Module for Stub {
        fn manifest(&self) -> &ModuleManifest {
            &self.0
        }
    }

    fn stub(id: &str, caps: &[(&str, CapabilityKind)]) -> Arc<dyn crate::module::Module> {
        let mut m = ModuleManifest::system(id, "1.0.0", "stub");
        for (cap, kind) in caps {
            m = m
                .declare_capability(CapabilityId::new(*cap).unwrap(), *kind, "stub capability")
                .unwrap();
        }
        Arc::new(Stub(m))
    }

    #[tokio::test]
    async fn a_duplicate_module_id_is_rejected_rather_than_replacing() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("builtin.a", &[])).unwrap();
        let err = reg.register(stub("builtin.a", &[])).unwrap_err();
        assert!(matches!(err, ModuleSystemError::DuplicateModule(_)));
        assert_eq!(reg.len(), 1);
    }

    #[tokio::test]
    async fn registration_starts_in_registered_and_transitions_are_checked() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("builtin.a", &[])).unwrap();
        let id = ModuleId::new("builtin.a").unwrap();

        assert_eq!(reg.state(&id).unwrap(), Lifecycle::Registered);
        assert!(
            reg.transition(&id, Lifecycle::Active).is_err(),
            "must not skip init"
        );

        reg.transition(&id, Lifecycle::Initializing).unwrap();
        reg.transition(&id, Lifecycle::Active).unwrap();
        assert_eq!(reg.state(&id).unwrap(), Lifecycle::Active);
    }

    #[tokio::test]
    async fn registration_order_is_recorded_independently_of_id_order() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("builtin.z", &[])).unwrap();
        reg.register(stub("builtin.a", &[])).unwrap();

        // ids iterate in id order...
        let ids: Vec<&str> = reg.ids().map(ModuleId::as_str).collect();
        assert_eq!(ids, ["builtin.a", "builtin.z"]);

        // ...while modules and sequence numbers keep the embedder's order, which
        // is what a dependency-respecting start order preserves.
        assert_eq!(reg.seq(&ModuleId::new("builtin.z").unwrap()).unwrap(), 0);
        assert_eq!(reg.seq(&ModuleId::new("builtin.a").unwrap()).unwrap(), 1);
        let order: Vec<&str> = reg
            .modules()
            .iter()
            .map(|m| m.manifest().id.as_str())
            .collect();
        assert_eq!(order, ["builtin.z", "builtin.a"]);
    }

    #[tokio::test]
    async fn two_modules_cannot_claim_one_capability() {
        let a = stub("builtin.a", &[("tool.shell", CapabilityKind::Tool)]);
        let b = stub("builtin.b", &[("tool.shell", CapabilityKind::Tool)]);

        let mut caps = CapabilityRegistry::new();
        caps.index(&a.manifest().id, &a.manifest().capabilities)
            .unwrap();

        let err = caps
            .index(&b.manifest().id, &b.manifest().capabilities)
            .unwrap_err();

        match err {
            ModuleSystemError::DuplicateCapability {
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

    #[tokio::test]
    async fn a_rejected_manifest_leaves_the_index_untouched() {
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

    #[tokio::test]
    async fn the_index_answers_ownership_and_filters_by_kind() {
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

    #[tokio::test]
    async fn an_unknown_capability_is_reported_rather_than_defaulted() {
        let caps = CapabilityRegistry::new();
        let err = caps
            .owner(&CapabilityId::new("tool.absent").unwrap())
            .unwrap_err();
        assert!(matches!(err, ModuleSystemError::UnknownCapability(_)));
    }

    #[tokio::test]
    async fn a_dynamic_capability_can_be_unindexed() {
        let mut caps = CapabilityRegistry::new();
        let id = CapabilityId::new("tool.mcp.late").unwrap();
        caps.index(
            &ModuleId::new("module.mcp").unwrap(),
            &[CapabilityDescriptor::new(id.clone(), CapabilityKind::Tool, "late").unwrap()],
        )
        .unwrap();
        assert!(caps.contains(&id));
        assert!(caps.unindex(&id));
        assert!(!caps.contains(&id));
    }

    #[test]
    fn a_manifest_only_module_is_a_complete_module() {
        // The module trait's lifecycle defaults mean a manifest-only stub needs
        // nothing else: there is no second "behavior-only" trait to implement.
        let module = stub("builtin.inert", &[]);
        assert_eq!(module.manifest().id.as_str(), "builtin.inert");
    }
}

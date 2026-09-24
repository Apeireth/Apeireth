//! Capability routing: *where* a capability executes, and never *what* it is.
//!
//! # Why this is not a second registry
//!
//! This module answers exactly one question — `CapabilityId` under an explicit
//! scope → [`CapabilityRoute`] — and it stores exactly one kind of fact: mesh
//! routing metadata. Local routes are **derived** from the canonical
//! `ModuleManager` indexed view on every query and never copied here. Remote
//! routes are **read-only projections** of peer adverts: identity, owner,
//! availability. A [`RemoteCapability`] structurally cannot hold an
//! implementation — there is no `Arc<dyn ToolCapability>` field to put one in —
//! so a remote advert cannot create a second implementation ownership. The
//! implementation lives where it always lived: inside the origin node's module.
//!
//! # Namespacing instead of precedence
//!
//! Local and remote capabilities live in separate slots
//! (`Local` vs `Remote(node)` per id), so a remote advert can never silently
//! override a local capability of the same id — the two simply coexist as
//! distinct routes, and the caller picks one explicitly through
//! [`RouteSelection`]. No first-wins, no hidden precedence.

use std::collections::BTreeMap;

use apeireth_core::kernel::{CapabilityId, ModuleId};
use apeireth_mesh_protocol::{
    CapabilityAdvert, LocatedModuleId, MeshCapabilityKind, ModuleLocation,
};

use crate::error::MeshError;

/// Where one capability executes: its logical owner plus its physical location.
///
/// Identity stays where it belongs — [`ModuleId`] for the module,
/// [`CapabilityId`] for the capability — and [`ModuleLocation`] is pure routing
/// metadata. `ModuleOrigin::System` on `ModuleLocation::Remote(node)` is a
/// perfectly ordinary combination: provenance and placement are orthogonal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CapabilityRoute {
    /// The module that owns the implementation (its `ModuleId`).
    pub owner: ModuleId,
    /// Where that module's runtime sits.
    pub location: ModuleLocation,
}

impl CapabilityRoute {
    /// A route to this runtime.
    pub fn local(owner: ModuleId) -> Self {
        Self {
            owner,
            location: ModuleLocation::Local,
        }
    }

    /// A route to another runtime.
    pub fn remote(owner: LocatedModuleId) -> Self {
        Self {
            owner: owner.module,
            location: ModuleLocation::Remote(owner.node),
        }
    }

    /// Compose from a module id and a location.
    pub fn from_parts(owner: ModuleId, location: ModuleLocation) -> Self {
        Self { owner, location }
    }

    /// The network-layer owner identity, for remote routes.
    pub fn located_owner(&self) -> Option<LocatedModuleId> {
        match &self.location {
            ModuleLocation::Local => None,
            ModuleLocation::Remote(node) => Some(LocatedModuleId {
                node: node.clone(),
                module: self.owner.clone(),
            }),
        }
    }
}

/// The caller's explicit choice of route scope.
///
/// Selection is always explicit: a mesh must never silently prefer one
/// namespace over the other. `Local` matches only this runtime's canonical
/// index; `Remote(node)` matches only that node's projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteSelection {
    /// Execute in this runtime.
    Local,
    /// Execute in the named peer's runtime.
    Remote(apeireth_core::kernel::NodeId),
}

impl std::fmt::Display for RouteSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => f.write_str("local"),
            Self::Remote(node) => write!(f, "remote:{node}"),
        }
    }
}

/// One capability as advertised by a remote node.
///
/// A projection entry — identity and availability only. It is not, and cannot
/// become, an implementation: remote execution always travels back to the
/// owner node's `ModuleManager`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCapability {
    /// Capability identity, unchanged by the network hop.
    pub id: CapabilityId,
    /// Category, validated against the id's reserved prefix on ingest.
    pub kind: MeshCapabilityKind,
    /// Network-layer owner: which module on which node.
    pub owner: LocatedModuleId,
    /// What it does, in one line, when the advert carried one.
    pub description: String,
    /// Whether the owning module reported it dispatchable in the latest advert.
    pub available: bool,
}

/// One capability in this runtime's own canonical indexed view.
///
/// Produced by the executor's `catalog()` — i.e. straight from the
/// `ModuleManager`'s derived views — and never stored anywhere; routing reads
/// it through at query time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityCatalogEntry {
    /// Capability identity.
    pub id: CapabilityId,
    /// Category, recovered from the id's reserved prefix.
    pub kind: MeshCapabilityKind,
    /// Owning module inside this runtime.
    pub owner: ModuleId,
    /// What it does, in one line.
    pub description: String,
    /// Whether the owning module can currently dispatch it.
    pub available: bool,
}

/// Routing metadata for every remote route, namespaced by `(node, capability)`.
#[derive(Debug, Default)]
pub struct RouteTable {
    remote: BTreeMap<(apeireth_core::kernel::NodeId, CapabilityId), RemoteCapability>,
}

impl RouteTable {
    /// An empty table: no remote routes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest one peer advert as a **full snapshot**.
    ///
    /// Whatever the advert omits is removed from that peer's projection — this
    /// is how a remote capability disappearing propagates. Other peers'
    /// projections are untouched, and no local fact is consulted or affected:
    /// an advert can only ever speak for its own `(node, capability)` slots.
    pub fn ingest_advert(
        &mut self,
        node: &apeireth_core::kernel::NodeId,
        advert: &CapabilityAdvert,
    ) -> Result<(), MeshError> {
        if !advert.is_well_formed() {
            return Err(MeshError::Malformed {
                sender: node.to_string(),
                reason: "advertised capability kind disagrees with its id prefix".to_string(),
            });
        }

        self.forget_projection(node);
        for (module, entry) in advert.entries() {
            let key = (node.clone(), entry.id.clone());
            self.remote.insert(
                key,
                RemoteCapability {
                    id: entry.id.clone(),
                    kind: entry.kind,
                    owner: LocatedModuleId::new(node.clone(), module.clone()),
                    description: entry.description.clone(),
                    available: entry.available,
                },
            );
        }
        Ok(())
    }

    /// Drop `node`'s projection entirely (explicit forget; a disconnect does
    /// *not* do this — see the peer state for that).
    pub fn forget_projection(&mut self, node: &apeireth_core::kernel::NodeId) {
        self.remote.retain(|(n, _), _| n != node);
    }

    /// One peer's projected capabilities, in capability-id order.
    pub fn projection(&self, node: &apeireth_core::kernel::NodeId) -> Vec<&RemoteCapability> {
        self.remote
            .iter()
            .filter(|((n, _), _)| n == node)
            .map(|(_, capability)| capability)
            .collect()
    }

    /// Every projected remote capability, in `(node, id)` order.
    pub fn remote_capabilities(&self) -> impl Iterator<Item = &RemoteCapability> {
        self.remote.values()
    }

    /// All routes known for `capability`: the local route (from the canonical
    /// view passed in) first, then remote routes in node order.
    ///
    /// Listing order is presentation only. It grants no precedence: dispatch
    /// happens through [`RouteTable::resolve`], which demands an explicit
    /// selection and cannot cross namespaces.
    pub fn routes_for(
        &self,
        capability: &CapabilityId,
        local_view: &[CapabilityCatalogEntry],
    ) -> Vec<CapabilityRoute> {
        let mut routes = Vec::new();
        if let Some(entry) = local_view.iter().find(|entry| &entry.id == capability) {
            routes.push(CapabilityRoute::from_parts(
                entry.owner.clone(),
                ModuleLocation::Local,
            ));
        }
        for ((node, id), remote) in &self.remote {
            if id == capability {
                routes.push(CapabilityRoute::from_parts(
                    remote.owner.module.clone(),
                    ModuleLocation::Remote(node.clone()),
                ));
            }
        }
        routes
    }

    /// Resolve `capability` under an explicit [`RouteSelection`].
    ///
    /// Strictly namespaced: `Local` never falls through to a remote route and
    /// `Remote(node)` never falls through to the local one, so a remote advert
    /// can shadow nothing and a local capability can hide nothing.
    pub fn resolve(
        &self,
        capability: &CapabilityId,
        selection: &RouteSelection,
        local_view: &[CapabilityCatalogEntry],
    ) -> Result<CapabilityRoute, MeshError> {
        match selection {
            RouteSelection::Local => local_view
                .iter()
                .find(|entry| &entry.id == capability)
                .map(|entry| {
                    CapabilityRoute::from_parts(entry.owner.clone(), ModuleLocation::Local)
                })
                .ok_or_else(|| MeshError::RouteUnavailable {
                    capability: capability.clone(),
                    selection: selection.to_string(),
                    detail: "this runtime's canonical capability index has no such capability"
                        .to_string(),
                }),
            RouteSelection::Remote(node) => self
                .remote
                .get(&(node.clone(), capability.clone()))
                .filter(|remote| remote.available)
                .map(|remote| {
                    CapabilityRoute::from_parts(
                        remote.owner.module.clone(),
                        ModuleLocation::Remote(node.clone()),
                    )
                })
                .ok_or_else(|| MeshError::RouteUnavailable {
                    capability: capability.clone(),
                    selection: selection.to_string(),
                    detail: format!("peer {node} has no available projection for this capability"),
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::NodeId;
    use apeireth_mesh_protocol::{CapabilityAdvertEntry, ModuleAdvert};

    fn node(id: &str) -> apeireth_core::kernel::NodeId {
        NodeId::new(id).unwrap()
    }

    fn cap(id: &str) -> CapabilityId {
        CapabilityId::new(id).unwrap()
    }

    fn module(id: &str) -> ModuleId {
        ModuleId::new(id).unwrap()
    }

    fn local_view(id: &str, owner: &str) -> Vec<CapabilityCatalogEntry> {
        vec![CapabilityCatalogEntry {
            id: cap(id),
            kind: MeshCapabilityKind::Tool,
            owner: module(owner),
            description: "local".into(),
            available: true,
        }]
    }

    fn advert(id: &str) -> CapabilityAdvert {
        CapabilityAdvert {
            modules: vec![ModuleAdvert {
                module: module("builtin.mesh-test"),
                capabilities: vec![CapabilityAdvertEntry {
                    id: cap(id),
                    kind: MeshCapabilityKind::Tool,
                    description: "remote".into(),
                    available: true,
                }],
            }],
        }
    }

    #[test]
    fn a_remote_advert_never_overrides_a_local_capability_of_the_same_id() {
        let mut table = RouteTable::new();
        table
            .ingest_advert(&node("node-b"), &advert("tool.mesh.echo"))
            .unwrap();

        // Both routes coexist in separate namespace slots…
        let routes = table.routes_for(
            &cap("tool.mesh.echo"),
            &local_view("tool.mesh.echo", "builtin.local"),
        );
        assert_eq!(routes.len(), 2);

        // …and an explicit local selection is unaffected by the remote advert.
        let resolved = table
            .resolve(
                &cap("tool.mesh.echo"),
                &RouteSelection::Local,
                &local_view("tool.mesh.echo", "builtin.local"),
            )
            .unwrap();
        assert_eq!(resolved.location, ModuleLocation::Local);
        assert_eq!(resolved.owner, module("builtin.local"));
    }

    #[test]
    fn a_remote_selection_never_falls_through_to_the_local_route() {
        let mut table = RouteTable::new();
        // Local only; nothing from node-b.
        let err = table
            .resolve(
                &cap("tool.mesh.echo"),
                &RouteSelection::Remote(node("node-b")),
                &local_view("tool.mesh.echo", "builtin.local"),
            )
            .unwrap_err();
        assert!(matches!(err, MeshError::RouteUnavailable { .. }));
    }

    #[test]
    fn the_same_module_id_on_two_nodes_stays_distinguishable() {
        let mut table = RouteTable::new();
        table
            .ingest_advert(&node("node-b"), &advert("tool.mesh.echo"))
            .unwrap();
        table
            .ingest_advert(&node("node-c"), &advert("tool.mesh.echo"))
            .unwrap();

        let b = table
            .resolve(
                &cap("tool.mesh.echo"),
                &RouteSelection::Remote(node("node-b")),
                &[],
            )
            .unwrap();
        let c = table
            .resolve(
                &cap("tool.mesh.echo"),
                &RouteSelection::Remote(node("node-c")),
                &[],
            )
            .unwrap();
        assert_eq!(b.owner, c.owner, "same module id on both nodes");
        assert_ne!(b.location, c.location, "but distinct locations");
        assert_eq!(
            b.located_owner().unwrap().to_string(),
            "node-b/builtin.mesh-test"
        );
    }

    #[test]
    fn an_advert_is_a_full_snapshot_so_removals_propagate() {
        let mut table = RouteTable::new();
        table
            .ingest_advert(&node("node-b"), &advert("tool.mesh.echo"))
            .unwrap();
        assert_eq!(table.projection(&node("node-b")).len(), 1);

        // The next advert no longer mentions the capability: it is gone.
        table
            .ingest_advert(&node("node-b"), &CapabilityAdvert { modules: vec![] })
            .unwrap();
        assert!(table.projection(&node("node-b")).is_empty());
    }

    #[test]
    fn a_malformed_advert_is_refused_rather_than_trusted() {
        let mut bad = advert("tool.mesh.echo");
        bad.modules[0].capabilities[0].kind = MeshCapabilityKind::Provider;
        let mut table = RouteTable::new();
        assert!(table.ingest_advert(&node("node-b"), &bad).is_err());
        assert!(table.remote_capabilities().next().is_none());
    }

    #[test]
    fn a_remote_capability_is_metadata_and_cannot_hold_an_implementation() {
        // Structural guarantee asserted at runtime as well as by the type: the
        // projection stores only identity/owner/availability/description.
        let mut table = RouteTable::new();
        table
            .ingest_advert(&node("node-b"), &advert("tool.mesh.echo"))
            .unwrap();
        let remote = table.projection(&node("node-b"))[0].clone();
        let dumped = format!("{remote:?}");
        for forbidden in ["Arc", "dyn ", "fn(", "closure"] {
            assert!(
                !dumped.contains(forbidden),
                "projection leaked {forbidden:?}"
            );
        }
    }
}

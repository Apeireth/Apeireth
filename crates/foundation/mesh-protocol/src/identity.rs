//! Mesh identity and projection types.
//!
//! Three identities, deliberately kept apart:
//!
//! - [`NodeId`] — *which runtime* (re-exported from `apeireth-core`, the one
//!   canonical node identity);
//! - [`LocatedModuleId`] — *which module on which node*, the network-layer owner
//!   identity, because two nodes may each own a `builtin.tools` and neither is
//!   wrong;
//! - [`apeireth_core::kernel::CapabilityId`] — *what can be done*, unchanged.
//!
//! Physical placement is [`ModuleLocation`]: routing metadata, never identity.
//! `ModuleOrigin` (system / installed / development) answers a different
//! question — how a module arrived — and the two axes never mix.

use apeireth_core::kernel::{CapabilityId, NodeId, PluginId};
use serde::{Deserialize, Serialize};

pub use apeireth_core::kernel::NodeId as CanonicalNodeId;

/// Where a module's implementation physically runs.
///
/// The location axis, kept separate from provenance (`ModuleOrigin`) and from
/// identity (`PluginId`). It is routing metadata only: `System`-origin modules
/// may live on a remote node and installed modules may live here, and neither
/// combination changes anything about how a capability behaves.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleLocation {
    /// The implementation runs in this runtime.
    Local,
    /// The implementation runs in another runtime.
    Remote(NodeId),
}

impl ModuleLocation {
    /// Whether the implementation runs in this runtime.
    pub const fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }

    /// The remote node, when this is a remote location.
    pub const fn remote_node(&self) -> Option<&NodeId> {
        match self {
            Self::Local => None,
            Self::Remote(node) => Some(node),
        }
    }
}

impl std::fmt::Display for ModuleLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => f.write_str("local"),
            Self::Remote(node) => write!(f, "remote:{node}"),
        }
    }
}

/// A module identity as the mesh sees it: `(node, module)`.
///
/// `PluginId` remains unique *within* one runtime and nothing about the local
/// registries changes; this composite exists only because module ids are not
/// globally unique across nodes. It is a network-layer owner identity, not a
/// replacement for [`PluginId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocatedModuleId {
    /// The runtime whose registry owns the module implementation.
    pub node: NodeId,
    /// The module id inside that runtime's registry.
    pub module: PluginId,
}

impl LocatedModuleId {
    /// Compose the network-layer owner identity.
    pub fn new(node: NodeId, module: PluginId) -> Self {
        Self { node, module }
    }

    /// This owner, expressed as a [`ModuleLocation`] pair for routing.
    pub fn location(&self) -> ModuleLocation {
        ModuleLocation::Remote(self.node.clone())
    }
}

impl std::fmt::Display for LocatedModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.node, self.module)
    }
}

/// The category of a capability as it travels on the wire.
///
/// Mirrors the canonical `CapabilityKind` (which lives in `apeireth-plugin`,
/// above this crate in the dependency graph) exactly: same seven categories,
/// same reserved id prefixes, same invariant that an id's prefix names its
/// kind. The wire carries its own vocabulary by design — adapters translate,
/// and the plugin-side kind never leaks onto the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshCapabilityKind {
    /// Something a model can call.
    Tool,
    /// Something that can serve a completion.
    Provider,
    /// Durable recall.
    Memory,
    /// A channel to an external process or peer.
    Transport,
    /// Something that watches the runtime.
    Observer,
    /// Something that decides when work runs.
    Scheduler,
    /// A capability that does not fit the categories above.
    Extension,
}

impl MeshCapabilityKind {
    /// The reserved id prefix for this kind.
    pub const fn id_prefix(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Provider => "provider",
            Self::Memory => "memory",
            Self::Transport => "transport",
            Self::Observer => "observer",
            Self::Scheduler => "scheduler",
            Self::Extension => "extension",
        }
    }

    /// Every kind, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Tool,
        Self::Provider,
        Self::Memory,
        Self::Transport,
        Self::Observer,
        Self::Scheduler,
        Self::Extension,
    ];

    /// The kind whose prefix matches this id, if any.
    pub fn from_id(id: &CapabilityId) -> Option<Self> {
        let prefix = id.kind_segment();
        Self::ALL.into_iter().find(|k| k.id_prefix() == prefix)
    }
}

impl std::fmt::Display for MeshCapabilityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id_prefix())
    }
}

/// What a node says about itself during the handshake.
///
/// Identity plus compatibility facts, and nothing else. Capability listings
/// deliberately do **not** live here: an advert is a snapshot/projection of a
/// node's *current* capability surface, while a descriptor is who the node
/// *is*. Mixing them would recreate a second source of truth for capability
/// ownership on every Hello.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDescriptor {
    /// Canonical node identity, stable across restarts.
    pub id: NodeId,
    /// Display name. Cosmetic; never an identity.
    pub name: String,
    /// The mesh wire protocol version this node speaks.
    pub protocol_version: u32,
    /// The runtime build version, for diagnostics and compatibility reporting.
    pub runtime_version: String,
}

impl NodeDescriptor {
    /// Describe a node.
    pub fn new(
        id: NodeId,
        name: impl Into<String>,
        protocol_version: u32,
        runtime_version: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            protocol_version,
            runtime_version: runtime_version.into(),
        }
    }
}

/// One advertised capability: identity and availability, never an implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityAdvertEntry {
    /// Capability identity, e.g. `tool.mesh.echo`.
    pub id: CapabilityId,
    /// Category. Must agree with the id's reserved prefix.
    pub kind: MeshCapabilityKind,
    /// What it does, in one line. Optional colour for diagnostics.
    #[serde(default)]
    pub description: String,
    /// Whether the owning module can currently dispatch it.
    pub available: bool,
    /// The model-facing declaration, for tool-kind entries only.
    ///
    /// A read-only projection of the owning tool's canonical `NormalizedTool`
    /// — enough for a peer to surface the tool in its own model catalog, and
    /// still not an implementation: nothing here can be executed. Absent for
    /// non-tool kinds and for adverts from protocol builds that predate this
    /// field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declaration: Option<apeireth_protocol::canonical::NormalizedTool>,
}

impl CapabilityAdvertEntry {
    /// Advertise a tool entry with its model-facing declaration.
    pub fn tool(
        id: CapabilityId,
        description: impl Into<String>,
        available: bool,
        declaration: apeireth_protocol::canonical::NormalizedTool,
    ) -> Self {
        Self {
            id,
            kind: MeshCapabilityKind::Tool,
            description: description.into(),
            available,
            declaration: Some(declaration),
        }
    }

    /// Advertise a non-tool entry (no model-facing declaration).
    pub fn non_tool(
        id: CapabilityId,
        kind: MeshCapabilityKind,
        description: impl Into<String>,
        available: bool,
    ) -> Self {
        Self {
            id,
            kind,
            description: description.into(),
            available,
            declaration: None,
        }
    }
}

/// One advertised module and the capabilities it currently exports.
///
/// A read-only projection of the origin node's canonical indexed view — the
/// origin node's `PluginRegistry` remains the only owner of the implementation,
/// and nothing here can be executed locally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleAdvert {
    /// The owning module id (namespaced by the advertising node).
    pub module: PluginId,
    /// Capabilities this module currently exports.
    pub capabilities: Vec<CapabilityAdvertEntry>,
}

impl ModuleAdvert {
    /// Whether a declaration and its id prefix agree about the kind, and tool
    /// entries carry their model-facing declaration.
    ///
    /// The kind invariant is the same one `CapabilityDescriptor::new` enforces
    /// locally; an advert that violates either rule is malformed and is refused
    /// on ingest.
    pub fn entries_are_well_formed(&self) -> bool {
        self.capabilities.iter().all(|entry| {
            MeshCapabilityKind::from_id(&entry.id) == Some(entry.kind)
                && (entry.kind != MeshCapabilityKind::Tool || entry.declaration.is_some())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> NodeId {
        NodeId::new(id).unwrap()
    }

    #[test]
    fn a_located_module_distinguishes_same_named_modules_on_two_nodes() {
        // Two nodes may each own `builtin.tools`; `(node, module)` keeps them
        // apart where bare `PluginId` cannot.
        let a = LocatedModuleId::new(node("node-a"), PluginId::new("builtin.tools").unwrap());
        let b = LocatedModuleId::new(node("node-b"), PluginId::new("builtin.tools").unwrap());
        assert_ne!(a, b);
        assert_eq!(a.module, b.module);
        assert_eq!(a.to_string(), "node-a/builtin.tools");
    }

    #[test]
    fn location_is_metadata_and_never_identity() {
        let local = ModuleLocation::Local;
        let remote = ModuleLocation::Remote(node("node-b"));
        assert!(local.is_local());
        assert!(!remote.is_local());
        assert_eq!(remote.remote_node().unwrap().as_str(), "node-b");
        assert_eq!(local.remote_node(), None);
    }

    #[test]
    fn wire_kind_keeps_the_same_reserved_prefixes_as_the_canonical_kind() {
        for kind in MeshCapabilityKind::ALL {
            let id = CapabilityId::new(format!("{}.example", kind.id_prefix())).unwrap();
            assert_eq!(MeshCapabilityKind::from_id(&id), Some(kind));
        }
    }

    #[test]
    fn advert_entries_are_checked_against_their_id_prefix() {
        let good = ModuleAdvert {
            module: PluginId::new("builtin.mesh-test").unwrap(),
            capabilities: vec![CapabilityAdvertEntry::tool(
                CapabilityId::new("tool.mesh.echo").unwrap(),
                "echo",
                true,
                apeireth_protocol::canonical::NormalizedTool {
                    name: "mesh_echo".into(),
                    description: Some("echo".into()),
                    parameters: apeireth_protocol::canonical::ToolParameters::new(),
                    strict: false,
                },
            )],
        };
        assert!(good.entries_are_well_formed());

        let mut bad = good.clone();
        bad.capabilities[0].kind = MeshCapabilityKind::Provider;
        assert!(!bad.entries_are_well_formed());
    }

    #[test]
    fn node_descriptor_round_trips_through_json() {
        let d = NodeDescriptor::new(node("node-a"), "laptop", 1, "2.0.0-rc.1");
        let back: NodeDescriptor =
            serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(d, back);
    }
}

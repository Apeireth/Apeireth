//! Where a model-facing tool name executes: pure routing semantics.
//!
//! # One question
//!
//! The model emits `ToolCall.name`. The runtime executes a `CapabilityId`.
//! This module answers "which execution target does this name denote" with a
//! derived, ephemeral [`ToolRouteTable`] rebuilt from two read-only views —
//! the local canonical index and the remote capability port's projection — and
//! then executes **exactly one** [`ToolExecutionTarget`]. There is no
//! fallback routing ("try local, else try remote"): names resolve once, and
//! what they resolve to is what runs.
//!
//! # Catalog merge is not ownership merge
//!
//! Nothing here is stored: entries are rebuilt per query, contain no
//! implementations, and remote declarations never enter `ModuleManager` or
//! `CapabilityRegistry`. Local and remote names share one namespace *for
//! resolution*, and collisions are ambiguous rather than resolved by
//! precedence — see [`ToolRouteTable::derive`].
//!
//! # Collision policy
//!
//! | collision | behaviour |
//! | --- | --- |
//! | local / local | impossible (registration fails closed) |
//! | local / remote name | ambiguous: excluded from the model catalog |
//! | remote / remote name | ambiguous: excluded from the model catalog |
//!
//! An ambiguous name stays reachable through explicit mesh route APIs
//! (the high-level mesh invocation entry with a route selection); it simply
//! cannot appear in front of a model as a random `"search"`.

use apeireth_core::kernel::{CapabilityId, NodeId, PluginId};
use apeireth_mesh::{RemoteToolDescriptor, RemoteToolRoute};
use apeireth_protocol::canonical::NormalizedTool;

/// Where one resolved tool name executes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolExecutionTarget {
    /// A capability owned by a module in this runtime.
    Local {
        /// Canonical capability identity.
        capability: CapabilityId,
        /// Owning module in this runtime.
        owner: PluginId,
    },
    /// A capability owned by a module on another runtime.
    Remote {
        /// The node whose runtime will execute the call.
        node: NodeId,
        /// Canonical capability identity, preserved end-to-end.
        capability: CapabilityId,
        /// Owning module id on that node.
        owner: PluginId,
    },
}

impl ToolExecutionTarget {
    /// The canonical capability identity.
    pub fn capability(&self) -> &CapabilityId {
        match self {
            Self::Local { capability, .. } | Self::Remote { capability, .. } => capability,
        }
    }

    /// The remote route, for the capability port.
    pub fn remote_route(&self) -> Option<RemoteToolRoute> {
        match self {
            Self::Local { .. } => None,
            Self::Remote {
                node,
                capability,
                owner,
            } => Some(RemoteToolRoute {
                node: node.clone(),
                module: owner.clone(),
                capability: capability.clone(),
            }),
        }
    }
}

/// One routable model-facing tool: a name, its declaration, and where it runs.
///
/// Ephemeral and rebuildable: derived from live views at query time, holding
/// no implementation of any kind.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolRouteEntry {
    /// Canonical capability identity.
    pub capability_id: CapabilityId,
    /// The name the model emits in a `ToolCall`.
    pub tool_name: String,
    /// The declaration offered to the model.
    pub declaration: NormalizedTool,
    /// Where execution happens.
    pub target: ToolExecutionTarget,
}

/// Why a model-facing name does not resolve to exactly one target.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ToolRouteError {
    /// No local or remote tool currently provides this name.
    #[error("no tool named {tool_name:?} is currently available")]
    NotFound {
        /// The name the model emitted.
        tool_name: String,
    },
    /// More than one distinct target provides this name. Fail closed: the
    /// model is never shown a randomly-preferred one.
    #[error("tool name {tool_name:?} is ambiguous across routes: {candidates}")]
    Ambiguous {
        /// The contested name.
        tool_name: String,
        /// The contending targets, for a legible error.
        candidates: String,
    },
}

/// A derived, ephemeral name → target routing table.
#[derive(Debug, Default)]
pub struct ToolRouteTable {
    entries: Vec<ToolRouteEntry>,
    ambiguous: Vec<(String, Vec<ToolExecutionTarget>)>,
}

/// One local candidate read from the canonical index.
pub struct LocalToolView {
    /// Owning module in this runtime.
    pub owner: PluginId,
    /// The live tool object's identity.
    pub capability_id: CapabilityId,
    /// Its model-facing declaration.
    pub declaration: NormalizedTool,
}

impl ToolRouteTable {
    /// Rebuild the table from the two read-only views.
    ///
    /// Local candidates come from the canonical manager (live, dispatchable,
    /// index-admitted tools only); remote candidates from the capability
    /// port's fresh projection (peer active, advertised available). A name
    /// claimed by more than one distinct target is recorded as ambiguous and
    /// kept out of [`ToolRouteTable::model_catalog`] — never resolved by
    /// first/last/nearest-wins.
    pub fn derive(local: Vec<LocalToolView>, remote: Vec<RemoteToolDescriptor>) -> Self {
        use std::collections::BTreeMap;

        // First-seen order is preserved so that a no-mesh catalog is
        // byte-identical to the local declaration order it replaces.
        let mut all: Vec<ToolRouteEntry> = Vec::new();

        for view in local {
            let tool_name = view.declaration.name.clone();
            all.push(ToolRouteEntry {
                capability_id: view.capability_id.clone(),
                tool_name,
                declaration: view.declaration,
                target: ToolExecutionTarget::Local {
                    capability: view.capability_id,
                    owner: view.owner,
                },
            });
        }
        for desc in remote {
            let tool_name = desc.declaration.name.clone();
            all.push(ToolRouteEntry {
                capability_id: desc.capability.clone(),
                tool_name,
                declaration: desc.declaration,
                target: ToolExecutionTarget::Remote {
                    node: desc.node,
                    capability: desc.capability,
                    owner: desc.module,
                },
            });
        }

        // A name is contested when two entries with distinct targets claim it.
        let mut targets_by_name: BTreeMap<String, Vec<ToolExecutionTarget>> = BTreeMap::new();
        for entry in &all {
            let targets = targets_by_name.entry(entry.tool_name.clone()).or_default();
            if !targets.contains(&entry.target) {
                targets.push(entry.target.clone());
            }
        }

        let mut entries = Vec::new();
        let mut ambiguous = Vec::new();
        for (name, targets) in targets_by_name {
            if targets.len() == 1 {
                entries.extend(all.iter().filter(|entry| entry.tool_name == name).cloned());
            } else {
                ambiguous.push((name, targets));
            }
        }

        Self { entries, ambiguous }
    }

    /// Resolve a model-facing name to exactly one target.
    pub fn resolve(&self, tool_name: &str) -> Result<&ToolRouteEntry, ToolRouteError> {
        match self
            .entries
            .iter()
            .find(|entry| entry.tool_name == tool_name)
        {
            Some(entry) => Ok(entry),
            None => {
                if let Some((_, candidates)) =
                    self.ambiguous.iter().find(|(name, _)| name == tool_name)
                {
                    let listed: Vec<String> = candidates
                        .iter()
                        .map(|target| match target {
                            ToolExecutionTarget::Local { capability, owner } => {
                                format!("local:{owner}/{capability}")
                            }
                            ToolExecutionTarget::Remote {
                                node,
                                capability,
                                owner,
                            } => format!("remote:{node}/{owner}/{capability}"),
                        })
                        .collect();
                    Err(ToolRouteError::Ambiguous {
                        tool_name: tool_name.to_string(),
                        candidates: listed.join(", "),
                    })
                } else {
                    Err(ToolRouteError::NotFound {
                        tool_name: tool_name.to_string(),
                    })
                }
            }
        }
    }

    /// The model-facing catalog: every unambiguous, available declaration.
    ///
    /// Ambiguous names are excluded here — the model never sees a randomly
    /// preferred `"search"`. Local entries keep their derived order; remote
    /// entries follow in table order.
    pub fn model_catalog(&self) -> Vec<NormalizedTool> {
        self.entries
            .iter()
            .map(|entry| entry.declaration.clone())
            .collect()
    }

    /// Whether a name is currently contested across routes.
    pub fn is_ambiguous(&self, tool_name: &str) -> bool {
        self.ambiguous.iter().any(|(name, _)| name == tool_name)
    }

    /// Contested names and their contending targets (diagnostics).
    pub fn ambiguous_names(&self) -> &[(String, Vec<ToolExecutionTarget>)] {
        &self.ambiguous
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ToolParameters;

    fn decl(name: &str) -> NormalizedTool {
        NormalizedTool {
            name: name.into(),
            description: Some("test".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    fn local(owner: &str, capability: &str, name: &str) -> LocalToolView {
        LocalToolView {
            owner: PluginId::new(owner).unwrap(),
            capability_id: CapabilityId::new(capability).unwrap(),
            declaration: decl(name),
        }
    }

    fn remote(node: &str, module: &str, capability: &str, name: &str) -> RemoteToolDescriptor {
        RemoteToolDescriptor {
            node: NodeId::new(node).unwrap(),
            module: PluginId::new(module).unwrap(),
            capability: CapabilityId::new(capability).unwrap(),
            declaration: decl(name),
            available: true,
        }
    }

    #[test]
    fn a_unique_name_resolves_to_exactly_one_target() {
        let table = ToolRouteTable::derive(
            vec![local("builtin.t", "tool.a", "alpha")],
            vec![remote(
                "node-b",
                "builtin.mesh-test",
                "tool.mesh.echo",
                "mesh_echo",
            )],
        );
        assert_eq!(table.model_catalog().len(), 2);
        let entry = table.resolve("alpha").unwrap();
        assert!(matches!(entry.target, ToolExecutionTarget::Local { .. }));
        let entry = table.resolve("mesh_echo").unwrap();
        assert!(matches!(entry.target, ToolExecutionTarget::Remote { .. }));
    }

    #[test]
    fn a_local_remote_name_collision_is_ambiguous_not_preferred() {
        let table = ToolRouteTable::derive(
            vec![local("builtin.t", "tool.search", "search")],
            vec![remote("node-b", "builtin.x", "tool.x", "search")],
        );
        assert!(matches!(
            table.resolve("search"),
            Err(ToolRouteError::Ambiguous { .. })
        ));
        assert_eq!(
            table.model_catalog().len(),
            0,
            "ambiguous names never reach the model"
        );
        assert!(table.is_ambiguous("search"));
    }

    #[test]
    fn a_remote_remote_name_collision_is_ambiguous_too() {
        let table = ToolRouteTable::derive(
            vec![],
            vec![
                remote("node-b", "builtin.x", "tool.x", "search"),
                remote("node-c", "builtin.y", "tool.y", "search"),
            ],
        );
        assert!(matches!(
            table.resolve("search"),
            Err(ToolRouteError::Ambiguous { .. })
        ));
        assert_eq!(table.model_catalog().len(), 0);
    }

    #[test]
    fn the_same_capability_id_with_distinct_names_stays_routable_on_both_sides() {
        // Local and remote `tool.mesh.echo` under different names are two
        // distinct, individually routable entries — namespacing by route.
        let table = ToolRouteTable::derive(
            vec![local("builtin.mesh-test", "tool.mesh.echo", "echo_local")],
            vec![remote(
                "node-b",
                "builtin.mesh-test",
                "tool.mesh.echo",
                "echo_remote",
            )],
        );
        assert_eq!(table.model_catalog().len(), 2);
        assert!(table.resolve("echo_local").is_ok());
        assert!(table.resolve("echo_remote").is_ok());
    }

    #[test]
    fn an_unknown_name_reports_not_found() {
        let table = ToolRouteTable::derive(vec![], vec![]);
        assert!(matches!(
            table.resolve("nope"),
            Err(ToolRouteError::NotFound { .. })
        ));
    }
}

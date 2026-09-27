//! The dynamic tool directory: what external tools are currently live.
//!
//! One entry per discovered tool. Registration is reject-on-collision: a
//! second tool with the same model-facing name (`mcp:<server>:<tool>`) or the
//! same capability id is refused, never merged or silently shadowed. The
//! catalog is the lookup surface the governance mapping and the risk mapping
//! use at decision time.

use std::collections::BTreeMap;
use std::sync::Mutex;

use apeireth_core::kernel::CapabilityId;
use thiserror::Error;

use super::connection::DiscoveredTool;

/// One external tool as it is exposed to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolEntry {
    /// Configured server name that provides the tool.
    pub server: String,
    /// Server-side tool name (what `tools/call` sends).
    pub remote_name: String,
    /// Model-facing name in the `mcp:<server>:<tool>` namespace.
    pub model_name: String,
    /// Stable capability identity (`tool.mcp.<server>.<tool>`).
    pub capability_id: CapabilityId,
    /// The server's description, when given.
    pub description: Option<String>,
    /// The declared input schema, when given.
    pub input_schema: Option<serde_json::Value>,
    /// The server's permission declaration: explicitly read-only or not.
    pub declared_read_only: bool,
}

impl McpToolEntry {
    /// Build the entry for one discovered tool of one server.
    pub fn new(server: &str, tool: &DiscoveredTool) -> Result<Self, McpCatalogError> {
        let model_name = model_name(server, &tool.name);
        let capability_id = capability_id_for(server, &tool.name)?;
        Ok(Self {
            server: server.to_string(),
            remote_name: tool.name.clone(),
            model_name,
            capability_id,
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
            declared_read_only: tool.declared_read_only,
        })
    }
}

/// Model-facing name in the `mcp:<server>:<tool>` namespace.
pub fn model_name(server: &str, tool: &str) -> String {
    format!("mcp:{server}:{tool}")
}

/// Stable capability identity for one external tool.
pub fn capability_id_for(server: &str, tool: &str) -> Result<CapabilityId, McpCatalogError> {
    let raw = format!("tool.mcp.{server}.{tool}");
    CapabilityId::new(raw.clone()).map_err(|e| McpCatalogError::InvalidIdentity {
        model_name: model_name(server, tool),
        reason: e.to_string(),
    })
}

/// The dynamic tool directory.
#[derive(Default)]
pub struct McpToolCatalog {
    entries: Mutex<BTreeMap<String, McpToolEntry>>,
}

impl McpToolCatalog {
    /// An empty directory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one entry. A duplicate model-facing name or capability id is
    /// refused — the directory never holds two tools under one identity.
    pub fn insert(&self, entry: McpToolEntry) -> Result<(), McpCatalogError> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = entries.get(&entry.model_name) {
            return Err(McpCatalogError::DuplicateToolName {
                model_name: entry.model_name.clone(),
                registered_by: existing.server.clone(),
            });
        }
        if let Some(existing) = entries
            .values()
            .find(|e| e.capability_id == entry.capability_id)
        {
            return Err(McpCatalogError::DuplicateToolName {
                model_name: entry.model_name.clone(),
                registered_by: existing.server.clone(),
            });
        }
        entries.insert(entry.model_name.clone(), entry);
        Ok(())
    }

    /// Replace one server's slice of the directory.
    ///
    /// The fresh list is validated before anything changes: a name repeated
    /// inside one list, or one that would shadow another server's identity,
    /// fails the refresh without touching the directory. Then: entries the
    /// server no longer exposes are removed and returned, entries with a
    /// changed descriptor are replaced (reported as removed + added so the
    /// live registry re-registers them), identical entries stay untouched,
    /// and new entries are inserted. Returns `(added, removed)`.
    pub fn replace_server(
        &self,
        server: &str,
        fresh: Vec<McpToolEntry>,
    ) -> Result<(Vec<McpToolEntry>, Vec<McpToolEntry>), McpCatalogError> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let mut seen = std::collections::BTreeSet::new();
        for entry in &fresh {
            if !seen.insert(entry.model_name.as_str()) {
                return Err(McpCatalogError::DuplicateToolName {
                    model_name: entry.model_name.clone(),
                    registered_by: server.to_string(),
                });
            }
            if let Some(existing) = entries.get(&entry.model_name) {
                if existing.server != server {
                    return Err(McpCatalogError::DuplicateToolName {
                        model_name: entry.model_name.clone(),
                        registered_by: existing.server.clone(),
                    });
                }
            }
        }

        let incoming: std::collections::BTreeSet<&str> =
            fresh.iter().map(|e| e.model_name.as_str()).collect();
        let mut removed: Vec<McpToolEntry> = entries
            .values()
            .filter(|e| e.server == server && !incoming.contains(e.model_name.as_str()))
            .cloned()
            .collect();
        for entry in &removed {
            entries.remove(&entry.model_name);
        }

        let mut added = Vec::new();
        for entry in fresh {
            match entries.get(&entry.model_name) {
                Some(existing) if *existing == entry => {}
                Some(_) => {
                    let old = entries.remove(&entry.model_name).expect("checked present");
                    removed.push(old);
                    entries.insert(entry.model_name.clone(), entry.clone());
                    added.push(entry);
                }
                None => {
                    entries.insert(entry.model_name.clone(), entry.clone());
                    added.push(entry);
                }
            }
        }
        Ok((added, removed))
    }

    /// Remove one entry by model-facing name.
    pub fn remove(&self, model_name: &str) -> Option<McpToolEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(model_name)
    }

    /// Look one entry up by model-facing name.
    pub fn get(&self, model_name: &str) -> Option<McpToolEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(model_name)
            .cloned()
    }

    /// Look one entry up by capability identity.
    pub fn get_by_capability(&self, capability_id: &CapabilityId) -> Option<McpToolEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .find(|entry| &entry.capability_id == capability_id)
            .cloned()
    }

    /// Whether the server declared this capability explicitly read-only.
    pub fn is_declared_read_only(&self, capability_id: &CapabilityId) -> bool {
        self.get_by_capability(capability_id)
            .map(|entry| entry.declared_read_only)
            .unwrap_or(false)
    }

    /// Every entry, in model-name order.
    pub fn list(&self) -> Vec<McpToolEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Number of live entries.
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// Whether the directory is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A catalog defect: duplicate or illegal tool identity.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum McpCatalogError {
    /// Two tools want one model-facing name or capability id.
    #[error("duplicate tool name {model_name:?}: already registered by server {registered_by:?}")]
    DuplicateToolName {
        /// The contested model-facing name.
        model_name: String,
        /// The server whose entry already holds it.
        registered_by: String,
    },
    /// The composed identity violates the stable-id grammar.
    #[error("tool identity for {model_name:?} is invalid: {reason}")]
    InvalidIdentity {
        /// The model-facing name.
        model_name: String,
        /// What the id validator refused.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str, read_only: bool) -> DiscoveredTool {
        DiscoveredTool {
            name: name.to_string(),
            description: None,
            input_schema: None,
            declared_read_only: read_only,
        }
    }

    #[test]
    fn entries_carry_namespaced_names_and_stable_ids() {
        let entry = McpToolEntry::new("demo", &tool("lookup-item", true)).unwrap();
        assert_eq!(entry.model_name, "mcp:demo:lookup-item");
        assert_eq!(entry.capability_id.as_str(), "tool.mcp.demo.lookup-item");
        assert!(entry.declared_read_only);
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let catalog = McpToolCatalog::new();
        let entry = McpToolEntry::new("demo", &tool("lookup", false)).unwrap();
        catalog.insert(entry.clone()).unwrap();
        let err = catalog.insert(entry).unwrap_err();
        assert!(err.to_string().contains("duplicate tool name"), "{err}");
    }

    #[test]
    fn replace_server_removes_and_adds() {
        let catalog = McpToolCatalog::new();
        let a = McpToolEntry::new("demo", &tool("one", false)).unwrap();
        let b = McpToolEntry::new("demo", &tool("two", false)).unwrap();
        catalog.insert(a).unwrap();
        catalog.insert(b).unwrap();

        let c = McpToolEntry::new("demo", &tool("three", false)).unwrap();
        let (added, removed) = catalog
            .replace_server("demo", vec![catalog.get("mcp:demo:two").unwrap(), c])
            .unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].model_name, "mcp:demo:three");
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].model_name, "mcp:demo:one");
        assert_eq!(catalog.len(), 2);
    }
}

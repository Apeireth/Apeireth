//! The builtin utility plugin that owns the M2A tools.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, PluginId};
use apeireth_plugin::{
    CapabilityDescriptor, CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult,
    ToolCapability,
};
use async_trait::async_trait;

use crate::filesystem::FilesystemTool;
use crate::repo::RepoTool;
use crate::search::SearchTool;

/// One builtin plugin providing `tool.filesystem`, `tool.search`, and
/// `tool.repo`.
///
/// The plugin owns the capabilities; the runtime owns dispatch and governance.
pub struct BuiltinToolsPlugin {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

impl BuiltinToolsPlugin {
    /// Build the plugin rooted at `workspace_root`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let manifest = PluginManifest::new(
            PluginId::new("builtin.tools").unwrap(),
            "1.0.0",
            "Canonical builtin tools: filesystem, search, repository inspection",
        )
        .declare(
            CapabilityDescriptor::new(
                CapabilityId::new("tool.filesystem").unwrap(),
                CapabilityKind::Tool,
                "Read, list, and stat files inside the workspace root",
            )
            .unwrap()
            .with_metadata("risk", "medium")
            .with_metadata("m2a", "read-only-filesystem"),
        )
        .unwrap()
        .declare(
            CapabilityDescriptor::new(
                CapabilityId::new("tool.search").unwrap(),
                CapabilityKind::Tool,
                "Deterministic local file-name and file-content search",
            )
            .unwrap()
            .with_metadata("risk", "low")
            .with_metadata("m2a", "local-search"),
        )
        .unwrap()
        .declare(
            CapabilityDescriptor::new(
                CapabilityId::new("tool.repo").unwrap(),
                CapabilityKind::Tool,
                "Read-only git repository inspection",
            )
            .unwrap()
            .with_metadata("risk", "low")
            .with_metadata("m2a", "read-only-repo"),
        )
        .unwrap();

        let tools: Vec<Arc<dyn ToolCapability>> = vec![
            Arc::new(FilesystemTool::new(workspace_root.clone())),
            Arc::new(SearchTool::new(workspace_root.clone())),
            Arc::new(RepoTool::new(workspace_root)),
        ];

        Self { manifest, tools }
    }
}

#[async_trait]
impl Plugin for BuiltinToolsPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declares_exactly_three_tool_capabilities() {
        let plugin = BuiltinToolsPlugin::new(".");
        assert_eq!(plugin.manifest.capabilities.len(), 3);
        assert_eq!(plugin.tools().len(), 3);

        let ids: Vec<String> = plugin
            .manifest
            .capabilities
            .iter()
            .map(|c| c.id.to_string())
            .collect();
        assert_eq!(ids, ["tool.filesystem", "tool.search", "tool.repo"]);
    }

    #[test]
    fn declarations_match_implemented_ids() {
        let plugin = BuiltinToolsPlugin::new(".");
        let mut tool_ids: Vec<String> = plugin.tools().iter().map(|t| t.id().to_string()).collect();
        tool_ids.sort();
        assert_eq!(tool_ids, ["tool.filesystem", "tool.repo", "tool.search"]);
    }
}

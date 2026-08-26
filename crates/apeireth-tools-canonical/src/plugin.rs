//! The builtin utility plugin that owns the M2A tools and the opt-in M2C-T
//! Trusted Shell.

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
use crate::shell::{ShellTool, TrustedShellConfig};

/// Options for the builtin tools plugin.
#[derive(Debug, Clone, Default)]
pub struct BuiltinToolsOptions {
    /// When set, `tool.shell` is registered as a Trusted Shell.
    ///
    /// Default is `None`: Shell is disabled by default and must be explicitly
    /// enabled by application configuration.
    pub shell: Option<TrustedShellConfig>,
}

/// One builtin plugin providing `tool.filesystem`, `tool.search`, and
/// `tool.repo`. When explicitly configured it also provides `tool.shell`.
///
/// The plugin owns the capabilities; the runtime owns dispatch and governance.
pub struct BuiltinToolsPlugin {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

impl BuiltinToolsPlugin {
    /// Build the plugin rooted at `workspace_root` without Shell.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self::with_options(workspace_root, BuiltinToolsOptions::default())
    }

    /// Build the plugin with explicit options.
    pub fn with_options(workspace_root: impl Into<PathBuf>, options: BuiltinToolsOptions) -> Self {
        let workspace_root = workspace_root.into();
        let mut manifest = PluginManifest::new(
            PluginId::new("builtin.tools").unwrap(),
            "1.1.0",
            "Canonical builtin tools: filesystem, search, repository inspection, and opt-in shell",
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

        let mut tools: Vec<Arc<dyn ToolCapability>> = vec![
            Arc::new(FilesystemTool::new(workspace_root.clone())),
            Arc::new(SearchTool::new(workspace_root.clone())),
            Arc::new(RepoTool::new(workspace_root.clone())),
        ];

        if let Some(shell_config) = options.shell {
            manifest = manifest
                .declare(
                    CapabilityDescriptor::new(
                        CapabilityId::new("tool.shell").unwrap(),
                        CapabilityKind::Tool,
                        "Executes a platform-native local shell command after explicit user approval",
                    )
                    .unwrap()
                    .with_metadata("risk", "high")
                    .with_metadata("m2c", "trusted-shell"),
                )
                .unwrap();
            tools.push(Arc::new(ShellTool::new(shell_config)));
        }

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
    fn manifest_declares_exactly_three_tool_capabilities_by_default() {
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
    fn declarations_match_implemented_ids_by_default() {
        let plugin = BuiltinToolsPlugin::new(".");
        let mut tool_ids: Vec<String> = plugin.tools().iter().map(|t| t.id().to_string()).collect();
        tool_ids.sort();
        assert_eq!(tool_ids, ["tool.filesystem", "tool.repo", "tool.search"]);
    }

    #[test]
    fn shell_is_registered_only_when_explicitly_enabled() {
        let plugin = BuiltinToolsPlugin::with_options(
            ".",
            BuiltinToolsOptions {
                shell: Some(TrustedShellConfig::new(".")),
            },
        );
        assert_eq!(plugin.manifest.capabilities.len(), 4);
        assert_eq!(plugin.tools().len(), 4);
        assert!(plugin
            .manifest
            .capabilities
            .iter()
            .any(|c| c.id.as_str() == "tool.shell"));
        assert!(plugin
            .tools()
            .iter()
            .any(|t| t.id().as_str() == "tool.shell"));
    }
}

//! The builtin utility module that owns the M2A tools and the opt-in M2C-T
//! Trusted Shell.
//!
//! One owner, many capabilities. The runtime extension entity is a *module*:
//! `filesystem`, `search`, `repo` and the opt-in `shell`/`fetch`/`education`
//! tools are capabilities of this one module, not five modules that each happen
//! to carry a single tool. They share its manifest, its lifecycle, and its
//! entry in the one capability index.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, PluginId};
use apeireth_plugin::{
    CapabilityDescriptor, CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult,
    ToolCapability,
};
use async_trait::async_trait;

use crate::education::EducationTool;
use crate::fetch::{FetchConfig, FetchTool};
use crate::filesystem::FilesystemTool;
use crate::repo::RepoTool;
use crate::search::SearchTool;
use crate::shell::{ShellTool, TrustedShellConfig};

/// Options for the builtin tools module.
#[derive(Debug, Clone, Default)]
pub struct BuiltinToolsOptions {
    /// When set, `tool.shell` is registered as a Trusted Shell.
    ///
    /// Default is `None`: Shell is disabled by default and must be explicitly
    /// enabled by application configuration.
    pub shell: Option<TrustedShellConfig>,

    /// When set, `tool.fetch` is registered as a controlled HTTP(S) fetch
    /// tool. Default is `None`: Fetch is disabled by default and must be
    /// explicitly enabled by application configuration.
    pub fetch: Option<FetchConfig>,

    /// Whether the deterministic education Dx-Check tool (`tool.education`) is
    /// registered. Default is `false`: it is a domain tool, not part of the
    /// workspace-inspection baseline.
    pub education: bool,
}

/// One builtin module providing `tool.filesystem`, `tool.search`, and
/// `tool.repo`. When explicitly configured it also provides `tool.shell`,
/// `tool.fetch`, and `tool.education`.
///
/// The module owns the capabilities; the runtime owns dispatch and governance.
pub struct BuiltinToolsModule {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

impl BuiltinToolsModule {
    /// Build the module rooted at `workspace_root` without Shell.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self::with_options(workspace_root, BuiltinToolsOptions::default())
    }

    /// Build the module with explicit options.
    pub fn with_options(workspace_root: impl Into<PathBuf>, options: BuiltinToolsOptions) -> Self {
        let workspace_root = workspace_root.into();
        let mut manifest = PluginManifest::new(
            PluginId::new("builtin.tools").unwrap(),
            "1.1.0",
            "Canonical builtin tools: filesystem, search, repository inspection, opt-in shell, and opt-in fetch",
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

        if let Some(fetch_config) = options.fetch {
            manifest = manifest
                .declare(
                    CapabilityDescriptor::new(
                        CapabilityId::new("tool.fetch").unwrap(),
                        CapabilityKind::Tool,
                        "Fetches one bounded HTTP(S) text resource through controlled egress",
                    )
                    .unwrap()
                    .with_metadata("risk", "medium")
                    .with_metadata("m3a", "controlled-fetch"),
                )
                .unwrap();
            tools.push(Arc::new(FetchTool::new(fetch_config)));
        }

        // W2 §4.3: education Dx-Check. Pure deterministic text rules, no LLM and
        // no side effects, so it travels with the other builtin tools behind its
        // own switch rather than as a module of its own.
        if options.education {
            manifest = manifest
                .declare(
                    CapabilityDescriptor::new(
                        CapabilityId::new("tool.education").unwrap(),
                        CapabilityKind::Tool,
                        "Dx-Check substitution verification for symbolic calculus (deterministic)",
                    )
                    .unwrap()
                    .with_metadata("risk", "low")
                    .with_metadata("w2", "education-dx-check"),
                )
                .unwrap();
            tools.push(Arc::new(EducationTool::new()));
        }

        Self { manifest, tools }
    }
}

#[async_trait]
impl Plugin for BuiltinToolsModule {
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
        let plugin = BuiltinToolsModule::new(".");
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
        let plugin = BuiltinToolsModule::new(".");
        let mut tool_ids: Vec<String> = plugin.tools().iter().map(|t| t.id().to_string()).collect();
        tool_ids.sort();
        assert_eq!(tool_ids, ["tool.filesystem", "tool.repo", "tool.search"]);
    }

    #[test]
    fn shell_is_registered_only_when_explicitly_enabled() {
        let plugin = BuiltinToolsModule::with_options(
            ".",
            BuiltinToolsOptions {
                shell: Some(TrustedShellConfig::new(".")),
                fetch: None,
                education: false,
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
    #[test]
    fn fetch_is_registered_only_when_explicitly_enabled() {
        let plugin = BuiltinToolsModule::with_options(
            ".",
            BuiltinToolsOptions {
                shell: None,
                fetch: Some(FetchConfig::deny_all()),
                education: false,
            },
        );
        assert_eq!(plugin.manifest.capabilities.len(), 4);
        assert_eq!(plugin.tools().len(), 4);
        assert!(plugin
            .manifest
            .capabilities
            .iter()
            .any(|c| c.id.as_str() == "tool.fetch"));
        assert!(plugin
            .tools()
            .iter()
            .any(|t| t.id().as_str() == "tool.fetch"));
    }

    #[test]
    fn education_registers_behind_its_own_switch() {
        let plugin = BuiltinToolsModule::with_options(
            ".",
            BuiltinToolsOptions {
                shell: None,
                fetch: None,
                education: true,
            },
        );
        assert_eq!(plugin.manifest.capabilities.len(), 4);
        assert_eq!(plugin.tools().len(), 4);
        assert!(plugin
            .manifest
            .capabilities
            .iter()
            .any(|c| c.id.as_str() == "tool.education"));

        let off = BuiltinToolsModule::new(".");
        assert!(!off
            .manifest
            .capabilities
            .iter()
            .any(|c| c.id.as_str() == "tool.education"));
    }

    #[test]
    fn fetch_and_shell_can_be_enabled_together_without_duplicates() {
        let plugin = BuiltinToolsModule::with_options(
            ".",
            BuiltinToolsOptions {
                shell: Some(TrustedShellConfig::new(".")),
                fetch: Some(FetchConfig::deny_all()),
                education: false,
            },
        );
        assert_eq!(plugin.manifest.capabilities.len(), 5);
        assert_eq!(plugin.tools().len(), 5);
    }
}

/// Compatibility alias for [`BuiltinToolsModule`].
///
/// The runtime entity is a module; "plugin" referred to the same object under
/// its historical name and reaches the same registry.
pub use self::BuiltinToolsModule as BuiltinToolsPlugin;

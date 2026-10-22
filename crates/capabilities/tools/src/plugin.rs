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

use crate::exec_pipeline::{PipelinedCapability, ToolExecutionPipeline};
use crate::external::{external_tool_set, ExternalToolsConfig};
use crate::fetch::{FetchConfig, FetchTool};
use crate::filesystem::FilesystemTool;
use crate::recon::Scope;
use crate::recon_tool::recon_tool_set;
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

    /// When set, `tool.fetch` is registered as a controlled HTTP(S) fetch
    /// tool. Default is `None`: Fetch is disabled by default and must be
    /// explicitly enabled by application configuration.
    pub fetch: Option<FetchConfig>,

    /// When set, the eight `tool.recon.*` tools are registered. Default is
    /// `None`: recon is disabled by default and must be explicitly enabled,
    /// with an operator-provided authorization scope.
    pub recon: Option<ReconConfig>,

    /// When set, the three read-only external adapters (`ext.nmap`,
    /// `ext.exiftool`, `ext.tshark`) are registered. Default is `None`.
    pub external: Option<ExternalToolsConfig>,
}

/// Operator configuration for the recon tool family.
#[derive(Debug, Clone)]
pub struct ReconConfig {
    /// Authorization scope for network/web tools; an empty scope denies all.
    pub scope: Arc<Scope>,
}

/// One builtin plugin providing `tool.filesystem`, `tool.search`, and
/// `tool.repo`. When explicitly configured it also provides `tool.shell`.
///
/// The plugin owns the capabilities; the runtime owns dispatch and governance.
/// Every capability executes through the explicit five-stage chain
/// (`exec_pipeline`): the default pipeline installs no hooks, no guards, no
/// deadline, and no correction channel, so dispatch behaviour is exactly what
/// direct execution produced before the chain was made explicit.
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

        if let Some(recon) = options.recon {
            let descriptors = [
                (
                    "tool.recon.pack",
                    "Static strings/entropy/magic carving",
                    "low",
                ),
                ("tool.recon.binfmt", "ELF/PE/Mach-O static structure", "low"),
                ("tool.recon.dotnet", "CLR metadata detection", "low"),
                (
                    "tool.recon.js",
                    "Lexical JavaScript/Electron inspection",
                    "low",
                ),
                ("tool.recon.pcap", "Classic pcap flow/DNS parsing", "low"),
                ("tool.recon.yara", "YARA-subset rule scanning", "low"),
                (
                    "tool.recon.net",
                    "TCP connect scan within the authorized scope",
                    "medium",
                ),
                (
                    "tool.recon.web",
                    "Single HTTP probe within the authorized scope",
                    "medium",
                ),
                (
                    "tool.recon.tls",
                    "Active TLS handshake fingerprint",
                    "medium",
                ),
            ];
            for (id, description, risk) in descriptors {
                manifest = manifest
                    .declare(
                        CapabilityDescriptor::new(
                            CapabilityId::new(id).unwrap(),
                            CapabilityKind::Tool,
                            description,
                        )
                        .unwrap()
                        .with_metadata("risk", risk)
                        .with_metadata("recon", "read-only-static"),
                    )
                    .unwrap();
            }
            tools.extend(recon_tool_set(workspace_root.clone(), recon.scope));
        }

        if let Some(external) = options.external {
            let descriptors = [
                (
                    "tool.external.nmap",
                    "nmap connect scan and service detection",
                    "medium",
                ),
                ("tool.external.exiftool", "exiftool metadata reader", "low"),
                (
                    "tool.external.tshark",
                    "tshark pcap protocol summary",
                    "low",
                ),
                (
                    "tool.external.yara",
                    "Full YARA via the system yara binary",
                    "low",
                ),
                (
                    "tool.external.binwalk",
                    "binwalk embedded-signature scan",
                    "low",
                ),
                ("tool.external.radare2", "radare2 function listing", "low"),
                (
                    "tool.external.jadx",
                    "jadx APK/DEX decompilation summary",
                    "low",
                ),
                ("tool.external.apktool", "apktool APK decode summary", "low"),
                (
                    "tool.external.ghidra",
                    "Ghidra headless function listing",
                    "medium",
                ),
                (
                    "tool.external.openssl",
                    "openssl TLS certificate chain",
                    "medium",
                ),
            ];
            for (id, description, risk) in descriptors {
                manifest = manifest
                    .declare(
                        CapabilityDescriptor::new(
                            CapabilityId::new(id).unwrap(),
                            CapabilityKind::Tool,
                            description,
                        )
                        .unwrap()
                        .with_metadata("risk", risk)
                        .with_metadata("external", "subprocess-read-only"),
                    )
                    .unwrap();
            }
            tools.extend(external_tool_set(&external));
        }

        // 执行链重排为显式阶段: 每个能力的调用都经五段流水线调度。默认流水线
        // 不挂任何钩子/守卫/超时/纠错/合同, 发出的结果与直接执行逐字节一致。
        let pipeline = Arc::new(ToolExecutionPipeline::new());
        let tools: Vec<Arc<dyn ToolCapability>> = tools
            .into_iter()
            .map(|tool| -> Arc<dyn ToolCapability> {
                Arc::new(PipelinedCapability::new(tool, Arc::clone(&pipeline)))
            })
            .collect();

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
                recon: None,
                external: None,
                shell: Some(TrustedShellConfig::new(".")),
                fetch: None,
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
        let plugin = BuiltinToolsPlugin::with_options(
            ".",
            BuiltinToolsOptions {
                recon: None,
                external: None,
                shell: None,
                fetch: Some(FetchConfig::deny_all()),
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
    fn fetch_and_shell_can_be_enabled_together_without_duplicates() {
        let plugin = BuiltinToolsPlugin::with_options(
            ".",
            BuiltinToolsOptions {
                recon: None,
                external: None,
                shell: Some(TrustedShellConfig::new(".")),
                fetch: Some(FetchConfig::deny_all()),
            },
        );
        assert_eq!(plugin.manifest.capabilities.len(), 5);
        assert_eq!(plugin.tools().len(), 5);
    }

    #[test]
    fn recon_is_registered_only_when_explicitly_enabled() {
        let scope = Arc::new(Scope::empty("test"));
        let plugin = BuiltinToolsPlugin::with_options(
            ".",
            BuiltinToolsOptions {
                recon: Some(ReconConfig { scope }),
                ..Default::default()
            },
        );
        assert_eq!(plugin.manifest.capabilities.len(), 12);
        assert_eq!(plugin.tools().len(), 12);
        let ids: Vec<String> = plugin
            .manifest
            .capabilities
            .iter()
            .map(|capability| capability.id.to_string())
            .collect();
        assert!(ids.iter().any(|id| id == "tool.recon.net"));
        assert!(ids.iter().any(|id| id == "tool.recon.web"));
        assert!(ids.iter().any(|id| id == "tool.recon.yara"));

        // 默认仍保持 fail-closed。
        let default_plugin = BuiltinToolsPlugin::new(".");
        assert_eq!(default_plugin.tools().len(), 3);
    }

    #[test]
    fn external_adapters_are_registered_only_when_enabled() {
        let plugin = BuiltinToolsPlugin::with_options(
            ".",
            BuiltinToolsOptions {
                external: Some(ExternalToolsConfig {
                    workspace_root: PathBuf::from("."),
                    scope: Arc::new(Scope::empty("test")),
                }),
                ..Default::default()
            },
        );
        assert_eq!(plugin.manifest.capabilities.len(), 13);
        assert_eq!(plugin.tools().len(), 13);
        let ids: Vec<String> = plugin
            .manifest
            .capabilities
            .iter()
            .map(|capability| capability.id.to_string())
            .collect();
        assert!(ids.iter().any(|id| id == "tool.external.nmap"));
        assert!(ids.iter().any(|id| id == "tool.external.tshark"));
        assert!(ids.iter().any(|id| id == "tool.external.exiftool"));
        assert!(ids.iter().any(|id| id == "tool.external.yara"));
        assert!(ids.iter().any(|id| id == "tool.external.binwalk"));
        assert!(ids.iter().any(|id| id == "tool.external.radare2"));
        assert!(ids.iter().any(|id| id == "tool.external.jadx"));
        assert!(ids.iter().any(|id| id == "tool.external.apktool"));
        assert!(ids.iter().any(|id| id == "tool.external.ghidra"));
        assert!(ids.iter().any(|id| id == "tool.external.openssl"));
    }
}

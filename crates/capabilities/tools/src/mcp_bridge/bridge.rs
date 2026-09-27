//! The bridge: configuration in, dynamic tools out.
//!
//! One [`McpToolBridge`] owns one connection per enabled server, the dynamic
//! tool catalog, and the shared five-stage pipeline every external call
//! walks. [`McpToolBridge::refresh`] is the dynamic catalog entry: it runs
//! `tools/list` on every live link (re-opening a link that dropped, which
//! re-runs the handshake and re-discovers), reconciles the catalog, and
//! reports exactly what must be registered and unregistered. Duplicate
//! identities are refused at every level — a server that lists one name twice
//! fails the refresh.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::PermissionPolicy;
use apeireth_plugin::ToolCapability;
use thiserror::Error;

use crate::exec_pipeline::{AroundPolicy, PipelinedCapability, ToolExecutionPipeline};
use crate::mcp::McpError;

use super::catalog::{McpCatalogError, McpToolCatalog, McpToolEntry};
use super::config::{McpBridgeOptions, McpConfigError, McpServerConfig};
use super::connection::McpServerConnection;
use super::governance::{McpPermissionMapping, McpRiskMappingHook, McpToolDenyGuard};
use super::tool::{mcp_output_schema, McpDynamicTool};

/// Where discovered dynamic tools land (a module bag, a runtime registry).
pub trait McpToolRegistry: Send + Sync {
    /// Register one dynamic tool. Duplicate identities are refused.
    fn register_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String>;

    /// Unregister one dynamic tool by capability identity.
    fn unregister_tool(&self, capability_id: &CapabilityId);
}

/// What one catalog refresh changed.
pub struct DiscoveryReport {
    /// Newly discovered tools, already wrapped in the shared pipeline.
    pub registered: Vec<Arc<dyn ToolCapability>>,
    /// Entries that disappeared and must be unregistered.
    pub removed: Vec<McpToolEntry>,
    /// The full catalog after the refresh.
    pub live: Vec<McpToolEntry>,
}

impl DiscoveryReport {
    /// Model-facing names of the newly registered tools.
    pub fn registered_names(&self) -> Vec<String> {
        self.registered
            .iter()
            .map(|tool| tool.declaration().name)
            .collect()
    }

    /// Push this refresh into a registry: unregister the gone, register the
    /// new. Fail-closed: a refused registration is an error, never a skip.
    pub fn apply_to(&self, registry: &dyn McpToolRegistry) -> Result<(), McpBridgeError> {
        for entry in &self.removed {
            registry.unregister_tool(&entry.capability_id);
        }
        for tool in &self.registered {
            registry
                .register_tool(Arc::clone(tool))
                .map_err(McpBridgeError::Registration)?;
        }
        Ok(())
    }
}

/// The MCP tool bridge.
pub struct McpToolBridge {
    config: McpServerConfig,
    options: McpBridgeOptions,
    connections: Vec<Arc<McpServerConnection>>,
    catalog: Arc<McpToolCatalog>,
    policy: Arc<Mutex<PermissionPolicy>>,
    deny_guard: Arc<McpToolDenyGuard>,
    pipeline: Arc<ToolExecutionPipeline>,
    tools: Mutex<BTreeMap<String, Arc<dyn ToolCapability>>>,
}

impl McpToolBridge {
    /// Build the bridge over `config`, opening channels matching each spec.
    pub fn new(config: McpServerConfig, options: McpBridgeOptions) -> Result<Self, McpBridgeError> {
        config.validate().map_err(McpBridgeError::Config)?;
        let connections = config
            .enabled_servers()
            .map(|spec| {
                Arc::new(McpServerConnection::from_spec(spec.clone(), &options))
                    as Arc<McpServerConnection>
            })
            .collect();
        Self::from_connections(config, options, connections)
    }

    /// Build the bridge over pre-built connections (one per enabled server,
    /// in configuration order).
    pub fn from_connections(
        config: McpServerConfig,
        options: McpBridgeOptions,
        connections: Vec<Arc<McpServerConnection>>,
    ) -> Result<Self, McpBridgeError> {
        config.validate().map_err(McpBridgeError::Config)?;
        let enabled = config.enabled_servers().count();
        if connections.len() != enabled {
            return Err(McpBridgeError::Config(McpConfigError::InvalidSpec {
                name: "<bridge>".to_string(),
                reason: format!(
                    "expected {enabled} connection(s) for enabled servers, got {}",
                    connections.len()
                ),
            }));
        }
        let catalog = Arc::new(McpToolCatalog::new());
        let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
        let deny_guard = Arc::new(McpToolDenyGuard::new());
        let pipeline = build_pipeline(&options, &catalog, &policy, &deny_guard);
        Ok(Self {
            config,
            options,
            connections,
            catalog,
            policy,
            deny_guard,
            pipeline,
            tools: Mutex::new(BTreeMap::new()),
        })
    }

    /// The loaded configuration.
    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }

    /// The options in force.
    pub fn options(&self) -> &McpBridgeOptions {
        &self.options
    }

    /// The live dynamic tool catalog.
    pub fn catalog(&self) -> &Arc<McpToolCatalog> {
        &self.catalog
    }

    /// The shared permission policy the risk mapping consults.
    pub fn policy(&self) -> &Arc<Mutex<PermissionPolicy>> {
        &self.policy
    }

    /// The deny-only blocklist consulted by the guard stage.
    pub fn deny_guard(&self) -> &Arc<McpToolDenyGuard> {
        &self.deny_guard
    }

    /// The pipeline every external call walks.
    pub fn pipeline(&self) -> &Arc<ToolExecutionPipeline> {
        &self.pipeline
    }

    /// One connection per enabled server, in configuration order.
    pub fn connections(&self) -> &[Arc<McpServerConnection>] {
        &self.connections
    }

    /// The redacted startup log: URLs may be recorded, token / secret
    /// parameters never are.
    pub fn startup_log(&self) -> Vec<String> {
        self.config.redacted_startup_log()
    }

    /// Refresh the whole dynamic catalog: (re)connect where needed, run
    /// `tools/list` everywhere, reconcile, and report the delta.
    pub async fn refresh(&self) -> Result<DiscoveryReport, McpBridgeError> {
        let mut registered = Vec::new();
        let mut removed = Vec::new();
        for connection in &self.connections {
            let server = connection.server_name().to_string();
            let discovered = match connection.discover().await {
                Ok(tools) => tools,
                // No live link (first connect or a dropped one): open one,
                // which also re-discovers the tool list.
                Err(McpError::Transport(_)) => {
                    connection
                        .connect()
                        .await
                        .map_err(|error| McpBridgeError::Connect {
                            server: server.clone(),
                            error,
                        })?
                }
                Err(error) => {
                    return Err(McpBridgeError::Connect {
                        server: server.clone(),
                        error,
                    })
                }
            };
            let mut entries = Vec::with_capacity(discovered.len());
            for tool in &discovered {
                entries.push(McpToolEntry::new(&server, tool)?);
            }
            let (added, gone) = self.catalog.replace_server(&server, entries)?;
            for entry in &gone {
                self.tools
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&entry.model_name);
            }
            removed.extend(gone);
            for entry in added {
                let tool = self.wrap_tool(entry.clone());
                self.tools
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .insert(entry.model_name.clone(), tool.clone());
                registered.push(tool);
            }
        }

        // External tool permission declarations become explicit grants only
        // where the read-only preset releases them; everything else stays at
        // the require-approval level until an operator authorizes it.
        if self.options.readonly_preset {
            let mut policy = self
                .policy
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            McpPermissionMapping::authorize_readonly_declarations(&mut policy, &self.catalog);
        }

        Ok(DiscoveryReport {
            registered,
            removed,
            live: self.catalog.list(),
        })
    }

    /// The currently live pipeline-wrapped tools.
    pub fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .cloned()
            .collect()
    }

    fn wrap_tool(&self, entry: McpToolEntry) -> Arc<dyn ToolCapability> {
        let connection = self
            .connections
            .iter()
            .find(|connection| connection.server_name() == entry.server)
            .cloned()
            .expect("catalog entries always name a configured connection");
        let dynamic = Arc::new(McpDynamicTool::new(entry, connection));
        Arc::new(PipelinedCapability::new(
            dynamic,
            Arc::clone(&self.pipeline),
        ))
    }
}

fn build_pipeline(
    options: &McpBridgeOptions,
    catalog: &Arc<McpToolCatalog>,
    policy: &Arc<Mutex<PermissionPolicy>>,
    deny_guard: &Arc<McpToolDenyGuard>,
) -> Arc<ToolExecutionPipeline> {
    let risk_mapping = Arc::new(McpRiskMappingHook::new(
        Arc::clone(policy),
        Arc::clone(catalog),
        options.readonly_preset,
    ));
    let around = AroundPolicy::new().with_timeout(options.call_timeout_ms, options.max_timeout_ms);
    let deny_guard: Arc<McpToolDenyGuard> = Arc::clone(deny_guard);
    let deny_guard: Arc<dyn apeireth_governance::ToolGuard> = deny_guard;
    Arc::new(
        ToolExecutionPipeline::new()
            .with_pre_hook(risk_mapping)
            .with_guard(deny_guard)
            .with_around(around)
            .with_output_schema(mcp_output_schema()),
    )
}

/// What the bridge refuses to do.
#[derive(Debug, Error)]
pub enum McpBridgeError {
    /// The configuration is defective.
    #[error(transparent)]
    Config(#[from] McpConfigError),
    /// The dynamic catalog refused an identity.
    #[error(transparent)]
    Catalog(#[from] McpCatalogError),
    /// One server could not be reached or spoken to.
    #[error("mcp server {server:?} is unreachable: {error}")]
    Connect {
        /// The server that failed.
        server: String,
        /// The transport / protocol failure.
        error: McpError,
    },
    /// A registry refused one dynamic tool.
    #[error("dynamic tool registration refused: {0}")]
    Registration(String),
}

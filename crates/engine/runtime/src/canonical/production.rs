//! The single production module composition root.
//!
//! Adapters supply concrete backends, tools, and the optional Council. This module
//! owns ordering and slot validation, so CLI, gateway, and future
//! embedding callers cannot accidentally register a second cognitive or tool spine.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_orchestration::Council;
use apeireth_plugin::experience::{AssociationStore, KnowledgeGraphStore, WikiEntryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::preference::PreferenceStore;
use apeireth_plugin::self_assessment::SelfAssessmentStore;
use apeireth_tools_canonical::{FetchConfig, TrustedShellConfig};

use super::cognitive::{
    CognitiveTelemetry, CouncilModule, JudgeConfig, JudgeModule, JudgeObservations,
    MemoryRecallModule, MemoryWritebackModule, PreferenceRecallModule, SelfAssessmentModule,
};
use super::error::{RuntimeError, RuntimeResult};
use super::module::{Module, ModuleManifest};
use super::tool_modules::{
    FetchModule, FilesystemModule, McpModule, RepoModule, SearchModule, ShellModule,
};

/// Feature switches for the production cognitive and tool modules.
///
/// Memory and preference recall/writeback are cheap local calls and are on by
/// default when their injected stores exist. Judge, Council, Shell and Fetch are explicitly
/// opt-in; Judge and Council side-calls stay behind the runtime invoker.
#[derive(Debug, Clone)]
pub struct ProductionModulesConfig {
    /// Register memory recall when a memory backend is supplied.
    pub memory_recall: bool,
    /// Register AfterTurn memory writeback when a memory backend is supplied.
    pub memory_writeback: bool,
    /// Register preference recall when a preference store is supplied.
    pub preference_recall: bool,
    /// Register Judge-backed self-assessment when its store is supplied.
    pub self_assessment: bool,
    /// Enable the AI-evaluates-AI side-call.
    pub judge: JudgeConfig,
    /// Register the no-tool Council adapter.
    pub council: bool,
    /// Register filesystem tool module when workspace root is supplied.
    pub filesystem: bool,
    /// Register search tool module when workspace root is supplied.
    pub search: bool,
    /// Register repository inspection tool module when workspace root is supplied.
    pub repo: bool,
    /// Register shell tool module when config is supplied.
    pub shell: Option<TrustedShellConfig>,
    /// Register fetch tool module when config is supplied.
    pub fetch: Option<FetchConfig>,
    /// Register MCP capability module.
    pub mcp: bool,
}

impl Default for ProductionModulesConfig {
    fn default() -> Self {
        Self {
            memory_recall: true,
            memory_writeback: true,
            preference_recall: true,
            self_assessment: true,
            judge: JudgeConfig::default(),
            council: false,
            filesystem: true,
            search: true,
            repo: true,
            shell: None,
            fetch: None,
            mcp: false,
        }
    }
}

/// Compatibility alias for [`ProductionModulesConfig`].
pub type CognitiveModuleConfig = ProductionModulesConfig;

/// Concrete capability handles supplied by an adapter composition root.
///
/// Every field is optional so an embedding caller can choose a deliberate
/// subset.  A requested slot without its backend is a boot-time configuration
/// error, never a silently inert production registration.
#[derive(Default)]
pub struct ProductionBackends {
    /// Episode and history-stream backend.
    pub memory: Option<Arc<dyn MemoryBackend>>,
    /// Optional progressive-disclosure wiki store.
    pub wiki: Option<Arc<dyn WikiEntryStore>>,
    /// Optional knowledge graph store.
    pub graph: Option<Arc<dyn KnowledgeGraphStore>>,
    /// Optional association store.
    pub associations: Option<Arc<dyn AssociationStore>>,
    /// User preference store.
    pub preferences: Option<Arc<dyn PreferenceStore>>,
    /// Self-assessment store.
    pub self_assessments: Option<Arc<dyn SelfAssessmentStore>>,
    /// Council service, supplied only when the adapter explicitly enables it.
    pub council: Option<Arc<Council>>,
    /// Workspace root directory for local file tools.
    pub workspace_root: Option<PathBuf>,
}

/// Compatibility alias for [`ProductionBackends`].
pub type CognitiveBackends = ProductionBackends;

/// The validated, ordered module set to pass to [`RuntimeBuilder::with_module`].
pub struct ProductionModules {
    modules: Vec<Arc<dyn Module>>,
    telemetry: Arc<CognitiveTelemetry>,
}

/// Compatibility alias for [`ProductionModules`].
pub use ProductionModules as ProductionCognitiveModules;

impl ProductionModules {
    /// Build the canonical registration order.
    pub fn build(
        config: ProductionModulesConfig,
        backends: ProductionBackends,
        clock: Arc<dyn Clock>,
    ) -> RuntimeResult<Self> {
        let mut modules: Vec<Arc<dyn Module>> = Vec::new();
        let observations = Arc::new(JudgeObservations::default());
        let telemetry = Arc::new(CognitiveTelemetry::default());

        let experience_count = [
            backends.wiki.is_some(),
            backends.graph.is_some(),
            backends.associations.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();
        if experience_count != 0 && experience_count != 3 {
            return Err(RuntimeError::misconfigured(
                "Experience wiring must supply Wiki, knowledge graph, and association stores together",
            ));
        }

        // Register tool modules when configured and available
        if config.filesystem {
            if let Some(root) = &backends.workspace_root {
                modules.push(Arc::new(FilesystemModule::new(root.clone())));
            }
        }

        if config.search {
            if let Some(root) = &backends.workspace_root {
                modules.push(Arc::new(SearchModule::new(root.clone())));
            }
        }

        if config.repo {
            if let Some(root) = &backends.workspace_root {
                modules.push(Arc::new(RepoModule::new(root.clone())));
            }
        }

        if let Some(shell_config) = config.shell {
            modules.push(Arc::new(ShellModule::new(shell_config)));
        }

        if let Some(fetch_config) = config.fetch {
            modules.push(Arc::new(FetchModule::new(fetch_config)));
        }

        if config.mcp {
            modules.push(Arc::new(McpModule::new()));
        }

        // Register cognitive modules
        if config.memory_recall {
            let memory = required(backends.memory.clone(), "memory_recall", "memory")?;
            let mut module = MemoryRecallModule::new(memory);
            if let (Some(wiki), Some(graph), Some(associations)) =
                (&backends.wiki, &backends.graph, &backends.associations)
            {
                module = module.with_experience(
                    Arc::clone(wiki),
                    Arc::clone(graph),
                    Arc::clone(associations),
                );
            }
            modules.push(Arc::new(module.with_telemetry(Arc::clone(&telemetry))));
        }

        if config.preference_recall {
            modules.push(Arc::new(
                PreferenceRecallModule::new(required(
                    backends.preferences.clone(),
                    "preference_recall",
                    "preferences",
                )?)
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.judge.enabled {
            modules.push(Arc::new(
                JudgeModule::new(config.judge, Arc::clone(&observations))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.self_assessment {
            modules.push(Arc::new(
                SelfAssessmentModule::new(
                    required(
                        backends.self_assessments.clone(),
                        "self_assessment",
                        "self_assessments",
                    )?,
                    Arc::clone(&clock),
                    observations,
                )
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.council {
            modules.push(Arc::new(
                CouncilModule::new(
                    required(backends.council, "council", "council")?,
                    Arc::clone(&clock),
                )
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.memory_writeback {
            let mut module = MemoryWritebackModule::new(
                required(backends.memory, "memory_writeback", "memory")?,
                clock,
            );
            if let (Some(wiki), Some(graph), Some(associations)) =
                (&backends.wiki, &backends.graph, &backends.associations)
            {
                module = module.with_experience(
                    Arc::clone(wiki),
                    Arc::clone(graph),
                    Arc::clone(associations),
                );
            }
            modules.push(Arc::new(module.with_telemetry(Arc::clone(&telemetry))));
        }

        let mut seen = std::collections::BTreeSet::new();
        for module in &modules {
            let id = module.manifest().id.clone();
            if !seen.insert(id.clone()) {
                return Err(RuntimeError::misconfigured(format!(
                    "duplicate cognitive module id {id:?}"
                )));
            }
        }

        Ok(Self { modules, telemetry })
    }

    /// Ordered modules for registration in the canonical runtime.
    pub fn modules(&self) -> &[Arc<dyn Module>] {
        &self.modules
    }

    /// Consume the set into the builder's module registration calls.
    pub fn register_into(
        self,
        mut builder: super::runtime::RuntimeBuilder,
    ) -> super::runtime::RuntimeBuilder {
        builder = builder.with_cognitive_telemetry(Arc::clone(&self.telemetry));
        for module in self.modules {
            builder = builder.with_module(module);
        }
        builder
    }

    /// Stable slot ids in the exact registration order.
    pub fn ids(&self) -> Vec<String> {
        self.modules
            .iter()
            .map(|module| module.manifest().id.clone())
            .collect()
    }

    /// Shared non-sensitive hook telemetry for the registered modules.
    pub fn telemetry(&self) -> Arc<CognitiveTelemetry> {
        Arc::clone(&self.telemetry)
    }
}

fn required<T>(value: Option<Arc<T>>, slot: &str, dependency: &str) -> RuntimeResult<Arc<T>>
where
    T: ?Sized,
{
    value.ok_or_else(|| {
        RuntimeError::misconfigured(format!(
            "cognitive slot {slot:?} requires injected backend {dependency:?}"
        ))
    })
}

//! Tests proving one module world: unified registration, unified dispatch, and
//! dynamic MCP capability ownership through the same manager.
//!
//! These exercise the converged model end to end through the runtime:
//!
//! - a module may own lifecycle, hooks, tools, and providers at once;
//! - `runtime.tools()` reports what exists once — no module world merged with a
//!   plugin world;
//! - an installed-origin module uses exactly the registry path a builtin one
//!   does;
//! - a duplicate module id, capability id, or model-facing tool name is
//!   rejected before anything starts;
//! - a missing dependency fails the build before any module runs;
//! - an MCP tool admitted after boot cannot steal a builtin tool's id or name.

use apeireth_runtime_assembly as apeireth_runtime;

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Lifecycle, ModelId, ModuleId, SessionId};
use apeireth_governance::AllowAll;
use apeireth_plugin::{
    CapabilityKind, HookPoint, Module, ModuleContext, ModuleError, ModuleExports,
    ModuleInitContext, ModuleManifest, ModuleOrigin, ModuleOutcome, ModuleSystemError,
    PluginResult, ProviderCapability, ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedTool, ToolCall,
    ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{
    BuiltinToolsModule, McpModule, Runtime, RuntimeError, TurnRequest,
};
use apeireth_runtime_assembly::canonical::MCP_MODULE_ID;
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

struct MockMcpTool;

#[async_trait]
impl ToolCapability for MockMcpTool {
    fn id(&self) -> &CapabilityId {
        static ID: std::sync::OnceLock<CapabilityId> = std::sync::OnceLock::new();
        ID.get_or_init(|| CapabilityId::new("tool.mcp.custom_search").unwrap())
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "mcp_custom_search".into(),
            description: Some("Custom search from MCP server".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, serde_json::json!({ "results": ["found item"] }))
    }
}

struct CollidingTool {
    id: CapabilityId,
    name: String,
}

#[async_trait]
impl ToolCapability for CollidingTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.name.clone(),
            description: Some("colliding tool".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, serde_json::json!({ "stolen": true }))
    }
}

#[derive(Clone)]
enum Scripted {
    CallTool {
        call_id: &'static str,
        tool: &'static str,
        arguments: serde_json::Value,
    },
    Say(&'static str),
}

/// A scripted completion source. The provider capability is plain data; the
/// module that owns it is what carries the manifest.
struct ScriptedProviderCore {
    id: CapabilityId,
    script: Vec<Scripted>,
    calls: std::sync::atomic::AtomicUsize,
}

impl ScriptedProviderCore {
    fn new(id: &str, script: Vec<Scripted>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            script,
            calls: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    fn respond(&self, request: &NormalizedRequest) -> Result<NormalizedResponse, ProviderError> {
        let index = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let step = self
            .script
            .get(index)
            .unwrap_or_else(|| panic!("called beyond script"));
        let base = NormalizedResponse {
            id: format!("resp_{}", index + 1),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: apeireth_protocol::canonical::NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        match step {
            Scripted::CallTool {
                call_id,
                tool,
                arguments,
            } => Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![ToolCall {
                    id: (*call_id).to_string(),
                    name: (*tool).to_string(),
                    arguments: arguments.clone(),
                }],
                ..base
            }),
            Scripted::Say(text) => Ok(NormalizedResponse {
                content: (*text).to_string(),
                ..base
            }),
        }
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProviderCore {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new("mock-model").unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.respond(request)
    }
}

/// The module that owns a scripted provider. This is the object a "provider
/// plugin" always was: a module exporting one provider capability.
struct ScriptedProviderModule {
    manifest: ModuleManifest,
    provider: Arc<ScriptedProviderCore>,
}

impl ScriptedProviderModule {
    fn new(id: &str, module_id: &str, provider: Arc<ScriptedProviderCore>) -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::system(module_id, "1.0.0", "Scripted completions")
                .declare_capability(
                    CapabilityId::new(id).unwrap(),
                    CapabilityKind::Provider,
                    "Scripted completions",
                )
                .unwrap()
                .with_origin(ModuleOrigin::Installed),
            provider,
        })
    }
}

#[async_trait]
impl Module for ScriptedProviderModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &ModuleInitContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

/// A module that owns a provider capability, a tool capability, a hook, and a
/// lifecycle at once — the combination the converged trait exists to allow.
struct DualModule {
    manifest: ModuleManifest,
    booted: std::sync::atomic::AtomicUsize,
    hooks_seen: std::sync::Mutex<Vec<String>>,
    provider: Arc<ScriptedProviderCore>,
}

impl DualModule {
    fn new(provider: Arc<ScriptedProviderCore>) -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::system("vendor.dual", "1.0.0", "everything at once")
                .declare_capability(
                    CapabilityId::new("tool.mcp.custom_search").unwrap(),
                    CapabilityKind::Tool,
                    "custom search",
                )
                .unwrap()
                .declare_capability(
                    CapabilityId::new("provider.dual").unwrap(),
                    CapabilityKind::Provider,
                    "scripted completions",
                )
                .unwrap()
                .observes(HookPoint::TurnStart)
                .observes(HookPoint::AfterTurn)
                .with_origin(ModuleOrigin::Installed),
            booted: std::sync::atomic::AtomicUsize::new(0),
            hooks_seen: std::sync::Mutex::new(Vec::new()),
            provider,
        })
    }
}

#[async_trait]
impl Module for DualModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &ModuleInitContext) -> PluginResult<()> {
        self.booted
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        self.booted
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn exports(&self) -> ModuleExports {
        ModuleExports::new()
            .with_tools(vec![Arc::new(MockMcpTool) as Arc<dyn ToolCapability>])
            .with_providers(vec![
                Arc::clone(&self.provider) as Arc<dyn ProviderCapability>
            ])
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        self.hooks_seen.lock().unwrap().push(format!("{hook:?}"));
        Ok(ModuleOutcome::continue_())
    }
}

// ---------------------------------------------------------------------------
// One module world
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_modules_register_and_offer_declarations_to_turn() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path().to_path_buf();

    let tools = Arc::new(BuiltinToolsModule::new(root.clone()));
    let mcp = Arc::new(
        McpModule::new()
            .with_tool(Arc::new(MockMcpTool))
            .expect("mcp tool registers"),
    );

    let runtime = Runtime::builder()
        .with_module(Arc::clone(&tools) as Arc<dyn Module>)
        .with_module(Arc::clone(&mcp) as Arc<dyn Module>)
        .build()
        .await
        .expect("runtime build succeeds with tool modules");

    let decls = runtime.tool_declarations();
    let names: Vec<String> = decls.into_iter().map(|t| t.name).collect();

    assert!(names.contains(&"filesystem".to_string()));
    assert!(names.contains(&"search".to_string()));
    assert!(names.contains(&"repo".to_string()));
    assert!(names.contains(&"mcp_custom_search".to_string()));

    // One world: every tool appears exactly once, whoever owns it, and no
    // declaration is duplicated because two registries were merged.
    let live: Vec<String> = runtime
        .tools()
        .iter()
        .map(|tool| tool.declaration().name)
        .collect();
    for name in ["filesystem", "search", "repo", "mcp_custom_search"] {
        assert_eq!(
            live.iter().filter(|seen| seen.as_str() == name).count(),
            1,
            "{name} must be listed exactly once"
        );
    }
    assert_eq!(live.len(), names.len());
}

#[tokio::test]
async fn one_module_carries_lifecycle_hooks_tools_and_providers() {
    let provider = ScriptedProviderCore::new("provider.dual", vec![Scripted::Say("all good")]);
    let module = DualModule::new(Arc::clone(&provider));

    let mut runtime = Runtime::builder()
        .with_module(Arc::clone(&module) as Arc<dyn Module>)
        .with_governance(Arc::new(AllowAll))
        .with_default_model("mock-model")
        .build()
        .await
        .expect("one module with every surface builds");

    // Lifecycle ran.
    assert_eq!(
        module.booted.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "initialize ran"
    );

    // An installed-origin module takes the identical registry path a builtin
    // one does: same registry, same lifecycle states, same capability index.
    let id = ModuleId::new("vendor.dual").unwrap();
    let manager = runtime.module_manager();
    assert_eq!(manager.state(&id).unwrap(), Lifecycle::Active);
    assert_eq!(manager.modules().len(), 1);
    assert_eq!(manager.capabilities().len(), 2);
    assert_eq!(
        manager
            .capabilities()
            .owner(&CapabilityId::new("tool.mcp.custom_search").unwrap())
            .unwrap()
            .as_str(),
        "vendor.dual"
    );
    assert_eq!(
        manager.modules().modules()[0].manifest().origin,
        ModuleOrigin::Installed
    );

    // Provider routing reads the same capability index tools do.
    assert_eq!(runtime.providers().len(), 1);

    // The hook ran through the ordinary hook path, and the provider it owns
    // served the turn.
    let request = TurnRequest::new(SessionId::new(), "hello").with_model("mock-model");
    let response = runtime.execute(request).await.expect("turn runs");
    assert_eq!(response.text, "all good");
    let seen = module.hooks_seen.lock().unwrap();
    assert!(seen.contains(&"TurnStart".to_string()), "{seen:?}");
    assert!(
        seen.contains(&"AfterTurn".to_string()),
        "declared hooks run through the ordinary hook path: {seen:?}"
    );
    // Declaration is the dispatch list: this module declares exactly these two
    // points, so no other point of the turn may reach it.
    for undeclared in ["BeforeModelCall", "AfterModelResponse", "BeforeFinalCommit"] {
        assert!(
            !seen.iter().any(|hook| hook == undeclared),
            "undeclared hook {undeclared} was dispatched to this module: {seen:?}"
        );
    }

    // Shutdown ran.
    assert!(runtime.shutdown().await.is_empty());
    assert_eq!(module.booted.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[tokio::test]
async fn tool_module_invocation_executes_end_to_end() {
    let script = vec![
        Scripted::CallTool {
            call_id: "call_mcp_1",
            tool: "mcp_custom_search",
            arguments: serde_json::json!({ "query": "hello" }),
        },
        Scripted::Say("MCP search returned found item"),
    ];
    let provider = ScriptedProviderCore::new("provider.mock", script);
    let module = ScriptedProviderModule::new("provider.mock", "vendor.mock", provider);
    let mcp = Arc::new(
        McpModule::new()
            .with_tool(Arc::new(MockMcpTool))
            .expect("mcp tool registers"),
    );

    let mut runtime = Runtime::builder()
        .with_module(Arc::clone(&mcp) as Arc<dyn Module>)
        .with_module(module as Arc<dyn Module>)
        .with_governance(Arc::new(AllowAll))
        .with_default_model("mock-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    let req = TurnRequest::new(SessionId::new(), "search for hello").with_model("mock-model");

    let response = runtime.execute(req).await.expect("turn executes");

    assert_eq!(response.text, "MCP search returned found item");
    assert_eq!(response.rounds, 2);
}

// ---------------------------------------------------------------------------
// Dynamic MCP capability ownership
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mcp_dynamic_registration_and_unregistration_after_build_is_live() {
    let script = vec![
        // Turn 1: Successfully calls dynamic tool
        Scripted::CallTool {
            call_id: "dynamic_call_1",
            tool: "mcp_custom_search",
            arguments: serde_json::json!({ "query": "test" }),
        },
        Scripted::Say("Search handled dynamically"),
        // Turn 2: Attempting to call after unregistering
        Scripted::CallTool {
            call_id: "dynamic_call_2",
            tool: "mcp_custom_search",
            arguments: serde_json::json!({ "query": "test" }),
        },
        Scripted::Say("Recovered from missing tool"),
    ];

    let provider = ScriptedProviderCore::new("provider.mock", script);
    let module = ScriptedProviderModule::new("provider.mock", "vendor.mock", provider);
    let mcp = Arc::new(McpModule::new()); // Initially empty

    let mut runtime = Runtime::builder()
        .with_module(Arc::clone(&mcp) as Arc<dyn Module>)
        .with_module(module as Arc<dyn Module>)
        .with_governance(Arc::new(AllowAll))
        .with_default_model("mock-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    // Initially no tools offered
    assert_eq!(runtime.tool_declarations().len(), 0);

    // Dynamic registration post-build
    let tool = Arc::new(MockMcpTool);
    let tool_id = tool.id().clone();
    runtime
        .register_dynamic_tool(MCP_MODULE_ID, tool)
        .await
        .expect("dynamic mcp registration is live");

    // Tool declarations immediately reflect dynamic tool
    assert_eq!(runtime.tool_declarations().len(), 1);
    assert_eq!(runtime.tool_declarations()[0].name, "mcp_custom_search");

    // The owning module is recorded in the one capability index.
    assert_eq!(
        runtime
            .module_manager()
            .capabilities()
            .owner(&tool_id)
            .unwrap()
            .as_str(),
        MCP_MODULE_ID
    );

    // Turn 1 executes with the dynamically registered tool
    let req1 = TurnRequest::new(SessionId::new(), "search query").with_model("mock-model");
    let resp1 = runtime.execute(req1).await.expect("turn 1 executes");
    assert_eq!(resp1.text, "Search handled dynamically");

    // Unregister through the same path it arrived on.
    assert!(runtime
        .unregister_dynamic_tool(MCP_MODULE_ID, &tool_id)
        .await
        .expect("unregister succeeds"));

    // Tool declarations immediately reflect removal
    assert_eq!(runtime.tool_declarations().len(), 0);

    // Turn 2 fails to find the tool, runtime records error and model recovers cleanly
    let req2 = TurnRequest::new(SessionId::new(), "search query again").with_model("mock-model");
    let resp2 = runtime.execute(req2).await.expect("turn 2 executes");
    assert_eq!(resp2.text, "Recovered from missing tool");
}

#[tokio::test]
async fn hostile_mcp_cannot_steal_builtin_name_or_capability_id() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut runtime = Runtime::builder()
        .with_module(Arc::new(BuiltinToolsModule::new(temp_dir.path())) as Arc<dyn Module>)
        .with_module(Arc::new(McpModule::new()) as Arc<dyn Module>)
        .with_governance(Arc::new(AllowAll))
        .build()
        .await
        .expect("runtime builds");

    let name_collision = Arc::new(CollidingTool {
        id: CapabilityId::new("tool.mcp.imposter").unwrap(),
        name: "repo".into(),
    });
    let name_err = runtime
        .register_dynamic_tool(MCP_MODULE_ID, name_collision)
        .await
        .expect_err("duplicate model-facing name must be rejected");
    assert!(
        name_err.to_string().contains("model-facing tool name"),
        "{name_err}"
    );

    let id_collision = Arc::new(CollidingTool {
        id: CapabilityId::new("tool.repo").unwrap(),
        name: "not_repo".into(),
    });
    let id_err = runtime
        .register_dynamic_tool(MCP_MODULE_ID, id_collision)
        .await
        .expect_err("duplicate capability id must be rejected");
    assert!(id_err.to_string().contains("declared by both"), "{id_err}");

    // Nothing was admitted, so the live set is still exactly the builtin tools.
    let names: Vec<String> = runtime
        .tool_declarations()
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert_eq!(names, vec!["filesystem", "repo", "search"]);
}

#[tokio::test]
async fn a_module_local_bag_cannot_collide_with_itself() {
    let mcp = McpModule::new()
        .with_tool(Arc::new(MockMcpTool))
        .expect("first tool");
    let err = mcp
        .register_tool(Arc::new(MockMcpTool))
        .expect_err("duplicate id inside mcp bag");
    assert!(err.contains("declared by both"), "{err}");

    // A dynamic owner's exports are its truth: what it holds at registration is
    // live, exactly like a static module's declarations are.
    let runtime = Runtime::builder()
        .with_module(Arc::new(mcp) as Arc<dyn Module>)
        .build()
        .await
        .expect("runtime builds");
    assert_eq!(runtime.tool_declarations().len(), 1);
    assert_eq!(runtime.tools().len(), 1);
}

#[tokio::test]
async fn a_tool_that_bypasses_the_manager_is_not_dispatchable() {
    // The module-local bag can be filled directly, skipping the manager's global
    // uniqueness check. Such a tool is not indexed, so it is neither dispatchable
    // nor listed: the only way in is the one path that checks everyone.
    let mcp = Arc::new(McpModule::new());
    let mut runtime = Runtime::builder()
        .with_module(Arc::clone(&mcp) as Arc<dyn Module>)
        .build()
        .await
        .expect("runtime builds");

    let smuggled: Arc<dyn ToolCapability> = Arc::new(CollidingTool {
        id: CapabilityId::new("tool.mcp.smuggled").unwrap(),
        name: "mcp_smuggled".into(),
    });
    mcp.register_tool(Arc::clone(&smuggled))
        .expect("the module-local bag accepts it");
    assert_eq!(mcp.tools().len(), 1);
    assert!(runtime.tool_declarations().is_empty());
    assert!(runtime.tools().is_empty());

    // Admitted through the one path, it becomes live immediately.
    runtime
        .register_dynamic_tool(MCP_MODULE_ID, smuggled)
        .await
        .expect("admitted through the manager");
    assert_eq!(runtime.tool_declarations().len(), 1);
    assert_eq!(runtime.tool_declarations()[0].name, "mcp_smuggled");
    let _ = runtime.shutdown().await;
}

#[tokio::test]
async fn a_module_without_dynamic_ownership_refuses_a_dynamic_tool() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut runtime = Runtime::builder()
        .with_module(Arc::new(BuiltinToolsModule::new(temp_dir.path())) as Arc<dyn Module>)
        .build()
        .await
        .expect("runtime builds");

    let late = Arc::new(CollidingTool {
        id: CapabilityId::new("tool.late").unwrap(),
        name: "late".into(),
    });
    let error = runtime
        .register_dynamic_tool("builtin.tools", late)
        .await
        .expect_err("a static module must refuse dynamic capability");
    assert!(
        matches!(
            error,
            RuntimeError::Plugin(ModuleSystemError::DynamicCapabilityUnsupported { .. })
        ),
        "{error}"
    );
}

// ---------------------------------------------------------------------------
// Registration discipline
// ---------------------------------------------------------------------------

#[tokio::test]
async fn two_modules_cannot_claim_one_capability() {
    let temp_dir = tempfile::tempdir().unwrap();
    // One builtin tools module, and an installed module that declares the very
    // same tool capability.
    let builtin = BuiltinToolsModule::new(temp_dir.path());
    let imposter = ImposterModule::new();

    let error = Runtime::builder()
        .with_module(Arc::new(builtin) as Arc<dyn Module>)
        .with_module(Arc::new(imposter) as Arc<dyn Module>)
        .build()
        .await
        .expect_err("a duplicate capability id must fail the build");

    assert!(
        error.to_string().contains("tool.filesystem"),
        "the collision must name the contested capability, got {error}"
    );
}

struct ImposterModule {
    manifest: ModuleManifest,
}

impl ImposterModule {
    fn new() -> Self {
        Self {
            manifest: ModuleManifest::system("vendor.imposter", "1.0.0", "claims a builtin tool")
                .declare_capability(
                    CapabilityId::new("tool.filesystem").unwrap(),
                    CapabilityKind::Tool,
                    "not the filesystem tool",
                )
                .unwrap(),
        }
    }
}

#[async_trait]
impl Module for ImposterModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn exports(&self) -> ModuleExports {
        ModuleExports::new().with_tools(vec![Arc::new(CollidingTool {
            id: CapabilityId::new("tool.filesystem").unwrap(),
            name: "filesystem".into(),
        })])
    }
}

#[tokio::test]
async fn registering_the_same_module_twice_is_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let error = Runtime::builder()
        .with_module(Arc::new(BuiltinToolsModule::new(temp_dir.path())) as Arc<dyn Module>)
        .with_module(Arc::new(BuiltinToolsModule::new(temp_dir.path())) as Arc<dyn Module>)
        .build()
        .await
        .expect_err("a duplicate module id must fail the build");
    assert!(error.to_string().contains("already registered"), "{error}");
}

#[tokio::test]
async fn a_missing_dependency_is_caught_before_any_module_starts() {
    let dependent = Arc::new(DependentModule::new());
    let error = Runtime::builder()
        .with_module(Arc::clone(&dependent) as Arc<dyn Module>)
        .build()
        .await
        .expect_err("a missing dependency must fail the build");
    assert!(error.to_string().contains("not registered"), "{error}");
    assert_eq!(
        dependent.boots.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "no module may run before validation completes"
    );
}

struct DependentModule {
    manifest: ModuleManifest,
    boots: std::sync::atomic::AtomicUsize,
}

impl DependentModule {
    fn new() -> Self {
        Self {
            manifest: ModuleManifest::system(
                "builtin.dependent",
                "1.0.0",
                "needs an absent module",
            )
            .depends_on(ModuleId::new("builtin.absent").unwrap()),
            boots: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Module for DependentModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &ModuleInitContext) -> PluginResult<()> {
        self.boots.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn dependency_order_is_respected_and_shutdown_reverses_it() {
    let log = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let first = OrderedModule::new("builtin.first", Arc::clone(&log), Vec::new());
    let second = OrderedModule::new("builtin.second", Arc::clone(&log), vec!["builtin.first"]);
    // Registered in an order that contradicts the dependency chain.
    let mut runtime = Runtime::builder()
        .with_module(Arc::clone(&second) as Arc<dyn Module>)
        .with_module(Arc::clone(&first) as Arc<dyn Module>)
        .build()
        .await
        .expect("runtime builds");

    {
        let entries = log.lock().unwrap();
        assert_eq!(entries.as_slice(), ["builtin.first", "builtin.second"]);
    }

    assert!(runtime.shutdown().await.is_empty());
    let entries = log.lock().unwrap();
    assert_eq!(
        entries.as_slice(),
        [
            "builtin.first",
            "builtin.second",
            "stop:builtin.second",
            "stop:builtin.first"
        ]
    );
}

struct OrderedModule {
    manifest: ModuleManifest,
    log: Arc<std::sync::Mutex<Vec<String>>>,
}

impl OrderedModule {
    fn new(id: &str, log: Arc<std::sync::Mutex<Vec<String>>>, deps: Vec<&str>) -> Arc<Self> {
        let mut manifest = ModuleManifest::system(id, "1.0.0", "ordered module");
        for dep in deps {
            manifest = manifest.depends_on(ModuleId::new(dep).unwrap());
        }
        Arc::new(Self { manifest, log })
    }
}

#[async_trait]
impl Module for OrderedModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &ModuleInitContext) -> PluginResult<()> {
        self.log.lock().unwrap().push(self.manifest.id.to_string());
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        self.log
            .lock()
            .unwrap()
            .push(format!("stop:{}", self.manifest.id));
        Ok(())
    }
}

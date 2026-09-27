//! End-to-end: external (MCP) dynamic tools through the canonical runtime.
//!
//! The runtime, plugin registry, capability registry, governance, provider
//! router, and the production assembly are real; the external server is the
//! in-process JSON-RPC test double (no sockets, no subprocesses). The chain
//! under test: production assembly loads the server list (fail-closed), the
//! bridge discovers tools, they register as dynamic tools, and a real turn
//! dispatches through the five-stage pipeline into the server and back.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::clock::SystemClock;
use apeireth_core::kernel::{CapabilityId, Clock, SessionId};
use apeireth_core::stored_doc;
use apeireth_governance::AllowAll;
use apeireth_plugin::{ProviderCapability, ProviderError};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedUsage, ToolCall,
};
use apeireth_runtime::canonical::{
    McpModule, ProductionBackends, ProductionModules, ProductionModulesConfig, Runtime, TurnRequest,
};
use apeireth_runtime_assembly as apeireth_runtime;
use apeireth_tools_canonical::mcp_bridge::{
    doc_compat, server_list_path, InProcessMcpServer, McpBridgeOptions, McpPermissionMapping,
    McpServerConfig, McpServerConnection, McpServerSpec, McpToolBridge, McpToolRegistry,
    MockToolSpec,
};
use async_trait::async_trait;

const MODEL: &str = "mock-model";
const SERVER: &str = "demo";

fn test_options() -> McpBridgeOptions {
    McpBridgeOptions::default().with_call_timeout(5_000, 10_000)
}

fn demo_spec() -> McpServerSpec {
    McpServerSpec::stdio(SERVER, "in-process-test-server")
}

/// A provider that emits one scripted tool call, then closes the turn.
struct ScriptedProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    tool_call: ToolCall,
    final_text: String,
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            apeireth_core::kernel::ModelId::new(MODEL).unwrap(),
            self.id.clone(),
        )
        .with_feature(ModelFeature::ToolCalls)]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        let base = NormalizedResponse {
            id: format!("resp_{index}"),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        if index == 0 {
            return Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![self.tool_call.clone()],
                ..base
            });
        }
        Ok(NormalizedResponse {
            content: self.final_text.clone(),
            ..base
        })
    }
}

struct ScriptedProviderPlugin {
    manifest: apeireth_plugin::PluginManifest,
    provider: Arc<ScriptedProvider>,
}

#[async_trait]
impl apeireth_plugin::Plugin for ScriptedProviderPlugin {
    fn manifest(&self) -> &apeireth_plugin::PluginManifest {
        &self.manifest
    }

    async fn initialize(
        &self,
        _ctx: &apeireth_plugin::PluginContext,
    ) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

fn scripted_plugin(tool_call: ToolCall, final_text: &str) -> Arc<ScriptedProviderPlugin> {
    let provider = Arc::new(ScriptedProvider {
        id: CapabilityId::new("provider.mock").unwrap(),
        calls: AtomicUsize::new(0),
        tool_call,
        final_text: final_text.to_string(),
    });
    Arc::new(ScriptedProviderPlugin {
        manifest: apeireth_plugin::PluginManifest::new(
            apeireth_core::kernel::PluginId::new("vendor.mock").unwrap(),
            "1.0.0",
            "Scripted provider for external tool acceptance",
        )
        .declare_capability(
            provider.id.clone(),
            apeireth_plugin::CapabilityKind::Provider,
            "Scripted completions",
        )
        .unwrap(),
        provider,
    })
}

fn bridge_over(server: &Arc<InProcessMcpServer>) -> McpToolBridge {
    let config = McpServerConfig::new(vec![demo_spec()]);
    let options = test_options();
    let connection = Arc::new(McpServerConnection::new(
        demo_spec(),
        server.factory(),
        &options,
    ));
    McpToolBridge::from_connections(config, options, vec![connection]).unwrap()
}

fn production_config(mcp: bool, data_dir: Option<std::path::PathBuf>) -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        mcp,
        mcp_data_dir: data_dir,
        ..ProductionModulesConfig::default()
    }
}

fn build_production(config: ProductionModulesConfig) -> Result<ProductionModules, String> {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    ProductionModules::build(config, ProductionBackends::default(), clock)
        .map_err(|error| error.to_string())
}

#[tokio::test]
async fn production_assembly_loads_the_server_list_when_enabled() {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-mcp-runtime-config-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let body = McpServerConfig::new(vec![McpServerSpec::stdio(SERVER, "demo-server")
        .with_arg("--token")
        .with_arg("secret-value")]);
    stored_doc::save_single(
        &server_list_path(&dir),
        &doc_compat(),
        body,
        stored_doc::DEFAULT_DOC_MODE,
    )
    .unwrap();

    let modules = build_production(production_config(true, Some(dir.clone())))
        .expect("assembly must accept a valid stored server list");
    let bridge = modules.mcp_bridge().expect("bridge assembled");
    assert_eq!(bridge.config().servers.len(), 1);
    assert!(modules.mcp_module().is_some());
    for line in bridge.startup_log() {
        assert!(!line.contains("secret-value"), "token leaked: {line}");
    }

    // The slot stays light: disabled by default, no bridge assembled.
    let off = build_production(production_config(false, None)).expect("default assembly");
    assert!(off.mcp_bridge().is_none());

    // A defective stored configuration refuses to assemble (fail-closed).
    std::fs::write(server_list_path(&dir), "{ not a stored document").unwrap();
    let Err(err) = build_production(production_config(true, Some(dir.clone()))) else {
        panic!("a broken server list must refuse to open");
    };
    assert!(err.contains("refused to load"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn dynamic_external_tool_serves_a_full_runtime_turn() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("echo", "echo says hi").with_description("Echo one message")
    ]);
    let bridge = bridge_over(&server);

    // Discovery → dynamic registration (module bag + live registry).
    let report = bridge.refresh().await.expect("discovery");
    assert_eq!(report.registered_names(), ["mcp:demo:echo"]);
    McpPermissionMapping::authorize(
        &mut bridge.policy().lock().unwrap(),
        &bridge.catalog().get("mcp:demo:echo").unwrap(),
    );

    let provider_call = ToolCall {
        id: "call_mcp_1".into(),
        name: "mcp:demo:echo".into(),
        arguments: serde_json::json!({ "text": "hi" }),
    };
    let plugin = scripted_plugin(provider_call, "turn complete");

    let mut runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_governance(Arc::new(AllowAll))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .expect("runtime builds");

    let mcp_module = Arc::new(McpModule::new());
    report
        .apply_to(mcp_module.as_ref())
        .expect("module bag accepts the discovered tools");
    for tool in &report.registered {
        runtime
            .register_dynamic_tool("module.mcp", Arc::clone(tool))
            .expect("dynamic registration is live");
    }

    let names: Vec<String> = runtime
        .tool_declarations()
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert!(names.contains(&"mcp:demo:echo".to_string()), "{names:?}");

    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "please echo").with_model(MODEL))
        .await
        .expect("turn executes");
    assert_eq!(response.text, "turn complete");

    let calls = server.call_log();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "echo");
    assert_eq!(calls[0].1, serde_json::json!({ "text": "hi" }));

    // The full chain is observable: five stages, one remote call.
    assert_eq!(server.method_count("tools/call"), 1);
}

#[tokio::test]
async fn duplicate_dynamic_registration_is_rejected_by_the_module_bag() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("echo", "hi")]);
    let bridge = bridge_over(&server);
    let report = bridge.refresh().await.expect("discovery");

    let mcp_module = McpModule::new();
    report.apply_to(&mcp_module).expect("first registration");
    let err = report
        .apply_to(&mcp_module)
        .expect_err("the same name must be refused twice");
    assert!(err.to_string().contains("duplicate capability id"), "{err}");

    // The live registry rejects a colliding model-facing name as well.
    let runtime = Runtime::builder()
        .with_governance(Arc::new(AllowAll))
        .build()
        .await
        .expect("runtime builds");
    runtime
        .register_dynamic_tool("module.mcp", report.registered[0].clone())
        .expect("first live registration");
    let err = runtime
        .register_dynamic_tool("module.mcp", report.registered[0].clone())
        .expect_err("duplicate live registration must fail closed");
    assert!(err.to_string().contains("duplicate"), "{err}");
}

#[tokio::test]
async fn unauthorized_external_call_pends_approval_before_any_server_contact() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let report = bridge.refresh().await.expect("discovery");

    // No authorization is granted: the risk mapping keeps the call at the
    // require-approval level, and the server is never contacted.
    let provider_call = ToolCall {
        id: "call_mcp_1".into(),
        name: "mcp:demo:echo".into(),
        arguments: serde_json::json!({}),
    };
    let plugin = scripted_plugin(provider_call, "recovered");
    let mut runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_governance(Arc::new(AllowAll))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .expect("runtime builds");
    for tool in &report.registered {
        runtime
            .register_dynamic_tool("module.mcp", Arc::clone(tool))
            .expect("dynamic registration is live");
    }

    let response = runtime
        .execute(TurnRequest::new(SessionId::new(), "please echo").with_model(MODEL))
        .await
        .expect("turn executes");
    assert_eq!(response.text, "recovered");
    assert!(
        server.call_log().is_empty(),
        "an unauthorized external tool must not reach the server"
    );
}

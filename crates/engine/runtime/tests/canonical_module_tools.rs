//! Tests proving tool capabilities owned and registered via Modules.

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::{system_clock, CapabilityId, RequestId, SessionId, TraceId};
use apeireth_governance::AllowAll;
use apeireth_plugin::{PluginResult, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use apeireth_runtime::{
    FilesystemModule, McpModule, RepoModule, Runtime, SearchModule, TurnRequest,
};
use apeireth_tools_canonical::BuiltinToolsPlugin;
use async_trait::async_trait;

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

#[tokio::test]
async fn tool_modules_register_and_offer_declarations_to_turn() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path().to_path_buf();

    let fs_module = Arc::new(FilesystemModule::new(root.clone()));
    let search_module = Arc::new(SearchModule::new(root.clone()));
    let repo_module = Arc::new(RepoModule::new(root.clone()));
    let mcp_module = Arc::new(McpModule::new().with_tool(Arc::new(MockMcpTool)));

    let runtime = Runtime::builder()
        .with_module(fs_module)
        .with_module(search_module)
        .with_module(repo_module)
        .with_module(mcp_module)
        .build()
        .await
        .expect("runtime build succeeds with tool modules");

    let decls = runtime.tool_declarations();
    let names: Vec<String> = decls.into_iter().map(|t| t.name).collect();

    assert!(names.contains(&"filesystem".to_string()));
    assert!(names.contains(&"search".to_string()));
    assert!(names.contains(&"repo".to_string()));
    assert!(names.contains(&"mcp_custom_search".to_string()));
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

struct ScriptedProvider {
    id: CapabilityId,
    script: Vec<Scripted>,
    calls: std::sync::atomic::AtomicUsize,
}

impl ScriptedProvider {
    fn new(id: &str, script: Vec<Scripted>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            script,
            calls: std::sync::atomic::AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl apeireth_plugin::ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<apeireth_protocol::canonical::ModelDescriptor> {
        vec![apeireth_protocol::canonical::ModelDescriptor::new(
            apeireth_core::kernel::ModelId::new("mock-model").unwrap(),
            self.id.clone(),
        )
        .with_feature(apeireth_protocol::canonical::ModelFeature::ToolCalls)]
    }

    async fn complete(
        &self,
        request: &apeireth_protocol::canonical::NormalizedRequest,
    ) -> Result<apeireth_protocol::canonical::NormalizedResponse, apeireth_plugin::ProviderError>
    {
        let index = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let step = self
            .script
            .get(index)
            .unwrap_or_else(|| panic!("called beyond script"));

        let base = apeireth_protocol::canonical::NormalizedResponse {
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
            } => Ok(apeireth_protocol::canonical::NormalizedResponse {
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
            Scripted::Say(text) => Ok(apeireth_protocol::canonical::NormalizedResponse {
                content: (*text).to_string(),
                ..base
            }),
        }
    }
}

struct ScriptedProviderPlugin {
    manifest: apeireth_plugin::PluginManifest,
    provider: Arc<ScriptedProvider>,
}

impl ScriptedProviderPlugin {
    fn new(provider: Arc<ScriptedProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: apeireth_plugin::PluginManifest::new(
                apeireth_core::kernel::PluginId::new("vendor.mock").unwrap(),
                "1.0.0",
                "Mock provider plugin",
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

    fn providers(&self) -> Vec<Arc<dyn apeireth_plugin::ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn apeireth_plugin::ProviderCapability>]
    }
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
    let provider = ScriptedProvider::new("provider.mock", script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let mcp_module = Arc::new(McpModule::new().with_tool(Arc::new(MockMcpTool)));

    let mut runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_module(mcp_module)
        .with_default_model("mock-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    let req = TurnRequest::new(SessionId::new(), "search for hello").with_model("mock-model");

    let response = runtime.execute(req).await.expect("turn executes");

    assert_eq!(response.text, "MCP search returned found item");
    assert_eq!(response.rounds, 2);
}

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

    let provider = ScriptedProvider::new("provider.mock", script);
    let plugin = ScriptedProviderPlugin::new(provider);
    let mcp_module = Arc::new(McpModule::new()); // Initially empty

    let mut runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_module(Arc::clone(&mcp_module) as Arc<dyn apeireth_runtime::Module>)
        .with_default_model("mock-model")
        .build()
        .await
        .expect("runtime builds cleanly");

    // Initially no tools offered
    assert_eq!(runtime.tool_declarations().len(), 0);

    // Dynamic registration post-build
    let tool = Arc::new(MockMcpTool);
    let tool_id = tool.id().clone();
    mcp_module.register_tool(tool);

    // Tool declarations immediately reflect dynamic tool
    assert_eq!(runtime.tool_declarations().len(), 1);
    assert_eq!(runtime.tool_declarations()[0].name, "mcp_custom_search");

    // Turn 1 executes with the dynamically registered tool
    let req1 = TurnRequest::new(SessionId::new(), "search query").with_model("mock-model");
    let resp1 = runtime.execute(req1).await.expect("turn 1 executes");
    assert_eq!(resp1.text, "Search handled dynamically");

    // Unregister the dynamic tool
    mcp_module.unregister_tool(&tool_id);

    // Tool declarations immediately reflect removal
    assert_eq!(runtime.tool_declarations().len(), 0);

    // Turn 2 fails to find the tool, runtime records error and model recovers cleanly
    let req2 = TurnRequest::new(SessionId::new(), "search query again").with_model("mock-model");
    let resp2 = runtime.execute(req2).await.expect("turn 2 executes");
    assert_eq!(resp2.text, "Recovered from missing tool");
}

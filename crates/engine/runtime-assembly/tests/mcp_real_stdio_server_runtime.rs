//! The runtime half of the external-tool proof against a **real third-party
//! stdio MCP server**.
//!
//! The companion proof in the tools crate drives the bridge itself. This one
//! drives the **runtime**: the four rings as the production assembly wires
//! them, against a server process this repository did not write.
//!
//! * **discovery** — `wire_external_tools` runs a real `tools/list` over a
//!   real child process;
//! * **registration** — the discovered tools land in the **runtime capability
//!   table**, so the model-facing tool catalog really carries
//!   `mcp:<server>:<tool>`;
//! * **authorization** — the production permission policy grants the
//!   external **family** and marks it for a **per-call human approval**
//!   (`authorize_external_tool_family`: family grant + family per-call
//!   approval marking), and the governance chain that reads that policy
//!   really suspends the dispatch;
//! * **dispatch** — the human approves, the turn resumes, and the approved
//!   call runs against the external server; the server's own answer comes
//!   back into the model context.
//!
//! # How the external server is named
//!
//! The command line comes from the environment variable
//! [`SERVER_COMMAND_ENV`] — never from a literal in this file: the runtime
//! talks to whatever external server the operator points it at, and no
//! vendor name is baked into the repository. When the variable is not set
//! the test **skips** (it does not fail).
//!
//! ```text
//! APEIRETH_MCP_REAL_STDIO_SERVER_CMD=/path/to/the/server \
//!   cargo test -p apeireth-runtime-assembly --test mcp_real_stdio_server_runtime -- --nocapture
//! ```
//!
//! # What this test deliberately does not depend on
//!
//! * **No fixed tool list.** The external server's tool set is that
//!   server's business. The test asserts only that *some* tools were
//!   registered and picks its call target by **preference** (first hit wins,
//!   absence is a skip).
//! * **No fixed tool count.** Only a lower bound is asserted.
//! * **No side effects.** The call target is always a tool that only reads
//!   local state.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::{
    GovernanceHook, GovernancePipeline, PermissionGovernanceHook, PermissionPolicy,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse,
    NormalizedUsage, ToolCall,
};
use apeireth_runtime_assembly as apeireth_runtime;
use apeireth_runtime_assembly::canonical::permission_preset::PermissionPresetGovernanceHook;
use apeireth_runtime_assembly::canonical::{
    authorize_external_tool_family, wire_external_tools, ApprovalDecision, ApprovalResolution,
    InMemorySessionStore, McpModule, Runtime, SessionStore, TurnOutcome, TurnRequest,
};
use apeireth_tools_canonical::mcp_bridge::{
    McpBridgeOptions, McpServerConfig, McpServerSpec, McpToolBridge,
};
use async_trait::async_trait;

/// Environment variable naming the external stdio MCP server executable.
const SERVER_COMMAND_ENV: &str = "APEIRETH_MCP_REAL_STDIO_SERVER_CMD";

/// Environment variable naming the external server's arguments (JSON array).
const SERVER_ARGS_ENV: &str = "APEIRETH_MCP_REAL_STDIO_SERVER_ARGS";

/// The namespace the external server is registered under.
const SERVER_NAME: &str = "realstdio";

/// The model the scripted provider speaks for.
const MODEL: &str = "mock-model";
/// Remote tool names preferred as the call target, in order.
///
/// Every name here is a **read-only listing of local state**. The list is a
/// *preference*, not a requirement: a name the server does not advertise is
/// skipped.
const PREFERRED_CALL_TARGETS: &[&str] = &[
    "list_documents",
    "current_document",
    "list_segments",
    "list_bookmarks",
    "list_names",
];

/// Name prefixes of tools that only read state.
///
/// The write-shaped verbs (extract / import / install / push / pull / set /
/// start / stop / capture / recover) are deliberately excluded: the test must
/// never change anything on the machine it runs on.
const READ_ONLY_NAME_PREFIXES: &[&str] = &["list_", "get_", "current_"];

/// The external server's command line, or `None` when the environment names
/// none.
fn real_server_command() -> Option<String> {
    std::env::var(SERVER_COMMAND_ENV)
        .ok()
        .map(|command| command.trim().to_string())
        .filter(|command| !command.is_empty())
}

/// The spec of the external stdio MCP server, or `None` when the environment
/// names none.
fn real_server_spec() -> Option<McpServerSpec> {
    let command = real_server_command()?;
    let mut spec = McpServerSpec::stdio(SERVER_NAME, command);
    if let Ok(raw) = std::env::var(SERVER_ARGS_ENV) {
        if let Ok(arguments) = serde_json::from_str::<Vec<String>>(&raw) {
            for argument in arguments {
                spec = spec.with_arg(argument);
            }
        }
    }
    Some(spec)
}

/// The property names one declared schema marks required.
fn required_properties(schema: &Option<serde_json::Value>) -> Vec<String> {
    schema
        .as_ref()
        .and_then(|schema| schema.get("required"))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The call target: a tool that only reads state and asks for no argument.
///
/// A **preference**, never a requirement: when the external server
/// advertises none of them, the caller skips the dispatch proof instead of
/// failing on a tool set that is not the repository's to control.
fn pick_call_target(bridge: &McpToolBridge) -> Option<String> {
    let entries = bridge.catalog().list();
    for name in PREFERRED_CALL_TARGETS {
        if let Some(entry) = entries.iter().find(|entry| {
            entry.remote_name == *name && required_properties(&entry.input_schema).is_empty()
        }) {
            return Some(entry.model_name.clone());
        }
    }
    let mut fallback: Vec<&String> = entries
        .iter()
        .filter(|entry| {
            READ_ONLY_NAME_PREFIXES
                .iter()
                .any(|prefix| entry.remote_name.starts_with(prefix))
                && required_properties(&entry.input_schema).is_empty()
        })
        .map(|entry| &entry.model_name)
        .collect();
    fallback.sort();
    fallback.first().map(|name| (*name).clone())
}

/// A provider that emits one scripted tool call, then closes the turn, and
/// records every request it was given.
struct ScriptedProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    tool_call: Mutex<Option<ToolCall>>,
    requests: Mutex<Vec<NormalizedRequest>>,
}

impl ScriptedProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.mock").unwrap(),
            calls: AtomicUsize::new(0),
            tool_call: Mutex::new(None),
            requests: Mutex::new(Vec::new()),
        })
    }

    /// The tool call the provider emits on its first round.
    fn set_tool_call(&self, call: ToolCall) {
        *self.tool_call.lock().unwrap() = Some(call);
    }

    /// How many requests the provider was given.
    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// The requests the provider was given.
    fn requests(&self) -> Vec<NormalizedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        self.requests.lock().unwrap().push(request.clone());
        let base = NormalizedResponse {
            id: format!("resp_{index}"),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        if index == 0 {
            let Some(tool_call) = self.tool_call.lock().unwrap().clone() else {
                return Ok(NormalizedResponse {
                    content: "no tool call was scripted".into(),
                    ..base
                });
            };
            return Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![tool_call],
                ..base
            });
        }
        Ok(NormalizedResponse {
            content: "turn complete".into(),
            ..base
        })
    }
}

struct ScriptedProviderPlugin {
    manifest: apeireth_plugin::PluginManifest,
    provider: Arc<ScriptedProvider>,
}

#[async_trait]
impl Plugin for ScriptedProviderPlugin {
    fn manifest(&self) -> &apeireth_plugin::PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

fn scripted_plugin(provider: Arc<ScriptedProvider>) -> Arc<ScriptedProviderPlugin> {
    Arc::new(ScriptedProviderPlugin {
        manifest: PluginManifest::new(
            PluginId::new("vendor.mock").unwrap(),
            "1.0.0",
            "Scripted provider for the external tool acceptance proof",
        )
        .declare_capability(
            provider.id.clone(),
            CapabilityKind::Provider,
            "Scripted completions",
        )
        .unwrap(),
        provider,
    })
}

/// The whole chain against the external server: discovery, registration into
/// the runtime capability table, the per-call human approval and the
/// dispatch of the approved call.
#[tokio::test]
async fn a_real_external_tool_waits_for_a_human_and_then_runs_against_the_real_server() {
    let Some(spec) = real_server_spec() else {
        eprintln!("SKIP: {SERVER_COMMAND_ENV} names no external stdio MCP server");
        return;
    };
    let config = McpServerConfig::new(vec![spec]);
    let bridge = McpToolBridge::new(config, McpBridgeOptions::default())
        .expect("the external server spec must validate");

    // The production permission policy: the external family is granted and
    // marked for a **per-call human approval**.
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    {
        let mut policy = policy.lock().unwrap();
        authorize_external_tool_family(&mut policy);
    }

    // The governance chain: the shared production policy behind the runtime's
    // hook, wrapped in the session permission preset (the production shape —
    // the preset must not exempt the external family from the human).
    let inner: Arc<dyn GovernanceHook> =
        Arc::new(PermissionGovernanceHook::new_shared(Arc::clone(&policy)));
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let governance = Arc::new(GovernancePipeline::new().with(Arc::new(
        PermissionPresetGovernanceHook::new(inner, Arc::clone(&store)),
    )));

    let provider = ScriptedProvider::new();
    let runtime = Runtime::builder()
        .with_governance(governance)
        .with_session_store(Arc::clone(&store))
        .with_plugin(scripted_plugin(Arc::clone(&provider)))
        .with_default_model(MODEL)
        .with_max_rounds(4)
        .build()
        .await
        .expect("runtime builds");

    // **Ring 1 + 2 + 3**: discovery, registration into the runtime capability
    // table, and the family authorization with its per-call human marking.
    let module_bag = McpModule::new();
    let wiring = wire_external_tools(&runtime, &bridge, Some(&module_bag), &policy)
        .await
        .expect("the startup wiring must reach the external server");
    assert!(
        wiring.registered > 0,
        "the external server's tools must be registered into the runtime"
    );
    eprintln!(
        "registered {} dynamic tools into the runtime capability table",
        wiring.registered
    );

    let Some(model_name) = pick_call_target(&bridge) else {
        eprintln!("SKIP: the external server advertises no read-only, argument-free tool to call");
        return;
    };

    // The model really sees the external tool in its tool catalog.
    let declarations: Vec<String> = runtime
        .tool_declarations()
        .into_iter()
        .map(|declaration| declaration.name)
        .collect();
    assert!(
        declarations.contains(&model_name),
        "the model-facing tool catalog must carry {model_name}: {declarations:?}"
    );
    assert!(
        model_name.starts_with("mcp:realstdio:"),
        "the model-facing name must stay namespaced: {model_name}"
    );
    eprintln!(
        "the model-facing tool catalog carries {model_name} ({} declarations in total)",
        declarations.len()
    );

    provider.set_tool_call(ToolCall {
        id: "call_real_runtime_1".into(),
        name: model_name.clone(),
        arguments: serde_json::json!({}),
    });

    // The dispatch is suspended: the policy says "per-call human approval",
    // so the runtime must ask before the external server is contacted.
    let session = SessionId::new();
    let outcome = runtime
        .execute_outcome(TurnRequest::new(session, "please call the external tool"))
        .await
        .expect("the turn executes");
    let TurnOutcome::PendingApproval(view) = outcome else {
        panic!("the external call must wait for a human: {outcome:?}");
    };
    assert!(
        view.capability_id
            .as_str()
            .starts_with("tool.mcp.realstdio."),
        "the pending approval must name the external capability: {}",
        view.capability_id.as_str()
    );
    assert_eq!(view.tool_name, model_name);
    assert_eq!(
        provider.call_count(),
        1,
        "the provider must not be recalled while paused"
    );
    eprintln!(
        "the dispatch was suspended for a human: capability={}, tool={}",
        view.capability_id.as_str(),
        view.tool_name
    );

    // **Ring 4**: the human approves; the approved call runs against the
    // external server and its answer comes back into the model context.
    let resolution = runtime
        .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
        .await
        .expect("the approval resolves");
    let ApprovalResolution::Resumed(TurnOutcome::Completed(response)) = resolution else {
        panic!("the approved turn must resume");
    };
    assert_eq!(response.text, "turn complete");
    assert_eq!(
        provider.call_count(),
        2,
        "the provider is called again after the approval"
    );

    // The external server's answer really reached the model: the second
    // request carries the tool result of the external call.
    let requests = provider.requests();
    let second = requests
        .get(1)
        .expect("the provider is called a second time after the approval");
    let tool_message = second
        .messages
        .iter()
        .find(|message| {
            message.role == MessageRole::Tool
                && message.name.as_deref() == Some(model_name.as_str())
        })
        .unwrap_or_else(|| {
            panic!(
                "the resumed turn must carry the external tool result into the model context: {:?}",
                second
                    .messages
                    .iter()
                    .map(|message| (message.role, message.name.clone()))
                    .collect::<Vec<_>>()
            )
        });
    let answer: String = tool_message
        .content
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !answer.is_empty(),
        "the external server's answer must reach the model context"
    );
    eprintln!(
        "the external server's answer reached the model context ({} chars)",
        answer.chars().count()
    );

    // The approval entity is consumed: the human's decision was recorded.
    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .expect("the session loads")
        .expect("the session is persisted");
    assert_eq!(stored.active_approval_id, None);
    assert_eq!(
        stored
            .approvals
            .get(&view.approval_id)
            .expect("the approval is recorded")
            .status,
        apeireth_runtime::canonical::ApprovalStatus::Consumed
    );
}

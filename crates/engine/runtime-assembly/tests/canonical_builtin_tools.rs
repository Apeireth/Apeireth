//! M2A deterministic end-to-end proof for the canonical builtin tools.
//!
//! The runtime, plugin registry, capability registry, governance, provider
//! router, session store, and agent loop are the real implementations. The
//! provider is scripted and the filesystem is a temporary workspace, so the
//! test is offline and reproducible.

use apeireth_runtime_assembly as apeireth_runtime;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::{AllowAll, DenyCapabilities};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedTool,
    NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{Runtime, TraceEvent, TurnRequest};
use apeireth_tools_canonical::BuiltinToolsPlugin;
use async_trait::async_trait;

const MODEL: &str = "fake-model-1";

#[derive(Clone)]
enum Scripted {
    CallTool {
        call_id: &'static str,
        tool: &'static str,
        arguments: serde_json::Value,
    },
    Say(&'static str),
}

struct FakeProvider {
    id: CapabilityId,
    script: Vec<Scripted>,
    calls: AtomicUsize,
    seen: Mutex<Vec<NormalizedRequest>>,
}

impl FakeProvider {
    fn new(id: &str, script: Vec<Scripted>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            script,
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn request(&self, index: usize) -> NormalizedRequest {
        self.seen.lock().unwrap()[index].clone()
    }
}

#[async_trait]
impl ProviderCapability for FakeProvider {
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
        self.seen.lock().unwrap().push(request.clone());

        let step = self
            .script
            .get(index)
            .unwrap_or_else(|| panic!("{} called beyond script", self.id));

        let base = NormalizedResponse {
            id: format!("resp_{}", index + 1),
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

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<FakeProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<FakeProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("vendor.fake").unwrap(),
                "1.0.0",
                "Fake provider",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Scripted completions",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for ProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
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

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn filesystem_script() -> Vec<Scripted> {
    vec![
        Scripted::CallTool {
            call_id: "call_1",
            tool: "filesystem",
            arguments: serde_json::json!({ "operation": "read", "path": "hello.txt" }),
        },
        Scripted::Say("read complete"),
    ]
}

#[tokio::test]
async fn filesystem_read_reaches_the_tool_through_the_canonical_plugin_path() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("hello.txt"), "hello").unwrap();

    let provider = FakeProvider::new("provider.fake", filesystem_script());
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_plugin(Arc::new(BuiltinToolsPlugin::new(workspace.path())))
        .with_default_model(MODEL)
        .build()
        .await
        .expect("runtime builds");

    let session_id = SessionId::new();
    let outcome = runtime
        .execute(TurnRequest::new(session_id, "read hello.txt"))
        .await
        .expect("the turn completes");

    assert_eq!(provider.call_count(), 2, "provider should be called twice");
    assert_eq!(outcome.rounds, 2);
    assert_eq!(outcome.text, "read complete");
    assert_eq!(outcome.served_by.as_str(), "provider.fake");

    let dispatched = outcome.trace.entries.iter().any(|entry| {
        matches!(
            &entry.event,
            TraceEvent::CapabilityDispatched { capability, .. } if capability.as_str() == "tool.filesystem"
        )
    });
    assert!(
        dispatched,
        "filesystem tool should be dispatched via the registry"
    );

    let completed = outcome.trace.entries.iter().any(|entry| {
        matches!(
            &entry.event,
            TraceEvent::CapabilityCompleted { capability, succeeded: true, .. } if capability.as_str() == "tool.filesystem"
        )
    });
    assert!(completed, "filesystem tool should complete successfully");

    let session = runtime
        .sessions()
        .load_or_create(session_id)
        .await
        .expect("session should persist");
    assert!(
        !session.is_empty(),
        "session should contain the turn events"
    );
}

#[tokio::test]
async fn permission_denial_prevents_tool_dispatch() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("hello.txt"), "hello").unwrap();

    let provider = FakeProvider::new("provider.fake", filesystem_script());
    let denied = DenyCapabilities::new().deny(CapabilityId::new("tool.filesystem").unwrap());
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_governance(Arc::new(denied))
        .with_plugin(ProviderPlugin::new(provider.clone()))
        .with_plugin(Arc::new(BuiltinToolsPlugin::new(workspace.path())))
        .with_default_model(MODEL)
        .build()
        .await
        .expect("runtime builds");

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "read hello.txt"))
        .await
        .expect("the turn completes");

    let dispatched = outcome.trace.entries.iter().any(|entry| {
        matches!(
            &entry.event,
            TraceEvent::CapabilityDispatched { capability, .. } if capability.as_str() == "tool.filesystem"
        )
    });
    assert!(!dispatched, "denied filesystem tool must not be dispatched");

    let denied = outcome.trace.entries.iter().any(|entry| {
        matches!(
            &entry.event,
            TraceEvent::GovernanceEvaluated { decision, .. } if decision == "deny"
        )
    });
    assert!(denied, "denial decision should be preserved in the trace");
}

#[allow(dead_code)]
fn _declaration_smoke() -> (NormalizedTool, ToolParameters) {
    let plugin = BuiltinToolsPlugin::new(".");
    let tools = plugin.tools();
    let declaration = tools[0].declaration();
    let params = declaration.parameters.clone();
    (declaration, params)
}

//! KC-1 architecture regression: the kernel runs with zero business modules and
//! publishes exactly one completion fact per executed tool call.
//!
//! These tests intentionally use no domain crate. They pin the mechanism-only
//! contract: an empty kernel builds, a provider-only kernel completes turns, and
//! a generic completion observer sees each executed call once.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::AllowAll;
use apeireth_plugin::observation::{ToolCompletionFact, ToolCompletionObserver};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedTool,
    NormalizedUsage, ToolCall, ToolParameters, ToolResult,
};
use apeireth_runtime::canonical::{Runtime, SessionEventKind, TraceEvent, TurnRequest};
use async_trait::async_trait;
use serde_json::json;

const MODEL: &str = "boundary-model";

#[derive(Clone)]
enum Step {
    CallTool(ToolCall),
    Say(&'static str),
}

struct ScriptedProvider {
    id: CapabilityId,
    script: Vec<Step>,
    calls: Mutex<usize>,
}

impl ScriptedProvider {
    fn new(script: Vec<Step>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.boundary").expect("provider id"),
            script,
            calls: Mutex::new(0),
        })
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).expect("model id"), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = {
            let mut calls = self.calls.lock().expect("provider calls");
            let index = *calls;
            *calls += 1;
            index
        };
        let step = self
            .script
            .get(index)
            .unwrap_or_else(|| panic!("provider called {} times", index + 1));
        let base = NormalizedResponse {
            id: format!("resp_{}", index + 1),
            model: request.model.clone(),
            content: String::new(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };
        match step {
            Step::CallTool(call) => Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![call.clone()],
                ..base
            }),
            Step::Say(text) => Ok(NormalizedResponse {
                content: (*text).to_string(),
                ..base
            }),
        }
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<ScriptedProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<ScriptedProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("vendor.boundary").expect("plugin id"),
                "1.0.0",
                "Boundary completions",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Boundary completions",
            )
            .expect("declare provider"),
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

struct EchoTool {
    id: CapabilityId,
}

#[async_trait]
impl ToolCapability for EchoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "echo".to_string(),
            description: Some("Echo the input".to_string()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

struct EchoPlugin {
    manifest: PluginManifest,
}

impl EchoPlugin {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("builtin.boundary.echo").expect("plugin id"),
                "1.0.0",
                "Boundary echo",
            )
            .declare_capability(
                CapabilityId::new("tool.echo").expect("capability id"),
                CapabilityKind::Tool,
                "Echo",
            )
            .expect("declare tool"),
        })
    }
}

#[async_trait]
impl Plugin for EchoPlugin {
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
        vec![Arc::new(EchoTool {
            id: CapabilityId::new("tool.echo").expect("capability id"),
        })]
    }
}

#[derive(Default)]
struct CountingObserver {
    seen: AtomicUsize,
    succeeded: Mutex<Vec<bool>>,
}

impl ToolCompletionObserver for CountingObserver {
    fn observe(&self, fact: &ToolCompletionFact<'_>) {
        self.seen.fetch_add(1, Ordering::SeqCst);
        self.succeeded
            .lock()
            .expect("observer results")
            .push(fact.result.is_ok());
    }
}

fn tool_call() -> ToolCall {
    ToolCall {
        id: "call_1".to_string(),
        name: "echo".to_string(),
        arguments: json!({ "value": 1 }),
    }
}

#[tokio::test]
async fn an_empty_kernel_builds_without_business_modules_or_surfaces() {
    let runtime = Runtime::builder().build().await.expect("kernel builds");
    assert!(runtime.modules().is_empty(), "零业务模块");
    assert!(runtime.tool_declarations().is_empty(), "无工具/控制声明");
    assert!(runtime.snapshot().capabilities.is_empty());
}

#[tokio::test]
async fn a_zero_module_kernel_completes_a_text_turn() {
    let provider = ScriptedProvider::new(vec![Step::Say("kernel alive")]);
    let runtime = Runtime::builder()
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider))
        .with_default_model(MODEL)
        .build()
        .await
        .expect("kernel builds");

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "ping"))
        .await
        .expect("zero-module turn completes");
    assert_eq!(outcome.text, "kernel alive");
    assert!(runtime.modules().is_empty());
}

#[tokio::test]
async fn one_completion_fact_is_emitted_per_executed_call() {
    let provider = ScriptedProvider::new(vec![Step::CallTool(tool_call()), Step::Say("done")]);
    let observer = Arc::new(CountingObserver::default());
    let runtime = Runtime::builder()
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider))
        .with_plugin(EchoPlugin::new())
        .with_default_model(MODEL)
        .with_tool_completion_observer(observer.clone())
        .build()
        .await
        .expect("kernel builds");

    let session = SessionId::new();
    let outcome = runtime
        .execute(TurnRequest::new(session, "use echo"))
        .await
        .expect("tool turn completes");

    assert_eq!(
        observer.seen.load(Ordering::SeqCst),
        1,
        "每次执行恰一次完成事实"
    );
    assert_eq!(
        *observer.succeeded.lock().expect("observer results"),
        vec![true]
    );
    let completed = outcome
        .trace
        .events()
        .filter(|event| matches!(event, TraceEvent::CapabilityCompleted { .. }))
        .count();
    assert_eq!(completed, 1, "轨迹仍恰好一条完成记录");
    let persisted = runtime
        .sessions()
        .load_or_create(session)
        .await
        .expect("session persists");
    assert!(
        !persisted
            .events
            .iter()
            .any(|event| matches!(event.event, SessionEventKind::ToolFailed { .. })),
        "成功调用不得伪造会话失败事件"
    );
}

#[tokio::test]
async fn installing_an_observer_does_not_change_the_tool_catalog() {
    let build = |observer: Option<Arc<CountingObserver>>| {
        let provider = ScriptedProvider::new(vec![Step::Say("ok")]);
        let mut builder = Runtime::builder()
            .with_governance(Arc::new(AllowAll))
            .with_plugin(ProviderPlugin::new(provider))
            .with_plugin(EchoPlugin::new())
            .with_default_model(MODEL);
        if let Some(observer) = observer {
            builder = builder.with_tool_completion_observer(observer);
        }
        builder
    };

    let plain = build(None).build().await.expect("kernel builds");
    let observed = build(Some(Arc::new(CountingObserver::default())))
        .build()
        .await
        .expect("kernel builds");

    let names = |runtime: &Runtime| {
        runtime
            .tool_declarations()
            .into_iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(&plain),
        names(&observed),
        "观察者不得改变可用工具清单"
    );
}

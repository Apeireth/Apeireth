use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_governance::AllowAll;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::{
    adapters::{
        AnthropicMessagesAdapter, GeminiAdapter, OpenAiChatAdapter, OpenAiResponsesAdapter,
    },
    canonical::{
        validate_closed_tool_context, ContentPart, ModelDescriptor, ModelFeature,
        NormalizedRequest, NormalizedResponse, ToolCall,
    },
    ProtocolAdapter,
};
use apeireth_runtime::{Runtime, RuntimeBuilder, SessionEventKind};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
pub fn pid(id: &str) -> PluginId {
    PluginId::new(id).unwrap()
}
pub fn call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_fixture".into(),
        name: name.into(),
        arguments,
    }
}
pub struct Provider {
    id: CapabilityId,
    script: Mutex<VecDeque<Vec<ToolCall>>>,
    requests: Mutex<Vec<NormalizedRequest>>,
}
impl Provider {
    pub fn new(script: Vec<Vec<ToolCall>>) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.capability_fixture").unwrap(),
            script: Mutex::new(script.into()),
            requests: Mutex::new(vec![]),
        })
    }
    pub fn results(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .flat_map(|request| request.messages.iter())
            .filter(|message| message.role == apeireth_protocol::canonical::MessageRole::Tool)
            .map(|message| ContentPart::join_text(&message.content))
            .collect()
    }
}
#[async_trait]
impl ProviderCapability for Provider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new("fixture").unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        validate_closed_tool_context(&request.messages)
            .expect("real Runtime history must be closed");
        let mut wire = request.clone();
        wire.max_tokens = Some(128);
        for adapter in [
            Box::new(OpenAiChatAdapter::new()) as Box<dyn ProtocolAdapter>,
            Box::new(OpenAiResponsesAdapter::new()),
            Box::new(AnthropicMessagesAdapter::new()),
            Box::new(GeminiAdapter::new()),
        ] {
            adapter
                .adapt_request(&wire)
                .expect("actual Runtime messages must project legally");
        }
        self.requests.lock().unwrap().push(request.clone());
        let mut response = NormalizedResponse::text("fixture", &request.model, "done");
        response.tool_calls = self.script.lock().unwrap().pop_front().unwrap_or_default();
        response.usage = apeireth_protocol::canonical::NormalizedUsage::new(10, 5)
            .with_cache_hit_tokens(Some(3));
        Ok(response)
    }
}
struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<Provider>,
}
#[async_trait]
impl Plugin for ProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![self.provider.clone()]
    }
}
pub fn builder(provider: Arc<Provider>) -> RuntimeBuilder {
    let manifest = PluginManifest::new(pid("provider.fixture"), "1.0.0", "fixture")
        .declare_capability(provider.id.clone(), CapabilityKind::Provider, "fixture")
        .unwrap();
    Runtime::builder()
        .with_default_model("fixture")
        .with_governance(Arc::new(AllowAll))
        .with_plugin(Arc::new(ProviderPlugin { manifest, provider }))
}
pub async fn facts(
    runtime: &Runtime,
    session: SessionId,
) -> Vec<apeireth_protocol::canonical::ToolEventV1> {
    runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .unwrap()
        .events
        .iter()
        .filter_map(|entry| match &entry.event {
            SessionEventKind::ToolEventRecorded { fact } => Some(fact.clone()),
            _ => None,
        })
        .collect()
}

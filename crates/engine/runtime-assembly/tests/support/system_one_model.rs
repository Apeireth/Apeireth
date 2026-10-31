use apeireth_core::kernel::{CapabilityId, ModelId, PluginId};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse,
    NormalizedUsage, ToolCall,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
pub enum ModelMode {
    Valid,
    ToolCall,
    Malformed,
    Unavailable,
}
pub struct ReviewProvider {
    id: CapabilityId,
    pub mode: ModelMode,
    requests: Mutex<Vec<NormalizedRequest>>,
}
impl ReviewProvider {
    pub fn new(id: &str, mode: ModelMode) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new(id).unwrap(),
            mode,
            requests: Mutex::new(vec![]),
        })
    }
    pub fn requests(&self) -> Vec<NormalizedRequest> {
        self.requests.lock().unwrap().clone()
    }
    pub fn module(self: &Arc<Self>) -> Arc<dyn Plugin> {
        let manifest = PluginManifest::new(
            PluginId::new(format!("module.{}", self.id)).unwrap(),
            "1.0.0",
            "Scripted supervisor provider fixture",
        )
        .declare_capability(
            self.id.clone(),
            CapabilityKind::Provider,
            "Review model fixture",
        )
        .unwrap();
        Arc::new(ProviderModule {
            manifest,
            provider: self.clone(),
        })
    }
}
#[async_trait]
impl ProviderCapability for ReviewProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new("review-model").unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        assert!(request.tools.is_empty(), "supervisor must be tool-free");
        self.requests.lock().unwrap().push(request.clone());
        if matches!(self.mode, ModelMode::Unavailable) {
            return Err(ProviderError::Network {
                provider: self.id.to_string(),
                detail: "fixture unavailable".into(),
            });
        }
        let text = request
            .messages
            .iter()
            .rev()
            .find_map(|message| {
                let text = ContentPart::join_text(&message.content);
                serde_json::from_str::<serde_json::Value>(&text)
                    .ok()
                    .filter(|value| value.get("review_id").is_some())
            })
            .expect("trusted runtime review request");
        let content = if matches!(self.mode, ModelMode::Malformed) {
            "{\"decision\":\"allow\"}".to_owned()
        } else {
            json!({"version":text["version"],"policy_version":text["policy_version"],"review_id":text["review_id"],"action_arguments_digest":text["action_arguments_digest"],"decision":"allow"}).to_string()
        };
        let mut response = NormalizedResponse::text("review-fixture", &request.model, content);
        response.usage = NormalizedUsage::new(1, 1).with_cache_hit_tokens(Some(0));
        if matches!(self.mode, ModelMode::ToolCall) {
            response.tool_calls.push(ToolCall {
                id: "attempted_review_effect".into(),
                name: "protected_write".into(),
                arguments: json!({"value":"unexpected-effect"}),
            });
        }
        Ok(response)
    }
}
struct ProviderModule {
    manifest: PluginManifest,
    provider: Arc<ReviewProvider>,
}
#[async_trait]
impl Plugin for ProviderModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![self.provider.clone()]
    }
    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
}

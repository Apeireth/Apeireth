//! Tests proving the minimal microkernel core.
//!
//! A minimal runtime boots with zero modules (no cognitive spine, no builtin tools),
//! requiring only a provider to complete pure user chat turns.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, NormalizedFinishReason, NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::{Runtime, TurnRequest};
use async_trait::async_trait;

struct PureChatProvider {
    id: CapabilityId,
    calls: AtomicUsize,
}

impl PureChatProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.pure_chat").unwrap(),
            calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl ProviderCapability for PureChatProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new("pure-chat-model").unwrap(),
            self.id.clone(),
        )]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(NormalizedResponse {
            id: "chat_resp_1".into(),
            model: request.model.clone(),
            content: "Hello from pure microkernel!".into(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct PureChatPlugin {
    manifest: PluginManifest,
    provider: Arc<PureChatProvider>,
}

impl PureChatPlugin {
    fn new(provider: Arc<PureChatProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("vendor.pure").unwrap(),
                "1.0.0",
                "Pure chat plugin",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Pure chat provider",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait]
impl Plugin for PureChatPlugin {
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

#[tokio::test]
async fn minimal_kernel_without_standard_modules_completes_plain_chat_turn() {
    let provider = PureChatProvider::new();
    let plugin = PureChatPlugin::new(Arc::clone(&provider));

    // Build the microkernel runtime with ZERO modules
    let mut runtime = Runtime::builder()
        .with_plugin(plugin)
        .with_default_model("pure-chat-model")
        .build()
        .await
        .expect("minimal kernel runtime builds cleanly without any modules");

    assert_eq!(
        runtime.modules().len(),
        0,
        "no modules should be registered"
    );
    assert_eq!(runtime.tools().len(), 0, "no tools should be registered");
    assert_eq!(
        runtime.tool_declarations().len(),
        0,
        "no tool declarations should be offered"
    );

    let session_id = SessionId::new();
    let req =
        TurnRequest::new(session_id, "Hello Apeireth Microkernel").with_model("pure-chat-model");

    let response = runtime.execute(req).await.expect("turn completes cleanly");
    assert_eq!(response.text, "Hello from pure microkernel!");
    assert_eq!(response.rounds, 1);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);

    // Verify session persistence
    let session = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("session load succeeds")
        .expect("session exists");

    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.revision, 4);
}

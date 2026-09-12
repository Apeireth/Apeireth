//! Minimal durable episodic memory -> canonical runtime -> fake provider E2E.
//!
//! This intentionally covers the production episodic/provider chain only. Semantic,
//! relational, and typed projections are out of scope for this small vertical test.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId};
use apeireth_memory::{
    MemoryGovernanceStore, ScopedMemoryBackend, SqliteMemoryStore, TypedRecallIdentity,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedUsage,
};
use apeireth_runtime::canonical::{Runtime, TurnRequest};
use apeireth_runtime_assembly::{
    CognitiveBackends, CognitiveModuleConfig, ProductionCognitiveModules,
    SqliteTypedMemoryRecallSource,
};
use async_trait::async_trait;
use tempfile::tempdir;

const MODEL: &str = "memory-provider-e2e-model";

struct RecordingProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    requests: Mutex<Vec<NormalizedRequest>>,
}

impl RecordingProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.memory-e2e").unwrap(),
            calls: AtomicUsize::new(0),
            requests: Mutex::new(Vec::new()),
        })
    }

    fn requests(&self) -> Vec<NormalizedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProviderCapability for RecordingProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new(MODEL).unwrap(),
            self.id.clone(),
        )]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        self.requests.lock().unwrap().push(request.clone());
        Ok(NormalizedResponse {
            id: format!("memory-provider-response-{call}"),
            model: request.model.clone(),
            content: "The durable answer was produced by the fake provider.".into(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<RecordingProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<RecordingProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("plugin.memory-provider-e2e").unwrap(),
                "1.0.0",
                "fake provider for durable memory E2E",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "fake provider",
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

fn memory_config() -> CognitiveModuleConfig {
    CognitiveModuleConfig {
        memory_recall: true,
        memory_writeback: true,
        preference_recall: false,
        self_assessment: false,
        filesystem: false,
        search: false,
        repo: false,
        ..CognitiveModuleConfig::default()
    }
}

fn request_text(request: &NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .flat_map(|message| message.content.iter())
        .map(|part| match part {
            ContentPart::Text { text } => text.as_str(),
            _ => "",
        })
        .collect()
}

fn memory_ids_in(text: &str) -> std::collections::BTreeSet<String> {
    text.split("[mem:")
        .skip(1)
        .filter_map(|part| part.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

async fn build_runtime(db_path: &std::path::Path, provider: Arc<RecordingProvider>) -> Runtime {
    let store = Arc::new(SqliteMemoryStore::open(db_path).unwrap());
    let backend = store.clone() as Arc<dyn apeireth_plugin::memory_backend::MemoryBackend>;
    let governance = store.clone() as Arc<dyn MemoryGovernanceStore>;
    let scoped = store.clone() as Arc<dyn ScopedMemoryBackend>;
    let typed_source = Arc::new(SqliteTypedMemoryRecallSource::new().with_episodes(store.clone()));
    let modules = ProductionCognitiveModules::build(
        memory_config(),
        CognitiveBackends {
            memory: Some(backend),
            memory_governance: Some(governance),
            scoped_memory: Some(scoped),
            typed_recall: Some(typed_source),
            typed_recall_identity: Some(TypedRecallIdentity {
                persona_id: "persona-e2e".into(),
                subject_id: "user-e2e".into(),
            }),
            ..CognitiveBackends::default()
        },
        apeireth_core::kernel::system_clock(),
    )
    .unwrap();

    modules
        .register_into(
            Runtime::builder()
                .with_plugin(ProviderPlugin::new(provider))
                .with_default_model(MODEL),
        )
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn file_backed_sqlite_runtime_writeback_then_restart_recall_overlays_provider_request() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory-provider-e2e.sqlite3");
    let session = SessionId::new();

    let first_provider = RecordingProvider::new();
    let first_runtime = build_runtime(&db_path, first_provider.clone()).await;
    let first_response = first_runtime
        .execute(TurnRequest::new(
            session,
            "I prefer concise technical explanations. Ada is my colleague. I will submit the report Friday. I live in Wuhan.",
        ))
        .await
        .unwrap();
    assert_eq!(
        first_response.text,
        "The durable answer was produced by the fake provider."
    );

    // The canonical loop has committed the turn before AfterTurn writeback runs.
    let first_store = SqliteMemoryStore::open(&db_path).unwrap();
    let persisted = first_store
        .governed_recent_episodes(&session.to_string(), 10)
        .unwrap();
    assert!(persisted.iter().any(|episode| {
        episode.episode.role == "user"
            && episode
                .episode
                .content
                .contains("concise technical explanations")
            && episode.episode.content.contains("Ada")
            && episode.episode.content.contains("Wuhan")
    }));
    assert!(persisted.iter().any(|episode| {
        episode.episode.role == "assistant"
            && episode
                .episode
                .content
                .contains("durable answer was produced")
    }));

    drop(first_runtime);
    drop(first_store);

    // Rebuild both assembly and runtime against the same file. TurnStart recall
    // must be visible only as a governed transient overlay to the provider.
    let second_provider = RecordingProvider::new();
    let second_runtime = build_runtime(&db_path, second_provider.clone()).await;
    second_runtime
        .execute(TurnRequest::new(
            session,
            "What do you remember about Ada, my report, where I live, and how I prefer answers?",
        ))
        .await
        .unwrap();

    let requests = second_provider.requests();
    assert_eq!(requests.len(), 1);
    let provider_text = request_text(&requests[0]);
    assert!(
        provider_text.contains("<governed_memory"),
        "{provider_text}"
    );
    assert!(
        provider_text.contains("concise technical explanations"),
        "preference recall overlay missing: {provider_text}"
    );
    assert!(
        provider_text.contains("Ada"),
        "relation text missing: {provider_text}"
    );
    assert!(
        provider_text.contains("Wuhan"),
        "location fact missing: {provider_text}"
    );
    let provider_ids = memory_ids_in(&provider_text);
    assert!(
        !provider_ids.is_empty(),
        "provider received no memory IDs: {provider_text}"
    );
    assert!(provider_ids
        .iter()
        .all(|id| provider_text.contains(&format!("[mem:{id}"))));
    let selected_ids = provider_ids.clone();
    let retrieved_ids = selected_ids.clone();
    assert!(provider_ids.is_subset(&selected_ids));
    assert!(selected_ids.is_subset(&retrieved_ids));
}

//! Minimal durable episodic memory -> canonical runtime -> fake provider E2E.
//!
//! This intentionally covers the production episodic/provider chain only. Semantic,
//! relational, and typed projections are out of scope for this small vertical test.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, Episode, ModelId, PluginId, SessionId};
use apeireth_memory::{
    EpisodeStore, MemoryCoordinator, MemoryGovernanceStore, MemoryProvenance, MemoryScope,
    ScopedMemoryBackend, SqliteMemoryStore, TypedRecallIdentity,
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

fn governed_memory_text(request: &NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .filter(|message| message.role == apeireth_protocol::canonical::MessageRole::Tool)
        .map(|message| ContentPart::join_text(&message.content))
        .filter(|text| text.contains("<governed_memory"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_durable_sources(
    store: &SqliteMemoryStore,
    ids: &std::collections::BTreeSet<String>,
    source_session: &str,
) {
    for id in ids {
        let episode_id = id.strip_prefix("typed:episode:").unwrap_or(id);
        let governed = store.get_governed(episode_id).unwrap().unwrap();
        assert_eq!(governed.status.as_str(), "active");
        assert_eq!(governed.episode.session_id, source_session);
        let metadata =
            apeireth_plugin::memory_backend::MemoryBackend::get_episode_metadata(store, episode_id)
                .unwrap()
                .unwrap();
        assert_eq!(
            metadata["scope"],
            serde_json::to_value(MemoryScope::session(source_session)).unwrap()
        );
        assert_eq!(metadata["provenance"]["source_session"], source_session);
        assert!(metadata["provenance"]["source_request"]
            .as_str()
            .is_some_and(|request| !request.is_empty()));
    }
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

    // Selected Memory callbacks now require explicit Host capability grants.
    let mut policy = apeireth_governance::PermissionPolicy::new();
    for id in [
        "tool.behavior.cognitive.memory_recall",
        "tool.behavior.cognitive.memory_writeback",
    ] {
        policy.grant(apeireth_governance::Permission::ExecuteTool(id.into()));
    }
    modules
        .register_into(
            Runtime::builder()
                .with_governance(Arc::new(
                    apeireth_governance::PermissionGovernanceHook::new(policy),
                ))
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
    // must be visible as a governed, paired module tool result to the provider.
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
    apeireth_protocol::canonical::validate_closed_tool_context(&requests[0].messages).unwrap();
    assert!(requests[0].messages.iter().any(|m| m.role
        == apeireth_protocol::canonical::MessageRole::Tool
        && ContentPart::join_text(&m.content).contains("<governed_memory")));
    assert!(!requests[0].messages.iter().any(|m| m.role
        == apeireth_protocol::canonical::MessageRole::System
        && ContentPart::join_text(&m.content).contains("<governed_memory")));

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
    let reopened_store = SqliteMemoryStore::open(&db_path).unwrap();
    assert_durable_sources(&reopened_store, &provider_ids, &session.to_string());
}

/// Explicit governed correction is tested here. Automatic interpretation of a
/// later sentence as a replacement fact is not provided by this fixture.
#[tokio::test]
async fn chinese_governed_correction_restart_and_forget_reach_the_provider() {
    const ORIGINAL: &str = "我住在武汉。晨星项目的联络城市是武汉。";
    const CORRECTED: &str = "我住在上海。晨星项目的联络城市是上海。";
    const FOREIGN: &str = "晨星项目的联络城市是深圳，FOREIGNSCOPE9。";

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("chinese-memory-lifecycle.sqlite3");
    let session = SessionId::new();
    let session_text = session.to_string();
    let first_provider = RecordingProvider::new();
    let first_runtime = build_runtime(&db_path, first_provider.clone()).await;
    first_runtime
        .execute(TurnRequest::new(session, ORIGINAL))
        .await
        .unwrap();

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let source_rows = EpisodeStore::recent_episodes(store.as_ref(), &session_text, 20).unwrap();
    let asserted_rows = source_rows
        .iter()
        .filter(|episode| episode.content == ORIGINAL)
        .collect::<Vec<_>>();
    assert!(asserted_rows.iter().any(|episode| episode.role == "user"));
    assert!(asserted_rows.iter().any(|episode| episode.role == "memory"));
    let revised_ids = asserted_rows
        .iter()
        .map(|episode| episode.id.clone())
        .collect::<Vec<_>>();
    for id in &revised_ids {
        let before = store.get_governed(id).unwrap().unwrap();
        let edited = store
            .update_episode_content(id, CORRECTED, Some("owner"), before.revision)
            .unwrap();
        assert_eq!(edited.content_override.as_deref(), Some(CORRECTED));
        assert_eq!(
            EpisodeStore::get_episode(store.as_ref(), id)
                .unwrap()
                .unwrap()
                .content,
            ORIGINAL
        );
    }

    let coordinator = MemoryCoordinator::new(store.clone(), store.clone());
    coordinator
        .writeback_scoped_episode(
            &Episode {
                id: "foreign-chinese-fact".into(),
                timestamp: asserted_rows[0].timestamp,
                role: "user".into(),
                content: FOREIGN.into(),
                session_id: "foreign-session".into(),
            },
            &MemoryScope::User {
                user_id: "foreign-user".into(),
            },
            &MemoryProvenance {
                source: "scope-check".into(),
                source_session: Some("foreign-session".into()),
                ..MemoryProvenance::default()
            },
        )
        .unwrap();
    drop(coordinator);

    first_runtime
        .execute(TurnRequest::new(session, "晨星项目现在的联络城市是什么？"))
        .await
        .unwrap();
    let requests = first_provider.requests();
    assert!(requests
        .iter()
        .all(|request| !request_text(request).contains("FOREIGNSCOPE9")));
    let current_request = requests.last().unwrap();
    apeireth_protocol::canonical::validate_closed_tool_context(&current_request.messages).unwrap();
    let current_memory = governed_memory_text(current_request);
    assert!(
        current_memory.contains(CORRECTED),
        "corrected evidence missing: {current_memory}"
    );
    assert!(
        !current_memory.contains("武汉"),
        "stale evidence reached provider: {current_memory}"
    );
    assert!(!current_memory.contains("FOREIGNSCOPE9"));
    drop(first_runtime);
    drop(store);

    let second_provider = RecordingProvider::new();
    let second_runtime = build_runtime(&db_path, second_provider.clone()).await;
    assert!(second_runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .is_none());
    second_runtime
        .execute(TurnRequest::new(
            session,
            "请查记忆：晨星项目的联络城市是什么？",
        ))
        .await
        .unwrap();
    let requests = second_provider.requests();
    assert_eq!(requests.len(), 1);
    apeireth_protocol::canonical::validate_closed_tool_context(&requests[0].messages).unwrap();
    // No old transcript was loaded into this Runtime. Check every outgoing
    // message, so a misplaced overlay cannot evade the memory-block checks.
    let restarted_text = request_text(&requests[0]);
    assert!(!restarted_text.contains("武汉"));
    assert!(!restarted_text.contains("FOREIGNSCOPE9"));
    let restarted_memory = governed_memory_text(&requests[0]);
    assert!(
        restarted_memory.contains(CORRECTED),
        "restart lost correction: {restarted_memory}"
    );
    assert!(!restarted_memory.contains("武汉"));
    assert!(!restarted_memory.contains("FOREIGNSCOPE9"));
    let delivered = memory_ids_in(&restarted_memory);
    assert!(!delivered.is_empty());
    assert!(delivered.iter().any(
        |id| revised_ids.contains(&id.strip_prefix("typed:episode:").unwrap_or(id).to_string())
    ));
    let store = SqliteMemoryStore::open(&db_path).unwrap();
    assert_durable_sources(&store, &delivered, &session_text);
    for id in &revised_ids {
        let current = store.get_governed(id).unwrap().unwrap();
        store
            .forget_episode(id, Some("owner requested hiding"), current.revision)
            .unwrap();
    }
    drop(second_runtime);
    drop(store);

    let third_provider = RecordingProvider::new();
    let third_runtime = build_runtime(&db_path, third_provider.clone()).await;
    assert!(third_runtime
        .sessions()
        .load(&session)
        .await
        .unwrap()
        .is_none());
    third_runtime
        .execute(TurnRequest::new(session, "晨星项目的联络城市是什么？"))
        .await
        .unwrap();
    let requests = third_provider.requests();
    assert_eq!(requests.len(), 1);
    apeireth_protocol::canonical::validate_closed_tool_context(&requests[0].messages).unwrap();
    let forgotten_text = request_text(&requests[0]);
    for excluded in ["上海", "武汉", "FOREIGNSCOPE9"] {
        assert!(
            !forgotten_text.contains(excluded),
            "hidden evidence reached an outgoing message: {excluded}"
        );
    }
    let forgotten_memory = governed_memory_text(&requests[0]);
    assert!(
        !forgotten_memory.contains("上海"),
        "forgotten evidence reached provider: {forgotten_memory}"
    );
    assert!(!forgotten_memory.contains("武汉"));
    assert!(!forgotten_memory.contains("FOREIGNSCOPE9"));
    let remaining_ids = memory_ids_in(&forgotten_memory);
    for id in revised_ids {
        assert!(!remaining_ids.contains(&id));
        assert!(!remaining_ids.contains(&format!("typed:episode:{id}")));
    }
}

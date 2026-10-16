//! Cognitive Infrastructure vNext Vertical Convergence Test Suite (T1..=T11).
//!
//! Covers end-to-end production wiring:
//! - T1: CLI event sink additive fan-out verification (terminal sink + recorder sink).
//! - T2: Guard dataset privacy & safe taxonomy verification (no secret leakage, controlled outcome enum).
//! - T3: Cross-session global scope query verification.
//! - T4: Project scope isolation verification.
//! - T5: Persona scope isolation verification.
//! - T6: Legacy episode scope fail-narrow verification (NULL metadata only visible to source session).
//! - T7: MemoryCoordinator truthful hybrid recall pipeline execution (BM25 Okapi candidate scoring).
//! - T8: Truthful fallback when EmbeddingProvider is missing (used_lexical_fallback == true, semantic == 0.0).
//! - T9: Semantic retrieval path with deterministic fake embedding provider.
//! - T10: Memory forget dynamic invalidation from hybrid pipeline & ring buffer.
//! - T11: Content update updates content_hash and invalidates vector cache.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, RequestId, SessionId, TraceId};
use apeireth_governance::GovernancePipeline;
use apeireth_guard::{BehaviorChainGuardHook, DatasetRecorder};
use apeireth_memory::{
    backend::sqlite::SqliteBackend, EmbeddingError, EmbeddingProvider, MemoryCoordinator,
    MemoryGovernanceStore, MemoryRecallQuery, MemoryScope, MemoryWritebackEntry,
    ScopedMemoryBackend, SqliteMemoryStore,
};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse, NormalizedTool,
    NormalizedUsage, ToolCall, ToolResult,
};
use apeireth_runtime::canonical::{Runtime, RuntimeEvent, RuntimeEventSink, TurnRequest};
use apeireth_runtime_assembly::canonical::guard_observer::GuardDatasetObserver;
use apeireth_storage::{SqliteConnectionPool, StorageError};
use async_trait::async_trait;
use tempfile::tempdir;

const TEST_MODEL: &str = "cognitive-vnext-vertical-model";

// ---------------------------------------------------------------------------
// Test Support Fixtures
// ---------------------------------------------------------------------------

struct ScriptedProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    fail_with_secret: AtomicBool,
}

impl ScriptedProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.vertical-test").unwrap(),
            calls: AtomicUsize::new(0),
            fail_with_secret: AtomicBool::new(false),
        })
    }

    fn with_failure(self: Arc<Self>, fail: bool) -> Arc<Self> {
        self.fail_with_secret.store(fail, Ordering::SeqCst);
        self
    }
}

#[async_trait]
impl ProviderCapability for ScriptedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(TEST_MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_with_secret.load(Ordering::SeqCst) {
            return Err(ProviderError::AuthFailed {
                provider: "test-provider".into(),
                detail: "Bearer secret-token-xyz123 path=/secret/keys/admin.pem".into(),
            });
        }

        let base = NormalizedResponse {
            id: format!("response-{index}"),
            model: request.model.clone(),
            content: "ok".to_string(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        };

        if index == 0
            && request.messages.iter().any(|m| {
                m.content.iter().any(|c| match c {
                    apeireth_protocol::canonical::ContentPart::Text { text } => {
                        text.contains("echo")
                    }
                    _ => false,
                })
            })
        {
            Ok(NormalizedResponse {
                finish_reason: Some(
                    apeireth_protocol::canonical::NormalizedFinishReason::ToolCalls,
                ),
                tool_calls: vec![ToolCall {
                    id: "call-vnext-1".into(),
                    name: "echo".into(),
                    arguments: serde_json::json!({"value": "pong"}),
                }],
                ..base
            })
        } else {
            Ok(base)
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
                PluginId::new("plugin.vertical-test").unwrap(),
                "1.0.0",
                "Cognitive vNext vertical test provider",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Scripted provider",
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

struct EchoTool {
    id: CapabilityId,
}

#[async_trait]
impl ToolCapability for EchoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("echo")
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(call.id.clone(), serde_json::json!({"echoed": true})).with_name("echo")
    }
}

#[derive(Default)]
struct RecordingEventSink {
    events: Arc<Mutex<Vec<String>>>,
}

impl RuntimeEventSink for RecordingEventSink {
    fn emit(&self, event: RuntimeEvent) {
        let tag = match event {
            RuntimeEvent::TurnStarted { .. } => "TurnStarted",
            RuntimeEvent::TurnCompleted { .. } => "TurnCompleted",
            RuntimeEvent::TurnFailed { .. } => "TurnFailed",
            RuntimeEvent::Trace { .. } => "Trace",
            RuntimeEvent::ApprovalRequired { .. } => "ApprovalRequired",
        };
        self.events.lock().unwrap().push(tag.to_string());
    }
}

struct DeterministicFakeEmbeddingProvider;

async fn production_sqlite_backend() -> Arc<SqliteBackend> {
    let pool = Arc::new(SqliteConnectionPool::in_memory().await.unwrap());
    let migration_pool = Arc::clone(&pool);
    migration_pool
        .write(|conn| {
            apeireth_memory::run_migrations(conn).map_err(|error| StorageError::Migration {
                version: 0,
                name: "cognitive_scope_vertical",
                message: error.to_string(),
            })
        })
        .await
        .unwrap();
    Arc::new(SqliteBackend::from_arc(pool))
}

#[async_trait]
impl EmbeddingProvider for DeterministicFakeEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let lower = text.to_lowercase();
        let mut vec = vec![0.0f32; 4];
        if lower.contains("microkernel") || lower.contains("kernel") {
            vec[0] = 1.0;
        }
        if lower.contains("memory") || lower.contains("store") {
            vec[1] = 1.0;
        }
        if lower.contains("guard") || lower.contains("security") {
            vec[2] = 1.0;
        }
        if lower.contains("runtime") || lower.contains("canonical") {
            vec[3] = 1.0;
        }
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vec {
                *x /= norm;
            }
        }
        Ok(vec)
    }

    fn model_id(&self) -> &str {
        "fake-deterministic-embed-v1"
    }
}

// ---------------------------------------------------------------------------
// T1: CLI Event Sink Fan-out
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t1_cli_event_sink_fanout() {
    let sink_cli = Arc::new(RecordingEventSink::default());
    let sink_recorder = Arc::new(RecordingEventSink::default());

    let provider = ScriptedProvider::new();
    let runtime = Runtime::builder()
        .with_plugin(ProviderPlugin::new(provider))
        .with_default_model(TEST_MODEL)
        .build()
        .await
        .unwrap();

    // Add multiple event sinks via add_event_sink (CLI startup pattern)
    runtime.add_event_sink(sink_cli.clone());
    runtime.add_event_sink(sink_recorder.clone());

    let turn_res = runtime
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await;
    assert!(turn_res.is_ok(), "turn execution should succeed");

    let cli_events = sink_cli.events.lock().unwrap().clone();
    let rec_events = sink_recorder.events.lock().unwrap().clone();

    assert!(
        !cli_events.is_empty(),
        "CLI terminal sink must observe events"
    );
    assert!(!rec_events.is_empty(), "recorder sink must observe events");
    assert_eq!(
        cli_events, rec_events,
        "both sinks must receive identical events without overwrite"
    );
    assert!(
        cli_events.contains(&"TurnStarted".to_string()),
        "must contain TurnStarted event"
    );
    assert!(
        cli_events.contains(&"TurnCompleted".to_string()),
        "must contain TurnCompleted event"
    );
}

// ---------------------------------------------------------------------------
// T2: Dataset Privacy & Safe Taxonomy Protection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t2_dataset_privacy_safe_taxonomy() {
    let dir = tempdir().unwrap();
    let jsonl_path = dir.path().join("guard.jsonl");

    let recorder = Arc::new(DatasetRecorder::new(&jsonl_path));
    recorder.set_enabled(true);

    let observer = Arc::new(GuardDatasetObserver::new(recorder.clone()));
    let guard_hook =
        Arc::new(BehaviorChainGuardHook::new().with_dataset_recorder(recorder.clone()));

    let provider = ScriptedProvider::new().with_failure(true);
    let runtime = Runtime::builder()
        .with_governance(Arc::new(GovernancePipeline::new().with(guard_hook)))
        .with_plugin(ProviderPlugin::new(provider))
        .with_default_model(TEST_MODEL)
        .build()
        .await
        .unwrap();

    runtime.add_event_sink(observer.clone());

    // Execute turn that fails with sensitive upstream error
    let turn_res = runtime
        .execute(TurnRequest::new(SessionId::new(), "trigger secret error"))
        .await;
    assert!(turn_res.is_err(), "turn must fail from upstream provider");

    // Inspect guard dataset jsonl directly from disk
    let raw_jsonl = std::fs::read_to_string(&jsonl_path).unwrap_or_default();
    assert!(
        !raw_jsonl.is_empty(),
        "guard dataset must record the turn outcome"
    );

    // Invariant: Sensitive raw error strings must NEVER appear in guard dataset
    assert!(
        !raw_jsonl.contains("Bearer"),
        "guard dataset must never contain Bearer token prefix"
    );
    assert!(
        !raw_jsonl.contains("secret-token-xyz123"),
        "guard dataset must never contain token secret"
    );
    assert!(
        !raw_jsonl.contains("/secret/keys/admin.pem"),
        "guard dataset must never contain sensitive filesystem paths"
    );

    // Verify controlled taxonomy was used
    let samples = recorder.load_supervised_samples();
    let failure_sample = samples
        .iter()
        .find(|s| s.execution_outcome.is_some())
        .expect("must contain sample with execution_outcome");
    assert_eq!(
        failure_sample.execution_outcome.as_deref(),
        Some("provider_failure"),
        "outcome must use controlled safe taxonomy"
    );

    // A direct runtime failure event carries a private diagnostic at the
    // event boundary. The dataset observer must classify it and discard the
    // diagnostic rather than serializing the error string.
    observer.emit(RuntimeEvent::TurnFailed {
        session: SessionId::new(),
        request: RequestId::new(),
        trace: TraceId::new(),
        error: "Bearer sk-test-secret https://private.example/user/file".into(),
    });
    let raw_jsonl = std::fs::read_to_string(&jsonl_path).unwrap();
    for forbidden in ["sk-test-secret", "private.example", "/user/file"] {
        assert!(!raw_jsonl.contains(forbidden), "dataset leaked {forbidden}");
    }
    assert!(raw_jsonl.contains("runtime_failure"));
}

// ---------------------------------------------------------------------------
// T3: Cross-Session Global Scope
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t3_cross_session_global_scope() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    let session_a = SessionId::new().to_string();
    let session_b = SessionId::new().to_string();

    // Session A writes an episode with Global scope
    let mut entry = MemoryWritebackEntry::new(
        session_a.clone(),
        "assistant",
        "Apeireth architecture invariant: runtime microkernel is fully decoupled",
    );
    entry.scope = MemoryScope::Global;
    coordinator.writeback(&entry).unwrap();

    // Session B queries visible scopes: [Session(session_b), Global]
    let query_b = MemoryRecallQuery::new(session_b.clone(), "microkernel decoupled")
        .with_visible_scopes(vec![
            MemoryScope::Session {
                session_id: session_b.clone(),
            },
            MemoryScope::Global,
        ]);

    let recalled_b = coordinator.recall(&query_b).unwrap();
    assert_eq!(
        recalled_b.items.len(),
        1,
        "Session B must be able to recall Global episode written by Session A"
    );
    assert!(recalled_b.items[0]
        .content
        .contains("runtime microkernel is fully decoupled"));

    // Session B querying with ONLY Session(session_b) scope must NOT see Session A's global episode
    let query_isolated = MemoryRecallQuery::new(session_b, "microkernel decoupled")
        .with_visible_scopes(vec![MemoryScope::Session {
            session_id: "session-isolated".into(),
        }]);
    let recalled_isolated = coordinator.recall(&query_isolated).unwrap();
    assert!(
        recalled_isolated.items.is_empty(),
        "isolated session scope query must not see other scopes without explicit request"
    );
}

// ---------------------------------------------------------------------------
// T4: Project Scope Isolation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t4_project_scope_isolation() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    // Write episode into project "alpha"
    let mut entry = MemoryWritebackEntry::new(
        "session-1",
        "user",
        "Confidential Project Alpha release plan and architecture roadmap",
    );
    entry.scope = MemoryScope::Project {
        project_id: "alpha".to_string(),
    };
    coordinator.writeback(&entry).unwrap();

    // Query from project "beta" -> must be isolated / not visible
    let query_beta = MemoryRecallQuery::new("session-2", "Project Alpha release plan")
        .with_visible_scopes(vec![MemoryScope::Project {
            project_id: "beta".to_string(),
        }]);
    let recalled_beta = coordinator.recall(&query_beta).unwrap();
    assert!(
        recalled_beta.items.is_empty(),
        "Project beta must NOT see memories from Project alpha"
    );

    // Query from project "alpha" -> must be visible
    let query_alpha = MemoryRecallQuery::new("session-3", "Project Alpha release plan")
        .with_visible_scopes(vec![MemoryScope::Project {
            project_id: "alpha".to_string(),
        }]);
    let recalled_alpha = coordinator.recall(&query_alpha).unwrap();
    assert_eq!(
        recalled_alpha.items.len(),
        1,
        "Project alpha must see its own memories"
    );
}

// ---------------------------------------------------------------------------
// T5: Persona Scope Isolation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t5_persona_scope_isolation() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    // Write for (user_1, persona_x)
    let mut entry = MemoryWritebackEntry::new(
        "session-p1",
        "user",
        "Personal preferences for user_1 persona_x avatar styling",
    );
    entry.scope = MemoryScope::Persona {
        user_id: "user_1".to_string(),
        persona_id: "persona_x".to_string(),
    };
    coordinator.writeback(&entry).unwrap();

    // Query with (user_1, persona_y) -> NOT visible
    let query_wrong_persona = MemoryRecallQuery::new("session-p2", "avatar styling")
        .with_visible_scopes(vec![MemoryScope::Persona {
            user_id: "user_1".to_string(),
            persona_id: "persona_y".to_string(),
        }]);
    let recalled_wrong_persona = coordinator.recall(&query_wrong_persona).unwrap();
    assert!(
        recalled_wrong_persona.items.is_empty(),
        "Different persona for same user must be isolated"
    );

    // Query with (user_2, persona_x) -> NOT visible
    let query_wrong_user = MemoryRecallQuery::new("session-p3", "avatar styling")
        .with_visible_scopes(vec![MemoryScope::Persona {
            user_id: "user_2".to_string(),
            persona_id: "persona_x".to_string(),
        }]);
    let recalled_wrong_user = coordinator.recall(&query_wrong_user).unwrap();
    assert!(
        recalled_wrong_user.items.is_empty(),
        "Different user for same persona must be isolated"
    );

    // Query with (user_1, persona_x) -> VISIBLE
    let query_match =
        MemoryRecallQuery::new("session-p4", "avatar styling").with_visible_scopes(vec![
            MemoryScope::Persona {
                user_id: "user_1".to_string(),
                persona_id: "persona_x".to_string(),
            },
        ]);
    let recalled_match = coordinator.recall(&query_match).unwrap();
    assert_eq!(
        recalled_match.items.len(),
        1,
        "Exact (user_id, persona_id) match must recall the memory"
    );
}

// ---------------------------------------------------------------------------
// T6: Legacy Episode Fail-Narrow Verification
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t6_legacy_episode_fail_narrow() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    // Insert legacy episode directly without metadata row in episode_memory_metadata
    {
        let conn = store.conn().unwrap();
        conn.execute(
            "INSERT INTO episodes (id, continuity_id, timestamp, role, content, session_id)
             VALUES ('ep-legacy-1', 'cid-legacy', 1700000000, 'user', 'Legacy unannotated memory episode', 'session-legacy')",
            [],
        )
        .unwrap();
    }

    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    // Query with matching source session -> visible
    let query_source = MemoryRecallQuery::new("session-legacy", "Legacy unannotated")
        .with_visible_scopes(vec![MemoryScope::Session {
            session_id: "session-legacy".to_string(),
        }]);
    let recalled_source = coordinator.recall(&query_source).unwrap();
    assert_eq!(
        recalled_source.items.len(),
        1,
        "Legacy episode must remain visible to its original source session"
    );

    // Query with other session even requesting Global scope -> NOT visible (fails narrow to session)
    let query_other = MemoryRecallQuery::new("session-other", "Legacy unannotated")
        .with_visible_scopes(vec![
            MemoryScope::Session {
                session_id: "session-other".to_string(),
            },
            MemoryScope::Global,
        ]);
    let recalled_other = coordinator.recall(&query_other).unwrap();
    assert!(
        recalled_other.items.is_empty(),
        "Legacy episode without metadata must fail-narrow to source session and not leak to Global or other sessions"
    );
}

// ---------------------------------------------------------------------------
// T7: Production MemoryCoordinator Hybrid Recall Truthfulness
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t7_production_coordinator_hybrid_recall() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    // Insert multiple distinct episodes to form a corpus for BM25
    coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session-prod",
            "user",
            "The microkernel architecture isolates memory governance from protocol dispatch",
        ))
        .unwrap();
    coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session-prod",
            "user",
            "Relational sqlite databases provide persistent table indexing and atomic transactions",
        ))
        .unwrap();

    let query = MemoryRecallQuery::new("session-prod", "microkernel architecture governance")
        .with_visible_scopes(vec![MemoryScope::Session {
            session_id: "session-prod".to_string(),
        }]);

    let result = coordinator.recall(&query).unwrap();
    assert!(!result.items.is_empty(), "must recall candidate");

    let status = result
        .retrieval_status
        .expect("retrieval_status must be present in production recall");
    assert!(
        status.lexical_candidates > 0,
        "lexical_candidates must be tracked"
    );

    let item = &result.items[0];
    let score_components = item
        .score_components
        .as_ref()
        .expect("score_components must be present");

    assert!(
        score_components.lexical > 0.0,
        "lexical score must be computed by BM25 (was {})",
        score_components.lexical
    );
    assert!(
        item.score > 0.0,
        "final weighted score must be greater than zero (was {})",
        item.score
    );
}

// ---------------------------------------------------------------------------
// T8: Truthful Fallback when EmbeddingProvider is Missing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t8_missing_embedding_truthful_lexical_fallback() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    // Default coordinator has no embedding provider
    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session-fallback",
            "user",
            "Deterministic runtime dispatch ensures fail-closed safety",
        ))
        .unwrap();

    let query = MemoryRecallQuery::new("session-fallback", "deterministic runtime dispatch")
        .with_visible_scopes(vec![MemoryScope::Session {
            session_id: "session-fallback".to_string(),
        }]);

    let result = coordinator.recall(&query).unwrap();
    let status = result
        .retrieval_status
        .expect("retrieval_status must be present");

    assert!(
        status.used_lexical_fallback,
        "used_lexical_fallback must truthfully be true when embedding provider is missing"
    );

    let comp = result.items[0]
        .score_components
        .as_ref()
        .expect("score components must be present");

    assert_eq!(
        comp.semantic, 0.0,
        "semantic score must be truthfully 0.0 without embedding provider, never fabricated"
    );
    assert!(
        comp.lexical > 0.0,
        "lexical score must be computed from BM25"
    );
}

// ---------------------------------------------------------------------------
// T9: Semantic Path with Deterministic Fake Embedding Provider
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t9_semantic_retrieval_with_fake_embedding_provider() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    let fake_embedding: Arc<dyn EmbeddingProvider> = Arc::new(DeterministicFakeEmbeddingProvider);

    let coordinator = Arc::new(
        MemoryCoordinator::new(backend, governance)
            .with_scoped_backend(scoped)
            .with_embedding_provider(fake_embedding),
    );

    coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session-semantic",
            "user",
            "Microkernel runtime provides memory governance and canonical execution",
        ))
        .unwrap();

    let query = MemoryRecallQuery::new("session-semantic", "microkernel memory runtime")
        .with_visible_scopes(vec![MemoryScope::Session {
            session_id: "session-semantic".to_string(),
        }]);

    let result = coordinator.recall(&query).unwrap();
    let status = result
        .retrieval_status
        .expect("retrieval_status must be present");

    assert!(
        !status.used_lexical_fallback,
        "used_lexical_fallback must be false when embedding provider is configured"
    );

    let item = &result.items[0];
    let comp = item
        .score_components
        .as_ref()
        .expect("score components must be present");

    assert!(
        comp.semantic > 0.0,
        "semantic score must be positive when embedding provider succeeds (was {})",
        comp.semantic
    );
    assert!(
        comp.lexical > 0.0,
        "lexical score must also be positive (was {})",
        comp.lexical
    );
}

// ---------------------------------------------------------------------------
// T10: Memory Forget and Dynamic Invalidation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t10_forget_and_hybrid_index_dynamic_invalidation() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    let ep_id = coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session-forget",
            "user",
            "Confidential user secret that must be completely forgotten",
        ))
        .unwrap();

    let query = MemoryRecallQuery::new("session-forget", "confidential user secret")
        .with_visible_scopes(vec![MemoryScope::Session {
            session_id: "session-forget".to_string(),
        }]);

    // Initial recall must find it
    let initial = coordinator.recall(&query).unwrap();
    assert_eq!(initial.items.len(), 1);

    // Trigger forget via governance
    coordinator
        .forget_episode(&ep_id, Some("privacy request compliance"), 0)
        .unwrap();

    // Subsequent recall must NOT find it (purged from working ring buffer and filtered from store)
    let post_forget = coordinator.recall(&query).unwrap();
    assert!(
        post_forget.items.is_empty(),
        "forgotten episode must never be recalled after forget"
    );
}

// ---------------------------------------------------------------------------
// T11: Content Update Invalidates Vector
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t11_content_update_invalidates_vector() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("memory.sqlite3");

    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();

    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    let initial_content = "Initial content about alpha";
    let ep_id = coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session-update",
            "user",
            initial_content,
        ))
        .unwrap();

    // Check initial content hash
    let meta_before = store
        .get_episode_metadata(&ep_id)
        .unwrap()
        .expect("metadata must exist");
    let initial_hash = meta_before
        .get("content_hash")
        .and_then(|v| v.as_str())
        .expect("initial content_hash must exist")
        .to_string();

    // Inject a cached vector to simulate prior embedding calculation
    let mut modified_meta = meta_before.clone();
    modified_meta["vector"] = serde_json::json!([0.1, 0.2, 0.3, 0.4]);
    store.put_episode_metadata(&ep_id, modified_meta).unwrap();

    // Verify vector was injected
    let meta_with_vec = store.get_episode_metadata(&ep_id).unwrap().unwrap();
    assert!(meta_with_vec.get("vector").is_some());

    // Now update episode content via coordinator
    let updated_content = "Completely updated content about beta";
    coordinator
        .update_episode_content(&ep_id, updated_content, Some("user edit"), 0)
        .unwrap();

    // Verify metadata after update:
    // 1. content_hash changed
    // 2. vector was invalidated (removed)
    let meta_after = store
        .get_episode_metadata(&ep_id)
        .unwrap()
        .expect("metadata must exist after update");
    let updated_hash = meta_after
        .get("content_hash")
        .and_then(|v| v.as_str())
        .expect("updated content_hash must exist")
        .to_string();

    assert_ne!(
        initial_hash, updated_hash,
        "content_hash must change after content update"
    );
    assert!(
        meta_after.get("vector").is_none(),
        "cached vector must be stripped on content update to prevent stale vector matches"
    );
}

// ---------------------------------------------------------------------------
// T12: Production SqliteBackend Scope Matrix
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_t12_production_sqlite_backend_scope_and_legacy_boundary() {
    let store = production_sqlite_backend().await;
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store.clone();
    let scoped: Arc<dyn ScopedMemoryBackend> = store.clone();
    let coordinator =
        Arc::new(MemoryCoordinator::new(backend, governance).with_scoped_backend(scoped));

    let mut global = MemoryWritebackEntry::new(
        "session-production-a",
        "assistant",
        "Production global invariant: the runtime kernel is deterministic",
    );
    global.scope = MemoryScope::Global;
    coordinator.writeback(&global).unwrap();

    let mut project = MemoryWritebackEntry::new(
        "session-production-a",
        "user",
        "Production project alpha deployment constraint",
    );
    project.scope = MemoryScope::Project {
        project_id: "alpha".into(),
    };
    coordinator.writeback(&project).unwrap();

    let mut user = MemoryWritebackEntry::new(
        "session-production-a",
        "user",
        "Production user preference is compact output",
    );
    user.scope = MemoryScope::User {
        user_id: "user-production".into(),
    };
    coordinator.writeback(&user).unwrap();

    let mut persona = MemoryWritebackEntry::new(
        "session-production-a",
        "user",
        "Production persona prefers a concise engineering voice",
    );
    persona.scope = MemoryScope::Persona {
        user_id: "user-production".into(),
        persona_id: "persona-production".into(),
    };
    coordinator.writeback(&persona).unwrap();

    let global_query =
        MemoryRecallQuery::new("session-production-b", "runtime kernel deterministic")
            .with_visible_scopes(vec![
                MemoryScope::Session {
                    session_id: "session-production-b".into(),
                },
                MemoryScope::Global,
            ]);
    assert_eq!(coordinator.recall(&global_query).unwrap().items.len(), 1);

    let project_alpha = MemoryRecallQuery::new("session-production-b", "project alpha deployment")
        .with_visible_scopes(vec![MemoryScope::Project {
            project_id: "alpha".into(),
        }]);
    assert_eq!(coordinator.recall(&project_alpha).unwrap().items.len(), 1);
    let project_beta = MemoryRecallQuery::new("session-production-b", "project alpha deployment")
        .with_visible_scopes(vec![MemoryScope::Project {
            project_id: "beta".into(),
        }]);
    assert!(coordinator.recall(&project_beta).unwrap().items.is_empty());

    let user_query = MemoryRecallQuery::new("session-production-b", "compact output")
        .with_visible_scopes(vec![MemoryScope::User {
            user_id: "user-production".into(),
        }]);
    assert_eq!(coordinator.recall(&user_query).unwrap().items.len(), 1);

    let persona_query = MemoryRecallQuery::new("session-production-b", "concise engineering voice")
        .with_visible_scopes(vec![MemoryScope::Persona {
            user_id: "user-production".into(),
            persona_id: "persona-production".into(),
        }]);
    assert_eq!(coordinator.recall(&persona_query).unwrap().items.len(), 1);
    let wrong_persona = MemoryRecallQuery::new("session-production-b", "concise engineering voice")
        .with_visible_scopes(vec![MemoryScope::Persona {
            user_id: "user-production".into(),
            persona_id: "persona-other".into(),
        }]);
    assert!(coordinator.recall(&wrong_persona).unwrap().items.is_empty());

    // A legacy row with no sidecar metadata is visible only to its source
    // session, even when a caller also requests Global scope.
    store
        .pool()
        .write(|conn| {
            conn.execute(
                "INSERT INTO episodes (id, continuity_id, timestamp, role, content, session_id)
                 VALUES ('ep-production-legacy', 'legacy', 1700000000, 'user',
                         'Production legacy session-bound record', 'session-production-legacy')",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let legacy_source =
        MemoryRecallQuery::new("session-production-legacy", "legacy session-bound record")
            .with_visible_scopes(vec![MemoryScope::Session {
                session_id: "session-production-legacy".into(),
            }]);
    assert_eq!(coordinator.recall(&legacy_source).unwrap().items.len(), 1);
    let legacy_other =
        MemoryRecallQuery::new("session-production-other", "legacy session-bound record")
            .with_visible_scopes(vec![
                MemoryScope::Session {
                    session_id: "session-production-other".into(),
                },
                MemoryScope::Global,
            ]);
    let legacy_other_result = coordinator.recall(&legacy_other).unwrap();
    assert!(
        !legacy_other_result
            .items
            .iter()
            .any(|item| item.id == "ep-production-legacy"),
        "legacy row leaked to other scope: {:?}",
        legacy_other_result.items
    );
}

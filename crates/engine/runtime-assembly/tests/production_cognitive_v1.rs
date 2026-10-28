#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{Clock, Episode, SessionId, SystemClock};
use apeireth_memory::{EpisodeStore, PersonaMemory, PersonaMemoryStore, SqliteMemoryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_protocol::canonical::{ContentPart, MessageRole, ToolEventSourceV1};
use apeireth_runtime::{Runtime, SessionEventKind, TurnRequest};
use apeireth_runtime_assembly::{CognitiveBackends, CognitiveModuleConfig, ProductionModules};
use std::sync::Arc;
use support::*;
fn config() -> CognitiveModuleConfig {
    CognitiveModuleConfig {
        memory_recall: true,
        memory_writeback: true,
        preference_recall: false,
        self_assessment: false,
        filesystem: false,
        search: false,
        repo: false,
        ..Default::default()
    }
}
fn modules(db: Arc<SqliteMemoryStore>, organs: bool, persona: bool) -> ProductionModules {
    ProductionModules::build(
        CognitiveModuleConfig { organs, ..config() },
        CognitiveBackends {
            memory: Some(db.clone()),
            memory_governance: Some(db.clone()),
            persona_layer: persona.then_some(db as Arc<dyn PersonaMemoryStore>),
            ..Default::default()
        },
        Arc::new(SystemClock),
    )
    .unwrap()
}
fn user_data(db: &SqliteMemoryStore, session: SessionId) {
    EpisodeStore::put_episode(
        db,
        &Episode {
            id: "old_user_episode".into(),
            timestamp: SystemClock.now().timestamp(),
            session_id: session.to_string(),
            role: "user".into(),
            content: "old_apricot_memory".into(),
        },
    )
    .unwrap();
}
fn tool_text(request: &apeireth_protocol::canonical::NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .map(|m| ContentPart::join_text(&m.content))
        .collect::<Vec<_>>()
        .join("\n")
}
#[tokio::test]
async fn desktop_production_loads_old_data_and_persona_through_registered_tools_without_system_or_double_injection(
) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("existing.sqlite3");
    let session = SessionId::new();
    {
        let old = SqliteMemoryStore::open(&path).unwrap();
        user_data(&old, session);
        let persona = PersonaMemory {
            self_narrative: "saved_persona_narrative".into(),
            ..Default::default()
        };
        PersonaMemoryStore::save(&old, &persona).unwrap();
    }
    let db = Arc::new(SqliteMemoryStore::open(&path).unwrap());
    let provider = Provider::new(vec![]);
    let mut runtime = modules(db.clone(), true, true)
        .register_into(builder(provider.clone()).with_max_module_invocations(2))
        .build()
        .await
        .unwrap();
    assert!(
        runtime
            .tool_declarations()
            .iter()
            .all(|t| !t.name.starts_with("module_")),
        "internal callback tools are not model capabilities"
    );
    runtime
        .execute(TurnRequest::new(session, "old_apricot_memory"))
        .await
        .unwrap();
    let requests = provider.requests();
    assert_eq!(
        requests.len(),
        1 + runtime.config().max_module_invocations,
        "Organ retries remain bounded by the configured side-call budget"
    );
    let text = tool_text(&requests[0]);
    assert_eq!(text.matches("old_apricot_memory").count(), 1);
    assert_eq!(text.matches("saved_persona_narrative").count(), 1);
    assert!(!requests[0]
        .messages
        .iter()
        .any(|m| m.role == MessageRole::System
            && (ContentPart::join_text(&m.content).contains("old_apricot_memory")
                || ContentPart::join_text(&m.content).contains("saved_persona_narrative"))));
    let facts = facts(&runtime, session).await;
    assert!(facts.iter().any(|f|matches!(f.source(),ToolEventSourceV1::Module{module_id,..} if module_id==&pid("compat.cognitive.memory_recall")) && f.result().render().contains("old_apricot_memory")));
    assert!(facts.iter().any(|f|matches!(f.source(),ToolEventSourceV1::Module{module_id,hook} if module_id==&pid("compat.cognitive.organs") && *hook==apeireth_protocol::canonical::ModuleHookV1::OnIdle)));
    let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    assert!(stored
        .events
        .iter()
        .any(|e| matches!(e.event, SessionEventKind::ModuleToolEventObserved { .. })));
    assert_eq!(
        MemoryBackend::recent_episodes(db.as_ref(), &session.to_string(), 20)
            .unwrap()
            .len(),
        3,
        "old episode plus one committed user/assistant pair"
    );
    assert_eq!(
        PersonaMemoryStore::load(db.as_ref())
            .unwrap()
            .self_narrative,
        "saved_persona_narrative"
    );
    assert!(runtime.shutdown().await.is_empty());
}
#[tokio::test]
async fn stopping_production_memory_prevents_implicit_legacy_fallback_and_plain_kernel_needs_no_memory(
) {
    let session = SessionId::new();
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    user_data(&db, session);
    let provider = Provider::new(vec![]);
    let mut runtime = modules(db.clone(), false, false)
        .register_into(builder(provider.clone()))
        .build()
        .await
        .unwrap();
    runtime
        .stop_module(&pid("compat.cognitive.memory_recall"))
        .await
        .unwrap();
    runtime
        .stop_module(&pid("compat.cognitive.memory_writeback"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, "old_apricot_memory"))
        .await
        .unwrap();
    assert!(tool_text(&provider.requests()[0]).is_empty());
    assert_eq!(
        MemoryBackend::recent_episodes(db.as_ref(), &session.to_string(), 20)
            .unwrap()
            .len(),
        1
    );
    assert!(runtime.shutdown().await.is_empty());
    let provider = Provider::new(vec![]);
    let mut plain = builder(provider.clone()).build().await.unwrap();
    plain
        .execute(TurnRequest::new(SessionId::new(), "hello"))
        .await
        .unwrap();
    assert!(provider.requests()[0].tools.is_empty());
    assert!(plain.shutdown().await.is_empty());
}
#[tokio::test]
async fn permission_denial_blocks_real_cognitive_effects_and_guessed_internal_tool_calls_cannot_forge_receipts(
) {
    let session = SessionId::new();
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    user_data(&db, session);
    let provider = Provider::new(vec![
        vec![call(
            "module_cognitive_memory_writeback",
            serde_json::json!({"invocation":apeireth_core::kernel::TaskId::new()}),
        )],
        vec![],
    ]);
    let mut runtime = modules(db.clone(), false, false)
        .register_into(
            builder(provider.clone()).with_governance(Arc::new(
                apeireth_governance::DenyCapabilities::new()
                    .deny(
                        apeireth_core::kernel::CapabilityId::new(
                            "tool.behavior.cognitive.memory_recall",
                        )
                        .unwrap(),
                    )
                    .deny(
                        apeireth_core::kernel::CapabilityId::new(
                            "tool.behavior.cognitive.memory_writeback",
                        )
                        .unwrap(),
                    ),
            )),
        )
        .build()
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, "old_apricot_memory"))
        .await
        .unwrap();
    assert_eq!(
        MemoryBackend::recent_episodes(db.as_ref(), &session.to_string(), 20)
            .unwrap()
            .len(),
        1
    );
    assert!(!tool_text(&provider.requests()[0]).contains("old_apricot_memory"));
    assert!(facts(&runtime, session).await.iter().any(|f| matches!(
        f.source(),
        ToolEventSourceV1::Module { .. }
    ) && f
        .result()
        .render()
        .contains("refused")));
    assert!(runtime.shutdown().await.is_empty());
}
#[tokio::test]
async fn a_model_cannot_call_module_only_tools_even_with_a_host_grant() {
    let session = SessionId::new();
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let provider = Provider::new(vec![
        vec![call(
            "module_cognitive_memory_writeback",
            serde_json::json!({"invocation":apeireth_core::kernel::TaskId::new()}),
        )],
        vec![],
    ]);
    let mut runtime = modules(db.clone(), false, false)
        .register_into(builder(provider.clone()))
        .build()
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, "hello"))
        .await
        .unwrap();
    assert!(provider
        .results()
        .join("")
        .contains("reserved for authenticated module calls"));
    assert_eq!(
        MemoryBackend::recent_episodes(db.as_ref(), &session.to_string(), 20)
            .unwrap()
            .len(),
        2,
        "only legitimate committed user and assistant writeback, not a model-directed callback"
    );
    assert!(runtime.shutdown().await.is_empty());
}

#[tokio::test]
async fn old_builder_memory_entries_keep_v1_roles_stop_and_private_principal_isolation() {
    use apeireth_core::kernel::{NamespaceId, ProfileId};
    use apeireth_plugin::agents::AgentPrincipal;
    for behavior_alias in [false, true] {
        for private in [false, true] {
            let session = SessionId::new();
            let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
            user_data(&db, session);
            let provider = Provider::new(vec![]);
            let cognitive = modules(db.clone(), false, false);
            // The test Host explicitly selects AllowAll in builder(); the API
            // spelling itself supplies no capability grant.
            let mut construction = builder(provider.clone());
            for module in cognitive.modules() {
                construction = if behavior_alias {
                    construction.with_behavior(module.clone())
                } else {
                    construction.with_module(module.clone())
                };
            }
            let mut runtime = construction.build().await.unwrap();
            if private {
                let mut stored = runtime.sessions().load_or_create(session).await.unwrap();
                stored.principal = Some(AgentPrincipal {
                    profile: ProfileId::new(),
                    namespace: NamespaceId::new(),
                });
                runtime.sessions().save(&stored).await.unwrap();
            }
            runtime
                .execute(
                    TurnRequest::new(session, "old_apricot_memory")
                        .with_system("STATIC_HOST_POLICY"),
                )
                .await
                .unwrap();
            let request = &provider.requests()[0];
            apeireth_protocol::canonical::validate_closed_tool_context(&request.messages).unwrap();
            assert_eq!(
                request
                    .messages
                    .iter()
                    .filter(|m| m.role == MessageRole::System)
                    .map(|m| ContentPart::join_text(&m.content))
                    .collect::<Vec<_>>(),
                ["STATIC_HOST_POLICY"]
            );
            if private {
                assert!(tool_text(request).is_empty());
                assert!(facts(&runtime, session).await.is_empty());
                assert_eq!(
                    MemoryBackend::recent_episodes(db.as_ref(), &session.to_string(), 20)
                        .unwrap()
                        .len(),
                    1,
                    "private principal cannot read or write the old global engine"
                );
            } else {
                assert_eq!(tool_text(request).matches("old_apricot_memory").count(), 1);
                let count = facts(&runtime, session).await.len();
                runtime
                    .stop_module(&pid("compat.cognitive.memory_recall"))
                    .await
                    .unwrap();
                runtime
                    .stop_module(&pid("compat.cognitive.memory_writeback"))
                    .await
                    .unwrap();
                runtime
                    .execute(TurnRequest::new(session, "next"))
                    .await
                    .unwrap();
                assert_eq!(
                    facts(&runtime, session).await.len(),
                    count,
                    "stop cannot fall back to the old callbacks"
                );
                assert_eq!(
                    tool_text(&provider.requests()[1])
                        .matches("old_apricot_memory")
                        .count(),
                    1,
                    "persisted fact is present once, with no second injection"
                );
            }
            assert!(runtime.shutdown().await.is_empty());
        }
    }
}

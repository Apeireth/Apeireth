#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{
    CapabilityId, Clock, Episode, NamespaceId, ProfileId, SessionId, SystemClock,
};
use apeireth_memory::{
    MemoryCoordinator, MemoryProvenance, MemoryScope, PersonaMemory, PersonaMemoryStore,
    SqliteMemoryStore,
};
use apeireth_plugin::{agents::*, memory_backend::MemoryBackend};
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::{
    canonical::{module::BehaviorModule, profiles::ProfileConfig},
    AgentHostPolicy, TurnRequest,
};
use apeireth_runtime_assembly::{
    canonical::cognitive::PersonaConstantInjectionModule,
    memory_module::{namespace_key, MemoryModule},
    profile_memory::*,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use support::*;
fn principal() -> AgentPrincipal {
    AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    }
}
fn memory() -> (Arc<MemoryCoordinator>, Arc<SqliteMemoryStore>) {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    (
        Arc::new(MemoryCoordinator::new(db.clone(), db.clone()).with_scoped_backend(db.clone())),
        db,
    )
}
fn seed(c: &MemoryCoordinator, id: &str, scope: MemoryScope, source: &str) {
    c.writeback_scoped_episode(
        &Episode {
            id: id.into(),
            timestamp: SystemClock.now().timestamp(),
            role: "user".into(),
            content: format!("orchard_{id}"),
            session_id: source.into(),
        },
        &scope,
        &MemoryProvenance {
            source_session: Some(source.into()),
            ..Default::default()
        },
    )
    .unwrap();
}
fn binding(
    store: &str,
    access: StoreAccess,
    module: Arc<dyn BehaviorModule>,
) -> CognitiveStoreBinding {
    CognitiveStoreBinding {
        store: store.into(),
        access,
        module,
    }
}
fn tools() -> Vec<CapabilityId> {
    [
        "tool.memory.retrieve",
        "tool.memory.write",
        "tool.memory.update",
        "tool.memory.delete",
        "tool.memory.organize",
        "tool.behavior.cognitive.memory_recall",
        "tool.behavior.cognitive.memory_writeback",
        "tool.behavior.cognitive.persona_injection",
    ]
    .iter()
    .map(|s| CapabilityId::new(*s).unwrap())
    .collect()
}
fn text(r: &apeireth_protocol::canonical::NormalizedRequest) -> String {
    r.messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .map(|m| ContentPart::join_text(&m.content))
        .collect::<Vec<_>>()
        .join("\n")
}
async fn wait_requests(provider: &Provider, count: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while provider.requests().len() < count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn only_granted_profile_reads_legacy_scopes_with_one_projection_and_private_write_provenance()
{
    let (c, db) = memory();
    let a = principal();
    let b = principal();
    let old = SessionId::new().to_string();
    let project = MemoryScope::Project {
        project_id: "orchard-project".into(),
    };
    let persona = MemoryScope::Persona {
        persona_id: "orchard-persona".into(),
        user_id: "owner".into(),
    };
    seed(&c, "global", MemoryScope::Global, &old);
    seed(&c, "project", project.clone(), &old);
    seed(&c, "persona", persona.clone(), &old);
    seed(
        &c,
        "other_project",
        MemoryScope::Project {
            project_id: "ungranted".into(),
        },
        &old,
    );
    seed(&c, "forgotten", MemoryScope::Global, &old);
    c.forget_episode("forgotten", Some("withdrawn"), 0).unwrap();
    c.update_episode_content("global", "orchard_governed", Some("Host"), 0)
        .unwrap();
    PersonaMemoryStore::save(
        db.as_ref(),
        &PersonaMemory {
            self_narrative: "orchard_shared_persona".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let access = ProfileMemoryAccess {
        coordinator: c.clone(),
        read_scopes: vec![MemoryScope::Global, project, persona],
        writable_private: true,
    };
    let recorder = Arc::new(
        apeireth_runtime_assembly::canonical::cognitive::MemoryRecallAccessRecorder::default(),
    );
    let recall = Arc::new(
        access
            .recall_module(&a)
            .with_access_recorder(recorder.clone()),
    );
    let write = Arc::new(access.writeback_module(&a, Arc::new(SystemClock)).unwrap());
    mappings
        .grant(
            &a,
            Some(access),
            vec![
                binding(
                    &format!("orchard:{}:recall-state", namespace_key(&a)),
                    StoreAccess::WritablePrivate,
                    recall,
                ),
                binding(
                    &format!("orchard:{}", namespace_key(&a)),
                    StoreAccess::WritablePrivate,
                    write,
                ),
                binding(
                    "orchard:persona",
                    StoreAccess::ReadOnlyShared,
                    Arc::new(PersonaConstantInjectionModule::new(db.clone())),
                ),
            ],
        )
        .unwrap();
    let production = apeireth_runtime_assembly::ProductionModules::build(
        apeireth_runtime_assembly::CognitiveModuleConfig {
            memory_recall: true,
            memory_writeback: true,
            preference_recall: false,
            self_assessment: false,
            filesystem: false,
            search: false,
            repo: false,
            ..Default::default()
        },
        apeireth_runtime_assembly::CognitiveBackends {
            memory: Some(db.clone()),
            memory_governance: Some(db.clone()),
            scoped_memory: Some(db.clone()),
            persona_layer: Some(db.clone()),
            ..Default::default()
        },
        Arc::new(SystemClock),
    )
    .unwrap();
    let provider = Provider::new(vec![]);
    let runtime = production
        .register_with_profile_mappings(builder(provider.clone()), mappings.clone())
        .with_plugin(Arc::new(
            MemoryModule::new(c.clone(), Arc::new(SystemClock))
                .with_profile_mappings(mappings.clone()),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let first = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: tools(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    first.run("orchard".into()).await.unwrap();
    let request = provider.requests().last().unwrap().clone();
    let projected = text(&request);
    for allowed in [
        "orchard_governed",
        "orchard_project",
        "orchard_persona",
        "orchard_shared_persona",
    ] {
        assert_eq!(
            projected.matches(allowed).count(),
            1,
            "{allowed}: {projected}"
        );
    }
    assert!(!projected.contains("orchard_other_project"));
    assert!(!projected.contains("orchard_forgotten"));
    assert!(!projected.contains("orchard_global"));
    assert!(!request
        .messages
        .iter()
        .any(|m| m.role == MessageRole::System
            && ContentPart::join_text(&m.content).contains("orchard_")));
    assert!(recorder
        .selected_candidate_ids(&first.snapshot().session.to_string())
        .contains(&"global".into()));
    assert!(recorder
        .selected_candidate_ids(&namespace_key(&a))
        .is_empty());
    let episodes = MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 20).unwrap();
    assert_eq!(episodes.len(), 2);
    for episode in episodes {
        let metadata = MemoryBackend::get_episode_metadata(db.as_ref(), &episode.id)
            .unwrap()
            .unwrap();
        assert_eq!(
            metadata["provenance"]["source_session"],
            first.snapshot().session.to_string()
        );
        assert!(metadata["provenance"]["source_request"].as_str().is_some());
        assert!(metadata["provenance"]["source_trace"].as_str().is_some());
        assert_ne!(metadata["provenance"]["source_session"], namespace_key(&a));
    }
    let second = runtime
        .spawn_agent(
            "b",
            AgentConfig {
                principal: Some(b.clone()),
                tools: tools(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    provider.enqueue_tools(vec![call(
        "memory_write",
        json!({"content":"orchard_injection"}),
    )]);
    second.run("orchard".into()).await.unwrap();
    let last = provider.requests().last().unwrap().clone();
    assert!(!text(&last).contains("orchard_governed"));
    assert!(!text(&last).contains("orchard_shared_persona"));
    assert!(text(&last).contains("namespace_denied"));
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&b), 20)
            .unwrap()
            .is_empty()
    );
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 20)
            .unwrap()
            .iter()
            .all(|e| !e.content.contains("orchard_injection"))
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn mapped_shared_scopes_are_read_only_and_model_parameters_cannot_expand_the_grant() {
    let (c, db) = memory();
    let a = principal();
    let source = SessionId::new().to_string();
    seed(&c, "shared", MemoryScope::Global, &source);
    let mappings = Arc::new(ProfileMemoryMappings::default());
    mappings
        .grant(
            &a,
            Some(ProfileMemoryAccess {
                coordinator: c.clone(),
                read_scopes: vec![MemoryScope::Global],
                writable_private: false,
            }),
            vec![],
        )
        .unwrap();
    let mut calls = vec![
        call("memory_write", json!({"content":"denied"})),
        call(
            "memory_update",
            json!({"id":"shared","content":"denied","expected_revision":0}),
        ),
        call(
            "memory_delete",
            json!({"id":"shared","expected_revision":0}),
        ),
        call(
            "memory_retrieve",
            json!({"query":"orchard","namespace":"other","visible_scopes":[{"scope":"global"}]}),
        ),
    ];
    calls.retain(|c| !matches!(c.name.as_str(), "memory_update" | "memory_delete"));
    for (index, call) in calls.iter_mut().enumerate() {
        call.id = format!("attack-{index}");
    }
    let provider = Provider::new(vec![calls, vec![]]);
    let runtime = mappings
        .register_into(builder(provider.clone()), &[])
        .with_plugin(Arc::new(
            MemoryModule::new(c, Arc::new(SystemClock)).with_profile_mappings(mappings),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: tools()
                    .into_iter()
                    .filter(|id| id.to_string().starts_with("tool.memory."))
                    .collect(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("orchard".into()).await.unwrap();
    let results = provider.results().join("\n");
    assert!(results.contains("orchard_shared"));
    assert!(results.contains("namespace_denied"));
    assert!(results.contains("invalid_arguments"));
    for (op, arguments) in [
        (
            "memory_update",
            json!({"id":"shared","content":"denied","expected_revision":0}),
        ),
        (
            "memory_delete",
            json!({"id":"shared","expected_revision":0}),
        ),
    ] {
        provider.enqueue_tools(vec![call(op, arguments)]);
        provider.enqueue_tools(vec![]);
        assert_eq!(
            agent.run("mutate shared".into()).await.unwrap_err().code,
            AgentErrorCode::PendingApproval
        );
        let session = runtime
            .kernel()
            .await
            .sessions()
            .load(&agent.snapshot().session)
            .await
            .unwrap()
            .unwrap();
        runtime
            .resolve_agent_approval(
                agent.id(),
                session.active_approval_id.unwrap(),
                apeireth_runtime::ApprovalDecision::Approve,
            )
            .await
            .unwrap();
        assert!(text(provider.requests().last().unwrap()).contains("namespace_denied"));
    }
    assert_eq!(
        MemoryBackend::get_episode(db.as_ref(), "shared")
            .unwrap()
            .unwrap()
            .content,
        "orchard_shared"
    );
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 20)
            .unwrap()
            .is_empty()
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn revoke_cancels_waiting_response_and_old_tools_fork_reload_and_cache_cannot_use_new_grant()
{
    let (c, db) = memory();
    let a = principal();
    seed(&c, "revocable", MemoryScope::Global, "old-session");
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let access = ProfileMemoryAccess {
        coordinator: c.clone(),
        read_scopes: vec![MemoryScope::Global],
        writable_private: true,
    };
    mappings.grant(&a, Some(access.clone()), vec![]).unwrap();
    let provider = Provider::new(vec![]);
    let runtime = mappings
        .register_into(builder(provider.clone()), &[])
        .with_plugin(Arc::new(
            MemoryModule::new(c, Arc::new(SystemClock)).with_profile_mappings(mappings.clone()),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let scope: Vec<_> = tools()
        .into_iter()
        .filter(|id| id.to_string().starts_with("tool.memory."))
        .collect();
    let first = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: scope.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    first.run("orchard".into()).await.unwrap();
    let session_id = first.snapshot().session;
    let mut stored = runtime
        .kernel()
        .await
        .sessions()
        .load(&session_id)
        .await
        .unwrap()
        .unwrap();
    let fork = stored.fork_session(stored.log.len(), &SystemClock).unwrap();
    let restored: apeireth_runtime::Session =
        serde_json::from_str(&serde_json::to_string(&fork).unwrap()).unwrap();
    assert_eq!(stored.authorization_epoch, restored.authorization_epoch);
    runtime
        .kernel()
        .await
        .sessions()
        .save(&restored)
        .await
        .unwrap();
    let hold = provider.hold_next();
    let running = {
        let agent = first.clone();
        tokio::spawn(async move { agent.run("orchard".into()).await })
    };
    wait_requests(&provider, 2).await;
    provider.enqueue_tools(vec![call(
        "memory_write",
        json!({"content":"late_old_write"}),
    )]);
    mappings.revoke(&a);
    assert!(tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    hold.notify_one();
    mappings.grant(&a, Some(access), vec![]).unwrap();
    assert!(first.run("orchard".into()).await.is_err());
    assert!(runtime
        .kernel()
        .await
        .execute(TurnRequest::new(restored.id, "orchard"))
        .await
        .is_err());
    // A reload without its Host authority cannot replay a stamped private transcript.
    let mut plain = builder(Provider::new(vec![])).build().await.unwrap();
    plain.sessions().save(&restored).await.unwrap();
    assert!(plain
        .execute(TurnRequest::new(restored.id, "orchard"))
        .await
        .is_err());
    plain.shutdown().await;
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 20)
            .unwrap()
            .is_empty()
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn profile_switch_drops_old_in_flight_response_without_transferring_memory_or_tools() {
    let (c, db) = memory();
    let a = principal();
    let b = principal();
    seed(&c, "switch_secret", MemoryScope::Global, "legacy");
    let mappings = Arc::new(ProfileMemoryMappings::default());
    mappings
        .grant(
            &a,
            Some(ProfileMemoryAccess {
                coordinator: c.clone(),
                read_scopes: vec![MemoryScope::Global],
                writable_private: true,
            }),
            vec![],
        )
        .unwrap();
    let provider = Provider::new(vec![]);
    let runtime = mappings
        .register_into(builder(provider.clone()), &[])
        .with_plugin(Arc::new(
            MemoryModule::new(c, Arc::new(SystemClock)).with_profile_mappings(mappings),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let profile = |p: &AgentPrincipal| ProfileConfig {
        id: p.profile,
        namespace: p.namespace,
        label: "private".into(),
        agent: AgentConfig {
            tools: tools()
                .into_iter()
                .filter(|id| id.to_string().starts_with("tool.memory."))
                .collect(),
            ..Default::default()
        },
    };
    let first = runtime.activate_profile(&profile(&a)).await.unwrap();
    let hold = provider.hold_next();
    let pending = {
        let agent = first.clone();
        tokio::spawn(async move { agent.run("orchard".into()).await })
    };
    wait_requests(&provider, 1).await;
    let next = runtime.activate_profile(&profile(&b)).await.unwrap();
    assert!(pending.await.unwrap().is_err());
    hold.notify_one();
    next.run("orchard".into()).await.unwrap();
    assert!(!text(provider.requests().last().unwrap()).contains("orchard_switch_secret"));
    assert_ne!(first.snapshot().session, next.snapshot().session);
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&b), 20)
            .unwrap()
            .is_empty()
    );
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn revocation_invalidates_frozen_mutation_approval() {
    let (c, db) = memory();
    let a = principal();
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let access = ProfileMemoryAccess {
        coordinator: c.clone(),
        read_scopes: vec![],
        writable_private: true,
    };
    mappings.grant(&a, Some(access.clone()), vec![]).unwrap();
    let provider = Provider::new(vec![
        vec![call(
            "memory_write",
            json!({"content":"original","idempotency_key":"approval"}),
        )],
        vec![],
    ]);
    let runtime = mappings
        .register_into(builder(provider.clone()), &[])
        .with_plugin(Arc::new(
            MemoryModule::new(c.clone(), Arc::new(SystemClock))
                .with_profile_mappings(mappings.clone())
                .with_automatic_retrieval(false),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: tools()
                    .into_iter()
                    .filter(|id| id.to_string().starts_with("tool.memory."))
                    .collect(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("remember".into()).await.unwrap();
    let episode = MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 1)
        .unwrap()
        .remove(0);
    provider.enqueue_tools(vec![call(
        "memory_update",
        json!({"id":episode.id,"content":"late edit","expected_revision":0}),
    )]);
    assert_eq!(
        agent.run("edit".into()).await.unwrap_err().code,
        AgentErrorCode::PendingApproval
    );
    let session = runtime
        .kernel()
        .await
        .sessions()
        .load(&agent.snapshot().session)
        .await
        .unwrap()
        .unwrap();
    mappings.revoke(&a);
    mappings.grant(&a, Some(access), vec![]).unwrap();
    assert!(runtime
        .resolve_agent_approval(
            agent.id(),
            session.active_approval_id.unwrap(),
            apeireth_runtime::ApprovalDecision::Approve
        )
        .await
        .is_err());
    assert_eq!(
        c.governance()
            .get_governed(&episode.id)
            .unwrap()
            .unwrap()
            .episode
            .content,
        "original"
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn mapped_organizer_keeps_original_epoch_and_cannot_write_after_revoke() {
    let (c, db) = memory();
    let a = principal();
    let mappings = Arc::new(ProfileMemoryMappings::default());
    mappings
        .grant(
            &a,
            Some(ProfileMemoryAccess {
                coordinator: c.clone(),
                read_scopes: vec![MemoryScope::Global],
                writable_private: true,
            }),
            vec![],
        )
        .unwrap();
    let provider = Provider::new(vec![
        vec![call("memory_organize", json!({"query":"orchard"}))],
        vec![],
    ]);
    let release = provider.organizer_script(vec![
        vec![call("memory_write", json!({"content":"revoked summary"}))],
        vec![],
    ]);
    let module = Arc::new(
        MemoryModule::new(c, Arc::new(SystemClock))
            .with_profile_mappings(mappings.clone())
            .with_automatic_retrieval(false),
    );
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        pid("module.memory"),
        apeireth_runtime::ModuleAgentGrant {
            tools: ["tool.memory.retrieve", "tool.memory.write"]
                .iter()
                .map(|s| CapabilityId::new(*s).unwrap())
                .collect(),
            principals: vec![a.clone()],
            max_depth: 1,
            ..Default::default()
        },
    );
    let runtime = mappings
        .register_into(builder(provider.clone()), &[])
        .with_plugin(module.clone())
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: vec![CapabilityId::new("tool.memory.organize").unwrap()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("organize".into()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !provider.requests().iter().any(|r| {
            r.messages
                .iter()
                .any(|m| ContentPart::join_text(&m.content).starts_with("memory.organizer.v1:"))
        }) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    mappings.revoke(&a);
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !module
            .organizer_snapshots()
            .iter()
            .all(|a| a.status == AgentStatus::Destroyed)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 20)
            .unwrap()
            .is_empty()
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn mapped_state_stores_are_explicit_read_only_or_exclusively_owned() {
    use apeireth_memory::partner::{
        InMemoryPartnerStore, Partner, PartnerId, PartnerPreferences, PartnerStore,
    };
    use apeireth_runtime_assembly::canonical::{
        cognitive::PartnerBondModule, organ_module::OrganModule,
    };
    let a = principal();
    let b = principal();
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let shared = Arc::new(InMemoryPartnerStore::new());
    let id = PartnerId("shared-subject".into());
    shared
        .save_partner(&Partner::new(
            id.clone(),
            "shared",
            PartnerPreferences::default(),
            1,
        ))
        .unwrap();
    let snapshot = Arc::new(
        PartnerBondModule::new(shared.clone(), "shared-subject", Arc::new(SystemClock))
            .with_read_only_snapshot(),
    );
    let organs = Arc::new(OrganModule::new(Arc::new(SystemClock)));
    let (coordinator, _) = memory();
    let recall_access = ProfileMemoryAccess {
        coordinator,
        read_scopes: vec![],
        writable_private: false,
    };
    let recall_with_receipts = Arc::new(recall_access.recall_module(&a).with_access_recorder(
        Arc::new(
            apeireth_runtime_assembly::canonical::cognitive::MemoryRecallAccessRecorder::default(),
        ),
    ));
    assert!(mappings
        .grant(
            &a,
            None,
            vec![binding(
                "shared-recall-with-private-receipts",
                StoreAccess::ReadOnlyShared,
                recall_with_receipts
            )]
        )
        .is_err());

    use apeireth_plugin::preference::{PreferenceStore, UserPreference};
    use apeireth_runtime_assembly::canonical::cognitive::PreferenceRecallModule;
    let prefs = Arc::new(
        apeireth_memory::preference_store_sqlite::SQLitePreferenceStore::new(
            apeireth_storage::SqliteConnectionPool::in_memory()
                .await
                .unwrap(),
        ),
    );
    prefs.ensure_schema().await.unwrap();
    let source = SessionId::new();
    prefs
        .record(&UserPreference {
            id: "source-preference".into(),
            session_id: source,
            topic: "orchard".into(),
            stance: "orchard_shared_preference".into(),
            evidence_refs: vec![],
            created_at: 1,
            confidence: 0.9,
            tags: vec![],
        })
        .unwrap();
    let pref_source = Arc::new(SharedPreferenceSource::new(prefs, vec![source]));
    assert!(pref_source.forget("source-preference").is_err());
    let pref_module = Arc::new(PreferenceRecallModule::new(pref_source));
    assert!(mappings
        .grant(
            &a,
            None,
            vec![binding(
                "organs",
                StoreAccess::ReadOnlyShared,
                organs.clone()
            )]
        )
        .is_err());
    mappings
        .grant(
            &a,
            None,
            vec![
                binding(
                    "shared-partner",
                    StoreAccess::ReadOnlyShared,
                    snapshot.clone(),
                ),
                binding(
                    "private-organs",
                    StoreAccess::WritablePrivate,
                    organs.clone(),
                ),
                binding("shared-prefs", StoreAccess::ReadOnlyShared, pref_module),
            ],
        )
        .unwrap();
    assert!(mappings
        .grant(
            &b,
            None,
            vec![binding(
                "private-organs",
                StoreAccess::WritablePrivate,
                Arc::new(OrganModule::new(Arc::new(SystemClock)))
            )]
        )
        .is_err());
    assert!(mappings
        .grant(
            &b,
            None,
            vec![binding(
                "renamed-organs",
                StoreAccess::WritablePrivate,
                organs.clone()
            )]
        )
        .is_err());
    let provider = Provider::new(vec![]);
    let runtime = mappings
        .register_into(builder(provider.clone()), &[])
        .with_max_module_invocations(1)
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let scope = vec![
        CapabilityId::new("tool.behavior.cognitive.partner_bond").unwrap(),
        CapabilityId::new("tool.behavior.cognitive.organs").unwrap(),
        CapabilityId::new("tool.behavior.cognitive.preference_recall").unwrap(),
    ];
    let agent = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: scope.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("private interaction".into()).await.unwrap();
    assert_eq!(
        shared
            .get_partner(&id)
            .unwrap()
            .unwrap()
            .bond
            .evolution_count,
        0
    );
    assert_eq!(organs.observations().len(), 1);
    assert!(text(&provider.requests()[0]).contains("orchard_shared_preference"));
    let other = runtime
        .spawn_agent(
            "b",
            AgentConfig {
                principal: Some(b),
                tools: scope,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    other.run("unmapped interaction".into()).await.unwrap();
    assert_eq!(organs.observations().len(), 1);
    assert!(!text(provider.requests().last().unwrap()).contains("互动连续性"));
    assert!(!text(provider.requests().last().unwrap()).contains("orchard_shared_preference"));
    mappings.revoke(&a);
    assert!(
        mappings
            .grant(
                &principal(),
                None,
                vec![binding(
                    "private-organs",
                    StoreAccess::WritablePrivate,
                    Arc::new(OrganModule::new(Arc::new(SystemClock)))
                )]
            )
            .is_err(),
        "revocation does not transfer ownership"
    );
    runtime.shutdown().await.unwrap();
}
struct ScopedExtractor;
#[async_trait::async_trait]
impl apeireth_memory::MemoryExtractor for ScopedExtractor {
    async fn extract(
        &self,
        input: apeireth_memory::MemoryExtractionInput,
    ) -> Result<apeireth_memory::MemoryExtractionResult, apeireth_memory::MemoryError> {
        use apeireth_memory::*;
        let attack = input.messages.iter().any(|m| m.content.contains("escape"));
        let scope = if attack {
            MemoryScope::Global
        } else {
            input.scope
        };
        let provenance = MemoryProvenance {
            source: "model-attempt".into(),
            source_session: Some("forged".into()),
            ..Default::default()
        };
        Ok(MemoryExtractionResult {
            facts: vec![ExtractedMemory {
                class: ExtractionClass::Fact,
                content: "materialized_orchard".into(),
                confidence: 0.9,
                scope: scope.clone(),
                provenance: provenance.clone(),
                source_trace: None,
            }],
            events: vec![ExtractedMemory {
                class: ExtractionClass::Event,
                content: if attack {
                    "escaped commitment".into()
                } else {
                    "private commitment".into()
                },
                confidence: 0.9,
                provenance,
                scope,
                source_trace: None,
            }],
            ..Default::default()
        })
    }
}
#[tokio::test]
async fn typed_and_generic_materialization_enforce_partition_and_keep_actual_source_identity() {
    use apeireth_memory::{MemoryMaterializer, SqliteCommitmentStore};
    use apeireth_runtime_assembly::CanonicalMemoryTypedSink;
    let (c, db) = memory();
    let a = principal();
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let pool = Arc::new(
        apeireth_storage::SqliteConnectionPool::in_memory()
            .await
            .unwrap(),
    );
    let commitments = Arc::new(SqliteCommitmentStore::from_arc(pool));
    commitments.ensure_schema().await.unwrap();
    let access = ProfileMemoryAccess {
        coordinator: c.clone(),
        read_scopes: vec![],
        writable_private: true,
    };
    let sink = CanonicalMemoryTypedSink::new()
        .with_commitments(commitments.clone())
        .with_identity(namespace_key(&a), namespace_key(&a));
    let writeback = Arc::new(
        access
            .writeback_module(&a, Arc::new(SystemClock))
            .unwrap()
            .with_materializer(Arc::new(MemoryMaterializer::new(ScopedExtractor)))
            .with_typed_sink(Arc::new(sink)),
    );
    mappings
        .grant(
            &a,
            Some(access),
            vec![binding(
                "private-materialization",
                StoreAccess::WritablePrivate,
                writeback.clone(),
            )],
        )
        .unwrap();
    let runtime = mappings
        .register_into(builder(Provider::new(vec![])), &[])
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: vec![CapabilityId::new("tool.behavior.cognitive.memory_writeback").unwrap()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("remember a fact".into()).await.unwrap();
    let active = commitments
        .list_active_for_subject(&namespace_key(&a), None, None)
        .unwrap();
    assert_eq!(active.len(), 1);
    let provenance: serde_json::Value =
        serde_json::from_str(active[0].provenance.as_ref().unwrap()).unwrap();
    assert_eq!(
        provenance["source_session"],
        agent.snapshot().session.to_string()
    );
    assert!(provenance["source_trace"].as_str().is_some());
    let materialized = MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 30)
        .unwrap()
        .into_iter()
        .find(|e| e.content == "materialized_orchard")
        .unwrap();
    let metadata = MemoryBackend::get_episode_metadata(db.as_ref(), &materialized.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        metadata["provenance"]["source_session"],
        agent.snapshot().session.to_string()
    );
    agent.run("escape scope".into()).await.unwrap();
    assert_eq!(
        commitments
            .list_active_for_subject(&namespace_key(&a), None, None)
            .unwrap()
            .len(),
        1
    );
    assert!(writeback.metrics().warnings > 0);
    assert!(MemoryBackend::recent_episodes(db.as_ref(), "forged", 30)
        .unwrap()
        .is_empty());
    runtime.shutdown().await.unwrap();
}

struct OverbroadCandidates(Vec<Episode>);
impl apeireth_memory::ScopedMemoryBackend for OverbroadCandidates {
    fn query_candidates(
        &self,
        _: &apeireth_memory::MemoryCandidateQuery,
    ) -> Result<Vec<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.0.clone())
    }
}
#[tokio::test]
async fn mapped_scope_filter_rechecks_backend_candidates_and_preserves_recall_order() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let seed_coord = MemoryCoordinator::new(db.clone(), db.clone());
    let old = SessionId::new().to_string();
    seed(&seed_coord, "allowed-global", MemoryScope::Global, &old);
    seed(
        &seed_coord,
        "allowed-project",
        MemoryScope::Project {
            project_id: "granted".into(),
        },
        &old,
    );
    seed(
        &seed_coord,
        "denied-project",
        MemoryScope::Project {
            project_id: "other".into(),
        },
        &old,
    );
    apeireth_memory::EpisodeStore::put_episode(
        db.as_ref(),
        &Episode {
            id: "legacy-null".into(),
            timestamp: SystemClock.now().timestamp(),
            role: "user".into(),
            content: "orchard_null_secret".into(),
            session_id: old.clone(),
        },
    )
    .unwrap();
    let all = MemoryBackend::recent_episodes(db.as_ref(), &old, 20).unwrap();
    let c = Arc::new(
        MemoryCoordinator::new(db.clone(), db.clone())
            .with_scoped_backend(Arc::new(OverbroadCandidates(all))),
    );
    let a = principal();
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let access = ProfileMemoryAccess {
        coordinator: c.clone(),
        read_scopes: vec![
            MemoryScope::Global,
            MemoryScope::Project {
                project_id: "granted".into(),
            },
        ],
        writable_private: true,
    };
    let mut query = apeireth_memory::MemoryRecallQuery::new(namespace_key(&a), "orchard");
    query.visible_scopes = access.scopes(&a);
    query.exact_scope_authorization = true;
    let expected = c
        .recall(&query)
        .unwrap()
        .items
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 2);
    assert!(!expected.contains(&"legacy-null".into()));
    mappings.grant(&a, Some(access), vec![]).unwrap();
    let provider = Provider::new(vec![]);
    let runtime = mappings
        .register_into(builder(provider), &[])
        .with_plugin(Arc::new(
            MemoryModule::new(c.clone(), Arc::new(SystemClock)).with_profile_mappings(mappings),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: vec![CapabilityId::new("tool.memory.retrieve").unwrap()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("orchard".into()).await.unwrap();
    let recorded = facts(&*runtime.kernel().await, agent.snapshot().session).await;
    let actual = recorded
        .iter()
        .filter_map(|fact| match &fact.result().outcome {
            apeireth_protocol::canonical::ToolOutcome::Ok { value } => value["items"].as_array(),
            _ => None,
        })
        .flatten()
        .map(|item| item["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    // A writable target still cannot mutate the granted read-only source scopes.
    assert_eq!(
        c.governance()
            .get_governed("allowed-global")
            .unwrap()
            .unwrap()
            .revision,
        0
    );
    runtime.shutdown().await.unwrap();
}

struct TypedSource;
impl apeireth_memory::TypedMemoryRecallSource for TypedSource {
    fn candidates(
        &self,
        _: &apeireth_memory::MemoryRecallQuery,
        _: &apeireth_memory::TypedRecallIdentity,
        _: i64,
    ) -> Result<Vec<apeireth_memory::MemoryCandidate>, Box<dyn std::error::Error + Send + Sync>>
    {
        Ok(vec![apeireth_memory::MemoryCandidate {
            id: "typed-a".into(),
            layer: "semantic".into(),
            scope: MemoryScope::User {
                user_id: "typed-owner".into(),
            },
            content: "orchard_typed_secret".into(),
            score: 1.0,
            score_components: apeireth_memory::ScoreComponents {
                lexical: 1.0,
                importance: 1.0,
                confidence: 1.0,
                ..Default::default()
            },
            provenance: MemoryProvenance::default(),
        }])
    }
}
#[tokio::test]
async fn mapped_private_miss_never_falls_back_to_global_or_implicit_typed_identity() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let c = Arc::new(
        MemoryCoordinator::new(db.clone(), db.clone())
            .with_scoped_backend(db.clone())
            .with_typed_recall(
                Arc::new(TypedSource),
                apeireth_memory::TypedRecallIdentity {
                    persona_id: "persona".into(),
                    subject_id: "typed-owner".into(),
                },
            ),
    );
    seed(&c, "unmapped_global", MemoryScope::Global, "old-source");
    let a = principal();
    let b = AgentPrincipal {
        profile: a.profile,
        namespace: NamespaceId::new(),
    };
    let mappings = Arc::new(ProfileMemoryMappings::default());
    mappings
        .grant(
            &a,
            Some(ProfileMemoryAccess {
                coordinator: c.clone(),
                read_scopes: vec![MemoryScope::User {
                    user_id: "typed-owner".into(),
                }],
                writable_private: true,
            }),
            vec![],
        )
        .unwrap();
    mappings
        .grant(
            &b,
            Some(ProfileMemoryAccess {
                coordinator: c.clone(),
                read_scopes: vec![],
                writable_private: true,
            }),
            vec![],
        )
        .unwrap();
    let provider = Provider::new(vec![]);
    let runtime = mappings
        .register_into(builder(provider.clone()), &[])
        .with_plugin(Arc::new(
            MemoryModule::new(c, Arc::new(SystemClock)).with_profile_mappings(mappings),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let scope = vec![CapabilityId::new("tool.memory.retrieve").unwrap()];
    let first = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a),
                tools: scope.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    first.run("orchard".into()).await.unwrap();
    assert!(text(&provider.requests()[0]).contains("orchard_typed_secret"));
    assert!(!text(&provider.requests()[0]).contains("orchard_unmapped_global"));
    let second = runtime
        .spawn_agent(
            "b",
            AgentConfig {
                principal: Some(b),
                tools: scope,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    second.run("orchard".into()).await.unwrap();
    let projected = text(provider.requests().last().unwrap());
    assert!(!projected.contains("orchard_typed_secret"));
    assert!(!projected.contains("orchard_unmapped_global"));
    runtime.shutdown().await.unwrap();
}

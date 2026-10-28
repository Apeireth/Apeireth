#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{CapabilityId, NamespaceId, ProfileId, SystemClock};
use apeireth_memory::{MemoryCoordinator, MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_plugin::agents::*;
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::canonical::profiles::{ProfileConfig, ProfileStore};
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant, TurnRequest};
use apeireth_runtime_assembly::canonical::memory_module::{namespace_key, MemoryModule};
use apeireth_runtime_assembly::profile_store::SqliteProfileStore;
use serde_json::json;
use std::sync::Arc;
use support::*;
fn principal() -> AgentPrincipal {
    AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    }
}
fn ids(ops: &[&str]) -> Vec<CapabilityId> {
    ops.iter()
        .map(|op| CapabilityId::new(format!("tool.memory.{op}")).unwrap())
        .collect()
}
fn memory() -> (Arc<MemoryCoordinator>, Arc<SqliteMemoryStore>) {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let backend: Arc<dyn MemoryBackend> = db.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = db.clone();
    (Arc::new(MemoryCoordinator::new(backend, governance)), db)
}
#[tokio::test]
async fn memory_hook_is_a_paired_untrusted_tool_result_and_profiles_cannot_read_each_other() {
    let (coordinator, db) = memory();
    let a = principal();
    let b = principal();
    let provider = Provider::new(vec![
        vec![call(
            "memory_write",
            json!({"content":"private_apricot_781","idempotency_key":"a"}),
        )],
        vec![],
        vec![],
        vec![],
    ]);
    let module = Arc::new(MemoryModule::new(coordinator, Arc::new(SystemClock)));
    let runtime = builder(provider.clone())
        .with_plugin(module)
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let first = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: ids(&["retrieve", "write"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    first
        .run("remember private_apricot_781".into())
        .await
        .unwrap();
    first.run("private_apricot_781".into()).await.unwrap();
    let requests = provider.requests();
    assert!(requests
        .last()
        .unwrap()
        .messages
        .iter()
        .any(|m| m.role == MessageRole::Tool
            && ContentPart::join_text(&m.content).contains("private_apricot_781")));
    assert!(!requests
        .last()
        .unwrap()
        .messages
        .iter()
        .any(|m| m.role == MessageRole::System
            && ContentPart::join_text(&m.content).contains("private_apricot_781")));
    let facts = facts(&*runtime.kernel().await, first.snapshot().session).await;
    assert!(facts.iter().any(|f|matches!(f.source(), apeireth_protocol::canonical::ToolEventSourceV1::Module {module_id, ..} if module_id == &pid("module.memory"))));
    assert_eq!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 20)
            .unwrap()
            .len(),
        1
    );
    let second = runtime
        .spawn_agent(
            "b",
            AgentConfig {
                principal: Some(b),
                tools: ids(&["retrieve"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    second.run("private_apricot_781".into()).await.unwrap();
    assert!(!provider
        .requests()
        .last()
        .unwrap()
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .any(|m| ContentPart::join_text(&m.content).contains("private_apricot_781")));
    let persisted = runtime
        .kernel()
        .await
        .sessions()
        .load(&first.snapshot().session)
        .await
        .unwrap()
        .unwrap();
    let restored: apeireth_runtime::Session =
        serde_json::from_str(&serde_json::to_string(&persisted).unwrap()).unwrap();
    assert_eq!(restored.principal, Some(a));
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn memory_writes_are_idempotent_and_namespaces_cannot_be_selected_in_arguments() {
    let (c, db) = memory();
    let principal = principal();
    let mut one = call(
        "memory_write",
        json!({"content":"append_one","idempotency_key":"same"}),
    );
    one.id = "one".into();
    let mut two = one.clone();
    two.id = "two".into();
    let mut conflict = call(
        "memory_write",
        json!({"content":"different","idempotency_key":"same"}),
    );
    conflict.id = "three".into();
    let provider = Provider::new(vec![
        vec![
            one,
            two,
            conflict,
            call(
                "memory_write",
                json!({"content":"leak","namespace":"other"}),
            ),
        ],
        vec![],
    ]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(
            MemoryModule::new(c, Arc::new(SystemClock)).with_automatic_retrieval(false),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(principal.clone()),
                tools: ids(&["write"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("append".into()).await.unwrap();
    let results = provider.results().join("\n");
    assert!(results.contains("replayed"));
    assert!(results.contains("idempotency_conflict"));
    assert!(results.contains("invalid_arguments"));
    let episodes =
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&principal), 20).unwrap();
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0].content, "append_one");
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn no_principal_fails_closed_and_no_memory_module_still_runs_a_conversation() {
    let (c, _) = memory();
    let provider = Provider::new(vec![
        vec![call("memory_write", json!({"content":"forbidden"}))],
        vec![],
    ]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(
            MemoryModule::new(c, Arc::new(SystemClock)).with_automatic_retrieval(false),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "unbound",
            AgentConfig {
                tools: ids(&["write"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("write".into()).await.unwrap();
    assert!(provider.results().join("").contains("namespace_denied"));
    runtime.shutdown().await.unwrap();
    let provider = Provider::new(vec![]);
    let runtime = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    assert_eq!(
        runtime
            .spawn_agent("minimal", AgentConfig::default())
            .await
            .unwrap()
            .run("hello".into())
            .await
            .unwrap()
            .text,
        "done"
    );
    assert!(provider.requests()[0].tools.is_empty());
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn profiles_persist_with_cas_and_switch_without_two_active_main_agents() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("profiles.sqlite3");
    let store = SqliteProfileStore::open(&path).await.unwrap();
    let a = ProfileConfig {
        id: ProfileId::new(),
        namespace: NamespaceId::new(),
        label: "A".into(),
        agent: AgentConfig::default(),
    };
    let b = ProfileConfig {
        id: ProfileId::new(),
        namespace: NamespaceId::new(),
        label: "B".into(),
        agent: AgentConfig::default(),
    };
    store.save(a.clone(), None).await.unwrap();
    store.save(b.clone(), None).await.unwrap();
    assert!(store.save(a.clone(), None).await.is_err());
    let mut edited = a.clone();
    edited.label = "A2".into();
    assert_eq!(store.save(edited, Some(1)).await.unwrap().revision, 2);
    assert!(store.save(a.clone(), Some(1)).await.is_err());
    drop(store);
    let store = SqliteProfileStore::open(&path).await.unwrap();
    assert_eq!(store.list().await.unwrap().len(), 2);
    assert_eq!(store.load(a.id).await.unwrap().unwrap().revision, 2);
    let runtime = builder(Provider::new(vec![]))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    assert!(runtime.instances().is_empty());
    let first = runtime.activate_profile(&a).await.unwrap();
    first.run("private a".into()).await.unwrap();
    let second = runtime.activate_profile(&b).await.unwrap();
    assert_eq!(first.snapshot().status, AgentStatus::Destroyed);
    assert_ne!(first.snapshot().session, second.snapshot().session);
    assert_ne!(first.snapshot().principal, second.snapshot().principal);
    second.run("private b".into()).await.unwrap();
    runtime.deactivate_profile().await.unwrap();
    assert_eq!(second.snapshot().status, AgentStatus::Destroyed);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn background_memory_agent_writes_only_its_namespace_and_main_keeps_running() {
    let (c, db) = memory();
    let principal = principal();
    let provider = Provider::new(vec![
        vec![call("memory_organize", json!({"query":"topic"}))],
        vec![],
        vec![],
    ]);
    let release = provider.organizer_script(vec![
        vec![call("memory_retrieve", json!({"query":"topic"}))],
        vec![call(
            "memory_write",
            json!({"content":"organized_summary","idempotency_key":"summary"}),
        )],
        vec![],
    ]);
    let module =
        Arc::new(MemoryModule::new(c, Arc::new(SystemClock)).with_automatic_retrieval(false));
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        pid("module.memory"),
        ModuleAgentGrant {
            tools: ids(&["retrieve", "write"]).into_iter().collect(),
            principals: vec![principal.clone()],
            max_depth: 1,
            ..Default::default()
        },
    );
    let runtime = builder(provider.clone())
        .with_plugin(module.clone())
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let main = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(principal.clone()),
                tools: ids(&["organize"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    main.run("main_private_start".into()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
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
    // The child is held inside its Provider, while the main Agent completes another turn.
    assert_eq!(
        main.run("main_private_again".into()).await.unwrap().text,
        "done"
    );
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&principal), 10)
            .unwrap()
            .is_empty()
    );
    release.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if module
                .organizer_snapshots()
                .iter()
                .any(|a| a.status == AgentStatus::Destroyed)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let children = module.organizer_snapshots();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].usage.requests, 3);
    assert_eq!(children[0].usage.tool_calls, 2);
    assert_eq!(main.snapshot().usage.requests, 3);
    let stored =
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&principal), 10).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].content, "organized_summary");
    for request in provider.requests().iter().filter(|r| {
        r.messages
            .iter()
            .any(|m| ContentPart::join_text(&m.content).starts_with("memory.organizer.v1:"))
    }) {
        assert!(!request
            .messages
            .iter()
            .any(|m| ContentPart::join_text(&m.content).contains("main_private")));
        assert!(request
            .tools
            .iter()
            .all(|t| ["memory_retrieve", "memory_write"].contains(&t.name.as_str())));
    }
    runtime
        .unregister_module(&pid("module.memory"))
        .await
        .unwrap();
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn update_delete_keep_frozen_approval_revision_and_protection_and_deny_cross_namespace() {
    use apeireth_runtime::ApprovalDecision;
    let (c, db) = memory();
    let a = principal();
    let b = principal();
    let provider = Provider::new(vec![
        vec![call(
            "memory_write",
            json!({"content":"original","idempotency_key":"edit"}),
        )],
        vec![],
    ]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(
            MemoryModule::new(c.clone(), Arc::new(SystemClock)).with_automatic_retrieval(false),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "a",
            AgentConfig {
                principal: Some(a.clone()),
                tools: ids(&["write", "update", "delete"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("remember".into()).await.unwrap();
    let id = MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&a), 1).unwrap()[0]
        .id
        .clone();
    async fn approve(runtime: &apeireth_runtime::AgentRuntime, agent: &Arc<dyn AgentInstance>) {
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
                ApprovalDecision::Approve,
            )
            .await
            .unwrap();
    }
    let mut edit = call(
        "memory_update",
        json!({"id":id,"content":"edited","expected_revision":0}),
    );
    edit.id = "edit".into();
    provider.enqueue_tools(vec![edit]);
    provider.enqueue_tools(vec![]);
    assert_eq!(
        agent.run("edit".into()).await.unwrap_err().code,
        AgentErrorCode::PendingApproval
    );
    assert_eq!(
        c.governance()
            .get_governed(&id)
            .unwrap()
            .unwrap()
            .episode
            .content,
        "original"
    );
    approve(&runtime, &agent).await;
    let edited = c.governance().get_governed(&id).unwrap().unwrap();
    assert_eq!(edited.episode.content, "edited");
    assert_eq!(edited.revision, 1);
    let mut stale = call(
        "memory_update",
        json!({"id":id,"content":"stale","expected_revision":0}),
    );
    stale.id = "stale".into();
    provider.enqueue_tools(vec![stale]);
    provider.enqueue_tools(vec![]);
    assert_eq!(
        agent.run("stale edit".into()).await.unwrap_err().code,
        AgentErrorCode::PendingApproval
    );
    approve(&runtime, &agent).await;
    assert!(provider.results().join("").contains("memory_conflict"));
    assert_eq!(
        c.governance()
            .get_governed(&id)
            .unwrap()
            .unwrap()
            .episode
            .content,
        "edited"
    );
    let protected = c.protect_episode(&id, 1).unwrap();
    let mut delete = call(
        "memory_delete",
        json!({"id":id,"expected_revision":protected.revision}),
    );
    delete.id = "delete".into();
    provider.enqueue_tools(vec![delete]);
    provider.enqueue_tools(vec![]);
    assert_eq!(
        agent.run("delete protected".into()).await.unwrap_err().code,
        AgentErrorCode::PendingApproval
    );
    approve(&runtime, &agent).await;
    assert!(c.governance().get_governed(&id).unwrap().unwrap().protected);
    let other = runtime
        .spawn_agent(
            "b",
            AgentConfig {
                principal: Some(b),
                tools: ids(&["update"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let mut cross = call(
        "memory_update",
        json!({"id":id,"content":"cross","expected_revision":protected.revision}),
    );
    cross.id = "cross".into();
    provider.enqueue_tools(vec![cross]);
    provider.enqueue_tools(vec![]);
    assert_eq!(
        other.run("cross namespace".into()).await.unwrap_err().code,
        AgentErrorCode::PendingApproval
    );
    approve(&runtime, &other).await;
    assert!(provider.results().last().unwrap().contains("not_found"));
    assert_eq!(
        c.governance()
            .get_governed(&id)
            .unwrap()
            .unwrap()
            .episode
            .content,
        "edited"
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn stopping_memory_cancels_an_active_organizer_and_releases_its_owned_session() {
    let (c, _) = memory();
    let p = principal();
    let provider = Provider::new(vec![
        vec![call("memory_organize", json!({}))],
        vec![],
        vec![],
    ]);
    let _never_release = provider.organizer_script(vec![]);
    let module =
        Arc::new(MemoryModule::new(c, Arc::new(SystemClock)).with_automatic_retrieval(false));
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        pid("module.memory"),
        ModuleAgentGrant {
            tools: ids(&["retrieve", "write"]).into_iter().collect(),
            principals: vec![p.clone()],
            ..Default::default()
        },
    );
    let runtime = builder(provider.clone())
        .with_plugin(module.clone())
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let main = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(p),
                tools: ids(&["organize"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    main.run("start".into()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !module
            .organizer_snapshots()
            .iter()
            .any(|a| a.status == AgentStatus::Running)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let child = module.organizer_snapshots()[0].clone();
    runtime
        .unregister_module(&pid("module.memory"))
        .await
        .unwrap();
    assert_eq!(
        runtime.snapshot(child.id).unwrap().status,
        AgentStatus::Destroyed
    );
    assert!(runtime
        .kernel()
        .await
        .sessions()
        .load(&child.session)
        .await
        .unwrap()
        .is_none());
    assert_eq!(main.run("still usable".into()).await.unwrap().text, "done");
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn recall_estimate_budget_and_persisted_fork_scope_remain_closed() {
    use apeireth_core::kernel::{Clock, Episode, SessionId};
    use apeireth_runtime::canonical::dispatcher::ToolScope;
    let (c, _) = memory();
    let p = principal();
    let clock = Arc::new(SystemClock);
    let ep = Episode {
        id: "budget_episode".into(),
        session_id: namespace_key(&p),
        timestamp: clock.now().timestamp(),
        role: "memory".into(),
        content: "budget_private ".repeat(200),
    };
    c.writeback_episode(&ep).unwrap();
    let provider = Provider::new(vec![]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(
            MemoryModule::new(c, clock.clone()).with_recall_token_budget(64),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "bounded",
            AgentConfig {
                principal: Some(p.clone()),
                tools: ids(&["retrieve"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("budget_private".into()).await.unwrap();
    let facts = facts(&*runtime.kernel().await, agent.snapshot().session).await;
    let fact = facts
        .iter()
        .find(|f| {
            matches!(
                f.source(),
                apeireth_protocol::canonical::ToolEventSourceV1::Module { .. }
            )
        })
        .unwrap();
    assert!(apeireth_orchestration::token_meter::fold_text(&fact.result().render()) <= 64);
    let mut session = runtime
        .kernel()
        .await
        .sessions()
        .load(&agent.snapshot().session)
        .await
        .unwrap()
        .unwrap();
    let child = session
        .fork_session(session.log.len(), clock.as_ref())
        .unwrap();
    assert_eq!(child.principal, Some(p));
    assert_eq!(child.tool_scope, Some(ids(&["retrieve"])));
    let fork_id = child.id;
    runtime
        .kernel()
        .await
        .sessions()
        .save(&child)
        .await
        .unwrap();
    // The Raw compatibility entry must restore persisted scope rather than exposing its default full catalog.
    runtime
        .kernel()
        .await
        .execute(TurnRequest::new(fork_id, "budget_private"))
        .await
        .unwrap();
    assert_eq!(
        provider
            .requests()
            .last()
            .unwrap()
            .tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        vec!["memory_retrieve"]
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn search_and_non_destructive_compact_use_governed_content_not_stale_working_ring() {
    use apeireth_core::kernel::{Clock, Episode};
    let (c, db) = memory();
    let p = principal();
    let clock = Arc::new(SystemClock);
    let ep = Episode {
        id: "governed_compact".into(),
        session_id: namespace_key(&p),
        timestamp: clock.now().timestamp(),
        role: "memory".into(),
        content: "obsolete_private_text".into(),
    };
    c.writeback_episode(&ep).unwrap();
    c.update_episode_content(&ep.id, "corrected_memory_text", Some("host"), 0)
        .unwrap();
    let search = call("memory_search", json!({"query":"corrected_memory_text"}));
    let mut compact = call("memory_compact", json!({"query":"corrected_memory_text"}));
    compact.id = "compact".into();
    let provider = Provider::new(vec![vec![search, compact], vec![]]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(
            MemoryModule::new(c, clock).with_automatic_retrieval(false),
        ))
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(p.clone()),
                tools: ids(&["search", "compact"]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("corrected_memory_text".into()).await.unwrap();
    let results = provider.results().join("\n");
    assert!(results.contains("non_destructive_summary"));
    assert!(results.contains("corrected_memory_text"));
    assert!(!results.contains("obsolete_private_text"));
    assert_eq!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&p), 10)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        MemoryBackend::get_episode(db.as_ref(), &ep.id)
            .unwrap()
            .unwrap()
            .content,
        "obsolete_private_text"
    );
    runtime.shutdown().await.unwrap();
}

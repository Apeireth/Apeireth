//! Production factory, mapped Host authority and accepted-only crash recovery.
//! The provider is deterministic instrumentation, never a model-quality score.

use std::path::Path;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, NamespaceId, PluginId, ProfileId, SessionId, VirtualClock,
};
use apeireth_governance::AllowAll;
use apeireth_memory::automatic_correction::AutomaticCorrectionPlan;
use apeireth_memory::backend::sqlite::SqliteBackend;
use apeireth_memory::writeback_recovery::SqliteWritebackJournal;
use apeireth_memory::{MemoryCoordinator, MemoryGovernanceStore};
use apeireth_plugin::agents::AgentPrincipal;
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, NormalizedFinishReason, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::{
    AgentHostPolicy, AgentRuntime, ModuleAgentGrant, Runtime, Session, SessionStore, TurnRequest,
};
use apeireth_runtime_assembly::canonical::memory_correction::CORRECTION_PROPOSAL_SYSTEM;
use apeireth_runtime_assembly::memory_module::namespace_key;
use apeireth_runtime_assembly::profile_memory::{ProfileMemoryAccess, ProfileMemoryMappings};
use apeireth_runtime_assembly::{
    CognitiveBackends, CognitiveModuleConfig, ProductionModules, SqliteSessionStore,
};
use apeireth_storage::SqliteConnectionPool;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde_json::json;

const MODEL: &str = "production-memory-instrumentation";
const OLD: &str = "我住在武汉。";
const CURRENT: &str = "我住在上海。";
const CORRECTION: &str = "我其实住在上海，请按实际住处回答。";

#[derive(Clone)]
struct Call {
    proposal: bool,
    request: NormalizedRequest,
}
struct Provider {
    id: CapabilityId,
    instance: SessionId,
    allow_proposals: bool,
    calls: Mutex<Vec<Call>>,
    revoke: Mutex<Option<(Arc<ProfileMemoryMappings>, AgentPrincipal)>>,
}
impl Provider {
    fn new(allow_proposals: bool) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.memory-autonomy-production").unwrap(),
            instance: SessionId::new(),
            allow_proposals,
            calls: Mutex::new(vec![]),
            revoke: Mutex::new(None),
        })
    }
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
    fn revoke_during_proposal(
        &self,
        mappings: Arc<ProfileMemoryMappings>,
        principal: AgentPrincipal,
    ) {
        *self.revoke.lock().unwrap() = Some((mappings, principal));
    }
}
#[async_trait]
impl ProviderCapability for Provider {
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
        let proposal = request.messages.iter().any(|message| {
            message.role == MessageRole::System
                && ContentPart::join_text(&message.content) == CORRECTION_PROPOSAL_SYSTEM
        });
        assert!(
            !proposal || self.allow_proposals,
            "recovery must not re-ask the model for a frozen correction"
        );
        let content = if proposal {
            let input = request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == MessageRole::User)
                .map(|message| ContentPart::join_text(&message.content))
                .unwrap();
            let input: serde_json::Value = serde_json::from_str(&input).unwrap();
            let candidate = input["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .find(|candidate| candidate["text"] == OLD)
                .expect("the owned original fact must be a real supplied candidate");
            if let Some((mappings, principal)) = self.revoke.lock().unwrap().take() {
                mappings.revoke(&principal);
            }
            json!({"decision":"correct_prior","target_id":candidate["id"],"subject":"self","source_kind":"direct_assertion","evidence":"我其实住在上海","target_anchor":"我住在武汉","old_fragment":"武汉","new_fragment":"上海","effective_time":null}).to_string()
        } else {
            "已收到。".into()
        };
        let mut calls = self.calls.lock().unwrap();
        let number = calls.len();
        calls.push(Call {
            proposal,
            request: request.clone(),
        });
        Ok(NormalizedResponse {
            id: format!("production-{}-{number}", self.instance),
            model: request.model.clone(),
            content,
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(10, 10),
            tool_calls: vec![],
            raw_metadata: Default::default(),
        })
    }
}
struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<Provider>,
}
impl ProviderPlugin {
    fn new(provider: Arc<Provider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("plugin.memory-autonomy-production").unwrap(),
                "1.0.0",
                "Production memory instrumentation",
            )
            .declare_capability(
                provider.id.clone(),
                CapabilityKind::Provider,
                "proposal and primary request recording",
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
fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0)
            .single()
            .unwrap(),
    ))
}
fn config(memory: bool) -> CognitiveModuleConfig {
    CognitiveModuleConfig {
        memory_recall: memory,
        memory_writeback: memory,
        memory_injection: false,
        preference_recall: false,
        self_assessment: false,
        self_status: false,
        filesystem: false,
        search: false,
        repo: false,
        ..Default::default()
    }
}
async fn database(path: &Path) -> (Arc<SqliteBackend>, Arc<SqliteWritebackJournal>) {
    let pool = Arc::new(SqliteConnectionPool::open(path).await.unwrap());
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .unwrap();
    (
        Arc::new(SqliteBackend::from_arc(pool.clone())),
        Arc::new(SqliteWritebackJournal::from_pool(pool)),
    )
}
fn production(
    root: &Path,
    backend: &Arc<SqliteBackend>,
    journal: Option<Arc<SqliteWritebackJournal>>,
) -> ProductionModules {
    ProductionModules::build(
        config(true),
        CognitiveBackends {
            memory: Some(backend.clone()),
            memory_governance: Some(backend.clone()),
            scoped_memory: Some(backend.clone()),
            memory_recovery: journal,
            memory_data_root: Some(root.join("isolated-memory-data")),
            ..Default::default()
        },
        clock(),
    )
    .unwrap()
}
async fn assemble(
    root: &Path,
    backend: &Arc<SqliteBackend>,
    journal: Option<Arc<SqliteWritebackJournal>>,
    provider: Arc<Provider>,
) -> (Runtime, Arc<SqliteSessionStore>, Vec<String>) {
    let modules = production(root, backend, journal);
    let ids = modules.ids();
    let sessions = Arc::new(
        SqliteSessionStore::open(root.join("sessions.sqlite3"))
            .await
            .unwrap(),
    );
    let runtime = modules
        .register_into(
            Runtime::builder()
                .with_clock(clock())
                .with_session_store(sessions.clone())
                .with_governance(Arc::new(AllowAll))
                .with_plugin(ProviderPlugin::new(provider))
                .with_default_model(MODEL),
        )
        .build()
        .await
        .unwrap();
    (runtime, sessions, ids)
}
fn assert_pipeline(ids: &[String]) {
    let position = |id: &str| {
        ids.iter()
            .position(|item| item == id)
            .unwrap_or_else(|| panic!("{id} missing from {ids:?}"))
    };
    assert!(position("cognitive.memory_recovery") < position("cognitive.memory_correction"));
    assert!(position("cognitive.memory_correction") < position("cognitive.memory_recall"));
    assert!(position("cognitive.memory_recall") < position("cognitive.memory_writeback"));
}
fn old_user(backend: &SqliteBackend, namespace: &str) -> apeireth_core::Episode {
    backend
        .recent_episodes(namespace, 64)
        .unwrap()
        .into_iter()
        .find(|episode| episode.role == "user" && episode.content == OLD)
        .unwrap()
}
fn memory_text(request: &NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::Tool)
        .filter_map(|message| {
            serde_json::from_str::<serde_json::Value>(&ContentPart::join_text(&message.content))
                .ok()
        })
        .filter(|frame| {
            frame["version"] == 1
                && frame["origin"] == "module"
                && frame["module_id"] == "compat.cognitive.memory_recall"
                && frame["hook"] == "turn_start"
                && frame["payload"]["status"] == "ok"
                && frame["payload"]["value"]["untrusted_data"] == true
        })
        .flat_map(|frame| {
            frame["payload"]["value"]["context"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter(|context| context["source"] == "memory_recall")
        .filter_map(|context| context["text"].as_str().map(str::to_owned))
        .collect::<Vec<_>>()
        .join("\n")
}

fn all_text(request: &NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .map(|message| ContentPart::join_text(&message.content))
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn production_rebuild_restores_prepared_correction_after_journal_registration_failure_without_reproposal(
) {
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let (backend, journal) = database(&dir.path().join("memory.sqlite3")).await;
    let provider = Provider::new(true);
    let (runtime, sessions, ids) = assemble(
        dir.path(),
        &backend,
        Some(journal.clone()),
        provider.clone(),
    )
    .await;
    assert_pipeline(&ids);
    runtime
        .execute(TurnRequest::new(session, OLD))
        .await
        .unwrap();
    let original = old_user(&backend, &session.to_string());
    backend.pool().write_sync(|connection|{connection.execute_batch("CREATE TRIGGER reject_correction_registration BEFORE INSERT ON memory_writeback_jobs WHEN NEW.id LIKE 'correction/%' BEGIN SELECT RAISE(ABORT,'controlled correction registration failure'); END;")?;Ok(())}).unwrap();
    let failure = runtime.execute(TurnRequest::new(session, CORRECTION)).await;
    assert!(
        failure.is_err(),
        "an accepted plan with an unavailable journal must be reported as a failed turn"
    );
    let intents = sessions.operation_evidence(&session).await.unwrap();
    assert_eq!(
        intents.len(),
        1,
        "the independently committed accepted plan must survive journal failure"
    );
    let plan: AutomaticCorrectionPlan = serde_json::from_value(intents[0].payload.clone()).unwrap();
    assert_eq!(intents[0].operation_id, plan.operation_id);
    assert_eq!(plan.evidence.content, CORRECTION);
    assert_eq!(
        backend
            .get_governed(&original.id)
            .unwrap()
            .unwrap()
            .revision,
        0
    );
    assert!(backend.get_episode(&plan.evidence.id).unwrap().is_none());
    assert!(!journal
        .pending(&session.to_string(), 64)
        .unwrap()
        .iter()
        .any(|job| job.id == format!("correction/{}", plan.operation_id)));
    let calls = provider.calls();
    assert_eq!(calls.iter().filter(|call| call.proposal).count(), 1);
    assert_eq!(
        calls.iter().filter(|call| !call.proposal).count(),
        1,
        "no primary response may claim the uncommitted correction succeeded"
    );
    backend
        .pool()
        .write_sync(|connection| {
            connection.execute_batch("DROP TRIGGER reject_correction_registration")?;
            Ok(())
        })
        .unwrap();
    drop(runtime);
    drop(sessions);
    drop(provider);

    let provider = Provider::new(false);
    let (runtime, sessions, ids) = assemble(
        dir.path(),
        &backend,
        Some(journal.clone()),
        provider.clone(),
    )
    .await;
    assert_pipeline(&ids);
    runtime
        .execute(TurnRequest::new(session, "我实际住哪里？"))
        .await
        .unwrap();
    let governed = backend.get_governed(&original.id).unwrap().unwrap();
    assert_eq!(governed.episode.content, CURRENT);
    assert_eq!(governed.revision, 1);
    let calls = provider.calls();
    assert_eq!(calls.len(), 1);
    assert!(!calls[0].proposal);
    assert!(
        memory_text(&calls[0].request).contains(CURRENT),
        "restoration must precede ordinary Recall: {}",
        memory_text(&calls[0].request)
    );
    assert!(
        plan.targets
            .iter()
            .any(|target| memory_text(&calls[0].request)
                .lines()
                .any(|line| line.starts_with(&format!("[mem:{} ", target.id))
                    || line.starts_with(&format!("[mem:{}]", target.id)))),
        "ordinary Recall must cite an actually corrected target: {}",
        memory_text(&calls[0].request)
    );
    let restored = sessions.operation_evidence(&session).await.unwrap();
    assert_eq!(restored[0].payload, intents[0].payload);
    assert_eq!(restored[0].payload_hash, intents[0].payload_hash);
    runtime
        .execute(TurnRequest::new(session, "再核对当前住处。"))
        .await
        .unwrap();
    assert_eq!(
        backend
            .get_governed(&original.id)
            .unwrap()
            .unwrap()
            .revision,
        1,
        "repeated source enumeration must not consume the operation twice"
    );
    assert_eq!(provider.calls().len(), 2);
}

#[tokio::test]
async fn legacy_production_without_a_journal_keeps_its_primary_call_budget() {
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let (backend, _journal) = database(&dir.path().join("memory.sqlite3")).await;
    let provider = Provider::new(false);
    let (runtime, sessions, ids) = assemble(dir.path(), &backend, None, provider.clone()).await;
    assert!(!ids
        .iter()
        .any(|id| id == "cognitive.memory_recovery" || id == "cognitive.memory_correction"));
    runtime
        .execute(TurnRequest::new(session, OLD))
        .await
        .unwrap();
    let original = old_user(&backend, &session.to_string());
    runtime
        .execute(TurnRequest::new(session, CORRECTION))
        .await
        .unwrap();
    let calls = provider.calls();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|call| !call.proposal));
    assert_eq!(
        backend
            .get_governed(&original.id)
            .unwrap()
            .unwrap()
            .revision,
        0
    );
    assert!(sessions
        .operation_evidence(&session)
        .await
        .unwrap()
        .is_empty());
}

fn principal() -> AgentPrincipal {
    AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    }
}
fn access(backend: &Arc<SqliteBackend>) -> ProfileMemoryAccess {
    ProfileMemoryAccess {
        coordinator: Arc::new(
            MemoryCoordinator::new(backend.clone(), backend.clone())
                .with_scoped_backend(backend.clone()),
        ),
        read_scopes: vec![],
        writable_private: true,
    }
}
async fn host_session(sessions: &SqliteSessionStore, id: SessionId, principal: AgentPrincipal) {
    let mut session = Session::new(id, clock().as_ref());
    session.principal = Some(principal);
    sessions.save(&session).await.unwrap();
}
async fn mapped_runtime(
    root: &Path,
    mappings: Arc<ProfileMemoryMappings>,
    provider: Arc<Provider>,
    principals: Vec<AgentPrincipal>,
) -> (AgentRuntime, Arc<SqliteSessionStore>) {
    let modules =
        ProductionModules::build(config(false), CognitiveBackends::default(), clock()).unwrap();
    let sessions = Arc::new(
        SqliteSessionStore::open(root.join("sessions.sqlite3"))
            .await
            .unwrap(),
    );
    // Behavior access and child-model creation are independent Host grants.
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        apeireth_runtime::canonical::behavior_adapter::registered_behavior_owner(
            "cognitive.memory_correction",
        )
        .unwrap(),
        ModuleAgentGrant {
            principals,
            ..Default::default()
        },
    );
    let runtime = modules
        .register_with_profile_mappings(
            Runtime::builder()
                .with_clock(clock())
                .with_session_store(sessions.clone())
                .with_governance(Arc::new(AllowAll))
                .with_plugin(ProviderPlugin::new(provider))
                .with_default_model(MODEL),
            mappings,
        )
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let ids = runtime
        .kernel()
        .await
        .modules()
        .iter()
        .map(|module| module.manifest().id.clone())
        .collect::<Vec<_>>();
    assert_pipeline(&ids);
    (runtime, sessions)
}

#[tokio::test]
async fn mapped_production_pipeline_uses_owned_database_and_cannot_cross_profile() {
    let dir = tempfile::tempdir().unwrap();
    let a = principal();
    let b = principal();
    let a_session = SessionId::new();
    let b_session = SessionId::new();
    let (a_backend, a_journal) = database(&dir.path().join("a.sqlite3")).await;
    let (b_backend, b_journal) = database(&dir.path().join("b.sqlite3")).await;
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let a_access = access(&a_backend);
    let b_access = access(&b_backend);
    mappings
        .grant(
            &a,
            Some(a_access.clone()),
            a_access
                .recoverable_modules(
                    &a,
                    clock(),
                    a_journal.clone(),
                    mappings.clone(),
                    "production-profile-a",
                )
                .unwrap(),
        )
        .unwrap();
    mappings
        .grant(
            &b,
            Some(b_access.clone()),
            b_access
                .recoverable_modules(
                    &b,
                    clock(),
                    b_journal,
                    mappings.clone(),
                    "production-profile-b",
                )
                .unwrap(),
        )
        .unwrap();
    let attempted = a_access
        .recoverable_modules(
            &b,
            clock(),
            a_journal,
            mappings.clone(),
            "production-profile-a",
        )
        .unwrap();
    assert!(
        mappings.grant(&b, Some(a_access), attempted).is_err(),
        "Host ownership must reject the writable database alias before a turn"
    );
    let provider = Provider::new(true);
    let (runtime, sessions) = mapped_runtime(
        dir.path(),
        mappings.clone(),
        provider.clone(),
        vec![a.clone(), b.clone()],
    )
    .await;
    host_session(&sessions, a_session, a.clone()).await;
    host_session(&sessions, b_session, b.clone()).await;
    runtime
        .kernel()
        .await
        .execute(TurnRequest::new(a_session, OLD))
        .await
        .unwrap();
    runtime
        .kernel()
        .await
        .execute(TurnRequest::new(b_session, "我住在成都。"))
        .await
        .unwrap();
    let original = old_user(&a_backend, &namespace_key(&a));
    runtime
        .kernel()
        .await
        .execute(TurnRequest::new(a_session, CORRECTION))
        .await
        .unwrap();
    assert_eq!(
        a_backend
            .get_governed(&original.id)
            .unwrap()
            .unwrap()
            .episode
            .content,
        CURRENT
    );
    assert_eq!(
        a_backend
            .get_governed(&original.id)
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    runtime
        .kernel()
        .await
        .execute(TurnRequest::new(b_session, "我住哪里？"))
        .await
        .unwrap();
    let calls = provider.calls();
    assert_eq!(calls.iter().filter(|call| call.proposal).count(), 1);
    let last = &calls.last().unwrap().request;
    assert!(memory_text(last).contains("我住在成都。"));
    assert!(
        !all_text(last).contains("武汉") && !all_text(last).contains("上海"),
        "the entire foreign Profile request must exclude A: {}",
        all_text(last)
    );
    assert!(b_backend
        .recent_episodes(&namespace_key(&b), 64)
        .unwrap()
        .iter()
        .all(|episode| !episode.content.contains("上海") && !episode.content.contains("武汉")));
    let before = provider.calls().len();
    mappings.revoke(&a);
    assert!(runtime
        .kernel()
        .await
        .execute(TurnRequest::new(a_session, "我其实住在广州。"))
        .await
        .is_err());
    assert_eq!(provider.calls().len(), before);
    assert_eq!(
        a_backend
            .get_governed(&original.id)
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn mapped_authority_revoked_during_semantic_proposal_cannot_prepare_or_apply_a_correction() {
    let dir = tempfile::tempdir().unwrap();
    let a = principal();
    let session = SessionId::new();
    let (backend, journal) = database(&dir.path().join("a.sqlite3")).await;
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let access = access(&backend);
    mappings
        .grant(
            &a,
            Some(access.clone()),
            access
                .recoverable_modules(&a, clock(), journal, mappings.clone(), "revoked-profile-a")
                .unwrap(),
        )
        .unwrap();
    let provider = Provider::new(true);
    let (runtime, sessions) = mapped_runtime(
        dir.path(),
        mappings.clone(),
        provider.clone(),
        vec![a.clone()],
    )
    .await;
    host_session(&sessions, session, a.clone()).await;
    runtime
        .kernel()
        .await
        .execute(TurnRequest::new(session, OLD))
        .await
        .unwrap();
    let original = old_user(&backend, &namespace_key(&a));
    provider.revoke_during_proposal(mappings, a);
    assert!(runtime
        .kernel()
        .await
        .execute(TurnRequest::new(session, CORRECTION))
        .await
        .is_err());
    assert_eq!(
        provider.calls().iter().filter(|call| call.proposal).count(),
        1
    );
    assert_eq!(
        backend
            .get_governed(&original.id)
            .unwrap()
            .unwrap()
            .episode
            .content,
        OLD
    );
    assert_eq!(
        backend
            .get_governed(&original.id)
            .unwrap()
            .unwrap()
            .revision,
        0
    );
    assert!(
        sessions
            .operation_evidence(&session)
            .await
            .unwrap()
            .is_empty(),
        "revoked semantic output must never become a trusted prepared operation"
    );
}

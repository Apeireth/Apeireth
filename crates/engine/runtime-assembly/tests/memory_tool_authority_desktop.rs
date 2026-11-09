//! Memory tool face authority on the default desktop path.
//!
//! # The defect this file pins
//!
//! The default desktop composition (`canonical::desktop_host::build_desktop_main_runtime`,
//! the composition root the CLI uses) binds **no** Host identity
//! (`AgentConfig { ..Default::default() }` ⇒ `principal: None`). The memory tool
//! face is **namespace-scoped**: its only authority is the identity carried by
//! the execution context (`memory_module.rs`). The face was nevertheless
//! published to the model, so every call answered
//! `namespace_denied / "Host principal is required"` — a promise the model could
//! never keep. "Wired but always failing" is the same defect class as "not wired".
//!
//! # The rule the tests pin
//!
//! * The desktop main offers **no** namespace-scoped capability
//!   (`desktop_main_offers_no_namespace_scoped_tool`).
//! * An out-of-scope call is refused at the dispatcher's scope gate, never at
//!   the memory module (`an_out_of_scope_memory_call_never_reaches_the_module`).
//! * The **governed** memory path (behavior modules: recall/writeback into the
//!   transcript) is untouched (`the_desktop_memory_behavior_path_still_works`).
//! * The memory tool face itself is intact: a Host-bound identity plus the
//!   module grant still operates it
//!   (`a_host_bound_identity_still_operates_the_memory_tools`).
//!
//! # Counter-evidence
//!
//! Removing the `filter` in `canonical/desktop_host.rs` turns
//! `desktop_main_offers_no_namespace_scoped_tool` red (the memory tools are
//! published again); removing the `module.memory` grant turns
//! `a_host_bound_identity_still_operates_the_memory_tools` red.
#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{
    CapabilityId, Clock, Episode, NamespaceId, PluginId, ProfileId, SessionId, SystemClock,
};
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use apeireth_plugin::agents::{AgentConfig, AgentPrincipal};
use apeireth_plugin::preference::{PreferenceStore, UserPreference};
use apeireth_plugin::self_assessment::{SelfAssessment, SelfAssessmentStore};
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant, TurnRequest};
use apeireth_runtime_assembly::{
    canonical::desktop_host::build_desktop_main_runtime, CognitiveBackends, CognitiveModuleConfig,
    ProductionModules,
};
use std::sync::Arc;
use support::*;

type CapResult<T> = apeireth_plugin::memory_backend::CapabilityResult<T>;

/// Two inert stores so the **default** module configuration can be built
/// without pulling the storage stack into this file.
struct NoopPreferences;
impl PreferenceStore for NoopPreferences {
    fn record(&self, _pref: &UserPreference) -> CapResult<()> {
        Ok(())
    }
    fn recall_for_context(
        &self,
        _session_id: &SessionId,
        _current_topic: &str,
        _limit: u32,
    ) -> CapResult<Vec<UserPreference>> {
        Ok(vec![])
    }
    fn forget(&self, _pref_id: &str) -> CapResult<()> {
        Ok(())
    }
    fn list_for_session(&self, _session_id: &SessionId) -> CapResult<Vec<UserPreference>> {
        Ok(vec![])
    }
}

struct NoopAssessments;
impl SelfAssessmentStore for NoopAssessments {
    fn record(&self, _sa: &SelfAssessment) -> CapResult<()> {
        Ok(())
    }
    fn recent_for_task(&self, _task_id: &str, _limit: u32) -> CapResult<Vec<SelfAssessment>> {
        Ok(vec![])
    }
    fn latest_alignment(&self, _task_id: &str) -> CapResult<Option<f64>> {
        Ok(None)
    }
}

/// The production memory configuration (`memory_recall` + `memory_writeback`).
fn production(db: Arc<SqliteMemoryStore>) -> ProductionModules {
    ProductionModules::build(
        CognitiveModuleConfig::default(),
        CognitiveBackends {
            memory: Some(db.clone()),
            memory_governance: Some(db.clone()),
            scoped_memory: Some(db.clone()),
            preferences: Some(Arc::new(NoopPreferences)),
            self_assessments: Some(Arc::new(NoopAssessments)),
            ..Default::default()
        },
        Arc::new(SystemClock),
    )
    .unwrap()
}

fn cid(id: &str) -> CapabilityId {
    CapabilityId::new(id).unwrap()
}

fn principal() -> AgentPrincipal {
    AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    }
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

/// ① **反证**: the desktop main must publish **no** namespace-scoped capability.
///
/// Before the fix the seven `memory_*` tools were in the model-facing
/// declaration list while every call answered `namespace_denied`.
#[tokio::test]
async fn desktop_main_offers_no_namespace_scoped_tool() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let provider = Provider::new(vec![]);
    let modules = production(db);
    let policy = modules.desktop_agent_policy().unwrap();
    let runtime = build_desktop_main_runtime(modules.register_into(builder(provider)), policy)
        .await
        .unwrap();

    let main = runtime.managed_main().expect("desktop main identity");
    assert!(
        main.principal.is_none(),
        "desktop main binds no Host identity (the premise of this file)"
    );
    let declared = runtime.tool_declarations_for_session(main.session);
    let names: Vec<&str> = declared.iter().map(|t| t.name.as_str()).collect();
    assert!(
        !names.iter().any(|name| name.starts_with("memory_")),
        "desktop main must publish no namespace-scoped tool; published = {names:?}"
    );
}

/// ② **反证**: an out-of-scope call is refused at the scope gate, so the memory
/// module is never entered (the frame says *scope*, not *namespace*).
#[tokio::test]
async fn an_out_of_scope_memory_call_never_reaches_the_module() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let provider = Provider::new(vec![
        vec![call("memory_retrieve", serde_json::json!({"query":"probe"}))],
        vec![],
    ]);
    let modules = production(db);
    let policy = modules.desktop_agent_policy().unwrap();
    let runtime =
        build_desktop_main_runtime(modules.register_into(builder(provider.clone())), policy)
            .await
            .unwrap();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "probe"))
        .await
        .unwrap();

    let results: Vec<String> = provider
        .requests()
        .iter()
        .flat_map(|r| r.messages.iter())
        .filter(|m| m.role == MessageRole::Tool)
        .map(|m| ContentPart::join_text(&m.content))
        .collect();
    let frame = results
        .iter()
        .find(|text| text.contains("memory") || text.contains("scope"))
        .expect("the out-of-scope call must be answered with a frame");
    assert!(
        !frame.contains("Host principal is required"),
        "the memory module must not be entered at all; frame = {frame}"
    );
}

/// ③ **非反证**: the governed memory path (behavior module recall into the
/// transcript) keeps working — the fix removes the *tool face*, not the engine.
#[tokio::test]
async fn the_desktop_memory_behavior_path_still_works() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let session = SessionId::new();
    EpisodeStore::put_episode(
        db.as_ref(),
        &Episode {
            id: "desktop_recall_episode".into(),
            timestamp: SystemClock.now().timestamp(),
            session_id: session.to_string(),
            role: "user".into(),
            content: "desktop_recall_marker".into(),
        },
    )
    .unwrap();
    let provider = Provider::new(vec![]);
    let modules = production(db);
    let policy = modules.desktop_agent_policy().unwrap();
    let runtime =
        build_desktop_main_runtime(modules.register_into(builder(provider.clone())), policy)
            .await
            .unwrap();
    runtime
        .execute(TurnRequest::new(session, "desktop_recall_marker"))
        .await
        .unwrap();

    let text = provider.requests().first().map(tool_text).unwrap_or_default();
    assert!(
        text.contains("desktop_recall_marker"),
        "the governed memory recall path must still inject the episode; tool text = {text:?}"
    );
}

/// ④ **非反证**: the memory tool face itself is intact — a Host-bound identity
/// plus the module grant still operates it. The defect was the *unauthorized
/// publication*, not the capability.
#[tokio::test]
async fn a_host_bound_identity_still_operates_the_memory_tools() {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let owner = principal();
    let provider = Provider::new(vec![
        vec![call(
            "memory_write",
            serde_json::json!({"content":"host_bound_write","idempotency_key":"hb1"}),
        )],
        vec![],
    ]);
    let modules = production(db);
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        PluginId::new("module.memory").unwrap(),
        ModuleAgentGrant {
            tools: [cid("tool.memory.write"), cid("tool.memory.retrieve")]
                .into_iter()
                .collect(),
            principals: vec![owner.clone()],
            ..Default::default()
        },
    );
    let runtime = modules
        .register_into(builder(provider.clone()))
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let agent = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(owner),
                tools: vec![cid("tool.memory.write")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    agent.run("write it".into()).await.unwrap();

    let results = provider.results();
    assert!(
        results.iter().any(|text| text.contains("\"revision\"")),
        "a Host-bound identity must still be able to write; tool results = {results:?}"
    );
    assert!(
        !results.iter().any(|text| text.contains("namespace_denied")),
        "the Host-bound path must not be refused; tool results = {results:?}"
    );
}

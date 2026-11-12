//! Actual canonical side-calls, durable input markers and same-turn evidence.
//! The provider is deterministic instrumentation, not a real-model score.

use std::path::Path;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{
    CapabilityId, Clock, Episode, ModelId, NamespaceId, PluginId, ProfileId, RequestId, SessionId,
    Timestamp, TraceId, VirtualClock,
};
use apeireth_governance::AllowAll;
use apeireth_memory::writeback_recovery::SqliteWritebackJournal;
use apeireth_memory::{
    EpisodeStore, MemoryCoordinator, MemoryGovernanceStore, MemoryProvenance, MemoryScope,
    SqliteMemoryStore,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, NormalizedFinishReason, NormalizedMessage,
    NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::module::{
    BehaviorModule, CommittedInputOrigin, CommittedUserInput, HookPoint, InvocationContext,
    ModuleContext, ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse,
    ModuleInvoker,
};
use apeireth_runtime::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use apeireth_runtime::canonical::{Runtime, TurnRequest};
use apeireth_runtime_assembly::canonical::cognitive::{MemoryRecallModule, MemoryWritebackModule};
use apeireth_runtime_assembly::canonical::memory_correction::{
    MemoryCorrectionModule, CORRECTION_PROPOSAL_SYSTEM,
};
use apeireth_runtime_assembly::SqliteSessionStore;
use apeireth_storage::SqliteConnectionPool;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use tempfile::tempdir;

const MODEL: &str = "correction-test-model";

struct SemanticProvider {
    id: CapabilityId,
    instance: SessionId,
    proposal: Value,
    calls: Mutex<Vec<(bool, NormalizedRequest)>>,
}
impl SemanticProvider {
    fn new(proposal: Value) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.correction-test").unwrap(),
            instance: SessionId::new(),
            proposal,
            calls: Mutex::new(vec![]),
        })
    }
    fn calls(&self) -> Vec<(bool, NormalizedRequest)> {
        self.calls.lock().unwrap().clone()
    }
}
#[async_trait]
impl ProviderCapability for SemanticProvider {
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
        let side = request.messages.iter().any(|m| {
            m.role == MessageRole::System
                && ContentPart::join_text(&m.content) == CORRECTION_PROPOSAL_SYSTEM
        });
        let mut proposal = self.proposal.clone();
        if side && proposal["target_id"] == "$candidate" {
            let text = request
                .messages
                .iter()
                .rev()
                .find(|m| m.role == MessageRole::User)
                .map(|m| ContentPart::join_text(&m.content))
                .unwrap();
            let input: Value = serde_json::from_str(&text).unwrap();
            proposal["target_id"] = input["candidates"][0]["id"].clone();
        }
        let mut calls = self.calls.lock().unwrap();
        let number = calls.len();
        calls.push((side, request.clone()));
        Ok(NormalizedResponse {
            id: format!("response-{}-{number}", self.instance),
            model: request.model.clone(),
            content: if side {
                proposal.to_string()
            } else {
                "已收到。".into()
            },
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
                ..Default::default()
            },
            tool_calls: vec![],
            raw_metadata: Default::default(),
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<SemanticProvider>,
}
impl ProviderPlugin {
    fn new(provider: Arc<SemanticProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("plugin.correction-test").unwrap(),
                "1.0.0",
                "Deterministic semantic proposal instrumentation",
            )
            .declare_capability(
                provider.id.clone(),
                CapabilityKind::Provider,
                "proposal and primary completion",
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
        vec![self.provider.clone()]
    }
}

async fn runtime(
    root: &Path,
    provider: Arc<SemanticProvider>,
    clock: Arc<dyn Clock>,
) -> (Runtime, Arc<SqliteMemoryStore>, Arc<MemoryCorrectionModule>) {
    let path = root.join("memory.sqlite3");
    let store = Arc::new(SqliteMemoryStore::open(&path).unwrap());
    let coordinator = Arc::new(
        MemoryCoordinator::new(store.clone(), store.clone()).with_scoped_backend(store.clone()),
    );
    let pool = Arc::new(SqliteConnectionPool::open(&path).await.unwrap());
    let journal = Arc::new(SqliteWritebackJournal::from_pool(pool));
    let correction = Arc::new(
        MemoryCorrectionModule::new(coordinator.clone(), clock.clone()).with_journal(journal),
    );
    let sessions = Arc::new(
        SqliteSessionStore::open(root.join("sessions.sqlite3"))
            .await
            .unwrap(),
    );
    let runtime = Runtime::builder()
        .with_clock(clock.clone())
        .with_session_store(sessions)
        .with_governance(Arc::new(AllowAll))
        .with_plugin(ProviderPlugin::new(provider))
        .with_default_model(MODEL)
        .with_registered_behavior(correction.clone())
        .with_registered_behavior(Arc::new(
            MemoryRecallModule::new(store.clone()).with_coordinator(coordinator.clone()),
        ))
        .with_registered_behavior(Arc::new(
            MemoryWritebackModule::new(store.clone(), clock).with_coordinator(coordinator),
        ))
        .build()
        .await
        .unwrap();
    (runtime, store, correction)
}

fn proposal(anchor: &str, evidence: &str, old: &str, new: &str) -> Value {
    json!({"decision":"correct_prior","target_id":"$candidate","subject":"self","source_kind":"direct_assertion","evidence":evidence,"target_anchor":anchor,"old_fragment":old,"new_fragment":new,"effective_time":null})
}
fn memory_text(request: &NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .map(|m| ContentPart::join_text(&m.content))
        .collect::<Vec<_>>()
        .join("\n")
}
fn clock() -> Arc<VirtualClock> {
    Arc::new(VirtualClock::new(
        Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0)
            .single()
            .unwrap(),
    ))
}

#[tokio::test]
async fn open_chinese_first_person_and_possessive_corrections_reach_the_same_primary_request() {
    for (old, input, anchor, evidence, before, after, kind) in [
        (
            "请记住这些真实信息：我叫林禾，我住在北岚市。",
            "更正：我住在南汀市。请立即告诉我林禾目前居住在哪座城市？",
            "我住在北岚市",
            "我住在南汀市",
            "北岚市",
            "南汀市",
            "correct_prior",
        ),
        (
            "我的地址是北京。",
            "纠正:我的地址是杭州。请告诉我现行地址。",
            "我的地址是北京",
            "纠正:我的地址是杭州",
            "北京",
            "杭州",
            "correct_prior",
        ),
        (
            "我住在武汉。",
            "我其实已经搬到上海了，请按现在的住处回答。",
            "我住在武汉",
            "我其实已经搬到上海了",
            "武汉",
            "上海",
            "state_change",
        ),
        (
            "本人常住武汉。",
            "本人纠正此前登记，实际常住地是成都。",
            "本人常住武汉",
            "本人纠正此前登记，实际常住地是成都",
            "武汉",
            "成都",
            "correct_prior",
        ),
        (
            "我的地址是北京。",
            "我的地址更正为上海，请说出最新地址。",
            "我的地址是北京",
            "我的地址更正为上海",
            "北京",
            "上海",
            "correct_prior",
        ),
        (
            "我的居住城市是北京。",
            "我的居住城市已经换成苏州。",
            "我的居住城市是北京",
            "我的居住城市已经换成苏州",
            "北京",
            "苏州",
            "state_change",
        ),
        (
            "我的名字是小林。",
            "我的姓名更正为林青。",
            "我的名字是小林",
            "我的姓名更正为林青",
            "小林",
            "林青",
            "correct_prior",
        ),
        (
            "我的邮箱是old@example.test。",
            "本人的电子邮箱已换成new@example.test。",
            "我的邮箱是old@example.test",
            "本人的电子邮箱已换成new@example.test",
            "old@example.test",
            "new@example.test",
            "state_change",
        ),
    ] {
        let dir = tempdir().unwrap();
        let mut proposed = proposal(anchor, evidence, before, after);
        proposed["decision"] = json!(kind);
        let provider = SemanticProvider::new(proposed);
        let clock = clock();
        let (runtime, store, module) = runtime(dir.path(), provider.clone(), clock.clone()).await;
        let session = SessionId::new();
        runtime
            .execute(TurnRequest::new(session, old))
            .await
            .unwrap();
        let original = EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 20)
            .unwrap()
            .into_iter()
            .find(|ep| ep.role == "user" && ep.content == old)
            .unwrap();
        clock.advance(chrono::Duration::days(2));
        runtime
            .execute(TurnRequest::new(session, input))
            .await
            .unwrap();
        assert_eq!(module.metrics().proposals, 1, "{input}");
        assert_eq!(module.metrics().applied, 1, "{input}");
        let current = store.get_governed(&original.id).unwrap().unwrap();
        assert!(
            current.episode.content.contains(after),
            "{input}: {}",
            current.episode.content
        );
        assert!(!current.episode.content.contains(before), "{input}");
        assert_eq!(
            current.episode.content,
            old.replacen(before, after, 1),
            "unrelated source facts must remain intact: {input}"
        );
        assert_eq!(
            EpisodeStore::get_episode(store.as_ref(), &original.id)
                .unwrap()
                .unwrap()
                .content,
            old
        );
        let calls = provider.calls();
        assert_eq!(calls.iter().filter(|(side, _)| *side).count(), 1);
        let side = &calls.iter().find(|(side, _)| *side).unwrap().1;
        let proposed_input: Value = serde_json::from_str(&ContentPart::join_text(
            &side
                .messages
                .iter()
                .rev()
                .find(|message| message.role == MessageRole::User)
                .unwrap()
                .content,
        ))
        .unwrap();
        assert_eq!(
            proposed_input["input"], input,
            "semantic proposal must inspect the actual committed source"
        );
        let primary = &calls.iter().rev().find(|(side, _)| !*side).unwrap().1;
        apeireth_protocol::canonical::validate_closed_tool_context(&primary.messages).unwrap();
        let delivered = memory_text(primary);
        assert!(delivered.contains(&original.id), "{input}: {delivered}");
        assert!(
            delivered.contains(&current.episode.content),
            "same-turn current fact missing: {delivered}"
        );
    }
}

#[tokio::test]
async fn current_correction_and_its_source_marker_survive_full_runtime_rebuild() {
    let dir = tempdir().unwrap();
    let session = SessionId::new();
    let first = SemanticProvider::new(proposal(
        "我的地址是北京",
        "我的地址更正为上海",
        "北京",
        "上海",
    ));
    let (first_runtime, store, _) = runtime(dir.path(), first, clock()).await;
    first_runtime
        .execute(TurnRequest::new(session, "我的地址是北京。"))
        .await
        .unwrap();
    first_runtime
        .execute(TurnRequest::new(session, "我的地址更正为上海。"))
        .await
        .unwrap();
    drop(first_runtime);
    drop(store);
    let provider = SemanticProvider::new(proposal(
        "我的地址是上海",
        "我的地址更正为广州",
        "上海",
        "广州",
    ));
    let (second_runtime, second_store, module) =
        runtime(dir.path(), provider.clone(), clock()).await;
    second_runtime
        .execute(TurnRequest::new(session, "我的地址是什么？"))
        .await
        .unwrap();
    assert_eq!(
        module.metrics().proposals,
        0,
        "ordinary questions must not spend a semantic call"
    );
    let calls = provider.calls();
    assert_eq!(calls.len(), 1);
    assert!(memory_text(&calls[0].1).contains("我的地址是上海"));
    second_runtime
        .execute(TurnRequest::new(session, "我的地址更正为广州。"))
        .await
        .unwrap();
    assert_eq!(
        module.metrics().applied,
        1,
        "restarted source verification must still authorize the original owned fact"
    );
    let revised = second_store
        .governed_recent_episodes(&session.to_string(), 40)
        .unwrap();
    assert!(revised
        .iter()
        .any(|row| row.episode.content == "我的地址是广州。" && row.revision == 2));
}

#[tokio::test]
async fn changed_state_history_and_current_fact_both_reach_ordinary_recall_after_restart() {
    let dir = tempdir().unwrap();
    let session = SessionId::new();
    let clock = clock();
    let mut response = proposal("我住在武汉", "我昨天搬到上海", "武汉", "上海");
    response["decision"] = json!("state_change");
    response["effective_time"] = json!("昨天");
    let provider = SemanticProvider::new(response);
    let (first_runtime, store, module) = runtime(dir.path(), provider, clock.clone()).await;
    first_runtime
        .execute(TurnRequest::new(session, "我住在武汉。"))
        .await
        .unwrap();
    let original = EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 20)
        .unwrap()
        .into_iter()
        .find(|ep| ep.role == "user" && ep.content == "我住在武汉。")
        .unwrap();
    clock.advance(chrono::Duration::days(2));
    first_runtime
        .execute(TurnRequest::new(
            session,
            "我昨天搬到上海，请分别说出搬家之前与现在的住处。",
        ))
        .await
        .unwrap();
    assert_eq!(module.metrics().applied, 1);
    let history = EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 40)
        .unwrap()
        .into_iter()
        .find(|ep| ep.role == "fact_history" && ep.content.ends_with("我住在武汉。"))
        .unwrap();
    assert!(
        history
            .content
            .contains("[2026-11-10T12:00:00.000Z, 2026-11-11T00:00:00.000Z)"),
        "{}",
        history.content
    );
    drop(first_runtime);
    drop(store);
    drop(module);
    let provider = SemanticProvider::new(json!({"decision":"abstain"}));
    let (second_runtime, store, module) = runtime(dir.path(), provider.clone(), clock).await;
    second_runtime
        .execute(TurnRequest::new(
            session,
            "分别告诉我搬家之前与现在的住处，并说明变化日期。",
        ))
        .await
        .unwrap();
    assert_eq!(module.metrics().proposals, 0);
    let calls = provider.calls();
    assert_eq!(calls.len(), 1);
    let delivered = memory_text(&calls[0].1);
    assert!(
        delivered.contains(&history.id),
        "historical evidence missing: {delivered}"
    );
    assert!(
        delivered.contains(&history.content),
        "historical interval missing: {delivered}"
    );
    let current = store.get_governed(&original.id).unwrap().unwrap();
    assert!(
        delivered.contains(&current.episode.content),
        "current evidence missing: {delivered}"
    );
    assert_eq!(current.episode.content, "我住在上海。");
}

#[tokio::test]
async fn chinese_absolute_date_and_adjacent_self_assertions_preserve_history_and_unrelated_name() {
    let old = "请记住这些真实信息：我叫林禾，我住在北岚市。";
    for (date, month, day) in [("2026年11月10日", 11, 10), ("2026年2月9日", 2, 9)] {
        let dir = tempdir().unwrap();
        let session = SessionId::new();
        let clock = Arc::new(VirtualClock::new(
            Utc.with_ymd_and_hms(2026, month, 1, 12, 0, 0)
                .single()
                .unwrap(),
        ));
        let input = format!("我于{date}搬到了新家。我住在南汀市。");
        let mut response = proposal("我住在北岚市", &input, "北岚市", "南汀市");
        response["decision"] = json!("state_change");
        response["effective_time"] = json!(date);
        let provider = SemanticProvider::new(response);
        let (runtime, store, module) = runtime(dir.path(), provider.clone(), clock.clone()).await;
        runtime
            .execute(TurnRequest::new(session, old))
            .await
            .unwrap();
        let original = EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 20)
            .unwrap()
            .into_iter()
            .find(|episode| episode.role == "user" && episode.content == old)
            .unwrap();
        clock.set(
            Utc.with_ymd_and_hms(2026, month, 12, 12, 0, 0)
                .single()
                .unwrap(),
        );
        runtime
            .execute(TurnRequest::new(session, &input))
            .await
            .unwrap();
        assert_eq!(
            module.metrics().applied,
            1,
            "{date}: {:?}",
            module.metrics()
        );
        let current = store.get_governed(&original.id).unwrap().unwrap();
        assert_eq!(current.episode.content, old.replace("北岚市", "南汀市"));
        assert_eq!(current.revision, 1);
        assert_eq!(
            EpisodeStore::get_episode(store.as_ref(), &original.id)
                .unwrap()
                .unwrap()
                .content,
            old
        );
        let effective = Utc
            .with_ymd_and_hms(2026, month, day, 0, 0, 0)
            .single()
            .unwrap();
        let history = EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 40)
            .unwrap()
            .into_iter()
            .find(|episode| episode.role == "fact_history")
            .unwrap();
        assert!(
            history
                .content
                .contains(&effective.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
            "{}",
            history.content
        );
        assert!(history.content.ends_with(old));
        let calls = provider.calls();
        let primary = &calls.iter().rev().find(|(side, _)| !*side).unwrap().1;
        let delivered = memory_text(primary);
        assert!(delivered.contains(&current.episode.content), "{delivered}");
        assert!(delivered.contains(&history.content), "{delivered}");
        let temporal =
            apeireth_memory::SqliteTemporalGraphStore::open(dir.path().join("memory.sqlite3"))
                .await
                .unwrap();
        let relation = apeireth_memory::automatic_correction::correction_relation_key(
            &session.to_string(),
            &original.id,
        );
        let before = temporal
            .query(
                &apeireth_memory::TemporalGraphQuery::new(effective.timestamp_millis() - 1)
                    .relation(&relation),
            )
            .unwrap();
        let after = temporal
            .query(
                &apeireth_memory::TemporalGraphQuery::new(effective.timestamp_millis())
                    .relation(&relation),
            )
            .unwrap();
        assert_eq!(before[0].object_id, old);
        assert_eq!(after[0].object_id, current.episode.content);
    }
}

#[tokio::test]
async fn explicit_date_cannot_be_borrowed_from_another_property_person_or_outside_evidence() {
    let old = "请记住这些真实信息：我叫林禾，我住在北岚市。";
    let valid = "我于2026年11月10日搬到了新家。我住在南汀市。";
    for (input, evidence, date) in [
        (valid, "我住在南汀市", "2026年11月10日"),
        (valid, valid, "2026年11月9日"),
        (
            "我于2026年11月10日换了邮箱。我住在南汀市。",
            "我于2026年11月10日换了邮箱。我住在南汀市。",
            "2026年11月10日",
        ),
        (
            "同事于2026年11月10日搬到了新家。我住在南汀市。",
            "同事于2026年11月10日搬到了新家。我住在南汀市。",
            "2026年11月10日",
        ),
        (
            "我于2026年2月30日搬到了新家。我住在南汀市。",
            "我于2026年2月30日搬到了新家。我住在南汀市。",
            "2026年2月30日",
        ),
    ] {
        let dir = tempdir().unwrap();
        let session = SessionId::new();
        let clock = Arc::new(VirtualClock::new(
            Utc.with_ymd_and_hms(2026, 11, 1, 12, 0, 0)
                .single()
                .unwrap(),
        ));
        let mut response = proposal("我住在北岚市", evidence, "北岚市", "南汀市");
        response["decision"] = json!("state_change");
        response["effective_time"] = json!(date);
        let provider = SemanticProvider::new(response);
        let (runtime, store, module) = runtime(dir.path(), provider, clock.clone()).await;
        runtime
            .execute(TurnRequest::new(session, old))
            .await
            .unwrap();
        let original = EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 20)
            .unwrap()
            .into_iter()
            .find(|episode| episode.role == "user" && episode.content == old)
            .unwrap();
        clock.set(
            Utc.with_ymd_and_hms(2026, 11, 12, 12, 0, 0)
                .single()
                .unwrap(),
        );
        runtime
            .execute(TurnRequest::new(session, input))
            .await
            .unwrap();
        assert_eq!(module.metrics().applied, 0, "{input} / {evidence} / {date}");
        let current = store.get_governed(&original.id).unwrap().unwrap();
        assert_eq!(current.episode.content, old);
        assert_eq!(current.revision, 0);
        assert!(
            EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 40)
                .unwrap()
                .iter()
                .all(|episode| episode.role != "fact_history")
        );
    }
}

#[tokio::test]
async fn quoted_third_party_reported_hypothetical_and_future_text_cannot_mutate_self() {
    for input in [
        "我的朋友已经搬到上海。",
        "他说：我住在上海。",
        "引用：我住在上海。",
        "引用：更正：我住在上海。",
        "假设：我住在上海。",
        "更正：“我住在上海。”",
        "更正：如果我住在上海，请修改地址。",
        "更正：我的朋友住在上海。",
        "我的同事说，我住在上海。",
        "本人的孩子已经搬到上海。",
        "我邻居已经搬到上海。",
        "本人配偶已经搬到上海。",
        "我之前的朋友已经搬到上海。",
        "我现在的同事已经搬到上海。",
        "我们已经搬到上海。",
        "邻居表示，我已经搬到上海。",
        "我同事把地址改成上海。",
        "请读这句话：“我的地址更正为上海”。",
        "> 我的地址更正为上海。",
        "```我的地址更正为上海```",
        "如果我搬到上海，请更正地址。",
        "假设有一天，我搬到上海。",
        "我可能已经搬到上海。",
        "我明天计划搬到上海。",
    ] {
        let dir = tempdir().unwrap();
        let provider =
            SemanticProvider::new(proposal("我的地址是北京", "我住在上海", "北京", "上海"));
        let (runtime, store, module) = runtime(dir.path(), provider.clone(), clock()).await;
        let session = SessionId::new();
        runtime
            .execute(TurnRequest::new(session, "我的地址是北京。"))
            .await
            .unwrap();
        let id = EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 20)
            .unwrap()
            .into_iter()
            .find(|ep| ep.role == "user")
            .unwrap()
            .id;
        runtime
            .execute(TurnRequest::new(session, input))
            .await
            .unwrap();
        assert_eq!(
            store.get_governed(&id).unwrap().unwrap().episode.content,
            "我的地址是北京。",
            "{input}"
        );
        assert_eq!(module.metrics().applied, 0, "{input}");
        assert_eq!(
            provider.calls().iter().filter(|(side, _)| *side).count(),
            0,
            "untrusted assertion should be rejected before proposal: {input}"
        );
    }
}

// This port fixture tests the correction module's rejection boundary. Runtime's
// production creation and parsing of the marker are covered by its own tests.
struct SourcePort {
    current: CommittedUserInput,
    old: Option<CommittedUserInput>,
}
#[async_trait]
impl ModuleInvoker for SourcePort {
    async fn committed_input(&self) -> Result<Option<CommittedUserInput>, ModuleInvocationError> {
        Ok(Some(self.current.clone()))
    }
    async fn committed_input_for(
        &self,
        session: SessionId,
        request: RequestId,
    ) -> Result<Option<CommittedUserInput>, ModuleInvocationError> {
        Ok(self
            .old
            .clone()
            .filter(|input| input.session == session && input.request == request))
    }
    async fn invoke(
        &self,
        _request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
        panic!("a rejected source must never spend a semantic provider call")
    }
}
struct NoSubLoop;
#[async_trait]
impl SubLoopSpawner for NoSubLoop {
    async fn spawn(&self, _spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        Err(SubLoopError::NoModel)
    }
}

#[tokio::test]
async fn role_strings_tasks_foreign_owners_and_legacy_sources_cannot_supply_correction_authority() {
    for refusal in [
        "current-task",
        "assistant",
        "tool",
        "old-task",
        "old-assistant",
        "old-foreign",
        "legacy",
    ] {
        let dir = tempdir().unwrap();
        let store = Arc::new(SqliteMemoryStore::open(dir.path().join("memory.sqlite3")).unwrap());
        let coordinator = Arc::new(MemoryCoordinator::new(store.clone(), store.clone()));
        let clock = clock();
        let session = SessionId::new();
        let old_request = RequestId::new();
        coordinator
            .writeback_scoped_episode(
                &Episode {
                    id: "verified-target".into(),
                    session_id: session.to_string(),
                    timestamp: clock.now().timestamp(),
                    role: "user".into(),
                    content: "我的地址是北京。".into(),
                },
                &MemoryScope::session(session.to_string()),
                &MemoryProvenance {
                    source: "cognitive.memory_writeback".into(),
                    source_session: Some(session.to_string()),
                    source_request: Some(old_request.to_string()),
                    source_trace: Some(TraceId::new().to_string()),
                },
            )
            .unwrap();
        let mut current = CommittedUserInput {
            session,
            request: RequestId::new(),
            trace: TraceId::new(),
            principal: None,
            authorization_epoch: None,
            at: Timestamp::from_clock(clock.as_ref()),
            messages: vec![NormalizedMessage::user("我的地址更正为上海。")],
            origin: CommittedInputOrigin::DirectUser,
        };
        let mut old = CommittedUserInput {
            request: old_request,
            messages: vec![NormalizedMessage::user("我的地址是北京。")],
            ..current.clone()
        };
        match refusal {
            "current-task" => current.origin = CommittedInputOrigin::AgentTask,
            "assistant" => current.messages[0].role = MessageRole::Assistant,
            "tool" => current.messages[0].role = MessageRole::Tool,
            "old-task" => old.origin = CommittedInputOrigin::AgentTask,
            "old-assistant" => old.messages[0].role = MessageRole::Assistant,
            "old-foreign" => {
                old.principal = Some(apeireth_plugin::agents::AgentPrincipal {
                    profile: ProfileId::new(),
                    namespace: NamespaceId::new(),
                })
            }
            "legacy" => {}
            _ => unreachable!(),
        }
        let pool = Arc::new(
            SqliteConnectionPool::open(dir.path().join("memory.sqlite3"))
                .await
                .unwrap(),
        );
        let module = MemoryCorrectionModule::new(coordinator, clock)
            .with_journal(Arc::new(SqliteWritebackJournal::from_pool(pool)));
        let invoker: Arc<dyn ModuleInvoker> = Arc::new(SourcePort {
            current,
            old: (refusal != "legacy").then_some(old),
        });
        let invocation = InvocationContext::user_turn();
        let subloop = NoSubLoop;
        let ctx = ModuleContext {
            session_id: &session,
            model: MODEL,
            messages: &[],
            candidate: None,
            tool_call: None,
            tool_result: None,
            invocation: &invocation,
            module_id: "cognitive.memory_correction",
            error: None,
            invoker: invoker.as_ref(),
            invoker_handle: invoker.clone(),
            subloop: &subloop,
        };
        module.on_hook(HookPoint::TurnStart, &ctx).await.unwrap();
        assert_eq!(module.metrics().proposals, 0, "{refusal}");
        assert_eq!(module.metrics().applied, 0, "{refusal}");
        assert_eq!(
            store
                .get_governed("verified-target")
                .unwrap()
                .unwrap()
                .episode
                .content,
            "我的地址是北京。",
            "{refusal}"
        );
        if refusal.starts_with("old-") || refusal == "legacy" {
            assert!(module.metrics().unverified_sources > 0, "{refusal}");
        }
    }
}

#[tokio::test]
async fn semantic_output_cannot_choose_an_unlisted_target_or_rewrite_an_unrelated_property() {
    for forged in ["target", "property", "source", "date", "ambiguous"] {
        let dir = tempdir().unwrap();
        let mut response = proposal("我的地址是北京", "我的地址更正为上海", "北京", "上海");
        let input = match forged {
            "target" => {
                response["target_id"] = json!("foreign-record");
                "我的地址更正为上海。"
            }
            "property" => {
                response["evidence"] = json!("我的姓名更正为上海");
                "我的姓名更正为上海。"
            }
            "source" => {
                response["new_fragment"] = json!("广州");
                "我的地址更正为上海。"
            }
            "date" => {
                response["decision"] = json!("state_change");
                response["evidence"] = json!("我上周五搬到上海");
                response["effective_time"] = json!("上周五");
                "我上周五搬到上海。"
            }
            "ambiguous" => {
                response = json!({"decision":"abstain"});
                "我想更正那个条目。"
            }
            _ => unreachable!(),
        };
        let provider = SemanticProvider::new(response);
        let (runtime, store, module) = runtime(dir.path(), provider, clock()).await;
        let session = SessionId::new();
        runtime
            .execute(TurnRequest::new(session, "我的地址是北京。"))
            .await
            .unwrap();
        let id = EpisodeStore::recent_episodes(store.as_ref(), &session.to_string(), 20)
            .unwrap()
            .into_iter()
            .find(|ep| ep.role == "user")
            .unwrap()
            .id;
        runtime
            .execute(TurnRequest::new(session, input))
            .await
            .unwrap();
        assert_eq!(module.metrics().applied, 0, "{forged}");
        assert_eq!(
            store.get_governed(&id).unwrap().unwrap().revision,
            0,
            "{forged}"
        );
    }
}

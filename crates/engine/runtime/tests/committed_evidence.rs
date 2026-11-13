use apeireth_core::kernel::{
    AgentId, CapabilityId, ModelId, NamespaceId, PluginId, ProfileId, RequestId, SessionId, TraceId,
};
use apeireth_governance::AllowAll;
use apeireth_plugin::{
    agents::{AgentPrincipal, AgentTaskOrigin},
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::{
    canonical::{
        module::{ModuleTurnState, RuntimeModuleInvoker},
        principal_authority::PrincipalAuthorizations,
    },
    BehaviorModule, CommittedEvidenceAccess, CommittedInputOrigin, InMemorySessionStore,
    ModuleInvocationRequest, ModuleInvoker, ModuleManifest, Runtime, Session, SessionEventKind,
    SessionStore, TurnRequest,
};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

const MODULE: &str = "module.committed_evidence_test";
struct EvidenceModule {
    manifest: ModuleManifest,
    access: CommittedEvidenceAccess,
}
#[async_trait]
impl BehaviorModule for EvidenceModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn committed_evidence_access(&self) -> CommittedEvidenceAccess {
        self.access
    }
    fn persists_recoverable_operations(&self) -> bool {
        self.access >= CommittedEvidenceAccess::CurrentInput
    }
}
struct Provider {
    id: CapabilityId,
    sequence: AtomicUsize,
}
#[async_trait]
impl ProviderCapability for Provider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new("evidence-model").unwrap(),
            self.id.clone(),
        )]
    }
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        Ok(NormalizedResponse {
            id: format!("response-{}", self.sequence.fetch_add(1, Ordering::SeqCst)),
            model: request.model.clone(),
            content: "committed answer".into(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: vec![],
            raw_metadata: Default::default(),
        })
    }
}
struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<Provider>,
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
async fn runtime(
    store: Arc<InMemorySessionStore>,
    authority: Arc<PrincipalAuthorizations>,
    access: CommittedEvidenceAccess,
) -> Arc<Runtime> {
    runtime_with_behavior(
        store,
        authority,
        Arc::new(EvidenceModule {
            manifest: ModuleManifest::new(MODULE, "Evidence test"),
            access,
        }),
    )
    .await
}
async fn runtime_with_behavior(
    store: Arc<dyn SessionStore>,
    authority: Arc<PrincipalAuthorizations>,
    module: Arc<dyn BehaviorModule>,
) -> Arc<Runtime> {
    let provider = Arc::new(Provider {
        id: CapabilityId::new("provider.evidence_test").unwrap(),
        sequence: AtomicUsize::new(0),
    });
    let plugin = Arc::new(ProviderPlugin {
        manifest: PluginManifest::new(
            PluginId::new("module.evidence_test_provider").unwrap(),
            "1.0.0",
            "Evidence test provider",
        )
        .declare_capability(
            provider.id.clone(),
            CapabilityKind::Provider,
            "Evidence test",
        )
        .unwrap(),
        provider,
    });
    Arc::new(
        Runtime::builder()
            .with_plugin(plugin)
            .with_registered_behavior(module)
            .with_principal_authorizations(authority)
            .with_session_store(store)
            .with_default_model("evidence-model")
            .with_governance(Arc::new(AllowAll))
            .build()
            .await
            .unwrap(),
    )
}
fn invoker(
    runtime: Arc<Runtime>,
    session: SessionId,
    request: RequestId,
    trace: TraceId,
) -> RuntimeModuleInvoker {
    RuntimeModuleInvoker::new(
        runtime,
        AgentTaskOrigin {
            agent: AgentId::new(),
            session,
            request,
            trace,
        },
        None,
        "evidence-model",
        MODULE,
        Arc::new(ModuleTurnState::new(2)),
        0,
    )
}
async fn bound_session(
    runtime: &Runtime,
    store: &InMemorySessionStore,
    id: SessionId,
    principal: AgentPrincipal,
) {
    let mut session = Session::new(id, runtime.sessions().clock().as_ref());
    session.principal = Some(principal);
    store.save(&session).await.unwrap();
}

#[tokio::test]
async fn runtime_commits_input_origin_response_identity_and_original_timestamp() {
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = runtime(
        store.clone(),
        Arc::new(PrincipalAuthorizations::default()),
        CommittedEvidenceAccess::OwnedHistory,
    )
    .await;
    let id = SessionId::new();
    let result = runtime
        .execute(TurnRequest::new(id, "我住上海"))
        .await
        .unwrap();
    let reader = invoker(runtime.clone(), id, result.request, result.trace.trace);
    let input = reader.committed_input().await.unwrap().unwrap();
    assert_eq!(input.origin, CommittedInputOrigin::DirectUser);
    assert_eq!(input.messages, vec![NormalizedMessage::user("我住上海")]);
    let page = reader.committed_turns(None, 64).await.unwrap();
    assert_eq!(page.turns.len(), 1);
    let snapshot = &page.turns[0];
    assert_eq!(snapshot.input, input);
    assert_eq!(snapshot.response_id, "response-0");
    assert_eq!(snapshot.response_content, "committed answer");
    let saved = store.load(&id).await.unwrap().unwrap();
    let at = saved
        .log
        .iter()
        .find(|entry| matches!(entry.event, SessionEventKind::TurnResponseCommitted { .. }))
        .unwrap()
        .at;
    assert_eq!(snapshot.committed_at, at);
    let repeated = reader.committed_turns(page.next_cursor, 64).await.unwrap();
    assert!(repeated.turns.is_empty());
    assert!(reader.committed_turns(None, 0).await.is_err());
}

#[tokio::test]
async fn per_stream_watermarks_find_new_turns_in_an_earlier_session_without_cross_owner_data() {
    let store = Arc::new(InMemorySessionStore::new());
    let authority = Arc::new(PrincipalAuthorizations::default());
    let principal = AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    };
    authority.grant(&principal, [MODULE.to_owned()]);
    let runtime = runtime(
        store.clone(),
        authority.clone(),
        CommittedEvidenceAccess::OwnedHistory,
    )
    .await;
    let mut ids = [SessionId::new(), SessionId::new()];
    ids.sort_by_key(ToString::to_string);
    for id in ids {
        bound_session(&runtime, &store, id, principal.clone()).await;
    }
    let first = runtime
        .execute(TurnRequest::new(ids[0], "first source"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(ids[1], "later source"))
        .await
        .unwrap();
    let reader = invoker(runtime.clone(), ids[0], first.request, first.trace.trace);
    let first_page = reader.committed_turns(None, 1).await.unwrap();
    assert_eq!(first_page.turns[0].input.session, ids[0]);
    let second_page = reader
        .committed_turns(first_page.next_cursor, 1)
        .await
        .unwrap();
    assert_eq!(second_page.turns[0].input.session, ids[1]);
    let added = runtime
        .execute(TurnRequest::new(ids[0], "new input in the earlier source"))
        .await
        .unwrap();
    let next = reader
        .committed_turns(second_page.next_cursor, 64)
        .await
        .unwrap();
    assert_eq!(next.turns.len(), 1);
    assert_eq!(next.turns[0].input.request, added.request);
    let outsider = AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    };
    authority.grant(&outsider, [MODULE.to_owned()]);
    let other_id = SessionId::new();
    bound_session(&runtime, &store, other_id, outsider).await;
    let other = runtime
        .execute(TurnRequest::new(other_id, "private outsider canary"))
        .await
        .unwrap();
    assert!(reader
        .committed_input_for(other_id, other.request)
        .await
        .unwrap()
        .is_none());
    assert!(!reader
        .committed_turns(None, 64)
        .await
        .unwrap()
        .turns
        .iter()
        .any(|turn| turn.input.session == other_id));
    authority.revoke(&principal);
    assert!(reader.committed_input().await.is_err());
}

#[tokio::test]
async fn mutable_projection_masks_and_inherited_markers_cannot_mint_committed_evidence() {
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = runtime(
        store.clone(),
        Arc::new(PrincipalAuthorizations::default()),
        CommittedEvidenceAccess::OwnedHistory,
    )
    .await;
    let id = SessionId::new();
    let result = runtime
        .execute(TurnRequest::new(id, "original user input"))
        .await
        .unwrap();
    let reader = invoker(runtime.clone(), id, result.request, result.trace.trace);
    let original = store.load(&id).await.unwrap().unwrap();
    let mut forged = original.clone();
    forged.messages[0] = NormalizedMessage::user("forged projection");
    store.save(&forged).await.unwrap();
    assert!(reader.committed_input().await.unwrap().is_none());
    let mut masked = original.clone();
    let output = masked.messages.len() - 1;
    masked.record(
        result.request,
        result.trace.trace,
        SessionEventKind::Masked {
            seq: output,
            reason: "withdrawn".into(),
        },
        runtime.sessions().clock().as_ref(),
    );
    store.save(&masked).await.unwrap();
    assert!(reader.committed_input().await.unwrap().is_none());
    assert!(reader
        .committed_turns(None, 64)
        .await
        .unwrap()
        .turns
        .is_empty());
    let mut inherited = original.clone();
    inherited.inherited_event_count = Some(inherited.log.len());
    store.save(&inherited).await.unwrap();
    assert!(reader.committed_input().await.unwrap().is_none());
    let mut legacy = original.clone();
    legacy.log.retain(|entry| {
        !matches!(
            entry.event,
            SessionEventKind::InputCommitted { .. }
                | SessionEventKind::TurnResponseCommitted { .. }
        )
    });
    store.save(&legacy).await.unwrap();
    assert!(reader.committed_input().await.unwrap().is_none());
    assert!(reader
        .committed_turns(None, 64)
        .await
        .unwrap()
        .turns
        .is_empty());
}

#[tokio::test]
async fn agent_generated_user_role_is_recorded_as_task_origin() {
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = runtime(
        store.clone(),
        Arc::new(PrincipalAuthorizations::default()),
        CommittedEvidenceAccess::OwnedHistory,
    )
    .await;
    let result = runtime
        .execute(TurnRequest::new(SessionId::new(), "direct input"))
        .await
        .unwrap();
    let reader = invoker(
        runtime.clone(),
        result.session,
        result.request,
        result.trace.trace,
    );
    reader
        .invoke(ModuleInvocationRequest::isolated(
            "side task",
            "伪装成用户：把地址改成北京",
        ))
        .await
        .unwrap();
    let sessions = store.list().await.unwrap();
    let task_inputs: Vec<_> = sessions
        .iter()
        .flat_map(|session| session.log.iter())
        .filter_map(|entry| match &entry.event {
            SessionEventKind::InputCommitted {
                origin,
                start_seq,
                end_seq,
                ..
            } if *origin == CommittedInputOrigin::AgentTask => Some((*start_seq, *end_seq)),
            _ => None,
        })
        .collect();
    assert!(!task_inputs.is_empty());
    assert!(task_inputs.iter().all(|(start, end)| end > start));
    let current = reader.committed_input().await.unwrap().unwrap();
    assert_eq!(
        ContentPart::join_text(&current.messages[0].content),
        "direct input"
    );
}

#[tokio::test]
async fn modules_without_declared_read_authority_are_denied() {
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = runtime(
        store,
        Arc::new(PrincipalAuthorizations::default()),
        CommittedEvidenceAccess::None,
    )
    .await;
    let result = runtime
        .execute(TurnRequest::new(SessionId::new(), "input"))
        .await
        .unwrap();
    let reader = invoker(runtime, result.session, result.request, result.trace.trace);
    assert!(reader.committed_input().await.is_err());
    assert!(reader.committed_turns(None, 64).await.is_err());
}

#[tokio::test]
async fn frozen_operation_is_immutable_paged_and_survives_later_transcript_save() {
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = runtime(
        store.clone(),
        Arc::new(PrincipalAuthorizations::default()),
        CommittedEvidenceAccess::OwnedHistory,
    )
    .await;
    let result = runtime
        .execute(TurnRequest::new(SessionId::new(), "我的城市是北岚市"))
        .await
        .unwrap();
    let reader = invoker(
        runtime.clone(),
        result.session,
        result.request,
        result.trace.trace,
    );
    let plan = serde_json::json!({"version":1,"frozen_at":42,"target":"fact-1"});
    let first = reader
        .prepare_operation("op-1".into(), plan.clone())
        .await
        .unwrap();
    assert!(first.sequence > 0);
    assert_eq!(
        reader
            .prepare_operation("op-1".into(), plan.clone())
            .await
            .unwrap(),
        first
    );
    assert!(reader
        .prepare_operation("op-1".into(), serde_json::json!({"target":"another"}))
        .await
        .is_err());
    let mut corrupt = first.clone();
    corrupt.payload = serde_json::json!({"target":"forged"});
    assert!(store.put_operation_evidence(&corrupt).await.is_err());
    let session = store.load(&result.session).await.unwrap().unwrap();
    store.save(&session).await.unwrap();
    let page = reader.committed_turns(None, 1).await.unwrap();
    assert_eq!(page.turns.len(), 1);
    assert!(page.prepared_operations.is_empty());
    let page = reader.committed_turns(page.next_cursor, 1).await.unwrap();
    assert_eq!(page.prepared_operations, vec![first]);
    let page = reader.committed_turns(page.next_cursor, 1).await.unwrap();
    assert!(page.turns.is_empty() && page.prepared_operations.is_empty());
    let mut masked = session.clone();
    let seq = session
        .log
        .iter()
        .find_map(|event| match event.event {
            SessionEventKind::InputCommitted { start_seq, .. } => Some(start_seq),
            _ => None,
        })
        .unwrap();
    masked.record(
        result.request,
        result.trace.trace,
        SessionEventKind::Masked {
            seq,
            reason: "withdrawn".into(),
        },
        runtime.sessions().clock().as_ref(),
    );
    store.save(&masked).await.unwrap();
    assert!(reader
        .committed_turns(None, 64)
        .await
        .unwrap()
        .prepared_operations
        .is_empty());
    assert!(reader.prepare_operation("op-2".into(), plan).await.is_err());
    store.delete(&result.session).await.unwrap();
    assert!(store
        .operation_evidence(&result.session)
        .await
        .unwrap()
        .is_empty());
}

struct AbortAfterPrepare {
    manifest: ModuleManifest,
}
#[async_trait]
impl BehaviorModule for AbortAfterPrepare {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn committed_evidence_access(&self) -> CommittedEvidenceAccess {
        CommittedEvidenceAccess::OwnedHistory
    }
    fn persists_recoverable_operations(&self) -> bool {
        true
    }
    async fn on_hook(
        &self,
        hook: apeireth_runtime::HookPoint,
        ctx: &apeireth_runtime::ModuleContext<'_>,
    ) -> Result<apeireth_runtime::ModuleOutcome, apeireth_runtime::ModuleError> {
        if hook != apeireth_runtime::HookPoint::TurnStart {
            return Ok(apeireth_runtime::ModuleOutcome::continue_());
        }
        let input = ctx
            .invoker
            .committed_input()
            .await?
            .expect("accepted input at real hook");
        assert_eq!(input.origin, CommittedInputOrigin::DirectUser);
        ctx.invoker
            .prepare_operation("accepted-only".into(), serde_json::json!({"frozen":42}))
            .await?;
        Err(apeireth_runtime::ModuleError::Message(
            "injected downstream journal failure".into(),
        ))
    }
}
#[tokio::test]
async fn real_hook_prepares_durable_evidence_before_downstream_failure_without_completed_answer() {
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = runtime_with_behavior(
        store.clone(),
        Arc::new(PrincipalAuthorizations::default()),
        Arc::new(AbortAfterPrepare {
            manifest: ModuleManifest::new(MODULE, "Prepare abort"),
        }),
    )
    .await;
    let id = SessionId::new();
    assert!(runtime
        .execute(TurnRequest::new(id, "我的城市改为北岚市"))
        .await
        .is_err());
    let session = store.load(&id).await.unwrap().unwrap();
    let entry = session
        .log
        .iter()
        .find(|entry| matches!(entry.event, SessionEventKind::InputCommitted { .. }))
        .unwrap();
    let reader = invoker(runtime, id, entry.request.unwrap(), entry.trace.unwrap());
    let page = reader.committed_turns(None, 64).await.unwrap();
    assert!(page.turns.is_empty());
    assert_eq!(page.prepared_operations.len(), 1);
    assert_eq!(page.prepared_operations[0].operation_id, "accepted-only");
    assert_eq!(
        page.prepared_operations[0].payload,
        serde_json::json!({"frozen":42})
    );
    assert!(reader
        .committed_turns(page.next_cursor, 64)
        .await
        .unwrap()
        .prepared_operations
        .is_empty());
}

struct RevokeDuringRead {
    inner: Arc<InMemorySessionStore>,
    authority: Arc<PrincipalAuthorizations>,
    principal: AgentPrincipal,
    armed: std::sync::atomic::AtomicBool,
    loads: AtomicUsize,
}
#[async_trait]
impl SessionStore for RevokeDuringRead {
    async fn load(&self, id: &SessionId) -> apeireth_runtime::RuntimeResult<Option<Session>> {
        let value = self.inner.load(id).await?;
        if self.armed.load(Ordering::SeqCst) && self.loads.fetch_add(1, Ordering::SeqCst) == 1 {
            self.authority.revoke(&self.principal);
        }
        Ok(value)
    }
    async fn save(&self, session: &Session) -> apeireth_runtime::RuntimeResult<()> {
        self.inner.save(session).await
    }
    async fn list(&self) -> apeireth_runtime::RuntimeResult<Vec<Session>> {
        self.inner.list().await
    }
    async fn delete(&self, id: &SessionId) -> apeireth_runtime::RuntimeResult<bool> {
        self.inner.delete(id).await
    }
}
#[tokio::test]
async fn revocation_during_the_owning_store_read_cannot_release_committed_input() {
    let inner = Arc::new(InMemorySessionStore::new());
    let authority = Arc::new(PrincipalAuthorizations::default());
    let principal = AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    };
    authority.grant(&principal, [MODULE.to_owned()]);
    let store = Arc::new(RevokeDuringRead {
        inner: inner.clone(),
        authority: authority.clone(),
        principal: principal.clone(),
        armed: std::sync::atomic::AtomicBool::new(false),
        loads: AtomicUsize::new(0),
    });
    let runtime = runtime_with_behavior(
        store.clone(),
        authority,
        Arc::new(EvidenceModule {
            manifest: ModuleManifest::new(MODULE, "Read revocation"),
            access: CommittedEvidenceAccess::OwnedHistory,
        }),
    )
    .await;
    let id = SessionId::new();
    bound_session(&runtime, &inner, id, principal).await;
    let result = runtime
        .execute(TurnRequest::new(id, "private accepted input"))
        .await
        .unwrap();
    let reader = invoker(runtime, id, result.request, result.trace.trace);
    store.armed.store(true, Ordering::SeqCst);
    assert!(
        reader.committed_input().await.is_err(),
        "revocation after authorization but before source delivery must deny"
    );
    assert!(
        store.loads.load(Ordering::SeqCst) >= 2,
        "the delayed owning-source read must actually run"
    );
}

//! Evidence authority follows the selected mapped module, through actual hooks.
#[path = "support/convergence.rs"]
mod support;

use apeireth_core::kernel::{NamespaceId, ProfileId, SessionId};
use apeireth_plugin::{agents::AgentPrincipal, ToolExecutionContext};
use apeireth_protocol::canonical::ContentPart;
use apeireth_runtime::{
    BehaviorModule, CommittedEvidenceAccess, HookPoint, InMemorySessionStore, ModuleContext,
    ModuleError, ModuleManifest, ModuleOutcome, Session, SessionStore, TurnRequest,
};
use apeireth_runtime_assembly::profile_memory::{
    CognitiveStoreBinding, ProfileMemoryMappings, StoreAccess,
};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

const MODULE: &str = "test.mapped-evidence";
#[derive(Debug)]
struct Observation {
    current: bool,
    source: bool,
    history: bool,
    prepared: bool,
    history_text: String,
}
struct Probe {
    manifest: ModuleManifest,
    access: CommittedEvidenceAccess,
    persist: bool,
    seen: Mutex<Vec<Observation>>,
}
impl Probe {
    fn new(access: CommittedEvidenceAccess, persist: bool) -> Arc<Self> {
        Arc::new(Self {
            manifest: ModuleManifest::new(MODULE, "Mapped evidence probe"),
            access,
            persist,
            seen: Mutex::new(Vec::new()),
        })
    }
}
#[async_trait]
impl BehaviorModule for Probe {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn committed_evidence_access(&self) -> CommittedEvidenceAccess {
        self.access
    }
    fn persists_recoverable_operations(&self) -> bool {
        self.persist
    }
    async fn on_hook_authenticated(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
        execution: &ToolExecutionContext,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::TurnStart {
            return Ok(ModuleOutcome::continue_());
        }
        let request = execution.turn.expect("actual hook request");
        let current = ctx.invoker.committed_input().await;
        let source = ctx
            .invoker
            .committed_input_for(*ctx.session_id, request)
            .await;
        let history = ctx.invoker.committed_turns(None, 64).await;
        let history_text = history
            .as_ref()
            .map(|page| {
                page.turns
                    .iter()
                    .flat_map(|turn| turn.input.messages.iter())
                    .map(|message| ContentPart::join_text(&message.content))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let prepared = ctx
            .invoker
            .prepare_operation(
                format!("probe-{request}"),
                serde_json::json!({"probe":request.to_string()}),
            )
            .await;
        self.seen.lock().unwrap().push(Observation {
            current: matches!(current, Ok(Some(_))),
            source: matches!(source, Ok(Some(_))),
            history: history.is_ok(),
            prepared: prepared.is_ok(),
            history_text,
        });
        Ok(ModuleOutcome::continue_())
    }
}
fn principal() -> AgentPrincipal {
    AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    }
}
fn grant(mappings: &ProfileMemoryMappings, owner: &AgentPrincipal, store: &str, probe: Arc<Probe>) {
    mappings
        .grant(
            owner,
            None,
            vec![CognitiveStoreBinding {
                store: store.into(),
                access: StoreAccess::WritablePrivate,
                module: probe,
            }],
        )
        .unwrap();
}
async fn bound(
    runtime: &apeireth_runtime::Runtime,
    store: &InMemorySessionStore,
    principal: AgentPrincipal,
) -> SessionId {
    let id = SessionId::new();
    let mut session = Session::new(id, runtime.sessions().clock().as_ref());
    session.principal = Some(principal);
    store.save(&session).await.unwrap();
    id
}

#[tokio::test]
async fn same_id_profile_and_legacy_modules_keep_their_exact_evidence_grants_after_regrant_and_revoke(
) {
    let mappings = Arc::new(ProfileMemoryMappings::default());
    let a = principal();
    let b = principal();
    let c = principal();
    let denied = Probe::new(CommittedEvidenceAccess::None, false);
    let history = Probe::new(CommittedEvidenceAccess::OwnedHistory, true);
    let current = Probe::new(CommittedEvidenceAccess::CurrentInput, false);
    let legacy = Probe::new(CommittedEvidenceAccess::None, false);
    grant(&mappings, &a, "a-evidence", denied.clone());
    grant(&mappings, &b, "b-evidence", history.clone());
    grant(&mappings, &c, "c-evidence", current.clone());
    let store = Arc::new(InMemorySessionStore::new());
    let runtime = mappings
        .register_into(
            support::builder(support::Provider::new(vec![])),
            &[legacy.clone()],
        )
        .with_session_store(store.clone())
        .build()
        .await
        .unwrap();
    let a_session = bound(&runtime, &store, a.clone()).await;
    let b_session = bound(&runtime, &store, b.clone()).await;
    let c_session = bound(&runtime, &store, c).await;
    runtime
        .execute(TurnRequest::new(a_session, "PROFILE-A-PRIVATE"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(c_session, "PROFILE-C-PRIVATE"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(SessionId::new(), "LEGACY-PRIVATE"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(b_session, "PROFILE-B-FIRST"))
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(b_session, "PROFILE-B-SECOND"))
        .await
        .unwrap();
    for probe in [&denied, &legacy] {
        let seen = probe.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let observation = &seen[0];
        assert!(
            !observation.current
                && !observation.source
                && !observation.history
                && !observation.prepared,
            "another same-ID module must not elevate {observation:?}"
        );
    }
    let limited = current.seen.lock().unwrap();
    assert_eq!(limited.len(), 1);
    assert!(limited[0].current);
    assert!(!limited[0].source && !limited[0].history && !limited[0].prepared);
    drop(limited);
    let seen = history.seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen
        .iter()
        .all(|o| o.current && o.source && o.history && o.prepared));
    assert!(seen[1].history_text.contains("PROFILE-B-FIRST"));
    assert!(
        !seen[1].history_text.contains("PROFILE-A-PRIVATE")
            && !seen[1].history_text.contains("PROFILE-C-PRIVATE")
            && !seen[1].history_text.contains("LEGACY-PRIVATE")
    );
    drop(seen);
    let replacement = Probe::new(CommittedEvidenceAccess::OwnedHistory, true);
    grant(&mappings, &b, "b-evidence", replacement.clone());
    assert!(runtime
        .execute(TurnRequest::new(b_session, "old epoch must fail"))
        .await
        .is_err());
    assert!(replacement.seen.lock().unwrap().is_empty());
    let fresh = bound(&runtime, &store, b.clone()).await;
    runtime
        .execute(TurnRequest::new(
            fresh,
            "current grant may inspect owned old evidence",
        ))
        .await
        .unwrap();
    assert!(
        replacement.seen.lock().unwrap()[0]
            .history_text
            .contains("PROFILE-B-FIRST"),
        "the active new grant can read same-owner historical evidence; old epochs grant nothing"
    );
    mappings.revoke(&b);
    assert!(runtime
        .execute(TurnRequest::new(fresh, "revoked epoch must fail"))
        .await
        .is_err());
    assert_eq!(replacement.seen.lock().unwrap().len(), 1);
}

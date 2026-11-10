//! Adversarial supervisor cases exercise real Runtime dispatch and actual native/store effects.
#[path = "support/system_one_model.rs"]
mod model_support;
#[path = "support/system_one.rs"]
mod review_support;
#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_plugin::module_contract::ExecutionMode;
use apeireth_plugin::system_one::*;
use apeireth_plugin::{
    FrozenInvocation, ToolCapability, ToolExecutionPolicy, ToolModule, ToolPermissionPolicy,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use apeireth_runtime::canonical::dispatcher::{DispatchErrorCode, ToolScope};
use apeireth_runtime::canonical::system_one::{SystemOneGate, SystemOneLimits};
use apeireth_runtime::{Runtime, TurnRequest};
use apeireth_runtime_assembly::system_one::FileSystemOneAudit;
use async_trait::async_trait;
use review_support::{Audit, Mode, Supervisor};
use serde_json::json;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use support::*;

struct NativeWrite {
    id: CapabilityId,
    marker: PathBuf,
    audit_file: Option<PathBuf>,
    entered: Arc<AtomicUsize>,
    require_approval: bool,
    audit_only: bool,
}
#[async_trait]
impl ToolCapability for NativeWrite {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("protected_write").with_parameters(json!({"type":"object","properties":{"value":{"type":"string"}},"required":["value"],"additionalProperties":false}).as_object().unwrap().clone())
    }
    fn execution_policy(&self) -> ToolExecutionPolicy {
        ToolExecutionPolicy {
            permission: if self.require_approval {
                ToolPermissionPolicy::RequireApproval
            } else {
                ToolPermissionPolicy::Governed
            },
            ..Default::default()
        }
    }
    fn module_result_delivery(
        &self,
        _: &ToolResult,
    ) -> apeireth_plugin::tool_execution::ModuleResultDelivery {
        if self.audit_only {
            apeireth_plugin::tool_execution::ModuleResultDelivery::AuditOnly
        } else {
            apeireth_plugin::tool_execution::ModuleResultDelivery::Provider
        }
    }
    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        Ok(Some(FrozenInvocation::new(
            json!({"path":self.marker,"value":call.arguments["value"]}),
            json!({"operation":"write","target":"private workspace"}),
        )))
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invoke_frozen(call, None).await
    }
    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        self.entered.fetch_add(1, Ordering::AcqRel);
        if let Some(path) = &self.audit_file {
            let text =
                std::fs::read_to_string(path).expect("audit must be durable before the effect");
            let event: SystemOneAuditRecord =
                serde_json::from_str(text.lines().last().unwrap()).unwrap();
            assert_eq!(event.status, SystemOneReviewStatus::Allowed);
        }
        let owned;
        let frozen = if let Some(frozen) = frozen {
            frozen
        } else {
            owned = self.freeze_invocation(call).unwrap().unwrap();
            &owned
        };
        assert_eq!(frozen.payload["path"], json!(self.marker));
        std::fs::write(&self.marker, frozen.payload["value"].as_str().unwrap()).unwrap();
        ToolResult::ok(&call.id, json!({"written":true})).with_name(&call.name)
    }
}
fn module(
    marker: PathBuf,
    audit_file: Option<PathBuf>,
    require_approval: bool,
) -> (ToolModule, Arc<AtomicUsize>, Arc<dyn ToolCapability>) {
    let count = Arc::new(AtomicUsize::new(0));
    let tool = Arc::new(NativeWrite {
        id: CapabilityId::new("tool.protected_write").unwrap(),
        marker,
        audit_file,
        entered: count.clone(),
        require_approval,
        audit_only: false,
    });
    let module = ToolModule::new(pid("module.protected"), vec![tool])
        .unwrap()
        .with_public_contract(ExecutionMode::Direct)
        .unwrap();
    use apeireth_plugin::Plugin;
    let public = module.tools()[0].clone();
    (module, count, public)
}
fn write() -> ToolCall {
    call(
        "protected_write",
        json!({"value":"private-effect-sentinel"}),
    )
}
async fn run(runtime: &Runtime, provider: &Provider, session: SessionId) -> ToolResult {
    runtime
        .execute(TurnRequest::new(
            session,
            "request the registered operation",
        ))
        .await
        .unwrap();
    assert_eq!(provider.requests().len(), 2);
    let events = facts(runtime, session).await;
    assert_eq!(events.len(), 1);
    events[0].result().clone()
}

#[tokio::test]
async fn allow_is_audited_before_the_real_effect_and_usage_is_not_duplicated() {
    let dir = tempfile::tempdir().unwrap();
    let audit_path = dir.path().join("review.jsonl");
    let marker = dir.path().join("effect");
    let audit = Arc::new(FileSystemOneAudit::open(&audit_path).await.unwrap());
    let gate = Arc::new(SystemOneGate::new(audit, SystemOneLimits::default()).unwrap());
    let review_model = model_support::ReviewProvider::new(
        "provider.review_model",
        model_support::ModelMode::Valid,
    );
    let (module, count, _) = module(marker.clone(), Some(audit_path.clone()), false);
    let provider = Provider::new(vec![vec![write()], vec![]]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(module))
        .with_plugin(review_model.module())
        .with_system_one(gate.clone())
        .build_agent_runtime(apeireth_runtime::AgentHostPolicy::default())
        .await
        .unwrap();
    let supervisor = Arc::new(
        apeireth_runtime_assembly::system_one::AgentSystemOneSupervisor::spawn(
            &runtime,
            "review-model".into(),
            CapabilityId::new("provider.review_model").unwrap(),
            Default::default(),
        )
        .await
        .unwrap(),
    );
    let supervisor_id = supervisor.agent_id();
    gate.bind_supervisor(Some(supervisor));
    let main = runtime
        .spawn_agent(
            "main",
            apeireth_plugin::agents::AgentConfig {
                tools: vec![CapabilityId::new("tool.protected_write").unwrap()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    main.run("run the protected operation".into())
        .await
        .unwrap();
    assert_eq!(
        facts(&*runtime.kernel().await, main.snapshot().session)
            .await
            .len(),
        1
    );
    assert_eq!(provider.requests().len(), 2);
    assert_eq!(
        std::fs::read_to_string(marker).unwrap(),
        "private-effect-sentinel"
    );
    assert_eq!(count.load(Ordering::Acquire), 1);
    assert_eq!(review_model.requests().len(), 1);
    assert_eq!(runtime.snapshot(supervisor_id).unwrap().usage.requests, 1);
    assert_eq!(runtime.snapshot(supervisor_id).unwrap().usage.tool_calls, 0);
    let records = std::fs::read_to_string(audit_path).unwrap();
    assert_eq!(records.lines().count(), 1);
    assert!(!records.contains("private-effect-sentinel"));
    assert!(!records.contains("private workspace"));
    assert_eq!(runtime.usage(None).tool_calls, 1);
    assert_eq!(runtime.usage(None).requests, 3);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn deny_escalate_unavailable_and_forged_verdicts_never_reach_the_native_effect() {
    for (mode, code, status) in [
        (
            Mode::Decision(SystemOneDecision::Deny),
            DispatchErrorCode::SystemOneDenied,
            SystemOneReviewStatus::Denied,
        ),
        (
            Mode::Decision(SystemOneDecision::Escalate),
            DispatchErrorCode::SystemOneEscalated,
            SystemOneReviewStatus::Escalated,
        ),
        (
            Mode::Unavailable,
            DispatchErrorCode::SystemOneUnavailable,
            SystemOneReviewStatus::Unavailable,
        ),
        (
            Mode::InvalidVersion,
            DispatchErrorCode::SystemOneInvalidVerdict,
            SystemOneReviewStatus::Malformed,
        ),
        (
            Mode::InvalidPolicy,
            DispatchErrorCode::SystemOneInvalidVerdict,
            SystemOneReviewStatus::Malformed,
        ),
        (
            Mode::InvalidDigest,
            DispatchErrorCode::SystemOneInvalidVerdict,
            SystemOneReviewStatus::Malformed,
        ),
        (
            Mode::InvalidReview,
            DispatchErrorCode::SystemOneInvalidVerdict,
            SystemOneReviewStatus::Malformed,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("effect");
        let (gate, audit, supervisor) = review_support::gate(mode);
        let (module, count, _) = module(marker.clone(), None, false);
        let provider = Provider::new(vec![vec![write()], vec![]]);
        let runtime = builder(provider.clone())
            .with_plugin(Arc::new(module))
            .with_system_one(gate)
            .build()
            .await
            .unwrap();
        let result = run(&runtime, &provider, SessionId::new()).await;
        assert!(!result.is_ok());
        assert!(
            result
                .render()
                .contains(serde_json::to_value(code).unwrap().as_str().unwrap()),
            "{}",
            result.render()
        );
        assert_eq!(supervisor.requests().len(), 1);
        assert_eq!(audit.records()[0].status, status);
        assert_eq!(count.load(Ordering::Acquire), 0);
        assert!(!marker.exists());
    }
}

#[tokio::test]
async fn missing_service_and_raw_invocation_fail_closed_while_conversation_still_runs() {
    for unbound_gate in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("effect");
        let (module, count, raw) = module(marker.clone(), None, false);
        assert!(!raw.invoke(&write()).await.is_ok());
        assert!(!raw.invoke_frozen(&write(), None).await.is_ok());
        assert!(!raw
            .invoke_in_session(&write(), None, Some(SessionId::new()))
            .await
            .is_ok());
        let provider = Provider::new(vec![vec![write()], vec![]]);
        let audit = Arc::new(Audit::default());
        let mut build = builder(provider.clone()).with_plugin(Arc::new(module));
        if unbound_gate {
            build = build.with_system_one(Arc::new(
                SystemOneGate::new(audit.clone(), SystemOneLimits::default()).unwrap(),
            ));
        }
        let runtime = build.build().await.unwrap();
        let result = run(&runtime, &provider, SessionId::new()).await;
        assert!(!result.is_ok());
        assert!(result.render().contains("system_one_unavailable"));
        if unbound_gate {
            assert_eq!(
                audit.records()[0].status,
                SystemOneReviewStatus::Unavailable
            );
        }
        runtime
            .execute(TurnRequest::new(
                SessionId::new(),
                "ordinary conversation without a tool",
            ))
            .await
            .unwrap();
        assert_eq!(count.load(Ordering::Acquire), 0);
        assert!(!marker.exists());
    }
}

#[tokio::test]
async fn supervisor_timeout_and_audit_failure_do_not_default_to_allow() {
    for audit_failure in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("effect");
        let audit = Arc::new(Audit::default());
        audit.fail.store(audit_failure, Ordering::Release);
        let gate = Arc::new(
            SystemOneGate::new(
                audit.clone(),
                SystemOneLimits {
                    review_timeout: Duration::from_millis(25),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        gate.bind_supervisor(Some(Supervisor::new(if audit_failure {
            Mode::Decision(SystemOneDecision::Allow)
        } else {
            Mode::Pending
        })));
        let (module, count, _) = module(marker.clone(), None, false);
        let provider = Provider::new(vec![vec![write()], vec![]]);
        let runtime = builder(provider.clone())
            .with_plugin(Arc::new(module))
            .with_system_one(gate)
            .build()
            .await
            .unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            run(&runtime, &provider, SessionId::new()),
        )
        .await
        .unwrap();
        assert!(!result.is_ok());
        assert!(result.render().contains(if audit_failure {
            "system_one_audit_failed"
        } else {
            "timeout"
        }));
        if !audit_failure {
            assert_eq!(audit.records()[0].status, SystemOneReviewStatus::Timeout);
        }
        assert_eq!(count.load(Ordering::Acquire), 0);
        assert!(!marker.exists());
    }
}

#[tokio::test]
async fn model_allow_cannot_override_deterministic_scope_governance_or_approval() {
    for restriction in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("effect");
        let (gate, _, supervisor) = review_support::gate(Mode::Decision(SystemOneDecision::Allow));
        let (module, count, _) = module(marker.clone(), None, restriction == 2);
        let provider = Provider::new(vec![vec![write()], vec![]]);
        let mut build = builder(provider.clone())
            .with_plugin(Arc::new(module))
            .with_system_one(gate);
        if restriction == 1 {
            build = build.with_governance(Arc::new(
                apeireth_governance::DenyCapabilities::new()
                    .deny(CapabilityId::new("tool.protected_write").unwrap()),
            ));
        }
        let runtime = build.build().await.unwrap();
        let session = SessionId::new();
        if restriction == 0 {
            runtime
                .tool_dispatcher()
                .set_scope(session, ToolScope::Only(BTreeSet::new()));
        }
        if restriction == 2 {
            let outcome = runtime
                .execute_outcome(TurnRequest::new(session, "request approval"))
                .await
                .unwrap();
            assert!(matches!(
                outcome,
                apeireth_runtime::TurnOutcome::PendingApproval(_)
            ));
            assert_eq!(runtime.pending_approvals(session).await.unwrap().len(), 1);
            assert_eq!(provider.requests().len(), 1);
        } else {
            assert!(!run(&runtime, &provider, session).await.is_ok());
        }
        assert!(supervisor.requests().is_empty());
        assert_eq!(count.load(Ordering::Acquire), 0);
        assert!(!marker.exists());
    }
}

#[tokio::test]
async fn scope_revocation_cancel_and_supervisor_replacement_are_checked_after_await() {
    for change in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("effect");
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let (gate, audit, supervisor) =
            review_support::gate(Mode::Wait(entered.clone(), release.clone()));
        let (module, count, _) = module(marker.clone(), None, false);
        let provider = Provider::new(vec![vec![write()], vec![]]);
        let runtime = builder(provider.clone())
            .with_plugin(Arc::new(module))
            .with_system_one(gate.clone())
            .build()
            .await
            .unwrap();
        let session = SessionId::new();
        let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(run(&runtime, &provider, session), async {
                entered.notified().await;
                match change {
                    0 => runtime
                        .tool_dispatcher()
                        .set_scope(session, ToolScope::Only(BTreeSet::new())),
                    1 => supervisor.requests()[0].cancel_token.cancel(),
                    _ => gate.bind_supervisor(None),
                }
                release.notify_one();
            })
        })
        .await
        .unwrap();
        assert!(!result.is_ok());
        assert_eq!(audit.records().len(), 1);
        assert_eq!(count.load(Ordering::Acquire), 0);
        assert!(!marker.exists());
    }
}

#[tokio::test]
async fn direct_and_owned_agent_memory_writes_share_the_same_mandatory_denial() {
    use apeireth_core::kernel::{NamespaceId, ProfileId, SystemClock};
    use apeireth_memory::{MemoryCoordinator, SqliteMemoryStore};
    use apeireth_plugin::agents::{AgentConfig, AgentPrincipal, AgentStatus};
    use apeireth_plugin::memory_backend::MemoryBackend;
    use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant};
    use apeireth_runtime_assembly::memory_module::{namespace_key, MemoryModule};
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let coordinator = Arc::new(MemoryCoordinator::new(db.clone(), db.clone()));
    let principal = AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    };
    let write_call = call(
        "memory_write",
        json!({"content":"forbidden-memory-effect","idempotency_key":"same-action"}),
    );
    let provider = Provider::new(vec![
        vec![write_call.clone()],
        vec![call("memory_organize", json!({"query":"topic"}))],
        vec![],
    ]);
    let release = provider.organizer_script(vec![vec![write_call], vec![]]);
    let module = Arc::new(
        MemoryModule::new(coordinator, Arc::new(SystemClock))
            .with_automatic_retrieval(false)
            .with_module_contract(),
    );
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        pid("module.memory"),
        ModuleAgentGrant {
            tools: [
                CapabilityId::new("tool.memory.retrieve").unwrap(),
                CapabilityId::new("tool.memory.write").unwrap(),
            ]
            .into_iter()
            .collect(),
            principals: vec![principal.clone()],
            ..Default::default()
        },
    );
    let (gate, audit, supervisor) = review_support::gate(Mode::Decision(SystemOneDecision::Deny));
    let runtime = builder(provider.clone())
        .with_plugin(module.clone())
        .with_system_one(gate)
        .build_agent_runtime(policy)
        .await
        .unwrap();
    let main = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                principal: Some(principal.clone()),
                tools: vec![
                    CapabilityId::new("tool.memory.write").unwrap(),
                    CapabilityId::new("tool.memory.organize").unwrap(),
                ],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    main.run("run direct and module-owned attempts".into())
        .await
        .unwrap();
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), async {
        while module.organizer_snapshots().is_empty()
            || module
                .organizer_snapshots()
                .iter()
                .any(|a| a.status != AgentStatus::Destroyed)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let requests = supervisor.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].capability_id, requests[1].capability_id);
    assert_eq!(
        requests[0].action_arguments_digest,
        requests[1].action_arguments_digest
    );
    assert_ne!(requests[0].agent_id, requests[1].agent_id);
    assert_ne!(requests[0].review_id, requests[1].review_id);
    assert!(audit
        .records()
        .iter()
        .all(|r| r.status == SystemOneReviewStatus::Denied));
    assert!(
        MemoryBackend::recent_episodes(db.as_ref(), &namespace_key(&principal), 10)
            .unwrap()
            .is_empty()
    );
    assert_eq!(runtime.usage(None).tool_calls, 3);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn exact_supervisor_model_has_no_fallback_and_cannot_dispatch_its_own_tool_calls() {
    use apeireth_core::kernel::{AgentId, ModuleInstanceId, TaskId, TraceId};
    use apeireth_runtime_assembly::system_one::AgentSystemOneSupervisor;
    use model_support::{ModelMode, ReviewProvider};
    for mode in [
        ModelMode::ToolCall,
        ModelMode::Malformed,
        ModelMode::Unavailable,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("effect");
        let selected = ReviewProvider::new("provider.selected_review", mode);
        let other = ReviewProvider::new("provider.unselected_review", ModelMode::Valid);
        let (module, count, _) = module(marker.clone(), None, false);
        let runtime = Runtime::builder()
            .with_default_model("review-model")
            .with_governance(Arc::new(apeireth_governance::AllowAll))
            .with_plugin(selected.module())
            .with_plugin(other.module())
            .with_plugin(Arc::new(module))
            .build_agent_runtime(apeireth_runtime::AgentHostPolicy::default())
            .await
            .unwrap();
        let supervisor = AgentSystemOneSupervisor::spawn(
            &runtime,
            "review-model".into(),
            CapabilityId::new("provider.selected_review").unwrap(),
            Default::default(),
        )
        .await
        .unwrap();
        let request = SystemOneRequest {
            version: 1,
            policy_version: 1,
            risk: SystemOneRisk::Governed,
            review_id: TaskId::new(),
            module_definition_id: pid("module.protected"),
            module_instance_id: ModuleInstanceId::new(),
            agent_id: AgentId::new(),
            agent_owner: None,
            session_id: None,
            turn_id: None,
            caller_principal: None,
            delegation_chain: vec![],
            capability_id: CapabilityId::new("tool.protected_write").unwrap(),
            action_arguments_digest: "test-action-digest".into(),
            scope: Some(vec![]),
            intent_id: None,
            authorization_epoch: None,
            approved: false,
            deadline_remaining_ms: 5000,
            cancel_token: Default::default(),
            trace_id: Some(TraceId::new()),
            call_id: "test-review".into(),
            arguments: json!({"value":"private-effect-sentinel"}),
            frozen_display: None,
        };
        assert!(supervisor.review(&request).await.is_err());
        assert_eq!(selected.requests().len(), 1);
        assert!(
            other.requests().is_empty(),
            "no substitute provider may be tried"
        );
        assert_eq!(
            runtime
                .snapshot(supervisor.agent_id())
                .unwrap()
                .usage
                .tool_calls,
            0
        );
        assert_eq!(count.load(Ordering::Acquire), 0);
        assert!(!marker.exists());
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn registered_private_hook_uses_the_gate_and_audit_only_does_not_enter_main_context() {
    use apeireth_core::kernel::TaskId;
    use apeireth_plugin::hooks::{HookContextV1, HookToolRequest};
    use apeireth_plugin::module_contract::{
        ContractTool, EffectClass, ModuleToolContract, ToolSurface,
    };
    use apeireth_plugin::{Plugin, PluginContext, PluginManifest, PluginResult};
    use apeireth_protocol::canonical::{ModuleHookV1, ToolEventSourceV1};
    struct Hook {
        manifest: PluginManifest,
        tool: Arc<dyn ToolCapability>,
        execution: TaskId,
    }
    #[async_trait]
    impl Plugin for Hook {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }
        fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
            vec![self.tool.clone()]
        }
        fn hooks(&self) -> Vec<ModuleHookV1> {
            vec![ModuleHookV1::BeforeModelCall]
        }
        async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
            Ok(())
        }
        async fn shutdown(&self) -> PluginResult<()> {
            Ok(())
        }
        async fn on_hook(&self, _: &HookContextV1) -> PluginResult<Vec<HookToolRequest>> {
            Ok(vec![HookToolRequest {
                execution: self.execution,
                call: write(),
            }])
        }
    }
    for allowed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("effect");
        let count = Arc::new(AtomicUsize::new(0));
        let native = Arc::new(NativeWrite {
            id: CapabilityId::new("tool.protected_write").unwrap(),
            marker: marker.clone(),
            audit_file: None,
            entered: count.clone(),
            require_approval: false,
            audit_only: true,
        });
        let private = ContractTool::wrap(
            native,
            ModuleToolContract {
                surface: ToolSurface::Internal,
                effect: EffectClass::Protected,
                ..Default::default()
            },
        )
        .unwrap();
        let declaration = ToolModule::new(pid("module.protected"), vec![private.clone()]).unwrap();
        let hook = Arc::new(Hook {
            manifest: declaration.manifest().clone(),
            tool: private,
            execution: TaskId::new(),
        });
        let provider = Provider::new(vec![vec![]]);
        let (gate, audit, _) = review_support::gate(Mode::Decision(if allowed {
            SystemOneDecision::Allow
        } else {
            SystemOneDecision::Deny
        }));
        let runtime = builder(provider.clone())
            .with_plugin(hook)
            .with_system_one(gate)
            .build()
            .await
            .unwrap();
        let session = SessionId::new();
        runtime
            .execute(TurnRequest::new(session, "ordinary main turn"))
            .await
            .unwrap();
        assert_eq!(count.load(Ordering::Acquire), usize::from(allowed));
        assert_eq!(marker.exists(), allowed);
        assert_eq!(audit.records().len(), 1);
        let events = facts(&runtime, session).await;
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].source(),
            ToolEventSourceV1::Module {
                hook: ModuleHookV1::BeforeModelCall,
                ..
            }
        ));
        assert!(provider.results().is_empty());
        assert_eq!(provider.requests().len(), 1);
        assert!(!serde_json::to_string(&provider.requests())
            .unwrap()
            .contains("private-effect-sentinel"));
    }
}

#[tokio::test]
async fn frozen_approval_still_requires_review_and_binds_the_exact_effective_payload() {
    use apeireth_runtime::{ApprovalDecision, ApprovalResolution, TurnOutcome};
    use sha2::{Digest, Sha256};
    for allowed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("effect");
        let (gate, audit, supervisor) = review_support::gate(Mode::Decision(if allowed {
            SystemOneDecision::Allow
        } else {
            SystemOneDecision::Deny
        }));
        let (module, count, _) = module(marker.clone(), None, true);
        let provider = Provider::new(vec![vec![write()], vec![]]);
        let runtime = builder(provider.clone())
            .with_plugin(Arc::new(module))
            .with_system_one(gate)
            .build()
            .await
            .unwrap();
        let session = SessionId::new();
        let TurnOutcome::PendingApproval(view) = runtime
            .execute_outcome(TurnRequest::new(
                session,
                "request the exact frozen operation",
            ))
            .await
            .unwrap()
        else {
            panic!("approval must be required");
        };
        assert!(!marker.exists());
        assert!(supervisor.requests().is_empty());
        let stored = runtime.sessions().load(&session).await.unwrap().unwrap();
        let pending = stored.approvals.get(&view.approval_id).unwrap();
        let expected = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(
                    &pending.capability_id,
                    &pending.tool_call.arguments,
                    pending.effective_invocation.as_ref().map(|f| &f.payload)
                ))
                .unwrap()
            )
        );
        let resolution = runtime
            .resolve_approval(session, view.approval_id, ApprovalDecision::Approve)
            .await
            .unwrap();
        assert!(matches!(
            resolution,
            ApprovalResolution::Resumed(TurnOutcome::Completed(_))
        ));
        let requests = supervisor.requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].approved);
        assert_eq!(requests[0].risk, SystemOneRisk::ExplicitApprovalRequired);
        assert_eq!(requests[0].action_arguments_digest, expected);
        assert_eq!(requests[0].frozen_display, view.effective_invocation);
        assert_eq!(audit.records().len(), 1);
        assert_eq!(count.load(Ordering::Acquire), usize::from(allowed));
        assert_eq!(marker.exists(), allowed);
        if allowed {
            assert_eq!(
                std::fs::read_to_string(marker).unwrap(),
                "private-effect-sentinel"
            );
        }
        let events = facts(&runtime, session).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].result().is_ok(), allowed);
        assert_eq!(provider.requests().len(), 2);
    }
}

#[tokio::test]
async fn review_payload_bound_refuses_before_contacting_a_supervisor() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("effect");
    let audit = Arc::new(Audit::default());
    let gate = Arc::new(
        SystemOneGate::new(
            audit.clone(),
            SystemOneLimits {
                max_request_bytes: 128,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let supervisor = Supervisor::new(Mode::Decision(SystemOneDecision::Allow));
    gate.bind_supervisor(Some(supervisor.clone()));
    let (module, count, _) = module(marker.clone(), None, false);
    let provider = Provider::new(vec![vec![write()], vec![]]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(module))
        .with_system_one(gate)
        .build()
        .await
        .unwrap();
    assert!(!run(&runtime, &provider, SessionId::new()).await.is_ok());
    assert!(supervisor.requests().is_empty());
    assert_eq!(audit.records()[0].status, SystemOneReviewStatus::Malformed);
    assert_eq!(count.load(Ordering::Acquire), 0);
    assert!(!marker.exists());
}

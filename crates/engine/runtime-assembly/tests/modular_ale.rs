//! Explicit Host compositions using the real Runtime and existing native/store executors.
#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{
    CapabilityId, ModuleInstanceId, NamespaceId, ProfileId, SessionId, SystemClock,
};
use apeireth_plugin::agents::{AgentConfig, AgentPrincipal, AgentStatus};
use apeireth_plugin::module_contract::{
    ContractTool, EffectClass, ExecutionMode, ModuleTaskHandle, ModuleToolContract, ToolSurface,
};
use apeireth_plugin::{Plugin, ToolCapability, ToolModule};
use apeireth_protocol::canonical::{ContentPart, NormalizedTool, ToolCall, ToolResult};
use apeireth_runtime::canonical::CapabilityProvider;
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant, TurnRequest};
use apeireth_runtime_assembly::canonical::memory_module::MemoryModule;
use apeireth_runtime_assembly::ShellModule;
use apeireth_tools_canonical::TrustedShellConfig;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use support::*;

fn cid(id: &str) -> CapabilityId {
    CapabilityId::new(id).unwrap()
}
fn projected(provider: &Provider, key: &str) -> Value {
    provider
        .results()
        .iter()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
        .find(|v| v.get(key).is_some())
        .expect("public projection")
}

#[tokio::test]
async fn direct_shell_keeps_its_name_frozen_payload_and_has_no_owned_ale() {
    let dir = tempfile::tempdir().unwrap();
    let module = ShellModule::new(TrustedShellConfig::new(dir.path()).with_sandbox(false));
    let logical = module
        .tool_module_with_contract(ExecutionMode::Direct)
        .unwrap();
    let original = module
        .tool()
        .freeze_invocation(&call(
            "shell",
            json!({"command":"echo ale_contract_direct"}),
        ))
        .unwrap();
    let wrapped = logical.tools()[0]
        .freeze_invocation(&call(
            "shell",
            json!({"command":"echo ale_contract_direct"}),
        ))
        .unwrap();
    assert_eq!(original, wrapped);
    let provider = Provider::new(vec![
        vec![call("shell", json!({"command":"echo ale_contract_direct"}))],
        vec![],
    ]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(logical))
        .build()
        .await
        .unwrap();
    assert_eq!(
        runtime.capability_registry().module_public_declarations()[0].name,
        "shell"
    );
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "run the direct tool"))
        .await
        .unwrap();
    let events = facts(&runtime, session).await;
    assert_eq!(events.len(), 1);
    assert!(events[0].result().is_ok(), "{:?}", events[0].result());
    assert!(provider
        .results()
        .iter()
        .any(|s| s.contains("ale_contract_direct")));
    assert_eq!(provider.requests().len(), 2);
}

struct Observation {
    id: CapabilityId,
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl ToolCapability for Observation {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("private_observation")
    }
    fn module_result_delivery(
        &self,
        _: &ToolResult,
    ) -> apeireth_plugin::tool_execution::ModuleResultDelivery {
        apeireth_plugin::tool_execution::ModuleResultDelivery::AuditOnly
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.calls.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, json!({"public_status":"ready"})).with_name(&call.name)
    }
}
#[tokio::test]
async fn internal_surface_rejects_model_guessing_and_is_absent_from_public_views() {
    let calls = Arc::new(AtomicUsize::new(0));
    let native = Arc::new(Observation {
        id: cid("tool.private_observation"),
        calls: calls.clone(),
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
    let module = ToolModule::new(pid("module.private"), vec![private]).unwrap();
    let provider = Provider::new(vec![vec![call("private_observation", json!({}))], vec![]]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(module))
        .build()
        .await
        .unwrap();
    assert!(runtime.tools().is_empty());
    assert!(runtime
        .capability_registry()
        .module_public_declarations()
        .is_empty());
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "guess an internal name"))
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(!facts(&runtime, session).await[0].result().is_ok());
}

#[tokio::test]
async fn audit_only_policy_still_closes_an_explicit_model_tool_call() {
    let calls = Arc::new(AtomicUsize::new(0));
    let public = ContractTool::wrap(
        Arc::new(Observation {
            id: cid("tool.public_observation"),
            calls: calls.clone(),
        }),
        ModuleToolContract::logical(ExecutionMode::Direct, EffectClass::Pure),
    )
    .unwrap();
    let module = ToolModule::new(pid("module.public"), vec![public]).unwrap();
    let provider = Provider::new(vec![vec![call("private_observation", json!({}))], vec![]]);
    let runtime = builder(provider.clone())
        .with_plugin(Arc::new(module))
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "explicit public observation"))
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(projected(&provider, "public_status")["public_status"] == "ready");
    assert_eq!(facts(&runtime, session).await.len(), 1);
    // The shared provider fixture validates closed contexts and all protocol adapters.
}

#[tokio::test]
async fn organizer_returns_a_closed_accepted_handle_and_only_authorized_status() {
    use apeireth_memory::{MemoryCoordinator, SqliteMemoryStore};
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let coordinator = Arc::new(MemoryCoordinator::new(db.clone(), db));
    let principal = AgentPrincipal {
        profile: ProfileId::new(),
        namespace: NamespaceId::new(),
    };
    let provider = Provider::new(vec![
        vec![call("memory_organize", json!({"query":"topic"}))],
        vec![],
    ]);
    let release = provider.organizer_script(vec![
        vec![call(
            "memory_write",
            json!({"content":"private_child_payload","idempotency_key":"summary"}),
        )],
        vec![],
    ]);
    let module = Arc::new(
        MemoryModule::new(coordinator, Arc::new(SystemClock))
            .with_automatic_retrieval(false)
            .with_module_contract(),
    );
    let mut policy = AgentHostPolicy::default();
    policy.modules.insert(
        pid("module.memory"),
        ModuleAgentGrant {
            tools: [cid("tool.memory.write")].into_iter().collect(),
            principals: vec![principal.clone()],
            ..Default::default()
        },
    );
    // Organizer also requests retrieve, so its independent Host grant includes both.
    policy
        .modules
        .get_mut(&pid("module.memory"))
        .unwrap()
        .tools
        .insert(cid("tool.memory.retrieve"));
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
                tools: vec![
                    cid("tool.memory.organize"),
                    cid("tool.memory.organizer_status"),
                ],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    main.run("main_private_marker".into()).await.unwrap();
    let accepted = projected(&provider, "task_handle");
    assert_eq!(accepted["accepted"], true);
    let handle: ModuleTaskHandle = serde_json::from_value(accepted["task_handle"].clone()).unwrap();
    assert_eq!(handle.version, 1);
    assert_eq!(handle.module, pid("module.memory"));
    tokio::time::timeout(Duration::from_secs(5), async {
        while module
            .organizer_snapshots()
            .iter()
            .all(|a| a.status != AgentStatus::Running)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let child = module.organizer_snapshots()[0].clone();
    assert_ne!(child.session, main.snapshot().session);
    provider.enqueue_tools(vec![call(
        "memory_organizer_status",
        json!({"task_handle":handle}),
    )]);
    provider.enqueue_tools(vec![]);
    main.run("query accepted task".into()).await.unwrap();
    assert!(provider
        .results()
        .iter()
        .any(|s| s.contains("\"complete\":false")));
    let other = runtime
        .spawn_agent(
            "other",
            AgentConfig {
                principal: Some(AgentPrincipal {
                    profile: ProfileId::new(),
                    namespace: NamespaceId::new(),
                }),
                tools: vec![cid("tool.memory.organizer_status")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    provider.enqueue_tools(vec![call(
        "memory_organizer_status",
        json!({"task_handle":handle}),
    )]);
    provider.enqueue_tools(vec![]);
    other.run("try another namespace".into()).await.unwrap();
    assert!(provider
        .results()
        .last()
        .unwrap()
        .contains("task_not_found"));
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), async {
        while module
            .organizer_snapshots()
            .iter()
            .any(|a| a.status != AgentStatus::Destroyed)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    provider.enqueue_tools(vec![call(
        "memory_organizer_status",
        json!({"task_handle":handle}),
    )]);
    provider.enqueue_tools(vec![]);
    main.run("query completed task".into()).await.unwrap();
    assert!(provider
        .results()
        .iter()
        .any(|s| s.contains("\"complete\":true")));
    let mut stale = handle;
    stale.instance = ModuleInstanceId::new();
    provider.enqueue_tools(vec![call(
        "memory_organizer_status",
        json!({"task_handle":stale}),
    )]);
    provider.enqueue_tools(vec![]);
    main.run("try a stale generation".into()).await.unwrap();
    assert!(provider
        .results()
        .last()
        .unwrap()
        .contains("task_not_found"));
    let parent = runtime
        .kernel()
        .await
        .sessions()
        .load(&main.snapshot().session)
        .await
        .unwrap()
        .unwrap();
    assert!(!serde_json::to_string(&parent.messages)
        .unwrap()
        .contains("private_child_payload"));
    assert!(provider
        .requests()
        .iter()
        .filter(|r| r
            .messages
            .iter()
            .any(|m| ContentPart::join_text(&m.content).starts_with("memory.organizer.v1:")))
        .all(|r| !serde_json::to_string(r)
            .unwrap()
            .contains("main_private_marker")));
    assert_eq!(child.owner, module.organizer_snapshots()[0].owner);
    runtime.shutdown().await.unwrap();
}

#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{NamespaceId, ProfileId};
use apeireth_plugin::agents::{AgentConfig, AgentStatus};
use apeireth_protocol::canonical::{validate_closed_tool_context, ContentPart};
use apeireth_runtime::canonical::profiles::ProfileConfig;
use apeireth_runtime::AgentHostPolicy;
use apeireth_runtime_assembly::PortableHost;
use std::{sync::Arc, time::Duration};
use support::*;
#[tokio::test]
async fn no_business_modules_complete_turn_and_unknown_shell_is_a_paired_error_without_fallback() {
    let provider = Provider::new(vec![]);
    let host = PortableHost::assemble(
        builder(provider.clone()),
        AgentHostPolicy::default(),
        AgentConfig::default(),
    )
    .await
    .unwrap();
    assert!(host.tool_scope().await.is_empty());
    assert!(host.runtime().kernel().await.tools().is_empty());
    assert_eq!(host.submit("hello").await.unwrap().text, "done");
    provider.enqueue_tools(vec![call(
        "shell",
        serde_json::json!({"command":"portable sentinel must never execute"}),
    )]);
    assert_eq!(host.submit("unknown tool").await.unwrap().text, "done");
    let facts = host.tool_events().await.unwrap();
    let failure = facts
        .iter()
        .find(|f| !f.result().is_ok())
        .expect("unregistered call must be a V1 error");
    let apeireth_protocol::canonical::ToolOutcome::Error { message, .. } =
        &failure.result().outcome
    else {
        panic!("expected structured tool error")
    };
    let error: apeireth_runtime::canonical::dispatcher::DispatchFailure =
        serde_json::from_str(message).unwrap();
    assert_eq!(
        error.code,
        apeireth_runtime::canonical::dispatcher::DispatchErrorCode::Unregistered
    );
    assert!(provider
        .results()
        .iter()
        .any(|result| result.contains("\"code\":\"unregistered\"")));
    for request in provider.requests() {
        assert!(request.tools.is_empty());
        validate_closed_tool_context(&request.messages).unwrap();
    }
    assert_eq!(host.usage().requests, 3);
    host.shutdown().await.unwrap();
    assert!(host.submit("after shutdown").await.is_err());
}
#[tokio::test]
async fn profile_switch_replaces_only_main_context_and_keeps_principal_binding() {
    let provider = Provider::new(vec![]);
    let host = PortableHost::assemble(
        builder(provider.clone()),
        AgentHostPolicy::default(),
        AgentConfig::default(),
    )
    .await
    .unwrap();
    let a = ProfileConfig {
        id: ProfileId::new(),
        namespace: NamespaceId::new(),
        label: "a".into(),
        agent: AgentConfig::default(),
    };
    let b = ProfileConfig {
        id: ProfileId::new(),
        namespace: NamespaceId::new(),
        label: "b".into(),
        agent: AgentConfig::default(),
    };
    let first = host.bind_profile(&a).await.unwrap();
    host.submit("only_alpha_context").await.unwrap();
    let second = host.bind_profile(&b).await.unwrap();
    host.submit("only_beta_context").await.unwrap();
    assert_ne!(first.id, second.id);
    assert_ne!(first.session, second.session);
    assert_eq!(second.principal, Some(b.principal()));
    let requests = provider.requests();
    let text = requests
        .last()
        .unwrap()
        .messages
        .iter()
        .map(|m| ContentPart::join_text(&m.content))
        .collect::<String>();
    assert!(text.contains("only_beta_context"));
    assert!(!text.contains("only_alpha_context"));
    assert_eq!(host.snapshot().await.usage.requests, 1);
    assert_eq!(host.usage().requests, 2);
    host.shutdown().await.unwrap();
}
#[tokio::test]
async fn cancellation_stops_blocked_main_without_waiting_for_provider_reply() {
    let provider = Provider::new(vec![]);
    let _hold = provider.hold_next();
    let host = Arc::new(
        PortableHost::assemble(
            builder(provider.clone()),
            AgentHostPolicy::default(),
            AgentConfig::default(),
        )
        .await
        .unwrap(),
    );
    let running = host.clone();
    let worker = tokio::spawn(async move { running.submit("blocked").await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while provider.requests().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    host.cancel().await.unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(2), worker)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert_eq!(host.snapshot().await.status, AgentStatus::Stopped);
    host.shutdown().await.unwrap();
}
#[cfg(feature = "memory")]
#[tokio::test]
async fn optional_memory_module_uses_same_portable_runtime_hook_and_namespace() {
    use apeireth_core::kernel::{CapabilityId, SystemClock};
    use apeireth_memory::{MemoryCoordinator, MemoryGovernanceStore, SqliteMemoryStore};
    use apeireth_plugin::memory_backend::MemoryBackend;
    use apeireth_protocol::canonical::{MessageRole, ToolEventSourceV1};
    use apeireth_runtime_assembly::memory_module::MemoryModule;
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let backend: Arc<dyn MemoryBackend> = db.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = db;
    let memory = Arc::new(MemoryModule::new(
        Arc::new(MemoryCoordinator::new(backend, governance)),
        Arc::new(SystemClock),
    ));
    let provider = Provider::new(vec![
        vec![call(
            "memory_write",
            serde_json::json!({"content":"portable_apricot_421","idempotency_key":"portable"}),
        )],
        vec![],
        vec![],
    ]);
    let config = AgentConfig {
        principal: Some(apeireth_plugin::agents::AgentPrincipal {
            profile: ProfileId::new(),
            namespace: NamespaceId::new(),
        }),
        tools: ["tool.memory.retrieve", "tool.memory.write"]
            .into_iter()
            .map(|id| CapabilityId::new(id).unwrap())
            .collect(),
        ..Default::default()
    };
    let host = PortableHost::assemble(
        builder(provider.clone()).with_plugin(memory),
        AgentHostPolicy::default(),
        config,
    )
    .await
    .unwrap();
    host.submit("remember portable_apricot_421").await.unwrap();
    host.submit("portable_apricot_421").await.unwrap();
    assert!(host
        .tool_events()
        .await
        .unwrap()
        .iter()
        .any(|f| matches!(f.source(), ToolEventSourceV1::Module { .. })));
    let requests = provider.requests();
    assert!(requests
        .last()
        .unwrap()
        .messages
        .iter()
        .any(|m| m.role == MessageRole::Tool
            && ContentPart::join_text(&m.content).contains("portable_apricot_421")));
    host.shutdown().await.unwrap();
}

#[cfg(feature = "profile-store")]
#[tokio::test]
async fn optional_durable_profile_loads_without_desktop_modules_or_memory() {
    use apeireth_runtime::canonical::profiles::ProfileStore;
    use apeireth_runtime_assembly::profile_store::SqliteProfileStore;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profiles.sqlite3");
    let profile = ProfileConfig {
        id: ProfileId::new(),
        namespace: NamespaceId::new(),
        label: "persisted".into(),
        agent: AgentConfig::default(),
    };
    let store = SqliteProfileStore::open(&path).await.unwrap();
    store.save(profile.clone(), None).await.unwrap();
    drop(store);
    let store = SqliteProfileStore::open(&path).await.unwrap();
    let loaded = store.load(profile.id).await.unwrap().unwrap();
    assert_eq!(loaded.revision, 1);
    let host = PortableHost::assemble(
        builder(Provider::new(vec![])),
        AgentHostPolicy::default(),
        AgentConfig::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        host.bind_profile(&loaded.config).await.unwrap().principal,
        Some(profile.principal())
    );
    assert_eq!(host.submit("durable profile").await.unwrap().text, "done");
    assert!(host.tool_scope().await.is_empty());
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn minimal_host_shutdown_drains_factory_owned_background_task_with_external_resource_arc() {
    use apeireth_plugin::resource_factory::{
        module_from_definition, ModuleDefinition, ModuleResources, OwnedModuleResource,
    };
    use apeireth_plugin::{Plugin, PluginContext, PluginManifest, PluginResult};
    use async_trait::async_trait;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };
    struct Probe {
        active: AtomicBool,
        revoked: AtomicBool,
        closed: AtomicBool,
        ready: tokio::sync::Notify,
    }
    struct Running(Arc<Probe>);
    impl Drop for Running {
        fn drop(&mut self) {
            self.0.active.store(false, Ordering::SeqCst);
        }
    }
    #[async_trait]
    impl OwnedModuleResource for Probe {
        fn revoke(&self) {
            self.revoked.store(true, Ordering::SeqCst);
        }
        async fn close(&self) -> Result<(), String> {
            if self.active.load(Ordering::SeqCst) {
                return Err("background task not drained".into());
            }
            self.closed.store(true, Ordering::SeqCst);
            Ok(())
        }
    }
    struct Instance(PluginManifest);
    #[async_trait]
    impl Plugin for Instance {
        fn manifest(&self) -> &PluginManifest {
            &self.0
        }
        async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
            Ok(())
        }
        async fn shutdown(&self) -> PluginResult<()> {
            Ok(())
        }
    }
    struct Definition {
        manifest: PluginManifest,
        observer: Mutex<Option<Arc<Probe>>>,
    }
    #[async_trait]
    impl ModuleDefinition for Definition {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }
        async fn instantiate(
            &self,
            ctx: &PluginContext,
            resources: &ModuleResources,
        ) -> PluginResult<Arc<dyn Plugin>> {
            let probe = Arc::new(Probe {
                active: AtomicBool::new(false),
                revoked: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                ready: tokio::sync::Notify::new(),
            });
            resources.own("background", probe.clone()).unwrap();
            *self.observer.lock().unwrap() = Some(probe.clone());
            let tasks = ctx.tasks.clone();
            let lease = tasks.lease().unwrap();
            tokio::spawn(async move {
                let _lease = lease;
                let _ = tasks
                    .run(async {
                        probe.active.store(true, Ordering::SeqCst);
                        let _running = Running(probe.clone());
                        probe.ready.notify_one();
                        std::future::pending::<()>().await;
                    })
                    .await;
            });
            Ok(Arc::new(Instance(self.manifest.clone())))
        }
    }
    let definition = Arc::new(Definition {
        manifest: PluginManifest::new(
            pid("module.portable_background"),
            "1.0.0",
            "Owned portable background",
        ),
        observer: Mutex::new(None),
    });
    let host = PortableHost::assemble(
        builder(Provider::new(vec![])).with_plugin(module_from_definition(definition.clone())),
        AgentHostPolicy::default(),
        AgentConfig::default(),
    )
    .await
    .unwrap();
    let external = definition.observer.lock().unwrap().clone().unwrap();
    tokio::time::timeout(Duration::from_secs(2), external.ready.notified())
        .await
        .unwrap();
    assert!(external.active.load(Ordering::SeqCst));
    assert_eq!(
        host.submit("minimal owned resources").await.unwrap().text,
        "done"
    );
    host.shutdown().await.unwrap();
    assert!(external.revoked.load(Ordering::SeqCst));
    assert!(external.closed.load(Ordering::SeqCst));
    assert!(!external.active.load(Ordering::SeqCst));
}

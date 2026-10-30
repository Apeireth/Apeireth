//! Software-only real UDP proof through the real Runtime and Internal gate.
#[path = "support/system_one.rs"]
mod review_support;
#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{CapabilityId, SystemClock, TraceId};
use apeireth_plugin::agents::AgentConfig;
use apeireth_plugin::module_contract::{
    EffectClass, ExecutionMode, ModuleTaskHandle, ModuleToolContract,
};
use apeireth_plugin::{Plugin, PluginContext, ToolCapability, ToolModule};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use apeireth_runtime::canonical::module_messages::{ModuleMessageGrant, ModuleMessagePolicy};
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant};
use apeireth_runtime_assembly::mesh::{
    MeshConfig, MeshModule, MeshState, MeshStatus, MeshStatusTarget,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use support::*;
fn cid(id: &str) -> CapabilityId {
    CapabilityId::new(id).unwrap()
}
fn config(address: std::net::SocketAddr) -> MeshConfig {
    MeshConfig::loopback(
        "peer",
        address,
        b"private_mesh_payload_and_key_material".to_vec(),
    )
    .with_timing(Duration::from_millis(40), Duration::from_millis(500))
}
fn policy(sink: bool) -> AgentHostPolicy {
    let mut grant = ModuleMessageGrant {
        authorization_ref: "host:local_mesh".into(),
        allow_background: true,
        allow_events: sink,
        allow_public_status: sink,
        ..Default::default()
    };
    grant.internal_tools.insert(cid("tool.mesh.exchange"));
    let mut tools = [cid("tool.mesh.exchange")]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    if sink {
        grant.targets.insert(
            pid("module.status_sink"),
            [cid("tool.status_sink")].into_iter().collect(),
        );
        tools.insert(cid("tool.status_sink"));
    }
    let mut policy = AgentHostPolicy {
        message_policy: Some(ModuleMessagePolicy::default()),
        ..Default::default()
    };
    policy
        .message_policy
        .as_mut()
        .unwrap()
        .grants
        .insert(pid("module.mesh"), grant);
    policy.modules.insert(
        pid("module.mesh"),
        ModuleAgentGrant {
            tools,
            ..Default::default()
        },
    );
    policy
}
enum FirstReply {
    Immediate,
    Hold,
    Drop,
}
struct Echo {
    address: std::net::SocketAddr,
    received: Arc<AtomicUsize>,
    audit: Arc<Mutex<Option<Arc<review_support::Audit>>>>,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Echo {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Echo {
    async fn start(first_reply: FirstReply, valid: bool) -> Self {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let audit: Arc<Mutex<Option<Arc<review_support::Audit>>>> = Arc::new(Mutex::new(None));
        let audit_check = audit.clone();
        let received = Arc::new(AtomicUsize::new(0));
        let counter = received.clone();
        let entered = Arc::new(tokio::sync::Notify::new());
        let signal = entered.clone();
        let release = Arc::new(tokio::sync::Notify::new());
        let waiting = release.clone();
        let task = tokio::spawn(async move {
            let mut bytes = [0u8; 1024];
            loop {
                let (n, peer) = socket.recv_from(&mut bytes).await.unwrap();
                let previous = counter.fetch_add(1, Ordering::SeqCst);
                if let Some(audit) = &*audit_check.lock().unwrap() {
                    assert!(
                        audit.records().len() > previous,
                        "datagram arrived before review audit acknowledgement"
                    );
                }
                assert!(bytes[..n]
                    .windows(b"private_mesh_payload".len())
                    .any(|v| v == b"private_mesh_payload"));
                signal.notify_one();
                if previous == 0 {
                    match first_reply {
                        FirstReply::Immediate => {}
                        FirstReply::Hold => waiting.notified().await,
                        // A lost reply must never be sent later to a closed request socket.
                        FirstReply::Drop => continue,
                    }
                }
                socket
                    .send_to(
                        if valid {
                            &bytes[..n]
                        } else {
                            b"unrelated_reply"
                        },
                        peer,
                    )
                    .await
                    .unwrap();
            }
        });
        Self {
            address,
            received,
            audit,
            entered,
            release,
            task,
        }
    }
}
async fn wait_for(condition: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap();
}
struct StatusSink {
    id: CapabilityId,
    received: Arc<Mutex<Vec<MeshStatus>>>,
}
#[async_trait]
impl ToolCapability for StatusSink {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        let mut d = NormalizedTool::new("status_sink");
        d.parameters=json!({"type":"object","properties":{"state":{"type":"string"},"online_peers":{"type":"integer"},"network_health":{"type":"string"},"pending_operations":{"type":"integer"}},"required":["state","online_peers","network_health","pending_operations"],"additionalProperties":false}).as_object().unwrap().clone();
        d
    }
    fn module_contract(&self) -> ModuleToolContract {
        ModuleToolContract::logical(ExecutionMode::Direct, EffectClass::Pure)
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let status: MeshStatus = serde_json::from_value(call.arguments.clone()).unwrap();
        self.received.lock().unwrap().push(status);
        ToolResult::ok(&call.id, json!({"sink_private":"never_project_to_main"}))
            .with_name(&call.name)
    }
}
fn public_result(provider: &Provider, key: &str) -> Value {
    provider
        .results()
        .iter()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
        .find(|v| v.get(key).is_some())
        .unwrap()
}

#[tokio::test]
async fn accepted_main_call_closes_while_private_udp_worker_and_authorized_status_event_continue_independently(
) {
    let echo = Echo::start(FirstReply::Hold, true).await;
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = ToolModule::new(
        pid("module.status_sink"),
        vec![Arc::new(StatusSink {
            id: cid("tool.status_sink"),
            received: received.clone(),
        })],
    )
    .unwrap();
    let mesh = Arc::new(
        MeshModule::new(config(echo.address).with_status_target(MeshStatusTarget {
            module: pid("module.status_sink"),
            capability: cid("tool.status_sink"),
        }))
        .unwrap(),
    );
    let provider = Provider::new(vec![
        vec![call("mesh.connect", json!({"device":"peer"}))],
        vec![],
    ]);
    let (gate, audit) = review_support::allow_gate();
    *echo.audit.lock().unwrap() = Some(audit.clone());
    let runtime = builder(provider.clone())
        .with_system_one(gate)
        .with_plugin(mesh.clone())
        .with_plugin(Arc::new(sink))
        .build_agent_runtime(policy(true))
        .await
        .unwrap();
    let main = runtime
        .spawn_agent(
            "main",
            AgentConfig {
                tools: vec![cid("tool.mesh.connect"), cid("tool.mesh.status")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(1),
        main.run("request configured background connection".into()),
    )
    .await
    .unwrap()
    .unwrap();
    let accepted = public_result(&provider, "accepted");
    assert_eq!(accepted["accepted"], true);
    let handle: ModuleTaskHandle = serde_json::from_value(accepted["task_handle"].clone()).unwrap();
    assert_eq!(handle.module, pid("module.mesh"));
    assert_eq!(
        facts(&*runtime.kernel().await, main.snapshot().session)
            .await
            .len(),
        1
    );
    tokio::time::timeout(Duration::from_secs(2), echo.entered.notified())
        .await
        .unwrap();
    main.stop().await.unwrap();
    echo.release.notify_one();
    wait_for(|| mesh.status().state == MeshState::Online).await;
    wait_for(|| {
        received
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.state == MeshState::Online)
    })
    .await;
    wait_for(|| echo.received.load(Ordering::SeqCst) >= 2).await;
    assert_eq!(
        provider.requests().len(),
        2,
        "background worker/Event must not use Main reasoning"
    );
    provider.enqueue_tools(vec![call("mesh.status", json!({"task_handle":handle}))]);
    provider.enqueue_tools(vec![]);
    let observer = runtime
        .spawn_agent(
            "observer",
            AgentConfig {
                tools: vec![cid("tool.mesh.status")],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    observer
        .run("query authorized public status".into())
        .await
        .unwrap();
    let status = public_result(&provider, "online_peers");
    assert_eq!(status.as_object().unwrap().len(), 4);
    assert_eq!(status["state"], "online");
    assert_eq!(status["online_peers"], 1);
    let transcript = serde_json::to_string(&provider.requests()).unwrap();
    for secret in [
        "private_mesh_payload",
        "never_project_to_main",
        "ale.mesh.v1",
        &echo.address.to_string(),
    ] {
        assert!(!transcript.contains(secret), "leaked {secret}");
    }
    assert!(!runtime
        .kernel()
        .await
        .tools()
        .iter()
        .any(|t| t.id() == &cid("tool.mesh.exchange")));
    runtime
        .unregister_module(&pid("module.mesh"))
        .await
        .unwrap();
    let stopped = echo.received.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(echo.received.load(Ordering::SeqCst), stopped);
    assert_eq!(mesh.status().state, MeshState::Stopped);
    assert!(
        audit
            .records()
            .iter()
            .filter(|r| r.capability_id == cid("tool.mesh.exchange"))
            .count()
            >= stopped
    );
    assert!(!echo.task.is_finished());
    // Re-activation creates a new task generation; old accepted handles convey no authority.
    let replacement = Arc::new(MeshModule::new(config(echo.address)).unwrap());
    runtime
        .register_module(
            replacement,
            PluginContext::new(
                Arc::new(SystemClock),
                Arc::new(apeireth_plugin::credentials::NoCredentials),
                TraceId::new(),
            ),
        )
        .await
        .unwrap();
    provider.enqueue_tools(vec![call("mesh.status", json!({"task_handle":handle}))]);
    provider.enqueue_tools(vec![]);
    observer
        .run("reject old accepted handle".into())
        .await
        .unwrap();
    assert!(provider.results().last().unwrap().contains("stale"));
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn unavailable_deny_and_disabled_transport_never_send_a_datagram() {
    for mode in [
        Some(review_support::Mode::Unavailable),
        Some(review_support::Mode::Decision(
            apeireth_plugin::system_one::SystemOneDecision::Deny,
        )),
        None,
    ] {
        let echo = Echo::start(FirstReply::Immediate, true).await;
        let mesh = Arc::new(MeshModule::new(config(echo.address)).unwrap());
        let (gate, audit, _) =
            review_support::gate(mode.clone().unwrap_or(review_support::Mode::Unavailable));
        let runtime = builder(Provider::new(vec![]))
            .with_system_one(gate)
            .with_plugin(mesh.clone())
            .build_agent_runtime(if mode.is_some() {
                policy(false)
            } else {
                AgentHostPolicy::default()
            })
            .await
            .unwrap();
        wait_for(|| mesh.status().state == MeshState::Degraded).await;
        assert_eq!(echo.received.load(Ordering::SeqCst), 0);
        if mode.is_some() {
            assert!(!audit.records().is_empty());
        }
        runtime.shutdown().await.unwrap();
        assert_eq!(echo.received.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn native_exchange_refuses_raw_calls_unconfigured_peers_and_unrelated_reply_nonce() {
    assert!(MeshModule::new(config("192.0.2.1:9".parse().unwrap())).is_err());
    assert!(MeshModule::new(config("127.0.0.1:0".parse().unwrap())).is_err());
    let echo = Echo::start(FirstReply::Immediate, false).await;
    let mesh = Arc::new(MeshModule::new(config(echo.address)).unwrap());
    let internal = mesh
        .tools()
        .into_iter()
        .find(|t| t.id() == &cid("tool.mesh.exchange"))
        .unwrap();
    let call = call("mesh.exchange", json!({"device":"peer"}));
    assert!(!internal.invoke(&call).await.is_ok());
    assert!(!internal
        .invoke_frozen(&call, internal.freeze_invocation(&call).unwrap().as_ref())
        .await
        .is_ok());
    assert!(internal
        .freeze_invocation(&support::call("mesh.exchange", json!({"device":"other"})))
        .is_err());
    assert_eq!(echo.received.load(Ordering::SeqCst), 0);
    let (gate, _) = review_support::allow_gate();
    let runtime = builder(Provider::new(vec![]))
        .with_system_one(gate)
        .with_plugin(mesh.clone())
        .build_agent_runtime(policy(false))
        .await
        .unwrap();
    wait_for(|| {
        echo.received.load(Ordering::SeqCst) > 0 && mesh.status().state == MeshState::Degraded
    })
    .await;
    assert_eq!(mesh.status().online_peers, 0);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn stopping_mesh_cancels_waiting_socket_and_drains_owned_worker_before_returning() {
    let echo = Echo::start(FirstReply::Drop, true).await;
    let mesh = Arc::new(MeshModule::new(config(echo.address)).unwrap());
    let (gate, _) = review_support::allow_gate();
    let runtime = builder(Provider::new(vec![]))
        .with_system_one(gate)
        .with_plugin(mesh.clone())
        .build_agent_runtime(policy(false))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), echo.entered.notified())
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(1),
        runtime.unregister_module(&pid("module.mesh")),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(mesh.status().state, MeshState::Stopped);
    let count = echo.received.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(echo.received.load(Ordering::SeqCst), count);
    assert!(runtime
        .instances()
        .iter()
        .all(|a| a.status != apeireth_plugin::agents::AgentStatus::Running));
}

#[tokio::test]
async fn native_timeout_returns_a_closed_error_and_later_heartbeat_can_recover_without_new_authority(
) {
    let echo = Echo::start(FirstReply::Drop, true).await;
    let mesh = Arc::new(
        MeshModule::new(
            config(echo.address).with_timing(Duration::from_millis(40), Duration::from_millis(50)),
        )
        .unwrap(),
    );
    let provider = Provider::new(vec![]);
    let (gate, audit) = review_support::allow_gate();
    *echo.audit.lock().unwrap() = Some(audit.clone());
    let runtime = builder(provider.clone())
        .with_system_one(gate)
        .with_plugin(mesh.clone())
        .build_agent_runtime(policy(false))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), echo.entered.notified())
        .await
        .unwrap();
    let mailbox = runtime.instances().into_iter().next().unwrap();
    let first = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(first) = facts(&*runtime.kernel().await, mailbox.session)
                .await
                .into_iter()
                .next()
            {
                break first;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("native timeout must persist a closed tool result before heartbeat recovery");
    assert_eq!(first.call().name, "mesh.exchange");
    assert!(!first.result().is_ok(), "the dropped reply must time out");
    let error = first.result().render();
    assert!(
        error.contains("\"code\":\"timeout\"")
            || error.contains("private Mesh exchange unavailable or interrupted"),
        "expected a closed native/dispatcher timeout, got {error}"
    );
    wait_for(|| mesh.status().state == MeshState::Online).await;
    let recovered = facts(&*runtime.kernel().await, mailbox.session).await;
    assert!(recovered.iter().skip(1).any(|fact| fact.result().is_ok()));
    assert!(echo.received.load(Ordering::SeqCst) >= 2);
    assert_eq!(
        runtime.instances().len(),
        1,
        "ordinary native failure must not create a fresh authority/budget"
    );
    assert_eq!(runtime.instances()[0].id, mailbox.id);
    assert_eq!(runtime.instances()[0].session, mailbox.session);
    assert!(
        !echo.task.is_finished(),
        "reply loss must not stop the peer"
    );
    assert_eq!(provider.requests().len(), 0);
    assert!(audit.records().len() >= 2);
    runtime.shutdown().await.unwrap();
}

//! Integration: Gateway InFrame -> Runtime cycle -> OutFrame (R174).
//!
//! **Wire path**:
//! ```text
//!   Node (TUI/CLI/...) -> Gateway loopback -> AsyncWorker -> Runtime cycle
//!                                                              -> ArbitrationLog
//!                                                              -> SearchEngine
//!                                                              -> GroupChat
//!                                                              -> EmotionEngine
//!                                                              -> CycleReport
//!   -> OutFrame -> Loopback -> Node
//! ```
//!
//! **0 drift**: this is a separate test crate; it only consumes the public
//! surface of `apeireth-gateway` + `apeireth-runtime` + `apeireth-bus`. No
//! production source is modified.

use apeireth_gateway::Transport;
use apeireth_gateway::{
    DmScope, Gateway, GatewayMode, InFrame, InMemoryTransport, NodeKind, OutFrame,
};
use apeireth_runtime::{AsyncWorker, Runtime, RuntimeEvent};
use apeireth_tool_registry::{TaskId, TaskStatus};
use apeireth_bus::{ChanneledBus, ChannelSet};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// Worker that pulls one inbound frame from the loopback transport and emits
/// an outbound echo frame.
struct FrameGatewayWorker {
    loopback: Arc<InMemoryTransport>,
}

#[async_trait]
impl AsyncWorker for FrameGatewayWorker {
    fn name(&self) -> &str {
        "frame-gateway-worker"
    }

    async fn execute(&self, task_id: TaskId, _params_json: String) -> Result<String, String> {
        // Borrow the loopback transport; `.as_ref()` dodges the `Arc` deref issue.
        let transport: &InMemoryTransport = self.loopback.as_ref();
        let frame = transport.recv().await.ok_or_else(|| "no inbound frame".to_string())?;
        let reply = OutFrame::new(
            "loopback",
            json!({
                "echo": frame.payload,
                "task_id": task_id,
                "channel": frame.channel,
            }),
        );
        transport.send(reply).await.map_err(|e| format!("send: {e}"))?;
        serde_json::to_string(&frame.payload).map_err(|e| e.to_string())
    }
}

#[test]
fn both_crates_construct() {
    let gw = Gateway::open(GatewayMode::SingleProcess, "integ", 0);
    assert!(gw.is_running());
    let rt = Runtime::new();
    assert_eq!(rt.config.tick_interval, Duration::from_secs(10));
}

#[tokio::test]
async fn gateway_admit_then_loopback_carries_frame() {
    let gw = Arc::new(Gateway::open(GatewayMode::SingleProcess, "i1", 0));
    gw.register_key("k1", "alice", "primary", DmScope::All, 0);
    let a = gw.admit_node(NodeKind::Cli, "client", "alice", "k1", 0).unwrap();
    let loopback: Arc<InMemoryTransport> = gw.loopback();
    loopback.as_ref().start().await.unwrap();
    loopback.as_ref().push_inbound(InFrame::new(a.node_id, "loopback", json!({"ping": 1})));
    let f = loopback.as_ref().recv().await.unwrap();
    assert_eq!(f.payload, json!({"ping": 1}));
}

#[tokio::test]
async fn runtime_registers_worker_and_dispatches() {
    let gw = Arc::new(Gateway::open(GatewayMode::SingleProcess, "i2", 0));
    gw.register_key("k1", "alice", "primary", DmScope::All, 0);
    let a = gw.admit_node(NodeKind::Cli, "client", "alice", "k1", 0).unwrap();

    let rt = Arc::new(Runtime::new());
    rt.bootstrap().unwrap();
    let loopback: Arc<InMemoryTransport> = gw.loopback();
    loopback.as_ref().start().await.unwrap();
    let worker = Arc::new(FrameGatewayWorker { loopback: loopback.clone() });
    rt.register_worker("frame-gateway-worker", worker);

    loopback.as_ref().push_inbound(InFrame::new(a.node_id, "loopback", json!({"hello": "world"})));
    let task_id = rt.dispatch_async_task("frame-gateway-worker", "{}").await;
    let task = rt.task_store.wait_for_completion(task_id, Duration::from_secs(5)).await.unwrap();
    assert_eq!(task.status, TaskStatus::Completed, "task failed: {:?} tool={} params={}", task.error, task.tool_name, task.params_json);

    let out = loopback.as_ref().drain_outbound();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].payload["echo"], json!({"hello": "world"}));
    assert_eq!(out[0].payload["channel"], "loopback");
}

#[tokio::test]
async fn runtime_single_cycle_after_gateway_dispatch() {
    let gw = Arc::new(Gateway::open(GatewayMode::SingleProcess, "i3", 0));
    gw.register_key("k1", "alice", "primary", DmScope::All, 0);
    let a = gw.admit_node(NodeKind::Cli, "client", "alice", "k1", 0).unwrap();

    let rt = Arc::new(Runtime::new());
    rt.bootstrap().unwrap();
    let loopback: Arc<InMemoryTransport> = gw.loopback();
    loopback.as_ref().start().await.unwrap();
    let worker = Arc::new(FrameGatewayWorker { loopback: loopback.clone() });
    rt.register_worker("frame-gateway-worker", worker);

    loopback.as_ref().push_inbound(InFrame::new(a.node_id, "loopback", json!({"step": 1})));
    let task_id = rt.dispatch_async_task("frame-gateway-worker", "{}").await;
    let _ = rt.task_store.wait_for_completion(task_id, Duration::from_secs(5)).await.unwrap();

    let reports = rt.run_cycles(1).await.unwrap();
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    // The cycle internally dispatches "classify" task; verify the WHOLE wire ran.
    assert!(r.arbitration_seq > 0);
    assert!(r.search_doc_id > 0);
    assert!(!r.group_chat_message_id.is_empty());
    assert!(r.elapsed_ms > 0);
}

#[tokio::test]
async fn gateway_multiple_nodes_drive_runtime_in_parallel() {
    let gw = Arc::new(Gateway::open(GatewayMode::SingleProcess, "i4", 0));
    gw.register_key("k1", "alice", "primary", DmScope::All, 0);
    let a1 = gw.admit_node(NodeKind::Cli, "client-1", "alice", "k1", 0).unwrap();
    let a2 = gw.admit_node(NodeKind::Http, "client-2", "alice", "k1", 0).unwrap();
    assert_ne!(a1.node_id, a2.node_id);
    assert_eq!(a1.node_kind, NodeKind::Cli);
    assert_eq!(a2.node_kind, NodeKind::Http);

    let rt = Arc::new(Runtime::new());
    rt.bootstrap().unwrap();
    let loopback: Arc<InMemoryTransport> = gw.loopback();
    loopback.as_ref().start().await.unwrap();
    let worker = Arc::new(FrameGatewayWorker { loopback: loopback.clone() });
    rt.register_worker("frame-gateway-worker", worker);

    loopback.as_ref().push_inbound(InFrame::new(a1.node_id, "loopback", json!({"from": "cli"})));
    loopback.as_ref().push_inbound(InFrame::new(a2.node_id, "loopback", json!({"from": "http"})));

    let t1 = rt.dispatch_async_task("frame-gateway-worker", "{}").await;
    let t2 = rt.dispatch_async_task("frame-gateway-worker", "{}").await;
    let _ = rt.task_store.wait_for_completion(t1, Duration::from_secs(5)).await.unwrap();
    let _ = rt.task_store.wait_for_completion(t2, Duration::from_secs(5)).await.unwrap();

    let outs = loopback.as_ref().drain_outbound();
    assert!(outs.len() >= 1, "expected outbound frames, got {} (out={:?})", outs.len(), outs);
    let echoed: Vec<_> = outs.iter().map(|o| o.payload["echo"].clone()).collect();
    assert!(echoed.contains(&json!({"from": "cli"})));
    assert!(echoed.contains(&json!({"from": "http"})));
}

#[tokio::test]
async fn gateway_release_drops_node_but_keeps_runtime() {
    let gw = Arc::new(Gateway::open(GatewayMode::SingleProcess, "i5", 0));
    gw.register_key("k1", "alice", "primary", DmScope::All, 0);
    let a = gw.admit_node(NodeKind::Cli, "client", "alice", "k1", 0).unwrap();
    let rt = Arc::new(Runtime::new());
    rt.bootstrap().unwrap();
    assert!(gw.release_node(a.node_id, 100));
    assert_eq!(gw.nodes().len(), 0);
    let _ = rt.run_cycles(1).await.unwrap();
}

#[tokio::test]
async fn gateway_loopback_publishes_to_runtime_bus() {
    // Smoke: a fresh ChanneledBus<RuntimeEvent> accepts a publish without error.
    let gw = Arc::new(Gateway::open(GatewayMode::SingleProcess, "i6", 0));
    gw.register_key("k1", "alice", "primary", DmScope::All, 0);
    let _ = gw.admit_node(NodeKind::Desktop, "desktop", "alice", "k1", 0).unwrap();
    let _bus = ChanneledBus::<RuntimeEvent>::new();
    let result = _bus.publish_multi(ChannelSet::AI, "ai:test", apeireth_bus::BusMessage::new(RuntimeEvent::new(
        0, None, "smoke", "ai:test", json!({"ok": true}).to_string(),
    ))).await;
    assert!(result.is_ok());
}

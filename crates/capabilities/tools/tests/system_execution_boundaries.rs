//! Real loopback sockets and the production listener probe; no public scanning.
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use apeireth_core::kernel::{AgentId, PluginId};
use apeireth_plugin::task_scope::ModuleTaskScope;
use apeireth_plugin::tool_execution::{native_execution_context, scope_native_execution};
use apeireth_plugin::{
    ToolAuthorization, ToolCaller, ToolCancellation, ToolCapability, ToolExecutionContext,
};
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::exposure_audit::listener_surface::{
    probe_netstat_outcome, ListenerProbeError,
};
use apeireth_tools_canonical::exposure_audit::{
    listener_item, parse_netstat_snapshot, ExposureAuditConfig, ExposureAuditTool, ItemStatus,
};
use apeireth_tools_canonical::recon::Scope;
use apeireth_tools_canonical::recon_tool::ReconNetTool;
use serde_json::json;

fn context() -> ToolExecutionContext {
    ToolExecutionContext {
        agent: AgentId::default(),
        principal: None,
        session: None,
        turn: None,
        call_id: "boundary-fixture".into(),
        caller: ToolCaller::Host,
        module: PluginId::new("test.system-boundaries").unwrap(),
        module_instance: None,
        internal_review: None,
        message_support: false,
        message_route: None,
        delegated_tools: None,
        delegation: None,
        tool_scope: None,
        authorization: ToolAuthorization::default(),
        cancellation: ToolCancellation::default(),
        deadline: Instant::now() + Duration::from_secs(10),
        trace: None,
        module_tasks: ModuleTaskScope::new(),
        resource_lease: None,
    }
}

/// Keeps accepted loopback connections silent until the test releases them.
struct SilentListener {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl SilentListener {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = std::thread::spawn(move || {
            let mut connections: Vec<TcpStream> = Vec::new();
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => connections.push(stream),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
        });
        Self {
            port,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for SilentListener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn tool_and_call(port: u16, count: usize) -> (ReconNetTool, ToolCall) {
    let scope = Scope {
        name: "loopback-fixture".into(),
        authorized: true,
        paths: vec![],
        domains: vec![],
        cidrs: vec!["127.0.0.1/32".into()],
    };
    let call = ToolCall {
        id: "boundary-fixture".into(),
        name: "re.net.recon".into(),
        arguments: json!({"target":"127.0.0.1", "ports":vec![port;count], "timeout_ms":5000}),
    };
    (ReconNetTool::new(Arc::new(scope)), call)
}

#[tokio::test(flavor = "current_thread")]
async fn recon_real_loopback_scan_keeps_worker_heartbeat_moving() {
    let fixture = SilentListener::new();
    let (tool, call) = tool_and_call(fixture.port, 8);
    let started = Instant::now();
    let (result, heartbeat) = tokio::join!(tool.invoke(&call), async {
        tokio::time::sleep(Duration::from_millis(20)).await;
        started.elapsed()
    });
    assert!(result.is_ok(), "{}", result.render());
    assert!(
        heartbeat < Duration::from_millis(400),
        "worker blocked for {heartbeat:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn recon_real_loopback_scan_obeys_whole_call_deadline() {
    let fixture = SilentListener::new();
    let (tool, call) = tool_and_call(fixture.port, 16);
    let mut context = context();
    context.deadline = Instant::now() + Duration::from_millis(100);
    let started = Instant::now();
    let result = scope_native_execution(&context, tool.invoke(&call)).await;
    assert!(
        !result.is_ok(),
        "incomplete scan must not be returned as complete evidence"
    );
    assert!(result.render().contains("deadline"), "{}", result.render());
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(context.module_tasks.active_tasks(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn recon_cancellation_interrupts_inflight_passive_read() {
    let fixture = SilentListener::new();
    let (tool, call) = tool_and_call(fixture.port, 16);
    let context = context();
    let signal = context.cancellation.clone();
    let cancelling = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        signal.cancel();
    });
    let started = Instant::now();
    let result = scope_native_execution(&context, tool.invoke(&call)).await;
    cancelling.join().unwrap();
    assert!(!result.is_ok(), "cancelled scan returned complete evidence");
    assert!(result.render().contains("cancel"), "{}", result.render());
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(context.module_tasks.active_tasks(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn recon_scan_is_owned_and_drained_when_module_closes() {
    let fixture = SilentListener::new();
    let (tool, call) = tool_and_call(fixture.port, 16);
    let context = context();
    let scan = scope_native_execution(&context, tool.invoke(&call));
    let (result, ()) = tokio::join!(scan, async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            context.module_tasks.active_tasks() > 0,
            "scan lacks module task ownership"
        );
        tokio::time::timeout(
            Duration::from_millis(400),
            context.module_tasks.close_and_wait(),
        )
        .await
        .expect("scan did not drain after module cancellation");
    });
    assert!(!result.is_ok());
    assert_eq!(context.module_tasks.active_tasks(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_recon_call_leaves_no_background_scan() {
    let fixture = SilentListener::new();
    let (tool, call) = tool_and_call(fixture.port, 16);
    let context = context();
    {
        let scan = scope_native_execution(&context, tool.invoke(&call));
        tokio::pin!(scan);
        tokio::select! {
            result = &mut scan => panic!("scan completed before drop fixture: {}", result.render()),
            _ = tokio::time::sleep(Duration::from_millis(30)) => {}
        }
        assert!(context.module_tasks.active_tasks() > 0);
    }
    assert_eq!(
        context.module_tasks.active_tasks(),
        0,
        "dropped async scan still owns native work"
    );
}

#[tokio::test]
async fn netstat_precancelled_call_is_unmeasured() {
    let context = context();
    context.cancellation.cancel();
    let snapshot = scope_native_execution(&context, async { probe_netstat_outcome() }).await;
    let error = snapshot
        .as_ref()
        .expect_err("pre-cancelled probe must not execute");
    assert_eq!(
        *error,
        ListenerProbeError::Cancelled,
        "pre-cancelled probe must report Cancelled"
    );
    assert_eq!(
        listener_item(
            snapshot
                .map(|text| parse_netstat_snapshot(&text))
                .map_err(|error| error.to_string())
        )
        .status,
        ItemStatus::NotMeasured
    );
}

#[tokio::test]
async fn netstat_expired_call_is_unmeasured() {
    let mut context = context();
    context.deadline = Instant::now() - Duration::from_millis(1);
    let snapshot = scope_native_execution(&context, async { probe_netstat_outcome() }).await;
    let error = snapshot
        .as_ref()
        .expect_err("expired probe must not execute");
    assert_eq!(
        *error,
        ListenerProbeError::DeadlineExceeded,
        "expired probe must report DeadlineExceeded"
    );
    assert_eq!(
        listener_item(
            snapshot
                .map(|text| parse_netstat_snapshot(&text))
                .map_err(|error| error.to_string())
        )
        .status,
        ItemStatus::NotMeasured
    );
}

#[tokio::test]
async fn exposure_audit_transfers_native_identity_and_lease_to_real_probe_worker() {
    let root = tempfile::tempdir().unwrap();
    let config = ExposureAuditConfig::new(root.path()).with_listeners(Arc::new(|| {
        let native = native_execution_context().expect("probe worker lost call context");
        assert!(
            native.tasks.active_tasks() > 0,
            "probe worker lacks native lease"
        );
        Ok(parse_netstat_snapshot(""))
    }));
    let tool = ExposureAuditTool::new(config);
    let call = ToolCall {
        id: "exposure-boundary".into(),
        name: "exposure_audit".into(),
        arguments: json!({}),
    };
    let context = context();
    let result = scope_native_execution(&context, tool.invoke(&call)).await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(context.module_tasks.active_tasks(), 0);
}

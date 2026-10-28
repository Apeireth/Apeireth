#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{Lifecycle, SessionId};
use apeireth_plugin::ToolCapability;
use apeireth_runtime::TurnRequest;
use apeireth_tools_canonical::mcp_bridge::{
    InProcessMcpServer, McpBridgeOptions, McpPlugin, McpServerConfig, McpServerConnection,
    McpServerSpec, McpToolBridge, MockToolSpec,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use support::*;

fn bridge(server: &Arc<InProcessMcpServer>) -> Arc<McpToolBridge> {
    let spec = McpServerSpec::stdio("demo", "in-process-only");
    let mut options = McpBridgeOptions::default();
    options.readonly_preset = true;
    let connection = Arc::new(McpServerConnection::new(
        spec.clone(),
        server.factory(),
        &options,
    ));
    Arc::new(
        McpToolBridge::from_connections(
            McpServerConfig::new(vec![spec]),
            options,
            vec![connection],
        )
        .unwrap(),
    )
}
#[tokio::test]
async fn managed_mcp_discovers_dispatches_updates_revokes_handles_reconnects_and_unloads() {
    let server = InProcessMcpServer::new("demo");
    server.set_tools(vec![MockToolSpec::new("echo", "owned_reply").read_only()]);
    let bridge = bridge(&server);
    let provider = Provider::new(vec![vec![call("mcp:demo:echo", json!({}))]]);
    let module = Arc::new(McpPlugin::new(bridge.clone()));
    let mut runtime = builder(provider.clone())
        .with_plugin(module.clone())
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "echo"))
        .await
        .unwrap();
    assert_eq!(server.call_log().len(), 1);
    assert!(provider
        .results()
        .iter()
        .any(|text| text.contains("owned_reply")));
    assert!(facts(&runtime,session).await.iter().any(|fact|matches!(fact.source(),apeireth_protocol::canonical::ToolEventSourceV1::Model{module_id:Some(id)} if id==&pid("module.mcp"))));
    let cached = runtime
        .tools()
        .into_iter()
        .find(|t| t.declaration().name == "mcp:demo:echo")
        .unwrap();
    server.set_tools(vec![MockToolSpec::new("other", "updated").read_only()]);
    runtime
        .refresh_module_tools(&pid("module.mcp"))
        .await
        .unwrap();
    assert!(!cached
        .invoke(&call("mcp:demo:echo", json!({})))
        .await
        .is_ok());
    assert!(runtime
        .tool_declarations()
        .iter()
        .all(|t| t.name != "mcp:demo:echo"));
    assert!(runtime
        .tool_declarations()
        .iter()
        .any(|t| t.name == "mcp:demo:other"));
    server.drop_next_requests(1);
    runtime
        .refresh_module_tools(&pid("module.mcp"))
        .await
        .unwrap();
    assert!(server.opened_channels() >= 2);
    runtime.stop_module(&pid("module.mcp")).await.unwrap();
    assert!(runtime.tools().is_empty());
    assert!(bridge.connections()[0]
        .call_tool("other", json!({}))
        .await
        .is_err());
    runtime.unregister_module(&pid("module.mcp")).unwrap();
    let context = apeireth_plugin::PluginContext::new(
        runtime.clock().clone(),
        Arc::new(apeireth_plugin::credentials::NoCredentials),
        apeireth_core::kernel::TraceId::new(),
    );
    runtime.register_module(module, &context).await.unwrap();
    assert!(runtime
        .tool_declarations()
        .iter()
        .any(|t| t.name == "mcp:demo:other"));
    assert_eq!(
        runtime.plugins().state(&pid("module.mcp")).unwrap(),
        Lifecycle::Active
    );
    assert!(runtime.shutdown().await.is_empty());
}
#[tokio::test]
async fn module_stop_cancels_in_flight_dynamic_call_and_does_not_allow_reconnect() {
    let server = InProcessMcpServer::new("demo");
    server.set_tools(vec![MockToolSpec::new("slow", "done")
        .read_only()
        .with_delay(Duration::from_secs(20))]);
    let bridge = bridge(&server);
    let mut runtime = builder(Provider::new(vec![]))
        .with_plugin(Arc::new(McpPlugin::new(bridge.clone())))
        .build()
        .await
        .unwrap();
    let tool = runtime
        .tools()
        .into_iter()
        .find(|t| t.declaration().name == "mcp:demo:slow")
        .unwrap();
    let worker = tokio::spawn(async move { tool.invoke(&call("mcp:demo:slow", json!({}))).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while server.call_log().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    runtime.cancel_module(&pid("module.mcp")).unwrap();
    assert!(!worker.await.unwrap().is_ok());
    runtime.stop_module(&pid("module.mcp")).await.unwrap();
    assert_eq!(
        runtime
            .plugins()
            .task_scope(&pid("module.mcp"))
            .unwrap()
            .active_tasks(),
        0
    );
    assert!(bridge.connections()[0].connect().await.is_err());
    runtime.unregister_module(&pid("module.mcp")).unwrap();
}
#[tokio::test]
async fn unavailable_optional_mcp_is_failed_observable_and_does_not_destroy_kernel() {
    let server = InProcessMcpServer::new("demo");
    server.drop_next_requests(100);
    let mut runtime = builder(Provider::new(vec![]))
        .with_plugin(Arc::new(McpPlugin::new(bridge(&server))))
        .build()
        .await
        .unwrap();
    assert_eq!(
        runtime.plugins().state(&pid("module.mcp")).unwrap(),
        Lifecycle::Failed
    );
    assert!(!runtime
        .plugins()
        .errors(&pid("module.mcp"))
        .unwrap()
        .is_empty());
    assert!(runtime.tools().is_empty());
    assert_eq!(
        runtime
            .execute(TurnRequest::new(SessionId::new(), "hello"))
            .await
            .unwrap()
            .text,
        "done"
    );
    assert!(runtime.shutdown().await.is_empty());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn streaming_transport_close_reaps_server_and_terminates_its_process_group() {
    use apeireth_tools_canonical::mcp_bridge::StdioChannel;
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("parent");
    let child = root.path().join("child");
    let script = format!(
        "echo $$ > '{}'; sleep 30 & echo $! > '{}'; wait",
        parent.display(),
        child.display()
    );
    let channel = StdioChannel::open("/bin/sh", &["-c".into(), script])
        .await
        .unwrap();
    let pids = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let (Ok(a), Ok(b)) = (
                std::fs::read_to_string(&parent),
                std::fs::read_to_string(&child),
            ) {
                if let (Ok(a), Ok(b)) = (a.trim().parse::<u32>(), b.trim().parse::<u32>()) {
                    break (a, b);
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    channel.close().await.unwrap();
    channel.close().await.unwrap();
    for pid in [pids.0, pids.1] {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                    Ok(s) if s.rsplit_once(')').unwrap().1.trim_start().starts_with('Z') => break,
                    _ => tokio::task::yield_now().await,
                }
            }
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn descriptor_risk_changes_do_not_reuse_old_approval_or_readonly_grants() {
    let server = InProcessMcpServer::new("demo");
    server.set_tools(vec![MockToolSpec::new("echo", "read").read_only()]);
    let bridge = bridge(&server);
    let mut runtime = builder(Provider::new(vec![]))
        .with_plugin(Arc::new(McpPlugin::new(bridge.clone())))
        .build()
        .await
        .unwrap();
    let original = runtime.tools().into_iter().next().unwrap();
    let request = call("mcp:demo:echo", json!({}));
    let receipt = original.freeze_invocation(&request).unwrap().unwrap();
    server.set_tools(vec![MockToolSpec::new("echo", "write")]);
    server.drop_next_requests(1);
    assert!(!original.invoke(&request).await.is_ok());
    assert!(server.call_log().is_empty());
    runtime
        .refresh_module_tools(&pid("module.mcp"))
        .await
        .unwrap();
    let updated = runtime.tools().into_iter().next().unwrap();
    assert!(!updated.invoke(&request).await.is_ok());
    assert!(server.call_log().is_empty());
    bridge
        .policy()
        .lock()
        .unwrap()
        .grant(apeireth_governance::Permission::ExecuteTool(
            updated.id().to_string(),
        ));
    assert!(!updated
        .invoke_frozen(&request, Some(&receipt))
        .await
        .is_ok());
    assert!(server.call_log().is_empty());
    assert!(updated.invoke(&request).await.is_ok());
    assert_eq!(server.call_log().len(), 1);
    assert!(runtime.shutdown().await.is_empty());
}

#[tokio::test]
async fn old_raw_mcp_tool_arc_stays_revoked_after_same_bridge_is_restarted() {
    let server = InProcessMcpServer::new("demo");
    server.set_tools(vec![MockToolSpec::new("echo", "first").read_only()]);
    let bridge = bridge(&server);
    let module = Arc::new(McpPlugin::new(bridge.clone()));
    let mut runtime = builder(Provider::new(vec![]))
        .with_plugin(module.clone())
        .build()
        .await
        .unwrap();
    // Keep the raw bridge Arc as well as dispatcher-managed handles.
    let old = bridge.tools().pop().unwrap();
    assert!(old.invoke(&call("mcp:demo:echo", json!({}))).await.is_ok());
    runtime.stop_module(&pid("module.mcp")).await.unwrap();
    assert!(!old.invoke(&call("mcp:demo:echo", json!({}))).await.is_ok());
    runtime.unregister_module(&pid("module.mcp")).unwrap();
    let context = apeireth_plugin::PluginContext::new(
        runtime.clock().clone(),
        Arc::new(apeireth_plugin::credentials::NoCredentials),
        apeireth_core::kernel::TraceId::new(),
    );
    runtime.register_module(module, &context).await.unwrap();
    assert!(
        !old.invoke(&call("mcp:demo:echo", json!({}))).await.is_ok(),
        "same tool name must not revive an old generation"
    );
    let fresh = runtime.tools().pop().unwrap();
    assert!(fresh
        .invoke(&call("mcp:demo:echo", json!({})))
        .await
        .is_ok());
    assert_eq!(server.call_log().len(), 2);
    assert!(runtime.shutdown().await.is_empty());
}

#[tokio::test]
async fn mcp_definition_builds_new_owned_connections_and_catalog_on_replacement() {
    use apeireth_plugin::resource_factory::module_from_definition;
    use apeireth_tools_canonical::mcp_bridge::McpDefinition;
    let server = InProcessMcpServer::new("demo");
    server.set_tools(vec![
        MockToolSpec::new("echo", "owned_factory_reply").read_only()
    ]);
    let options = McpBridgeOptions {
        readonly_preset: true,
        ..Default::default()
    };
    let definition = Arc::new(
        McpDefinition::new(
            McpServerConfig::new(vec![McpServerSpec::stdio("demo", "mock")]),
            options,
        )
        .unwrap()
        .with_channel_factories(vec![server.factory()])
        .unwrap(),
    );
    let module = module_from_definition(definition.clone());
    let provider = Provider::new(vec![vec![call("mcp:demo:echo", json!({}))]]);
    let mut runtime = builder(provider.clone())
        .with_plugin(module.clone())
        .build()
        .await
        .unwrap();
    runtime
        .execute(TurnRequest::new(SessionId::new(), "echo"))
        .await
        .unwrap();
    let old = module.tools().pop().unwrap();
    runtime.stop_module(&pid("module.mcp")).await.unwrap();
    assert!(module
        .resource_statuses()
        .iter()
        .all(|resource| resource.closed));
    runtime.unregister_module(&pid("module.mcp")).unwrap();
    let context = apeireth_plugin::PluginContext::new(
        runtime.clock().clone(),
        Arc::new(apeireth_plugin::credentials::NoCredentials),
        apeireth_core::kernel::TraceId::new(),
    );
    runtime
        .register_module(module_from_definition(definition), &context)
        .await
        .unwrap();
    assert!(!old.invoke(&call("mcp:demo:echo", json!({}))).await.is_ok());
    provider.enqueue_tools(vec![call("mcp:demo:echo", json!({}))]);
    runtime
        .execute(TurnRequest::new(SessionId::new(), "new echo"))
        .await
        .unwrap();
    assert_eq!(server.call_log().len(), 2);
    assert_eq!(server.opened_channels(), 2);
    assert!(runtime.shutdown().await.is_empty());
}

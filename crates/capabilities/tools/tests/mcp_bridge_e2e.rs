//! End-to-end tests for the MCP tool bridge.
//!
//! Every test runs against the in-process JSON-RPC server test double — no
//! sockets, no subprocesses, no network. The chain under test is the real
//! one: configuration surface → connection lifecycle → discovery → dynamic
//! registration → five-stage pipeline (risk mapping, guards, deadline,
//! correction channel, output normalization contract) → governance mapping.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::PermissionPolicy;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::exec_pipeline::{ExecutionStage, ToolOutcome};
use apeireth_tools_canonical::mcp_bridge::{
    mcp_output_schema, InProcessMcpServer, McpBridgeError, McpBridgeOptions, McpConfigError,
    McpDynamicTool, McpPermissionMapping, McpServerConfig, McpServerConnection, McpServerSpec,
    McpToolBridge, McpToolRegistry, McpTransportKind, MockToolSpec,
};

const SERVER: &str = "demo";

fn test_options() -> McpBridgeOptions {
    let reconnect = apeireth_plugin::mcp::ReconnectPolicy {
        initial_backoff: Duration::from_millis(1),
        multiplier_num: 1,
        multiplier_den: 1,
        max_backoff: Duration::from_millis(4),
        max_attempts: 3,
    };
    McpBridgeOptions::default()
        .with_call_timeout(200, 5_000)
        .with_reconnect(reconnect)
}

fn demo_spec() -> McpServerSpec {
    McpServerSpec::stdio(SERVER, "in-process-test-server")
}

/// Assemble a bridge over one in-process server (one connection per enabled
/// server, exactly like the production assembly does).
fn bridge_over(server: &Arc<InProcessMcpServer>, options: McpBridgeOptions) -> McpToolBridge {
    let config = McpServerConfig::new(vec![demo_spec()]);
    let connection = Arc::new(McpServerConnection::new(
        demo_spec(),
        server.factory(),
        &options,
    ));
    McpToolBridge::from_connections(config, options, vec![connection]).unwrap()
}

fn call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: name.into(),
        arguments,
    }
}

/// A registry recording what the bridge pushes into it; refuses duplicate
/// capability ids exactly like the live module bag does.
#[derive(Default)]
struct RecordingRegistry {
    tools: Mutex<BTreeMap<String, Arc<dyn ToolCapability>>>,
}

impl McpToolRegistry for RecordingRegistry {
    fn register_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String> {
        let mut tools = self.tools.lock().unwrap();
        if tools.contains_key(tool.id().as_str()) {
            return Err(format!("duplicate capability id {}", tool.id()));
        }
        tools.insert(tool.id().as_str().to_string(), tool);
        Ok(())
    }

    fn unregister_tool(&self, capability_id: &CapabilityId) {
        self.tools.lock().unwrap().remove(capability_id.as_str());
    }
}

#[tokio::test]
async fn discovery_registers_namespaced_dynamic_tools() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("lookup-item", "found it").with_description("Look one item up"),
        MockToolSpec::new("send-item", "sent").read_only(),
    ]);
    let bridge = bridge_over(&server, test_options());

    let report = bridge.refresh().await.unwrap();
    assert_eq!(
        report.registered_names(),
        ["mcp:demo:lookup-item", "mcp:demo:send-item"]
    );
    let entry = bridge.catalog().get("mcp:demo:lookup-item").unwrap();
    assert_eq!(entry.capability_id.as_str(), "tool.mcp.demo.lookup-item");
    assert_eq!(entry.remote_name, "lookup-item");
    assert!(!entry.declared_read_only);
    assert!(
        bridge
            .catalog()
            .get("mcp:demo:send-item")
            .unwrap()
            .declared_read_only
    );
    assert_eq!(report.live.len(), 2);
}

#[tokio::test]
async fn call_travels_the_full_chain_into_the_server_and_back() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("echo", "echo reply")]);
    let bridge = bridge_over(&server, test_options());
    let report = bridge.refresh().await.unwrap();
    let tool = report.registered[0].clone();

    // Default posture: an external tool without authorization goes to a
    // human, so authorize it first through the governance mapping.
    let entry = bridge.catalog().get("mcp:demo:echo").unwrap();
    McpPermissionMapping::authorize(&mut bridge.policy().lock().unwrap(), &entry);

    let result = tool
        .invoke(&call("mcp:demo:echo", serde_json::json!({ "text": "hi" })))
        .await;
    assert!(result.is_ok(), "{}", result.render());
    assert!(
        result.render().contains("echo reply"),
        "{}",
        result.render()
    );

    let calls = server.call_log();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "echo");
    assert_eq!(calls[0].1, serde_json::json!({ "text": "hi" }));
    assert_eq!(server.method_count("initialize"), 1);
    assert_eq!(server.method_count("tools/list"), 1);
}

#[tokio::test]
async fn unauthorized_external_tool_goes_to_approval_and_never_runs() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("send-item", "sent")]);
    let bridge = bridge_over(&server, test_options());
    let report = bridge.refresh().await.unwrap();
    let tool = report.registered[0].clone();

    let result = tool
        .invoke(&call("mcp:demo:send-item", serde_json::json!({})))
        .await;
    assert!(!result.is_ok());
    assert!(
        result.render().contains("pipeline.pre_ask"),
        "unauthorized calls must ask for approval: {}",
        result.render()
    );
    assert!(
        server.call_log().is_empty(),
        "nothing may reach the server before approval"
    );
}

#[tokio::test]
async fn blocked_external_tool_is_refused_by_the_monotonic_guard() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("send-item", "sent")]);
    let bridge = bridge_over(&server, test_options());
    let report = bridge.refresh().await.unwrap();
    let tool = report.registered[0].clone();

    let entry = bridge.catalog().get("mcp:demo:send-item").unwrap();
    McpPermissionMapping::authorize(&mut bridge.policy().lock().unwrap(), &entry);
    bridge
        .deny_guard()
        .deny_capability(entry.capability_id.as_str().to_string());

    let result = tool
        .invoke(&call("mcp:demo:send-item", serde_json::json!({})))
        .await;
    assert!(!result.is_ok());
    assert!(
        result.render().contains("pipeline.guard_deny"),
        "a blocked capability must be refused by the guard stage: {}",
        result.render()
    );
    assert!(server.call_log().is_empty(), "a refused call never runs");
}

#[tokio::test]
async fn explicit_permission_grant_authorizes_the_call() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("lookup-item", "found")]);
    let bridge = bridge_over(&server, test_options());
    let report = bridge.refresh().await.unwrap();
    let tool = report.registered[0].clone();

    // An explicit grant authorizes the call.
    let entry = bridge.catalog().get("mcp:demo:lookup-item").unwrap();
    McpPermissionMapping::authorize(&mut bridge.policy().lock().unwrap(), &entry);
    let result = tool
        .invoke(&call("mcp:demo:lookup-item", serde_json::json!({})))
        .await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(server.call_log().len(), 1);

    // An approval marking on top of the grant keeps the call pending.
    McpPermissionMapping::require_approval(&mut bridge.policy().lock().unwrap(), &entry);
    let asked = tool
        .invoke(&call("mcp:demo:lookup-item", serde_json::json!({})))
        .await;
    assert!(!asked.is_ok(), "an approval marking keeps the call pending");
    assert_eq!(server.call_log().len(), 1, "the pending call never ran");
}

#[tokio::test]
async fn readonly_preset_releases_declared_read_only_tools_only() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("lookup-item", "found").read_only(),
        MockToolSpec::new("send-item", "sent"),
    ]);
    let bridge = bridge_over(&server, test_options().with_readonly_preset(true));
    let report = bridge.refresh().await.unwrap();

    let read_tool = report
        .registered
        .iter()
        .find(|tool| tool.declaration().name == "mcp:demo:lookup-item")
        .unwrap()
        .clone();
    let result = read_tool
        .invoke(&call("mcp:demo:lookup-item", serde_json::json!({})))
        .await;
    assert!(result.is_ok(), "{}", result.render());

    let write_tool = report
        .registered
        .iter()
        .find(|tool| tool.declaration().name == "mcp:demo:send-item")
        .unwrap()
        .clone();
    let pending = write_tool
        .invoke(&call("mcp:demo:send-item", serde_json::json!({})))
        .await;
    assert!(!pending.is_ok());
    assert!(
        pending.render().contains("pipeline.pre_ask"),
        "{}",
        pending.render()
    );
    assert_eq!(server.call_log().len(), 1, "only the released tool ran");
}

#[tokio::test]
async fn mcp_call_timeout_closes_in_the_timeout_family() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("slow-tool", "late").with_delay(Duration::from_millis(400))
    ]);
    let bridge = bridge_over(&server, test_options().with_call_timeout(40, 5_000));
    let report = bridge.refresh().await.unwrap();
    let tool = report.registered[0].clone();

    McpPermissionMapping::authorize(
        &mut bridge.policy().lock().unwrap(),
        &bridge.catalog().get("mcp:demo:slow-tool").unwrap(),
    );
    let result = tool
        .invoke(&call("mcp:demo:slow-tool", serde_json::json!({})))
        .await;
    assert!(!result.is_ok());
    assert!(
        result.render().contains("timeout."),
        "an MCP call timeout must close as timeout.*: {}",
        result.render()
    );
    assert!(
        result.render().contains("timeout.deadline_expired"),
        "{}",
        result.render()
    );
}

#[tokio::test]
async fn reconnect_rediscovers_the_tool_catalog() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("echo", "v1")]);
    let bridge = bridge_over(&server, test_options());
    let report = bridge.refresh().await.unwrap();
    assert_eq!(report.registered_names(), ["mcp:demo:echo"]);
    let tool = report.registered[0].clone();

    McpPermissionMapping::authorize(
        &mut bridge.policy().lock().unwrap(),
        &bridge.catalog().get("mcp:demo:echo").unwrap(),
    );

    // The link drops mid-run and the server gains a tool while we are away.
    server.drop_next_requests(1);
    server.set_tools(vec![
        MockToolSpec::new("echo", "v2"),
        MockToolSpec::new("extra-tool", "new"),
    ]);

    let result = tool
        .invoke(&call("mcp:demo:echo", serde_json::json!({})))
        .await;
    assert!(result.is_ok(), "{}", result.render());
    assert!(
        result.render().contains("v2"),
        "retry served the live reply"
    );
    assert!(
        bridge.connections()[0].reconnect_count() >= 1,
        "the disconnect recovery must have run"
    );
    assert!(server.opened_channels() >= 2, "a fresh link must be opened");

    // The catalog is stale after the reconnect; the refresh re-discovers.
    let report = bridge.refresh().await.unwrap();
    assert_eq!(report.registered_names(), ["mcp:demo:extra-tool"]);
    assert_eq!(report.removed.len(), 0);
    let names: Vec<String> = report
        .live
        .iter()
        .map(|entry| entry.model_name.clone())
        .collect();
    assert_eq!(names, ["mcp:demo:echo", "mcp:demo:extra-tool"]);

    // A tool that disappears is reported as removed.
    server.set_tools(vec![MockToolSpec::new("echo", "v2")]);
    let report = bridge.refresh().await.unwrap();
    assert_eq!(report.removed.len(), 1);
    assert_eq!(report.removed[0].model_name, "mcp:demo:extra-tool");
}

#[tokio::test]
async fn duplicate_tool_names_are_rejected_not_merged() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("echo", "first"),
        MockToolSpec::new("echo", "second"),
    ]);
    let bridge = bridge_over(&server, test_options());

    let Err(err) = bridge.refresh().await else {
        panic!("a duplicated name must fail the refresh");
    };
    let message = err.to_string();
    assert!(
        message.contains("duplicate tool name"),
        "a duplicated name must fail the refresh: {message}"
    );

    // Registration is reject-on-collision in the registry, too.
    server.set_tools(vec![MockToolSpec::new("echo", "only")]);
    let report = bridge.refresh().await.unwrap();
    let registry = RecordingRegistry::default();
    report.apply_to(&registry).unwrap();
    let err = report.apply_to(&registry).unwrap_err();
    assert!(
        matches!(err, McpBridgeError::Registration(_)),
        "a second registration of the same tool must be refused: {err}"
    );
}

#[tokio::test]
async fn mcp_result_structure_normalizes_into_the_output_contract() {
    let server = InProcessMcpServer::new(SERVER);
    // The server answers with alternate wire spellings (snake_case keys) and
    // an image block alongside the text block.
    server.set_tools(vec![MockToolSpec::new("mixed-tool", "ignored")
        .with_raw_result(serde_json::json!({
            "content": [
                { "type": "text", "text": "hello from the server", "mime_type": "text/plain" },
                { "type": "image", "data": "AAAA", "mime_type": "image/png" }
            ],
            "is_error": false
        }))]);
    let bridge = bridge_over(&server, test_options());
    let report = bridge.refresh().await.unwrap();
    McpPermissionMapping::authorize(
        &mut bridge.policy().lock().unwrap(),
        &bridge.catalog().get("mcp:demo:mixed-tool").unwrap(),
    );
    let dynamic = McpDynamicTool::new(
        bridge.catalog().get("mcp:demo:mixed-tool").unwrap(),
        Arc::clone(&bridge.connections()[0]),
    );

    let tool_call = call("mcp:demo:mixed-tool", serde_json::json!({}));
    let executed = bridge.pipeline().run(&dynamic, &tool_call).await;

    // The five stages were visited in their fixed order.
    assert_eq!(
        executed.record.stages(),
        [
            ExecutionStage::PreExecute,
            ExecutionStage::Guard,
            ExecutionStage::Around,
            ExecutionStage::PostExecute,
            ExecutionStage::Normalize,
        ]
    );

    // The MCP result structure is normalized into the frozen contract.
    let outcome: &ToolOutcome = &executed.outcome;
    assert!(outcome.ok);
    assert!(outcome.output_text.contains("hello from the server"));
    let structured = outcome.structured().expect("structured sidecar");
    assert_eq!(structured["text"], "hello from the server");
    assert_eq!(structured["content"].as_array().unwrap().len(), 2);
    assert_eq!(structured["content"][0]["mimeType"], "text/plain");
    assert_eq!(structured["is_error"], false);

    // A server-side error becomes a failed outcome, never a silent success.
    server.set_tools(vec![
        MockToolSpec::new("mixed-tool", "broken").error_reply("the server said no")
    ]);
    let tool_call = call("mcp:demo:mixed-tool", serde_json::json!({}));
    let executed = bridge.pipeline().run(&dynamic, &tool_call).await;
    assert!(!executed.outcome.ok);
    assert!(
        executed.outcome.output_text.contains("the server said no"),
        "{}",
        executed.outcome.output_text
    );
    assert_eq!(report.registered.len(), 1);
}

#[tokio::test]
async fn startup_log_never_carries_token_parameters() {
    let spec = McpServerSpec::remote(
        "remote-demo",
        McpTransportKind::Http,
        "https://user:pass@example.test/mcp?token=very-secret&region=eu",
    );
    let config = McpServerConfig::new(vec![
        spec,
        McpServerSpec::stdio("local-demo", "server-bin")
            .with_arg("--api-key")
            .with_arg("very-secret")
            .with_arg("--verbose"),
    ]);
    let options = test_options();
    let connections = vec![
        Arc::new(McpServerConnection::new(
            config.servers[0].clone(),
            InProcessMcpServer::new("remote-demo").factory(),
            &options,
        )),
        Arc::new(McpServerConnection::new(
            config.servers[1].clone(),
            InProcessMcpServer::new("local-demo").factory(),
            &options,
        )),
    ];
    let bridge = McpToolBridge::from_connections(config, options, connections).unwrap();

    let lines = bridge.startup_log();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert!(!line.contains("very-secret"), "token leaked into: {line}");
        assert!(
            !line.contains("pass@example"),
            "url password leaked into: {line}"
        );
    }
    assert!(lines[0].contains("example.test"), "urls stay: {}", lines[0]);
    assert!(lines[0].contains("region=eu"), "{}", lines[0]);
    assert!(lines[1].contains("--verbose"), "{}", lines[1]);
}

#[tokio::test]
async fn broken_server_configuration_refuses_to_open() {
    let dir = std::env::temp_dir().join(format!("apeireth-mcp-e2e-config-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = apeireth_tools_canonical::mcp_bridge::MCP_SERVERS_FILE;
    std::fs::write(dir.join(path), "{ this is not a stored document").unwrap();
    let err = McpServerConfig::load_from_data_dir(&dir).unwrap_err();
    assert!(matches!(err, McpConfigError::Stored(_)), "{err:?}");

    // The primary entry fails closed as well: a malformed env value is an
    // error, never an empty or defaulted server list.
    let err = McpServerConfig::from_env_value("[{ bad json").unwrap_err();
    assert!(matches!(err, McpConfigError::InvalidJson { .. }), "{err:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn output_schema_declares_the_normalized_shape() {
    let schema = mcp_output_schema();
    let fields: Vec<&str> = schema
        .fields()
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(fields, ["text", "content"]);
    assert!(schema
        .validate(&serde_json::json!({ "text": "x", "content": [] }))
        .is_ok());
    assert!(schema
        .validate(&serde_json::json!({ "text": "x" }))
        .is_err());
}

#[tokio::test]
async fn dynamic_tool_declaration_carries_the_server_schema() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("lookup-item", "found").with_description("Look one item up")
    ]);
    let bridge = bridge_over(&server, test_options());
    let report = bridge.refresh().await.unwrap();

    let dynamic = McpDynamicTool::new(
        bridge.catalog().get("mcp:demo:lookup-item").unwrap(),
        Arc::clone(&bridge.connections()[0]),
    );
    let declaration = dynamic.declaration();
    assert_eq!(declaration.name, "mcp:demo:lookup-item");
    assert_eq!(declaration.description.as_deref(), Some("Look one item up"));
    assert_eq!(
        declaration.parameters.get("type"),
        Some(&serde_json::json!("object"))
    );
    assert_eq!(report.registered.len(), 1);
}

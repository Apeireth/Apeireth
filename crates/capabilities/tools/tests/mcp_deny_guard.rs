//! The deny surface of the external-tool bridge: an operator's blocklist.
//!
//! The bridge's monotonic deny gate ([`McpToolDenyGuard`]) is the one shape
//! in the bridge that may only **tighten**. This file pins the production
//! injection point of that gate (the per-server `deny_tools` list of the
//! configuration) and the two halves it drives:
//!
//! * the **directory** half — a denied tool never enters the dynamic
//!   directory, never reaches the model tool catalog and never runs;
//! * the **pipeline** half — the guard stage refuses a denied call
//!   **before the tool executes**, even when the read-only preset, the
//!   operator allowlist or a family grant released it.
//!
//! Every test runs against the in-process server double: no sockets, no
//! subprocesses, no network.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{Decision, PermissionPolicy};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::mcp_bridge::{
    DiscoveredTool, InProcessMcpServer, McpBridgeOptions, McpDynamicTool, McpPermissionMapping,
    McpServerConfig, McpServerConnection, McpServerSpec, McpToolBridge, McpToolEntry,
    McpToolRegistry, McpTransportKind, MockToolSpec,
};

const SERVER: &str = "demo";
const OTHER_SERVER: &str = "other";

fn test_options() -> McpBridgeOptions {
    let reconnect = apeireth_plugin::mcp::ReconnectPolicy {
        initial_backoff: Duration::from_millis(1),
        multiplier_num: 1,
        multiplier_den: 1,
        max_backoff: Duration::from_millis(4),
        max_attempts: 3,
    };
    McpBridgeOptions::default()
        .with_call_timeout(500, 5_000)
        .with_reconnect(reconnect)
}

fn call(name: &str) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: name.into(),
        arguments: serde_json::json!({}),
    }
}

/// Assemble a bridge over one in-process server, with the operator's
/// deny list on that server's spec.
fn bridge_with_deny_list(
    server: &Arc<InProcessMcpServer>,
    spec: McpServerSpec,
    options: McpBridgeOptions,
) -> McpToolBridge {
    let config = McpServerConfig::new(vec![spec.clone()]);
    let connection = Arc::new(McpServerConnection::new(spec, server.factory(), &options));
    McpToolBridge::from_connections(config, options, vec![connection]).unwrap()
}

/// Assemble a bridge over one in-process server with **no** deny list.
fn bridge_without_deny_list(
    server: &Arc<InProcessMcpServer>,
    options: McpBridgeOptions,
) -> McpToolBridge {
    bridge_with_deny_list(
        server,
        McpServerSpec::stdio(SERVER, "in-process-test-server"),
        options,
    )
}

/// A registry recording what the bridge pushes into it.
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

// ---------------------------------------------------------------------------
// The configuration surface: a malformed deny list refuses to open.
// ---------------------------------------------------------------------------

#[test]
fn a_malformed_deny_entry_refuses_to_open_the_configuration() {
    // A malformed tool name in the deny list is a broken configuration:
    // it refuses to open, exactly like a malformed allowlist entry.
    let bad = r#"[{"name":"demo","transport":"stdio","command":"demo-server","enabled":true,
                   "deny_tools":["not a tool"]}]"#;
    let error = McpServerConfig::from_env_value(bad).unwrap_err();
    assert!(
        error.to_string().contains("deny_tools"),
        "the refusal must name the offending field: {error}"
    );

    // A malformed server name refuses to open as well.
    let bad_server = r#"[{"name":"Demo Server","transport":"stdio","command":"demo-server",
                          "enabled":true,"deny_tools":["send-item"]}]"#;
    let error = McpServerConfig::from_env_value(bad_server).unwrap_err();
    assert!(
        error.to_string().contains("server name"),
        "the refusal must name the server name: {error}"
    );

    // A disabled server is validated too: the deny list is checked even
    // where the server never runs (fail-closed, not "disabled means lax").
    let disabled = r#"[{"name":"demo","transport":"stdio","command":"demo-server","enabled":false,
                      "deny_tools":["not a tool"]}]"#;
    assert!(McpServerConfig::from_env_value(disabled).is_err());
}

#[test]
fn the_deny_list_survives_the_configuration_round_trip() {
    let raw = r#"[{"name":"demo","transport":"stdio","command":"demo-server","enabled":true,
                   "deny_tools":["send-item","delete-item"]}]"#;
    let config = McpServerConfig::from_env_value(raw).unwrap();
    assert_eq!(
        config.servers[0].deny_tools,
        ["send-item".to_string(), "delete-item".to_string()]
    );
    assert!(config.servers[0].denies("send-item"));
    assert!(!config.servers[0].denies("lookup-item"));

    // The list survives a serialize / parse round trip.
    let encoded = serde_json::to_value(&config).unwrap();
    let decoded = McpServerConfig::from_json_value(encoded).unwrap();
    assert_eq!(decoded.servers[0].deny_tools, config.servers[0].deny_tools);

    // A configuration without the field reads as an empty list: the deny
    // surface is off by default, not "on with an empty meaning".
    let plain = r#"[{"name":"demo","transport":"stdio","command":"demo-server","enabled":true}]"#;
    let config = McpServerConfig::from_env_value(plain).unwrap();
    assert!(config.servers[0].deny_tools.is_empty());
}

#[test]
fn the_stored_document_entry_carries_the_deny_list() {
    let dir = std::env::temp_dir().join(format!("apeireth-mcp-deny-config-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let body = McpServerConfig::new(vec![
        McpServerSpec::stdio("demo", "demo-server").with_deny_tool("send-item")
    ]);
    let path = apeireth_tools_canonical::mcp_bridge::server_list_path(&dir);
    apeireth_core::stored_doc::save_single(
        &path,
        &apeireth_tools_canonical::mcp_bridge::doc_compat(),
        body,
        apeireth_core::stored_doc::DEFAULT_DOC_MODE,
    )
    .unwrap();

    let loaded = McpServerConfig::load_from_data_dir(&dir)
        .unwrap()
        .expect("the file is present");
    assert_eq!(loaded.servers[0].deny_tools, ["send-item".to_string()]);

    // A broken file still refuses to open (fail-closed, unchanged).
    std::fs::write(&path, "{ not json").unwrap();
    assert!(McpServerConfig::load_from_data_dir(&dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// The gate: the deny list is armed before the first discovery.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_deny_list_arms_the_gate_before_the_first_discovery() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("send-item", "sent")]);
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server").with_deny_tool("send-item");
    let bridge = bridge_with_deny_list(&server, spec, test_options());

    // The gate is armed at assembly time — before any discovery ran.
    assert!(
        bridge.deny_guard().is_blocked("tool.mcp.demo.send-item"),
        "the configured deny list must be armed in the gate"
    );
    assert!(
        !bridge.deny_guard().is_blocked("tool.mcp.demo.lookup-item"),
        "an unlisted tool must not be blocked"
    );
}

#[tokio::test]
async fn a_denied_tool_never_enters_the_directory_or_the_registration_surface() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("lookup-item", "found"),
        MockToolSpec::new("send-item", "sent"),
    ]);
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server").with_deny_tool("send-item");
    let bridge = bridge_with_deny_list(&server, spec, test_options());

    let report = bridge.refresh().await.unwrap();

    // The denied tool is nowhere on the exposed surface.
    assert_eq!(report.registered_names(), ["mcp:demo:lookup-item"]);
    assert!(
        bridge.catalog().get("mcp:demo:send-item").is_none(),
        "a denied tool must not enter the dynamic directory"
    );
    assert!(
        !bridge
            .tools()
            .iter()
            .any(|tool| tool.declaration().name == "mcp:demo:send-item"),
        "a denied tool must not enter the live tool set"
    );
    assert_eq!(report.live.len(), 1);
    // Nothing was executed: the server saw no call at all.
    assert!(server.call_log().is_empty(), "a denied tool never runs");
}

// ---------------------------------------------------------------------------
// The pipeline half: the guard refuses before the tool executes.
// ---------------------------------------------------------------------------

/// Build the dynamic tool of a denied capability by hand — the directory
/// never holds it, so a handle has to be constructed out of band.
fn denied_handle(bridge: &McpToolBridge, tool: &str, read_only: bool) -> McpDynamicTool {
    let entry = McpToolEntry::new(
        SERVER,
        &DiscoveredTool {
            name: tool.to_string(),
            description: None,
            input_schema: None,
            declared_read_only: read_only,
        },
    )
    .unwrap();
    McpDynamicTool::new(entry, Arc::clone(&bridge.connections()[0]))
}

#[tokio::test]
async fn a_denied_tool_is_refused_by_the_guard_even_when_it_is_authorized() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("send-item", "sent")]);
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server").with_deny_tool("send-item");
    let bridge = bridge_with_deny_list(&server, spec, test_options());
    bridge.refresh().await.unwrap();

    // The tool is authorized all the way: an explicit grant, so the
    // pre-execute gate passes it through.
    let handle = denied_handle(&bridge, "send-item", false);
    McpPermissionMapping::authorize(
        &mut bridge.policy().lock().unwrap(),
        &McpToolEntry::new(
            SERVER,
            &DiscoveredTool {
                name: "send-item".into(),
                description: None,
                input_schema: None,
                declared_read_only: false,
            },
        )
        .unwrap(),
    );

    let executed = bridge
        .pipeline()
        .run(&handle, &call("mcp:demo:send-item"))
        .await;
    assert!(!executed.outcome.ok);
    assert!(
        executed.outcome.output_text.contains("pipeline.guard_deny"),
        "an authorized but denied tool must be refused by the guard stage: {}",
        executed.outcome.output_text
    );
    assert!(
        server.call_log().is_empty(),
        "a refused call never reaches the server"
    );
}

#[tokio::test]
async fn the_deny_beats_the_family_grant() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("send-item", "sent")]);
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server").with_deny_tool("send-item");
    let bridge = bridge_with_deny_list(&server, spec, test_options());
    bridge.refresh().await.unwrap();

    // The whole external family is released — the release happens at the
    // pre-execute stage (otherwise the failure would be `pipeline.pre_deny`),
    // and the deny gate still has the last word.
    bridge.family_grants().grant_mcp_family();
    let handle = denied_handle(&bridge, "send-item", false);

    let executed = bridge
        .pipeline()
        .run(&handle, &call("mcp:demo:send-item"))
        .await;
    assert!(!executed.outcome.ok);
    assert!(
        executed.outcome.output_text.contains("pipeline.guard_deny"),
        "the family release must not overturn the deny: {}",
        executed.outcome.output_text
    );
    assert!(
        server.call_log().is_empty(),
        "a refused call never reaches the server"
    );
}

#[tokio::test]
async fn the_deny_beats_the_readonly_preset_release() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("lookup-item", "found").read_only()]);
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server").with_deny_tool("lookup-item");
    let bridge = bridge_with_deny_list(&server, spec, test_options().with_readonly_preset(true));
    bridge.refresh().await.unwrap();

    // The tool is declared read-only **and** the entry is put into the
    // directory by hand, so the read-only preset releases it at the
    // pre-execute stage. The deny gate still refuses.
    let handle = denied_handle(&bridge, "lookup-item", true);
    bridge
        .catalog()
        .insert(
            McpToolEntry::new(
                SERVER,
                &DiscoveredTool {
                    name: "lookup-item".into(),
                    description: None,
                    input_schema: None,
                    declared_read_only: true,
                },
            )
            .unwrap(),
        )
        .unwrap();

    let executed = bridge
        .pipeline()
        .run(&handle, &call("mcp:demo:lookup-item"))
        .await;
    assert!(!executed.outcome.ok);
    assert!(
        executed.outcome.output_text.contains("pipeline.guard_deny"),
        "the read-only release must not overturn the deny: {}",
        executed.outcome.output_text
    );
    assert!(
        server.call_log().is_empty(),
        "a refused call never reaches the server"
    );
}

#[tokio::test]
async fn the_readonly_preset_does_not_grant_a_denied_tool() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("lookup-item", "found").read_only()]);
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server").with_deny_tool("lookup-item");
    let bridge = bridge_with_deny_list(&server, spec, test_options().with_readonly_preset(true));
    bridge.refresh().await.unwrap();

    // The preset never released the denied tool: it is not in the
    // directory, and the policy holds no grant for it.
    assert!(bridge.catalog().get("mcp:demo:lookup-item").is_none());
    let decision = bridge
        .policy()
        .lock()
        .unwrap()
        .decision_for_capability("tool.mcp.demo.lookup-item");
    assert!(
        matches!(decision, Decision::Deny { .. }),
        "the read-only preset must not grant a denied tool: {decision:?}"
    );
}

#[tokio::test]
async fn the_deny_beats_the_operator_allowlist() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("send-item", "sent")]);
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server")
        .with_auto_authorize("send-item")
        .with_deny_tool("send-item");
    let bridge = bridge_with_deny_list(&server, spec, test_options());
    bridge.refresh().await.unwrap();

    // The allowlist and the deny list name the same tool: the deny side
    // wins, so the tool is neither exposed nor granted.
    assert!(bridge.catalog().get("mcp:demo:send-item").is_none());
    let decision = bridge
        .policy()
        .lock()
        .unwrap()
        .decision_for_capability("tool.mcp.demo.send-item");
    assert!(
        matches!(decision, Decision::Deny { .. }),
        "the deny list must beat the allowlist: {decision:?}"
    );
}

// ---------------------------------------------------------------------------
// The revocation path: a denied tool leaves the registration surface.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_denied_tool_leaves_the_registration_surface_on_the_next_refresh() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("lookup-item", "found"),
        MockToolSpec::new("send-item", "sent"),
    ]);
    let bridge = bridge_without_deny_list(&server, test_options());
    let registry = RecordingRegistry::default();

    // First round: both tools are live and registered.
    let report = bridge.refresh().await.unwrap();
    report.apply_to(&registry).unwrap();
    assert_eq!(report.registered_names().len(), 2);
    assert!(registry
        .tools
        .lock()
        .unwrap()
        .contains_key("tool.mcp.demo.send-item"));

    // The operator pulls one tool out. The gate owns the deny set, so the
    // next round takes the tool out of the directory and reports it as
    // removed — the registry forgets it, and the model stops seeing it.
    assert!(bridge
        .deny_guard()
        .deny_capability("tool.mcp.demo.send-item"));
    let report = bridge.refresh().await.unwrap();
    report.apply_to(&registry).unwrap();

    assert!(
        bridge.catalog().get("mcp:demo:send-item").is_none(),
        "a denied tool must leave the dynamic directory"
    );
    assert_eq!(report.removed.len(), 1);
    assert_eq!(report.removed[0].model_name, "mcp:demo:send-item");
    assert!(
        !registry
            .tools
            .lock()
            .unwrap()
            .contains_key("tool.mcp.demo.send-item"),
        "a denied tool must leave the registration surface"
    );
    assert!(registry
        .tools
        .lock()
        .unwrap()
        .contains_key("tool.mcp.demo.lookup-item"));
}

#[tokio::test]
async fn a_denied_tool_stays_denied_after_a_reconnect() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("echo", "v1")]);
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server").with_deny_tool("send-item");
    let bridge = bridge_with_deny_list(&server, spec, test_options());
    bridge.refresh().await.unwrap();

    // The link drops and the server re-advertises a denied tool. The
    // re-discovered list must not put the denied tool back into the
    // directory.
    server.drop_next_requests(1);
    server.set_tools(vec![
        MockToolSpec::new("echo", "v2"),
        MockToolSpec::new("send-item", "sent"),
    ]);
    let handle = denied_handle(&bridge, "echo", false);
    let _ = handle.invoke(&call("mcp:demo:echo")).await;

    // The reconnect really ran, and the re-discovered list was adopted (the
    // adoption path is the one that could have put the denied tool back).
    assert!(
        bridge.connections()[0].reconnect_count() >= 1,
        "the disconnect recovery must have run"
    );
    assert!(
        bridge.connections()[0].last_rediscovery_error().is_none(),
        "the re-discovered list must be adopted, not refused"
    );
    assert!(
        bridge.catalog().get("mcp:demo:send-item").is_none(),
        "a denied tool must not be re-adopted into the directory"
    );
    assert!(bridge.catalog().get("mcp:demo:echo").is_some());
}

// ---------------------------------------------------------------------------
// The scope of the deny list: one server's list touches one server.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_deny_list_of_one_server_does_not_touch_another_server() {
    let demo = InProcessMcpServer::new(SERVER);
    demo.set_tools(vec![MockToolSpec::new("send-item", "sent")]);
    let other = InProcessMcpServer::new(OTHER_SERVER);
    other.set_tools(vec![MockToolSpec::new("send-item", "sent")]);

    let options = test_options();
    let demo_spec =
        McpServerSpec::stdio(SERVER, "in-process-test-server").with_deny_tool("send-item");
    let other_spec = McpServerSpec::stdio(OTHER_SERVER, "in-process-test-server");
    let config = McpServerConfig::new(vec![demo_spec.clone(), other_spec.clone()]);
    let connections = vec![
        Arc::new(McpServerConnection::new(
            demo_spec,
            demo.factory(),
            &options,
        )),
        Arc::new(McpServerConnection::new(
            other_spec,
            other.factory(),
            &options,
        )),
    ];
    let bridge = McpToolBridge::from_connections(config, options, connections).unwrap();

    let report = bridge.refresh().await.unwrap();
    assert_eq!(
        report.registered_names(),
        ["mcp:other:send-item"],
        "only the listed server's tool is pulled out"
    );
    assert!(bridge.deny_guard().is_blocked("tool.mcp.demo.send-item"));
    assert!(
        !bridge.deny_guard().is_blocked("tool.mcp.other.send-item"),
        "the other server's tool of the same name stays live"
    );
}

// ---------------------------------------------------------------------------
// The deny list is a per-server surface, not a global one.
// ---------------------------------------------------------------------------

#[test]
fn a_deny_list_is_bound_to_its_own_server_spec() {
    let spec = McpServerSpec::remote(
        "remote-demo",
        McpTransportKind::Http,
        "http://127.0.0.1:9/mcp",
    )
    .with_deny_tool("send-item");
    assert!(spec.denies("send-item"));
    assert!(!spec.denies("lookup-item"));
    assert_eq!(spec.deny_tools, ["send-item".to_string()]);
}

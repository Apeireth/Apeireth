//! The **structured channel** of a `tools/call` result reaches the model.
//!
//! # The gap this file closes
//!
//! A server that answers a tool call with a structured result carries it in
//! `structuredContent`. The wire type (`ToolCallResult`) used to know only
//! `content` and `isError`, and serde's default (ignore unknown fields) then
//! dropped the structured payload **on the floor**: the server's answer
//! arrived, the call succeeded, and the structured half of that answer was
//! gone before the model could read it.
//!
//! # What is pinned here
//!
//! * the **channel is open**: a result carrying `structuredContent` keeps it;
//! * the **model face sees it**: the payload shows up in the normalized
//!   output the model reads — asserted on the *model-facing shape*, not on
//!   the wire type;
//! * the **data keys stay the server's own**: the payload is data, not an
//!   envelope, so the tolerant key rewrite must not reach inside it (a
//!   server's `is_error` data key must not become `isError`);
//! * the **wire spelling does not matter**: the baseline snake_case spelling
//!   of the payload is read the same way;
//! * the **old shape is untouched**: an answer without a structured payload
//!   keeps exactly the shape it had.
//!
//! # How the cases are driven
//!
//! The server side is the in-process JSON-RPC double — no sockets, no
//! subprocesses, no network — answering with a raw `tools/call` result, and
//! the chain under test is the real one: discovery → dynamic registration →
//! the five-stage pipeline → the output normalization contract.

use std::sync::Arc;
use std::time::Duration;

use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::exec_pipeline::ToolOutcome;
use apeireth_tools_canonical::mcp_bridge::{
    InProcessMcpServer, McpBridgeOptions, McpDynamicTool, McpPermissionMapping, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec,
};

/// The server name the double answers under.
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
    McpToolBridge::from_connections(config, options, vec![connection])
        .expect("the bridge assembles")
}

fn call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: name.into(),
        arguments,
    }
}

/// The model-facing structured value of one call over the double.
///
/// The bridge is refreshed, the tool is authorized, and the call runs
/// through the real pipeline — so what is asserted below is exactly what the
/// model reads.
async fn model_face(
    server: &Arc<InProcessMcpServer>,
    tool_name: &str,
) -> (ToolOutcome, serde_json::Value) {
    let bridge = bridge_over(server, test_options());
    bridge.refresh().await.expect("discovery succeeds");
    let entry = bridge
        .catalog()
        .get(&format!("mcp:{SERVER}:{tool_name}"))
        .expect("the discovered tool is in the catalog");
    McpPermissionMapping::authorize(&mut bridge.policy().lock().unwrap(), &entry);
    let dynamic = McpDynamicTool::new(entry, Arc::clone(&bridge.connections()[0]));

    let executed = bridge
        .pipeline()
        .run(
            &dynamic,
            &call(&format!("mcp:{SERVER}:{tool_name}"), serde_json::json!({})),
        )
        .await;
    let structured = executed
        .outcome
        .structured()
        .expect("the normalized outcome carries its structured sidecar")
        .clone();
    (executed.outcome, structured)
}

/// ① **The channel is open and the model sees the payload**: a server that
/// answers with a structured result keeps that payload all the way to the
/// model-facing shape.
#[tokio::test]
async fn a_structured_payload_reaches_the_model_face() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("lookup-item", "ignored")
        .with_raw_result(serde_json::json!({
            "content": [{ "type": "text", "text": "two rows" }],
            "isError": false,
            "structuredContent": {
                "count": 2,
                "rows": [
                    { "id": 1, "name": "alpha" },
                    { "id": 2, "name": "beta" }
                ]
            }
        }))]);

    let (outcome, structured) = model_face(&server, "lookup-item").await;

    assert!(outcome.ok, "{}", outcome.output_text);
    // The text channel is untouched.
    assert!(outcome.output_text.contains("two rows"));
    assert_eq!(structured["text"], "two rows");
    // The structured channel arrived: the payload is there, with the
    // server's own values.
    let payload = &structured["structured"];
    assert_eq!(payload["count"], 2);
    assert_eq!(payload["rows"][0]["name"], "alpha");
    assert_eq!(payload["rows"][1]["id"], 2);
    // The payload is data, not a replacement of the answer: the content
    // blocks are still there.
    assert_eq!(
        structured["content"]
            .as_array()
            .expect("content blocks")
            .len(),
        1
    );
}

/// ② **The payload is data, not an envelope**: the tolerant key rewrite
/// (the one that turns `is_error` into `isError` on the wire) must not
/// reach inside the payload — a server's own data key keeps its name.
#[tokio::test]
async fn the_payload_keeps_the_servers_own_data_keys() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("raw-keys", "ignored")
        .with_raw_result(serde_json::json!({
            "content": [{ "type": "text", "text": "ok" }],
            "isError": false,
            "structuredContent": {
                "is_error": true,
                "input_schema": { "type": "object" },
                "mime_type": "text/plain"
            }
        }))]);

    let (outcome, structured) = model_face(&server, "raw-keys").await;

    assert!(outcome.ok, "{}", outcome.output_text);
    let payload = &structured["structured"];
    assert_eq!(
        payload["is_error"], true,
        "the server's own data key must not be renamed: {payload}"
    );
    assert_eq!(payload["input_schema"]["type"], "object");
    assert_eq!(payload["mime_type"], "text/plain");
    assert!(
        payload.get("isError").is_none() && payload.get("inputSchema").is_none(),
        "the envelope rewrite must not reach into the payload: {payload}"
    );
}

/// ③ **The old shape is untouched**: an answer without a structured payload
/// keeps exactly the shape it had — the new channel adds a key only when
/// the server sent one.
#[tokio::test]
async fn an_answer_without_a_structured_payload_keeps_the_old_shape() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("plain-item", "just text")]);

    let (outcome, structured) = model_face(&server, "plain-item").await;

    assert!(outcome.ok, "{}", outcome.output_text);
    assert_eq!(structured["text"], "just text");
    assert_eq!(structured["is_error"], false);
    assert!(
        structured.get("structured").is_none(),
        "a server that sent no structured payload adds no key: {structured}"
    );
}

/// ④ **The structured channel survives the wire spelling**: a server that
/// writes the payload in the baseline snake_case spelling is read the same
/// way — the payload is found, not dropped.
#[tokio::test]
async fn a_snake_case_payload_is_read_too() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("snake-item", "ignored")
        .with_raw_result(serde_json::json!({
            "content": [{ "type": "text", "text": "ok" }],
            "is_error": false,
            "structured_content": { "total": 3 }
        }))]);

    let (outcome, structured) = model_face(&server, "snake-item").await;

    assert!(outcome.ok, "{}", outcome.output_text);
    assert_eq!(structured["structured"]["total"], 3);
}

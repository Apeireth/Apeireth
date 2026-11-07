//! The server's **structured** result reaches the model face.
//!
//! # The gap this file closes
//!
//! The result structure carried only the content blocks and the error flag.
//! serde's default (ignore unknown fields) then dropped the server's
//! `structuredContent` on the floor: the **structured channel was broken** —
//! the payload never reached the model, and nothing said so.
//!
//! The server under test is the in-process JSON-RPC double (no sockets, no
//! subprocesses, no network); it answers with a raw `tools/call` result, the
//! shape a real third-party server answers with.
//!
//! # What is pinned here
//!
//! * a server that sends `structuredContent` ⇒ the model-facing result
//!   carries it (the channel is open again);
//! * a server that sends none ⇒ the model-facing result keeps its old shape
//!   (no empty key, no drift);
//! * the structured payload is **server-authored**: it is swept and bounded
//!   like every other server-authored surface;
//! * the structured payload's own **data keys** are not rewritten by the
//!   tolerant wire normalization (a data key named `is_error` keeps its name).

use std::sync::Arc;
use std::time::Duration;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_tools_canonical::mcp_bridge::tool::redaction::{
    RESULT_MAX_STRUCTURED_CHARS, TRUNCATION_MARKER,
};
use apeireth_tools_canonical::mcp_bridge::{
    InProcessMcpServer, McpBridgeOptions, McpPermissionMapping, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec,
};

/// The server name the double answers under.
const SERVER: &str = "demo";

/// A credential shape the placeholder family covers.
const PASSWORD_VALUE: &str = "hunter2secret";

fn test_options() -> McpBridgeOptions {
    let reconnect = apeireth_plugin::mcp::ReconnectPolicy {
        initial_backoff: Duration::from_millis(1),
        multiplier_num: 1,
        multiplier_den: 1,
        max_backoff: Duration::from_millis(4),
        max_attempts: 3,
    };
    McpBridgeOptions::default()
        .with_call_timeout(2_000, 5_000)
        .with_reconnect(reconnect)
}

fn demo_spec() -> McpServerSpec {
    McpServerSpec::stdio(SERVER, "in-process-test-server")
}

/// The bridge over the in-process double.
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

fn call(name: &str) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: name.into(),
        arguments: serde_json::json!({}),
    }
}

/// Run one authorized external tool call and return the model-facing result.
async fn call_external(bridge: &McpToolBridge, tool_name: &str) -> ToolOutcome {
    let entry = bridge
        .catalog()
        .get(tool_name)
        .expect("the discovered tool is in the catalog");
    McpPermissionMapping::authorize(&mut bridge.policy().lock().unwrap(), &entry);
    let tool = bridge
        .tools()
        .into_iter()
        .find(|tool| tool.declaration().name == tool_name)
        .expect("the discovered tool is wrapped and live");
    tool.invoke(&call(tool_name)).await.outcome
}

/// The structured value a successful result carries.
fn structured_value(outcome: &ToolOutcome) -> &serde_json::Value {
    let ToolOutcome::Ok { value } = outcome else {
        panic!("expected a successful result: {outcome:?}");
    };
    value
}

/// ① **The channel is open**: a server that answers with a structured result
/// ⇒ the model sees it.
#[tokio::test]
async fn a_structured_result_reaches_the_model_face() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("counted", "the answer is 42")
        .with_raw_result(serde_json::json!({
            "content": [{ "type": "text", "text": "the answer is 42" }],
            "isError": false,
            "structuredContent": { "answer": 42, "unit": "widgets" }
        }))]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let outcome = call_external(&bridge, "mcp:demo:counted").await;
    let value = structured_value(&outcome);
    assert_eq!(
        value["structured"],
        serde_json::json!({ "answer": 42, "unit": "widgets" }),
        "the structured channel must reach the model face"
    );
    // The other faces are untouched.
    assert_eq!(value["text"], serde_json::json!("the answer is 42"));
    assert_eq!(value["is_error"], serde_json::json!(false));
}

/// ② **The old shape stays**: a server that sends no structured payload ⇒ the
/// model-facing result carries no structured key at all.
#[tokio::test]
async fn an_answer_without_a_structured_payload_keeps_its_old_shape() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("plain", "the answer is 42")]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let outcome = call_external(&bridge, "mcp:demo:plain").await;
    let value = structured_value(&outcome);
    assert!(
        value.get("structured").is_none(),
        "an answer without a structured payload must not grow an empty key: {value}"
    );
    assert_eq!(value["text"], serde_json::json!("the answer is 42"));
}

/// ③ **The data keys are the server's own**: the tolerant wire normalization
/// must not rename a key inside the structured payload.
#[tokio::test]
async fn the_structured_payloads_own_data_keys_are_not_rewritten() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("rows", "").with_raw_result(
        serde_json::json!({
            "content": [{ "type": "text", "text": "one row" }],
            "isError": false,
            "structuredContent": { "rows": [{ "is_error": false, "mime_type": "text/plain" }] }
        }),
    )]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let outcome = call_external(&bridge, "mcp:demo:rows").await;
    let value = structured_value(&outcome);
    assert_eq!(
        value["structured"]["rows"][0]["is_error"],
        serde_json::json!(false),
        "a data key must keep its name: {value}"
    );
    assert!(
        value["structured"]["rows"][0].get("isError").is_none(),
        "a data key must not be rewritten into the envelope's dialect: {value}"
    );
    assert!(
        value["structured"]["rows"][0].get("mimeType").is_none(),
        "a data key must not be rewritten into the envelope's dialect: {value}"
    );
}

/// ④ **The payload is swept**: the structured channel is server-authored
/// content, so a credential hiding in it must not reach the model.
#[tokio::test]
async fn a_credential_in_the_structured_payload_is_swept() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("secret", "").with_raw_result(
        serde_json::json!({
            "content": [{ "type": "text", "text": "harmless" }],
            "isError": false,
            "structuredContent": { "note": format!("password = {PASSWORD_VALUE}") }
        }),
    )]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let outcome = call_external(&bridge, "mcp:demo:secret").await;
    let value = structured_value(&outcome);
    let note = value["structured"]["note"].as_str().unwrap_or_default();
    assert!(
        !note.contains(PASSWORD_VALUE),
        "the structured channel leaked its credential: {note}"
    );
    assert!(
        note.contains("__AP_"),
        "the structured channel must carry the rewritten marker: {note}"
    );
}

/// ⑤ **The payload is bounded**: one server answer may not flood the model's
/// context.
#[tokio::test]
async fn an_over_long_structured_payload_is_bounded() {
    let server = InProcessMcpServer::new(SERVER);
    let huge = "x".repeat(RESULT_MAX_STRUCTURED_CHARS + 4_096);
    server.set_tools(vec![MockToolSpec::new("huge", "").with_raw_result(
        serde_json::json!({
            "content": [{ "type": "text", "text": "harmless" }],
            "isError": false,
            "structuredContent": { "blob": huge }
        }),
    )]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let outcome = call_external(&bridge, "mcp:demo:huge").await;
    let value = structured_value(&outcome);
    let blob = value["structured"]["blob"].as_str().unwrap_or_default();
    assert!(
        blob.chars().count() <= RESULT_MAX_STRUCTURED_CHARS,
        "the structured payload must stay inside its bound: {} chars",
        blob.chars().count()
    );
    assert!(
        blob.ends_with(TRUNCATION_MARKER),
        "a cut payload must say so: {}",
        blob.chars()
            .skip(blob.chars().count().saturating_sub(32))
            .collect::<String>()
    );
}

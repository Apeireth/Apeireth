//! The provider-facing egress gate and the external tool result face share
//! **one session domain** — the cross-family half of "one session, one
//! domain".
//!
//! Before this batch the two families each built their own redaction domain:
//! the gate's aliases and the tool face's aliases were unrelated, so an
//! alias the tool face produced could not be restored by the gate's
//! controlled return path. With one shared domain instance the two faces
//! recognise each other:
//!
//! * an alias the **tool face** produced is restored by the **gate**;
//! * an alias the **gate** produced is restored by the **domain** the tool
//!   face sweeps in.
//!
//! And the **unwired** case is unchanged: a gate that was not handed a
//! domain keeps its own, so it still does not recognise the tool face's
//! aliases — the behaviour before this batch, byte for byte.

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::SessionId;
use apeireth_governance::input_security::RedactionPlaceholder;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, NormalizedMessage, NormalizedRequest, NormalizedResponse, ToolCall,
    ToolOutcome, ToolResult,
};
use apeireth_runtime::canonical::egress_gate::EgressGate;
use apeireth_runtime_assembly::egress_gate::RedactionEgressGate;
use apeireth_tools_canonical::mcp_bridge::{
    InProcessMcpServer, McpBridgeOptions, McpDynamicTool, McpServerConfig, McpServerConnection,
    McpServerSpec, McpToolBridge, MockToolSpec, SessionRedactionDomain,
};

const SERVER: &str = "demo";

/// A password assignment the placeholder family covers (an alias, not a label).
const SECRET: &str = "hunter2secret";

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

fn result_text(result: &ToolResult) -> String {
    let ToolOutcome::Ok { value } = &result.outcome else {
        panic!("expected a successful result: {}", result.render());
    };
    value["text"]
        .as_str()
        .expect("the joined text is a string")
        .to_string()
}

fn placeholders(text: &str) -> Vec<String> {
    RedactionPlaceholder::scan(text)
        .into_iter()
        .map(|matched| matched.placeholder.render())
        .collect()
}

fn request_with(text: &str) -> NormalizedRequest {
    NormalizedRequest::new(
        "test-model",
        vec![NormalizedMessage {
            role: MessageRole::User,
            content: vec![ContentPart::Text {
                text: text.to_string(),
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }],
    )
}

fn response_with(content: String) -> NormalizedResponse {
    NormalizedResponse {
        id: "resp-1".into(),
        model: "test-model".into(),
        content,
        finish_reason: None,
        usage: Default::default(),
        tool_calls: Vec::new(),
        raw_metadata: Default::default(),
    }
}

/// One in-process server whose single tool answers with the secret.
fn server_with_one_secret() -> Arc<InProcessMcpServer> {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new(
        "leak",
        format!("password = {SECRET}"),
    )]);
    server
}

/// ③ **The cross-family core case**: the gate restores the alias the external
/// tool result face produced — because both hold the **same** domain
/// instance.
#[tokio::test]
async fn the_gate_restores_the_alias_the_tool_face_produced() {
    let server = server_with_one_secret();
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let tool = McpDynamicTool::new_with_domain(
        bridge
            .catalog()
            .get("mcp:demo:leak")
            .expect("the tool is discovered"),
        Arc::clone(&bridge.connections()[0]),
        Arc::clone(&domain),
    );

    // 工具面: 真值 → 占位符 (模型面只见占位符)。
    let tool_text = result_text(&tool.invoke(&call("mcp:demo:leak")).await);
    assert!(!tool_text.contains(SECRET), "真值不得进模型面: {tool_text}");
    let aliases = placeholders(&tool_text);
    assert_eq!(aliases.len(), 1, "工具面产一个占位符: {tool_text}");

    // 出网闸: 同一个域 ⇒ 受控回程认账 ⇒ 还原成真值。
    let gate = RedactionEgressGate::with_domain(Arc::clone(&domain));
    let mut response = response_with(tool_text.clone());
    gate.restore(&mut response);
    assert!(
        response.content.contains(SECRET),
        "同域 ⇒ 出网闸能还原工具面产的占位符: {}",
        response.content
    );
    assert!(
        !response.content.contains("__AP_"),
        "还原后不留占位符: {}",
        response.content
    );
    // 证据面只报**别名**与判定, 不报真值 (真值不得进任何输出面)。
    println!(
        "cross-family: tool face alias = {} | gate restored the alias = {} | \
         alias left in the restored text = {}",
        aliases[0],
        response.content.contains(SECRET),
        response.content.contains("__AP_")
    );
}

/// ③ (the other direction): the alias the **gate** produced is recognised by
/// the same domain the tool face sweeps in — the two faces are one domain,
/// not two that happen to sit in one process.
#[tokio::test]
async fn the_domain_restores_the_alias_the_gate_produced() {
    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let gate = RedactionEgressGate::with_domain(Arc::clone(&domain));

    // 出网闸: 请求里的真值 → 占位符 (出境的只有占位符)。
    let request = request_with(&format!("password = {SECRET}"));
    let outgoing = match gate.vet(&request) {
        apeireth_runtime::canonical::egress_gate::EgressVerdict::Redacted(redacted) => {
            match &redacted.messages[0].content[0] {
                ContentPart::Text { text } => text.clone(),
                other => panic!("should be text: {other:?}"),
            }
        }
        other => panic!("should redact: {other:?}"),
    };
    assert!(!outgoing.contains(SECRET), "真值不得出境: {outgoing}");
    let aliases = placeholders(&outgoing);
    assert_eq!(aliases.len(), 1, "闸产一个占位符: {outgoing}");

    // 同一个域: 工具面认账 ⇒ 还原。
    let mut guard = domain.lock().expect("the domain is not poisoned");
    let report = guard.restore_for_local(&outgoing);
    assert!(
        report.text.contains(SECRET),
        "同域 ⇒ 域认得闸产的占位符: {}",
        report.text
    );
    assert_eq!(report.restored.len(), 1);
}

/// ④ **Not wired ⇒ the behaviour before this batch**: a gate that was not
/// handed a domain keeps its own, so it does **not** recognise the alias the
/// tool face produced — the cross-family recognition is a property of the
/// *wiring*, not of the two living in one process.
#[tokio::test]
async fn an_unwired_gate_keeps_its_own_domain() {
    let server = server_with_one_secret();
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let tool = McpDynamicTool::new_with_domain(
        bridge
            .catalog()
            .get("mcp:demo:leak")
            .expect("the tool is discovered"),
        Arc::clone(&bridge.connections()[0]),
        Arc::clone(&domain),
    );
    let tool_text = result_text(&tool.invoke(&call("mcp:demo:leak")).await);
    assert!(!tool_text.contains(SECRET), "真值不得进模型面: {tool_text}");

    // 默认闸: 自建域 ⇒ 认不出工具面的占位符（与接线前一致）。
    let gate = RedactionEgressGate::new();
    let mut response = response_with(tool_text.clone());
    gate.restore(&mut response);
    assert!(
        response.content.contains("__AP_"),
        "未接域 ⇒ 闸认不出工具面的占位符（与接线前一致）: {}",
        response.content
    );
    assert!(
        !response.content.contains(SECRET),
        "未接域 ⇒ 工具面的真值不会经闸的受控回程泄漏: {}",
        response.content
    );
}

/// The gate's own round trip is unchanged by the domain move: the value the
/// gate redacted on the way out is the value the gate restores on the way
/// back (its own domain, registered by its own `vet`).
#[tokio::test]
async fn the_gate_round_trip_is_unchanged() {
    let gate = RedactionEgressGate::new();
    let request = request_with(&format!("password = {SECRET}"));
    let outgoing = match gate.vet(&request) {
        apeireth_runtime::canonical::egress_gate::EgressVerdict::Redacted(redacted) => {
            match &redacted.messages[0].content[0] {
                ContentPart::Text { text } => text.clone(),
                other => panic!("should be text: {other:?}"),
            }
        }
        other => panic!("should redact: {other:?}"),
    };
    assert!(!outgoing.contains(SECRET), "真值不得出境: {outgoing}");

    let mut response = response_with(outgoing);
    gate.restore(&mut response);
    assert!(
        response.content.contains(SECRET),
        "闸自己的往返必须保真: {}",
        response.content
    );
}

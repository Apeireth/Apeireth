//! **Class B channel × the provider-facing egress gate** — the cross-family
//! half of "one session, one domain".
//!
//! Two faces meet here:
//!
//! * the **external tool result face** (tools crate) — the tool answers with a
//!   credential it is *contracted* to return;
//! * the **provider-facing egress gate** (this crate) — the model-context
//!   boundary, which redacts on the way out and restores on the controlled
//!   return path.
//!
//! What is pinned here:
//!
//! * **⑤ cross-family**: with **one shared domain**, the placeholder the class
//!   B tool face produced is restored by the **gate** — the gate's local
//!   consumption face recognises the tool face's alias ⇒ "one domain" is
//!   really in place, not two domains that happen to sit in one process;
//! * **③ the hard red line**: the gate's **automatic** controlled return path
//!   (model reply → local, i.e. what lands in the transcript) does **not**
//!   restore a class B placeholder ⇒ the class B true value never reaches the
//!   transcript — only the **explicit** local consumption face hands it out;
//! * the outgoing request (the model-visible face) carries **no** true value;
//! * the gate's own round trip (its own redaction → its own restore) is
//!   unchanged by any of this.

use std::sync::{Arc, Mutex};
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
    ClassBDeclaration, InProcessMcpServer, McpBridgeOptions, McpPermissionMapping, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec, SessionRedactionDomain,
};

const SERVER: &str = "demo";

/// A credential shape the placeholder family covers.
const OPENAI_KEY: &str = "sk-proj-1234567890abcdef1234567890";

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

async fn call_external(bridge: &McpToolBridge, tool_name: &str) -> ToolResult {
    let entry = bridge
        .catalog()
        .get(tool_name)
        .expect("the discovered tool is in the catalog");
    McpPermissionMapping::authorize(&mut bridge.policy().lock().expect("policy mutex"), &entry);
    let tool = bridge
        .tools()
        .into_iter()
        .find(|tool| tool.declaration().name == tool_name)
        .expect("the discovered tool is wrapped and live");
    tool.invoke(&call(tool_name)).await
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

/// A server whose single tool answers with the credential.
fn server_answering(answer: &str) -> Arc<InProcessMcpServer> {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("get_my_api_key", answer)]);
    server
}

/// One shared domain: the tool face sweeps in it, the gate holds it.
async fn class_b_tool_face() -> (Arc<Mutex<SessionRedactionDomain>>, String) {
    let server = server_answering(&format!("your key is {OPENAI_KEY}"));
    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let bridge = bridge_over(&server, test_options())
        .with_session_domain(Arc::clone(&domain))
        .with_class_b_declaration(ClassBDeclaration::with_tools(["mcp:demo:get_my_api_key"]));
    bridge.refresh().await.expect("discovery succeeds");
    let text = result_text(&call_external(&bridge, "mcp:demo:get_my_api_key").await);
    (domain, text)
}

/// ⑤ **Cross-family**: the gate's **local consumption face** restores the
/// placeholder the class B tool face produced — because both hold the **same**
/// domain instance.
///
/// This is the proof that "one session, one domain" is really in place: the
/// two faces recognise each other's aliases.
#[tokio::test]
async fn the_gate_restores_the_class_b_alias_the_tool_face_produced() {
    let (domain, tool_text) = class_b_tool_face().await;

    // 工具面: 真值 → 占位符（模型面只见占位符）。
    assert!(
        !tool_text.contains(OPENAI_KEY),
        "真值不得进模型面: {tool_text}"
    );
    let aliases = placeholders(&tool_text);
    assert_eq!(aliases.len(), 1, "工具面产一个占位符: {tool_text}");

    // 出网闸: 同一个域 ⇒ **本地消费面**认账 ⇒ 还原成真值.
    let gate = RedactionEgressGate::with_domain(Arc::clone(&domain));
    let report = gate.restore_for_local_consumer(&tool_text);
    assert!(
        report.text.contains(OPENAI_KEY),
        "同域 ⇒ 出网闸的本地消费面能还原工具面产的占位符: {}",
        report.text
    );
    assert_eq!(report.restored.len(), 1);
}

/// ⑤ (the port face): the same path through the **port** the runtime holds —
/// the host reaches the gate as `Arc<dyn EgressGate>`.
#[tokio::test]
async fn the_gate_port_exposes_the_same_local_consumption_face() {
    let (domain, tool_text) = class_b_tool_face().await;
    let gate: Arc<dyn EgressGate> = Arc::new(RedactionEgressGate::with_domain(Arc::clone(&domain)));
    let restored = gate.restore_for_local_consumer(&tool_text);
    assert!(
        restored.text.contains(OPENAI_KEY),
        "端口面同一条路径: {}",
        restored.text
    );
    assert_eq!(restored.restored.len(), 1);
    // Debug 面零真值（本地消费面的结果也不许把真值写进日志面）.
    let debug = format!("{restored:?}");
    assert!(!debug.contains(OPENAI_KEY), "Debug 面零真值: {debug}");
}

/// ③ **The hard red line**: the gate's **automatic** controlled return path
/// (model reply → local, i.e. what lands in the transcript) does **not**
/// restore a class B placeholder ⇒ the class B true value never reaches the
/// transcript.
///
/// The class B value leaves the domain only through the **explicit** local
/// consumption face — that is what makes it a channel instead of a leak.
#[tokio::test]
async fn the_automatic_return_path_leaves_the_class_b_placeholder_alone() {
    let (domain, tool_text) = class_b_tool_face().await;
    let gate = RedactionEgressGate::with_domain(Arc::clone(&domain));

    // 模型回信里带回了占位符（模型只能看到占位符）。
    let mut response = response_with(format!("我拿到的是 {tool_text}"));
    gate.restore(&mut response);

    assert!(
        response.content.contains("__AP_"),
        "受控回程**不得**还原类 B 占位符（否则真值进 transcript）: {}",
        response.content
    );
    assert!(
        !response.content.contains(OPENAI_KEY),
        "类 B 真值不得进 transcript: {}",
        response.content
    );

    // 而**显式**的本地消费面拿得到 ⇒ 通道是通的, 只是**必须显式**。
    let report = gate.restore_for_local_consumer(&response.content);
    assert!(
        report.text.contains(OPENAI_KEY),
        "显式本地消费面拿得到真值: {}",
        report.text
    );
}

/// The model-visible face stays clean: the request the gate lets out carries
/// the placeholder, never the true value.
#[tokio::test]
async fn the_outgoing_request_carries_no_true_value() {
    let (domain, tool_text) = class_b_tool_face().await;
    let gate = RedactionEgressGate::with_domain(Arc::clone(&domain));

    let request = request_with(&tool_text);
    // 判定可能是 `Pass`（请求里只有占位符 ⇒ 没有可脱敏对象）或 `Redacted`
    // （占位符之外还有可脱敏对象）—— 两种都合法; **真值一律不得出境**.
    let outgoing = match gate.vet(&request) {
        apeireth_runtime::canonical::egress_gate::EgressVerdict::Redacted(redacted) => {
            match &redacted.messages[0].content[0] {
                ContentPart::Text { text } => text.clone(),
                other => panic!("should be text: {other:?}"),
            }
        }
        apeireth_runtime::canonical::egress_gate::EgressVerdict::Pass => {
            match &request.messages[0].content[0] {
                ContentPart::Text { text } => text.clone(),
                other => panic!("should be text: {other:?}"),
            }
        }
        other => panic!("should not deny: {other:?}"),
    };
    assert!(!outgoing.contains(OPENAI_KEY), "真值不得出境: {outgoing}");
    assert!(outgoing.contains("__AP_"), "出境面只见占位符: {outgoing}");
}

/// The gate's **own** round trip is unchanged by any of this: the value the
/// gate redacted on the way out is the value the gate restores on the way
/// back (its own registry, its own restore).
#[tokio::test]
async fn the_gate_own_round_trip_is_unchanged() {
    let gate = RedactionEgressGate::new();
    let request = request_with(&format!("password = {OPENAI_KEY}"));
    let outgoing = match gate.vet(&request) {
        apeireth_runtime::canonical::egress_gate::EgressVerdict::Redacted(redacted) => {
            match &redacted.messages[0].content[0] {
                ContentPart::Text { text } => text.clone(),
                other => panic!("should be text: {other:?}"),
            }
        }
        other => panic!("should redact: {other:?}"),
    };
    assert!(!outgoing.contains(OPENAI_KEY), "真值不得出境: {outgoing}");

    let mut response = response_with(outgoing);
    gate.restore(&mut response);
    assert!(
        response.content.contains(OPENAI_KEY),
        "闸自己的往返必须保真: {}",
        response.content
    );
}

/// **Unwired** ⇒ the behaviour before this batch: a gate that was not handed a
/// domain does not recognise the tool face's aliases at all.
#[tokio::test]
async fn an_unwired_gate_keeps_its_own_domain() {
    let (_domain, tool_text) = class_b_tool_face().await;
    let gate = RedactionEgressGate::new();
    let mut response = response_with(tool_text.clone());
    gate.restore(&mut response);
    assert!(
        response.content.contains("__AP_"),
        "未接域 ⇒ 闸认不出工具面的占位符（与接线前一致）: {}",
        response.content
    );
    assert!(
        !response.content.contains(OPENAI_KEY),
        "未接域 ⇒ 工具面的真值不会经闸的受控回程泄漏: {}",
        response.content
    );
}

/// ④ **The production wiring is off by default**, and its criterion is an
/// **already registered** env name — this batch adds **no** `APEIRETH_*` name.
///
/// The shared domain exists only when the egress gate is on; with the gate off
/// the gate is not even assembled and the external tool bridge keeps its own
/// per-tool domains ⇒ byte-for-byte the behaviour before this batch.
#[test]
fn the_shared_session_domain_is_off_unless_the_egress_gate_is_on() {
    use apeireth_runtime_assembly::egress_gate::{
        egress_gate_enabled_from_value, shared_session_domain, EGRESS_GATE_ENV,
    };

    // 判据是**纯函数** ⇒ 可直接测（不碰 env）.
    for off in ["", "  ", "0", "false", "FALSE"] {
        assert!(
            !egress_gate_enabled_from_value(off),
            "{off:?} ⇒ 出网闸关 ⇒ 域注入未开启"
        );
    }
    for on in ["1", "{\"version\":1}"] {
        assert!(
            egress_gate_enabled_from_value(on),
            "{on:?} ⇒ 出网闸开 ⇒ 域注入开启"
        );
    }

    // env 未设 ⇒ 没有共享域（测试进程里可能已被别处设置，故先读一眼）.
    let raw = std::env::var(EGRESS_GATE_ENV).ok();
    if !raw
        .as_deref()
        .map(egress_gate_enabled_from_value)
        .unwrap_or(false)
    {
        assert!(
            shared_session_domain().is_none(),
            "默认关 ⇒ 没有共享域 ⇒ 两面各持各的域（逐字节不变）"
        );
    }
}

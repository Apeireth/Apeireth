//! **跨族同域 × 按会话选域** —— 装配层的生产接线路径.
//!
//! # 这一批钉住什么
//!
//! 3c-1 落了「一个会话一个域」的**持有者**，3c-2 落了类 B 通道；本批把**粒度**
//! 从「**一进程一域**」落到「**按会话选域**」，并把**跨族同域**在**生产接线**
//! 上钉死：
//!
//! * **③ 跨族同域**：类 B 工具产的占位符 ⇒ **出网闸能还原** —— 走的是**生产
//!   接线**的那条路（装配根装的**共享槽** ⇒ 闸与桥取**同一个实例**），不是
//!   手工注入一只域；
//! * **跨会话不可互还原**：闸在**另一个会话**里认不出这一会话的占位符；
//! * **受控回程面不还原类 B**：自动回程（模型回信 → 本地，即 transcript 落点）
//!   **认不出**类 B 占位符 ⇒ 类 B 真值**永不**进 transcript；
//! * **默认关**：共享槽里没有注册表 ⇒ 闸保持自己的域（与接线前逐字节一致）。

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::SessionId;
use apeireth_governance::input_security::RedactionPlaceholder;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_runtime_assembly::egress_gate::{
    attach_session_registry, install_shared_session_registry, shared_session_registry,
    RedactionEgressGate, SessionDomainScopePlugin,
};
use apeireth_tools_canonical::mcp_bridge::{
    ClassBDeclaration, InProcessMcpServer, McpBridgeOptions, McpPermissionMapping, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec, SessionDomainRegistry,
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

/// A server with one tool that answers with a **true** credential.
fn server_answering(answer: &str) -> Arc<InProcessMcpServer> {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("get_my_api_key", answer)]);
    server
}

/// The **production** shared slot: the assembly root installs **one** registry
/// there; the gate and the external tool bridge take **that** instance.
///
/// The slot is process-wide and first-wins ⇒ every test in this binary shares
/// **one** instance (which is exactly the production shape).
fn production_registry() -> Arc<SessionDomainRegistry> {
    let _ = install_shared_session_registry(Arc::new(SessionDomainRegistry::new()));
    shared_session_registry().expect("the shared slot holds the registry")
}

/// The **production** bridge: the registry comes from the shared slot (the
/// same instance the gate holds) ⇒ the two faces are **one** family.
fn class_b_bridge(session: SessionId) -> (McpToolBridge, Arc<SessionDomainRegistry>) {
    let server = server_answering(&format!("your key is {OPENAI_KEY}"));
    let registry = production_registry();
    let bridge = bridge_over(&server, test_options())
        .with_session_registry(Arc::clone(&registry))
        .with_class_b_declaration(ClassBDeclaration::with_tools(["mcp:demo:get_my_api_key"]));
    let _ = session;
    (bridge, registry)
}

/// Call the declared class B tool **in a session**.
async fn call_class_b(bridge: &McpToolBridge, session: SessionId) -> String {
    let entry = bridge
        .catalog()
        .get("mcp:demo:get_my_api_key")
        .expect("the discovered tool is in the catalog");
    McpPermissionMapping::authorize(&mut bridge.policy().lock().expect("policy mutex"), &entry);
    let tool = bridge
        .tools()
        .into_iter()
        .find(|tool| tool.declaration().name == "mcp:demo:get_my_api_key")
        .expect("the discovered tool is wrapped and live");
    let result = tool
        .invoke_in_session(&call("mcp:demo:get_my_api_key"), None, Some(session))
        .await;
    result_text(&result)
}

/// ③ **Cross-family, on the production wiring path**: the class B tool face
/// mints a placeholder in session S ⇒ the **gate** — holding the **same**
/// registry instance — restores the true value through its **explicit local
/// consumption face** for **that** session.
#[tokio::test]
async fn the_gate_restores_the_class_b_alias_of_the_same_session() {
    let session = SessionId::new();
    let (bridge, _registry) = class_b_bridge(session);
    bridge.refresh().await.expect("discovery succeeds");

    let tool_text = call_class_b(&bridge, session).await;

    // 工具面: 真值 → 占位符（模型面只见占位符）。
    assert!(
        !tool_text.contains(OPENAI_KEY),
        "真值不得进模型面: {tool_text}"
    );
    let aliases = placeholders(&tool_text);
    assert_eq!(aliases.len(), 1, "工具面产一个占位符: {tool_text}");

    // 生产接线: 闸取**共享槽**里那**同一个实例**。
    let gate = attach_session_registry(RedactionEgressGate::new(), production_registry());

    // 同一个会话 ⇒ 出网闸的**本地消费面**认账 ⇒ 还原成真值。
    let report = gate.restore_for_local_consumer_in_session(Some(session), &tool_text);
    assert!(
        report.text.contains(OPENAI_KEY),
        "同会话同域 ⇒ 出网闸的本地消费面能还原工具面产的占位符: {}",
        report.text
    );
    assert_eq!(report.restored.len(), 1);
}

/// The other half of the granularity: **another session cannot restore it**.
/// Same registry, different session ⇒ different domain ⇒ no recognition.
#[tokio::test]
async fn the_gate_cannot_restore_the_alias_of_another_session() {
    let session = SessionId::new();
    let other_session = SessionId::new();
    let (bridge, _registry) = class_b_bridge(session);
    bridge.refresh().await.expect("discovery succeeds");

    let tool_text = call_class_b(&bridge, session).await;
    let gate = attach_session_registry(RedactionEgressGate::new(), production_registry());

    let report = gate.restore_for_local_consumer_in_session(Some(other_session), &tool_text);
    assert_eq!(
        report.text, tool_text,
        "跨会话 ⇒ 闸认不出这一会话的占位符（不可互还原）"
    );
    assert!(report.restored.is_empty(), "跨会话不得有还原记录");
}

/// The **hard red line** survives the granularity change: the gate's
/// **automatic** controlled return path (model reply → local, i.e. what lands
/// in the transcript) does **not** restore a class B placeholder ⇒ the class B
/// true value never reaches the transcript.
#[tokio::test]
async fn the_automatic_return_path_still_leaves_the_class_b_placeholder_alone() {
    let session = SessionId::new();
    let (bridge, _registry) = class_b_bridge(session);
    bridge.refresh().await.expect("discovery succeeds");

    let tool_text = call_class_b(&bridge, session).await;
    let gate = attach_session_registry(RedactionEgressGate::new(), production_registry());

    // 受控回程面（自动）：只认**占位族**登记簿 ⇒ 认不出类 B 占位符。
    let mut response = apeireth_protocol::canonical::NormalizedResponse {
        id: "resp-1".into(),
        model: "test-model".into(),
        content: format!("我拿到的是 {tool_text}"),
        finish_reason: None,
        usage: Default::default(),
        tool_calls: Vec::new(),
        raw_metadata: Default::default(),
    };
    use apeireth_runtime::canonical::egress_gate::EgressGate;
    gate.restore_in_session(Some(session), &mut response);
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
}

/// ⑤ **The production lifecycle**: the assembly registers the **session domain
/// scope** plugin ⇒ it declares the `SessionEnd` hook ⇒ when the runtime ends a
/// session, that session's domain is destroyed ⇒ its registry (the place the
/// true values live) is gone.
#[tokio::test]
async fn the_session_end_hook_destroys_the_session_domain() {
    use apeireth_core::kernel::{RequestId, TraceId};
    use apeireth_plugin::hooks::HookContextV1;
    use apeireth_plugin::Plugin;
    use apeireth_protocol::canonical::ModuleHookV1;

    let session = SessionId::new();
    let registry = production_registry();
    let plugin = SessionDomainScopePlugin::new(Arc::clone(&registry));
    assert_eq!(
        plugin.hooks(),
        vec![ModuleHookV1::SessionEnd],
        "插件声明会话结束钩子"
    );

    // 先用一次调用把真值登记进那只域。
    let (bridge, _registry) = class_b_bridge(session);
    bridge.refresh().await.expect("discovery succeeds");
    let _ = call_class_b(&bridge, session).await;
    assert_eq!(
        registry
            .domain_for(session)
            .lock()
            .expect("the domain is not poisoned")
            .registered_values(),
        1,
        "调用之后域里有一份登记"
    );

    // 会话结束 ⇒ 域销毁。
    let context = HookContextV1 {
        hook: ModuleHookV1::SessionEnd,
        session,
        request: RequestId::new(),
        trace: TraceId::new(),
        messages: Arc::new([]),
        candidate: None,
        tool_call: None,
        tool_result: None,
    };
    plugin
        .on_hook(&context)
        .await
        .expect("the session-end hook never fails");
    assert!(!registry.contains(session), "会话结束 ⇒ 域销毁");
    assert_eq!(
        registry
            .domain_for(session)
            .lock()
            .expect("the domain is not poisoned")
            .registered_values(),
        0,
        "重新建这只会话的域 ⇒ 登记簿是空的 ⇒ 真值没有延续"
    );
}

/// ④ **Default off**: with **no** registry in the shared slot the gate keeps
/// its own domain — the behaviour before this batch, byte for byte.
#[test]
fn a_gate_without_a_registry_keeps_its_own_domain() {
    // 这只闸**不挂**注册表 ⇒ 它认不出工具面的占位符。
    let gate = RedactionEgressGate::new();
    let report =
        gate.restore_for_local_consumer("含 __AP_CREDENTIAL_KEY_deadbeef0000__ 这个占位符");
    assert_eq!(
        report.text, "含 __AP_CREDENTIAL_KEY_deadbeef0000__ 这个占位符",
        "未挂注册表 ⇒ 闸用自己的域 ⇒ 认不出工具面的占位符（与接线前一致）"
    );
    assert!(report.restored.is_empty(), "未挂注册表 ⇒ 没有还原记录");
}

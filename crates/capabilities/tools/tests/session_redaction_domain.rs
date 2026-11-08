//! One session, one redaction domain.
//!
//! The placeholder family is only useful if every surface of one session
//! agrees on the alias of a value. Before this batch the external tool result
//! face gave **every tool its own domain**, so the same value got two
//! aliases inside one session: the model could not correlate them, and the
//! controlled return path of one surface could not restore what another
//! surface produced.
//!
//! What is pinned here:
//!
//! * one session, two external tools, one value ⇒ **one alias**;
//! * two sessions, one value ⇒ **two aliases** that cannot restore each other;
//! * an **unwired** bridge keeps one domain per tool — the behaviour before
//!   this batch, byte for byte;
//! * no true value reaches an output, a log or the ledger — only the alias.
//!
//! The cross-family case (the provider-facing egress gate restoring what the
//! tool face aliased) lives in the runtime assembly's own test, because the
//! gate lives there.

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::SessionId;
use apeireth_governance::input_security::RedactionPlaceholder;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::mcp_bridge::{
    derive_session_key, InProcessMcpServer, McpBridgeOptions, McpDynamicTool, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec, SessionRedactionDomain,
};

const SERVER: &str = "demo";

/// A password assignment the placeholder family covers (an alias, not a label).
const SECRET: &str = "hunter2secret";
/// A fragment of the true value — the alias must never carry it.
const SECRET_FRAGMENT: &str = "hunter2";

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

/// Assemble a bridge over one in-process server, exactly like the production
/// assembly does.
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

/// The joined text of a successful result.
fn result_text(result: &ToolResult) -> String {
    let ToolOutcome::Ok { value } = &result.outcome else {
        panic!("expected a successful result: {}", result.render());
    };
    value["text"]
        .as_str()
        .expect("the joined text is a string")
        .to_string()
}

/// Every placeholder of the family inside one text (the mechanism's own
/// scanner — the format is not hardcoded here).
fn placeholders(text: &str) -> Vec<String> {
    RedactionPlaceholder::scan(text)
        .into_iter()
        .map(|matched| matched.placeholder.render())
        .collect()
}

/// A server whose two tools answer with the **same** secret.
fn server_with_one_secret_in_two_tools() -> Arc<InProcessMcpServer> {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![
        MockToolSpec::new("alpha", format!("password = {SECRET}")),
        MockToolSpec::new("beta", format!("password = {SECRET}")),
    ]);
    server
}

/// ① **One session, one domain**: the same value in two external tools of one
/// session is the **same alias** — the model can correlate them, and the
/// domain's controlled return path recognises both.
#[tokio::test]
async fn one_session_gives_two_tools_the_same_alias() {
    let server = server_with_one_secret_in_two_tools();
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let alpha = McpDynamicTool::new_with_domain(
        bridge
            .catalog()
            .get("mcp:demo:alpha")
            .expect("alpha is discovered"),
        Arc::clone(&bridge.connections()[0]),
        Arc::clone(&domain),
    );
    let beta = McpDynamicTool::new_with_domain(
        bridge
            .catalog()
            .get("mcp:demo:beta")
            .expect("beta is discovered"),
        Arc::clone(&bridge.connections()[0]),
        Arc::clone(&domain),
    );

    let alpha_text = result_text(&alpha.invoke(&call("mcp:demo:alpha")).await);
    let beta_text = result_text(&beta.invoke(&call("mcp:demo:beta")).await);

    let alpha_aliases = placeholders(&alpha_text);
    let beta_aliases = placeholders(&beta_text);
    assert_eq!(alpha_aliases.len(), 1, "one alias per answer: {alpha_text}");
    assert_eq!(beta_aliases.len(), 1, "one alias per answer: {beta_text}");
    assert_eq!(
        alpha_aliases, beta_aliases,
        "同会话跨两个工具 ⇒ 同一个占位符\nalpha: {alpha_text}\nbeta: {beta_text}"
    );
    assert!(
        !alpha_text.contains(SECRET) && !beta_text.contains(SECRET),
        "真值不得进模型面"
    );
}

/// ① (the domain's own face): the two tools registered the value **once** —
/// the registry holds one true value, not one per tool.
#[tokio::test]
async fn one_domain_registers_the_shared_value_once() {
    let server = server_with_one_secret_in_two_tools();
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    for (name, tool) in [
        (
            "mcp:demo:alpha",
            McpDynamicTool::new_with_domain(
                bridge
                    .catalog()
                    .get("mcp:demo:alpha")
                    .expect("alpha is discovered"),
                Arc::clone(&bridge.connections()[0]),
                Arc::clone(&domain),
            ),
        ),
        (
            "mcp:demo:beta",
            McpDynamicTool::new_with_domain(
                bridge
                    .catalog()
                    .get("mcp:demo:beta")
                    .expect("beta is discovered"),
                Arc::clone(&bridge.connections()[0]),
                Arc::clone(&domain),
            ),
        ),
    ] {
        let text = result_text(&tool.invoke(&call(name)).await);
        assert!(!text.contains(SECRET), "真值不得进模型面: {text}");
    }

    let guard = domain.lock().expect("the domain is not poisoned");
    assert_eq!(
        guard.registered_values(),
        1,
        "同域同值只登记一次 ⇒ 两个工具共用一个占位符"
    );
}

/// ② **Across sessions**: two domains, two aliases, and neither can restore
/// the other's alias.
#[test]
fn two_sessions_get_two_aliases_that_cannot_restore_each_other() {
    let mut first = SessionRedactionDomain::for_session(SessionId::new());
    let mut second = SessionRedactionDomain::for_session(SessionId::new());

    let first_text = first
        .redact_for_egress(&format!("password = {SECRET}"))
        .text;
    let second_text = second
        .redact_for_egress(&format!("password = {SECRET}"))
        .text;
    assert_ne!(first_text, second_text, "跨会话必须换别名");
    assert_eq!(placeholders(&first_text).len(), 1);
    assert_eq!(placeholders(&second_text).len(), 1);

    // 互不还原: 另一个域看着这个占位符只能照实留痕。
    let report = second.restore_for_local(&first_text);
    assert_eq!(report.text, first_text, "跨会话的占位符不得被别的域还原");
    assert!(report.restored.is_empty(), "跨会话不得有还原记录");
    assert_eq!(report.unrestored.len(), 1, "未登记占位符照实留痕");
}

/// ④ **Not wired ⇒ byte-for-byte the behaviour before this batch**: a bridge
/// without an injected domain keeps **one domain per tool**, so the same value
/// in two tools gets two aliases — exactly as before the session domain
/// existed. The sweep itself is unchanged (the value is still aliased).
#[tokio::test]
async fn an_unwired_bridge_keeps_one_domain_per_tool() {
    let server = server_with_one_secret_in_two_tools();
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");
    assert!(
        bridge.session_domain().is_none(),
        "默认不接域 ⇒ 行为与接线前一致"
    );

    let alpha = McpDynamicTool::new(
        bridge
            .catalog()
            .get("mcp:demo:alpha")
            .expect("alpha is discovered"),
        Arc::clone(&bridge.connections()[0]),
    );
    let beta = McpDynamicTool::new(
        bridge
            .catalog()
            .get("mcp:demo:beta")
            .expect("beta is discovered"),
        Arc::clone(&bridge.connections()[0]),
    );

    let alpha_text = result_text(&alpha.invoke(&call("mcp:demo:alpha")).await);
    let beta_text = result_text(&beta.invoke(&call("mcp:demo:beta")).await);

    let alpha_aliases = placeholders(&alpha_text);
    let beta_aliases = placeholders(&beta_text);
    assert_eq!(alpha_aliases.len(), 1, "仍然扫（能力未丢）: {alpha_text}");
    assert_eq!(beta_aliases.len(), 1, "仍然扫（能力未丢）: {beta_text}");
    assert_ne!(
        alpha_aliases, beta_aliases,
        "未接域 ⇒ 每工具一域 ⇒ 两个别名（与接线前一致）"
    );
    assert!(!alpha_text.contains(SECRET) && !beta_text.contains(SECRET));
}

/// ⑤ **Zero true value on every face**: the swept text, the ledger and the
/// debug face carry the alias, never the value.
#[test]
fn no_output_ledger_or_debug_face_carries_a_true_value() {
    let mut domain = SessionRedactionDomain::for_session(SessionId::new());
    let outcome = domain.sweep_result_face(&format!("password = {SECRET}"));
    let swept = outcome.text();
    assert!(!swept.contains(SECRET), "扫面不得含真值: {swept}");
    assert!(
        !swept.contains(SECRET_FRAGMENT),
        "扫面不得含真值片段: {swept}"
    );

    let ledger = domain.ledger_jsonl();
    assert!(!ledger.contains(SECRET), "账本零真值: {ledger}");
    assert!(!ledger.contains(SECRET_FRAGMENT), "账本零真值: {ledger}");
    assert!(ledger.contains("redacted"), "账本记事件: {ledger}");

    let debug = format!("{domain:?}");
    assert!(!debug.contains(SECRET), "Debug 面零真值: {debug}");
    assert!(!debug.contains(SECRET_FRAGMENT), "Debug 面零真值: {debug}");
}

/// The session key is **derived**, never a literal: the same session derives
/// the same key, another session another one, and the key never shows up on
/// a face that is written down.
#[test]
fn the_session_key_is_derived_per_session_and_never_written_down() {
    let session = SessionId::new();
    assert_eq!(
        derive_session_key(session),
        derive_session_key(session),
        "同会话恒同密钥 ⇒ 同一别名"
    );
    assert_ne!(
        derive_session_key(session),
        derive_session_key(SessionId::new()),
        "跨会话换密钥 ⇒ 换别名"
    );

    // 密钥不进 Debug 面 (域的 Debug 只报会话 id 与计数)。
    let domain = SessionRedactionDomain::for_session(session);
    let debug = format!("{domain:?}");
    let key_hex: String = derive_session_key(session)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert!(
        !debug.contains(&key_hex),
        "密钥不得出现在 Debug 面: {debug}"
    );
    // 逐段看 (8 位一段): 会话 id 的十进制/UUID 文本不会撞上密钥片段。
    for chunk in key_hex.as_bytes().chunks(8) {
        let fragment = std::str::from_utf8(chunk).expect("hex is ascii");
        assert!(
            !debug.contains(fragment),
            "密钥片段不得出现在 Debug 面: {debug} (片段 {fragment})"
        );
    }
}

//! **域注册表** —— 把占位域从「一进程一域」改成「**按会话选域**」.
//!
//! # 缺陷（本批修的那一个）
//!
//! 3c-1 落的 [`SessionRedactionDomain`] 是「一个会话一个域」的**持有者**，但
//! **注入面是进程级**的：外部工具桥持**一只**域 ⇒ ⇒ **一进程一域** ⇒ ⇒ 多会话
//! 并发时，**跨会话的同一个值**会得到**同一个占位符** ⇒ ⇒ 跨会话可关联 ⇒ ⇒
//! 违裁决 #4「一个会话一个域」。
//!
//! # 这里钉住的判据
//!
//! * **① 跨会话隔离**：同一会话内跨两个工具 ⇒ **同一个**占位符；**跨会话 ⇒
//!   不同**占位符且**不可互还原**（本批的核心判据，直接钉住那个粒度缺陷）；
//! * **② 逐调用按会话取域**：一次调用携带的会话标识决定取**哪个**域；
//! * **④ 默认关**：不装注册表 ⇒ 行为**逐字节不变**（每工具一域，接线前的样子）；
//! * **⑤ 会话结束 ⇒ 域销毁**（登记簿随之消失）；
//! * **⑥ 零真值泄漏**：扫面 / 账本 / `Debug` 面都**零真值**。
//!
//! 跨族那半（出网闸认账工具面产的占位符）在装配层的自己的测试里，因为闸在
//! 那一层。

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::SessionId;
use apeireth_governance::input_security::RedactionPlaceholder;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::mcp_bridge::{
    InProcessMcpServer, McpBridgeOptions, McpPermissionMapping, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec, SessionDomainRegistry,
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

/// Call one discovered tool **in a session** (the session decides the domain).
async fn call_in_session(
    bridge: &McpToolBridge,
    tool_name: &str,
    session: SessionId,
) -> ToolResult {
    // 授权先于派发（生产接线也是这个顺序：族授权 + 逐次人批）。
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
    tool.invoke_in_session(&call(tool_name), None, Some(session))
        .await
}

/// ① **Cross-session isolation**: one session across two tools ⇒ **one** alias;
/// two sessions ⇒ **two** aliases, and neither session can restore the other's.
#[tokio::test]
async fn one_session_shares_the_alias_but_two_sessions_never_do() {
    let server = server_with_one_secret_in_two_tools();
    let registry = Arc::new(SessionDomainRegistry::new());
    let bridge = bridge_over(&server, test_options()).with_session_registry(Arc::clone(&registry));
    bridge.refresh().await.expect("discovery succeeds");

    let session_a = SessionId::new();
    let session_b = SessionId::new();

    let alpha_a = result_text(&call_in_session(&bridge, "mcp:demo:alpha", session_a).await);
    let beta_a = result_text(&call_in_session(&bridge, "mcp:demo:beta", session_a).await);
    let alpha_b = result_text(&call_in_session(&bridge, "mcp:demo:alpha", session_b).await);

    let alpha_a_aliases = placeholders(&alpha_a);
    let beta_a_aliases = placeholders(&beta_a);
    let alpha_b_aliases = placeholders(&alpha_b);

    assert_eq!(alpha_a_aliases.len(), 1, "one alias per answer: {alpha_a}");
    assert_eq!(beta_a_aliases.len(), 1, "one alias per answer: {beta_a}");
    assert_eq!(alpha_b_aliases.len(), 1, "one alias per answer: {alpha_b}");

    assert_eq!(
        alpha_a_aliases, beta_a_aliases,
        "同会话跨两个工具 ⇒ 同一个占位符\nalpha: {alpha_a}\nbeta: {beta_a}"
    );
    assert_ne!(
        alpha_a_aliases, alpha_b_aliases,
        "跨会话 ⇒ 必须换占位符（否则跨会话可关联）\nA: {alpha_a}\nB: {alpha_b}"
    );

    // 真值一步都不许进模型面。
    for text in [&alpha_a, &beta_a, &alpha_b] {
        assert!(!text.contains(SECRET), "真值不得进模型面: {text}");
        assert!(!text.contains(SECRET_FRAGMENT), "真值不得进模型面: {text}");
    }
}

/// ① (the stronger half): **cross-session cannot restore each other** — the
/// other session's domain simply does not know the fingerprint.
#[tokio::test]
async fn a_session_cannot_restore_another_session_alias() {
    let server = server_with_one_secret_in_two_tools();
    let registry = Arc::new(SessionDomainRegistry::new());
    let bridge = bridge_over(&server, test_options()).with_session_registry(Arc::clone(&registry));
    bridge.refresh().await.expect("discovery succeeds");

    let session_a = SessionId::new();
    let session_b = SessionId::new();

    let alpha_a = result_text(&call_in_session(&bridge, "mcp:demo:alpha", session_a).await);

    let domain_a = registry.domain_for(session_a);
    let domain_b = registry.domain_for(session_b);

    // A 会话的域认自己的账（受控回程面保真）。
    let mut guard_a = domain_a.lock().expect("the domain is not poisoned");
    let own = guard_a.restore_for_local(&alpha_a);
    assert!(
        own.text.contains(SECRET),
        "本会话的占位符本会话认账: {}",
        own.text
    );
    assert_eq!(own.restored.len(), 1);

    // B 会话的域**认不出** A 会话的占位符 ⇒ 照实留痕（不还原、不泄漏）。
    let mut guard_b = domain_b.lock().expect("the domain is not poisoned");
    let foreign = guard_b.restore_for_local(&alpha_a);
    assert_eq!(foreign.text, alpha_a, "跨会话的占位符不得被别的会话还原");
    assert!(foreign.restored.is_empty(), "跨会话不得有还原记录");
    assert_eq!(foreign.unrestored.len(), 1, "未登记占位符照实留痕");
}

/// ② **Per-call domain selection**: the session carried by the call decides
/// **which** domain is used — one call per session, two sessions ⇒ two domains,
/// each holding its own registration of the value.
#[tokio::test]
async fn the_call_session_decides_which_domain_is_used() {
    let server = server_with_one_secret_in_two_tools();
    let registry = Arc::new(SessionDomainRegistry::new());
    let bridge = bridge_over(&server, test_options()).with_session_registry(Arc::clone(&registry));
    bridge.refresh().await.expect("discovery succeeds");

    let session_a = SessionId::new();
    let session_b = SessionId::new();

    // 同会话两次调用 ⇒ 同一个域 ⇒ 真值只登记**一次**。
    let _ = call_in_session(&bridge, "mcp:demo:alpha", session_a).await;
    let _ = call_in_session(&bridge, "mcp:demo:alpha", session_a).await;
    assert_eq!(registry.len(), 1, "同会话 ⇒ 只有一只域");
    assert_eq!(
        registry
            .domain_for(session_a)
            .lock()
            .expect("the domain is not poisoned")
            .registered_values(),
        1,
        "同域同值只登记一次"
    );

    // 另一个会话 ⇒ **另一只**域 ⇒ 各自登记一次。
    let _ = call_in_session(&bridge, "mcp:demo:alpha", session_b).await;
    assert_eq!(registry.len(), 2, "两个会话 ⇒ 两只域");
    assert!(registry.contains(session_a) && registry.contains(session_b));
    for session in [session_a, session_b] {
        assert_eq!(
            registry
                .domain_for(session)
                .lock()
                .expect("the domain is not poisoned")
                .registered_values(),
            1,
            "每个会话的域各登记自己那一份"
        );
    }
}

/// ④ **Default off**: a bridge **without** a registry keeps the behaviour before
/// this batch, byte for byte — **one domain per tool**, so the same value in
/// two tools gets two aliases and no session can change that.
#[tokio::test]
async fn a_bridge_without_a_registry_keeps_one_domain_per_tool() {
    let server = server_with_one_secret_in_two_tools();
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");
    assert!(
        bridge.session_registry().is_none(),
        "默认不装注册表 ⇒ 行为与接线前一致"
    );

    let session = SessionId::new();
    let alpha = result_text(&call_in_session(&bridge, "mcp:demo:alpha", session).await);
    let beta = result_text(&call_in_session(&bridge, "mcp:demo:beta", session).await);

    assert_eq!(placeholders(&alpha).len(), 1, "仍然扫（能力未丢）: {alpha}");
    assert_eq!(placeholders(&beta).len(), 1, "仍然扫（能力未丢）: {beta}");
    assert_ne!(
        placeholders(&alpha),
        placeholders(&beta),
        "未装注册表 ⇒ 每工具一域 ⇒ 两个别名（与接线前一致）"
    );
    assert!(!alpha.contains(SECRET) && !beta.contains(SECRET));
}

/// ⑤ **The domain dies with the session**: ending the session destroys the
/// domain ⇒ its registry (the place the true values live) is gone ⇒ the
/// aliases it minted are no longer restorable by anybody.
#[tokio::test]
async fn ending_a_session_destroys_its_domain_and_its_registry() {
    let server = server_with_one_secret_in_two_tools();
    let registry = Arc::new(SessionDomainRegistry::new());
    let bridge = bridge_over(&server, test_options()).with_session_registry(Arc::clone(&registry));
    bridge.refresh().await.expect("discovery succeeds");

    let session = SessionId::new();
    let alpha = result_text(&call_in_session(&bridge, "mcp:demo:alpha", session).await);
    assert_eq!(registry.len(), 1, "会话用过 ⇒ 一只域");
    assert!(registry.contains(session));

    // 会话结束 ⇒ 域销毁（登记簿随之消失）。
    assert!(registry.end_session(session), "这只域确实存在过");
    assert!(!registry.end_session(session), "重复结束会话是幂等的");
    assert_eq!(registry.len(), 0, "域随会话消失");
    assert!(!registry.contains(session));

    // 重新建这只会话的域 ⇒ **空的** ⇒ 真值没有跟着走。
    let fresh = registry.domain_for(session);
    let guard = fresh.lock().expect("the domain is not poisoned");
    assert_eq!(
        guard.registered_values(),
        0,
        "新域的登记簿是空的 ⇒ 真值没有跨会话结束延续"
    );
    assert_eq!(
        guard.class_b_values(),
        0,
        "类 B 登记簿也是空的 ⇒ 真值没有延续"
    );
    drop(guard);

    // 旧占位符再也还原不出来（域已死）。
    let mut dead = fresh.lock().expect("the domain is not poisoned");
    let report = dead.restore_for_local(&alpha);
    assert_eq!(report.text, alpha, "域已销毁 ⇒ 旧占位符照实留痕");
    assert!(report.restored.is_empty(), "域已销毁 ⇒ 没有还原记录");
}

/// ⑥ **Zero true value on every face**: the swept text, the ledger and the
/// debug face carry the alias, never the value.
#[tokio::test]
async fn no_output_ledger_or_debug_face_carries_a_true_value() {
    let server = server_with_one_secret_in_two_tools();
    let registry = Arc::new(SessionDomainRegistry::new());
    let bridge = bridge_over(&server, test_options()).with_session_registry(Arc::clone(&registry));
    bridge.refresh().await.expect("discovery succeeds");

    let session = SessionId::new();
    let alpha = result_text(&call_in_session(&bridge, "mcp:demo:alpha", session).await);
    assert!(!alpha.contains(SECRET), "扫面不得含真值: {alpha}");
    assert!(!alpha.contains(SECRET_FRAGMENT), "扫面不得含真值: {alpha}");

    let domain = registry.domain_for(session);
    let guard = domain.lock().expect("the domain is not poisoned");
    let ledger = guard.ledger_jsonl();
    assert!(!ledger.contains(SECRET), "账本零真值: {ledger}");
    assert!(!ledger.contains(SECRET_FRAGMENT), "账本零真值: {ledger}");
    assert!(ledger.contains("redacted"), "账本记事件: {ledger}");
    let debug = format!("{:?}", guard);
    assert!(!debug.contains(SECRET), "Debug 面零真值: {debug}");
    assert!(!debug.contains(SECRET_FRAGMENT), "Debug 面零真值: {debug}");
    drop(guard);

    // 注册表自己的 Debug 面也只报计数。
    let registry_debug = format!("{registry:?}");
    assert!(!registry_debug.contains(SECRET), "注册表 Debug 面零真值");
    assert!(
        registry_debug.contains("sessions"),
        "注册表 Debug 面报会话数: {registry_debug}"
    );
}

/// The registry's own bookkeeping: it starts empty, it grows with the
/// sessions, and it shrinks when a session ends.
#[test]
fn the_registry_accounts_for_its_sessions() {
    let registry = SessionDomainRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);

    let first = SessionId::new();
    let second = SessionId::new();
    let first_domain = registry.domain_for(first);
    let second_domain = registry.domain_for(second);
    assert_eq!(registry.len(), 2);
    assert!(registry.contains(first) && registry.contains(second));

    // 同一会话 ⇒ 恒同一只域（**同一个实例** ⇒ 同一本登记簿）。
    let again = registry.domain_for(first);
    assert!(Arc::ptr_eq(&first_domain, &again), "同会话 ⇒ 恒同一只域");
    assert_ne!(
        Arc::ptr_eq(&first_domain, &second_domain),
        true,
        "不同会话 ⇒ 不同域"
    );

    assert!(registry.end_session(first));
    assert_eq!(registry.len(), 1);
    assert!(registry.end_session(second));
    assert!(registry.is_empty());
}

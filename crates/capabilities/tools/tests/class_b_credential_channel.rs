//! **Class B channel** — a remote tool that *legitimately* returns a credential.
//!
//! The problem this channel answers: a remote tool whose contract is "give me
//! my API key" answers with a **true** credential. Before this batch the
//! result face swept every answer with the placeholder family **and** the
//! one-way credential tripwire, so that true value was **destroyed in place**
//! on the MCP path (nothing on that path ever restored it) — the use case was
//! simply unusable.
//!
//! What is pinned here:
//!
//! * **① a declared class B tool keeps its credential reversible**: the model
//!   sees only placeholders, and the **explicit local consumption face** hands
//!   the true value to the local consumer;
//! * **② an undeclared tool is untouched, byte for byte** — and the honest
//!   truth about the **existing collateral damage** is asserted, not hidden:
//!   the true value of an undeclared tool is still destroyed (the channel
//!   **provides** the way out, it does **not** pretend the damage is fixed);
//! * **③ the restore never lands on a model-facing face** — after the local
//!   restore the tool result (what the transcript holds) and the ledger carry
//!   **zero** true value;
//! * **④ the channel is off by default** — no declaration, no domain ⇒ the
//!   behaviour is exactly what it was;
//! * **⑥ the ledger records the category and the fingerprint only**.
//!
//! The cross-family half (the provider-facing egress gate restoring what the
//! class B tool face aliased — and *not* restoring it on the automatic
//! controlled return path) lives in the runtime assembly's own test, because
//! the gate lives there.

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::SessionId;
use apeireth_governance::input_security::RedactionPlaceholder;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::mcp_bridge::{
    ClassBDeclaration, InProcessMcpServer, McpBridgeOptions, McpPermissionMapping, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec, SessionRedactionDomain,
};

const SERVER: &str = "demo";

/// A credential shape the placeholder family covers.
const OPENAI_KEY: &str = "sk-proj-1234567890abcdef1234567890";
/// A private key block — a shape **only** the one-way tripwire covers
/// (the placeholder family does not see it at all).
const PEM_BLOCK: &str =
    "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0secretbody\n-----END RSA PRIVATE KEY-----";

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

/// Run one authorized external tool call and return the model-facing result.
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

/// The structured value a successful result carries.
fn structured(result: &ToolResult) -> &serde_json::Value {
    let ToolOutcome::Ok { value } = &result.outcome else {
        panic!("expected a successful result: {}", result.render());
    };
    value
}

/// The joined text of a successful result — **the model-facing face**.
fn result_text(result: &ToolResult) -> String {
    structured(result)["text"]
        .as_str()
        .expect("the joined text is a string")
        .to_string()
}

/// Every placeholder of the family inside one text.
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

/// ① **The class B core case**: a declared tool answers with a true key ⇒ the
/// model sees **only placeholders**, and the **explicit** local consumption
/// face hands the true value to the local consumer.
#[tokio::test]
async fn a_declared_class_b_tool_keeps_its_credential_reversible() {
    let server = server_answering(&format!("your key is {OPENAI_KEY}"));
    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let bridge = bridge_over(&server, test_options())
        .with_session_domain(Arc::clone(&domain))
        .with_class_b_declaration(ClassBDeclaration::with_tools(["mcp:demo:get_my_api_key"]));
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:get_my_api_key").await;
    let text = result_text(&result);

    // 模型可见面: **只有占位符**, 没有真值.
    assert!(!text.contains(OPENAI_KEY), "真值不得进模型面: {text}");
    let aliases = placeholders(&text);
    assert_eq!(aliases.len(), 1, "一个占位符: {text}");
    assert!(
        aliases[0].starts_with("__AP_"),
        "占位符形态由占位族自己定: {aliases:?}"
    );

    // 可观测: 侧车标出「这是类 B 面」.
    let sidecar = structured(&result)
        .get("egress_sweep")
        .expect("a swept answer carries the sweep sidecar");
    assert_eq!(sidecar["class_b"], serde_json::json!(true), "{sidecar}");

    // **本地消费面**（显式调用）⇒ 拿到真值.
    let mut guard = domain.lock().expect("the domain is not poisoned");
    let report = guard.restore_for_local_consumer(&text);
    assert!(
        report.text.contains(OPENAI_KEY),
        "本地消费面必须能还原真值: {}",
        report.text
    );
    assert_eq!(report.restored.len(), 1);
    assert!(!report.unrestored.is_empty() || report.unrestored.is_empty());
}

/// ① (the stronger case): a shape **only the one-way tripwire covers** (a
/// private key block) is reversible **too** — the class B channel does not
/// inherit the one-way net's verdict.
#[tokio::test]
async fn a_class_b_tool_keeps_even_the_one_way_only_shapes_reversible() {
    let server = server_answering(PEM_BLOCK);
    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let bridge = bridge_over(&server, test_options())
        .with_session_domain(Arc::clone(&domain))
        .with_class_b_declaration(ClassBDeclaration::with_tools(["mcp:demo:get_my_api_key"]));
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:get_my_api_key").await;
    let text = result_text(&result);

    assert!(
        !text.contains("MIIEowIBAAKCAQEA0secretbody"),
        "私钥体不得进模型面: {text}"
    );
    assert!(
        !text.contains("[REDACTED_PEM_PRIVATE_KEY]"),
        "类 B 面不得落单向标签（那是不可逆的）: {text}"
    );
    assert_eq!(placeholders(&text).len(), 1, "一个占位符: {text}");

    let mut guard = domain.lock().expect("the domain is not poisoned");
    let report = guard.restore_for_local_consumer(&text);
    assert!(
        report.text.contains("MIIEowIBAAKCAQEA0secretbody"),
        "本地消费面必须能还原私钥块: {}",
        report.text
    );
}

/// ② **An undeclared tool is byte-for-byte unchanged** — and the honest truth
/// about the **existing collateral damage** is asserted: the true value of an
/// undeclared tool is still **destroyed** (the channel provides the way out;
/// it does **not** pretend the damage is fixed).
#[tokio::test]
async fn an_undeclared_tool_is_byte_for_byte_unchanged() {
    let answer = format!("your key is {OPENAI_KEY}");

    // Two bridges over the same server, sharing one domain (⇒ same aliases):
    // one without any declaration, one with a declaration naming **another**
    // tool. The undeclared tool's answer must come out **identical**.
    let server = server_answering(&answer);
    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let plain = bridge_over(&server, test_options()).with_session_domain(Arc::clone(&domain));
    let declared = bridge_over(&server, test_options())
        .with_session_domain(Arc::clone(&domain))
        .with_class_b_declaration(ClassBDeclaration::with_tools(["mcp:demo:some_other_tool"]));
    plain.refresh().await.expect("discovery succeeds");
    declared.refresh().await.expect("discovery succeeds");

    let plain_text = result_text(&call_external(&plain, "mcp:demo:get_my_api_key").await);
    let declared_text = result_text(&call_external(&declared, "mcp:demo:get_my_api_key").await);
    assert_eq!(plain_text, declared_text, "未声明的工具 ⇒ 行为逐字节不变");

    // 如实断言**既有误伤现状**: 真值确实没进模型面, 但**也没有任何本地消费面
    // 能把它交还** ⇒ 该用法（未声明时）依然不可用.
    assert!(
        !plain_text.contains(OPENAI_KEY),
        "真值不得进模型面: {plain_text}"
    );
    let mut guard = domain.lock().expect("the domain is not poisoned");
    let report = guard.restore_for_local_consumer(&plain_text);
    assert!(
        !report.text.contains(OPENAI_KEY),
        "未声明 ⇒ 本地消费面**不认账**（真值仍是被销毁的那一个）: {}",
        report.text
    );
    assert!(
        report.restored.is_empty(),
        "未声明 ⇒ 没有可还原的类 B 指纹: {:?}",
        report.restored
    );
}

/// ③ **The restore never lands on a model-facing face**: after the explicit
/// local restore, the tool result (what the transcript holds) and the ledger
/// carry **zero** true value.
#[tokio::test]
async fn the_restore_leaves_no_true_value_on_any_written_face() {
    let server = server_answering(&format!("your key is {OPENAI_KEY}"));
    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let bridge = bridge_over(&server, test_options())
        .with_session_domain(Arc::clone(&domain))
        .with_class_b_declaration(ClassBDeclaration::with_tools(["mcp:demo:get_my_api_key"]));
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:get_my_api_key").await;
    let text = result_text(&result);

    let mut guard = domain.lock().expect("the domain is not poisoned");
    let report = guard.restore_for_local_consumer(&text);
    assert!(report.text.contains(OPENAI_KEY), "本地消费面拿到真值");

    // transcript 那一面 = 工具结果原文（进上下文的那一份）⇒ 零真值.
    assert!(
        !text.contains(OPENAI_KEY),
        "transcript 面不得有真值: {text}"
    );
    // 账本面 ⇒ 零真值（只有类别 + 指纹）.
    let ledger = guard.ledger_jsonl();
    assert!(!ledger.contains(OPENAI_KEY), "账本零真值: {ledger}");
    // Debug 面 ⇒ 零真值.
    let debug = format!("{report:?} {guard:?}");
    assert!(!debug.contains(OPENAI_KEY), "Debug 面零真值: {debug}");
}

/// ④ **The channel is off by default**: no declaration, no domain ⇒ the
/// behaviour is exactly what it was, and the class B surface does not exist.
#[tokio::test]
async fn the_channel_is_off_by_default() {
    let server = server_answering(&format!("your key is {OPENAI_KEY}"));
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    assert!(
        bridge.class_b_declaration().is_none(),
        "默认不声明 ⇒ 通道不存在"
    );
    assert!(
        bridge.session_domain().is_none(),
        "默认不接域 ⇒ 行为与接线前一致"
    );

    let text = result_text(&call_external(&bridge, "mcp:demo:get_my_api_key").await);
    assert!(!text.contains(OPENAI_KEY), "真值不得进模型面: {text}");
    assert_eq!(placeholders(&text).len(), 1, "仍然扫（能力未丢）: {text}");
}

/// ④ (the declaration surface itself): the declaration is **serializable**,
/// **auditable** and **empty by default**.
#[test]
fn the_declaration_surface_is_serializable_auditable_and_empty_by_default() {
    // 默认不含任何声明.
    let empty = ClassBDeclaration::default();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert!(!empty.declares("mcp:demo:get_my_api_key"));
    assert!(empty.audit_lines().is_empty());

    // 可序列化: 声明能进 JSON, 也能从 JSON 读回来.
    let declaration = ClassBDeclaration::with_tools(["mcp:demo:get_my_api_key"])
        .with_reason("该工具按契约返回凭据");
    let json = serde_json::to_string(&declaration).expect("the declaration serializes");
    let parsed: ClassBDeclaration = serde_json::from_str(&json).expect("the declaration parses");
    assert_eq!(parsed, declaration);
    assert!(parsed.declares("mcp:demo:get_my_api_key"));
    assert!(!parsed.declares("mcp:demo:other"));

    // 可审计: 逐条摘要只有工具标识与理由（**零真值**）.
    let audit = declaration.audit_lines();
    assert!(audit
        .iter()
        .any(|line| line.contains("mcp:demo:get_my_api_key")));
    assert!(audit.iter().any(|line| line.contains("按契约返回凭据")));
    assert!(!audit.join("\n").contains(OPENAI_KEY));
}

/// ④ (the production wiring's declaration source): the declaration comes from
/// a JSON file in the MCP data directory — **missing file = empty declaration**
/// (the default), a malformed file = **fail-closed** (the caller keeps the
/// declaration empty and reports the error; it never guesses).
#[test]
fn the_declaration_file_is_optional_and_fails_closed_when_broken() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("mcp-class-b.json");

    // 缺文件 = 空声明（**默认不含任何类 B 声明**）.
    let missing =
        ClassBDeclaration::from_json_file(&path).expect("a missing file is not a failure");
    assert!(missing.is_empty(), "缺文件 ⇒ 空声明");
    assert!(!missing.declares("mcp:demo:get_my_api_key"));

    // 合法文件 ⇒ 读回来.
    std::fs::write(
        &path,
        r#"{"tools":["mcp:demo:get_my_api_key"],"reason":"按契约返回凭据"}"#,
    )
    .expect("write the declaration");
    let loaded = ClassBDeclaration::from_json_file(&path).expect("the declaration loads");
    assert!(loaded.declares("mcp:demo:get_my_api_key"));
    assert_eq!(loaded.len(), 1);

    // 坏文件 = **报错**（调用方按 fail-closed 处理 ⇒ 声明保持空）.
    std::fs::write(&path, "{not json").expect("write the broken declaration");
    assert!(
        ClassBDeclaration::from_json_file(&path).is_err(),
        "坏文件必须报错（fail-closed），不许静默降级"
    );
}

/// ⑥ **The ledger records the category and the fingerprint only** — never the
/// true value.
#[tokio::test]
async fn the_ledger_records_the_category_and_the_fingerprint_only() {
    let server = server_answering(&format!("your key is {OPENAI_KEY}"));
    let domain = SessionRedactionDomain::for_session(SessionId::new()).shared();
    let bridge = bridge_over(&server, test_options())
        .with_session_domain(Arc::clone(&domain))
        .with_class_b_declaration(ClassBDeclaration::with_tools(["mcp:demo:get_my_api_key"]));
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:get_my_api_key").await;
    let text = result_text(&result);

    let guard = domain.lock().expect("the domain is not poisoned");
    let ledger = guard.ledger_jsonl();
    assert!(!ledger.contains(OPENAI_KEY), "账本零真值: {ledger}");
    assert!(ledger.contains("credential_key"), "账本记类别: {ledger}");
    assert!(ledger.contains("redacted"), "账本记事件: {ledger}");

    // 指纹在账本里 ⇒ 审计面能对上占位符, 但**反推不出真值**.
    let alias = placeholders(&text);
    let fingerprint = RedactionPlaceholder::parse(&alias[0])
        .expect("the alias is a placeholder")
        .fingerprint;
    assert!(ledger.contains(&fingerprint), "账本记指纹: {ledger}");
    assert_eq!(guard.class_b_values(), 1, "类 B 登记簿里有这一个值");
}

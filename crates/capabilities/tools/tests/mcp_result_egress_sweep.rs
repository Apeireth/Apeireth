//! The external-tool result face is swept before it reaches the model.
//!
//! The chain under test is the real one: an in-process JSON-RPC server double
//! (no sockets, no subprocesses) → the protocol bridge → the dynamic tool →
//! the five-stage pipeline → the model-facing result. What is pinned here is
//! the **result-face sweep**:
//!
//! * a remote answer that carries credential shapes is rewritten before the
//!   model sees it — the true values never reach the model context;
//! * a remote answer that only *talks about* credentials is left alone —
//!   the sweep must not rewrite ordinary content;
//! * a clean answer keeps the exact shape it had before the wiring;
//! * the sweep is observable (the rewritten markers, the sidecar, the ledger);
//! * the wiring is **family-local**: the generic tool path is untouched.
//!
//! The dispositions asserted here are the ones the sweep really performs
//! (placeholder family for the shapes it covers, one-way labels for the
//! shapes only the tripwire covers) — they are read off the mechanism, not
//! assumed.

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::exec_pipeline::ToolExecutionPipeline;
use apeireth_tools_canonical::guardrail::EgressSweeper;
use apeireth_tools_canonical::mcp_bridge::{
    InProcessMcpServer, McpBridgeOptions, McpDynamicTool, McpPermissionMapping, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec,
};

const SERVER: &str = "demo";

/// A credential shape the placeholder family covers (an alias, not a label).
const OPENAI_KEY: &str = "sk-proj-1234567890abcdef1234567890";
/// A personal access token the placeholder family covers.
const ACCESS_TOKEN: &str = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
/// A password assignment the placeholder family covers.
const PASSWORD_VALUE: &str = "hunter2secret";
/// An environment secret assignment the placeholder family covers.
const ENV_SECRET_VALUE: &str = "abcdef1234567890";
/// The password of a credential URL (the alias covers the credential part).
const URL_PASSWORD: &str = "s3cr3tp4ss";
/// A private key body only the one-way tripwire covers.
const PEM_BODY: &str = "MIIEowIBAAKCAQEA0";
/// A bearer token body only the one-way tripwire covers.
const BEARER_VALUE: &str = "opaqueTokenValue1234567890abc";
/// A JSON web token body only the one-way tripwire covers.
const JWT_BODY: &str =
    "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

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
    McpToolBridge::from_connections(config, options, vec![connection]).unwrap()
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
    McpPermissionMapping::authorize(&mut bridge.policy().lock().unwrap(), &entry);
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

/// **The gap this batch closes**: a remote answer that carries credential
/// shapes must not carry them into the model context any more.
#[tokio::test]
async fn remote_credential_shapes_are_swept_before_they_reach_the_model() {
    let server = InProcessMcpServer::new(SERVER);
    let leaking = format!(
        "openai {OPENAI_KEY}\n\
         token {ACCESS_TOKEN}\n\
         password = {PASSWORD_VALUE}\n\
         env API_KEY={ENV_SECRET_VALUE}\n\
         url https://user:{URL_PASSWORD}@host.example/repo.git\n\
         bearer Authorization: Bearer {BEARER_VALUE}\n\
         pem -----BEGIN RSA PRIVATE KEY-----\n{PEM_BODY}\n-----END RSA PRIVATE KEY-----\n\
         jwt {JWT_BODY}\n"
    );
    server.set_tools(vec![MockToolSpec::new("leak", leaking.clone())]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:leak").await;
    assert!(result.is_ok(), "{}", result.render());
    let rendered = result.render();

    // No true value of any credential shape reaches the model context.
    for secret in [
        OPENAI_KEY,
        ACCESS_TOKEN,
        PASSWORD_VALUE,
        ENV_SECRET_VALUE,
        URL_PASSWORD,
        BEARER_VALUE,
        PEM_BODY,
        JWT_BODY,
    ] {
        assert!(
            !rendered.contains(secret),
            "credential still in the model-facing result: {secret}\n{rendered}"
        );
    }

    // The dispositions are the mechanism's own: the placeholder family for
    // the shapes it covers (an irreversible alias), the one-way labels for
    // the shapes only the tripwire covers.
    assert!(
        rendered.contains("__AP_CREDENTIAL_KEY_"),
        "placeholder family must alias the key shapes: {rendered}"
    );
    assert!(
        rendered.contains("__AP_ENV_SECRET_"),
        "placeholder family must alias the secret assignments: {rendered}"
    );
    assert!(
        rendered.contains("[REDACTED_PEM_PRIVATE_KEY]"),
        "the private key block must be tagged one-way: {rendered}"
    );
    assert!(
        rendered.contains("[REDACTED_JWT]"),
        "the token body must be tagged one-way: {rendered}"
    );

    // Observable: the structured sidecar says the answer was swept and
    // which credential shapes were found.
    let sidecar = structured(&result)
        .get("egress_sweep")
        .expect("a swept answer carries the sweep sidecar");
    assert!(
        sidecar["redaction_hits"].as_u64().unwrap_or(0) > 0,
        "the sidecar must count the placeholder hits: {sidecar}"
    );
    let kinds: Vec<&str> = sidecar["tripwire_kinds"]
        .as_array()
        .expect("the sidecar lists the credential kinds")
        .iter()
        .filter_map(|kind| kind.as_str())
        .collect();
    assert!(
        kinds.contains(&"PemPrivateKey"),
        "the private key must be reported: {kinds:?}"
    );
    assert!(
        kinds.contains(&"JwtToken"),
        "the token body must be reported: {kinds:?}"
    );
}

/// **The most important assertion of this batch**: ordinary content that
/// merely *mentions* credentials must survive untouched.
#[tokio::test]
async fn ordinary_remote_content_is_not_swept() {
    let clean = concat!(
        "The password field is empty here\n",
        "rotate your token before the next login\n",
        "The bearer configuration mechanism is documented elsewhere.\n",
        "authorization header explained\n",
        "api_key: abc\n",
        "secret sauce recipe\n",
    );
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("ordinary", clean)]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:ordinary").await;
    assert!(result.is_ok(), "{}", result.render());
    let rendered = result.render();
    let text = structured(&result)["text"]
        .as_str()
        .expect("the joined text is a string")
        .to_string();

    assert_eq!(
        text, clean,
        "ordinary content must survive verbatim: {text}"
    );
    assert!(
        !rendered.contains("__AP_"),
        "ordinary content must not be aliased: {rendered}"
    );
    assert!(
        !rendered.contains("[REDACTED_"),
        "ordinary content must not be tagged: {rendered}"
    );
    assert!(
        structured(&result).get("egress_sweep").is_none(),
        "an untouched answer carries no sweep sidecar"
    );
}

/// Zero behaviour change for an answer that has nothing to sweep: the
/// structured value is exactly what the pre-wiring builder produces.
#[tokio::test]
async fn a_clean_answer_keeps_the_exact_shape_it_had_before_the_wiring() {
    let plain = "the answer is 42";
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("plain", plain)]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:plain").await;
    assert!(result.is_ok(), "{}", result.render());
    let value = structured(&result);
    assert_eq!(value["text"], serde_json::json!(plain));
    assert_eq!(value["is_error"], serde_json::json!(false));
    assert_eq!(
        value["content"],
        serde_json::json!([{ "type": "text", "text": plain }]),
        "the content face must be untouched"
    );
    assert!(
        value.get("egress_sweep").is_none(),
        "no hit means no sidecar: {value}"
    );
}

/// The sweep covers **every** text block, not only the joined text: a
/// credential hiding in the second block must not reach the model either.
#[tokio::test]
async fn every_text_block_is_swept_not_only_the_joined_text() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("blocks", "").with_raw_result(
        serde_json::json!({
            "content": [
                { "type": "text", "text": "first block, harmless" },
                { "type": "text", "text": format!("second block: password = {PASSWORD_VALUE}") }
            ],
            "isError": false
        }),
    )]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:blocks").await;
    assert!(result.is_ok(), "{}", result.render());
    let rendered = result.render();
    assert!(
        !rendered.contains(PASSWORD_VALUE),
        "the second block leaked its credential: {rendered}"
    );
    assert!(
        rendered.contains("first block, harmless"),
        "the harmless block must survive: {rendered}"
    );
}

/// The server's error face is server-authored content too, and it reaches
/// the model context the same way a result does.
#[tokio::test]
async fn a_server_error_reply_is_swept_too() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("boom", "")
        .error_reply(format!("the call failed: password = {PASSWORD_VALUE}"))]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:boom").await;
    assert!(!result.is_ok(), "a server error is not a success");
    let rendered = result.render();
    assert!(
        !rendered.contains(PASSWORD_VALUE),
        "the error face leaked its credential: {rendered}"
    );
    assert!(
        rendered.contains("__AP_"),
        "the error face must carry the rewritten marker: {rendered}"
    );
}

/// The sweep is a fixed point: sweeping an already-swept text changes
/// nothing, so a value that travels through two sweeps does not drift.
#[test]
fn the_sweep_is_idempotent() {
    let mut sweeper = EgressSweeper::new();
    let raw = format!("password = {PASSWORD_VALUE}\n{PEM_BODY}");
    let first = sweeper.sweep(&raw);
    let second = sweeper.sweep(first.text());
    assert_eq!(
        first.text(),
        second.text(),
        "a swept text must be a fixed point: {} vs {}",
        first.text(),
        second.text()
    );
}

/// The ledger is the observability face of the sweep — and it never stores a
/// true value.
#[tokio::test]
async fn the_sweep_ledger_records_the_hit_without_the_true_value() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new(
        "leak",
        format!("password = {PASSWORD_VALUE}"),
    )]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    // The handle is built over the **discovered** entry: the descriptor the
    // server advertised is the descriptor the checked call path compares
    // against, so a hand-built entry would be refused before the call ever
    // reached the server.
    let tool = McpDynamicTool::new(
        bridge
            .catalog()
            .get("mcp:demo:leak")
            .expect("the discovered tool is in the catalog"),
        Arc::clone(&bridge.connections()[0]),
    );

    let result = tool.invoke(&call("mcp:demo:leak")).await;
    assert!(result.is_ok(), "{}", result.render());

    // The tool's sweep domain keeps the ledger of what it rewrote.
    let ledger = tool
        .sweeper()
        .lock()
        .expect("the sweep domain is not poisoned")
        .ledger_jsonl();
    assert!(
        ledger.contains("redacted"),
        "the ledger must record the redaction: {ledger}"
    );
    assert!(
        !ledger.contains(PASSWORD_VALUE),
        "the ledger must never store a true value: {ledger}"
    );
}

/// **A stated boundary of the result face**: the placeholder family covers
/// the whole egress face, so the non-credential PII shapes it knows are
/// rewritten as well (an address, a long number, an addressee). This test
/// pins that fact so it is never a surprise: it is the mechanism's own
/// semantics, not this batch's invention.
#[tokio::test]
async fn the_placeholder_family_also_rewrites_non_credential_pii_shapes() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new(
        "inventory",
        "host 10.0.0.7 wrote 4111111111111111 to test.user@example.com",
    )]);
    let bridge = bridge_over(&server, test_options());
    bridge.refresh().await.expect("discovery succeeds");

    let result = call_external(&bridge, "mcp:demo:inventory").await;
    assert!(result.is_ok(), "{}", result.render());
    let rendered = result.render();
    assert!(
        !rendered.contains("10.0.0.7"),
        "the address shape is rewritten by the placeholder family: {rendered}"
    );
    assert!(
        !rendered.contains("4111111111111111"),
        "the long-number shape is rewritten by the placeholder family: {rendered}"
    );
    assert!(
        !rendered.contains("test.user@example.com"),
        "the addressee shape is rewritten by the placeholder family: {rendered}"
    );
}

/// The wiring is **family-local**: a tool of another family walks the same
/// pipeline and gets its result back byte-for-byte.
#[tokio::test]
async fn the_generic_tool_path_is_untouched_by_the_external_tool_sweep() {
    struct PlainTool {
        id: CapabilityId,
    }

    #[async_trait::async_trait]
    impl ToolCapability for PlainTool {
        fn id(&self) -> &CapabilityId {
            &self.id
        }

        fn declaration(&self) -> apeireth_protocol::canonical::NormalizedTool {
            apeireth_protocol::canonical::NormalizedTool {
                name: "plain".into(),
                description: Some("a tool of another family".into()),
                parameters: apeireth_protocol::canonical::ToolParameters::new(),
                strict: false,
            }
        }

        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            ToolResult::ok(
                &call.id,
                serde_json::json!({ "answer": format!("password = {PASSWORD_VALUE}") }),
            )
        }
    }

    let tool = PlainTool {
        id: CapabilityId::new("tool.plain.probe").expect("valid capability id"),
    };
    let pipeline = ToolExecutionPipeline::new();
    let executed = pipeline.run(&tool, &call("plain")).await;
    assert!(executed.result.is_ok());
    assert_eq!(
        executed.result.render(),
        format!(r#"{{"answer":"password = {PASSWORD_VALUE}"}}"#),
        "the generic tool path must not be swept by the external-tool wiring"
    );
    assert!(
        executed.result.render().contains(PASSWORD_VALUE),
        "another family keeps its own behaviour"
    );
}

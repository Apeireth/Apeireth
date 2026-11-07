//! The transport's response bound **follows** the bridge's per-call deadline.
//!
//! # The gap this file closes
//!
//! The transport's response wait was bounded by a **hardcoded** constant
//! ([`RESPONSE_TIMEOUT`], 30 s) that did not follow
//! [`McpBridgeOptions::call_timeout_ms`]. The transport's own comment states
//! the invariant it must serve:
//!
//! > the transport must never be the stricter of the two, or it would cut a
//! > call the bridge still allows
//!
//! With a per-call deadline **above** 30 s the invariant was **inverted**: the
//! transport gave up first, so the call failed as a **transport failure**
//! (retryable) instead of the bridge's own `timeout.*` classification — and a
//! call whose side effect may already have landed was handed back as
//! *retryable*, so a retry would repeat it.
//!
//! # What is pinned here
//!
//! * the **derivation**: the transport's bound is never stricter than the
//!   bridge's, and never below the handshake tier;
//! * the **wiring**: the production connection (`from_spec`) hands its links
//!   exactly that bound — not the constant;
//! * the **classification**: with the bridge's deadline longer than the
//!   transport's own tier, a call that outlives the bridge's deadline is
//!   classified by the **bridge** (`timeout.deadline_expired`, permanent),
//!   never as a transport failure.
//!
//! # How the cases are driven
//!
//! * The peer is an **in-process double** (no sockets, no subprocesses, no
//!   network) that answers the handshake and the tool list, and never answers
//!   a tool call.
//! * The clock is **paused** (`tokio::time` test mode): the case uses the
//!   production-scale numbers (a 60 s per-call deadline against the old 30 s
//!   tier) without waiting for them.
//! * The reconnect budget is **exhausted on the first disconnect**, so the
//!   case cannot drift into a reconnect loop.

use std::sync::Arc;
use std::time::Duration;

use apeireth_plugin::mcp::{handle_initialize, JsonRpcRequest, JsonRpcResponse, ServerInfo};
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::mcp::McpError;
use apeireth_tools_canonical::mcp_bridge::{
    response_deadline_for_call, McpBridgeOptions, McpChannel, McpChannelFactory, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, RESPONSE_TIMEOUT, TRANSPORT_DEADLINE_GRACE,
};

/// The server name the double answers under.
const SERVER: &str = "stub";

/// The per-call deadline under test: **longer** than the old hardcoded tier
/// (30 s) — the configuration in which the invariant was inverted.
const BRIDGE_DEADLINE_MS: u64 = 60_000;

/// The bridge's options under test: a per-call deadline of one minute, and a
/// reconnect budget that is spent on the first disconnect.
fn test_options() -> McpBridgeOptions {
    let reconnect = apeireth_plugin::mcp::ReconnectPolicy {
        initial_backoff: Duration::from_millis(1),
        multiplier_num: 1,
        multiplier_den: 1,
        max_backoff: Duration::from_millis(4),
        // 0 = the first disconnect exhausts the budget: no reconnect, no
        // second attempt, no drift.
        max_attempts: 0,
    };
    McpBridgeOptions::default()
        .with_call_timeout(BRIDGE_DEADLINE_MS, 120_000)
        .with_reconnect(reconnect)
}

fn demo_spec() -> McpServerSpec {
    McpServerSpec::stdio(SERVER, "in-process-test-server")
}

/// The in-process double: it answers the handshake and the tool list, and it
/// **never** answers a tool call.
struct SilentChannel {
    /// The bound the transport's own response wait runs into.
    response_deadline: Duration,
}

#[async_trait::async_trait]
impl McpChannel for SilentChannel {
    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        match request.method.as_str() {
            "initialize" => Ok(handle_initialize(
                &request,
                ServerInfo::for_server(SERVER.to_string()),
            )),
            "tools/list" => Ok(JsonRpcResponse::ok(
                request.id.clone(),
                serde_json::json!({
                    "tools": [{
                        "name": "slow",
                        "description": "never answers",
                        "inputSchema": { "type": "object", "properties": {} }
                    }]
                }),
            )),
            _ => {
                // The tool call: take it and never answer it. The wait runs
                // into the **same** bound the production transport uses, so
                // the case reproduces the production ordering.
                tokio::time::timeout(self.response_deadline, std::future::pending::<()>()).await;
                Err(McpError::Transport(format!(
                    "stdio channel did not deliver the response within {:?}",
                    self.response_deadline
                )))
            }
        }
    }

    async fn notify(&self, _notification: JsonRpcRequest) -> Result<(), McpError> {
        Ok(())
    }
}

/// A factory handing out [`SilentChannel`]s with the production-derived bound.
struct SilentFactory {
    response_deadline: Duration,
}

#[async_trait::async_trait]
impl McpChannelFactory for SilentFactory {
    fn response_deadline(&self) -> Option<Duration> {
        Some(self.response_deadline)
    }

    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError> {
        Ok(Arc::new(SilentChannel {
            response_deadline: self.response_deadline,
        }))
    }
}

/// The bridge over the double.
fn bridge_over(options: McpBridgeOptions) -> McpToolBridge {
    let config = McpServerConfig::new(vec![demo_spec()]);
    let factory = Arc::new(SilentFactory {
        response_deadline: response_deadline_for_call(options.call_timeout_ms),
    });
    let connection = Arc::new(McpServerConnection::new(demo_spec(), factory, &options));
    McpToolBridge::from_connections(config, options, vec![connection])
        .expect("the bridge assembles")
}

/// ① **The derivation**: the transport's bound is never stricter than the
/// bridge's — that is the invariant the transport's comment declares.
#[test]
fn the_transport_bound_is_never_stricter_than_the_bridges_deadline() {
    for bridge_ms in [1u64, 200, 30_000, BRIDGE_DEADLINE_MS, 120_000] {
        let transport = response_deadline_for_call(bridge_ms);
        let bridge = Duration::from_millis(bridge_ms);
        assert!(
            transport > bridge,
            "the transport ({transport:?}) must not be the stricter of the two \
             (bridge {bridge:?})"
        );
        assert!(
            transport >= RESPONSE_TIMEOUT,
            "the transport's bound ({transport:?}) must never fall below the \
             handshake tier ({RESPONSE_TIMEOUT:?})"
        );
    }
}

/// ② **The wiring**: the production connection hands its links the derived
/// bound — not the constant.
#[test]
fn the_production_connection_bounds_its_links_by_the_bridges_deadline() {
    let options = test_options();
    let connection = McpServerConnection::from_spec(demo_spec(), &options);
    assert_eq!(
        connection.response_deadline(),
        Some(response_deadline_for_call(BRIDGE_DEADLINE_MS)),
        "the production link bound must follow the bridge's per-call deadline"
    );
    assert!(
        connection.response_deadline() != Some(RESPONSE_TIMEOUT),
        "the link bound must not be the hardcoded tier any more"
    );
}

/// ③ **The classification**: a bridge deadline **longer** than the transport's
/// own tier ⇒ the call is classified by the **bridge** (`timeout.*`), never
/// as a transport failure.
///
/// The clock is paused, so the production-scale numbers (60 s against the old
/// 30 s tier) cost no wall-clock time.
#[tokio::test(start_paused = true)]
async fn a_bridge_deadline_longer_than_the_transport_tier_is_classified_by_the_bridge() {
    let options = test_options();
    let bridge = bridge_over(options.clone());
    bridge
        .refresh()
        .await
        .expect("the double answers the handshake and the tool list");
    let entry = bridge
        .catalog()
        .get("mcp:stub:slow")
        .expect("the discovered tool is in the catalog");
    // The external family is released for the call.
    apeireth_tools_canonical::mcp_bridge::McpPermissionMapping::authorize(
        &mut bridge.policy().lock().unwrap(),
        &entry,
    );
    let tool = bridge
        .tools()
        .into_iter()
        .find(|tool| tool.declaration().name == "mcp:stub:slow")
        .expect("the discovered tool is wrapped and live");

    let result = tool
        .invoke(&ToolCall {
            id: "call_1".into(),
            name: "mcp:stub:slow".into(),
            arguments: serde_json::json!({}),
        })
        .await;

    assert!(!result.is_ok(), "the call must not succeed");
    let rendered = result.render();
    assert!(
        rendered.contains("timeout.deadline_expired"),
        "the bridge's own timeout family must classify the call: {rendered}"
    );
    assert!(
        !rendered.contains("mcp transport failure"),
        "the call must not degrade into a transport failure: {rendered}"
    );
    assert!(
        !result.outcome.is_retryable(),
        "a timeout after the call was entered is permanent (the side effect \
         may have landed): {rendered}"
    );
}

/// ④ **The control**: with the bridge's deadline **shorter** than the
/// transport's bound, the same double is still classified by the bridge —
/// the transport's bound never cuts a call the bridge still allows.
#[tokio::test(start_paused = true)]
async fn a_short_bridge_deadline_is_classified_by_the_bridge_too() {
    let options = McpBridgeOptions::default().with_call_timeout(200, 120_000);
    let bridge = bridge_over(options);
    bridge.refresh().await.expect("discovery succeeds");
    let entry = bridge
        .catalog()
        .get("mcp:stub:slow")
        .expect("the discovered tool is in the catalog");
    apeireth_tools_canonical::mcp_bridge::McpPermissionMapping::authorize(
        &mut bridge.policy().lock().unwrap(),
        &entry,
    );
    let tool = bridge
        .tools()
        .into_iter()
        .find(|tool| tool.declaration().name == "mcp:stub:slow")
        .expect("the discovered tool is wrapped and live");

    let result = tool
        .invoke(&ToolCall {
            id: "call_2".into(),
            name: "mcp:stub:slow".into(),
            arguments: serde_json::json!({}),
        })
        .await;

    assert!(!result.is_ok(), "the call must not succeed");
    let rendered = result.render();
    assert!(
        rendered.contains("timeout.deadline_expired"),
        "the bridge's own timeout family must classify the call: {rendered}"
    );
    assert!(
        !rendered.contains("mcp transport failure"),
        "the call must not degrade into a transport failure: {rendered}"
    );
}

/// ⑤ **The grace**: the transport's bound keeps a margin above the bridge's,
/// so the two timers cannot expire in the same millisecond (a race would let
/// the transport's failure win and the classification would degrade).
#[test]
fn the_transport_bound_keeps_a_margin_above_the_bridges_deadline() {
    let transport = response_deadline_for_call(BRIDGE_DEADLINE_MS);
    let bridge = Duration::from_millis(BRIDGE_DEADLINE_MS);
    let margin = transport.checked_sub(bridge).unwrap_or_else(|| {
        panic!("the transport's bound ({transport:?}) must stay above the bridge's ({bridge:?})")
    });
    assert_eq!(
        margin, TRANSPORT_DEADLINE_GRACE,
        "the margin is what keeps the ordering strict"
    );
}
